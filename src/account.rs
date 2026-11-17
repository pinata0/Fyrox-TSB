//! Native account DTOs. The device credential stays in the directory worker.
use crate::online::OnlineSession;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone, Debug, Deserialize)]
pub struct Profile {
    pub handle: String,
    pub real_name: String,
    pub display_name: String,
    pub bio: String,
    pub avatar: String,
    pub avatar_url: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Person {
    pub id: String,
    pub user_id: Option<String>,
    pub handle: String,
    pub display_name: String,
    #[serde(default)]
    pub bio: String,
    pub status: Option<String>,
    pub relationship_status: Option<String>,
    #[serde(default)]
    pub incoming: bool,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Block {
    pub user_id: String,
    pub handle: String,
    pub display_name: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct HostedRoom {
    pub id: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Invite {
    pub id: String,
    pub title: String,
    pub host_name: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Dashboard {
    pub profile: Option<Profile>,
    pub relationships: Vec<Person>,
    pub blocks: Vec<Block>,
    pub rooms: Vec<HostedRoom>,
    pub invites: Vec<Invite>,
}
#[derive(Clone, Debug)]
pub enum Action {
    Dashboard,
    Search(String),
    Save(Profile),
    UploadPhoto,
    RemovePhoto,
    Request(String),
    Accept(String),
    Remove(String),
    Block(String),
    Unblock(String),
    Invite { session: String, user: String },
    Decline(String),
    Join(String),
}
impl Action {
    pub fn request(&self) -> (&'static str, Value) {
        match self {
            Self::Dashboard => ("dashboard", json!({})),
            Self::Search(query) => ("search_players", json!({"query":query})),
            Self::Save(p) => (
                "save_profile",
                json!({"handle":p.handle,"real_name":p.real_name,"display_name":p.display_name,"bio":p.bio,"avatar":p.avatar}),
            ),
            Self::UploadPhoto | Self::RemovePhoto => ("photo", json!({})),
            Self::Request(handle) => ("request_friend", json!({"handle":handle})),
            Self::Accept(id) => ("accept_friend", json!({"id":id})),
            Self::Remove(id) => ("remove_friend", json!({"id":id})),
            Self::Block(handle) => ("block_user", json!({"handle":handle})),
            Self::Unblock(id) => ("unblock_user", json!({"user_id":id})),
            Self::Invite { session, user } => (
                "invite_friend",
                json!({"session_id":session,"user_id":user}),
            ),
            Self::Decline(id) => ("decline_invite", json!({"id":id})),
            Self::Join(id) => ("join_invite", json!({"id":id})),
        }
    }
}
pub enum Response {
    Dashboard(Dashboard),
    Search(Vec<Person>),
    Changed,
    Joined {
        code: Option<String>,
        session: OnlineSession,
    },
    Expired,
}

pub fn parse(action: &Action, value: Value) -> Result<Response, String> {
    let invalid = || "계정 서비스 응답을 확인해 주세요.".to_string();
    match action {
        Action::Dashboard => {
            let data: Dashboard = serde_json::from_value(value).map_err(|_| invalid())?;
            if data.relationships.len() > 2000
                || data.blocks.len() > 2000
                || data.invites.len() > 1000
                || data.profile.as_ref().is_some_and(|p| !valid_profile(p))
                || data.relationships.iter().any(|p| !valid_person(p))
            {
                return Err(invalid());
            }
            Ok(Response::Dashboard(data))
        }
        Action::Search(_) => {
            let people: Vec<Person> = serde_json::from_value(value).map_err(|_| invalid())?;
            if people.len() > 20 || people.iter().any(|p| !valid_person(p)) {
                return Err(invalid());
            }
            Ok(Response::Search(people))
        }
        Action::Join(_) => {
            let session: OnlineSession =
                serde_json::from_value(value["session"].clone()).map_err(|_| invalid())?;
            session
                .validate(crate::online::unix_now())
                .map_err(|_| invalid())?;
            if session.mode != "join" {
                return Err(invalid());
            }
            let code = match value["code"].as_str() {
                Some(code) => {
                    Some(crate::room_directory::normalize_code(code).ok_or_else(invalid)?)
                }
                None => None,
            };
            Ok(Response::Joined { code, session })
        }
        _ if value["ok"] == true => Ok(Response::Changed),
        _ => Err(invalid()),
    }
}
fn bounded(value: &str, max: usize) -> bool {
    value.chars().count() <= max && !value.chars().any(char::is_control)
}
fn valid_person(p: &Person) -> bool {
    bounded(&p.display_name, 32) && bounded(&p.handle, 24) && p.bio.chars().count() <= 240
}
fn valid_profile(p: &Profile) -> bool {
    bounded(&p.display_name, 32)
        && bounded(&p.handle, 24)
        && bounded(&p.real_name, 50)
        && p.bio.chars().count() <= 240
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_malformed_data_and_host_credentials_in_invite_response() {
        assert!(parse(&Action::Dashboard, json!({"profile":null})).is_err());
        assert!(parse(
            &Action::Search("친구".into()),
            json!([{ "id":"a", "handle":"friend", "display_name":"x\n" }])
        )
        .is_err());
        assert!(parse(&Action::Request("friend".into()), json!({"ok":false})).is_err());
        assert!(parse(
            &Action::Join("a".into()),
            json!({"session":{"mode":"host"}})
        )
        .is_err());
    }
}
