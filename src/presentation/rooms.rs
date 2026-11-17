//! Home -> room browser -> match lobby. Network work runs off the render thread.
use super::*;
use fyrox::gui::canvas::CanvasBuilder;
use std::time::Duration;
use trickcal_shooting_balddagu::room_directory::{
    authentication_url, normalize_code, portal_url, AuthMode, AvatarImage, Command, Directory,
    LinkStatus, PlayerProfile, Reply, Room,
};

const PAGE_SIZE: usize = 6;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Home,
    Create,
    Join,
    Refresh,
    Previous,
    Next,
    Resume,
    Leave,
    Row(usize),
    Title,
    Address,
    Confirm,
    Cancel,
}
impl Action {
    fn bounds(self) -> (f32, f32, f32, f32) {
        match self {
            Self::Home => (44., 714., 152., 56.),
            Self::Create | Self::Resume => (840., 134., 184., 54.),
            Self::Join | Self::Leave => (1040., 134., 196., 54.),
            Self::Refresh => (650., 134., 174., 54.),
            Self::Previous => (930., 640., 140., 44.),
            Self::Next => (1086., 640., 150., 44.),
            Self::Row(i) => (44., 246. + i as f32 * 62., 1192., 54.),
            Self::Title => (390., 320., 500., 54.),
            Self::Address => (390., 414., 500., 54.),
            Self::Confirm => (650., 538., 240., 52.),
            Self::Cancel => (390., 538., 240., 52.),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Modal {
    None,
    Create,
    Join,
}
#[derive(Debug)]
pub(super) struct Browser {
    pub open: bool,
    api: Option<Directory>,
    rooms: Vec<Room>,
    page: usize,
    focus: Action,
    modal: Modal,
    title: ChatInput,
    address: ChatInput,
    notice: String,
    list_error: String,
    loading: bool,
    busy: bool,
    ime_active: bool,
    pub(super) linked: Option<PlayerProfile>,
    pub(super) avatar: Option<TextureResource>,
    avatar_loading: bool,
    link_fragment: Option<&'static str>,
    auth_mode: Option<AuthMode>,
    link_epoch: u64,
    link_code: Option<String>,
    link_started: Instant,
    link_poll_pending: bool,
    last_poll: Instant,
    last_refresh: Instant,
    last_heartbeat: Instant,
    heartbeat_pending: bool,
    sync_warning: String,
    pending_server: Option<Box<Server>>,
    pending_title: Option<String>,
    create_started: Instant,
    pub room_code: Option<String>,
    hosting: bool,
}
impl Default for Browser {
    fn default() -> Self {
        Self {
            open: false,
            api: None,
            rooms: vec![],
            page: 0,
            focus: Action::Create,
            modal: Modal::None,
            title: ChatInput::default(),
            address: ChatInput::default(),
            notice: String::new(),
            list_error: String::new(),
            loading: false,
            busy: false,
            ime_active: false,
            linked: None,
            avatar: None,
            avatar_loading: false,
            link_fragment: None,
            auth_mode: None,
            link_epoch: 0,
            link_code: None,
            link_started: Instant::now(),
            link_poll_pending: false,
            last_poll: Instant::now(),
            last_refresh: Instant::now() - Duration::from_secs(20),
            last_heartbeat: Instant::now() - Duration::from_secs(20),
            heartbeat_pending: false,
            sync_warning: String::new(),
            pending_server: None,
            pending_title: None,
            create_started: Instant::now(),
            room_code: None,
            hosting: false,
        }
    }
}
impl Browser {
    pub(super) fn authenticate(&mut self, email: String, password: String) {
        if self.busy {
            return;
        }
        self.cancel_login();
        self.busy = true;
        self.notice = "로그인 중…".into();
        self.send(Command::Login {
            epoch: self.link_epoch,
            email,
            password,
        });
    }
    pub(super) fn prepare_photo(&mut self, request: u64, path: std::path::PathBuf) {
        self.send(Command::PreparePhoto {
            epoch: self.link_epoch,
            request,
            path,
        });
    }
    pub(super) fn discard_photo(&mut self) {
        self.send(Command::DiscardPhoto);
    }
    pub(super) fn refresh_profile(&mut self) {
        if (self.linked.is_some() || self.link_code.is_some()) && !self.link_poll_pending {
            self.last_poll = Instant::now();
            self.link_poll_pending = true;
            self.send(Command::LinkStatus(self.link_epoch));
        }
    }
    pub(super) fn open_account(&mut self, fragment: Option<&'static str>) {
        self.link_fragment = fragment;
        if self.linked.is_some() || self.link_code.is_some() {
            self.launch_account_browser();
        } else if !self.busy {
            self.busy = true;
            self.notice = "로그인을 준비하고 있어요.".into();
            self.link_epoch += 1;
            self.send(Command::Link(self.link_epoch));
        }
    }
    pub(super) fn begin_login(&mut self, mode: AuthMode) {
        self.auth_mode = Some(mode);
        self.open_account(None);
    }
    fn launch_account_browser(&mut self) {
        let result = portal_url().and_then(|mut url| {
            if let Some(code) = &self.link_code {
                if let Some(mode) = self.auth_mode {
                    return authentication_url(url, mode, code);
                }
                url.query_pairs_mut().append_pair("connect", code);
            }
            url.set_fragment(self.link_fragment);
            Ok(url)
        });
        match result {
            Ok(url) => {
                if open::that_detached(url.as_str()).is_err() {
                    self.notice =
                        "브라우저를 열지 못했어요. DOTG 웹 라운지에서 아래 코드를 입력해 주세요."
                            .into();
                }
            }
            Err(e) => self.notice = e,
        }
    }
    pub(super) fn cancel_login(&mut self) {
        // Invalidate in-flight replies before a guest choice or a new login attempt.
        self.link_epoch += 1;
        self.link_code = None;
        self.linked = None;
        self.avatar = None;
        self.avatar_loading = false;
        self.link_poll_pending = false;
        self.auth_mode = None;
        self.busy = false;
        self.notice.clear();
        if let Some(api) = &self.api {
            api.send(Command::ForgetAccount);
        }
    }
    pub(super) fn login_notice(&self) -> &str {
        &self.notice
    }
    pub(super) fn login_code(&self) -> Option<&str> {
        self.link_code.as_deref()
    }
    pub(super) fn login_busy(&self) -> bool {
        self.busy
    }
    pub(super) fn apply_link_status(&mut self, status: LinkStatus) {
        match status {
            LinkStatus::Connected(profile) => {
                if self.linked.is_none() {
                    self.notice = format!(
                        "{} 님, 연결됐어요. 방을 만들거나 참여해 보세요.",
                        profile.display_name
                    );
                }
                if self.linked.as_ref().and_then(|p| p.avatar_url.as_ref())
                    != profile.avatar_url.as_ref()
                {
                    self.avatar = None;
                    self.avatar_loading = false;
                }
                self.linked = Some(profile);
                self.link_code = None;
            }
            LinkStatus::Pending | LinkStatus::Expired => {
                self.linked = None;
                self.avatar = None;
                self.avatar_loading = false;
                if matches!(status, LinkStatus::Expired) {
                    self.link_code = None;
                    self.notice = "로그인이 만료됐어요. 다시 로그인해 주세요.".into();
                }
            }
        }
    }
    fn apply_avatar(&mut self, url: &str, result: Result<AvatarImage, String>) {
        if self.linked.as_ref().and_then(|p| p.avatar_url.as_deref()) != Some(url) {
            return;
        }
        self.avatar_loading = false;
        self.avatar = result.ok().and_then(|image| {
            Texture::from_bytes(
                TextureKind::Rectangle {
                    width: image.width,
                    height: image.height,
                },
                TexturePixelKind::RGBA8,
                image.pixels,
            )
            .map(TextureResource::new_embedded)
        });
    }
    pub(super) fn profile_detail(&self, korean: bool) -> String {
        if let Some(profile) = &self.linked {
            if let Some(handle) = profile.handle.as_deref().filter(|s| !s.is_empty()) {
                return format!("@{handle}");
            }
            return if korean {
                "나의 프로필"
            } else {
                "MY PROFILE"
            }
            .into();
        }
        if let Some(code) = &self.link_code {
            return format!("{} {code}", if korean { "연결 코드" } else { "LINK CODE" });
        }
        if korean { "게스트" } else { "GUEST" }.into()
    }
    fn send(&mut self, command: Command) {
        self.api.get_or_insert_with(Directory::new).send(command);
    }
    pub(super) fn account_epoch(&self) -> u64 {
        self.link_epoch
    }
    pub(super) fn account_request(
        &mut self,
        request: u64,
        action: trickcal_shooting_balddagu::account::Action,
    ) {
        self.send(Command::Account {
            epoch: self.link_epoch,
            request,
            action,
        });
    }
    pub fn refresh(&mut self) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.last_refresh = Instant::now();
        self.send(Command::List);
    }
    pub fn close_room(&mut self) {
        if self.hosting {
            self.send(Command::Close);
        }
        self.room_code = None;
        self.hosting = false;
        self.sync_warning.clear();
    }
    pub fn status(&self) -> String {
        match &self.room_code {
            Some(code) if self.sync_warning.is_empty() => {
                format!("방 코드  {code}  /  웹 방 목록에 연결됨")
            }
            Some(code) => format!("방 코드  {code}  /  {}", self.sync_warning),
            None => String::new(),
        }
    }
    fn actions(&self, active: bool) -> Vec<Action> {
        match self.modal {
            Modal::Create if !lan_mode() => vec![Action::Title, Action::Confirm, Action::Cancel],
            Modal::Create => vec![
                Action::Title,
                Action::Address,
                Action::Confirm,
                Action::Cancel,
            ],
            Modal::Join => vec![Action::Address, Action::Confirm, Action::Cancel],
            Modal::None => {
                let mut actions = vec![Action::Home, Action::Refresh];
                if active {
                    actions.extend([Action::Resume, Action::Leave]);
                } else {
                    actions.extend([Action::Create, Action::Join]);
                    for i in 0..PAGE_SIZE {
                        if self
                            .rooms
                            .get(self.page * PAGE_SIZE + i)
                            .is_some_and(|r| r.players < r.capacity)
                        {
                            actions.push(Action::Row(i));
                        }
                    }
                }
                if self.page > 0 {
                    actions.push(Action::Previous);
                }
                if (self.page + 1) * PAGE_SIZE < self.rooms.len() {
                    actions.push(Action::Next);
                }
                actions
            }
        }
    }
    fn hit(&self, active: bool, size: Vector2<f32>, cursor: Vector2<f32>) -> Option<Action> {
        self.actions(active)
            .into_iter()
            .find(|a| selection::contains(cursor, selection::rect(size.x, size.y, a.bounds())))
    }
}
fn lan_mode() -> bool {
    !cfg!(feature = "eos") || std::env::args().any(|a| a == "--lan")
}
fn default_endpoint() -> String {
    if let Ok(addr) = std::env::var("DOTG_HOST_ADDRESS") {
        return addr;
    }
    let ip = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("8.8.8.8:80")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".into());
    format!("{ip}:28080")
}
impl Runtime {
    fn browser_action(&mut self, action: Action, window: &Window) {
        if action == Action::Cancel && self.browser.pending_title.is_some() {
            self.browser.pending_title = None;
            self.browser.pending_server = None;
            self.browser.busy = false;
        }
        if self.browser.busy
            || !self
                .browser
                .actions(self.lobby.room_active)
                .contains(&action)
        {
            return;
        }
        match action {
            Action::Home => {
                self.lobby.open = false;
                self.home_open = true;
            }
            Action::Resume => {
                self.browser.open = false;
            }
            Action::Leave => {
                self.replace_room(Session::Solo(Box::default()), false);
                self.browser.refresh();
            }
            Action::Refresh => self.browser.refresh(),
            Action::Previous => self.browser.page = self.browser.page.saturating_sub(1),
            Action::Next => self.browser.page += 1,
            Action::Create => {
                // A web account is only needed to publish a listed room. A
                // private EOS room must also work from the normal game UI.
                if self.lobby.solo || self.browser.linked.is_none() {
                    if let Err(error) = self.create_room() {
                        self.browser.notice = error;
                    }
                    return;
                }
                self.browser.modal = Modal::Create;
                self.browser.focus = Action::Title;
                self.browser.notice.clear();
                self.browser.title.clear();
                self.browser.title.insert(&format!(
                    "{}의 방",
                    self.browser
                        .linked
                        .as_ref()
                        .map_or("우리", |p| &p.display_name)
                ));
                self.browser.address.clear();
                if lan_mode() {
                    self.browser.address.insert(&default_endpoint());
                }
            }
            Action::Join => {
                self.browser.modal = Modal::Join;
                self.browser.focus = Action::Address;
                self.browser.address.clear();
                self.browser.notice.clear();
            }
            Action::Row(i) => {
                let code = self.browser.rooms[self.browser.page * PAGE_SIZE + i]
                    .code
                    .clone();
                self.browser_join(&code);
            }
            Action::Cancel => {
                self.browser.modal = Modal::None;
                self.browser.notice.clear();
                self.browser.focus = Action::Create;
            }
            Action::Title | Action::Address => self.browser.focus = action,
            Action::Confirm => {
                if self.browser.modal == Modal::Create {
                    let title = self.browser.title.draft.trim().to_string();
                    let endpoint = self.browser.address.draft.trim().to_string();
                    if title.is_empty() || title.chars().count() > 60 {
                        self.browser.notice = "방 이름을 1~60자로 입력해 주세요.".into();
                        return;
                    }
                    if !lan_mode() {
                        match Server::host_eos(None, true) {
                            Ok(mut server) => {
                                server.world.select_map(self.session.world().map_id);
                                self.browser.pending_server = Some(Box::new(server));
                                self.browser.pending_title = Some(title);
                                self.browser.create_started = Instant::now();
                                self.browser.busy = true;
                                self.browser.notice =
                                    "EOS 방을 준비하고 있어요. 잠시 기다려 주세요.".into();
                            }
                            Err(e) => {
                                self.browser.notice = format!("EOS 방을 만들지 못했어요. {e}")
                            }
                        }
                        return;
                    }
                    // Bind first: a port conflict must not create a room on the website.
                    let Some((host, port)) = endpoint.rsplit_once(':') else {
                        self.browser.notice = "접속 주소:포트 형식으로 입력해 주세요.".into();
                        return;
                    };
                    let Ok(port) = port.parse::<u16>() else {
                        self.browser.notice = "포트를 확인해 주세요.".into();
                        return;
                    };
                    if port == 0 || host.is_empty() || ["0.0.0.0", "[::]"].contains(&host) {
                        self.browser.notice =
                            "다른 PC에서 접속할 수 있는 주소를 입력해 주세요.".into();
                        return;
                    }
                    let bind = format!(
                        "{}:{port}",
                        if host.starts_with('[') {
                            "[::]"
                        } else {
                            "0.0.0.0"
                        }
                    );
                    match Server::bind(bind, true) {
                        Ok(mut server) => {
                            server.world.select_map(self.session.world().map_id);
                            self.browser.pending_server = Some(Box::new(server));
                        }
                        Err(_) => {
                            self.browser.notice =
                                "포트를 열지 못했어요. 실행 중인 다른 게임과 포트를 확인해 주세요."
                                    .into();
                            return;
                        }
                    }
                    self.browser.busy = true;
                    self.browser.notice = "웹에 방을 등록하고 있어요.".into();
                    self.browser.send(Command::Create {
                        title,
                        endpoint,
                        map: self.session.world().map_id.slug().into(),
                    });
                } else if self.browser.modal == Modal::Join {
                    let value = self.browser.address.draft.clone();
                    self.browser_join(&value);
                }
            }
        }
        self.keys = Keys::default();
        set_cursor(window, false);
    }
    fn browser_join(&mut self, value: &str) {
        if value.trim().starts_with("eos:") {
            match Client::connect_address(value.trim()) {
                Ok(client) => {
                    self.replace_room(Session::Join(Box::new(client)), true);
                    self.browser.modal = Modal::None;
                }
                Err(e) => self.browser.notice = format!("EOS 방 코드를 확인해 주세요. {e}"),
            }
        } else if let Some(code) = normalize_code(value) {
            if self.browser.linked.is_none() {
                self.browser.notice = "웹 방은 로그인이 필요해요. 홈의 프로필에서 로그인하거나 EOS 방 코드를 입력해 주세요.".into();
                return;
            }
            self.browser.busy = true;
            self.browser.notice = "방에 참여하고 있어요.".into();
            self.browser.send(Command::Join(code));
        } else {
            match lobby::parse_address(value) {
                Ok(addr) => match Client::connect(addr) {
                    Ok(client) => {
                        self.replace_room(Session::Join(Box::new(client)), true);
                        self.browser.modal = Modal::None;
                    }
                    Err(_) => {
                        self.browser.notice =
                            "호스트에 연결하지 못했어요. 주소와 포트를 확인해 주세요.".into()
                    }
                },
                Err(_) => {
                    self.browser.notice =
                        "받은 EOS 방 코드 전체를 붙여 넣어 주세요. 웹 방 코드는 8자리예요.".into()
                }
            }
        }
    }
}
pub(super) fn update(rt: &mut Runtime) {
    if rt.browser.pending_title.is_some()
        && rt.browser.create_started.elapsed() > Duration::from_secs(75)
    {
        rt.browser.pending_title = None;
        rt.browser.pending_server = None;
        rt.browser.busy = false;
        rt.browser.notice = "EOS 방 연결 시간이 초과됐어요. 다시 시도해 주세요.".into();
    }
    if rt.browser.pending_title.is_some() {
        let prepared = rt
            .browser
            .pending_server
            .as_mut()
            .map(|server| server.update(Input::default()).map(|()| server.room_code()));
        match prepared {
            Some(Ok(Some(endpoint))) => {
                let title = rt.browser.pending_title.take().unwrap();
                rt.browser.notice = "웹에 EOS 방을 등록하고 있어요.".into();
                rt.browser.send(Command::Create {
                    title,
                    endpoint,
                    map: rt.session.world().map_id.slug().into(),
                });
            }
            Some(Err(error)) => {
                rt.browser.pending_title = None;
                rt.browser.pending_server = None;
                rt.browser.busy = false;
                rt.browser.notice = format!("EOS 방 생성 실패: {error}");
            }
            _ => (),
        }
    }
    loop {
        let reply = rt
            .browser
            .api
            .as_ref()
            .and_then(|api| api.replies.try_recv().ok());
        let Some(reply) = reply else {
            break;
        };
        match reply {
            Reply::LoggedIn(epoch, result) => {
                if epoch != rt.browser.link_epoch {
                    continue;
                }
                rt.browser.busy = false;
                match result {
                    Ok(profile) => {
                        rt.browser.apply_link_status(LinkStatus::Connected(profile));
                        rt.browser.refresh_profile();
                    }
                    Err(error) => rt.browser.notice = error,
                }
            }
            Reply::PhotoPrepared {
                epoch,
                request,
                result,
            } => {
                if epoch == rt.browser.link_epoch {
                    account::photo_prepared(rt, request, result);
                }
            }
            Reply::Account {
                epoch,
                request,
                result,
            } => {
                if epoch == rt.browser.link_epoch {
                    account::apply_reply(rt, request, result);
                }
            }
            Reply::Rooms(result) => {
                rt.browser.loading = false;
                match result {
                    Ok(rooms) => {
                        rt.browser.rooms = rooms;
                        rt.browser.page = rt
                            .browser
                            .page
                            .min(rt.browser.rooms.len().saturating_sub(1) / PAGE_SIZE);
                        rt.browser.list_error.clear();
                    }
                    Err(e) => {
                        rt.browser.rooms.clear();
                        rt.browser.page = 0;
                        rt.browser.list_error = e;
                    }
                }
            }
            Reply::Link(epoch, result) => {
                if epoch != rt.browser.link_epoch {
                    continue;
                }
                rt.browser.busy = false;
                match result {
                    Ok(code) => {
                        rt.browser.link_code = Some(code.clone());
                        rt.browser.link_started = Instant::now();
                        rt.browser.linked = None;
                        rt.browser.avatar = None;
                        rt.browser.avatar_loading = false;
                        rt.browser.notice =
                            format!("웹에서 연결 코드 {code}를 확인하고 승인해 주세요.");
                        rt.browser.launch_account_browser();
                    }
                    Err(e) => rt.browser.notice = e,
                }
            }
            Reply::Linked(epoch, result) => {
                if epoch != rt.browser.link_epoch {
                    continue;
                }
                rt.browser.link_poll_pending = false;
                match result {
                    Ok(status) => {
                        rt.browser.apply_link_status(status);
                        if !rt.browser.avatar_loading && rt.browser.avatar.is_none() {
                            if let Some(url) = rt
                                .browser
                                .linked
                                .as_ref()
                                .and_then(|p| p.avatar_url.clone())
                            {
                                rt.browser.avatar_loading = true;
                                rt.browser.send(Command::Avatar(url));
                            }
                        }
                    }
                    Err(e) => {
                        rt.browser.notice = e;
                    }
                }
            }
            Reply::Avatar { url, result } => rt.browser.apply_avatar(&url, result),
            Reply::Session(result) => {
                rt.browser.busy = false;
                let result = result.and_then(|(code, online)| {
                    let hosting = online.mode == "host";
                    let session = if hosting {
                        let mut server = rt
                            .browser
                            .pending_server
                            .take()
                            .ok_or("방 생성을 다시 시도해 주세요.")?;
                        if server.is_eos()
                            && (online.transport != "eos_p2p"
                                || server.room_code().as_deref() != Some(&online.address))
                        {
                            return Err("웹에서 받은 EOS 방 정보가 현재 방과 달라요.".into());
                        }
                        server.restrict_to_invites(
                            online.host_gate().map_err(|e| e.to_string())?,
                            online.profile,
                        );
                        Session::Host(server)
                    } else {
                        Session::Join(Box::new(
                            Client::connect_address(&online.address)
                                .and_then(|c| {
                                    c.with_invitation(online.ticket.ok_or_else(|| {
                                        std::io::Error::other("참가권을 확인해 주세요.")
                                    })?)
                                })
                                .map_err(|e| format!("호스트에 연결하지 못했어요. {e}"))?,
                        ))
                    };
                    rt.replace_room(session, true);
                    rt.browser.room_code = Some(code);
                    rt.browser.hosting = hosting;
                    rt.browser.modal = Modal::None;
                    rt.browser.notice.clear();
                    rt.browser.last_heartbeat = Instant::now() - Duration::from_secs(20);
                    Ok(())
                });
                if let Err(e) = result {
                    rt.browser.pending_server = None;
                    rt.browser.send(Command::Close);
                    rt.browser.notice = e;
                }
            }
            Reply::Heartbeat(result) => {
                rt.browser.heartbeat_pending = false;
                rt.browser.sync_warning = match result {
                    Ok(()) => String::new(),
                    Err(_) => "웹 목록 갱신 실패 · 자동 재시도 중".into(),
                };
            }
            Reply::Closed(result) => {
                if result.is_err() && rt.browser.open {
                    rt.browser.notice =
                        "웹 목록 종료 알림을 보내지 못했어요. 최대 60초 뒤 자동으로 내려갑니다."
                            .into();
                }
            }
        }
    }
    if rt.browser.modal == Modal::None
        && !rt
            .browser
            .actions(rt.lobby.room_active)
            .contains(&rt.browser.focus)
    {
        rt.browser.focus = if rt.lobby.room_active {
            Action::Resume
        } else {
            Action::Create
        };
    }
    if rt.lobby.open
        && rt.browser.open
        && !rt.lobby.solo
        && rt.browser.last_refresh.elapsed() >= Duration::from_secs(15)
    {
        rt.browser.refresh();
    }
    if rt.browser.link_code.is_some() {
        if rt.browser.link_started.elapsed() >= Duration::from_secs(600) {
            rt.browser.link_code = None;
            rt.browser.notice = "로그인 코드가 만료됐어요. 다시 시도해 주세요.".into();
        } else if !rt.browser.link_poll_pending
            && rt.browser.last_poll.elapsed() >= Duration::from_secs(3)
        {
            rt.browser.refresh_profile();
        }
    } else if rt.browser.linked.is_some()
        && !rt.browser.link_poll_pending
        && rt.browser.last_poll.elapsed() >= Duration::from_secs(30)
    {
        rt.browser.refresh_profile();
    }
    if rt.browser.hosting
        && rt.error.is_none()
        && !rt.browser.heartbeat_pending
        && rt.browser.last_heartbeat.elapsed() >= Duration::from_secs(15)
    {
        if let Session::Host(server) = &rt.session {
            let command = Command::Heartbeat {
                map: server.world.map_id.slug().into(),
                players: server.peer_count() + 1,
                state: match server.world.phase {
                    Phase::Lobby => "waiting",
                    Phase::Finished => "finished",
                    _ => "playing",
                }
                .into(),
            };
            rt.browser.last_heartbeat = Instant::now();
            rt.browser.heartbeat_pending = true;
            rt.browser.send(command);
        }
    }
}
fn insert_text(browser: &mut Browser, value: &str) {
    let title = browser.modal == Modal::Create && browser.focus == Action::Title;
    let input = if title {
        &mut browser.title
    } else {
        &mut browser.address
    };
    let value: String = value
        .chars()
        .filter(|c| !c.is_control() && (title || c.is_ascii_alphanumeric() || ".:[]-".contains(*c)))
        .collect();
    let limit = if title { 60 } else { 260 };
    if input.draft.chars().count() + value.chars().count() <= limit {
        input.insert(&value);
    }
}
pub(super) fn on_event(rt: &mut Runtime, event: &Event<()>, context: &PluginContext) {
    let window = &context.graphics_context.as_initialized_ref().window;
    let size = context.user_interfaces.first().screen_size();
    let mut action = None;
    match event {
        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            rt.cursor = Vector2::new(position.x as f32, position.y as f32);
            if rt.browser.modal == Modal::None {
                if let Some(a) = rt.browser.hit(rt.lobby.room_active, size, rt.cursor) {
                    rt.browser.focus = a;
                }
            }
        }
        Event::WindowEvent {
            event:
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } => action = rt.browser.hit(rt.lobby.room_active, size, rt.cursor),
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } if event.state == ElementState::Pressed => {
            let editing = rt.browser.modal != Modal::None
                && matches!(rt.browser.focus, Action::Title | Action::Address)
                && !rt.browser.busy;
            let input = if rt.browser.focus == Action::Title {
                &mut rt.browser.title
            } else {
                &mut rt.browser.address
            };
            if editing && !input.preedit.is_empty() {
                return;
            }
            match event.physical_key {
                PhysicalKey::Code(KeyCode::KeyV)
                    if editing
                        && rt.browser.focus == Action::Address
                        && rt.modifiers.control_key()
                        && !event.repeat =>
                {
                    rt.browser.notice = lobby::paste_room_address(
                        context.user_interfaces.first(),
                        &mut rt.browser.address,
                    )
                    .err()
                    .unwrap_or_default()
                    .into();
                }
                PhysicalKey::Code(KeyCode::F10) => context.loop_controller.exit(),
                PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                    action = Some(if rt.browser.modal == Modal::None {
                        Action::Home
                    } else {
                        Action::Cancel
                    })
                }
                PhysicalKey::Code(KeyCode::F5) if rt.browser.modal == Modal::None => {
                    action = Some(Action::Refresh)
                }
                PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) if !event.repeat => {
                    action = Some(if editing {
                        Action::Confirm
                    } else {
                        rt.browser.focus
                    })
                }
                PhysicalKey::Code(KeyCode::Tab | KeyCode::ArrowDown | KeyCode::ArrowUp)
                    if !event.repeat =>
                {
                    let actions = rt.browser.actions(rt.lobby.room_active);
                    let n = actions.len();
                    let index = actions
                        .iter()
                        .position(|a| *a == rt.browser.focus)
                        .unwrap_or(n - 1);
                    rt.browser.focus = actions[(index
                        + if event.physical_key == PhysicalKey::Code(KeyCode::ArrowUp) {
                            n - 1
                        } else {
                            1
                        })
                        % n];
                }
                PhysicalKey::Code(KeyCode::Backspace) if editing => input.backspace(),
                PhysicalKey::Code(KeyCode::Delete) if editing => input.delete(),
                PhysicalKey::Code(KeyCode::ArrowLeft) if editing => input.left(),
                PhysicalKey::Code(KeyCode::ArrowRight) if editing => input.right(),
                PhysicalKey::Code(KeyCode::Home) if editing => input.caret = 0,
                PhysicalKey::Code(KeyCode::End) if editing => input.caret = input.draft.len(),
                _ if editing && !rt.modifiers.control_key() && !rt.modifiers.super_key() => {
                    if let Some(value) = &event.text {
                        insert_text(&mut rt.browser, value);
                    }
                }
                _ => {}
            }
        }
        Event::WindowEvent {
            event: WindowEvent::Ime(fyrox::event::Ime::Commit(value)),
            ..
        } if rt.browser.modal != Modal::None && !rt.browser.busy => {
            rt.browser.title.preedit.clear();
            insert_text(&mut rt.browser, value)
        }
        Event::WindowEvent {
            event: WindowEvent::Ime(fyrox::event::Ime::Preedit(value, _)),
            ..
        } if rt.browser.modal == Modal::Create
            && rt.browser.focus == Action::Title
            && !rt.browser.busy =>
        {
            rt.browser.title.preedit = value.chars().take(60).collect();
        }
        Event::WindowEvent {
            event: WindowEvent::Focused(false),
            ..
        } => {
            rt.keys = Keys::default();
            set_cursor(window, false);
        }
        _ => {}
    }
    if let Some(action) = action {
        rt.browser_action(action, window);
    }
    sync_ime(rt, window);
}
pub(super) fn sync_ime(rt: &mut Runtime, window: &Window) {
    let active = rt.lobby.open
        && rt.browser.open
        && rt.browser.modal == Modal::Create
        && rt.browser.focus == Action::Title
        && !rt.browser.busy;
    if active != rt.browser.ime_active {
        window.set_ime_allowed(active);
        rt.browser.ime_active = active;
        rt.browser.title.preedit.clear();
    }
}

#[derive(Debug, Default)]
pub(super) struct BrowserUi {
    root: Handle<UiNode>,
    bg: Handle<UiNode>,
    panel: Handle<UiNode>,
    labels: Vec<Handle<UiNode>>,
    columns: [Handle<UiNode>; 4],
    states: [Handle<UiNode>; PAGE_SIZE],
    codes: [Handle<UiNode>; PAGE_SIZE],
    buttons: Vec<Handle<UiNode>>,
    rows: Vec<Handle<UiNode>>,
    modal: Handle<UiNode>,
    modal_bg: Handle<UiNode>,
    dim: Handle<UiNode>,
    fields: [Handle<UiNode>; 2],
    modal_buttons: [Handle<UiNode>; 2],
}
const BUTTONS: [Action; 8] = [
    Action::Home,
    Action::Create,
    Action::Join,
    Action::Refresh,
    Action::Previous,
    Action::Next,
    Action::Resume,
    Action::Leave,
];
impl BrowserUi {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        let mut view = Self {
            bg: lobby::flat(ctx, rgb([16, 34, 30]), 0.),
            panel: lobby::flat(ctx, rgb([24, 45, 39]), 12.),
            labels: (0..46)
                .map(|_| lobby::caption(ctx, [237, 244, 228]))
                .collect(),
            columns: std::array::from_fn(|_| lobby::caption(ctx, [167, 191, 180])),
            states: std::array::from_fn(|_| lobby::caption(ctx, [167, 221, 188])),
            codes: std::array::from_fn(|_| lobby::caption(ctx, [237, 244, 228])),
            buttons: BUTTONS
                .iter()
                .map(|_| lobby::flat(ctx, rgb([40, 69, 57]), 8.))
                .collect(),
            rows: (0..PAGE_SIZE)
                .map(|_| lobby::flat(ctx, rgb([32, 56, 47]), 8.))
                .collect(),
            dim: lobby::flat(ctx, Color::from_rgba(3, 14, 10, 228), 0.),
            modal_bg: lobby::flat(ctx, rgb([31, 53, 44]), 14.),
            fields: std::array::from_fn(|_| lobby::flat(ctx, rgb([17, 35, 28]), 6.)),
            modal_buttons: std::array::from_fn(|_| lobby::flat(ctx, rgb([51, 79, 58]), 6.)),
            ..Default::default()
        };
        view.modal = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_children(
                    [view.dim, view.modal_bg]
                        .into_iter()
                        .chain(view.fields)
                        .chain(view.modal_buttons)
                        .chain(view.labels[38..].iter().copied())
                        .collect::<Vec<_>>(),
                ),
        )
        .build(ctx)
        .to_base();
        view.root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_visibility(false)
                .with_children(
                    [view.bg, view.panel]
                        .into_iter()
                        .chain(view.buttons.iter().copied())
                        .chain(view.rows.iter().copied())
                        .chain(view.labels[..38].iter().copied())
                        .chain(view.columns)
                        .chain(view.states)
                        .chain(view.codes)
                        .chain([view.modal])
                        .collect::<Vec<_>>(),
                ),
        )
        .build(ctx)
        .to_base();
        view
    }
    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> + '_ {
        self.labels
            .iter()
            .copied()
            .chain(self.columns)
            .chain(self.states)
            .chain(self.codes)
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface) {
        let show = rt.lobby.open && rt.browser.open;
        visible(ui, self.root, show);
        if !show {
            return;
        }
        let b = &rt.browser;
        let size = ui.screen_size();
        let scale = (size.x / 1280.).min(size.y / 800.);
        let put = |h, r| {
            let (x, y, w, v) = selection::rect(size.x, size.y, r);
            place(ui, h, x, y, w, v);
        };
        let txt = |i: usize, r, font, value: &str| {
            put(self.labels[i], r);
            text_size(ui, self.labels[i], font * scale);
            text(ui, self.labels[i], value);
        };
        place(ui, self.root, 0., 0., size.x, size.y);
        place(ui, self.bg, 0., 0., size.x, size.y);
        put(self.panel, (28., 208., 1224., 420.));
        txt(0, (44., 40., 850., 52.), 36., "방 선택");
        txt(
            1,
            (46., 100., 900., 26.),
            15.,
            "홈  >  방 선택  >  대전 로비     ·     함께할 방을 찾아보세요",
        );
        for (i, action) in BUTTONS.iter().enumerate() {
            let enabled = b.actions(rt.lobby.room_active).contains(action) && !b.busy;
            let show = match action {
                Action::Create | Action::Join => !rt.lobby.room_active,
                Action::Resume | Action::Leave => rt.lobby.room_active,
                _ => true,
            };
            visible(ui, self.buttons[i], show);
            visible(ui, self.labels[5 + i], show);
            put(self.buttons[i], action.bounds());
            lobby::background(
                ui,
                self.buttons[i],
                if !enabled {
                    [30, 46, 39]
                } else if b.focus == *action {
                    [77, 103, 69]
                } else {
                    [41, 69, 55]
                },
            );
            let label = match action {
                Action::Home => "< 홈으로",
                Action::Create => "방 만들기",
                Action::Join => "방 참여하기",
                Action::Refresh => {
                    if b.loading {
                        "불러오는 중..."
                    } else {
                        "새로고침  F5"
                    }
                }
                Action::Previous => "< 이전",
                Action::Next => "다음 >",
                Action::Resume => "내 로비로",
                Action::Leave => "방 나가기",
                _ => "",
            };
            let (x, y, w, h) = action.bounds();
            txt(5 + i, (x + 16., y + 14., w - 24., h - 12.), 16., label);
        }
        txt(2, (60., 218., 470., 25.), 13., "방 이름 / 방장");
        for (i, (x, w, label)) in [
            (548., 264., "맵"),
            (818., 80., "인원"),
            (912., 100., "상태"),
            (1040., 180., "방 코드"),
        ]
        .into_iter()
        .enumerate()
        {
            put(self.columns[i], (x, 218., w, 25.));
            text_size(ui, self.columns[i], 13. * scale);
            text(ui, self.columns[i], label);
        }
        for i in 0..PAGE_SIZE {
            let room = b.rooms.get(b.page * PAGE_SIZE + i);
            visible(ui, self.rows[i], room.is_some());
            visible(ui, self.states[i], room.is_some());
            visible(ui, self.codes[i], room.is_some());
            for j in 0..4 {
                visible(ui, self.labels[14 + i * 4 + j], room.is_some());
            }
            if let Some(room) = room {
                let y = 246. + i as f32 * 62.;
                put(self.rows[i], Action::Row(i).bounds());
                lobby::background(
                    ui,
                    self.rows[i],
                    if b.focus == Action::Row(i) {
                        [54, 84, 63]
                    } else {
                        [32, 56, 47]
                    },
                );
                txt(
                    14 + i * 4,
                    (60., y + 7., 472., 24.),
                    17.,
                    &lobby::short_name(&room.title, 29),
                );
                txt(
                    15 + i * 4,
                    (60., y + 31., 470., 18.),
                    11.,
                    &format!("방장  {}", lobby::short_name(&room.host_name, 28)),
                );
                txt(16 + i * 4, (548., y + 15., 264., 26.), 16., room.map_name());
                txt(
                    17 + i * 4,
                    (818., y + 15., 80., 26.),
                    15.,
                    &format!("{}/{}", room.players, room.capacity),
                );
                put(self.states[i], (912., y + 15., 120., 26.));
                text_size(ui, self.states[i], 15. * scale);
                text(
                    ui,
                    self.states[i],
                    if room.players >= room.capacity {
                        "가득 참"
                    } else {
                        room.state_name()
                    },
                );
                put(self.codes[i], (1040., y + 15., 180., 26.));
                text_size(ui, self.codes[i], 15. * scale);
                text(ui, self.codes[i], &room.code);
            }
        }
        let empty = if !b.list_error.is_empty() {
            "방 목록에 연결하지 못했어요. ‘방 만들기’나 ‘방 참여하기’로 친구와 직접 연결할 수 있어요."
        } else if b.loading && b.rooms.is_empty() {
            "웹에서 방 목록을 불러오는 중이에요."
        } else if b.rooms.is_empty() {
            "아직 열린 방이 없어요. ‘방 만들기’로 첫 방을 열어보세요."
        } else {
            ""
        };
        txt(3, (60., 382., 1150., 90.), 18., empty);
        txt(
            4,
            (44., 646., 865., 48.),
            14.,
            &if !b.notice.is_empty() {
                b.notice.clone()
            } else {
                format!(
                    "{}개 방  ·  {}/{} 페이지  ·  15초마다 갱신",
                    b.rooms.len(),
                    b.page + 1,
                    b.rooms.len().div_ceil(PAGE_SIZE).max(1)
                )
            },
        );
        visible(ui, self.modal, b.modal != Modal::None);
        if b.modal == Modal::None {
            return;
        }
        place(ui, self.modal, 0., 0., size.x, size.y);
        place(ui, self.dim, 0., 0., size.x, size.y);
        put(self.modal_bg, (350., 202., 580., 420.));
        let create = b.modal == Modal::Create;
        txt(
            38,
            (390., 226., 500., 42.),
            27.,
            if create {
                "방 만들기"
            } else {
                "방 참여하기"
            },
        );
        txt(
            39,
            (390., 279., 500., 32.),
            14.,
            if create {
                "방 이름"
            } else {
                "방 코드 입력 · Ctrl+V로 받은 코드를 붙여 넣으세요."
            },
        );
        visible(ui, self.fields[0], create);
        visible(ui, self.labels[40], create);
        visible(ui, self.labels[41], create);
        for (i, action) in [Action::Title, Action::Address].iter().enumerate() {
            put(self.fields[i], action.bounds());
            lobby::background(
                ui,
                self.fields[i],
                if b.focus == *action {
                    [37, 65, 48]
                } else {
                    [17, 35, 28]
                },
            );
        }
        txt(
            40,
            (402., 334., 476., 30.),
            18.,
            &input_value(&b.title, b.focus == Action::Title),
        );
        txt(
            41,
            (390., 382., 500., 24.),
            13.,
            if lan_mode() {
                "친구가 접속할 LAN 주소"
            } else {
                "EOS 자동 연결 · 직접 연결이 어려우면 중계 사용"
            },
        );
        visible(ui, self.fields[1], !create || lan_mode());
        visible(ui, self.labels[42], !create || lan_mode());
        txt(
            42,
            (402., 428., 476., 30.),
            18.,
            &input_value(&b.address, b.focus == Action::Address),
        );
        txt(43, (390., 482., 500., 50.), 13., &b.notice);
        for (i, action) in [Action::Cancel, Action::Confirm].iter().enumerate() {
            put(self.modal_buttons[i], action.bounds());
            let (x, y, w, _) = action.bounds();
            txt(
                44 + i,
                (x + 20., y + 16., w - 30., 30.),
                16.,
                if i == 0 {
                    "취소"
                } else if b.busy {
                    "연결 중..."
                } else if create {
                    "방 만들기"
                } else {
                    "참여하기"
                },
            );
        }
    }
}
fn input_value(input: &ChatInput, focused: bool) -> String {
    let mut value = input.draft.clone();
    if focused {
        value.insert_str(input.caret, &format!("{}|", input.preedit));
    }
    // Keep the caret visible for long titles/addresses.
    if value.chars().count() > 34 {
        let start = value[..input.caret.min(value.len())]
            .chars()
            .count()
            .saturating_sub(28);
        value = value.chars().skip(start).take(34).collect();
    }
    value
}
pub(super) fn review_step(rt: &mut Runtime, window: &Window, step: u8) {
    match step {
        1 => {
            rt.browser.rooms = (0..9)
                .map(|i| Room {
                    code: format!("ABCD{i:04}"),
                    title: format!("검증용 방 {} · 함께 플레이", i + 1),
                    host_name: "테스트 플레이어".into(),
                    map_id: if i % 2 == 0 {
                        "elf-city"
                    } else {
                        "fairy-kingdom-castle"
                    }
                    .into(),
                    players: if i == 1 { 12 } else { 1 + i },
                    capacity: 12,
                    state: if i % 3 == 0 { "playing" } else { "waiting" }.into(),
                })
                .collect();
        }
        2 => rt.browser_action(Action::Join, window),
        3 => {
            rt.browser.address.insert("invalid");
            rt.browser_action(Action::Confirm, window);
            assert!(!rt.lobby.room_active && !rt.browser.notice.is_empty());
        }
        4 => {
            rt.browser_action(Action::Cancel, window);
            rt.browser_action(Action::Create, window);
            assert!(!rt.browser.open);
        }
        9 => rt.browser_action(Action::Resume, window),
        11 => {
            rt.lobby.open = true;
            rt.browser.open = true;
            rt.browser.modal = Modal::Create;
            rt.browser.focus = Action::Title;
            rt.browser.title.clear();
            rt.browser.title.insert("친구들과 함께하는 엘리아스 대전");
            rt.browser.notice.clear();
        }
        12 => {
            rt.browser.modal = Modal::None;
            rt.browser.page = 1;
        }
        13 => {
            rt.browser.rooms.clear();
            rt.browser.page = 0;
        }
        14 => {
            rt.browser.loading = true;
        }
        15 => {
            rt.browser.loading = false;
            rt.browser.list_error =
                "웹 방 서비스에 연결하지 못했어요. 새로고침으로 다시 시도해 주세요.".into();
        }
        16 => {
            rt.browser.list_error.clear();
            rt.browser.link_code = Some("ABC12345".into());
            rt.browser.link_started = Instant::now();
            rt.browser.last_poll = Instant::now() + Duration::from_secs(3600);
            rt.browser.notice = "웹에서 연결 코드 ABC12345를 확인하고 승인해 주세요.".into();
        }
        _ => {}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linked_profile_survives_room_exit_and_refreshes_or_clears_its_photo() {
        let profile = PlayerProfile {
            display_name: "로그인한 플레이어".into(),
            handle: Some("dotg_player".into()),
            avatar: Some("mint".into()),
            avatar_url: Some("https://example.com/first.webp".into()),
        };
        let mut b = Browser {
            link_code: Some("ABC12345".into()),
            ..Default::default()
        };
        b.apply_link_status(LinkStatus::Connected(profile.clone()));
        assert!(b.link_code.is_none());
        assert_eq!(b.profile_detail(true), "@dotg_player");
        b.apply_avatar(
            profile.avatar_url.as_deref().unwrap(),
            Ok(AvatarImage {
                width: 1,
                height: 1,
                pixels: vec![10, 20, 30, 255],
            }),
        );
        assert!(b.avatar.is_some());
        b.close_room();
        b.apply_link_status(LinkStatus::Connected(profile.clone()));
        assert!(b.avatar.is_some());
        let mut changed = profile.clone();
        changed.display_name = "바뀐 이름".into();
        changed.avatar_url = Some("https://example.com/second.webp".into());
        b.apply_link_status(LinkStatus::Connected(changed));
        assert!(b.avatar.is_none());
        b.apply_avatar(
            profile.avatar_url.as_deref().unwrap(),
            Ok(AvatarImage {
                width: 1,
                height: 1,
                pixels: vec![10, 20, 30, 255],
            }),
        );
        assert!(
            b.avatar.is_none(),
            "a late photo must not overwrite the new profile"
        );
        assert_eq!(b.linked.as_ref().unwrap().display_name, "바뀐 이름");
        let mut removed = profile;
        removed.avatar_url = None;
        b.apply_link_status(LinkStatus::Connected(removed));
        assert!(b.avatar.is_none());
        b.apply_link_status(LinkStatus::Expired);
        assert!(b.linked.is_none() && b.avatar.is_none());
        assert_eq!(b.profile_detail(true), "게스트");
    }
    #[test]
    fn browser_modal_blocks_rows_and_matches_scaled_hit_regions() {
        let mut b = Browser::default();
        for size in [
            Vector2::new(1280., 800.),
            Vector2::new(960., 640.),
            Vector2::new(1920., 800.),
        ] {
            for action in [Action::Home, Action::Create, Action::Join] {
                let (x, y, w, h) = selection::rect(size.x, size.y, action.bounds());
                let p = Vector2::new(x + w / 2., y + h / 2.);
                b.modal = Modal::None;
                assert_eq!(b.hit(false, size, p), Some(action));
                b.modal = Modal::Join;
                assert_ne!(b.hit(false, size, p), Some(action));
            }
        }
        assert!(!b.actions(true).contains(&Action::Create));
    }
}
