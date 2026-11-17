//! Pre-match room screen. Connection, hero selection and match entry are separate actions.
use super::*;
use fyrox::{
    asset::manager::ResourceManager,
    core::math::Rect,
    gui::{
        canvas::CanvasBuilder,
        copypasta::ClipboardProvider,
        image::{ImageBuilder, ImageMessage},
    },
};
use std::net::SocketAddr;

const PAPER: [u8; 3] = [247, 246, 231];
const MUTED: [u8; 3] = [167, 191, 180];
const FOREST: [u8; 3] = [25, 48, 43];
const GOLD: [u8; 3] = [255, 210, 128];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Dialog {
    #[default]
    None,
    Join,
    Map,
    Bot(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Back,
    Create,
    Join,
    Leave,
    CopyCode,
    Map,
    Castle,
    ElfCity,
    Hero,
    Start,
    Address,
    Confirm,
    Cancel,
    Slot(usize),
    BotHero(u8),
    BotSave,
    BotRemove,
    BotCancel,
}
impl Action {
    fn bounds(self) -> (f32, f32, f32, f32) {
        match self {
            Self::Back => (44.0, 714.0, 152.0, 56.0),
            Self::Create => (850.0, 48.0, 180.0, 54.0),
            Self::Join => (1046.0, 48.0, 190.0, 54.0),
            Self::Leave => (1090.0, 149.0, 128.0, 44.0),
            Self::CopyCode => (900.0, 149.0, 174.0, 44.0),
            Self::Map => (438.0, 224.0, 404.0, 334.0),
            Self::Castle => (392.0, 360.0, 236.0, 112.0),
            Self::ElfCity => (650.0, 360.0, 236.0, 112.0),
            Self::Hero => (798.0, 714.0, 196.0, 56.0),
            Self::Start => (1010.0, 714.0, 226.0, 56.0),
            Self::Address => (400.0, 364.0, 480.0, 56.0),
            Self::Confirm => (650.0, 490.0, 230.0, 52.0),
            Self::Cancel => (400.0, 490.0, 230.0, 52.0),
            Self::Slot(row) => (
                if row < TEAM_SIZE { 44.0 } else { 870.0 },
                278.0 + (row % TEAM_SIZE) as f32 * 62.0,
                366.0,
                56.0,
            ),
            Self::BotHero(id) => (
                324.0 + (id as usize % 5) as f32 * 128.0,
                232.0 + (id as usize / 5) as f32 * 116.0,
                120.0,
                104.0,
            ),
            Self::BotCancel => (324.0, 636.0, 194.0, 48.0),
            Self::BotSave => (544.0, 636.0, 194.0, 48.0),
            Self::BotRemove => (764.0, 636.0, 192.0, 48.0),
        }
    }
}

#[derive(Debug)]
pub(super) struct Lobby {
    pub open: bool,
    pub room_active: bool,
    pub return_from_selection: bool,
    pub start_requested: bool,
    pub review: bool,
    pub(super) solo: bool,
    observed_phase: Phase,
    dialog: Dialog,
    focus: Action,
    address: ChatInput,
    notice: String,
    pending_map: MapId,
    pending_bot: u8,
}
impl Lobby {
    pub(super) fn chat_available(&self) -> bool {
        matches!(self.dialog, Dialog::None | Dialog::Map)
    }
    pub fn new(session: &Session) -> Self {
        Self {
            open: false,
            room_active: !matches!(session, Session::Solo(_)),
            return_from_selection: false,
            start_requested: false,
            review: std::env::args().any(|a| a == "--lobby-review"),
            solo: std::env::args()
                .any(|a| matches!(a.as_str(), "--solo" | "--lobby-review" | "--home-review")),
            observed_phase: Phase::Lobby,
            dialog: Dialog::None,
            focus: Action::Create,
            address: ChatInput::default(),
            notice: String::new(),
            pending_map: session.world().map_id,
            pending_bot: 0,
        }
    }
    fn connected(&self, rt: &Runtime) -> bool {
        self.room_active
            && rt.error.is_none()
            && !matches!(&rt.session, Session::Host(s) if !s.ready())
            && !matches!(&rt.session, Session::Join(c) if !c.received || c.slot.is_none() || c.error.is_some())
    }
    fn enabled(&self, rt: &Runtime, action: Action) -> bool {
        match action {
            Action::Slot(row) => {
                self.can_edit_bots(rt)
                    && row < PLAYERS
                    && !rt.session.world().actors[self.slot_for_row(rt, row)].human
            }
            Action::BotHero(id) => {
                matches!(self.dialog, Dialog::Bot(slot) if self.can_edit_bots(rt) && rt.session.world().bot_selection_check(slot, id).is_ok())
            }
            Action::BotSave => self.enabled(rt, Action::BotHero(self.pending_bot)),
            Action::BotRemove => {
                matches!(self.dialog, Dialog::Bot(slot) if self.can_edit_bots(rt) && rt.session.world().actors[slot].active && !rt.session.world().actors[slot].human)
            }
            Action::Create | Action::Join => !self.room_active,
            Action::Leave => self.room_active,
            Action::CopyCode => self.connected(rt) && rt.host_room_code().is_some(),
            Action::Hero => self.connected(rt) && rt.session.world().can_select(rt.session.slot()),
            Action::Start => {
                self.connected(rt)
                    && (!matches!(rt.session, Session::Join(_))
                        || !matches!(rt.session.world().phase, Phase::Lobby | Phase::Finished))
            }
            Action::Map | Action::Castle | Action::ElfCity => {
                !matches!(rt.session, Session::Join(_))
                    && matches!(rt.session.world().phase, Phase::Lobby | Phase::Finished)
            }
            _ => true,
        }
    }
    fn can_edit_bots(&self, rt: &Runtime) -> bool {
        self.connected(rt)
            && !matches!(rt.session, Session::Join(_))
            && matches!(rt.session.world().phase, Phase::Lobby | Phase::Finished)
    }
    fn slot_for_row(&self, rt: &Runtime, row: usize) -> usize {
        let side = if row < TEAM_SIZE {
            team(rt.session.slot())
        } else {
            1 - team(rt.session.slot())
        };
        side * TEAM_SIZE + row % TEAM_SIZE
    }
    fn actions(&self) -> Vec<Action> {
        match self.dialog {
            Dialog::None => vec![
                Action::Create,
                Action::Join,
                Action::Leave,
                Action::CopyCode,
                Action::Map,
                Action::Hero,
                Action::Start,
                Action::Back,
            ]
            .into_iter()
            .chain((0..PLAYERS).map(Action::Slot))
            .collect(),
            Dialog::Join => vec![Action::Address, Action::Confirm, Action::Cancel],
            Dialog::Map => vec![
                Action::Castle,
                Action::ElfCity,
                Action::Confirm,
                Action::Cancel,
            ],
            Dialog::Bot(_) => (0..ROSTER.len() as u8)
                .map(Action::BotHero)
                .chain([Action::BotCancel, Action::BotSave, Action::BotRemove])
                .collect(),
        }
    }
    fn hit(&self, size: Vector2<f32>, cursor: Vector2<f32>) -> Option<Action> {
        self.actions()
            .into_iter()
            .find(|a| selection::contains(cursor, selection::rect(size.x, size.y, a.bounds())))
    }
}

pub(super) fn parse_address(value: &str) -> Result<SocketAddr, &'static str> {
    let addr: SocketAddr = value
        .trim()
        .parse()
        .map_err(|_| "IP:포트 형식으로 입력해 주세요. 예: 192.168.0.2:28080")?;
    if addr.port() == 0 || addr.ip().is_unspecified() || addr.ip().is_multicast() {
        return Err("접속할 호스트의 IP와 1~65535 포트를 입력해 주세요.");
    }
    Ok(addr)
}

pub(super) fn paste_room_address(
    ui: &UserInterface,
    input: &mut ChatInput,
) -> Result<(), &'static str> {
    let value = ui
        .clipboard_mut()
        .ok_or("클립보드를 열 수 없어요. 다시 시도해 주세요.")?
        .get_contents()
        .map_err(|_| "클립보드를 읽을 수 없어요. 다시 시도해 주세요.")?;
    let value = value.trim();
    if value.is_empty() {
        return Err("복사한 방 코드가 없어요.");
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || ".:[]-".contains(c))
    {
        return Err("복사한 방 코드 또는 IP:포트를 확인해 주세요.");
    }
    if input.draft.len() + value.len() > 260 {
        return Err("방 코드가 너무 길어요. 입력 내용을 확인해 주세요.");
    }
    input.draft.insert_str(input.caret, value);
    input.caret += value.len();
    input.touch();
    Ok(())
}

impl Runtime {
    /// Open this room's lobby without replacing the session or stopping the match.
    pub(super) fn return_to_game_lobby(&mut self) {
        self.home_open = false;
        self.browser.open = false;
        self.lobby.open = true;
        self.lobby.room_active = true;
        self.lobby.observed_phase = self.session.world().phase;
        self.lobby.dialog = Dialog::None;
        self.lobby.return_from_selection = false;
        self.lobby.start_requested = false;
        self.lobby.pending_map = self.session.world().map_id;
        self.lobby.focus = Action::Start;
        self.lobby.notice = if matches!(
            self.session.world().phase,
            Phase::Setup | Phase::Playing | Phase::Intermission
        ) {
            "게임 로비로 돌아왔어요. 진행 중인 경기는 계속됩니다.".into()
        } else {
            String::new()
        };
        self.selecting = false;
        self.pending_ready = None;
        self.selection_message.clear();
        self.grabbed = false;
        self.chat_restore_grab = false;
        self.keys = Keys::default();
        self.wheel.cancel();
        self.orbit = OrbitCamera::default();
        self.deathcam = DeathCamera::default();
        self.communication_notice.clear();
        self.communication_notice_time = 0.0;
    }

    pub(super) fn leave_room_to_home(&mut self) {
        self.replace_room(
            Session::Solo(Box::new(World::room(Default::default(), true))),
            false,
        );
        self.return_to_game_lobby();
        self.lobby.room_active = false;
        self.lobby.open = false;
        self.home_open = true;
    }

    /// Follow the authoritative phase, including clients that missed the result snapshot.
    /// Keep the existing session, room registration, chat and voice preferences intact.
    pub(super) fn sync_match_end(&mut self) -> bool {
        let phase = self.session.world().phase;
        if self.last_phase == phase || !matches!(phase, Phase::Finished | Phase::Lobby) {
            return false;
        }
        self.home_open = false;
        self.browser.open = false;
        self.lobby.open = phase == Phase::Lobby;
        self.settings_menu.open = false;
        if let Some(preview) = &mut self.preview {
            preview.cheeks.cancel();
        }
        self.lobby.room_active = true;
        self.lobby.dialog = Dialog::None;
        self.lobby.return_from_selection = false;
        self.lobby.start_requested = false;
        self.lobby.pending_map = self.session.world().map_id;
        self.lobby.focus = if matches!(self.session, Session::Join(_)) {
            Action::Hero
        } else {
            Action::Start
        };
        self.lobby.notice = if self.lobby.open {
            "경기가 종료되어 게임 로비로 돌아왔어요.".into()
        } else {
            String::new()
        };
        self.selecting = false;
        self.pending_ready = None;
        self.selection_message.clear();
        self.selected = self.session.world().actors[self.session.slot()].character as usize;
        self.grabbed = false;
        self.chat_restore_grab = false;
        self.keys = Keys::default();
        self.wheel.cancel();
        self.communication_notice.clear();
        self.communication_notice_time = 0.0;
        self.orbit = OrbitCamera::default();
        self.deathcam = DeathCamera::default();
        self.last_shots = self.session.world().actors.each_ref().map(|a| a.shot);
        self.tracer_time = [0.0; PLAYERS];
        self.yaw = self.session.world().actors[self.session.slot()].yaw;
        self.pitch = self.session.world().actors[self.session.slot()].pitch;
        self.view_pos = self.session.world().actors[self.session.slot()].eye();
        true
    }
    fn open_lobby_selection(&mut self, start: bool) {
        self.home_open = false;
        self.browser.open = false;
        self.settings_menu.open = false;
        self.lobby.open = false;
        self.lobby.dialog = Dialog::None;
        self.lobby.return_from_selection = true;
        self.lobby.start_requested = start;
        self.keys = Keys::default();
        self.wheel.cancel();
        self.orbit = OrbitCamera::default();
        self.pending_ready = None;
        self.selection_message.clear();
        self.selected = self.session.world().actors[self.session.slot()].character as usize;
        self.selecting = self.session.world().can_select(self.session.slot());
        self.grabbed = false;
        self.chat_restore_grab = false;
        self.preview_serial += 1;
    }

    /// Observe authoritative snapshots so every connected player enters selection once.
    /// Repeated snapshots and returning to the lobby during a match must not reopen it.
    pub(super) fn sync_match_start(&mut self) -> bool {
        if !self.lobby.connected(self) {
            return false;
        }
        let phase = self.session.world().phase;
        let previous = std::mem::replace(&mut self.lobby.observed_phase, phase);
        if !self.auto_start
            && self.smoke.is_none()
            && matches!(previous, Phase::Lobby | Phase::Finished)
            && matches!(phase, Phase::Setup | Phase::Playing)
        {
            self.open_lobby_selection(true);
            return true;
        }
        false
    }

    fn start_lobby_match(&mut self) -> bool {
        if !self.lobby.enabled(self, Action::Start) || self.session.world().phase != Phase::Lobby {
            return false;
        }
        self.lobby.observed_phase = self.session.world().phase;
        self.session.start();
        self.sync_match_start()
    }

    fn host_room_code(&self) -> Option<String> {
        if !self.lobby.room_active {
            return None;
        }
        match &self.session {
            Session::Host(server) => self
                .browser
                .room_code
                .clone()
                .or_else(|| server.room_code()),
            _ => None,
        }
    }

    fn copy_room_code(&mut self, ui: &UserInterface) {
        if !self.lobby.enabled(self, Action::CopyCode) {
            return;
        }
        let Some(code) = self.host_room_code() else {
            return;
        };
        self.lobby.notice = match ui.clipboard_mut() {
            Some(mut clipboard) => match clipboard.set_contents(code) {
                Ok(()) => "방 코드를 복사했어요. 친구에게 전달해 주세요.".into(),
                Err(_) => "클립보드에 복사하지 못했어요. 다시 시도해 주세요.".into(),
            },
            None => "클립보드를 열 수 없어요. 다시 시도해 주세요.".into(),
        };
    }

    pub(super) fn practice_room(&mut self) {
        if self.lobby.room_active {
            self.lobby.notice = "혼자 연습하려면 현재 방에서 먼저 나가 주세요.".into();
        } else {
            let world = World::room(self.session.world().map_id, true);
            self.replace_room(Session::Solo(Box::new(world)), true);
            self.lobby.focus = Action::Start;
        }
        self.home_open = false;
        self.lobby.open = true;
        self.browser.open = false;
        self.selecting = false;
        self.grabbed = false;
        self.keys = Keys::default();
    }

    fn edit_bot(&mut self, row: usize) {
        if !self.lobby.enabled(self, Action::Slot(row)) {
            return;
        }
        let slot = self.lobby.slot_for_row(self, row);
        let w = self.session.world();
        self.lobby.pending_bot = if w.actors[slot].active {
            w.actors[slot].character
        } else {
            (0..ROSTER.len() as u8)
                .find(|&id| w.bot_selection_check(slot, id).is_ok())
                .unwrap_or(0)
        };
        self.lobby.dialog = Dialog::Bot(slot);
        self.lobby.focus = Action::BotHero(self.lobby.pending_bot);
        self.lobby.notice.clear();
    }
    fn configure_bot(&mut self, hero: Option<u8>) {
        let Dialog::Bot(slot) = self.lobby.dialog else {
            return;
        };
        if !self.lobby.can_edit_bots(self) {
            return;
        }
        let result = match &mut self.session {
            Session::Solo(w) => w.set_bot(slot, hero),
            Session::Host(s) => s.world.set_bot(slot, hero),
            Session::Join(_) => return,
        };
        self.lobby.notice = match result {
            SelectionResult::Accepted => hero.map_or_else(
                || "봇을 삭제했어요.".into(),
                |id| format!("{} 봇을 배치했어요.", character(id).korean),
            ),
            SelectionResult::Taken => "같은 팀에 이미 있는 사도이에요.".into(),
            SelectionResult::PositionFull => "같은 포지션은 최대 3명까지 선택할 수 있어요.".into(),
            _ => "경기 시작 전, 빈 자리나 봇 자리에서 다시 선택해 주세요.".into(),
        };
        if result == SelectionResult::Accepted {
            self.lobby.dialog = Dialog::None;
            self.lobby.focus = Action::Slot(slot);
        }
    }
    pub(super) fn replace_room(&mut self, session: Session, active: bool) {
        self.sync_chat();
        self.chat_transcript.next_session();
        self.browser.close_room();
        self.browser.open = !active;
        self.session = session;
        self.last_phase = self.session.world().phase;
        self.lobby.observed_phase = Phase::Lobby;
        self.lobby.room_active = active;
        self.lobby.dialog = Dialog::None;
        self.lobby.notice.clear();
        self.lobby.return_from_selection = false;
        self.lobby.start_requested = false;
        self.error = None;
        self.keys = Keys::default();
        self.pending_ready = None;
        self.selection_message.clear();
        self.selected = self.session.world().actors[self.session.slot()].character as usize;
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.orbit = OrbitCamera::default();
        self.deathcam = DeathCamera::default();
        self.voice = voice::Voice::default();
        self.solo_chat = ChatHistory::default();
        self.chat_error.clear();
        self.chat.touch();
        self.sync_chat();
        self.last_shots = [0; PLAYERS];
        self.tracer_time = [0.0; PLAYERS];
        self.last_round = 0;
        self.last_dead = false;
    }
    pub(super) fn create_room(&mut self) -> Result<(), String> {
        if self.lobby.room_active {
            return Ok(());
        }
        let map = self.session.world().map_id;
        if self.lobby.solo {
            let world = World::room(map, true);
            self.replace_room(Session::Solo(Box::new(world)), true);
        } else {
            let mut server = if std::env::args().any(|a| a == "--lan") {
                Server::bind("0.0.0.0:28080", true)
            } else {
                Server::host_eos(None, true)
            }
            .map_err(|e| format!("방을 만들지 못했어요. ({e})"))?;
            server.world.select_map(map);
            self.replace_room(Session::Host(Box::new(server)), true);
        }
        self.lobby.focus = Action::Start;
        Ok(())
    }
    fn join_room(&mut self) -> Result<(), String> {
        if self.lobby.room_active {
            return Ok(());
        }
        let address = self.lobby.address.draft.trim();
        let client = if address.starts_with("eos:") {
            Client::connect_address(address)
        } else {
            Client::connect(parse_address(address)?)
        }
        .map_err(|e| format!("접속할 수 없어요. ({e})"))?;
        self.replace_room(Session::Join(Box::new(client)), true);
        self.lobby.focus = Action::Hero;
        Ok(())
    }
    fn lobby_action(&mut self, action: Action, window: &Window) {
        if !self.lobby.enabled(self, action) {
            return;
        }
        match action {
            // Clipboard access is handled by on_event with the active UI.
            Action::CopyCode => {}
            Action::Slot(row) => self.edit_bot(row),
            Action::BotHero(id) => self.lobby.pending_bot = id,
            Action::BotSave => self.configure_bot(Some(self.lobby.pending_bot)),
            Action::BotRemove => self.configure_bot(None),
            Action::Back => {
                self.browser.open = true;
                if !self.lobby.solo {
                    self.browser.refresh();
                }
            }
            Action::Create => {
                if let Err(e) = self.create_room() {
                    self.lobby.notice = e;
                }
            }
            Action::Join => {
                self.lobby.dialog = Dialog::Join;
                self.lobby.focus = Action::Address;
                self.lobby.notice.clear();
            }
            Action::Leave => {
                let world = World::room(Default::default(), true);
                self.replace_room(Session::Solo(Box::new(world)), false);
                self.lobby.focus = Action::Create;
            }
            Action::Map => {
                self.lobby.dialog = Dialog::Map;
                self.lobby.pending_map = self.session.world().map_id;
                self.lobby.focus = Action::Confirm;
            }
            Action::Castle => self.lobby.pending_map = MapId::FairyKingdomCastle,
            Action::ElfCity => self.lobby.pending_map = MapId::ElfCity,
            Action::Hero => {
                self.open_lobby_selection(false);
                set_cursor(window, false);
            }
            Action::Start => {
                if matches!(self.session.world().phase, Phase::Lobby | Phase::Finished) {
                    self.start_lobby_match();
                    set_cursor(window, false);
                } else {
                    self.lobby.open = false;
                    self.close_selection(window);
                }
            }
            Action::Address => self.lobby.focus = Action::Address,
            Action::Confirm => match self.lobby.dialog {
                Dialog::Join => {
                    if let Err(e) = self.join_room() {
                        self.lobby.notice = e;
                    }
                }
                Dialog::Map => {
                    let map = self.lobby.pending_map;
                    let accepted = match &mut self.session {
                        Session::Solo(w) => w.select_map(map),
                        Session::Host(s) => s.world.select_map(map),
                        Session::Join(_) => false,
                    };
                    self.lobby.dialog = Dialog::None;
                    self.lobby.notice = if accepted {
                        format!("{} 맵을 선택했어요.", map.display_name())
                    } else {
                        "맵은 방장이 경기 시작 전에 선택할 수 있어요.".into()
                    };
                    self.lobby.focus = Action::Map;
                }
                Dialog::None | Dialog::Bot(_) => {}
            },
            Action::Cancel | Action::BotCancel => {
                self.lobby.dialog = Dialog::None;
                self.lobby.notice.clear();
                self.lobby.focus = Action::Join;
            }
        }
    }
}

pub(super) fn on_event(rt: &mut Runtime, event: &Event<()>, context: &PluginContext) {
    let window = &context.graphics_context.as_initialized_ref().window;
    let mut action = None;
    match event {
        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            rt.cursor = Vector2::new(position.x as f32, position.y as f32);
            // Keep text focus while typing an address, even if the mouse moves.
            if rt.lobby.dialog != Dialog::Join {
                if let Some(a) = rt
                    .lobby
                    .hit(context.user_interfaces.first().screen_size(), rt.cursor)
                {
                    if rt.lobby.enabled(rt, a) {
                        rt.lobby.focus = a;
                    }
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
        } => {
            action = rt
                .lobby
                .hit(context.user_interfaces.first().screen_size(), rt.cursor);
        }
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } if event.state == ElementState::Pressed => {
            let editing = rt.lobby.dialog == Dialog::Join && rt.lobby.focus == Action::Address;
            match event.physical_key {
                PhysicalKey::Code(KeyCode::KeyV)
                    if editing && rt.modifiers.control_key() && !event.repeat =>
                {
                    rt.lobby.notice =
                        paste_room_address(context.user_interfaces.first(), &mut rt.lobby.address)
                            .err()
                            .unwrap_or_default()
                            .into();
                }
                PhysicalKey::Code(KeyCode::F10) => context.loop_controller.exit(),
                PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                    action = Some(if rt.lobby.dialog == Dialog::None {
                        Action::Back
                    } else {
                        Action::Cancel
                    })
                }
                PhysicalKey::Code(KeyCode::Tab | KeyCode::ArrowDown | KeyCode::ArrowUp)
                    if !event.repeat =>
                {
                    let actions: Vec<_> = rt
                        .lobby
                        .actions()
                        .into_iter()
                        .filter(|a| rt.lobby.enabled(rt, *a))
                        .collect();
                    let index = actions
                        .iter()
                        .position(|a| *a == rt.lobby.focus)
                        .unwrap_or(actions.len() - 1);
                    let step = if event.physical_key == PhysicalKey::Code(KeyCode::ArrowUp) {
                        actions.len() - 1
                    } else {
                        1
                    };
                    rt.lobby.focus = actions[(index + step) % actions.len()];
                }
                PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) if !event.repeat => {
                    action = Some(if editing {
                        Action::Confirm
                    } else {
                        rt.lobby.focus
                    })
                }
                PhysicalKey::Code(KeyCode::Space) if !editing && !event.repeat => {
                    action = Some(rt.lobby.focus);
                }
                PhysicalKey::Code(KeyCode::F5)
                    if !event.repeat && rt.lobby.dialog == Dialog::None =>
                {
                    action = Some(Action::Start)
                }
                PhysicalKey::Code(KeyCode::Backspace) if editing => rt.lobby.address.backspace(),
                PhysicalKey::Code(KeyCode::Delete) if editing => rt.lobby.address.delete(),
                PhysicalKey::Code(KeyCode::ArrowLeft) if editing => rt.lobby.address.left(),
                PhysicalKey::Code(KeyCode::ArrowRight) if editing => rt.lobby.address.right(),
                PhysicalKey::Code(KeyCode::Home) if editing => rt.lobby.address.caret = 0,
                PhysicalKey::Code(KeyCode::End) if editing => {
                    rt.lobby.address.caret = rt.lobby.address.draft.len()
                }
                _ if editing && !rt.modifiers.control_key() && !rt.modifiers.super_key() => {
                    if let Some(value) = &event.text {
                        let value: String = value
                            .chars()
                            .filter(|c| c.is_ascii_alphanumeric() || ".:[]-".contains(*c))
                            .collect();
                        if rt.lobby.address.draft.len() + value.len() <= 260 {
                            rt.lobby.address.insert(&value);
                        }
                    }
                }
                _ => {}
            }
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
    if let Some(a) = action {
        if a == Action::CopyCode {
            rt.copy_room_code(context.user_interfaces.first());
        } else {
            rt.lobby_action(a, window);
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct LobbyUi {
    root: Handle<UiNode>,
    backdrop: Handle<UiNode>,
    shade: Handle<UiNode>,
    title: Handle<UiNode>,
    subtitle: Handle<UiNode>,
    status_bg: Handle<UiNode>,
    status: Handle<UiNode>,
    code_bg: Handle<UiNode>,
    code_heading: Handle<UiNode>,
    code_value: Handle<UiNode>,
    teams: [Handle<UiNode>; 2],
    team_titles: [Handle<UiNode>; 2],
    team_counts: [Handle<UiNode>; 2],
    rows: [Handle<UiNode>; PLAYERS],
    avatars: [Handle<UiNode>; PLAYERS],
    names: [Handle<UiNode>; PLAYERS],
    details: [Handle<UiNode>; PLAYERS],
    badges: [Handle<UiNode>; PLAYERS],
    buttons: [Handle<UiNode>; 8],
    button_labels: [Handle<UiNode>; 8],
    map_image: Handle<UiNode>,
    map_heading: Handle<UiNode>,
    map_title: Handle<UiNode>,
    map_detail: Handle<UiNode>,
    map_chip: Handle<UiNode>,
    rules: Handle<UiNode>,
    footer: Handle<UiNode>,
    divider: Handle<UiNode>,
    modal: Handle<UiNode>,
    modal_dim: Handle<UiNode>,
    modal_bg: Handle<UiNode>,
    modal_title: Handle<UiNode>,
    modal_desc: Handle<UiNode>,
    modal_field: Handle<UiNode>,
    modal_value: Handle<UiNode>,
    modal_error: Handle<UiNode>,
    modal_map: Handle<UiNode>,
    modal_elf: Handle<UiNode>,
    map_choices: [Handle<UiNode>; 2],
    map_choice_names: [Handle<UiNode>; 2],
    modal_buttons: [Handle<UiNode>; 2],
    modal_labels: [Handle<UiNode>; 2],
    bot_cards: Vec<Handle<UiNode>>,
    bot_avatars: Vec<Handle<UiNode>>,
    bot_names: Vec<Handle<UiNode>>,
    bot_buttons: [Handle<UiNode>; 3],
    bot_labels: [Handle<UiNode>; 3],
    textures: Vec<TextureResource>,
    background_texture: Option<TextureResource>,
    map_textures: Vec<TextureResource>,
}
const BUTTONS: [Action; 8] = [
    Action::Back,
    Action::Create,
    Action::Join,
    Action::Leave,
    Action::CopyCode,
    Action::Map,
    Action::Hero,
    Action::Start,
];
pub(super) fn flat(ctx: &mut BuildContext, color: Color, radius: f32) -> Handle<UiNode> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_background(Brush::Solid(color).into()),
    )
    .with_corner_radius(radius.into())
    .with_stroke_thickness(fyrox::gui::Thickness::uniform(0.0).into())
    .build(ctx)
    .to_base()
}
fn image(ctx: &mut BuildContext, texture: TextureResource) -> Handle<UiNode> {
    ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
        .with_texture(texture)
        .with_sync_with_texture_size(false)
        .build(ctx)
        .to_base()
}
pub(super) fn caption(ctx: &mut BuildContext, color: [u8; 3]) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(color)).into()),
    )
    .with_font_size(16.0.into())
    .build(ctx)
    .to_base()
}
pub(super) fn background(ui: &UserInterface, h: Handle<UiNode>, color: [u8; 3]) {
    ui.send(
        h,
        WidgetMessage::Background(Brush::Solid(rgb(color)).into()),
    );
}
impl LobbyUi {
    pub fn new(ui: &mut UserInterface, manager: &ResourceManager) -> Self {
        let textures = portraits::load(manager);
        let background_texture = manager.request::<Texture>("assets/ui/home/elias-picnic-soft.png");
        let map_texture =
            manager.request::<Texture>("assets/maps/fairy-kingdom-castle/preview.png");
        let elf_texture = manager.request::<Texture>("assets/maps/elf-city/preview.png");
        let ctx = &mut ui.build_ctx();
        let mut view = Self {
            backdrop: image(ctx, background_texture.clone()),
            shade: flat(ctx, Color::from_rgba(14, 33, 29, 238), 0.0),
            title: caption(ctx, PAPER),
            subtitle: caption(ctx, MUTED),
            status_bg: flat(ctx, rgb(FOREST), 8.0),
            status: caption(ctx, PAPER),
            code_bg: flat(ctx, rgb(FOREST), 8.0),
            code_heading: caption(ctx, MUTED),
            code_value: caption(ctx, PAPER),
            teams: std::array::from_fn(|i| {
                flat(
                    ctx,
                    rgb(if i == 0 { [43, 79, 67] } else { [83, 53, 62] }),
                    8.0,
                )
            }),
            team_titles: std::array::from_fn(|i| caption(ctx, if i == 0 { MINT } else { PINK })),
            team_counts: std::array::from_fn(|_| caption(ctx, PAPER)),
            rows: std::array::from_fn(|_| flat(ctx, rgb(FOREST), 6.0)),
            avatars: std::array::from_fn(|_| image(ctx, textures[0].clone())),
            names: std::array::from_fn(|_| caption(ctx, PAPER)),
            details: std::array::from_fn(|_| caption(ctx, MUTED)),
            badges: std::array::from_fn(|_| caption(ctx, GOLD)),
            buttons: std::array::from_fn(|_| flat(ctx, rgb(FOREST), 8.0)),
            button_labels: std::array::from_fn(|_| caption(ctx, PAPER)),
            map_image: image(ctx, map_texture.clone()),
            map_heading: caption(ctx, MUTED),
            map_title: caption(ctx, PAPER),
            map_detail: caption(ctx, MUTED),
            map_chip: caption(ctx, MINT),
            rules: caption(ctx, MUTED),
            footer: caption(ctx, MUTED),
            divider: flat(ctx, rgb([54, 76, 65]), 0.0),
            modal_dim: flat(ctx, Color::from_rgba(5, 16, 13, 220), 0.0),
            modal_bg: flat(ctx, rgb([30, 53, 46]), 12.0),
            modal_title: caption(ctx, PAPER),
            modal_desc: caption(ctx, MUTED),
            modal_field: flat(ctx, rgb([17, 36, 30]), 6.0),
            modal_value: caption(ctx, PAPER),
            modal_error: caption(ctx, GOLD),
            modal_map: image(ctx, map_texture.clone()),
            modal_elf: image(ctx, elf_texture.clone()),
            map_choices: std::array::from_fn(|_| flat(ctx, rgb(FOREST), 6.0)),
            map_choice_names: std::array::from_fn(|_| caption(ctx, PAPER)),
            modal_buttons: std::array::from_fn(|_| flat(ctx, rgb(FOREST), 6.0)),
            modal_labels: std::array::from_fn(|_| caption(ctx, PAPER)),
            bot_cards: ROSTER.iter().map(|_| flat(ctx, rgb(FOREST), 6.0)).collect(),
            bot_avatars: textures.iter().map(|t| image(ctx, t.clone())).collect(),
            bot_names: ROSTER.iter().map(|_| caption(ctx, PAPER)).collect(),
            bot_buttons: std::array::from_fn(|_| flat(ctx, rgb(FOREST), 6.0)),
            bot_labels: std::array::from_fn(|_| caption(ctx, PAPER)),
            textures,
            background_texture: Some(background_texture),
            map_textures: vec![map_texture, elf_texture],
            ..Default::default()
        };
        let modal_children = [view.modal_dim, view.modal_bg, view.modal_field]
            .into_iter()
            .chain(view.map_choices)
            .chain([view.modal_map, view.modal_elf])
            .chain(view.map_choice_names)
            .chain(view.modal_buttons)
            .chain(view.modal_labels)
            .chain(view.bot_cards.iter().copied())
            .chain(view.bot_avatars.iter().copied())
            .chain(view.bot_names.iter().copied())
            .chain(view.bot_buttons)
            .chain(view.bot_labels)
            .chain([
                view.modal_title,
                view.modal_desc,
                view.modal_value,
                view.modal_error,
            ])
            .collect::<Vec<_>>();
        view.modal = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_children(modal_children)
                .with_hit_test_visibility(false),
        )
        .build(ctx)
        .to_base();
        let children = [
            view.backdrop,
            view.shade,
            view.status_bg,
            view.divider,
            view.code_bg,
        ]
        .into_iter()
        .chain(view.teams)
        .chain(view.rows)
        .chain(view.buttons)
        .chain([view.map_image])
        .chain(view.avatars)
        .chain(view.labels().filter(|h| {
            ![
                view.modal_title,
                view.modal_desc,
                view.modal_value,
                view.modal_error,
                view.modal_labels[0],
                view.modal_labels[1],
                view.map_choice_names[0],
                view.map_choice_names[1],
            ]
            .contains(h)
                && !view.bot_names.contains(h)
                && !view.bot_labels.contains(h)
        }))
        .chain([view.modal])
        .collect::<Vec<_>>();
        view.root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_children(children)
                .with_hit_test_visibility(false)
                .with_visibility(false),
        )
        .build(ctx)
        .to_base();
        view
    }
    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> + '_ {
        [
            self.title,
            self.subtitle,
            self.status,
            self.code_heading,
            self.code_value,
            self.map_heading,
            self.map_title,
            self.map_detail,
            self.map_chip,
            self.rules,
            self.footer,
            self.modal_title,
            self.modal_desc,
            self.modal_value,
            self.modal_error,
        ]
        .into_iter()
        .chain(self.team_titles)
        .chain(self.team_counts)
        .chain(self.names)
        .chain(self.details)
        .chain(self.badges)
        .chain(self.button_labels)
        .chain(self.modal_labels)
        .chain(self.map_choice_names)
        .chain(self.bot_names.iter().copied())
        .chain(self.bot_labels)
    }
    pub fn ready(&self) -> bool {
        self.textures
            .iter()
            .chain(self.background_texture.iter())
            .chain(self.map_textures.iter())
            .all(|t| t.is_ok())
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface) {
        visible(ui, self.root, rt.lobby.open && !rt.browser.open);
        if !rt.lobby.open || rt.browser.open {
            return;
        }
        let size = ui.screen_size();
        let s = (size.x / 1280.0).min(size.y / 800.0);
        let put = |h, r| {
            let (x, y, w, v) = selection::rect(size.x, size.y, r);
            place(ui, h, x, y, w, v);
        };
        let txt = |h, r, font: f32, value: &str| {
            put(h, r);
            text_size(ui, h, font * s);
            text(ui, h, value);
        };
        place(ui, self.root, 0.0, 0.0, size.x, size.y);
        place(ui, self.backdrop, 0.0, 0.0, size.x, size.y);
        place(ui, self.shade, 0.0, 0.0, size.x, size.y);
        let art_aspect = 1672.0 / 941.0;
        let aspect = size.x / size.y;
        let uv = if aspect < art_aspect {
            let w = aspect / art_aspect;
            Rect::new((1.0 - w) * 0.5, 0.0, w, 1.0)
        } else {
            let h = art_aspect / aspect;
            Rect::new(0.0, (1.0 - h) * 0.5, 1.0, h)
        };
        ui.send(self.backdrop, ImageMessage::UvRect(uv));
        txt(self.title, (44.0, 43.0, 580.0, 49.0), 36.0, "대전 로비");
        txt(
            self.subtitle,
            (46.0, 99.0, 580.0, 26.0),
            15.0,
            "엘리아스에 오신 것을 환영합니다.",
        );
        put(self.status_bg, (44.0, 146.0, 1192.0, 52.0));
        let room = &rt.lobby;
        let connected = room.connected(rt);
        let show_code = room.room_active
            && matches!(&rt.session, Session::Host(server) if server.is_eos() || rt.browser.room_code.is_some());
        for h in [self.code_bg, self.code_heading, self.code_value] {
            visible(ui, h, show_code);
        }
        if show_code {
            put(self.code_bg, (650.0, 34.0, 586.0, 98.0));
            txt(
                self.code_heading,
                (666.0, 44.0, 550.0, 24.0),
                14.0,
                "방 코드",
            );
            let code = rt.host_room_code();
            let shown = code.as_deref().map_or_else(
                || "방 코드를 준비하고 있어요...".into(),
                |code| {
                    if code.starts_with("eos:") {
                        code.rsplit_once(':').map_or_else(
                            || code.to_owned(),
                            |(host, socket)| format!("{host}:\n{socket}"),
                        )
                    } else {
                        code.to_owned()
                    }
                },
            );
            txt(self.code_value, (666.0, 74.0, 550.0, 52.0), 18.0, &shown);
        }
        let status = if let Some(e) = &rt.error {
            format!("연결 실패 · 방 나가기 후 다시 참여해 주세요.  {e}")
        } else if !room.room_active {
            "대기 중  /  방을 만들거나, 친구의 방에 참여해 주세요.".into()
        } else {
            match &rt.session {
                Session::Solo(w) => format!("연습 방  /  플레이어 1명 · 봇 {}명 · 혼자서도 바로 시작할 수 있어요", w.actors.iter().filter(|a| a.active && !a.human).count()),
                Session::Host(server) if rt.browser.room_code.is_some() => format!("{} · {} · {}명", rt.browser.status(), server.network_label(), server.peer_count()+1),
                Session::Host(server) if server.is_eos() && !server.ready() => "EOS 로그인 중... 방 코드가 준비되면 복사 버튼이 활성화돼요.".into(),
                Session::Host(server) if server.is_eos() => format!("내가 만든 방  /  {}  ·  {}명 접속  ·  친구에게 방 코드를 공유해 주세요", server.network_label(), server.peer_count()+1),
                Session::Host(server) => format!("내가 만든 방  /  UDP {}  ·  {}명 접속  ·  친구에게 내 PC의 IP:포트를 알려 주세요",server.addr().map_or(28080,|a|a.port()),server.peer_count()+1),
                Session::Join(_) if !connected => "친구의 방에 연결 중...".into(),
                Session::Join(_) => if matches!(rt.session.world().phase, Phase::Lobby | Phase::Finished) { "친구의 방  /  호스트가 시작하면 캐릭터 선택 화면으로 자동 이동해요".into() } else { "경기 진행 중  /  게임 참가를 눌러 복귀할 수 있어요".into() },
            }
        };
        txt(
            self.status,
            (62.0, 162.0, if show_code { 820.0 } else { 1018.0 }, 25.0),
            14.0,
            &status,
        );
        let own_team = team(rt.session.slot());
        for side in 0..2 {
            let x = if side == 0 { 44.0 } else { 870.0 };
            put(self.teams[side], (x, 224.0, 366.0, 44.0));
            txt(
                self.team_titles[side],
                (x + 16.0, 235.0, 260.0, 27.0),
                19.0,
                if side == 0 {
                    "우리 팀"
                } else {
                    "상대 팀"
                },
            );
            let t = if side == 0 { own_team } else { 1 - own_team };
            let humans = if connected {
                rt.session.world().actors[t * TEAM_SIZE..(t + 1) * TEAM_SIZE]
                    .iter()
                    .filter(|a| a.active)
                    .count()
            } else {
                0
            };
            txt(
                self.team_counts[side],
                (x + 263.0, 238.0, 94.0, 22.0),
                12.0,
                &format!("{humans} / 6 명"),
            );
            for row in 0..TEAM_SIZE {
                let i = side * TEAM_SIZE + row;
                let slot = t * TEAM_SIZE + row;
                let a = &rt.session.world().actors[slot];
                let y = 278.0 + row as f32 * 62.0;
                let occupied =
                    (connected && a.active) || (!room.room_active && slot == rt.session.slot());
                let me = occupied && slot == rt.session.slot();
                put(self.rows[i], (x, y, 366.0, 56.0));
                let editable = room.enabled(rt, Action::Slot(i));
                background(
                    ui,
                    self.rows[i],
                    if editable && room.dialog == Dialog::None && room.focus == Action::Slot(i) {
                        [65, 99, 79]
                    } else if me {
                        [51, 80, 65]
                    } else {
                        FOREST
                    },
                );
                put(self.avatars[i], (x + 8.0, y + 4.0, 48.0, 48.0));
                visible(ui, self.avatars[i], occupied);
                if occupied {
                    let tex = &self.textures[a.character as usize];
                    ui.send(self.avatars[i], ImageMessage::Texture(Some(tex.clone())));
                    ui.send(
                        self.avatars[i],
                        ImageMessage::UvRect(portraits::uv(a.character as usize, tex, 48.0, 48.0)),
                    );
                }
                let name = if !occupied {
                    if editable {
                        "+  봇 추가".into()
                    } else {
                        "빈 자리".into()
                    }
                } else if a.human {
                    rt.session.player_name(slot)
                } else {
                    format!("봇 · {}", character(a.character).korean)
                };
                let name = short_name(&name, 14);
                txt(self.names[i], (x + 68.0, y + 9.0, 241.0, 24.0), 16.0, &name);
                txt(
                    self.details[i],
                    (x + 68.0, y + 34.0, 258.0, 19.0),
                    11.0,
                    if !occupied {
                        if editable {
                            "클릭하여 사도 선택"
                        } else {
                            "플레이어 대기 · 방장이 봇 추가 가능"
                        }
                    } else if !a.human {
                        if editable {
                            "클릭하여 사도 변경 / 봇 삭제"
                        } else {
                            "방장이 추가한 봇"
                        }
                    } else {
                        character(a.character).korean
                    },
                );
                txt(
                    self.badges[i],
                    (x + 318.0, y + 17.0, 42.0, 22.0),
                    12.0,
                    if me {
                        "나"
                    } else if occupied && slot == 0 && !matches!(rt.session, Session::Solo(_)) {
                        "방장"
                    } else if occupied && !a.human {
                        "BOT"
                    } else {
                        ""
                    },
                );
            }
        }
        for (i, action) in BUTTONS.iter().copied().enumerate() {
            let enabled = room.enabled(rt, action);
            let focus = enabled && room.dialog == Dialog::None && room.focus == action;
            put(self.buttons[i], action.bounds());
            let color = if action == Action::Start && enabled {
                if focus {
                    [255, 224, 163]
                } else {
                    GOLD
                }
            } else if focus {
                [65, 99, 79]
            } else {
                FOREST
            };
            background(ui, self.buttons[i], color);
            ui.send(
                self.buttons[i],
                WidgetMessage::Opacity(Some(if enabled { 1.0 } else { 0.45 })),
            );
            ui.send(
                self.button_labels[i],
                WidgetMessage::Foreground(
                    Brush::Solid(rgb(if action == Action::Start && enabled {
                        INK
                    } else if enabled {
                        PAPER
                    } else {
                        MUTED
                    }))
                    .into(),
                ),
            );
            let value = match action {
                Action::Back => "<  방 목록",
                Action::Create => "+  방 만들기",
                Action::Join => "방 참여하기",
                Action::Leave => "방 나가기",
                Action::CopyCode => "코드 복사하기",
                Action::Map => "",
                Action::Hero => "사도 선택",
                Action::Start => {
                    if !room.room_active {
                        "게임 시작"
                    } else if !connected {
                        "연결 대기 중"
                    } else if matches!(rt.session, Session::Join(_))
                        && matches!(rt.session.world().phase, Phase::Lobby | Phase::Finished)
                    {
                        "호스트 시작 대기"
                    } else if !matches!(rt.session.world().phase, Phase::Lobby | Phase::Finished) {
                        "게임 참가  >"
                    } else {
                        "게임 시작  >"
                    }
                }
                _ => "",
            };
            let (x, y, w, h) = action.bounds();
            txt(
                self.button_labels[i],
                (x + 18.0, y + 17.0, w - 26.0, h - 10.0),
                if matches!(action, Action::Leave | Action::CopyCode) {
                    13.0
                } else {
                    19.0
                },
                value,
            );
            visible(
                ui,
                self.buttons[i],
                !matches!(action, Action::Create | Action::Join)
                    && (action != Action::CopyCode || show_code)
                    && (action != Action::Leave || room.room_active),
            );
            visible(
                ui,
                self.button_labels[i],
                !matches!(action, Action::Create | Action::Join)
                    && (action != Action::CopyCode || show_code)
                    && (action != Action::Leave || room.room_active),
            );
        }
        txt(
            self.map_heading,
            (456.0, 239.0, 230.0, 24.0),
            14.0,
            "선택한 맵",
        );
        txt(
            self.map_chip,
            (748.0, 240.0, 80.0, 23.0),
            12.0,
            if matches!(rt.session, Session::Join(_)) {
                "방장 선택"
            } else {
                "선택됨"
            },
        );
        put(self.map_image, (450.0, 274.0, 380.0, 186.0));
        ui.send(
            self.map_image,
            ImageMessage::Texture(Some(
                self.map_textures[rt.session.world().map_id as usize].clone(),
            )),
        );
        // Crop the landscape capture to the map card's aspect without stretching.
        ui.send(
            self.map_image,
            ImageMessage::UvRect(Rect::new(0.0, 0.16, 1.0, 0.783)),
        );
        txt(
            self.map_title,
            (456.0, 477.0, 368.0, 37.0),
            27.0,
            rt.session.world().map_id.display_name(),
        );
        txt(
            self.map_detail,
            (456.0, 522.0, 368.0, 24.0),
            13.0,
            &format!("최대 6 대 6 · {}", rt.session.world().map_id.detail()),
        );
        txt(self.rules,(450.0,581.0,388.0,96.0),14.0,"혼자서도 바로 시작할 수 있어요.\n빈 슬롯을 눌러 원하는 사도의 봇을 추가해요.\n봇 슬롯을 누르면 사도 변경 / 삭제가 가능해요.\n상대 팀이 비어 있으면 시간 제한 없이 연습!");
        put(self.divider, (44.0, 688.0, 1192.0, 1.0));
        txt(
            self.footer,
            (220.0, 727.0, 560.0, 44.0),
            12.0,
            if room.notice.is_empty() {
                "Enter 채팅 · Space 선택 · Tab / 방향키 이동\n빈 슬롯 클릭: 봇 추가 · Esc 돌아가기"
            } else {
                &room.notice
            },
        );
        visible(ui, self.modal, room.dialog != Dialog::None);
        if room.dialog == Dialog::None {
            return;
        }
        place(ui, self.modal, 0.0, 0.0, size.x, size.y);
        place(ui, self.modal_dim, 0.0, 0.0, size.x, size.y);
        let bot = matches!(room.dialog, Dialog::Bot(_));
        for h in self
            .bot_cards
            .iter()
            .chain(&self.bot_avatars)
            .chain(&self.bot_names)
            .chain(&self.bot_buttons)
            .chain(&self.bot_labels)
        {
            visible(ui, *h, bot);
        }
        if let Dialog::Bot(slot) = room.dialog {
            for h in [
                self.modal_field,
                self.modal_value,
                self.modal_map,
                self.modal_elf,
            ]
            .into_iter()
            .chain(self.map_choices)
            .chain(self.map_choice_names)
            .chain(self.modal_buttons)
            .chain(self.modal_labels)
            {
                visible(ui, h, false);
            }
            put(self.modal_bg, (300.0, 132.0, 680.0, 570.0));
            txt(
                self.modal_title,
                (324.0, 155.0, 632.0, 40.0),
                29.0,
                if rt.session.world().actors[slot].active {
                    "봇 사도 변경 / 삭제"
                } else {
                    "봇 추가 · 사도 선택"
                },
            );
            txt(
                self.modal_desc,
                (324.0, 201.0, 632.0, 26.0),
                14.0,
                "사도를 고른 뒤 적용해 주세요. 같은 팀 사도 중복 금지 · 포지션별 최대 3명",
            );
            for (id, hero) in ROSTER.iter().enumerate() {
                let action = Action::BotHero(id as u8);
                let check = rt.session.world().bot_selection_check(slot, id as u8);
                let selected = room.pending_bot == id as u8;
                let enabled = room.enabled(rt, action);
                let (x, y, w, h) = action.bounds();
                put(self.bot_cards[id], (x, y, w, h));
                background(
                    ui,
                    self.bot_cards[id],
                    if selected && enabled {
                        [64, 127, 114]
                    } else if enabled && room.focus == action {
                        [65, 99, 79]
                    } else {
                        [21, 39, 37]
                    },
                );
                put(self.bot_avatars[id], (x + 32.0, y + 5.0, 56.0, 56.0));
                ui.send(
                    self.bot_avatars[id],
                    ImageMessage::UvRect(portraits::uv(id, &self.textures[id], 56.0, 56.0)),
                );
                ui.send(
                    self.bot_avatars[id],
                    WidgetMessage::Opacity(Some(if enabled { 1.0 } else { 0.3 })),
                );
                txt(
                    self.bot_names[id],
                    (x + 6.0, y + 64.0, w - 12.0, 39.0),
                    13.0,
                    &format!(
                        "{}{}\n{}",
                        if selected { "● " } else { "" },
                        short_name(hero.korean, 8),
                        match check {
                            Err(SelectionResult::Taken) => "팀 내 선택됨",
                            Err(SelectionResult::PositionFull) => "포지션 가득 참",
                            _ => match hero.position as usize {
                                0 => "전열",
                                1 => "중열",
                                _ => "후열",
                            },
                        }
                    ),
                );
            }
            visible(ui, self.modal_error, true);
            txt(
                self.modal_error,
                (324.0, 589.0, 632.0, 39.0),
                14.0,
                if room.notice.is_empty() {
                    "선택한 사도으로 봇을 배치합니다. 취소하면 변경되지 않아요."
                } else {
                    &room.notice
                },
            );
            for (i, action) in [Action::BotCancel, Action::BotSave, Action::BotRemove]
                .into_iter()
                .enumerate()
            {
                let enabled = room.enabled(rt, action);
                let (x, y, w, h) = action.bounds();
                put(self.bot_buttons[i], (x, y, w, h));
                background(
                    ui,
                    self.bot_buttons[i],
                    if enabled && room.focus == action {
                        [88, 124, 94]
                    } else {
                        [52, 79, 63]
                    },
                );
                ui.send(
                    self.bot_buttons[i],
                    WidgetMessage::Opacity(Some(if enabled { 1.0 } else { 0.35 })),
                );
                txt(
                    self.bot_labels[i],
                    (x + 18.0, y + 14.0, w - 32.0, 30.0),
                    18.0,
                    ["취소", "선택한 사도 적용", "봇 삭제"][i],
                );
                ui.send(
                    self.bot_labels[i],
                    WidgetMessage::Opacity(Some(if enabled { 1.0 } else { 0.35 })),
                );
            }
            return;
        }
        for h in self.modal_buttons.into_iter().chain(self.modal_labels) {
            visible(ui, h, true);
        }
        put(self.modal_bg, (364.0, 244.0, 552.0, 330.0));
        let join = room.dialog == Dialog::Join;
        txt(
            self.modal_title,
            (400.0, 276.0, 480.0, 42.0),
            29.0,
            if join {
                "친구의 방에 참여하기"
            } else {
                "맵 선택"
            },
        );
        txt(
            self.modal_desc,
            (400.0, 328.0, 480.0, 27.0),
            14.0,
            if join {
                "Ctrl+V로 방 코드를 붙여 넣거나 IP:포트를 입력해 주세요."
            } else {
                "전장을 고른 뒤 ‘이 맵 선택’을 눌러 주세요."
            },
        );
        put(self.modal_field, Action::Address.bounds());
        visible(ui, self.modal_field, join);
        visible(ui, self.modal_value, join);
        visible(ui, self.modal_map, !join);
        visible(ui, self.modal_elf, !join);
        for (i, action) in [Action::Castle, Action::ElfCity].into_iter().enumerate() {
            visible(ui, self.map_choices[i], !join);
            visible(ui, self.map_choice_names[i], !join);
            let (x, y, w, h) = action.bounds();
            put(self.map_choices[i], (x, y, w, h));
            background(
                ui,
                self.map_choices[i],
                if room.pending_map as usize == i {
                    [64, 127, 114]
                } else if room.focus == action {
                    [58, 83, 79]
                } else {
                    [21, 39, 37]
                },
            );
            put(
                [self.modal_map, self.modal_elf][i],
                (x + 4.0, y + 4.0, w - 8.0, 70.0),
            );
            ui.send(
                [self.modal_map, self.modal_elf][i],
                ImageMessage::UvRect(Rect::new(0.0, 0.19, 1.0, 0.491)),
            );
            txt(
                self.map_choice_names[i],
                (x + 12.0, y + 82.0, w - 20.0, 25.0),
                16.0,
                &format!(
                    "{}{}",
                    if room.pending_map as usize == i {
                        "● "
                    } else {
                        ""
                    },
                    MapId::ALL[i].display_name()
                ),
            );
        }
        background(
            ui,
            self.modal_field,
            if room.focus == Action::Address {
                [50, 76, 62]
            } else {
                [17, 36, 30]
            },
        );
        let input = &room.address;
        let value = if input.draft.is_empty() {
            "방 코드 입력 · Ctrl+V로 붙여넣기".into()
        } else if room.focus == Action::Address {
            format!(
                "{}|{}",
                &input.draft[..input.caret],
                &input.draft[input.caret..]
            )
        } else {
            input.draft.clone()
        };
        let shown = if value.len() > 48 && value.is_ascii() {
            format!("{}…{}", &value[..21], &value[value.len() - 21..])
        } else {
            value
        };
        txt(self.modal_value, (414.0, 382.0, 452.0, 30.0), 16.0, &shown);
        visible(ui, self.modal_error, join);
        txt(
            self.modal_error,
            if join {
                (400.0, 437.0, 480.0, 44.0)
            } else {
                (577.0, 376.0, 303.0, 84.0)
            },
            12.0,
            if join {
                if room.notice.is_empty() {
                    "인터넷: EOS 방 코드 · 같은 PC: --lan / 127.0.0.1:28080"
                } else {
                    &room.notice
                }
            } else {
                ""
            },
        );
        for (i, a) in [Action::Cancel, Action::Confirm].into_iter().enumerate() {
            put(self.modal_buttons[i], a.bounds());
            background(
                ui,
                self.modal_buttons[i],
                if room.focus == a {
                    [88, 124, 94]
                } else {
                    [52, 79, 63]
                },
            );
            let (x, y, w, h) = a.bounds();
            txt(
                self.modal_labels[i],
                (x + 18.0, y + 16.0, w - 32.0, h - 10.0),
                18.0,
                if i == 0 {
                    "취소"
                } else if join {
                    "참여하기  >"
                } else {
                    "이 맵 선택"
                },
            );
        }
    }
}

pub(super) fn short_name(value: &str, max: usize) -> String {
    if value.chars().count() > max {
        format!("{}…", value.chars().take(max - 1).collect::<String>())
    } else {
        value.into()
    }
}

pub(super) fn review(rt: &mut Runtime, context: &PluginContext) {
    let window = &context.graphics_context.as_initialized_ref().window;
    if rt.review_frames == 0 {
        match rt.review_capture {
            1 => {
                rt.play_from_home(window);
                rooms::review_step(rt, window, 1);
                assert!(rt.lobby.open && !rt.selecting && !rt.grabbed);
            }
            2 | 3 => rooms::review_step(rt, window, rt.review_capture),
            4 => {
                rooms::review_step(rt, window, 4);
                assert!(rt.lobby.room_active);
                let original = rt.session.world().map_id;
                rt.lobby_action(Action::Map, window);
                rt.lobby_action(Action::ElfCity, window);
                rt.lobby_action(Action::Cancel, window);
                assert_eq!(rt.session.world().map_id, original);
            }
            5 => {
                rt.lobby_action(Action::Map, window);
                rt.lobby_action(Action::ElfCity, window);
            }
            6 => {
                rt.lobby_action(Action::Confirm, window);
                assert_eq!(rt.session.world().map_id, MapId::ElfCity);
                rt.lobby_action(Action::Hero, window);
                assert!(rt.selecting);
            }
            7 => rt.confirm_selection(),
            8 => {
                assert!(rt.lobby.open && rt.session.world().phase == Phase::Lobby);
                rt.lobby_action(Action::Back, window);
                assert!(rt.browser.open && rt.lobby.open && !rt.home_open);
            }
            9 => {
                rooms::review_step(rt, window, 9);
                rt.lobby_action(Action::Start, window);
                assert!(rt.selecting);
                rt.cancel_selection(window);
                assert!(rt.lobby.open);
            }
            10 => {
                rt.lobby_action(Action::Start, window);
                rt.confirm_selection();
            }
            11..=16 => rooms::review_step(rt, window, rt.review_capture),
            17 => {
                assert_eq!(
                    rt.session
                        .world()
                        .actors
                        .iter()
                        .filter(|a| a.active)
                        .count(),
                    1
                );
                rt.replace_room(
                    Session::Solo(Box::new(World::room(MapId::ElfCity, true))),
                    true,
                );
                rt.lobby.open = true;
                rt.grabbed = false;
                set_cursor(window, false);
            }
            18 => rt.lobby_action(Action::Slot(6), window),
            19 => rt.lobby_action(Action::BotHero(14), window),
            20 => {
                rt.lobby_action(Action::BotSave, window);
                assert_eq!(rt.session.world().actors[6].character, 14);
                assert!(rt.session.world().actors[6].active);
            }
            21 => {
                rt.lobby_action(Action::Slot(6), window);
                rt.lobby_action(Action::BotHero(4), window);
                rt.lobby_action(Action::BotSave, window);
                assert_eq!(rt.session.world().actors[6].character, 4);
                rt.lobby_action(Action::Slot(6), window);
            }
            22 => {
                rt.lobby_action(Action::BotRemove, window);
                assert!(!rt.session.world().actors[6].active);
            }
            23 => {
                rt.lobby_action(Action::Slot(1), window);
                assert!(!rt.lobby.enabled(rt, Action::BotHero(0)));
            }
            24 => {
                rt.lobby_action(Action::BotCancel, window);
                assert!(!rt.session.world().actors[1].active);
                rt.lobby_action(Action::Start, window);
                rt.confirm_selection();
            }
            25 => {
                rt.lobby.open = false;
                assert!(!rt.home_open && !rt.lobby.open && !rt.selecting && rt.grabbed);
                assert_ne!(rt.session.world().phase, Phase::Lobby);
                assert_eq!(rt.session.world().map_id, MapId::ElfCity);
                assert_eq!(
                    rt.session
                        .world()
                        .actors
                        .iter()
                        .filter(|a| a.active)
                        .count(),
                    1
                );
                rt.chat.insert("경기 후에도 유지되는 초안");
                if let Session::Solo(world) = &mut rt.session {
                    world.free_attack = true;
                    world.finish_round(true);
                }
            }
            26 => {
                assert_eq!(rt.session.world().phase, Phase::Finished);
                assert!(!rt.lobby.open && !rt.selecting && !rt.grabbed);
                if let Session::Solo(world) = &mut rt.session {
                    world.phase_time = 0.0;
                }
            }
            27 => {
                assert_eq!(rt.session.world().phase, Phase::Lobby);
                assert!(rt.lobby.open && rt.lobby.room_active && !rt.browser.open);
                assert!(!rt.home_open && !rt.selecting && !rt.grabbed);
                assert_eq!(rt.chat.draft, "경기 후에도 유지되는 초안");
                assert_eq!(rt.session.world().map_id, MapId::ElfCity);
                assert_eq!(
                    rt.session
                        .world()
                        .actors
                        .iter()
                        .filter(|a| a.active)
                        .count(),
                    1
                );
                rt.lobby_action(Action::Start, window);
                rt.confirm_selection();
            }
            28.. => {
                assert!(!rt.lobby.open && !rt.selecting && rt.grabbed);
                assert_eq!(rt.session.world().phase, Phase::Setup);
                println!("LOBBY_REVIEW_OK: home, rooms, map, hero, solo match entry, empty slots, bots, match result, return to same lobby and rematch");
                context.loop_controller.exit();
            }
            _ => {}
        }
    }
    rt.review_frames += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn runtime() -> Runtime {
        Runtime::new(Session::Solo(Box::default()), None, false)
    }
    #[test]
    fn menu_return_preserves_solo_match_and_cancels_pending_gameplay() {
        let mut rt = runtime();
        rt.session.start();
        rt.deliver_chat("경기 메시지").unwrap();
        rt.chat.insert("작성 중");
        rt.browser.room_code = Some("TEST01".into());
        rt.keys.fire = true;
        rt.keys.w = true;
        rt.keys.zoom = true;
        rt.pending_ready = Some(0);
        rt.selecting = true;
        rt.chat_restore_grab = true;
        rt.wheel.begin();
        rt.orbit.active = true;
        let phase = rt.session.world().phase;
        let remaining = rt.session.world().phase_time;
        rt.return_to_game_lobby();
        assert!(rt.lobby.open && rt.lobby.room_active);
        assert!(!rt.home_open && !rt.browser.open && !rt.selecting && !rt.grabbed);
        assert!(!rt.keys.fire && !rt.keys.w && !rt.keys.zoom && !rt.chat_restore_grab);
        assert!(!rt.wheel.open && !rt.orbit.active);
        assert_eq!(rt.pending_ready, None);
        assert!(!rt.sync_match_start());
        assert_eq!(rt.session.world().phase, phase);
        assert_eq!(rt.session.world().phase_time, remaining);
        assert_eq!(rt.browser.room_code.as_deref(), Some("TEST01"));
        assert_eq!(rt.chat_history().messages.len(), 1);
        assert_eq!(rt.chat.draft, "작성 중");
        assert!(rt.lobby.enabled(&rt, Action::Start));
        assert!(!rt.lobby.enabled(&rt, Action::Map));
    }

    #[test]
    fn menu_navigation_preserves_network_sessions_until_leaving() {
        let server = Server::bind("127.0.0.1:0", true).unwrap();
        let address = server.addr().unwrap();
        let client = Client::connect(address).unwrap();
        let mut host = Runtime::new(Session::Host(Box::new(server)), None, false);
        let mut guest = Runtime::new(Session::Join(Box::new(client)), None, false);
        for _ in 0..24 {
            guest.session.update(Input::default()).unwrap();
            host.session.update(Input::default()).unwrap();
        }
        assert!(guest.lobby.connected(&guest));
        host.session.start();
        for _ in 0..24 {
            guest.session.update(Input::default()).unwrap();
            host.session.update(Input::default()).unwrap();
        }
        let slot = guest.session.slot();
        assert_eq!(guest.session.world().phase, Phase::Setup);
        for rt in [&mut host, &mut guest] {
            rt.last_phase = rt.session.world().phase;
            rt.home_open = false;
            rt.browser.room_code = Some("TEST02".into());
            rt.return_to_game_lobby();
            assert!(rt.lobby.open && rt.lobby.room_active);
            assert!(!rt.sync_match_start());
            assert!(rt.lobby.enabled(rt, Action::Start));
            assert_eq!(rt.browser.room_code.as_deref(), Some("TEST02"));
        }
        assert!(
            matches!(&host.session, Session::Host(s) if s.peer_count() == 1 && s.addr().unwrap() == address)
        );
        assert!(matches!(guest.session, Session::Join(_)));
        assert_eq!(guest.session.slot(), slot);
        guest.leave_room_to_home();
        assert!(matches!(&host.session, Session::Host(s) if s.world.phase == Phase::Setup));
        host.leave_room_to_home();
        for rt in [&mut host, &mut guest] {
            assert!(rt.home_open && !rt.lobby.open && !rt.lobby.room_active);
            assert!(!rt.browser.open && rt.browser.room_code.is_none());
            assert!(matches!(rt.session, Session::Solo(_)));
            assert_eq!(rt.session.world().phase, Phase::Lobby);
            assert!(!rt.sync_match_end() && !rt.sync_match_start());
            assert!(!rt.controls_active());
        }
        // Leaving as host releases its socket, so another room can use the port.
        assert!(Server::bind(address, true).is_ok());
    }

    #[test]
    fn match_end_closes_overlays_and_restores_the_room_without_losing_chat() {
        let mut rt = runtime();
        rt.practice_room();
        rt.deliver_chat("경기 전 메시지").unwrap();
        rt.chat.insert("작성 중");
        rt.chat.open = true;
        rt.chat_restore_grab = true;
        rt.session.start();
        rt.last_phase = Phase::Playing;
        rt.selecting = true;
        rt.pending_ready = Some(0);
        rt.lobby.return_from_selection = true;
        rt.lobby.start_requested = true;
        rt.lobby.dialog = Dialog::Map;
        rt.settings_menu.open = true;
        rt.grabbed = true;
        rt.keys.fire = true;
        rt.keys.zoom = true;
        rt.wheel.begin();
        rt.orbit.active = true;
        if let Session::Solo(world) = &mut rt.session {
            world.free_attack = true;
            world.finish_round(true);
        }
        assert!(rt.sync_match_end());
        assert!(!rt.selecting && !rt.grabbed && !rt.wheel.open && !rt.orbit.active);
        assert!(!rt.keys.fire && !rt.keys.zoom && !rt.chat_restore_grab);
        assert_eq!(rt.pending_ready, None);
        assert!(!rt.lobby.open && !rt.home_open && !rt.browser.open);
        assert!(!rt.settings_menu.open);
        // A delayed selection confirmation or F5 must not bypass the return flow.
        rt.session.start();
        assert_eq!(rt.session.world().phase, Phase::Finished);
        rt.last_phase = Phase::Finished;
        if let Session::Solo(world) = &mut rt.session {
            world.phase_time = 0.0;
        }
        rt.session.update(Input::default()).unwrap();
        assert!(rt.sync_match_end());
        assert!(rt.lobby.open && rt.lobby.room_active);
        assert!(matches!(rt.session, Session::Solo(_)));
        assert!(!rt.home_open && !rt.browser.open && !rt.selecting && !rt.grabbed);
        assert_eq!(rt.lobby.dialog, Dialog::None);
        assert!(!rt.lobby.return_from_selection && !rt.lobby.start_requested);
        assert!(rt.lobby.enabled(&rt, Action::Start));
        assert!(rt.lobby.enabled(&rt, Action::Map));
        assert!(rt.chat.open);
        assert_eq!(rt.chat.draft, "작성 중");
        assert_eq!(rt.chat_history().messages.len(), 1);
        rt.last_phase = Phase::Lobby;
        assert!(!rt.sync_match_end());
        rt.session.start();
        assert_eq!(rt.session.world().phase, Phase::Setup);
    }

    #[test]
    fn guest_returns_even_when_the_finished_snapshot_was_missed() {
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let client = Client::connect(server.addr().unwrap()).unwrap();
        let mut rt = Runtime::new(Session::Join(Box::new(client)), None, false);
        for _ in 0..24 {
            rt.session.update(Input::default()).unwrap();
            server.update(Input::default()).unwrap();
        }
        assert!(rt.lobby.connected(&rt));
        rt.last_phase = Phase::Playing;
        rt.selecting = true;
        rt.pending_ready = Some(rt.selected as u8);
        rt.grabbed = true;
        assert!(rt.sync_match_end());
        assert!(rt.lobby.open && rt.lobby.room_active);
        assert!(!rt.home_open && !rt.browser.open && !rt.selecting && !rt.grabbed);
        assert_eq!(rt.pending_ready, None);
        assert!(!rt.lobby.enabled(&rt, Action::Start));
        assert!(!rt.lobby.enabled(&rt, Action::Map));
        assert!(rt.lobby.enabled(&rt, Action::Hero));
        assert_eq!(server.peer_count(), 1);
    }

    #[test]
    fn leaving_a_running_room_does_not_trigger_match_return() {
        let mut rt = runtime();
        rt.practice_room();
        rt.session.start();
        rt.last_phase = Phase::Playing;
        rt.replace_room(
            Session::Solo(Box::new(World::room(MapId::ElfCity, true))),
            false,
        );
        assert!(!rt.sync_match_end());
        assert!(!rt.lobby.room_active);
        assert!(rt.browser.open);
    }

    #[test]
    fn chat_history_and_draft_survive_map_hero_match_and_room_changes() {
        let mut rt = runtime();
        rt.practice_room();
        rt.deliver_chat("로비에서 보낸 메시지").unwrap();
        rt.lobby.dialog = Dialog::Map;
        assert!(rt.lobby.chat_available());
        rt.deliver_chat("맵 선택 중").unwrap();
        if let Session::Solo(w) = &mut rt.session {
            assert!(w.select_map(MapId::ElfCity));
        }
        rt.lobby.open = false;
        rt.selecting = true;
        rt.deliver_chat("사도 선택 중").unwrap();
        rt.session.start();
        rt.selecting = false;
        rt.deliver_chat("경기 중").unwrap();
        rt.chat.insert("작성 중인 내용");
        rt.chat.channel = trickcal_shooting_balddagu::chat::ChatChannel::Match;
        rt.replace_room(Session::Solo(Box::default()), false);
        rt.lobby.open = true;
        assert!(rt.deliver_chat("연결 안 됨").is_err());
        rt.practice_room();
        rt.deliver_chat("새 방").unwrap();
        assert_eq!(rt.chat.draft, "작성 중인 내용");
        assert_eq!(
            rt.chat.channel,
            trickcal_shooting_balddagu::chat::ChatChannel::Match
        );
        assert_eq!(
            rt.chat_history()
                .messages
                .iter()
                .map(|m| m.text.as_str())
                .collect::<Vec<_>>(),
            [
                "로비에서 보낸 메시지",
                "맵 선택 중",
                "사도 선택 중",
                "경기 중",
                "새 방"
            ]
        );
    }
    #[test]
    fn changing_rooms_routes_only_new_messages_to_the_new_peers() {
        use std::{thread, time::Duration};
        use trickcal_shooting_balddagu::chat::ChatChannel;
        let mut rt = runtime();
        let host = Server::bind("127.0.0.1:0", true).unwrap();
        let mut old_peer = Client::connect(host.addr().unwrap()).unwrap();
        rt.replace_room(Session::Host(Box::new(host)), true);
        rt.chat.channel = ChatChannel::Match;
        for _ in 0..20 {
            old_peer.update(Input::default()).unwrap();
            rt.session.update(Input::default()).unwrap();
            thread::sleep(Duration::from_millis(10));
        }
        rt.deliver_chat("old room").unwrap();
        for _ in 0..20 {
            rt.session.update(Input::default()).unwrap();
            old_peer.update(Input::default()).unwrap();
            thread::sleep(Duration::from_millis(10));
        }
        let mut new_host = Server::bind("127.0.0.1:0", true).unwrap();
        let new_client = Client::connect(new_host.addr().unwrap()).unwrap();
        rt.replace_room(Session::Join(Box::new(new_client)), true);
        for _ in 0..20 {
            rt.session.update(Input::default()).unwrap();
            new_host.update(Input::default()).unwrap();
            thread::sleep(Duration::from_millis(10));
        }
        rt.deliver_chat("new room").unwrap();
        new_host
            .send_chat(ChatChannel::Match, "new host reply")
            .unwrap();
        for _ in 0..30 {
            rt.session.update(Input::default()).unwrap();
            new_host.update(Input::default()).unwrap();
            old_peer.update(Input::default()).unwrap();
            thread::sleep(Duration::from_millis(10));
        }
        rt.sync_chat();
        assert_eq!(old_peer.chat.messages.len(), 1);
        assert_eq!(old_peer.chat.messages[0].text, "old room");
        assert_eq!(new_host.chat.messages.len(), 2);
        assert!(new_host.chat.messages.iter().all(|m| m.text != "old room"));
        assert_eq!(rt.chat_history().messages.len(), 3);
        assert_eq!(rt.chat_history().messages[0].text, "old room");
        assert!(rt
            .chat_history()
            .messages
            .iter()
            .any(|m| m.text == "new host reply"));
    }
    #[test]
    fn training_opens_a_solo_room_without_changing_future_online_creation() {
        let mut rt = runtime();
        rt.practice_room();
        assert!(!rt.home_open && rt.lobby.open && !rt.browser.open);
        assert!(rt.lobby.room_active && rt.lobby.connected(&rt));
        assert!(matches!(rt.session, Session::Solo(_)));
        assert!(
            !rt.lobby.solo,
            "training must not force later rooms into solo mode"
        );
        assert_eq!(
            rt.session
                .world()
                .actors
                .iter()
                .filter(|a| a.active)
                .count(),
            1
        );
        assert!(rt.lobby.enabled(&rt, Action::Start));
        rt.replace_room(
            Session::Solo(Box::new(World::room(MapId::default(), true))),
            false,
        );
        assert!(!rt.lobby.solo && rt.browser.open);
    }
    #[test]
    fn training_does_not_replace_an_existing_room() {
        let mut rt = runtime();
        let server = Server::bind("127.0.0.1:0", true).unwrap();
        let address = server.addr().unwrap();
        rt.replace_room(Session::Host(Box::new(server)), true);
        rt.practice_room();
        assert!(matches!(&rt.session, Session::Host(s) if s.addr().unwrap() == address));
        assert!(rt.lobby.notice.contains("먼저 나가"));
    }
    #[test]
    fn bot_controls_require_host_and_edit_only_bots_before_start() {
        let mut rt = runtime();
        assert!(!rt.lobby.enabled(&rt, Action::Slot(1)));
        rt.lobby.solo = true;
        rt.create_room().unwrap();
        assert_eq!(rt.session.world().team_count(0), 1);
        assert!(!rt.lobby.enabled(&rt, Action::Slot(0)));
        assert!(rt.lobby.enabled(&rt, Action::Slot(1)));
        rt.edit_bot(1);
        assert!(!rt.lobby.enabled(&rt, Action::BotHero(0)));
        assert!(!rt.lobby.enabled(&rt, Action::BotRemove));
        rt.configure_bot(Some(4));
        assert!(rt.session.world().actors[1].active);
        rt.edit_bot(1);
        assert!(rt.lobby.enabled(&rt, Action::BotRemove));
        rt.configure_bot(None);
        assert!(!rt.session.world().actors[1].active);
        rt.session.start();
        assert!(!rt.lobby.enabled(&rt, Action::Slot(1)));
        let mut client = Client::connect("127.0.0.1:28080").unwrap();
        client.received = true;
        client.slot = Some(6);
        rt.replace_room(Session::Join(Box::new(client)), true);
        assert_eq!(rt.lobby.slot_for_row(&rt, 0), 6);
        assert_eq!(rt.lobby.slot_for_row(&rt, 6), 0);
        assert!(!rt.lobby.enabled(&rt, Action::Slot(1)));
        rt.lobby.dialog = Dialog::Bot(1);
        rt.configure_bot(None);
        assert!(rt.session.world().actors[1].active);
    }
    #[test]
    fn bot_modal_hit_regions_scale_and_block_lobby_controls() {
        let mut rt = runtime();
        for size in [
            Vector2::new(960., 640.),
            Vector2::new(1920., 1200.),
            Vector2::new(1920., 800.),
        ] {
            for row in 0..PLAYERS {
                rt.lobby.dialog = Dialog::None;
                let action = Action::Slot(row);
                let (x, y, w, h) = selection::rect(size.x, size.y, action.bounds());
                assert_eq!(
                    rt.lobby.hit(size, Vector2::new(x + w / 2., y + h / 2.)),
                    Some(action)
                );
            }
            rt.lobby.dialog = Dialog::Bot(1);
            for action in rt.lobby.actions() {
                let (x, y, w, h) = selection::rect(size.x, size.y, action.bounds());
                assert_eq!(
                    rt.lobby.hit(size, Vector2::new(x + w / 2., y + h / 2.)),
                    Some(action)
                );
            }
            for action in [
                Action::Start,
                Action::Hero,
                Action::Slot(1),
                Action::Slot(6),
            ] {
                let (x, y, w, h) = selection::rect(size.x, size.y, action.bounds());
                assert_ne!(
                    rt.lobby.hit(size, Vector2::new(x + w / 2., y + h / 2.)),
                    Some(action)
                );
            }
        }
    }
    #[test]
    fn lobby_gates_start_and_guest_controls() {
        let mut rt = runtime();
        assert!(!rt.lobby.enabled(&rt, Action::Start));
        assert!(!rt.lobby.enabled(&rt, Action::Hero));
        rt.lobby.solo = true;
        rt.create_room().unwrap();
        assert!(rt.lobby.enabled(&rt, Action::Start));
        assert!(!rt.lobby.enabled(&rt, Action::Join));
        assert_eq!(rt.session.world().phase, Phase::Lobby);
        let mut client = Client::connect("127.0.0.1:28080").unwrap();
        client.received = true;
        client.slot = Some(6);
        rt.replace_room(Session::Join(Box::new(client)), true);
        assert!(!rt.lobby.enabled(&rt, Action::Start));
        assert!(!rt.lobby.enabled(&rt, Action::Map));
        assert!(rt.lobby.enabled(&rt, Action::Hero));
        if let Session::Join(c) = &mut rt.session {
            c.world.start();
        }
        assert!(rt.lobby.enabled(&rt, Action::Start));
        rt.error = Some("Disconnected".into());
        assert!(!rt.lobby.enabled(&rt, Action::Start));
    }
    #[test]
    fn map_preselection_survives_room_creation_and_running_round_locks_changes() {
        let mut rt = runtime();
        if let Session::Solo(w) = &mut rt.session {
            assert!(w.select_map(MapId::ElfCity));
        }
        rt.lobby.solo = true;
        rt.create_room().unwrap();
        assert_eq!(rt.session.world().map_id, MapId::ElfCity);
        assert!(rt.lobby.enabled(&rt, Action::Map));
        rt.session.start();
        assert!(!rt.lobby.enabled(&rt, Action::Map));
        assert!(!rt.lobby.enabled(&rt, Action::ElfCity));
        assert_eq!(rt.session.world().map_id, MapId::ElfCity);
    }

    #[test]
    fn host_start_opens_selection_for_all_twelve_players() {
        use std::{thread, time::Duration};

        for map in [MapId::FairyKingdomCastle, MapId::ElfCity] {
            let mut server = Server::bind("127.0.0.1:0", true).unwrap();
            server.world.select_map(map);
            let address = server.addr().unwrap();
            let mut host = Runtime::new(Session::Host(Box::new(server)), None, false);
            host.home_open = false;
            host.lobby.open = true;
            let mut guests: Vec<_> = (1..PLAYERS)
                .map(|_| {
                    let client = Client::connect(address).unwrap();
                    let mut rt = Runtime::new(Session::Join(Box::new(client)), None, false);
                    rt.home_open = false;
                    rt.lobby.open = true;
                    rt
                })
                .collect();
            for _ in 0..100 {
                host.session.update(Input::default()).unwrap();
                for guest in &mut guests {
                    guest.session.update(Input::default()).unwrap();
                    assert!(!guest.sync_match_start());
                }
                thread::sleep(Duration::from_millis(2));
            }
            assert_eq!(
                host.session
                    .world()
                    .actors
                    .iter()
                    .filter(|a| a.active && a.human)
                    .count(),
                PLAYERS
            );
            for guest in &mut guests {
                assert!(guest.lobby.connected(guest));
                assert!(!guest.start_lobby_match());
            }
            guests[0].lobby.dialog = Dialog::Map;
            guests[0].chat.open = true;
            guests[0].chat.insert("선택 화면에서도 유지할 초안");
            guests[1].open_lobby_selection(false);
            guests[1].confirm_selection();
            guests[2].home_open = true;
            guests[2].browser.open = true;
            guests[2].settings_menu.open = true;
            for guest in &mut guests {
                guest.keys.w = true;
                guest.keys.fire = true;
                guest.keys.score = true;
            }

            assert!(host.start_lobby_match());
            assert_eq!(host.session.world().phase, Phase::Setup);
            let mut transitions = [0; PLAYERS - 1];
            for _ in 0..100 {
                host.session.update(Input::default()).unwrap();
                assert!(!host.sync_match_start());
                for (i, guest) in guests.iter_mut().enumerate() {
                    guest.session.update(Input::default()).unwrap();
                    transitions[i] += usize::from(guest.sync_match_start());
                }
                thread::sleep(Duration::from_millis(2));
            }
            assert_eq!(transitions, [1; PLAYERS - 1]);
            for rt in std::iter::once(&host).chain(&guests) {
                assert!(selection::showing(rt));
                assert!(!rt.home_open && !rt.lobby.open && !rt.browser.open && !rt.grabbed);
                assert!(!rt.settings_menu.open);
                assert!(!rt.controls_active() && !rt.keys.w && !rt.keys.fire && !rt.keys.score);
                assert_eq!(rt.lobby.dialog, Dialog::None);
                assert!(rt.lobby.start_requested);
                assert_eq!(rt.pending_ready, None);
                assert_eq!(rt.session.world().map_id, map);
            }
            assert!(guests[0].chat.open);
            assert_eq!(guests[0].chat.draft, "선택 화면에서도 유지할 초안");
            assert!(!guests[0].chat_restore_grab);

            // A pre-start confirmation cannot send a guest back to the lobby.
            // A fresh confirmation still travels through authoritative selection.
            guests[1].confirm_selection();
            for _ in 0..100 {
                host.session.update(Input::default()).unwrap();
                for guest in &mut guests {
                    guest.session.update(Input::default()).unwrap();
                    assert!(!guest.sync_match_start());
                }
                thread::sleep(Duration::from_millis(2));
            }
            assert!(matches!(&guests[1].session, Session::Join(c)
                if !c.selection_pending() && c.selection_result == Some(SelectionResult::Accepted)));
        }
    }

    #[test]
    fn match_selection_opens_once_and_rearms_for_restart_or_a_new_room() {
        let mut rt = runtime();
        rt.practice_room();
        assert!(rt.start_lobby_match());
        assert!(selection::showing(&rt));
        rt.selecting = false;
        rt.lobby.open = true;
        assert!(!rt.sync_match_start());
        if let Session::Solo(w) = &mut rt.session {
            w.phase = Phase::Playing;
        }
        assert!(!rt.sync_match_start());
        assert!(rt.lobby.open && !rt.selecting);
        if let Session::Solo(w) = &mut rt.session {
            w.phase = Phase::Finished;
            w.phase_time = 0.0;
        }
        assert!(!rt.sync_match_start());
        assert!(!rt.start_lobby_match());
        // Results return to the lobby before the next match can start.
        rt.session.update(Input::default()).unwrap();
        assert_eq!(rt.session.world().phase, Phase::Lobby);
        assert!(!rt.sync_match_start());
        assert!(rt.start_lobby_match());
        assert!(selection::showing(&rt));
        rt.replace_room(
            Session::Solo(Box::new(World::room(MapId::ElfCity, true))),
            true,
        );
        rt.lobby.open = true;
        assert!(rt.start_lobby_match());
        assert!(selection::showing(&rt));
    }

    #[test]
    fn first_running_snapshot_opens_selection_but_auto_start_skips_it() {
        let mut rt = runtime();
        rt.practice_room();
        rt.session.start();
        if let Session::Solo(w) = &mut rt.session {
            w.phase = Phase::Playing;
        }
        assert!(rt.sync_match_start());
        assert!(selection::showing(&rt));

        let mut auto = Runtime::new(Session::Solo(Box::default()), None, true);
        auto.lobby.room_active = true;
        auto.session.start();
        assert!(!auto.sync_match_start());
        assert!(!auto.selecting);
    }
    #[test]
    fn modal_hit_targets_do_not_activate_underlying_buttons() {
        let mut rt = runtime();
        for size in [
            Vector2::new(960.0, 640.0),
            Vector2::new(1920.0, 1200.0),
            Vector2::new(1920.0, 800.0),
        ] {
            for action in BUTTONS {
                let (x, y, w, h) = selection::rect(size.x, size.y, action.bounds());
                let p = Vector2::new(x + w * 0.5, y + h * 0.5);
                rt.lobby.dialog = Dialog::None;
                assert_eq!(rt.lobby.hit(size, p), Some(action));
                rt.lobby.dialog = Dialog::Join;
                assert_ne!(rt.lobby.hit(size, p), Some(action));
            }
        }
    }
    #[test]
    fn address_validation_and_failed_join_keep_existing_session() {
        for value in [
            "",
            "localhost:28080",
            "127.0.0.1:0",
            "0.0.0.0:28080",
            "[::]:28080",
            "239.0.0.1:28080",
            "127.0.0.1:65536",
        ] {
            assert!(parse_address(value).is_err());
        }
        for value in ["192.168.0.2:28080", "127.0.0.1:28080", "[::1]:28080"] {
            assert!(parse_address(value).is_ok());
        }
        let mut rt = runtime();
        rt.lobby.address.insert("bad");
        assert!(rt.join_room().is_err());
        assert!(matches!(rt.session, Session::Solo(_)) && !rt.lobby.room_active);
    }

    #[test]
    fn joined_room_receives_names_teams_and_host_start() {
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let addr = server.addr().unwrap();
        server.player_names[0] = "우리 방장".into();
        let mut guest = runtime();
        guest.lobby.address.insert(&addr.to_string());
        guest.join_room().unwrap();
        assert!(!guest.lobby.enabled(&guest, Action::Start));
        for _ in 0..100 {
            guest.session.update(Input::default()).unwrap();
            server.update(Input::default()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(guest.lobby.connected(&guest));
        assert_eq!(team(guest.session.slot()), 1);
        assert_eq!(guest.session.player_name(0), "우리 방장");
        assert!(guest.lobby.enabled(&guest, Action::Hero));
        assert!(!guest.lobby.enabled(&guest, Action::Start));
        assert_eq!(server.world.phase, Phase::Lobby);
        server.start();
        for _ in 0..20 {
            server.update(Input::default()).unwrap();
            guest.session.update(Input::default()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_ne!(guest.session.world().phase, Phase::Lobby);
        assert!(guest.lobby.enabled(&guest, Action::Start));
        assert!(!guest.lobby.enabled(&guest, Action::Map));
    }
}
