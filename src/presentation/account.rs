//! Native social/profile screens sharing the room directory's device session.
use super::lobby::background;
use super::*;
use fyrox::gui::copypasta::ClipboardProvider;
use fyrox::gui::{
    button::{ButtonContent, ButtonMessage},
    canvas::CanvasBuilder,
    file_browser::{FileSelector, FileSelectorBuilder, FileSelectorMessage, FileType, PathFilter},
    image::{ImageBuilder, ImageMessage},
    window::{WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
};
use std::time::Duration;
use trickcal_shooting_balddagu::account::{
    Action as Request, Dashboard, Person, Profile, Response,
};
use trickcal_shooting_balddagu::room_directory::{LinkStatus, PlayerProfile};

const PAGE: usize = 5;
const COLORS: [&str; 4] = ["coral", "mint", "blue", "violet"];
const COLOR_NAMES: [&str; 4] = ["코랄", "민트", "블루", "바이올렛"];
const FIELD_NAMES: [&str; 5] = [
    "이름 · 본인에게만 표시",
    "닉네임",
    "사용자 ID · 영문 소문자/숫자/밑줄",
    "소개",
    "사용자 ID 또는 닉네임",
];
const LIMITS: [usize; 5] = [50, 32, 24, 240, 32];
type Bounds = (f32, f32, f32, f32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Friends,
    Incoming,
    Outgoing,
    Search,
    Blocks,
    Invites,
    Profile,
}
const TABS: [Tab; 7] = [
    Tab::Friends,
    Tab::Incoming,
    Tab::Outgoing,
    Tab::Search,
    Tab::Blocks,
    Tab::Invites,
    Tab::Profile,
];
impl Tab {
    fn name(self) -> &'static str {
        match self {
            Self::Friends => "내 친구",
            Self::Incoming => "받은 요청",
            Self::Outgoing => "보낸 요청",
            Self::Search => "친구 찾기",
            Self::Blocks => "차단 관리",
            Self::Invites => "방 초대",
            Self::Profile => "내 프로필",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Home,
    Tab(Tab),
    Refresh,
    Field(usize),
    Color(usize),
    Save,
    Search,
    BlockHandle,
    Row(usize, usize),
    Previous,
    Next,
    Login,
    Confirm,
    Cancel,
    Photo,
    PhotoPick,
    PhotoSave,
    PhotoReset,
    PhotoCancel,
}
#[derive(Clone)]
struct Control {
    action: Action,
    bounds: Bounds,
    label: String,
    enabled: bool,
}

#[derive(Debug)]
pub(super) struct Account {
    pub open: bool,
    pub review: bool,
    epoch: u64,
    tab: Tab,
    data: Option<Dashboard>,
    results: Vec<Person>,
    searched: bool,
    fields: [ChatInput; 5],
    color: usize,
    dirty: bool,
    selected_all: bool,
    page: usize,
    focus: Action,
    busy: Option<Request>,
    serial: u64,
    notice: String,
    confirm: Option<(String, Option<Request>)>,
    ime_active: bool,
    last_refresh: Instant,
    photo_open: bool,
    photo_preview: Option<TextureResource>,
    photo_loading: bool,
    picker_requested: bool,
}
impl Default for Account {
    fn default() -> Self {
        Self {
            open: false,
            review: std::env::args().any(|a| a == "--account-review"),
            epoch: 0,
            tab: Tab::Friends,
            data: None,
            results: vec![],
            searched: false,
            fields: std::array::from_fn(|_| ChatInput::default()),
            color: 0,
            dirty: false,
            selected_all: false,
            page: 0,
            focus: Action::Home,
            busy: None,
            serial: 0,
            notice: String::new(),
            confirm: None,
            ime_active: false,
            last_refresh: Instant::now(),
            photo_open: false,
            photo_preview: None,
            photo_loading: false,
            picker_requested: false,
        }
    }
}
impl Account {
    fn people(&self) -> Vec<&Person> {
        match self.tab {
            Tab::Search => self.results.iter().collect(),
            _ => self
                .data
                .iter()
                .flat_map(|d| d.relationships.iter())
                .filter(|p| match self.tab {
                    Tab::Friends => p.status.as_deref() == Some("accepted"),
                    Tab::Incoming => p.status.as_deref() == Some("pending") && p.incoming,
                    Tab::Outgoing => p.status.as_deref() == Some("pending") && !p.incoming,
                    _ => false,
                })
                .collect(),
        }
    }
    fn count(&self) -> usize {
        match self.tab {
            Tab::Blocks => self.data.as_ref().map_or(0, |d| d.blocks.len()),
            Tab::Invites => self.data.as_ref().map_or(0, |d| d.invites.len()),
            _ => self.people().len(),
        }
    }
    fn fill_profile(&mut self) {
        if let Some(p) = self.data.as_ref().and_then(|d| d.profile.as_ref()) {
            for (i, value) in [&p.real_name, &p.display_name, &p.handle, &p.bio]
                .iter()
                .enumerate()
            {
                self.fields[i].draft = (*value).clone();
                self.fields[i].caret = value.len();
                self.fields[i].preedit.clear();
            }
            self.color = COLORS.iter().position(|c| *c == p.avatar).unwrap_or(0);
        }
    }
    fn editing(&self) -> Option<usize> {
        match self.focus {
            Action::Field(i) if self.busy.is_none() && self.confirm.is_none() => Some(i),
            _ => None,
        }
    }
    fn insert(&mut self, index: usize, value: &str) {
        let input = &mut self.fields[index];
        if self.selected_all {
            input.clear();
            self.selected_all = false;
            if index < 4 {
                self.dirty = true;
            }
        }
        let remaining = LIMITS[index].saturating_sub(input.draft.chars().count());
        let value: String = value
            .chars()
            .filter(|c| !c.is_control())
            .take(remaining)
            .collect();
        input.draft.insert_str(input.caret, &value);
        input.caret += value.len();
        if index < 4 && !value.is_empty() {
            self.dirty = true;
        }
    }
    fn controls(&self, linked: bool, active_room: bool) -> Vec<Control> {
        let mut list = Vec::new();
        let mut add = |action, bounds, label: String, enabled| {
            list.push(Control {
                action,
                bounds,
                label,
                enabled,
            })
        };
        if self.confirm.is_some() {
            add(Action::Cancel, (410., 460., 220., 52.), "취소".into(), true);
            add(
                Action::Confirm,
                (650., 460., 220., 52.),
                "확인".into(),
                true,
            );
            return list;
        }
        if self.photo_open {
            let ready = self.busy.is_none() && !self.photo_loading;
            add(
                Action::PhotoPick,
                (380., 474., 220., 48.),
                "사진 파일 선택".into(),
                ready,
            );
            add(
                Action::PhotoSave,
                (640., 474., 260., 48.),
                "사진 저장".into(),
                ready && self.photo_preview.is_some(),
            );
            add(
                Action::PhotoReset,
                (380., 538., 220., 48.),
                "기본 사진으로 변경".into(),
                ready,
            );
            add(
                Action::PhotoCancel,
                (640., 538., 260., 48.),
                "취소 / 돌아가기".into(),
                self.busy.is_none(),
            );
            return list;
        }
        add(
            Action::Home,
            (40., 716., 210., 48.),
            "← 홈으로  Esc".into(),
            true,
        );
        for (i, tab) in TABS.into_iter().enumerate() {
            let count = self.data.as_ref().map_or(0, |d| match tab {
                Tab::Friends => d
                    .relationships
                    .iter()
                    .filter(|p| p.status.as_deref() == Some("accepted"))
                    .count(),
                Tab::Incoming => d
                    .relationships
                    .iter()
                    .filter(|p| p.status.as_deref() == Some("pending") && p.incoming)
                    .count(),
                Tab::Outgoing => d
                    .relationships
                    .iter()
                    .filter(|p| p.status.as_deref() == Some("pending") && !p.incoming)
                    .count(),
                Tab::Blocks => d.blocks.len(),
                Tab::Invites => d.invites.len(),
                _ => 0,
            });
            let label = if matches!(tab, Tab::Profile | Tab::Search) {
                tab.name().into()
            } else {
                format!("{}  {count}", tab.name())
            };
            add(
                Action::Tab(tab),
                (40., 214. + i as f32 * 58., 210., 48.),
                label,
                true,
            );
        }
        if !linked {
            add(
                Action::Login,
                (570., 410., 320., 56.),
                "로그인 / 계정 연결".into(),
                true,
            );
            return list;
        }
        let ready = self.busy.is_none();
        add(
            Action::Refresh,
            (1040., 48., 192., 44.),
            "새로고침  F5".into(),
            ready,
        );
        if self.tab == Tab::Profile {
            if self.data.is_none() {
                return list;
            }
            add(
                Action::Photo,
                (875., 151., 220., 42.),
                "프로필 사진 변경".into(),
                ready,
            );
            for i in 0..4 {
                add(
                    Action::Field(i),
                    (330., 218. + i as f32 * 88., 860., 48.),
                    String::new(),
                    ready,
                );
            }
            for (i, name) in COLOR_NAMES.iter().enumerate() {
                add(
                    Action::Color(i),
                    (330. + i as f32 * 145., 584., 130., 42.),
                    name.to_string(),
                    ready,
                );
            }
            add(
                Action::Save,
                (974., 642., 216., 50.),
                "프로필 저장".into(),
                ready && self.dirty,
            );
        } else {
            if matches!(self.tab, Tab::Search | Tab::Blocks) {
                add(
                    Action::Field(4),
                    (330., 160., 640., 44.),
                    String::new(),
                    ready,
                );
                add(
                    if self.tab == Tab::Search {
                        Action::Search
                    } else {
                        Action::BlockHandle
                    },
                    (988., 160., 202., 44.),
                    if self.tab == Tab::Search {
                        "검색"
                    } else {
                        "사용자 ID 차단"
                    }
                    .into(),
                    ready,
                );
            }
            let people = self.people();
            for row in 0..PAGE {
                let index = self.page * PAGE + row;
                if index >= self.count() {
                    break;
                }
                let y = 240. + row as f32 * 76.;
                let mut row_button = |slot, name: &str, enabled| {
                    add(
                        Action::Row(row, slot),
                        (884. + slot as f32 * 104., y, 96., 40.),
                        name.into(),
                        ready && enabled,
                    )
                };
                match self.tab {
                    Tab::Friends => {
                        row_button(
                            0,
                            "초대",
                            self.data.as_ref().is_some_and(|d| !d.rooms.is_empty()),
                        );
                        row_button(1, "삭제", true);
                        row_button(2, "차단", true);
                    }
                    Tab::Incoming => {
                        row_button(1, "수락", true);
                        row_button(2, "거절", true);
                    }
                    Tab::Outgoing => row_button(2, "요청 취소", true),
                    Tab::Search => {
                        let p = people[index];
                        let relation = self.data.as_ref().and_then(|d| {
                            d.relationships
                                .iter()
                                .find(|r| r.user_id.as_deref() == Some(p.id.as_str()))
                        });
                        let status = relation
                            .and_then(|r| r.status.as_deref())
                            .or(p.relationship_status.as_deref());
                        row_button(
                            1,
                            match status {
                                Some("accepted") => "내 친구",
                                Some(_) => "요청 대기",
                                None => "친구 추가",
                            },
                            status.is_none(),
                        );
                        row_button(2, "차단", true);
                    }
                    Tab::Blocks => row_button(2, "차단 해제", true),
                    Tab::Invites => {
                        row_button(1, "참여", !active_room);
                        row_button(2, "거절", true);
                    }
                    _ => {}
                }
            }
            add(
                Action::Previous,
                (906., 640., 132., 44.),
                "← 이전".into(),
                self.page > 0 && ready,
            );
            add(
                Action::Next,
                (1054., 640., 136., 44.),
                "다음 →".into(),
                (self.page + 1) * PAGE < self.count() && ready,
            );
        }
        list
    }
}

fn send(rt: &mut Runtime, action: Request) {
    if rt.account.busy.is_some() {
        return;
    }
    rt.account.epoch = rt.browser.account_epoch();
    rt.account.serial += 1;
    rt.account.busy = Some(action.clone());
    rt.account.last_refresh = Instant::now();
    rt.browser.account_request(rt.account.serial, action);
}
pub(super) fn open(rt: &mut Runtime, profile: bool) {
    rt.account.open = true;
    rt.account.tab = if profile { Tab::Profile } else { Tab::Friends };
    rt.account.page = 0;
    rt.account.focus = Action::Tab(rt.account.tab);
    rt.keys = Keys::default();
    if rt.browser.linked.is_some() {
        send(rt, Request::Dashboard);
    }
}
fn close(rt: &mut Runtime) {
    rt.account.open = false;
    rt.account.confirm = None;
    rt.account.dirty = false;
    rt.account.fill_profile();
    rt.keys = Keys::default();
}
pub(super) fn apply_reply(rt: &mut Runtime, serial: u64, result: Result<Response, String>) {
    if serial != rt.account.serial {
        return;
    }
    let pending = rt.account.busy.take();
    match result {
        Ok(Response::Expired) => {
            rt.browser.apply_link_status(LinkStatus::Expired);
            rt.account = Account {
                open: rt.account.open,
                tab: rt.account.tab,
                serial,
                epoch: rt.browser.account_epoch(),
                notice: "로그인이 만료됐어요. 다시 로그인해 주세요.".into(),
                ..Default::default()
            };
        }
        Ok(Response::Dashboard(data)) => {
            // Keep existing search results actionable after a request is cancelled
            // or a friendship is removed from another tab/client.
            for person in &mut rt.account.results {
                let relationship = data
                    .relationships
                    .iter()
                    .find(|r| r.user_id.as_deref() == Some(person.id.as_str()));
                person.relationship_status = relationship.and_then(|r| r.status.clone());
                person.incoming = relationship.is_some_and(|r| r.incoming);
            }
            rt.account
                .results
                .retain(|p| !data.blocks.iter().any(|b| b.user_id == p.id));
            if let Some(p) = &data.profile {
                rt.browser
                    .apply_link_status(LinkStatus::Connected(PlayerProfile {
                        display_name: p.display_name.clone(),
                        handle: Some(p.handle.clone()),
                        avatar: Some(p.avatar.clone()),
                        avatar_url: p.avatar_url.clone(),
                    }));
            }
            rt.account.data = Some(data);
            if !rt.account.dirty {
                rt.account.fill_profile();
            }
        }
        Ok(Response::Search(people)) => {
            rt.account.results = people;
            rt.account.searched = true;
            rt.account.page = 0;
            rt.account.notice.clear();
        }
        Ok(Response::Changed) => {
            rt.account.notice = match pending {
                Some(Request::Save(_)) => {
                    rt.account.dirty = false;
                    "프로필을 저장했어요."
                }
                Some(Request::UploadPhoto) => {
                    rt.account.photo_open = false;
                    rt.account.photo_preview = None;
                    "프로필 사진을 저장했어요."
                }
                Some(Request::RemovePhoto) => "기본 프로필 사진으로 변경했어요.",
                Some(Request::Request(_)) => "친구 요청을 보냈어요.",
                Some(Request::Accept(_)) => "친구 요청을 수락했어요.",
                Some(Request::Invite { .. }) => "방 초대를 보냈어요.",
                Some(Request::Block(_)) => "사용자를 차단했어요.",
                Some(Request::Unblock(_)) => "차단을 해제했어요.",
                _ => "변경 사항을 저장했어요.",
            }
            .into();
            if matches!(pending, Some(Request::Block(_))) {
                rt.account.results.clear();
                rt.account.searched = false;
            }
            send(rt, Request::Dashboard);
            rt.browser.refresh_profile();
        }
        Ok(Response::Joined { code, session }) => {
            let result = Client::connect_address(&session.address).and_then(|c| {
                c.with_invitation(
                    session
                        .ticket
                        .ok_or_else(|| std::io::Error::other("참가권을 확인해 주세요."))?,
                )
            });
            match result {
                Ok(client) => {
                    close(rt);
                    rt.replace_room(Session::Join(Box::new(client)), true);
                    rt.browser.room_code = code;
                }
                Err(_) => {
                    rt.account.notice =
                        "방에 연결하지 못했어요. 초대 목록에서 다시 시도해 주세요.".into()
                }
            }
        }
        Err(error) => rt.account.notice = error,
    }
    rt.account.page = rt
        .account
        .page
        .min(rt.account.count().saturating_sub(1) / PAGE);
}
pub(super) fn update(rt: &mut Runtime) {
    if rt.account.epoch != rt.browser.account_epoch()
        || (rt.browser.linked.is_none() && rt.account.data.is_some())
    {
        rt.account = Account {
            open: rt.account.open,
            tab: rt.account.tab,
            serial: rt.account.serial + 1,
            epoch: rt.browser.account_epoch(),
            ..Default::default()
        };
    }
    if rt.account.open
        && !rt.startup.active()
        && rt.browser.linked.is_some()
        && rt.account.busy.is_none()
        && ((rt.account.data.is_none()
            && rt.account.last_refresh.elapsed() >= Duration::from_secs(3))
            || (rt.account.tab != Tab::Profile
                && rt.account.last_refresh.elapsed() >= Duration::from_secs(20)))
    {
        send(rt, Request::Dashboard);
    }
}
fn choose(rt: &mut Runtime, action: Action) {
    rt.account.focus = action;
    rt.account.selected_all = false;
    match action {
        Action::Photo => {
            rt.account.photo_open = true;
            rt.account.focus = Action::PhotoPick;
            rt.account.notice.clear();
        }
        Action::PhotoPick => {
            rt.account.picker_requested = true;
        }
        Action::PhotoSave => {
            send(rt, Request::UploadPhoto);
        }
        Action::PhotoReset => {
            rt.account.photo_open = false;
            rt.account.photo_preview = None;
            rt.browser.discard_photo();
            rt.account.confirm = Some((
                "기본 프로필 사진으로 변경할까요?".into(),
                Some(Request::RemovePhoto),
            ));
            rt.account.focus = Action::Cancel;
        }
        Action::PhotoCancel => {
            rt.account.photo_open = false;
            rt.account.photo_preview = None;
            rt.account.photo_loading = false;
            rt.account.serial += 1;
            rt.account.picker_requested = false;
            rt.browser.discard_photo();
        }
        Action::Home => {
            if rt.account.dirty {
                rt.account.confirm = Some((
                    "저장하지 않은 프로필 변경을 버리고 홈으로 돌아갈까요?".into(),
                    None,
                ));
                rt.account.focus = Action::Cancel;
            } else {
                close(rt);
            }
        }
        Action::Tab(tab) => {
            rt.account.selected_all = false;
            rt.account.tab = tab;
            rt.account.page = 0;
            rt.account.focus = Action::Tab(tab);
        }
        Action::Login => rt.startup.choices(),
        Action::Cancel => {
            rt.account.confirm = None;
            rt.account.focus = Action::Tab(rt.account.tab);
        }
        Action::Confirm => {
            if let Some((_, request)) = rt.account.confirm.take() {
                if let Some(request) = request {
                    send(rt, request);
                } else {
                    close(rt);
                }
            }
        }
        _ if rt.account.busy.is_some() => {}
        Action::Refresh => {
            rt.account.notice.clear();
            send(rt, Request::Dashboard);
        }
        Action::Field(i) => {
            rt.account.selected_all = false;
            rt.account.focus = action;
            rt.account.fields[i].caret = rt.account.fields[i].draft.len();
        }
        Action::Color(i) => {
            rt.account.color = i;
            rt.account.dirty = true;
        }
        Action::Previous => rt.account.page = rt.account.page.saturating_sub(1),
        Action::Next => rt.account.page += 1,
        Action::Search => {
            let query = rt.account.fields[4].draft.trim().to_string();
            if query.chars().count() < 2 {
                rt.account.notice = "사용자 ID 또는 닉네임을 2자 이상 입력해 주세요.".into();
            } else {
                send(rt, Request::Search(query));
            }
        }
        Action::BlockHandle => {
            let handle = rt.account.fields[4].draft.trim().to_lowercase();
            if !valid_handle(&handle) {
                rt.account.notice = "사용자 ID는 영문 소문자·숫자·밑줄 3~24자입니다.".into();
            } else {
                rt.account.confirm = Some((
                    format!("@{handle} 님을 차단할까요? 친구 관계와 초대가 삭제됩니다."),
                    Some(Request::Block(handle)),
                ));
                rt.account.focus = Action::Cancel;
            }
        }
        Action::Save => {
            let p = Profile {
                real_name: rt.account.fields[0].draft.trim().into(),
                display_name: rt.account.fields[1].draft.trim().into(),
                handle: rt.account.fields[2].draft.trim().to_lowercase(),
                bio: rt.account.fields[3].draft.trim().into(),
                avatar: COLORS[rt.account.color].into(),
                avatar_url: None,
            };
            if p.real_name.is_empty() || p.display_name.is_empty() {
                rt.account.notice = "이름과 닉네임을 입력해 주세요.".into();
            } else if !valid_handle(&p.handle) {
                rt.account.notice = "사용자 ID는 영문 소문자·숫자·밑줄 3~24자입니다.".into();
            } else {
                send(rt, Request::Save(p));
            }
        }
        Action::Row(row, slot) => {
            let index = rt.account.page * PAGE + row;
            let (request, confirm) = match rt.account.tab {
                Tab::Blocks => {
                    let Some(p) = rt.account.data.as_ref().and_then(|d| d.blocks.get(index)) else {
                        return;
                    };
                    (Request::Unblock(p.user_id.clone()), None)
                }
                Tab::Invites => {
                    let Some(p) = rt.account.data.as_ref().and_then(|d| d.invites.get(index))
                    else {
                        return;
                    };
                    if slot == 1 && rt.lobby.room_active {
                        return;
                    }
                    (
                        if slot == 1 {
                            Request::Join(p.id.clone())
                        } else {
                            Request::Decline(p.id.clone())
                        },
                        None,
                    )
                }
                _ => {
                    let people = rt.account.people();
                    let Some(p) = people.get(index) else {
                        return;
                    };
                    match (rt.account.tab, slot) {
                        (Tab::Friends, 0) => {
                            let Some(room) = rt.account.data.as_ref().and_then(|d| d.rooms.first())
                            else {
                                return;
                            };
                            (
                                Request::Invite {
                                    session: room.id.clone(),
                                    user: p.user_id.clone().unwrap_or_default(),
                                },
                                None,
                            )
                        }
                        (Tab::Incoming, 1) => (Request::Accept(p.id.clone()), None),
                        (Tab::Search, 1) => (Request::Request(p.handle.clone()), None),
                        (Tab::Friends | Tab::Search, 2) => (
                            Request::Block(p.handle.clone()),
                            Some(format!(
                                "{} 님을 차단할까요? 친구 관계와 초대가 삭제됩니다.",
                                p.display_name
                            )),
                        ),
                        _ => (
                            Request::Remove(p.id.clone()),
                            Some(format!(
                                "{} 님의 {}할까요?",
                                p.display_name,
                                match rt.account.tab {
                                    Tab::Friends => "친구 관계를 삭제",
                                    Tab::Incoming => "친구 요청을 거절",
                                    _ => "친구 요청을 취소",
                                }
                            )),
                        ),
                    }
                }
            };
            if let Some(message) = confirm {
                rt.account.confirm = Some((message, Some(request)));
                rt.account.focus = Action::Cancel;
            } else {
                send(rt, request);
            }
        }
    }
}
fn valid_handle(s: &str) -> bool {
    (3..=24).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

pub(super) fn on_event(rt: &mut Runtime, event: &Event<()>, context: &PluginContext) -> bool {
    if !rt.account.open {
        return false;
    }
    if rt.account.review {
        return true;
    }
    if rt.hud.account.picker.is_some() {
        return true;
    }
    let window = &context.graphics_context.as_initialized_ref().window;
    let size = context.user_interfaces.first().screen_size();
    let controls = rt
        .account
        .controls(rt.browser.linked.is_some(), rt.lobby.room_active);
    let mut action = None;
    match event {
        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => rt.cursor = Vector2::new(position.x as f32, position.y as f32),
        Event::WindowEvent {
            event:
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } => {
            action = controls
                .iter()
                .find(|c| {
                    c.enabled
                        && selection::contains(rt.cursor, selection::rect(size.x, size.y, c.bounds))
                })
                .map(|c| c.action);
        }
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } if event.state == ElementState::Pressed => {
            let editing = rt.account.editing().filter(|_| rt.browser.linked.is_some());
            if editing.is_some_and(|i| !rt.account.fields[i].preedit.is_empty()) {
                return true;
            }
            match event.physical_key {
                PhysicalKey::Code(KeyCode::F10) => context.loop_controller.exit(),
                PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                    action = Some(if rt.account.confirm.is_some() {
                        Action::Cancel
                    } else if rt.account.photo_open {
                        Action::PhotoCancel
                    } else {
                        Action::Home
                    })
                }
                PhysicalKey::Code(KeyCode::F5) if !event.repeat && rt.browser.linked.is_some() => {
                    action = Some(Action::Refresh)
                }
                PhysicalKey::Code(KeyCode::Tab) if !event.repeat => {
                    rt.account.selected_all = false;
                    let actions: Vec<_> = controls
                        .iter()
                        .filter(|c| c.enabled)
                        .map(|c| c.action)
                        .collect();
                    let index = actions
                        .iter()
                        .position(|a| *a == rt.account.focus)
                        .unwrap_or(0);
                    rt.account.focus = actions[(index
                        + if rt.modifiers.shift_key() {
                            actions.len() - 1
                        } else {
                            1
                        })
                        % actions.len()];
                }
                PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) if !event.repeat => {
                    action = Some(match editing {
                        Some(4) => {
                            if rt.account.tab == Tab::Blocks {
                                Action::BlockHandle
                            } else {
                                Action::Search
                            }
                        }
                        Some(_) => Action::Save,
                        None => rt.account.focus,
                    });
                }
                PhysicalKey::Code(KeyCode::KeyV)
                    if editing.is_some() && rt.modifiers.control_key() =>
                {
                    let value = context
                        .user_interfaces
                        .first()
                        .clipboard_mut()
                        .and_then(|mut c| c.get_contents().ok());
                    if let Some(value) = value {
                        rt.account.insert(editing.unwrap(), &value);
                    } else {
                        rt.account.notice = "클립보드를 읽지 못했어요.".into();
                    }
                }
                PhysicalKey::Code(KeyCode::KeyA)
                    if editing.is_some() && rt.modifiers.control_key() =>
                {
                    rt.account.selected_all = true;
                }
                _ if editing.is_some() => {
                    let i = editing.unwrap();
                    if rt.account.selected_all
                        && matches!(
                            event.physical_key,
                            PhysicalKey::Code(KeyCode::Backspace | KeyCode::Delete)
                        )
                    {
                        rt.account.insert(i, "");
                        return true;
                    }
                    if matches!(
                        event.physical_key,
                        PhysicalKey::Code(
                            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Home | KeyCode::End
                        )
                    ) {
                        rt.account.selected_all = false;
                    }
                    let input = &mut rt.account.fields[i];
                    let before = input.draft.clone();
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Backspace) => input.backspace(),
                        PhysicalKey::Code(KeyCode::Delete) => input.delete(),
                        PhysicalKey::Code(KeyCode::ArrowLeft) => input.left(),
                        PhysicalKey::Code(KeyCode::ArrowRight) => input.right(),
                        PhysicalKey::Code(KeyCode::Home) => input.caret = 0,
                        PhysicalKey::Code(KeyCode::End) => input.caret = input.draft.len(),
                        _ if !rt.modifiers.control_key() && !rt.modifiers.super_key() => {
                            if let Some(value) = &event.text {
                                rt.account.insert(i, value);
                            }
                        }
                        _ => {}
                    }
                    if i < 4 && before != rt.account.fields[i].draft {
                        rt.account.dirty = true;
                    }
                }
                _ => {}
            }
        }
        Event::WindowEvent {
            event: WindowEvent::Ime(Ime::Commit(value)),
            ..
        } => {
            if let Some(i) = rt.account.editing() {
                rt.account.fields[i].preedit.clear();
                rt.account.insert(i, value);
            }
        }
        Event::WindowEvent {
            event: WindowEvent::Ime(Ime::Preedit(value, _)),
            ..
        } => {
            if let Some(i) = rt.account.editing() {
                rt.account.fields[i].preedit = value.chars().take(LIMITS[i]).collect();
            }
        }
        Event::WindowEvent {
            event: WindowEvent::Focused(false),
            ..
        } => {
            rt.keys = Keys::default();
            for field in &mut rt.account.fields {
                field.preedit.clear();
            }
            set_cursor(window, false);
        }
        Event::WindowEvent {
            event: WindowEvent::Focused(true),
            ..
        } if rt.browser.linked.is_some() && !rt.account.photo_open => {
            send(rt, Request::Dashboard);
        }
        _ => {}
    }
    if let Some(action) = action {
        if controls.iter().any(|c| c.action == action && c.enabled) || action == Action::Home {
            choose(rt, action);
        }
    }
    sync_ime(rt, window);
    true
}
pub(super) fn sync_ime(rt: &mut Runtime, window: &Window) {
    let active = rt.hud.account.picker.is_some()
        || (rt.account.open
            && !rt.startup.active()
            && rt.browser.linked.is_some()
            && !rt.account.photo_open
            && rt.account.editing().is_some());
    if active != rt.account.ime_active {
        window.set_ime_allowed(active);
        rt.account.ime_active = active;
        for field in &mut rt.account.fields {
            field.preedit.clear();
        }
    }
}

pub(super) fn photo_prepared(
    rt: &mut Runtime,
    request: u64,
    result: Result<trickcal_shooting_balddagu::room_directory::AvatarImage, String>,
) {
    if request != rt.account.serial || !rt.account.photo_open {
        return;
    }
    rt.account.photo_loading = false;
    match result {
        Ok(image) => {
            rt.account.photo_preview = Texture::from_bytes(
                TextureKind::Rectangle {
                    width: image.width,
                    height: image.height,
                },
                TexturePixelKind::RGBA8,
                image.pixels,
            )
            .map(TextureResource::new_embedded);
            rt.account.notice = "미리보기를 확인하고 ‘사진 저장’을 눌러 주세요.".into();
        }
        Err(error) => rt.account.notice = error,
    }
}
pub(super) fn update_picker(rt: &mut Runtime, ui: &mut UserInterface) {
    if rt.hud.account.picker.is_some() && !rt.account.photo_open {
        ui.send(rt.hud.account.picker, WindowMessage::Close);
        ui.send(rt.hud.account.picker, WidgetMessage::Remove);
        rt.hud.account.picker = Handle::NONE;
    }
    if !rt.account.picker_requested || rt.hud.account.picker.is_some() {
        return;
    }
    rt.account.picker_requested = false;
    let folder = if rt.account.review {
        std::path::PathBuf::from("artifacts/ingame-native-login")
    } else {
        std::env::var_os("USERPROFILE")
            .map(std::path::PathBuf::from)
            .map(|p| p.join("Pictures"))
            .filter(|p| p.is_dir())
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    };
    let size = ui.screen_size();
    let picker = FileSelectorBuilder::new(
        WindowBuilder::new(
            WidgetBuilder::new()
                .with_width((size.x - 50.).clamp(400., 860.))
                .with_height((size.y - 70.).clamp(350., 600.)),
        )
        .with_title(WindowTitle::text(
            "프로필 사진 선택 · JPG / PNG / WebP · 2MB 이하",
        ))
        .open(false),
    )
    .with_path(&folder)
    .with_filter(PathFilter {
        folders_only: false,
        types: ["jpg", "jpeg", "png", "webp", "JPG", "JPEG", "PNG", "WEBP"]
            .into_iter()
            .map(|e| FileType::new_extension(e).with_description(e))
            .collect(),
    })
    .build(&mut ui.build_ctx());
    rt.hud.account.picker = picker;
    let (ok, cancel) = (ui[picker].ok, ui[picker].cancel);
    ui.send(ok, ButtonMessage::Content(ButtonContent::text("선택")));
    ui.send(cancel, ButtonMessage::Content(ButtonContent::text("취소")));
    ui.send(
        picker,
        WindowMessage::Open {
            alignment: WindowAlignment::Center,
            modal: true,
            focus_content: true,
        },
    );
    // The browser needs layout information before it can reveal the starting folder.
    ui.send(picker, FileSelectorMessage::Path(folder));
    ui.send(picker, FileSelectorMessage::FocusCurrentPath);
}
pub(super) fn on_ui_message(
    rt: &mut Runtime,
    ui: &mut UserInterface,
    message: &fyrox::gui::message::UiMessage,
) {
    let picker = rt.hud.account.picker;
    if picker.is_none() || message.destination() != picker.to_base::<UiNode>() {
        return;
    }
    let selected = match message.data::<FileSelectorMessage>() {
        Some(FileSelectorMessage::Commit(path)) => Some(path.clone()),
        Some(FileSelectorMessage::Cancel) => None,
        _ if matches!(message.data::<WindowMessage>(), Some(WindowMessage::Close)) => None,
        _ => return,
    };
    ui.send(picker, WindowMessage::Close);
    ui.send(picker, WidgetMessage::Remove);
    rt.hud.account.picker = Handle::NONE;
    if let Some(path) = selected {
        rt.account.photo_preview = None;
        rt.account.photo_loading = true;
        rt.account.serial += 1;
        rt.account.notice = "사진을 읽고 있어요…".into();
        rt.browser.prepare_photo(rt.account.serial, path);
    }
}

#[derive(Debug, Default)]
pub(super) struct AccountUi {
    root: Handle<UiNode>,
    bg: Handle<UiNode>,
    panel: Handle<UiNode>,
    labels: Vec<Handle<UiNode>>,
    buttons: Vec<Handle<UiNode>>,
    button_text: Vec<Handle<UiNode>>,
    avatar: Handle<UiNode>,
    dim: Handle<UiNode>,
    modal: Handle<UiNode>,
    modal_text: Handle<UiNode>,
    picker: Handle<FileSelector>,
}
impl AccountUi {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        let caption = |ctx: &mut BuildContext| {
            TextBuilder::new(
                WidgetBuilder::new()
                    .with_hit_test_visibility(false)
                    .with_clip_to_bounds(true)
                    .with_foreground(Brush::Solid(rgb([237, 244, 228])).into()),
            )
            .with_wrap(fyrox::gui::formatted_text::WrapMode::Letter)
            .build(ctx)
            .to_base()
        };
        let mut view = Self {
            bg: lobby::flat(ctx, rgb([16, 34, 30]), 0.),
            panel: lobby::flat(ctx, rgb([24, 45, 39]), 14.),
            labels: (0..32).map(|_| caption(ctx)).collect(),
            buttons: (0..40)
                .map(|_| lobby::flat(ctx, rgb([40, 69, 57]), 8.))
                .collect(),
            button_text: (0..40).map(|_| caption(ctx)).collect(),
            avatar: ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
                .with_sync_with_texture_size(false)
                .build(ctx)
                .to_base(),
            dim: lobby::flat(ctx, Color::from_rgba(3, 14, 10, 232), 0.),
            modal: lobby::flat(ctx, rgb([31, 53, 44]), 14.),
            modal_text: caption(ctx),
            ..Default::default()
        };
        view.root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_visibility(false)
                .with_children(
                    [view.bg, view.panel]
                        .into_iter()
                        .chain(view.labels.iter().copied())
                        .chain([view.avatar, view.dim, view.modal, view.modal_text])
                        .chain(view.buttons.iter().copied())
                        .chain(view.button_text.iter().copied())
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
            .chain(self.button_text.iter().copied())
            .chain([self.modal_text])
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface) {
        visible(ui, self.root, rt.account.open);
        if !rt.account.open {
            return;
        }
        let a = &rt.account;
        let size = ui.screen_size();
        let scale = (size.x / 1280.).min(size.y / 800.);
        let put = |h, r| {
            let (x, y, w, v) = selection::rect(size.x, size.y, r);
            place(ui, h, x, y, w, v);
            ui.send(h, WidgetMessage::MaxSize(Vector2::new(w, v)));
        };
        let txt = |i: usize, r, font, value: &str| {
            visible(ui, self.labels[i], true);
            put(self.labels[i], r);
            text_size(ui, self.labels[i], font * scale);
            text(ui, self.labels[i], value);
        };
        place(ui, self.root, 0., 0., size.x, size.y);
        place(ui, self.bg, 0., 0., size.x, size.y);
        put(self.panel, (290., 130., 942., 572.));
        for &h in &self.labels {
            visible(ui, h, false);
        }
        txt(
            0,
            (40., 48., 900., 52.),
            34.,
            if a.tab == Tab::Profile {
                "프로필"
            } else {
                "소셜"
            },
        );
        txt(
            1,
            (42., 108., 900., 28.),
            15.,
            "홈  >  계정 · 친구     /     Tab 이동 · Enter 선택 · Esc 홈으로",
        );
        txt(
            2,
            (42., 156., 218., 44.),
            19.,
            &shorten(
                rt.browser
                    .linked
                    .as_ref()
                    .map_or("게스트", |p| p.display_name.as_str()),
                10,
            ),
        );
        let linked = rt.browser.linked.is_some();
        visible(
            ui,
            self.avatar,
            a.tab == Tab::Profile && linked && rt.browser.avatar.is_some(),
        );
        if a.tab == Tab::Profile {
            put(self.avatar, (1118., 142., 64., 64.));
            ui.send(
                self.avatar,
                ImageMessage::Texture(rt.browser.avatar.clone()),
            );
        }
        if !linked {
            txt(
                3,
                (380., 278., 750., 46.),
                28.,
                "로그인하고 친구와 함께 플레이하세요",
            );
            txt(
                4,
                (380., 342., 750., 42.),
                17.,
                "계정 연결 후 이 화면에서 친구와 프로필을 관리할 수 있어요.",
            );
        } else if a.tab == Tab::Profile {
            txt(3, (330., 151., 530., 40.), 22., "나의 플레이어 프로필");
            if a.data.is_some() {
                for (i, name) in FIELD_NAMES.iter().enumerate().take(4) {
                    txt(4 + i, (330., 190. + i as f32 * 88., 850., 26.), 14., name);
                }
                txt(8, (330., 552., 600., 26.), 14., "기본 프로필 색상");
                txt(
                    9,
                    (330., 646., 620., 42.),
                    15.,
                    if a.dirty {
                        "저장하지 않은 변경 사항이 있어요."
                    } else {
                        "친구와 게임에는 닉네임이 표시됩니다."
                    },
                );
            }
        } else {
            if !matches!(a.tab, Tab::Search | Tab::Blocks) {
                txt(3, (330., 157., 820., 40.), 24., a.tab.name());
            }
            let people = a.people();
            for row in 0..PAGE {
                let index = a.page * PAGE + row;
                let y = 236. + row as f32 * 76.;
                let (name, detail) = match a.tab {
                    Tab::Blocks => match a.data.as_ref().and_then(|d| d.blocks.get(index)) {
                        Some(p) => (p.display_name.clone(), format!("@{}", p.handle)),
                        None => continue,
                    },
                    Tab::Invites => match a.data.as_ref().and_then(|d| d.invites.get(index)) {
                        Some(p) => (p.title.clone(), format!("{} 님의 초대", p.host_name)),
                        None => continue,
                    },
                    _ => match people.get(index) {
                        Some(p) => (
                            p.display_name.clone(),
                            format!(
                                "@{}   {}",
                                p.handle,
                                p.bio
                                    .chars()
                                    .filter(|c| !c.is_control())
                                    .take(24)
                                    .collect::<String>()
                            ),
                        ),
                        None => continue,
                    },
                };
                txt(4 + row * 2, (330., y, 536., 30.), 20., &shorten(&name, 24));
                txt(
                    5 + row * 2,
                    (330., y + 32., 536., 25.),
                    13.,
                    &shorten(&detail, 39),
                );
            }
            if a.count() == 0 && a.busy.is_none() && a.data.is_some() {
                txt(
                    15,
                    (330., 300., 835., 76.),
                    19.,
                    match a.tab {
                        Tab::Friends => {
                            "아직 친구가 없어요. ‘친구 찾기’에서 첫 친구를 추가해 보세요."
                        }
                        Tab::Incoming => "받은 친구 요청이 없습니다.",
                        Tab::Outgoing => "수락을 기다리는 요청이 없습니다.",
                        Tab::Search if !a.searched => {
                            "사용자 ID 또는 닉네임을 입력해 친구를 찾아보세요."
                        }
                        Tab::Search => "검색 결과가 없습니다. 사용자 ID나 닉네임을 확인해 주세요.",
                        Tab::Blocks => "차단한 사용자가 없습니다.",
                        _ => "받은 방 초대가 없습니다.",
                    },
                );
            }
            txt(
                16,
                (330., 646., 500., 28.),
                15.,
                &format!(
                    "{}명 · {} / {} 페이지",
                    a.count(),
                    a.page + 1,
                    a.count().div_ceil(PAGE).max(1)
                ),
            );
            if a.tab == Tab::Invites && rt.lobby.room_active {
                txt(
                    17,
                    (330., 608., 820., 26.),
                    14.,
                    "현재 방에서 나간 뒤 초대에 참여할 수 있어요.",
                );
            }
        }
        let notice = if a.busy.is_some() {
            "처리 중…"
        } else {
            &a.notice
        };
        txt(
            20,
            (302., 720., 915., 60.),
            16.,
            &wrap_text(&shorten(notice, 100), 50),
        );
        visible(ui, self.dim, a.confirm.is_some() || a.photo_open);
        visible(ui, self.modal, a.confirm.is_some() || a.photo_open);
        visible(ui, self.modal_text, a.confirm.is_some());
        if let Some((message, _)) = &a.confirm {
            place(ui, self.dim, 0., 0., size.x, size.y);
            put(self.modal, (350., 282., 580., 264.));
            put(self.modal_text, (390., 318., 500., 112.));
            text_size(ui, self.modal_text, 22. * scale);
            text(ui, self.modal_text, wrap_text(message, 21));
        }
        if a.photo_open {
            place(ui, self.dim, 0., 0., size.x, size.y);
            put(self.modal, (350., 175., 580., 450.));
            visible(ui, self.modal_text, true);
            put(self.modal_text, (380., 191., 520., 38.));
            text_size(ui, self.modal_text, 24. * scale);
            text(ui, self.modal_text, "프로필 사진 미리보기");
            // The preview image must be above the modal's dim layer.
            ui.send(self.avatar, WidgetMessage::Topmost);
            let preview = a
                .photo_preview
                .clone()
                .or_else(|| rt.browser.avatar.clone());
            visible(ui, self.avatar, preview.is_some());
            put(self.avatar, (545., 235., 190., 190.));
            ui.send(self.avatar, ImageMessage::Texture(preview));
            txt(
                21,
                (380., 432., 520., 32.),
                14.,
                if a.photo_loading {
                    "사진을 읽고 있어요…"
                } else {
                    "JPG · PNG · WebP / 2MB · 1,600만 화소 이하"
                },
            );
            ui.send(self.labels[21], WidgetMessage::Topmost);
        }
        let controls = a.controls(linked, rt.lobby.room_active);
        for i in 0..self.buttons.len() {
            visible(ui, self.buttons[i], i < controls.len());
            visible(ui, self.button_text[i], i < controls.len());
            let Some(c) = controls.get(i) else {
                continue;
            };
            let selected = matches!(c.action,Action::Tab(tab) if tab==a.tab)
                || matches!(c.action,Action::Color(color) if color==a.color);
            let hovered = selection::contains(rt.cursor, selection::rect(size.x, size.y, c.bounds));
            background(
                ui,
                self.buttons[i],
                if !c.enabled {
                    [29, 44, 39]
                } else if selected {
                    [65, 116, 85]
                } else if c.action == a.focus || hovered {
                    [60, 88, 69]
                } else {
                    [34, 59, 48]
                },
            );
            put(self.buttons[i], c.bounds);
            let (x, y, w, h) = c.bounds;
            put(self.button_text[i], (x + 12., y + 8., w - 24., h - 12.));
            text_size(
                ui,
                self.button_text[i],
                if w < 110. { 14. } else { 17. } * scale,
            );
            ui.send(
                self.button_text[i],
                WidgetMessage::Foreground(
                    Brush::Solid(rgb(
                        if matches!(c.action, Action::Field(_))
                            && c.action == a.focus
                            && a.selected_all
                        {
                            [255, 218, 139]
                        } else if c.enabled {
                            [237, 244, 228]
                        } else {
                            [113, 133, 120]
                        },
                    ))
                    .into(),
                ),
            );
            let value = match c.action {
                Action::Field(index) => field_text(
                    &a.fields[index],
                    a.focus == c.action,
                    if index == 4 { 28 } else { 38 },
                    FIELD_NAMES[index],
                ),
                _ => c.label.clone(),
            };
            text(ui, self.button_text[i], &value);
        }
    }
}
fn shorten(value: &str, limit: usize) -> String {
    let mut text: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        text.push('…');
    }
    text
}
fn wrap_text(value: &str, width: usize) -> String {
    let mut result = String::new();
    for (i, c) in value.chars().enumerate() {
        if i > 0 && i % width == 0 {
            result.push('\n');
        }
        result.push(c);
    }
    result
}
fn field_text(input: &ChatInput, focused: bool, width: usize, placeholder: &str) -> String {
    if input.draft.is_empty() && input.preedit.is_empty() {
        return format!("{}{}", if focused { "|  " } else { "" }, placeholder);
    }
    if !focused {
        return input.draft.chars().take(width + 8).collect();
    }
    let before: Vec<_> = input.draft[..input.caret].chars().collect();
    let start = before.len().saturating_sub(width);
    format!(
        "{}{}{}|{}",
        if start > 0 { "…" } else { "" },
        before[start..].iter().collect::<String>(),
        input.preedit,
        input.draft[input.caret..]
            .chars()
            .take(12)
            .collect::<String>()
    )
}

pub(super) fn review_update(rt: &mut Runtime, context: &PluginContext) {
    if rt.review_frames != 0 {
        return;
    }
    if rt.review_capture >= 16 {
        println!("ACCOUNT_REVIEW_OK: social, profile, photo preview, in-game picker, confirmations and errors; 1280x800, 960x640, 1920x800");
        context.loop_controller.exit();
        return;
    }
    let step = rt.review_capture;
    rt.home_open = true;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = false;
    rt.account = Account {
        open: true,
        review: true,
        ..Default::default()
    };
    let data = serde_json::json!({
        "profile":{"handle":"dotg_player","real_name":"플레이어","display_name":"닷지 친구","bio":"친구들과 함께하는 트릭컬 한 판!","avatar":"mint","avatar_url":null},
        "relationships":[
            {"id":"1","user_id":"a","handle":"erpin","display_name":"에르핀 친구","bio":"오늘도 함께 플레이해요","status":"accepted","incoming":false},
            {"id":"2","user_id":"b","handle":"butter","display_name":"버터 좋아","bio":"친구 요청을 기다리고 있어요","status":"pending","incoming":true},
            {"id":"3","user_id":"c","handle":"aya","display_name":"아야의 눈꽃","bio":"","status":"pending","incoming":false}
        ],"blocks":[{"user_id":"d","handle":"blocked_player","display_name":"차단된 플레이어"}],
        "rooms":[{"id":"room"}],"invites":[{"id":"invite","title":"함께하는 요정왕국 성","host_name":"에르핀 친구"}]
    });
    apply_reply(
        rt,
        rt.account.serial,
        Ok(Response::Dashboard(serde_json::from_value(data).unwrap())),
    );
    rt.account.tab = match step {
        1 => Tab::Incoming,
        2 => Tab::Search,
        3 => Tab::Blocks,
        4 => Tab::Invites,
        5 | 6 | 9 | 10 => Tab::Profile,
        _ => Tab::Friends,
    };
    if step == 2 {
        rt.account.results = vec![Person {
            id: "e".into(),
            user_id: None,
            handle: "new_friend".into(),
            display_name: "새로운 친구".into(),
            bio: "한 판 같이 해요!".into(),
            status: None,
            relationship_status: None,
            incoming: false,
        }];
        rt.account.searched = true;
        rt.account.fields[4].draft = "친구".into();
        rt.account.fields[4].caret = "친구".len();
    }
    if matches!(step, 5 | 6 | 9 | 10) {
        rt.account.insert(1, "의 새 닉네임");
        rt.account.focus = Action::Field(1);
    }
    if step == 6 {
        choose(rt, Action::Home);
    }
    if step == 7 {
        rt.browser.linked = None;
        rt.account.data = None;
    }
    if step == 8 {
        rt.account.notice =
            "계정 서비스에 연결하지 못했어요. 새로고침으로 다시 시도해 주세요.".into();
        rt.account.data = None;
    }
    let dimensions = match step {
        9 | 11 | 14 => (960, 640),
        10 => (1920, 800),
        _ => (1280, 800),
    };
    if step >= 12 {
        rt.account.tab = Tab::Profile;
        rt.account.photo_open = true;
        rt.account.focus = Action::PhotoPick;
        if step >= 13 {
            rt.account.photo_preview = Texture::from_bytes(
                TextureKind::Rectangle {
                    width: 16,
                    height: 16,
                },
                TexturePixelKind::RGBA8,
                [82, 180, 143, 255].repeat(16 * 16),
            )
            .map(TextureResource::new_embedded);
            rt.account.notice = "미리보기를 확인하고 ‘사진 저장’을 눌러 주세요.".into();
        }
        if step == 15 {
            rt.account.picker_requested = true;
        }
    }
    let _ = context
        .graphics_context
        .as_initialized_ref()
        .window
        .request_inner_size(fyrox::dpi::PhysicalSize::new(dimensions.0, dimensions.1));
    rt.review_frames = 1;
}
pub(super) fn review_capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if rt.review_frames == 0 {
        return Ok(());
    }
    rt.review_frames += 1;
    if rt.review_frames < 24 {
        return Ok(());
    }
    let tag = format!("account-{:02}-", rt.review_capture);
    capture_preview_impl(rt.scene, 0, &tag, true, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_panels_keep_home_and_draft_and_require_explicit_login() {
        let mut rt = Runtime::new(Session::Solo(Box::default()), None, false);
        rt.home_open = true;
        rt.startup = startup::Startup::new(false, false);
        open(&mut rt, true);
        assert!(rt.home_open && rt.account.open && !rt.startup.active());
        assert!(rt
            .account
            .controls(false, false)
            .iter()
            .any(|c| c.action == Action::Login));
        assert!(rt.account.busy.is_none());
        rt.account.insert(1, "새 닉네임");
        choose(&mut rt, Action::Tab(Tab::Friends));
        choose(&mut rt, Action::Tab(Tab::Profile));
        assert_eq!(rt.account.fields[1].draft, "새 닉네임");
        choose(&mut rt, Action::Home);
        assert!(rt.account.confirm.is_some());
        choose(&mut rt, Action::Cancel);
        assert!(rt.account.dirty && rt.account.open);
        choose(&mut rt, Action::Home);
        choose(&mut rt, Action::Confirm);
        assert!(!rt.account.open && rt.home_open);
    }
    #[test]
    fn unicode_fields_support_full_bio_and_bounded_paste() {
        let mut a = Account::default();
        a.insert(3, &"가".repeat(260));
        assert_eq!(a.fields[3].draft.chars().count(), 240);
        a.fields[3].left();
        a.fields[3].backspace();
        a.insert(3, "나");
        assert_eq!(a.fields[3].draft.chars().count(), 240);
        a.insert(2, "a\nb\tc");
        assert_eq!(a.fields[2].draft, "abc");
        assert!(valid_handle("friend_123"));
        assert!(!valid_handle("한글"));
        a.selected_all = true;
        a.insert(3, "소개 교체");
        assert_eq!(a.fields[3].draft, "소개 교체");
        assert_eq!(a.fields[3].caret, "소개 교체".len());
        assert!(!a.selected_all);
    }

    #[test]
    fn refresh_preserves_unsaved_drafts_and_account_switch_clears_them() {
        let mut rt = Runtime::new(Session::Solo(Box::default()), None, false);
        rt.account.insert(1, "작성 중인 닉네임");
        rt.account.results.push(Person {
            id: "other".into(),
            user_id: None,
            handle: "friend".into(),
            display_name: "친구".into(),
            bio: String::new(),
            status: None,
            relationship_status: Some("pending".into()),
            incoming: false,
        });
        let data: Dashboard = serde_json::from_value(serde_json::json!({
            "profile":{"handle":"player","display_name":"서버 이름","real_name":"이름","bio":"","avatar":"coral","avatar_url":null},
            "relationships":[],"blocks":[],"rooms":[],"invites":[]
        })).unwrap();
        apply_reply(&mut rt, 0, Ok(Response::Dashboard(data)));
        assert_eq!(rt.account.fields[1].draft, "작성 중인 닉네임");
        assert!(rt.account.results[0].relationship_status.is_none());
        rt.account.busy = Some(Request::Search("친구".into()));
        rt.browser.cancel_login();
        update(&mut rt);
        assert!(rt.account.fields[1].draft.is_empty());
        assert!(rt.account.busy.is_none() && rt.account.data.is_none());
    }
    #[test]
    fn expired_and_stale_responses_do_not_restore_account_data() {
        let mut rt = Runtime::new(Session::Solo(Box::default()), None, false);
        rt.account.open = true;
        rt.account.serial = 3;
        rt.account.insert(1, "private draft");
        apply_reply(&mut rt, 2, Ok(Response::Expired));
        assert!(rt.account.dirty);
        apply_reply(&mut rt, 3, Ok(Response::Expired));
        assert!(!rt.account.dirty && rt.account.open);
        assert!(rt.account.fields[1].draft.is_empty());
    }
}
