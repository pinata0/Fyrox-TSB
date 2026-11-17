//! Short-lived, signed invitations exported by the DotG player lounge.
//! No Supabase password, access token or service key is stored in these files.
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use std::{
    fs, io,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerIdentity {
    pub id: String,
    pub display_name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnlineSession {
    pub version: u8,
    pub mode: String,
    pub transport: String,
    pub session_id: String,
    pub address: String,
    pub expires_at: u64,
    pub profile: PlayerIdentity,
    pub host_key: Option<String>,
    pub ticket: Option<String>,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn hex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) || !value.is_ascii() {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).ok())
        .collect()
}
fn valid_name(value: &str) -> bool {
    !value.trim().is_empty()
        && value.trim() == value
        && value.chars().count() <= 32
        && !value.chars().any(char::is_control)
}
impl OnlineSession {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        if fs::metadata(path.as_ref())?.len() > 16384 {
            return Err(invalid("Session file is too large"));
        }
        let data = fs::read(path)?;
        let session: Self =
            serde_json::from_slice(&data).map_err(|_| invalid("Invalid session file"))?;
        session.validate(unix_now())?;
        Ok(session)
    }
    pub fn validate(&self, now: u64) -> io::Result<()> {
        if self.version != 1 || !matches!(self.transport.as_str(), "direct_udp" | "eos_p2p") {
            return Err(invalid("Unsupported session transport/version"));
        }
        if !uuid(&self.session_id)
            || !uuid(&self.profile.id)
            || !valid_name(&self.profile.display_name)
        {
            return Err(invalid("Invalid player profile"));
        }
        if self.expires_at <= now || self.expires_at > now + 7260 {
            return Err(invalid(
                "Session expired; download a new file from the player lounge",
            ));
        }
        if self.transport == "eos_p2p" {
            self.address.parse::<crate::transport::RoomCode>()?;
        } else {
            let (host, port) = self
                .address
                .rsplit_once(':')
                .ok_or_else(|| invalid("Invalid host address"))?;
            if host.is_empty()
                || self.address.len() > 260
                || port.parse::<u16>().unwrap_or(0) == 0
                || !host
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b))
            {
                return Err(invalid("Invalid host address"));
            }
        }
        match self.mode.as_str() {
            "host"
                if self.ticket.is_none()
                    && self.host_key.as_ref().is_some_and(|s| {
                        s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
                    }) =>
            {
                Ok(())
            }
            "join"
                if self.host_key.is_none()
                    && self
                        .ticket
                        .as_ref()
                        .is_some_and(|s| s.is_ascii() && s.len() < 512) =>
            {
                Ok(())
            }
            _ => Err(invalid("Invalid session credentials")),
        }
    }
    pub fn bind_address(&self) -> String {
        let port = self.address.rsplit(':').next().unwrap_or("28080");
        format!(
            "{}:{port}",
            if self.address.starts_with('[') {
                "[::]"
            } else {
                "0.0.0.0"
            }
        )
    }
    pub fn host_gate(&self) -> io::Result<HostGate> {
        Ok(HostGate {
            session_id: self.session_id.clone(),
            expires_at: self.expires_at,
            key: self
                .host_key
                .clone()
                .ok_or_else(|| invalid("Host credentials missing"))?,
        })
    }
}

pub struct HostGate {
    session_id: String,
    expires_at: u64,
    key: String,
}
impl std::fmt::Debug for HostGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostGate")
            .field("session_id", &self.session_id)
            .finish_non_exhaustive()
    }
}
pub struct VerifiedInvite {
    pub identity: PlayerIdentity,
    pub ticket_id: String,
}
impl HostGate {
    pub fn verify(&self, ticket: &str, now: u64) -> Option<VerifiedInvite> {
        if now >= self.expires_at || ticket.len() >= 512 {
            return None;
        }
        let (payload, signature) = ticket.rsplit_once('.')?;
        let signature = hex(signature)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(self.key.as_bytes()).ok()?;
        mac.update(payload.as_bytes());
        mac.verify_slice(&signature).ok()?;
        let parts: Vec<_> = payload.split('|').collect();
        if parts.len() != 5 || parts[0] != self.session_id || !uuid(parts[1]) {
            return None;
        }
        let expires: u64 = parts[2].parse().ok()?;
        if expires <= now || expires > self.expires_at || expires > now + 330 {
            return None;
        }
        if parts[3].len() != 32 || !parts[3].bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let display_name = String::from_utf8(hex(parts[4])?).ok()?;
        if !valid_name(&display_name) {
            return None;
        }
        Some(VerifiedInvite {
            identity: PlayerIdentity {
                id: parts[1].into(),
                display_name,
            },
            ticket_id: parts[3].into(),
        })
    }
}

#[cfg(feature = "desktop")]
pub fn open_player_lounge(profile: bool) -> io::Result<()> {
    let mut url = crate::room_directory::portal_url().map_err(|message| invalid(&message))?;
    url.set_fragment(Some(if profile { "profile" } else { "friends" }));
    open::that_detached(url.as_str())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub const ROOM: &str = "11111111-1111-4111-8111-111111111111";
    pub const USER: &str = "22222222-2222-4222-8222-222222222222";
    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    pub fn gate(now: u64) -> HostGate {
        HostGate {
            session_id: ROOM.into(),
            expires_at: now + 7200,
            key: KEY.into(),
        }
    }
    pub fn ticket(expiry: u64, name: &str) -> String {
        let encoded: String = name.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
        let payload = format!("{ROOM}|{USER}|{expiry}|abcdef0123456789abcdef0123456789|{encoded}");
        let mut mac = Hmac::<Sha256>::new_from_slice(KEY.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let sig: String = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        format!("{payload}.{sig}")
    }
    #[test]
    fn signed_invites_bind_room_identity_name_and_expiry() {
        let gate = gate(1000);
        let ticket = ticket(1200, "닷지 친구");
        let parsed = gate.verify(&ticket, 1000).unwrap();
        assert_eq!(parsed.identity.display_name, "닷지 친구");
        assert_eq!(parsed.identity.id, USER);
        assert!(gate.verify(&ticket, 1200).is_none());
        assert!(gate.verify(&ticket.replace(USER, ROOM), 1000).is_none());
        assert!(gate.verify(&ticket.replace("1200", "9000"), 1000).is_none());
        for n in 0..ticket.len() {
            assert!(gate.verify(&ticket[..n], 1000).is_none());
        }
    }
    #[test]
    fn session_file_rejects_unknown_transports_expired_and_mixed_secrets() {
        let mut value = serde_json::json!({"version":1,"mode":"host","transport":"direct_udp","session_id":ROOM,"address":"127.0.0.1:28080","expires_at":1200,"profile":{"id":USER,"display_name":"Host"},"host_key":KEY});
        let valid: OnlineSession = serde_json::from_value(value.clone()).unwrap();
        assert!(valid.validate(1000).is_ok());
        assert!(valid.validate(1200).is_err());
        value["ticket"] = serde_json::json!("x");
        let mixed: OnlineSession = serde_json::from_value(value.clone()).unwrap();
        assert!(mixed.validate(1000).is_err());
        value["transport"] = serde_json::json!("eos");
        let eos: OnlineSession = serde_json::from_value(value).unwrap();
        assert!(eos.validate(1000).is_err());
    }
    #[test]
    fn eos_sessions_require_a_complete_room_code_and_keep_signed_credentials() {
        let code = format!("eos:{}:{}", "a".repeat(32), "b".repeat(32));
        let mut value = serde_json::json!({"version":1,"mode":"host","transport":"eos_p2p","session_id":ROOM,"address":code,"expires_at":1200,"profile":{"id":USER,"display_name":"Host"},"host_key":KEY});
        let host: OnlineSession = serde_json::from_value(value.clone()).unwrap();
        assert!(host.validate(1000).is_ok());
        value["address"] = serde_json::json!("127.0.0.1:28080");
        assert!(serde_json::from_value::<OnlineSession>(value.clone())
            .unwrap()
            .validate(1000)
            .is_err());
        value["address"] = serde_json::json!(code);
        value["mode"] = serde_json::json!("join");
        value["host_key"] = serde_json::Value::Null;
        value["ticket"] = serde_json::json!(ticket(1200, "Guest"));
        let guest: OnlineSession = serde_json::from_value(value).unwrap();
        assert!(guest.validate(1000).is_ok());
        assert!(guest.host_gate().is_err());
    }
}
