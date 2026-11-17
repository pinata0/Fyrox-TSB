//! Background HTTPS room directory. Credentials stay in memory, never in logs/files.
use crate::online::{unix_now, OnlineSession};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{io::Read, sync::mpsc, thread, time::Duration};

const DEFAULT_PORTAL_URL: &str = "https://www.dotgsince2023.co.kr/play";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthMode {
    Login,
    Signup,
}

/// Preserve the one-time device code across login, signup and email confirmation.
pub fn authentication_url(
    mut base: url::Url,
    mode: AuthMode,
    code: &str,
) -> Result<url::Url, String> {
    let code = normalize_code(code).ok_or("로그인 코드를 확인해 주세요.")?;
    base.set_path(match mode {
        AuthMode::Login => "/login",
        AuthMode::Signup => "/signup",
    });
    base.set_query(None);
    base.set_fragment(None);
    base.query_pairs_mut()
        .append_pair("next", &format!("/play?connect={code}"));
    Ok(base)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PlayerProfile {
    pub display_name: String,
    pub handle: Option<String>,
    pub avatar: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug)]
pub enum LinkStatus {
    Pending,
    Connected(PlayerProfile),
    Expired,
}

#[derive(Debug)]
pub struct AvatarImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Room {
    pub code: String,
    pub title: String,
    pub host_name: String,
    pub map_id: String,
    pub players: usize,
    pub capacity: usize,
    pub state: String,
}
impl Room {
    pub fn map_name(&self) -> &str {
        if self.map_id == "elf-city" {
            "엘프 도시"
        } else {
            "요정왕국 성"
        }
    }
    pub fn state_name(&self) -> &str {
        match self.state.as_str() {
            "playing" => "게임 중",
            "finished" => "경기 종료",
            _ => "대기 중",
        }
    }
}
pub enum Command {
    Login {
        epoch: u64,
        email: String,
        password: String,
    },
    PreparePhoto {
        epoch: u64,
        request: u64,
        path: std::path::PathBuf,
    },
    DiscardPhoto,
    Account {
        epoch: u64,
        request: u64,
        action: crate::account::Action,
    },
    List,
    Link(u64),
    LinkStatus(u64),
    ForgetAccount,
    Avatar(String),
    Create {
        title: String,
        endpoint: String,
        map: String,
    },
    Join(String),
    Heartbeat {
        map: String,
        players: usize,
        state: String,
    },
    Close,
    Shutdown,
}
pub enum Reply {
    LoggedIn(u64, Result<PlayerProfile, String>),
    PhotoPrepared {
        epoch: u64,
        request: u64,
        result: Result<AvatarImage, String>,
    },
    Account {
        epoch: u64,
        request: u64,
        result: Result<crate::account::Response, String>,
    },
    Rooms(Result<Vec<Room>, String>),
    Link(u64, Result<String, String>),
    Linked(u64, Result<LinkStatus, String>),
    Avatar {
        url: String,
        result: Result<AvatarImage, String>,
    },
    Session(Result<(String, OnlineSession), String>),
    Heartbeat(Result<(), String>),
    Closed(Result<(), String>),
}
pub struct Directory {
    tx: mpsc::Sender<Command>,
    pub replies: mpsc::Receiver<Reply>,
    worker: Option<thread::JoinHandle<()>>,
}
impl std::fmt::Debug for Directory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Directory { .. }")
    }
}
impl Directory {
    pub fn new() -> Self {
        let (tx, commands) = mpsc::channel();
        let (answers, replies) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut api = Api::new();
            while let Ok(command) = commands.recv() {
                let reply = match command {
                    Command::Login {
                        epoch,
                        email,
                        password,
                    } => Reply::LoggedIn(epoch, api.login(email, password)),
                    Command::PreparePhoto {
                        epoch,
                        request,
                        path,
                    } => Reply::PhotoPrepared {
                        epoch,
                        request,
                        result: api.prepare_photo(&path),
                    },
                    Command::DiscardPhoto => {
                        api.photo = None;
                        continue;
                    }
                    Command::Account {
                        epoch,
                        request,
                        action,
                    } => Reply::Account {
                        epoch,
                        request,
                        result: api.account(action),
                    },
                    Command::List => Reply::Rooms(api.list()),
                    Command::Link(epoch) => Reply::Link(epoch, api.link()),
                    Command::LinkStatus(epoch) => Reply::Linked(epoch, api.link_status()),
                    Command::ForgetAccount => {
                        api.logout();
                        continue;
                    }
                    Command::Avatar(url) => {
                        let result = api.avatar(&url);
                        Reply::Avatar { url, result }
                    }
                    Command::Create {
                        title,
                        endpoint,
                        map,
                    } => Reply::Session(api.session(
                        "create",
                        json!({"title": title,"endpoint":endpoint,"map_id":map}),
                    )),
                    Command::Join(code) => {
                        Reply::Session(api.session("join", json!({"code":code})))
                    }
                    Command::Heartbeat {
                        map,
                        players,
                        state,
                    } => Reply::Heartbeat(api.heartbeat(map, players, state)),
                    Command::Close => Reply::Closed(api.close()),
                    Command::Shutdown => {
                        let _ = api.close();
                        api.logout();
                        break;
                    }
                };
                let _ = answers.send(reply);
            }
        });
        Self {
            tx,
            replies,
            worker: Some(worker),
        }
    }
    pub fn send(&self, command: Command) {
        let _ = self.tx.send(command);
    }
}
impl Default for Directory {
    fn default() -> Self {
        Self::new()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        // A bounded HTTP timeout applies, including when the window is closed.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
pub fn portal_url() -> Result<url::Url, String> {
    let value = std::env::var("DOTG_PORTAL_URL").unwrap_or_else(|_| DEFAULT_PORTAL_URL.into());
    checked_url(&value)
}
fn checked_url(value: &str) -> Result<url::Url, String> {
    let mut url = url::Url::parse(value).map_err(|_| "웹사이트 주소를 확인해 주세요.")?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if (url.scheme() != "https" && !(url.scheme() == "http" && local))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
    {
        return Err("DOTG_PORTAL_URL은 HTTPS 또는 로컬 HTTP 주소여야 합니다.".into());
    }
    // Production redirects the apex domain to www. Resolve this known alias
    // before sending requests, including POST bodies containing device tokens.
    if url.host_str() == Some("dotgsince2023.co.kr") && url.port().is_none() {
        url.set_host(Some("www.dotgsince2023.co.kr"))
            .map_err(|_| "웹사이트 주소를 확인해 주세요.")?;
    }
    url.set_path("/play");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}
pub fn normalize_code(value: &str) -> Option<String> {
    let value: String = value
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_ascii_uppercase();
    (value.len() == 8 && value.bytes().all(|b| b.is_ascii_hexdigit())).then_some(value)
}
struct Api {
    agent: ureq::Agent,
    base: Result<url::Url, String>,
    token: Option<String>,
    hosted: Option<String>,
    native: Option<NativeSession>,
    photo: Option<PhotoFile>,
}
// Never derive Debug/Serialize: session credentials stay exclusively in this worker.
struct NativeSession {
    access: String,
    refresh: String,
    expires: std::time::Instant,
}
struct PhotoFile {
    bytes: Vec<u8>,
    mime: &'static str,
}
impl Api {
    fn new() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(6)))
                // Never forward credential-bearing JSON to a redirect target.
                .max_redirects(0)
                .http_status_as_error(false)
                .build()
                .into(),
            base: portal_url(),
            token: None,
            hosted: None,
            native: None,
            photo: None,
        }
    }
    fn request(&self, action: &str, mut payload: Value) -> Result<Value, String> {
        let (status, value) = self.response(action, &mut payload)?;
        checked_response(status, value)
    }
    fn response(&self, action: &str, payload: &mut Value) -> Result<(u16, Value), String> {
        self.response_at("/api/game/rooms", action, payload)
    }
    fn response_at(
        &self,
        path: &str,
        action: &str,
        payload: &mut Value,
    ) -> Result<(u16, Value), String> {
        let mut url = self.base.clone()?;
        url.set_path(path);
        if let Some(token) = &self.token {
            if !matches!(action, "list" | "link_begin") {
                payload["token"] = json!(token);
            }
        }
        let result = if action == "list" {
            self.agent.get(url.as_str()).call()
        } else {
            self.agent
                .post(url.as_str())
                .send_json(json!({"action":action,"payload":payload}))
        };
        let mut response = result.map_err(|_| {
            "웹 방 서비스에 연결하지 못했어요. 연결 상태와 웹사이트 주소를 확인해 주세요."
        })?;
        let status = response.status().as_u16();
        if response.status().is_redirection() {
            return Err(
                "웹사이트 주소가 변경됐어요. 게임을 업데이트하거나 웹사이트 연결 주소를 확인해 주세요."
                    .into(),
            );
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(131073)
            .read_to_end(&mut bytes)
            .map_err(|_| "방 정보를 읽지 못했어요.")?;
        if bytes.len() > 131072 {
            return Err("방 목록 응답이 너무 큽니다.".into());
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| "방 서비스 응답을 확인해 주세요.")?;
        Ok((status, value))
    }
    fn account(
        &mut self,
        action: crate::account::Action,
    ) -> Result<crate::account::Response, String> {
        if self.token.is_none() {
            return Ok(crate::account::Response::Expired);
        }
        if matches!(
            action,
            crate::account::Action::UploadPhoto | crate::account::Action::RemovePhoto
        ) {
            return self.change_photo(matches!(action, crate::account::Action::RemovePhoto));
        }
        let (name, mut payload) = action.request();
        let (status, value) = self.response_at("/api/game/account", name, &mut payload)?;
        if status == 401 {
            self.token = None;
            self.native = None;
            self.photo = None;
            return Ok(crate::account::Response::Expired);
        }
        crate::account::parse(&action, checked_response(status, value)?)
    }
    fn auth_request(&self, body: Value) -> Result<Value, String> {
        let mut url = self.base.clone()?;
        url.set_path("/api/game/auth");
        let response = self
            .agent
            .post(url.as_str())
            .send_json(body)
            .map_err(|_| "로그인 서비스에 연결하지 못했어요. 다시 시도해 주세요.")?;
        let (status, value) = read_native_response(response)?;
        checked_response(status, value)
    }
    fn read_session(value: &Value) -> Result<NativeSession, String> {
        let credential = |key: &str, max: usize| {
            value[key]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= max && !s.chars().any(char::is_whitespace))
                .map(str::to_owned)
                .ok_or_else(|| "로그인 응답을 확인해 주세요.".to_string())
        };
        let lifetime = value["expires_in"]
            .as_u64()
            .filter(|s| (1..=86400).contains(s))
            .ok_or("로그인 만료 시간을 확인해 주세요.")?;
        Ok(NativeSession {
            access: credential("access_token", 8192)?,
            refresh: credential("refresh_token", 2048)?,
            expires: std::time::Instant::now() + Duration::from_secs(lifetime.saturating_sub(30)),
        })
    }
    fn login(&mut self, email: String, password: String) -> Result<PlayerProfile, String> {
        self.photo = None;
        let value =
            self.auth_request(json!({"action":"login","email":email,"password":password}))?;
        let session = Self::read_session(&value)?;
        let token = value["device_token"]
            .as_str()
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or("게임 연결 응답을 확인해 주세요.")?
            .to_string();
        let mut profile = value["profile"].clone();
        profile["connected"] = json!(true);
        let LinkStatus::Connected(profile) = parse_profile(profile)? else {
            return Err("프로필을 확인해 주세요.".into());
        };
        self.token = Some(token);
        self.native = Some(session);
        Ok(profile)
    }
    fn logout(&mut self) {
        if let (Some(session), Some(token)) = (&self.native, &self.token) {
            let _ = self.auth_request(json!({"action":"logout","access_token":session.access,
                "refresh_token":session.refresh,"device_token":token}));
        }
        self.native = None;
        self.token = None;
        self.photo = None;
    }
    fn refresh_native(&mut self) -> Result<(), String> {
        let session = self
            .native
            .as_ref()
            .ok_or("사진을 변경하려면 게임에서 다시 로그인해 주세요.")?;
        let value =
            self.auth_request(json!({"action":"refresh","refresh_token":session.refresh}))?;
        self.native = Some(Self::read_session(&value)?);
        Ok(())
    }
    fn prepare_photo(&mut self, path: &std::path::Path) -> Result<AvatarImage, String> {
        self.photo = None;
        let (file, preview) = read_photo(path)?;
        self.photo = Some(file);
        Ok(preview)
    }
    fn change_photo(&mut self, remove: bool) -> Result<crate::account::Response, String> {
        if self
            .native
            .as_ref()
            .is_none_or(|s| s.expires <= std::time::Instant::now())
        {
            self.refresh_native()?;
        }
        let mut url = self.base.clone()?;
        url.set_path("/api/game/account/avatar");
        for attempt in 0..2 {
            let session = self
                .native
                .as_ref()
                .ok_or("게임에서 다시 로그인해 주세요.")?;
            let authorization = format!("Bearer {}", session.access);
            let device = self
                .token
                .as_deref()
                .ok_or("게임에서 다시 로그인해 주세요.")?;
            let response = if remove {
                self.agent
                    .delete(url.as_str())
                    .header("Authorization", &authorization)
                    .header("X-Game-Token", device)
                    .call()
            } else {
                let photo = self
                    .photo
                    .as_ref()
                    .ok_or("먼저 사진 파일을 선택해 주세요.")?;
                self.agent
                    .post(url.as_str())
                    .header("Authorization", &authorization)
                    .header("X-Game-Token", device)
                    .header("Content-Type", photo.mime)
                    .send(photo.bytes.as_slice())
            }
            .map_err(|_| "사진을 전송하지 못했어요. 연결을 확인하고 다시 시도해 주세요.")?;
            let (status, value) = read_native_response(response)?;
            if status == 401 && attempt == 0 {
                self.refresh_native()?;
                continue;
            }
            let value = checked_response(status, value)?;
            if value["ok"] != true {
                return Err("사진 저장 응답을 확인해 주세요.".into());
            }
            self.photo = None;
            return Ok(crate::account::Response::Changed);
        }
        Err("게임에서 다시 로그인해 주세요.".into())
    }
    fn list(&self) -> Result<Vec<Room>, String> {
        let value = self.request("list", json!({}))?;
        let rooms: Vec<Room> = serde_json::from_value(value["rooms"].clone())
            .map_err(|_| "방 목록 형식이 올바르지 않아요.")?;
        if rooms.len() > 100
            || rooms.iter().any(|r| {
                normalize_code(&r.code).as_deref() != Some(&r.code)
                    || r.capacity != 12
                    || !(1..=12).contains(&r.players)
                    || r.title.chars().count() > 60
                    || r.host_name.chars().count() > 32
                    || !["fairy-kingdom-castle", "elf-city"].contains(&r.map_id.as_str())
                    || !["waiting", "playing", "finished"].contains(&r.state.as_str())
            })
        {
            return Err("방 목록 정보가 올바르지 않아요.".into());
        }
        Ok(rooms)
    }
    fn link(&mut self) -> Result<String, String> {
        let value = self.request("link_begin", json!({}))?;
        let token = value["token"]
            .as_str()
            .filter(|t| t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or("계정 연결 응답이 올바르지 않아요.")?;
        let code = value["code"]
            .as_str()
            .and_then(normalize_code)
            .ok_or("연결 코드가 올바르지 않아요.")?;
        self.token = Some(token.into());
        Ok(code)
    }
    fn link_status(&mut self) -> Result<LinkStatus, String> {
        let (status, value) = self.response("link_status", &mut json!({}))?;
        if status == 401 {
            self.token = None;
            self.native = None;
            self.photo = None;
            return Ok(LinkStatus::Expired);
        }
        parse_profile(checked_response(status, value)?)
    }
    fn avatar(&self, source: &str) -> Result<AvatarImage, String> {
        let url = avatar_url(source)?;
        // Public avatar requests never carry the device credential or browser cookies.
        let mut response = self
            .agent
            .get(url.as_str())
            .call()
            .map_err(|_| "프로필 사진을 불러오지 못했어요.")?;
        if !response.status().is_success() {
            return Err("프로필 사진을 불러오지 못했어요.".into());
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "프로필 사진을 읽지 못했어요.")?;
        decode_avatar(&bytes)
    }
    fn session(&mut self, action: &str, payload: Value) -> Result<(String, OnlineSession), String> {
        let value = self.request(action, payload)?;
        let code = value["code"]
            .as_str()
            .and_then(normalize_code)
            .ok_or("방 코드가 올바르지 않아요.")?;
        // Retain the code even when validation fails so Close/Drop can clean up.
        if action == "create" {
            self.hosted = Some(code.clone());
        }
        let session: OnlineSession = serde_json::from_value(value["session"].clone())
            .map_err(|_| "참가 정보가 올바르지 않아요.")?;
        session.validate(unix_now()).map_err(|e| e.to_string())?;
        if session.mode != if action == "create" { "host" } else { "join" } {
            return Err("방 참가 유형이 올바르지 않아요.".into());
        }
        Ok((code, session))
    }
    fn heartbeat(&self, map: String, players: usize, state: String) -> Result<(), String> {
        if let Some(code) = &self.hosted {
            self.request(
                "heartbeat",
                json!({"code":code,"map_id":map,"players":players.to_string(),"state":state}),
            )?;
        }
        Ok(())
    }
    fn close(&mut self) -> Result<(), String> {
        if let Some(code) = &self.hosted {
            self.request("close", json!({"code":code}))?;
        }
        self.hosted = None;
        Ok(())
    }
}

fn read_native_response(
    mut response: ureq::http::Response<ureq::Body>,
) -> Result<(u16, Value), String> {
    if response.status().is_redirection() {
        return Err("서버 주소가 변경됐어요. 게임을 업데이트해 주세요.".into());
    }
    let status = response.status().as_u16();
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(131073)
        .read_to_end(&mut bytes)
        .map_err(|_| "서버 응답을 읽지 못했어요.")?;
    if bytes.len() > 131072 {
        return Err("서버 응답이 너무 큽니다.".into());
    }
    let value = serde_json::from_slice(&bytes).map_err(|_| "서버 응답을 확인해 주세요.")?;
    Ok((status, value))
}

fn read_photo(path: &std::path::Path) -> Result<(PhotoFile, AvatarImage), String> {
    use image::ImageDecoder;
    let mut file = std::fs::File::open(path).map_err(|_| "사진 파일을 열 수 없어요.")?;
    let metadata = file.metadata().map_err(|_| "사진 파일을 확인해 주세요.")?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 2 * 1024 * 1024 {
        return Err("2MB 이하의 JPG·PNG·WebP 사진을 선택해 주세요.".into());
    }
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "사진을 읽지 못했어요.")?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("사진은 2MB 이하여야 합니다.".into());
    }
    let format = image::guess_format(&bytes).map_err(|_| "JPG·PNG·WebP 사진을 선택해 주세요.")?;
    let mime = match format {
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::WebP => "image/webp",
        _ => return Err("JPG·PNG·WebP 사진을 선택해 주세요.".into()),
    };
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(&bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16000);
    limits.max_image_height = Some(16000);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|_| "사진을 읽지 못했어요.")?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > 16000000 {
        return Err("사진은 1,600만 화소 이하여야 합니다.".into());
    }
    let orientation = decoder
        .orientation()
        .map_err(|_| "사진 방향을 읽지 못했어요.")?;
    let mut picture =
        image::DynamicImage::from_decoder(decoder).map_err(|_| "사진을 읽지 못했어요.")?;
    picture.apply_orientation(orientation);
    let picture = picture
        .resize_to_fill(256, 256, image::imageops::FilterType::Triangle)
        .into_rgba8();
    let preview = AvatarImage {
        width: picture.width(),
        height: picture.height(),
        pixels: picture.into_raw(),
    };
    Ok((PhotoFile { bytes, mime }, preview))
}

fn checked_response(status: u16, value: Value) -> Result<Value, String> {
    if !(200..300).contains(&status) {
        return Err(value["error"]
            .as_str()
            .unwrap_or("방 서비스를 이용할 수 없어요.")
            .chars()
            .take(180)
            .collect());
    }
    Ok(value)
}

fn parse_profile(value: Value) -> Result<LinkStatus, String> {
    match value["connected"].as_bool() {
        Some(false) => Ok(LinkStatus::Pending),
        Some(true) => {
            let profile: PlayerProfile =
                serde_json::from_value(value).map_err(|_| "프로필을 확인해 주세요.")?;
            if profile.display_name.trim().is_empty()
                || profile.display_name.chars().count() > 32
                || profile.display_name.chars().any(char::is_control)
                || profile
                    .handle
                    .as_ref()
                    .is_some_and(|s| s.len() > 32 || s.chars().any(char::is_control))
            {
                return Err("프로필을 확인해 주세요.".into());
            }
            Ok(LinkStatus::Connected(profile))
        }
        None => Err("계정 연결 응답이 올바르지 않아요.".into()),
    }
}

fn avatar_url(value: &str) -> Result<url::Url, String> {
    let url = url::Url::parse(value).map_err(|_| "프로필 사진 주소를 확인해 주세요.")?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if value.len() > 2048
        || (url.scheme() != "https" && !(url.scheme() == "http" && local))
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("프로필 사진 주소를 확인해 주세요.".into());
    }
    Ok(url)
}

fn decode_avatar(bytes: &[u8]) -> Result<AvatarImage, String> {
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("프로필 사진이 너무 큽니다.".into());
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "프로필 사진 형식을 확인해 주세요.")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(2048);
    limits.max_image_height = Some(2048);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| "프로필 사진을 읽지 못했어요.")?
        .resize_to_fill(128, 128, image::imageops::FilterType::Triangle)
        .into_rgba8();
    Ok(AvatarImage {
        width: image.width(),
        height: image.height(),
        pixels: image.into_raw(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_login_refresh_photo_and_logout_keep_credentials_on_their_own_endpoints() {
        use std::{io::Write, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            for operation in ["login", "refresh", "upload", "remove", "logout"] {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut headers = vec![];
                let mut byte = [0];
                while !headers.ends_with(b"\r\n\r\n") {
                    socket.read_exact(&mut byte).unwrap();
                    headers.push(byte[0]);
                }
                let headers = String::from_utf8(headers).unwrap().to_ascii_lowercase();
                let length = headers
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("content-length:")
                            .map(|n| n.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                let mut body = vec![0; length];
                socket.read_exact(&mut body).unwrap();
                assert!(!headers.contains("cookie:") && !headers.contains("origin:"));
                let response = if matches!(operation, "upload" | "remove") {
                    assert!(headers.starts_with(if operation == "upload" {
                        "post /api/game/account/avatar "
                    } else {
                        "delete /api/game/account/avatar "
                    }));
                    assert!(headers.contains("authorization: bearer test-access-b"));
                    assert!(headers.contains(&format!("x-game-token: {}", "a".repeat(64))));
                    if operation == "upload" {
                        assert!(headers.contains("content-type: image/png"));
                        assert!(body.starts_with(b"\x89PNG"));
                    }
                    json!({"ok":true})
                } else {
                    assert!(headers.starts_with("post /api/game/auth "));
                    assert!(!headers.contains("authorization:"));
                    let request: Value = serde_json::from_slice(&body).unwrap();
                    assert_eq!(request["action"], operation);
                    match operation {
                        "login" => {
                            assert_eq!(request["email"], "player@example.test");
                            assert_eq!(request["password"], " untrimmed password ");
                            json!({"device_token":"a".repeat(64),"access_token":"test-access-a","refresh_token":"test-refresh-a","expires_in":3600,
                                "profile":{"display_name":"테스트","handle":"test_player","avatar":"mint","avatar_url":null}})
                        }
                        "refresh" => {
                            assert_eq!(request["refresh_token"], "test-refresh-a");
                            json!({"access_token":"test-access-b","refresh_token":"test-refresh-b","expires_in":3600})
                        }
                        _ => {
                            assert_eq!(request["device_token"], "a".repeat(64));
                            assert_eq!(request["refresh_token"], "test-refresh-b");
                            json!({"ok":true})
                        }
                    }
                };
                let body = response.to_string();
                write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
            }
        });
        let mut api = Api::new();
        api.base = checked_url(&format!("http://{address}"));
        assert_eq!(
            api.login("player@example.test".into(), " untrimmed password ".into())
                .unwrap()
                .display_name,
            "테스트"
        );
        let mut buffer = std::io::Cursor::new(vec![]);
        image::DynamicImage::new_rgb8(12, 8)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .unwrap();
        let path = std::env::temp_dir().join(format!("tsb-photo-test-{}.png", std::process::id()));
        std::fs::write(&path, buffer.into_inner()).unwrap();
        let preview = api.prepare_photo(&path).unwrap();
        assert_eq!((preview.width, preview.height), (256, 256));
        std::fs::remove_file(path).unwrap();
        api.native.as_mut().unwrap().expires = std::time::Instant::now();
        assert!(matches!(
            api.account(crate::account::Action::UploadPhoto).unwrap(),
            crate::account::Response::Changed
        ));
        assert!(api.photo.is_none());
        assert!(matches!(
            api.account(crate::account::Action::RemovePhoto).unwrap(),
            crate::account::Response::Changed
        ));
        api.logout();
        assert!(api.native.is_none() && api.token.is_none());
        server.join().unwrap();
    }
    #[test]
    fn login_and_signup_urls_keep_the_device_callback_on_the_portal_origin() {
        for (mode, path) in [(AuthMode::Login, "/login"), (AuthMode::Signup, "/signup")] {
            let url = authentication_url(
                checked_url("https://dotgsince2023.co.kr/play").unwrap(),
                mode,
                "ab12-34cd",
            )
            .unwrap();
            assert_eq!(url.host_str(), Some("www.dotgsince2023.co.kr"));
            assert_eq!(url.path(), path);
            assert_eq!(
                url.query_pairs().collect::<Vec<_>>(),
                vec![("next".into(), "/play?connect=AB1234CD".into())]
            );
        }
        assert!(authentication_url(
            checked_url("http://127.0.0.1:3000/play").unwrap(),
            AuthMode::Login,
            "https://evil.test"
        )
        .is_err());
    }
    #[test]
    fn profile_accepts_current_and_legacy_servers_without_losing_the_name() {
        for value in [
            json!({"connected":true,"display_name":"닷지 친구"}),
            json!({"connected":true,"display_name":"닷지 친구","handle":"dotg","avatar":"mint","avatar_url":"https://example.com/photo.webp"}),
        ] {
            let LinkStatus::Connected(profile) = parse_profile(value).unwrap() else {
                panic!("not linked")
            };
            assert_eq!(profile.display_name, "닷지 친구");
        }
        assert!(matches!(
            parse_profile(json!({"connected":false,"display_name":null})).unwrap(),
            LinkStatus::Pending
        ));
        for value in [
            json!({}),
            json!({"connected":true}),
            json!({"connected":true,"display_name":""}),
            json!({"connected":true,"display_name":"bad\nname"}),
        ] {
            assert!(parse_profile(value).is_err());
        }
    }
    #[test]
    fn avatars_decode_webp_and_bound_untrusted_images() {
        let source = image::RgbaImage::from_pixel(512, 512, image::Rgba([24, 180, 120, 255]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        source
            .write_to(&mut bytes, image::ImageFormat::WebP)
            .unwrap();
        let photo = decode_avatar(bytes.get_ref()).unwrap();
        assert_eq!(
            (photo.width, photo.height, photo.pixels.len()),
            (128, 128, 128 * 128 * 4)
        );
        assert_eq!(&photo.pixels[..4], &[24, 180, 120, 255]);
        assert!(decode_avatar(b"not an image").is_err());
        assert!(decode_avatar(&vec![0; 2 * 1024 * 1024 + 1]).is_err());
        let wide = image::RgbaImage::new(2049, 1);
        let mut bytes = std::io::Cursor::new(Vec::new());
        wide.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        assert!(decode_avatar(bytes.get_ref()).is_err());
        for url in [
            "file:///C:/secret",
            "http://example.com/photo",
            "https://user:pass@example.com/photo",
        ] {
            assert!(avatar_url(url).is_err());
        }
    }
    #[test]
    fn linked_profile_and_avatar_http_requests_keep_credentials_separate_and_expire() {
        use std::{io::Write, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let source = image::RgbaImage::from_pixel(8, 8, image::Rgba([24, 180, 120, 255]));
        let mut photo = std::io::Cursor::new(Vec::new());
        source
            .write_to(&mut photo, image::ImageFormat::WebP)
            .unwrap();
        let server = thread::spawn(move || {
            for step in 0..4 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut byte = [0];
                while !bytes.ends_with(b"\r\n\r\n") {
                    socket.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                }
                let headers = String::from_utf8(bytes).unwrap();
                assert!(!headers.contains("profile-test-token"));
                let (status, body) = if step == 1 {
                    assert!(headers.starts_with("GET /photo.webp "));
                    assert!(!headers.to_ascii_lowercase().contains("authorization:"));
                    assert!(!headers.to_ascii_lowercase().contains("cookie:"));
                    (200, photo.get_ref().clone())
                } else {
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|s| s.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    let mut body = vec![0; length];
                    socket.read_exact(&mut body).unwrap();
                    let request: Value = serde_json::from_slice(&body).unwrap();
                    assert_eq!(request["action"], "link_status");
                    assert_eq!(request["payload"]["token"], "profile-test-token");
                    if step == 3 {
                        (401, json!({"error":"expired"}).to_string().into_bytes())
                    } else {
                        (200, json!({"connected":true,"display_name":if step == 0 {"처음 이름"} else {"새 이름"},
                            "handle":"player","avatar":"mint","avatar_url":if step == 0 {Some(format!("http://{address}/photo.webp"))} else {None}}).to_string().into_bytes())
                    }
                };
                write!(
                    socket,
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                socket.write_all(&body).unwrap();
            }
        });
        let mut api = Api::new();
        api.base = checked_url(&format!("http://{address}"));
        api.token = Some("profile-test-token".into());
        let LinkStatus::Connected(profile) = api.link_status().unwrap() else {
            panic!("not linked")
        };
        assert_eq!(profile.display_name, "처음 이름");
        assert_eq!(
            api.avatar(profile.avatar_url.as_deref().unwrap())
                .unwrap()
                .pixels
                .len(),
            128 * 128 * 4
        );
        let LinkStatus::Connected(profile) = api.link_status().unwrap() else {
            panic!("not linked")
        };
        assert_eq!(profile.display_name, "새 이름");
        assert!(profile.avatar_url.is_none());
        assert!(matches!(api.link_status().unwrap(), LinkStatus::Expired));
        assert!(api.token.is_none());
        server.join().unwrap();
    }

    #[test]
    fn native_account_http_uses_device_body_and_drops_expired_credentials() {
        use crate::account::{Action, Response};
        use std::{io::Write, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            for action in ["dashboard", "request_friend", "search_players"] {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut byte = [0];
                while !bytes.ends_with(b"\r\n\r\n") {
                    socket.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                }
                let headers = String::from_utf8(bytes).unwrap().to_ascii_lowercase();
                assert!(headers.starts_with("post /api/game/account "));
                assert!(
                    !headers.contains("cookie:")
                        && !headers.contains("authorization:")
                        && !headers.contains("native-test-token")
                );
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length:")
                            .map(|s| s.trim().parse().unwrap())
                    })
                    .unwrap();
                let mut body = vec![0; length];
                socket.read_exact(&mut body).unwrap();
                let request: Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(request["action"], action);
                assert_eq!(request["payload"]["token"], "native-test-token");
                if action == "request_friend" {
                    assert_eq!(request["payload"]["handle"], "friend");
                }
                let (status, value) = match action {
                    "dashboard" => (
                        200,
                        json!({"profile":null,"relationships":[],"blocks":[],"rooms":[],"invites":[]}),
                    ),
                    "request_friend" => (200, json!({"ok":true})),
                    _ => (401, json!({"error":"expired"})),
                };
                let body = value.to_string();
                write!(
                    socket,
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        let mut api = Api::new();
        api.base = checked_url(&format!("http://{address}"));
        api.token = Some("native-test-token".into());
        assert!(matches!(
            api.account(Action::Dashboard).unwrap(),
            Response::Dashboard(_)
        ));
        assert!(matches!(
            api.account(Action::Request("friend".into())).unwrap(),
            Response::Changed
        ));
        assert!(matches!(
            api.account(Action::Search("친구".into())).unwrap(),
            Response::Expired
        ));
        assert!(api.token.is_none());
        assert!(matches!(
            api.account(Action::Dashboard).unwrap(),
            Response::Expired
        ));
        server.join().unwrap();
    }
    #[test]
    fn http_directory_lists_registers_updates_and_closes_without_leaking_credentials() {
        use std::{io::Write, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            for action in ["list", "create", "heartbeat", "close"] {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = vec![];
                let mut byte = [0];
                while !bytes.ends_with(b"\r\n\r\n") {
                    socket.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                }
                let headers = String::from_utf8(bytes).unwrap();
                assert!(headers.lines().next().unwrap().contains("/api/game/rooms"));
                let value = if action == "list" {
                    assert!(!headers.contains("native-test-token"));
                    json!({"rooms":[{"code":"ABCD1234","title":"테스트 방","host_name":"방장","map_id":"elf-city","players":2,"capacity":12,"state":"waiting"}]})
                } else {
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|s| s.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    let mut body = vec![0; length];
                    socket.read_exact(&mut body).unwrap();
                    let request: Value = serde_json::from_slice(&body).unwrap();
                    assert_eq!(request["action"], action);
                    assert_eq!(request["payload"]["token"], "native-test-token");
                    if action == "create" {
                        json!({"code":"ABCD1234","session":{"version":1,"mode":"host","transport":"direct_udp","session_id":"11111111-1111-4111-8111-111111111111","address":"127.0.0.1:28080","expires_at":unix_now()+7200,"profile":{"id":"22222222-2222-4222-8222-222222222222","display_name":"방장"},"host_key":"a".repeat(64)}})
                    } else {
                        assert_eq!(request["payload"]["code"], "ABCD1234");
                        json!({"ok":true})
                    }
                };
                let body = value.to_string();
                write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
            }
        });
        let mut api = Api::new();
        api.base = checked_url(&format!("http://{address}"));
        api.token = Some("native-test-token".into());
        assert_eq!(api.list().unwrap()[0].title, "테스트 방");
        let (code, session) = api
            .session(
                "create",
                json!({"title":"테스트 방","endpoint":"127.0.0.1:28080","map_id":"elf-city"}),
            )
            .unwrap();
        assert_eq!(code, "ABCD1234");
        assert!(session.host_gate().is_ok());
        api.heartbeat("elf-city".into(), 2, "waiting".into())
            .unwrap();
        api.close().unwrap();
        assert!(api.hosted.is_none());
        server.join().unwrap();
    }
    #[test]
    fn production_portal_uses_the_canonical_host_even_for_old_overrides() {
        for value in [
            DEFAULT_PORTAL_URL,
            "https://dotgsince2023.co.kr/play",
            "https://dotgsince2023.co.kr:443/old?connect=ABC12345#profile",
        ] {
            assert_eq!(checked_url(value).unwrap().as_str(), DEFAULT_PORTAL_URL);
        }
        for (value, expected) in [
            ("https://example.com/old", "https://example.com/play"),
            ("http://localhost:3000/old", "http://localhost:3000/play"),
            (
                "https://dotgsince2023.co.kr:8443/old",
                "https://dotgsince2023.co.kr:8443/play",
            ),
        ] {
            assert_eq!(checked_url(value).unwrap().as_str(), expected);
        }
    }
    #[test]
    fn redirect_response_reports_an_address_change_instead_of_invalid_json() {
        use std::{io::Write, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut headers = Vec::new();
            let mut byte = [0];
            while !headers.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).unwrap();
                headers.push(byte[0]);
            }
            socket.write_all(b"HTTP/1.1 308 Permanent Redirect\r\nLocation: https://example.invalid/api/game/rooms\r\nContent-Type: text/plain\r\nContent-Length: 15\r\nConnection: close\r\n\r\nRedirecting...\n").unwrap();
        });
        let mut api = Api::new();
        api.base = checked_url(&format!("http://{address}"));
        assert!(api
            .list()
            .unwrap_err()
            .contains("웹사이트 주소가 변경됐어요"));
        server.join().unwrap();
    }
    #[test]
    #[ignore = "Read-only production API check; requires internet access"]
    fn production_room_directory_returns_a_valid_list() {
        let mut api = Api::new();
        api.base = checked_url(DEFAULT_PORTAL_URL);
        api.list().expect("Production room list must be valid");
    }
    #[test]
    fn codes_and_portal_urls_are_normalized_without_allowing_insecure_remote_credentials() {
        assert_eq!(normalize_code("ab12-34ef"), Some("AB1234EF".into()));
        for code in ["", "123", "ZZ123456", "123456789", "１２３４５６７８"] {
            assert!(normalize_code(code).is_none());
        }
        assert!(checked_url("https://example.com/play#friends").is_ok());
        assert!(checked_url("https://dotgsince2023.co.kr/play").is_ok());
        for value in [
            "http://dotgsince2023.co.kr/play",
            "http://example.com",
            "https://user:secret@example.com",
            "file:///tmp/test",
        ] {
            assert!(checked_url(value).is_err());
        }
    }
}
