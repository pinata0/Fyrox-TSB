//! Datagram transport shared by gameplay, chat and voice.
use std::{
    fmt, io,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
};

pub const MAX_DATAGRAM: usize = 1200; // Preserve the existing game protocol.
pub const EOS_FRAME: usize = 1170; // EOS_P2P_MAX_PACKET_SIZE in the installed SDK.
const FRAGMENT_HEADER: usize = 16;
const FRAGMENT_DATA: usize = EOS_FRAME - FRAGMENT_HEADER;

/// Only the largest snapshots need two EOS frames. Incomplete snapshots expire
/// instead of being retransmitted behind more recent simulation state.
pub fn frames(bytes: &[u8], sequence: u64) -> io::Result<Vec<Vec<u8>>> {
    if bytes.len() > MAX_DATAGRAM {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Oversize game packet",
        ));
    }
    if bytes.len() <= EOS_FRAME {
        return Ok(vec![bytes.to_vec()]);
    }
    let count = bytes.len().div_ceil(FRAGMENT_DATA) as u8;
    Ok(bytes
        .chunks(FRAGMENT_DATA)
        .enumerate()
        .map(|(index, chunk)| {
            let mut frame = b"TSBF".to_vec();
            frame.extend(sequence.to_le_bytes());
            frame.extend((bytes.len() as u16).to_le_bytes());
            frame.extend([index as u8, count]);
            frame.extend(chunk);
            frame
        })
        .collect())
}
#[derive(Default)]
pub struct Reassembler {
    partial: std::collections::HashMap<([u8; 32], u64), PartialPacket>,
}
struct PartialPacket {
    created: std::time::Instant,
    total: usize,
    parts: [Option<Vec<u8>>; 2],
}
impl Reassembler {
    pub fn receive(
        &mut self,
        peer: [u8; 32],
        bytes: &[u8],
        now: std::time::Instant,
    ) -> Option<Vec<u8>> {
        self.partial
            .retain(|_, p| now.duration_since(p.created) < std::time::Duration::from_millis(250));
        if bytes.len() > EOS_FRAME {
            return None;
        }
        if !bytes.starts_with(b"TSBF") {
            return Some(bytes.to_vec());
        }
        if bytes.len() < FRAGMENT_HEADER {
            return None;
        }
        let sequence = u64::from_le_bytes(bytes[4..12].try_into().ok()?);
        let total = u16::from_le_bytes(bytes[12..14].try_into().ok()?) as usize;
        let index = bytes[14] as usize;
        if !(EOS_FRAME + 1..=MAX_DATAGRAM).contains(&total) || index > 1 || bytes[15] != 2 {
            return None;
        }
        let expected = if index == 0 {
            FRAGMENT_DATA
        } else {
            total - FRAGMENT_DATA
        };
        if bytes.len() != FRAGMENT_HEADER + expected {
            return None;
        }
        let key = (peer, sequence);
        if !self.partial.contains_key(&key) && self.partial.len() >= 128 {
            return None;
        }
        let packet = self.partial.entry(key).or_insert_with(|| PartialPacket {
            created: now,
            total,
            parts: [None, None],
        });
        if packet.total != total {
            return None;
        }
        packet.parts[index] = Some(bytes[FRAGMENT_HEADER..].to_vec());
        if packet.parts.iter().all(Option::is_some) {
            let packet = self.partial.remove(&key)?;
            Some(packet.parts.into_iter().flatten().flatten().collect())
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PeerAddr {
    Udp(SocketAddr),
    Eos([u8; 32]),
}
impl From<SocketAddr> for PeerAddr {
    fn from(value: SocketAddr) -> Self {
        Self::Udp(value)
    }
}
impl fmt::Display for PeerAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Udp(addr) => addr.fmt(f),
            Self::Eos(id) => write!(f, "eos:{}", String::from_utf8_lossy(id)),
        }
    }
}

/// A fresh socket secret isolates each room, even when the same host makes another.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoomCode {
    pub host: [u8; 32],
    pub socket: String,
}
pub fn eos_id(value: &str) -> io::Result<[u8; 32]> {
    if value.len() != 32
        || !value.bytes().all(|b| b.is_ascii_hexdigit())
        || value.bytes().all(|b| b == b'0')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid EOS user/socket ID",
        ));
    }
    Ok(value.to_ascii_lowercase().as_bytes().try_into().unwrap())
}
impl std::str::FromStr for RoomCode {
    type Err = io::Error;
    fn from_str(value: &str) -> io::Result<Self> {
        let (host, socket) = value
            .trim()
            .strip_prefix("eos:")
            .and_then(|s| s.split_once(':'))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Paste the complete EOS room code",
                )
            })?;
        eos_id(socket)?;
        Ok(Self {
            host: eos_id(host)?,
            socket: socket.to_ascii_lowercase(),
        })
    }
}
impl fmt::Display for RoomCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "eos:{}:{}",
            String::from_utf8_lossy(&self.host),
            self.socket
        )
    }
}

#[derive(Debug)]
pub enum Socket {
    Udp(UdpSocket),
    #[cfg(feature = "eos")]
    Eos(crate::eos::P2pSocket),
}
impl Socket {
    pub fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self::Udp(socket))
    }
    pub fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No server address"))?;
        let socket = UdpSocket::bind(if addr.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        })?;
        socket.connect(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self::Udp(socket))
    }
    pub fn host_eos(code: Option<&RoomCode>) -> io::Result<Self> {
        #[cfg(feature = "eos")]
        {
            Ok(Self::Eos(crate::eos::P2pSocket::open(code, true)?))
        }
        #[cfg(not(feature = "eos"))]
        {
            let _ = code;
            Err(eos_unavailable())
        }
    }
    pub fn join_eos(code: &RoomCode) -> io::Result<Self> {
        #[cfg(feature = "eos")]
        {
            Ok(Self::Eos(crate::eos::P2pSocket::open(Some(code), false)?))
        }
        #[cfg(not(feature = "eos"))]
        {
            let _ = code;
            Err(eos_unavailable())
        }
    }
    pub fn is_eos(&self) -> bool {
        !matches!(self, Self::Udp(_))
    }
    pub fn room_code(&self) -> Option<String> {
        match self {
            Self::Udp(_) => None,
            #[cfg(feature = "eos")]
            Self::Eos(socket) => socket.room_code(),
        }
    }
    pub fn ready(&self) -> bool {
        match self {
            Self::Udp(_) => true,
            #[cfg(feature = "eos")]
            Self::Eos(socket) => socket.ready(),
        }
    }
    pub fn network_label(&self) -> &'static str {
        match self {
            Self::Udp(_) => "UDP",
            #[cfg(feature = "eos")]
            Self::Eos(socket) => socket.network_label(),
        }
    }
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        match self {
            Self::Udp(socket) => socket.local_addr(),
            #[cfg(feature = "eos")]
            Self::Eos(_) => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "EOS rooms use room codes",
            )),
        }
    }
    pub fn send_to(&self, bytes: &[u8], peer: impl Into<PeerAddr>) -> io::Result<usize> {
        match (self, peer.into()) {
            (Self::Udp(socket), PeerAddr::Udp(addr)) => socket.send_to(bytes, addr),
            #[cfg(feature = "eos")]
            (Self::Eos(socket), PeerAddr::Eos(id)) => socket.send_to(bytes, id),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Peer transport mismatch",
            )),
        }
    }
    pub fn recv_from(&self, bytes: &mut [u8]) -> io::Result<(usize, PeerAddr)> {
        match self {
            Self::Udp(socket) => socket.recv_from(bytes).map(|(n, addr)| (n, addr.into())),
            #[cfg(feature = "eos")]
            Self::Eos(socket) => socket
                .recv_from(bytes)
                .map(|(n, id)| (n, PeerAddr::Eos(id))),
        }
    }
    pub fn send(&self, bytes: &[u8]) -> io::Result<usize> {
        match self {
            Self::Udp(socket) => socket.send(bytes),
            #[cfg(feature = "eos")]
            Self::Eos(socket) => socket.send(bytes),
        }
    }
    pub fn recv(&self, bytes: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Udp(socket) => socket.recv(bytes),
            #[cfg(feature = "eos")]
            Self::Eos(socket) => socket.recv_from(bytes).map(|(n, _)| n),
        }
    }
}
#[cfg(not(feature = "eos"))]
fn eos_unavailable() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "EOS support is missing. Build with scripts/Build-Eos.ps1; use --lan for direct UDP.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn room_codes_are_bounded_and_canonical() {
        let code = format!("eos:{}:{}", "A".repeat(32), "b".repeat(32));
        assert_eq!(
            code.parse::<RoomCode>().unwrap().to_string(),
            code.to_lowercase()
        );
        for invalid in [
            "eos:host:room".to_owned(),
            format!("{code}:extra"),
            format!("eos:{}:{}", "0".repeat(32), "b".repeat(32)),
        ] {
            assert!(invalid.parse::<RoomCode>().is_err());
        }
    }
    #[test]
    fn eos_fragments_reassemble_out_of_order_without_crossing_peers_and_expire() {
        let packet = vec![42; 1199];
        let f = frames(&packet, 4).unwrap();
        assert!(f.iter().all(|f| f.len() <= EOS_FRAME));
        let now = std::time::Instant::now();
        let mut r = Reassembler::default();
        assert!(r.receive([1; 32], &f[1], now).is_none());
        assert!(r.receive([2; 32], &f[0], now).is_none());
        assert!(r.receive([1; 32], &f[1], now).is_none());
        assert_eq!(r.receive([1; 32], &f[0], now), Some(packet));
        assert!(r
            .receive([2; 32], &f[1], now + std::time::Duration::from_secs(1))
            .is_none());
        assert!(r
            .receive(
                [1; 32],
                &f[0][..20],
                now + std::time::Duration::from_secs(1)
            )
            .is_none());
        assert!(frames(&[0; 1201], 1).is_err());
    }
}
