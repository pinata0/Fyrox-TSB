//! Publisher credits and an explicit account/guest gate before the home screen.
use super::*;
use fyrox::gui::copypasta::ClipboardProvider;
use fyrox::{
    asset::manager::ResourceManager,
    core::math::Rect,
    gui::{
        canvas::CanvasBuilder,
        image::{ImageBuilder, ImageMessage},
        HorizontalAlignment,
    },
};
use trickcal_shooting_balddagu::room_directory::AuthMode;

const LOGO_SECONDS: f32 = 2.6;
const FOREST: [u8; 3] = [35, 65, 55];
const MUTED: [u8; 3] = [105, 121, 109];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Logo(usize),
    Choice,
    Login,
    Waiting(AuthMode),
    Done,
}

pub(super) struct Startup {
    stage: Stage,
    elapsed: f32,
    last_frame: Option<Instant>,
    focus: usize,
    review: bool,
    review_step: u8,
    review_frames: u32,
    fields: [ChatInput; 2],
    selected_all: bool,
    input_notice: String,
    ime_active: bool,
}
impl std::fmt::Debug for Startup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Startup")
            .field("stage", &self.stage)
            .finish_non_exhaustive()
    }
}
impl Startup {
    pub fn new(enabled: bool, review: bool) -> Self {
        Self {
            stage: if enabled || review {
                Stage::Logo(0)
            } else {
                Stage::Done
            },
            elapsed: 0.0,
            last_frame: None,
            focus: 0,
            review,
            review_step: 0,
            review_frames: 0,
            fields: std::array::from_fn(|_| ChatInput::default()),
            selected_all: false,
            input_notice: String::new(),
            ime_active: false,
        }
    }
    pub fn active(&self) -> bool {
        self.stage != Stage::Done
    }
    pub fn choices(&mut self) {
        for field in &mut self.fields {
            field.clear();
        }
        self.selected_all = false;
        self.input_notice.clear();
        self.stage = Stage::Choice;
        self.focus = 0;
    }
    fn next_logo(&mut self) {
        self.stage = match self.stage {
            Stage::Logo(0) => Stage::Logo(1),
            Stage::Logo(_) => Stage::Choice,
            stage => stage,
        };
        self.elapsed = 0.0;
        self.last_frame = None;
    }
    fn advance(&mut self, dt: f32, ready: bool) {
        if matches!(self.stage, Stage::Logo(_)) && ready {
            // A stalled render or loading frame must not skip either credit.
            self.elapsed += dt.clamp(0.0, 0.1);
            if self.elapsed >= LOGO_SECONDS {
                self.next_logo();
            }
        }
    }
    fn opacity(&self) -> f32 {
        (self.elapsed / 0.45)
            .min((LOGO_SECONDS - self.elapsed) / 0.45)
            .clamp(0.0, 1.0)
    }
}

fn login_bounds(index: usize) -> (f32, f32, f32, f32) {
    match index {
        0 => (434., 536., 412., 50.),
        1 => (434., 600., 198., 44.),
        2 => (648., 600., 198., 44.),
        3 => (434., 378., 412., 48.),
        _ => (434., 466., 412., 48.),
    }
}
fn login_text(input: &ChatInput, password: bool, focused: bool) -> String {
    let clean = |s: &str| {
        if password {
            "*".repeat(s.chars().count())
        } else {
            s.to_string()
        }
    };
    let before = clean(&input.draft[..input.caret]);
    let end = before.chars().count().saturating_sub(24);
    format!(
        "{}{}{}{}",
        if end > 0 { "…" } else { "" },
        before.chars().skip(end).collect::<String>(),
        if focused {
            format!("{}|", clean(&input.preedit))
        } else {
            String::new()
        },
        clean(&input.draft[input.caret..])
            .chars()
            .take(12)
            .collect::<String>()
    )
}
fn insert_login(startup: &mut Startup, index: usize, value: &str) {
    let input = &mut startup.fields[index];
    if startup.selected_all {
        input.clear();
        startup.selected_all = false;
    }
    let limit: usize = if index == 0 { 254 } else { 128 };
    let value: String = value
        .chars()
        .filter(|c| !c.is_control())
        .take(limit.saturating_sub(input.draft.chars().count()))
        .collect();
    input.draft.insert_str(input.caret, &value);
    input.caret += value.len();
}
fn login_event(rt: &mut Runtime, event: &Event<()>, context: &PluginContext) {
    let size = context.user_interfaces.first().screen_size();
    let mut action = None;
    let editing = (3..=4)
        .contains(&rt.startup.focus)
        .then(|| rt.startup.focus - 3)
        .filter(|_| !rt.browser.login_busy());
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
            action = (0..5).find(|&i| {
                selection::contains(rt.cursor, selection::rect(size.x, size.y, login_bounds(i)))
            });
        }
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } if event.state == ElementState::Pressed => {
            if editing.is_some_and(|i| !rt.startup.fields[i].preedit.is_empty()) {
                return;
            }
            match event.physical_key {
                PhysicalKey::Code(KeyCode::Escape) if !event.repeat => action = Some(1),
                PhysicalKey::Code(KeyCode::F10) => context.loop_controller.exit(),
                PhysicalKey::Code(KeyCode::Tab) if !event.repeat => {
                    let order = if rt.browser.login_busy() {
                        vec![1, 2]
                    } else {
                        vec![3, 4, 0, 1, 2]
                    };
                    let current = order
                        .iter()
                        .position(|i| *i == rt.startup.focus)
                        .unwrap_or(0);
                    rt.startup.focus = order[(current
                        + if rt.modifiers.shift_key() {
                            order.len() - 1
                        } else {
                            1
                        })
                        % order.len()];
                    rt.startup.selected_all = false;
                }
                PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) if !event.repeat => {
                    action = Some(if editing.is_some() {
                        0
                    } else {
                        rt.startup.focus
                    })
                }
                PhysicalKey::Code(KeyCode::KeyA)
                    if editing.is_some() && rt.modifiers.control_key() =>
                {
                    rt.startup.selected_all = true
                }
                PhysicalKey::Code(KeyCode::KeyV)
                    if editing.is_some() && rt.modifiers.control_key() =>
                {
                    if let Some(value) = context
                        .user_interfaces
                        .first()
                        .clipboard_mut()
                        .and_then(|mut c| c.get_contents().ok())
                    {
                        insert_login(&mut rt.startup, editing.unwrap(), &value);
                    }
                }
                _ if editing.is_some() => {
                    let i = editing.unwrap();
                    if rt.startup.selected_all
                        && matches!(
                            event.physical_key,
                            PhysicalKey::Code(KeyCode::Backspace | KeyCode::Delete)
                        )
                    {
                        insert_login(&mut rt.startup, i, "");
                        return;
                    }
                    let field = &mut rt.startup.fields[i];
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Backspace) => field.backspace(),
                        PhysicalKey::Code(KeyCode::Delete) => field.delete(),
                        PhysicalKey::Code(KeyCode::ArrowLeft) => {
                            field.left();
                            rt.startup.selected_all = false;
                        }
                        PhysicalKey::Code(KeyCode::ArrowRight) => {
                            field.right();
                            rt.startup.selected_all = false;
                        }
                        PhysicalKey::Code(KeyCode::Home) => {
                            field.caret = 0;
                            rt.startup.selected_all = false;
                        }
                        PhysicalKey::Code(KeyCode::End) => {
                            field.caret = field.draft.len();
                            rt.startup.selected_all = false;
                        }
                        _ if !rt.modifiers.control_key() && !rt.modifiers.super_key() => {
                            if let Some(value) = &event.text {
                                insert_login(&mut rt.startup, i, value);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Event::WindowEvent {
            event: WindowEvent::Ime(Ime::Commit(value)),
            ..
        } => {
            if let Some(i) = editing {
                rt.startup.fields[i].preedit.clear();
                insert_login(&mut rt.startup, i, value);
            }
        }
        Event::WindowEvent {
            event: WindowEvent::Ime(Ime::Preedit(value, _)),
            ..
        } => {
            if let Some(i) = editing {
                rt.startup.fields[i].preedit = value.chars().take(128).collect();
            }
        }
        Event::WindowEvent {
            event: WindowEvent::Focused(false),
            ..
        } => {
            for field in &mut rt.startup.fields {
                field.preedit.clear();
            }
        }
        _ => {}
    }
    if let Some(index) = action {
        choose(rt, index);
    }
}

fn bounds(index: usize) -> (f32, f32, f32, f32) {
    (434.0, 392.0 + index as f32 * 70.0, 412.0, 56.0)
}
fn hit(size: Vector2<f32>, cursor: Vector2<f32>) -> Option<usize> {
    (0..3).find(|&i| selection::contains(cursor, selection::rect(size.x, size.y, bounds(i))))
}
fn choose(rt: &mut Runtime, index: usize) {
    match (rt.startup.stage, index) {
        (Stage::Choice, 0) => {
            rt.startup.stage = Stage::Login;
            rt.startup.focus = 3;
            rt.startup.input_notice.clear();
        }
        (Stage::Login, 0) if !rt.browser.login_busy() => {
            let email = rt.startup.fields[0].draft.trim().to_string();
            if !email.contains('@') || rt.startup.fields[1].draft.is_empty() {
                rt.startup.input_notice = "이메일과 비밀번호를 입력해 주세요.".into();
            } else {
                let password = std::mem::take(&mut rt.startup.fields[1].draft);
                rt.startup.fields[1].clear();
                rt.startup.input_notice.clear();
                rt.startup.focus = 0;
                rt.browser.authenticate(email, password);
            }
        }
        (Stage::Login, 1) => {
            rt.browser.cancel_login();
            rt.startup.choices();
        }
        (Stage::Login, 3 | 4) if !rt.browser.login_busy() => {
            rt.startup.focus = index;
            rt.startup.selected_all = false;
        }
        (Stage::Choice, 1) => {
            let mode = AuthMode::Signup;
            rt.startup.stage = Stage::Waiting(mode);
            rt.startup.focus = 0;
            rt.browser.begin_login(mode);
        }
        (Stage::Waiting(mode), 0) => rt.browser.begin_login(mode),
        (Stage::Waiting(_), 1) => {
            rt.browser.cancel_login();
            rt.startup.choices();
        }
        (Stage::Choice | Stage::Waiting(_) | Stage::Login, 2) => {
            rt.browser.cancel_login();
            rt.startup.choices();
            rt.startup.stage = Stage::Done;
            rt.keys = Keys::default();
        }
        _ => {}
    }
}

pub(super) fn on_event(rt: &mut Runtime, event: &Event<()>, context: &PluginContext) -> bool {
    if !rt.startup.active() {
        return false;
    }
    // Credits are a timed presentation; clicks and navigation keys cannot skip them.
    if matches!(rt.startup.stage, Stage::Logo(_)) {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } = event
        {
            if event.physical_key == PhysicalKey::Code(KeyCode::F10)
                && event.state == ElementState::Pressed
                && !event.repeat
            {
                context.loop_controller.exit();
            }
        }
        return true;
    }
    if rt.startup.stage == Stage::Login {
        login_event(rt, event, context);
        let active = rt.startup.stage == Stage::Login
            && (3..=4).contains(&rt.startup.focus)
            && !rt.browser.login_busy();
        if active != rt.startup.ime_active {
            context
                .graphics_context
                .as_initialized_ref()
                .window
                .set_ime_allowed(active);
            rt.startup.ime_active = active;
        }
        return true;
    }
    match event {
        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            rt.cursor = Vector2::new(position.x as f32, position.y as f32);
            if let Some(index) = hit(context.user_interfaces.first().screen_size(), rt.cursor) {
                rt.startup.focus = index;
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
            if let Some(index) = hit(context.user_interfaces.first().screen_size(), rt.cursor) {
                choose(rt, index);
            }
        }
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } if event.state == ElementState::Pressed && !event.repeat => match event.physical_key {
            PhysicalKey::Code(KeyCode::F10) => context.loop_controller.exit(),
            PhysicalKey::Code(KeyCode::Escape) => {
                if let Stage::Waiting(_) = rt.startup.stage {
                    choose(rt, 1);
                }
            }
            PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) => {
                choose(rt, rt.startup.focus);
            }
            PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::Tab)
                if !matches!(rt.startup.stage, Stage::Logo(_)) =>
            {
                let backwards = event.physical_key == PhysicalKey::Code(KeyCode::ArrowUp)
                    || (event.physical_key == PhysicalKey::Code(KeyCode::Tab)
                        && rt.modifiers.shift_key());
                rt.startup.focus = (rt.startup.focus + if backwards { 2 } else { 1 }) % 3;
            }
            _ => {}
        },
        Event::WindowEvent {
            event: WindowEvent::Focused(false),
            ..
        } => {
            rt.keys = Keys::default();
            set_cursor(&context.graphics_context.as_initialized_ref().window, false);
        }
        _ => {}
    }
    true
}

pub(super) fn resize(rt: &Runtime, ui: &mut UserInterface, size: Vector2<f32>) -> bool {
    if !rt.startup.active() || size.x <= 0.0 || size.y <= 0.0 {
        return false;
    }
    ui.set_screen_size(size);
    rt.hud.startup.render(rt, ui);
    while ui.poll_message().is_some() {}
    ui.update(size, 0.0, &Default::default());
    true
}

pub(super) fn update(
    rt: &mut Runtime,
    context: &mut PluginContext,
) -> Result<bool, fyrox::plugin::error::GameError> {
    if !rt.startup.active() {
        return Ok(false);
    }
    if rt.startup.review {
        if rt.startup.review_step >= 9 {
            println!(
                "STARTUP_REVIEW_OK: credits, native login, masked password, errors, signup link, compact/wide layout"
            );
            context.loop_controller.exit();
            return Ok(true);
        }
        let (stage, size) = match rt.startup.review_step {
            0 => (Stage::Logo(0), (1280, 800)),
            1 => (Stage::Logo(1), (1280, 800)),
            2 => (Stage::Choice, (1280, 800)),
            3 => (Stage::Login, (1280, 800)),
            4 => (Stage::Waiting(AuthMode::Signup), (1280, 800)),
            5 => (Stage::Login, (960, 640)),
            6 => (Stage::Login, (1920, 800)),
            _ => (Stage::Login, (1280, 800)),
        };
        rt.startup.stage = stage;
        rt.startup.elapsed = 1.0;
        if rt.startup.review_step >= 7 {
            if rt.startup.fields[0].draft.is_empty() {
                insert_login(&mut rt.startup, 0, "player@example.test");
                insert_login(&mut rt.startup, 1, "fixture-password");
            }
            rt.startup.focus = 4;
            if rt.startup.review_step == 8 {
                rt.startup.input_notice =
                    "이메일·비밀번호와 이메일 인증 여부를 확인해 주세요.".into();
            }
        }
        let window = &context.graphics_context.as_initialized_ref().window;
        let requested = fyrox::dpi::PhysicalSize::new(size.0, size.1);
        if window.inner_size() != requested {
            let _ = window.request_inner_size(requested);
        }
    } else if matches!(rt.startup.stage, Stage::Waiting(_) | Stage::Login)
        && rt.browser.linked.is_some()
    {
        rt.startup.choices();
        rt.startup.stage = Stage::Done;
        context
            .graphics_context
            .as_initialized_ref()
            .window
            .set_ime_allowed(false);
        rt.startup.ime_active = false;
    }
    rt.keys = Keys::default();
    rt.grabbed = false;
    let active = rt.startup.stage == Stage::Login
        && (3..=4).contains(&rt.startup.focus)
        && !rt.browser.login_busy();
    if active != rt.startup.ime_active {
        context
            .graphics_context
            .as_initialized_ref()
            .window
            .set_ime_allowed(active);
        rt.startup.ime_active = active;
    }
    // Keep explicitly supplied host/join sessions alive while the gate is open.
    if !matches!(rt.session, Session::Solo(_)) {
        if let Err(e) = rt.session.update(Input::default()) {
            rt.error = Some(e);
        }
    }
    render(rt, context)?;
    Ok(true)
}

pub(super) fn before_rendering(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if !rt.startup.review {
        let now = Instant::now();
        let dt = rt
            .startup
            .last_frame
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32());
        rt.startup.advance(dt, rt.hud.startup.ready());
        return Ok(());
    }
    rt.startup.review_frames += 1;
    if rt.startup.review_frames >= 36
        && rt.hud.startup.ready()
        && context
            .graphics_context
            .as_initialized_ref()
            .renderer
            .scene_data_map
            .contains_key(&rt.scene)
    {
        let size = context.user_interfaces.first().screen_size();
        let tag = format!(
            "startup-{}-{}x{}-",
            rt.startup.review_step, size.x as u32, size.y as u32
        );
        capture_preview_impl(rt.scene, 0, &tag, true, context)
            .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        rt.startup.review_frames = 0;
        rt.startup.review_step += 1;
    }
    Ok(())
}

#[derive(Debug, Default)]
pub(super) struct StartupUi {
    root: Handle<UiNode>,
    background: Handle<UiNode>,
    artwork: Handle<UiNode>,
    veil: Handle<UiNode>,
    card: Handle<UiNode>,
    logo: Handle<UiNode>,
    title: Handle<UiNode>,
    subtitle: Handle<UiNode>,
    detail: Handle<UiNode>,
    hint: Handle<UiNode>,
    buttons: [Handle<UiNode>; 3],
    captions: [Handle<UiNode>; 3],
    inputs: [Handle<UiNode>; 2],
    input_labels: [Handle<UiNode>; 2],
    input_values: [Handle<UiNode>; 2],
    textures: Vec<TextureResource>,
}
impl StartupUi {
    pub fn new(ui: &mut UserInterface, manager: &ResourceManager) -> Self {
        Self::with_textures(
            ui,
            [
                "assets/ui/startup/epid-games.png",
                "assets/ui/startup/dotg-logo.png",
                "assets/ui/home/trickcal-shooting-balddagu-logo.png",
                "assets/ui/home/elias-picnic-soft.png",
            ]
            .map(|p| manager.request::<Texture>(p))
            .to_vec(),
        )
    }
    fn with_textures(ui: &mut UserInterface, textures: Vec<TextureResource>) -> Self {
        let ctx = &mut ui.build_ctx();
        let make_image = |ctx: &mut BuildContext, texture| {
            ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
                .with_texture(texture)
                .with_sync_with_texture_size(false)
                .build(ctx)
                .to_base()
        };
        let mut view = Self {
            background: lobby::flat(ctx, Color::WHITE, 0.0),
            artwork: make_image(ctx, textures[3].clone()),
            veil: lobby::flat(ctx, Color::from_rgba(18, 43, 34, 150), 0.0),
            card: lobby::flat(ctx, rgb([253, 251, 243]), 20.0),
            logo: make_image(ctx, textures[0].clone()),
            title: label(ctx, "", 28.0, FOREST),
            subtitle: label(ctx, "", 16.0, MUTED),
            detail: label(ctx, "", 14.0, MUTED),
            hint: label(ctx, "", 12.0, MUTED),
            buttons: std::array::from_fn(|_| lobby::flat(ctx, rgb(FOREST), 9.0)),
            captions: std::array::from_fn(|_| label(ctx, "", 20.0, FOREST)),
            inputs: std::array::from_fn(|_| lobby::flat(ctx, rgb([236, 240, 229]), 8.)),
            input_labels: std::array::from_fn(|_| label(ctx, "", 14., MUTED)),
            input_values: std::array::from_fn(|_| label(ctx, "", 18., FOREST)),
            textures,
            ..Default::default()
        };
        for h in view.labels() {
            ctx[h]
                .cast_mut::<Text>()
                .unwrap()
                .formatted_text
                .borrow_mut()
                .set_horizontal_alignment(HorizontalAlignment::Center)
                .set_shadow(false)
                .set_wrap(fyrox::gui::formatted_text::WrapMode::Letter);
        }
        view.root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_visibility(false)
                .with_children(
                    [
                        view.background,
                        view.artwork,
                        view.veil,
                        view.card,
                        view.logo,
                    ]
                    .into_iter()
                    .chain(view.buttons)
                    .chain(view.inputs)
                    .chain(view.labels())
                    .collect::<Vec<_>>(),
                ),
        )
        .build(ctx)
        .to_base();
        view
    }
    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> + '_ {
        [self.title, self.subtitle, self.detail, self.hint]
            .into_iter()
            .chain(self.captions)
            .chain(self.input_labels)
            .chain(self.input_values)
    }
    fn ready(&self) -> bool {
        self.textures.iter().all(|t| !t.is_loading())
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface) {
        visible(ui, self.root, rt.startup.active());
        if !rt.startup.active() {
            return;
        }
        let size = ui.screen_size();
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }
        let s = (size.x / 1280.0).min(size.y / 800.0);
        let put = |h, r| {
            let (x, y, w, height) = selection::rect(size.x, size.y, r);
            place(ui, h, x, y, w, height);
        };
        let txt = |h, r, font, value: &str| {
            put(h, r);
            text_size(ui, h, font * s);
            text(ui, h, value);
        };
        for h in [self.root, self.background, self.artwork, self.veil] {
            place(ui, h, 0.0, 0.0, size.x, size.y);
        }
        let splash = matches!(rt.startup.stage, Stage::Logo(_));
        for h in self
            .inputs
            .into_iter()
            .chain(self.input_labels)
            .chain(self.input_values)
        {
            visible(ui, h, rt.startup.stage == Stage::Login);
        }
        visible(ui, self.title, !splash);
        visible(ui, self.hint, false);
        for h in [
            self.artwork,
            self.veil,
            self.card,
            self.subtitle,
            self.detail,
        ] {
            visible(ui, h, !splash);
        }
        for h in self.buttons.into_iter().chain(self.captions) {
            visible(ui, h, !splash);
        }
        if let Stage::Logo(index) = rt.startup.stage {
            ui.send(
                self.logo,
                ImageMessage::Texture(Some(self.textures[index].clone())),
            );
            ui.send(
                self.logo,
                ImageMessage::UvRect(if index == 0 {
                    // The official GIF has generous transparent side margins.
                    Rect::new(320.0 / 951.0, 0.0, 320.0 / 951.0, 1.0)
                } else {
                    Rect::new(0.0, 0.0, 1.0, 1.0)
                }),
            );
            put(
                self.logo,
                if index == 0 {
                    (440.0, 300.625, 400.0, 198.75)
                } else {
                    (480.0, 240.0, 320.0, 320.0)
                },
            );
            ui.send(
                self.logo,
                WidgetMessage::Opacity(Some(rt.startup.opacity())),
            );
            return;
        }
        // Cover the artwork without stretching on compact and ultrawide windows.
        let aspect = size.x / size.y;
        let art = 1672.0 / 941.0;
        ui.send(
            self.artwork,
            ImageMessage::UvRect(if aspect < art {
                let width = aspect / art;
                Rect::new((1.0 - width) * 0.5, 0.0, width, 1.0)
            } else {
                let height = art / aspect;
                Rect::new(0.0, (1.0 - height) * 0.5, 1.0, height)
            }),
        );
        put(self.card, (394.0, 80.0, 492.0, 636.0));
        put(self.logo, (480.0, 102.0, 320.0, 180.1));
        ui.send(
            self.logo,
            ImageMessage::Texture(Some(self.textures[2].clone())),
        );
        ui.send(
            self.logo,
            ImageMessage::UvRect(Rect::new(0.0, 0.0, 1.0, 1.0)),
        );
        for h in [self.logo, self.title] {
            ui.send(h, WidgetMessage::Opacity(Some(1.0)));
        }
        if rt.startup.stage == Stage::Login {
            txt(self.title, (414., 285., 452., 45.), 27., "DotG 로그인");
            txt(
                self.subtitle,
                (420., 333., 440., 30.),
                14.,
                "이메일·비밀번호로 게임 안에서 바로 시작하세요.",
            );
            for i in 0..2 {
                let r = login_bounds(i + 3);
                put(self.inputs[i], r);
                lobby::background(
                    ui,
                    self.inputs[i],
                    if rt.startup.focus == i + 3 {
                        [215, 233, 217]
                    } else {
                        [236, 240, 229]
                    },
                );
                txt(
                    self.input_labels[i],
                    (r.0, r.1 - 25., r.2, 22.),
                    13.,
                    if i == 0 { "이메일" } else { "비밀번호" },
                );
                txt(
                    self.input_values[i],
                    (r.0 + 12., r.1 + 12., r.2 - 24., r.3 - 14.),
                    17.,
                    &login_text(&rt.startup.fields[i], i == 1, rt.startup.focus == i + 3),
                );
            }
            for (i, caption) in [
                if rt.browser.login_busy() {
                    "로그인 중…"
                } else {
                    "로그인"
                },
                "돌아가기",
                "게스트로 시작",
            ]
            .into_iter()
            .enumerate()
            {
                let r = login_bounds(i);
                put(self.buttons[i], r);
                lobby::background(
                    ui,
                    self.buttons[i],
                    if rt.startup.focus == i {
                        [58, 108, 84]
                    } else {
                        FOREST
                    },
                );
                ui.send(
                    self.captions[i],
                    WidgetMessage::Foreground(Brush::Solid(rgb([255, 252, 238])).into()),
                );
                txt(
                    self.captions[i],
                    (r.0, r.1 + 12., r.2, r.3 - 10.),
                    18.,
                    caption,
                );
            }
            visible(ui, self.detail, true);
            visible(ui, self.hint, false);
            let notice = if rt.startup.input_notice.is_empty() {
                rt.browser.login_notice()
            } else {
                &rt.startup.input_notice
            };
            txt(self.detail, (424., 654., 432., 56.), 13., notice);
            return;
        }
        let waiting = matches!(rt.startup.stage, Stage::Waiting(_));
        txt(
            self.title,
            (414.0, 285.0, 452.0, 45.0),
            27.0,
            match rt.startup.stage {
                Stage::Waiting(AuthMode::Login) => "로그인을 기다리고 있어요",
                Stage::Waiting(AuthMode::Signup) => "새로운 만남의 시작",
                _ => "함께할 준비 되셨나요?",
            },
        );
        txt(
            self.subtitle,
            (420.0, 337.0, 440.0, 42.0),
            14.0,
            if waiting {
                "브라우저에서 인증과 연결 승인을 마치면 시작합니다."
            } else {
                "DOTG 계정으로 시작하거나 게스트로 플레이하세요."
            },
        );
        let captions = if waiting {
            [
                if rt.browser.login_busy() {
                    "준비 중..."
                } else {
                    "브라우저 다시 열기 / 재시도"
                },
                "돌아가기",
                "게스트로 로그인하기",
            ]
        } else {
            ["로그인", "회원가입", "게스트로 로그인하기"]
        };
        for (i, caption) in captions.into_iter().enumerate() {
            let focused = rt.startup.focus == i;
            let dark = i == 0;
            put(self.buttons[i], bounds(i));
            lobby::background(
                ui,
                self.buttons[i],
                if dark {
                    if focused {
                        [58, 108, 84]
                    } else {
                        FOREST
                    }
                } else if focused {
                    [215, 233, 217]
                } else {
                    [236, 240, 229]
                },
            );
            ui.send(
                self.captions[i],
                WidgetMessage::Foreground(
                    Brush::Solid(rgb(if dark { [255, 252, 238] } else { FOREST })).into(),
                ),
            );
            let (x, y, w, h) = bounds(i);
            txt(
                self.captions[i],
                (x, y + 15.0, w, h - 12.0),
                if waiting && i == 0 { 17.0 } else { 20.0 },
                caption,
            );
        }
        visible(ui, self.detail, waiting);
        visible(ui, self.hint, waiting && rt.browser.login_code().is_some());
        let detail = if waiting {
            let notice = rt.browser.login_notice();
            if notice.is_empty() {
                "브라우저에서 로그인 또는 회원가입을 완료해 주세요."
            } else {
                notice
            }
        } else {
            ""
        };
        txt(self.detail, (424.0, 608.0, 432.0, 54.0), 13.0, detail);
        let hint = rt
            .browser
            .login_code()
            .map(|code| format!("게임 연결 코드  {code}"))
            .unwrap_or_default();
        txt(self.hint, (420.0, 672.0, 440.0, 25.0), 12.0, &hint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn login_choice_is_native_and_passwords_are_masked_and_cleared_on_cancel() {
        let mut rt = Runtime::new(Session::Solo(Box::default()), None, false);
        rt.startup.choices();
        choose(&mut rt, 0);
        assert_eq!(rt.startup.stage, Stage::Login);
        assert!(!rt.browser.login_busy());
        assert!(rt.browser.login_code().is_none());
        insert_login(&mut rt.startup, 0, "player@example.test");
        insert_login(&mut rt.startup, 1, "비밀 password");
        rt.startup.fields[1].preedit = "입력".into();
        let rendered = login_text(&rt.startup.fields[1], true, true);
        assert!(
            !rendered.contains("비밀")
                && !rendered.contains("password")
                && !rendered.contains("입력")
        );
        assert!(!format!("{:?}", rt.startup).contains("password"));
        rt.startup.selected_all = true;
        insert_login(&mut rt.startup, 1, " replacement ");
        assert_eq!(rt.startup.fields[1].draft, " replacement ");
        choose(&mut rt, 1);
        assert!(rt
            .startup
            .fields
            .iter()
            .all(|f| f.draft.is_empty() && f.preedit.is_empty()));
        assert_eq!(rt.startup.stage, Stage::Choice);
    }
    #[test]
    fn credits_wait_for_assets_and_rendered_frames_then_require_a_choice() {
        let mut flow = Startup::new(true, false);
        for _ in 0..100 {
            flow.advance(0.1, false);
        }
        assert_eq!(flow.stage, Stage::Logo(0));
        flow.advance(60.0, true);
        assert_eq!(flow.stage, Stage::Logo(0));
        for _ in 0..26 {
            flow.advance(0.1, true);
        }
        assert_eq!(flow.stage, Stage::Logo(1));
        for _ in 0..27 {
            flow.advance(0.1, true);
        }
        assert_eq!(flow.stage, Stage::Choice);
        for _ in 0..100 {
            flow.advance(0.1, true);
        }
        assert_eq!(flow.stage, Stage::Choice);
        assert!(!Startup::new(false, false).active());
    }
    #[test]
    fn guest_does_not_contact_the_account_service_and_home_does_not_replay_credits() {
        let mut rt = Runtime::new(Session::Solo(Box::default()), None, false);
        rt.startup.choices();
        choose(&mut rt, 2);
        assert!(!rt.startup.active());
        assert!(rt.browser.linked.is_none());
        assert!(!rt.browser.login_busy());
        rt.startup.choices();
        assert_eq!(rt.startup.stage, Stage::Choice);
    }
    #[test]
    fn choices_remain_clickable_at_compact_and_ultrawide_sizes() {
        for size in [
            Vector2::new(960.0, 640.0),
            Vector2::new(1280.0, 800.0),
            Vector2::new(1920.0, 800.0),
        ] {
            for i in 0..3 {
                let (x, y, w, h) = selection::rect(size.x, size.y, bounds(i));
                assert_eq!(hit(size, Vector2::new(x + w / 2.0, y + h / 2.0)), Some(i));
            }
            assert_eq!(hit(size, Vector2::new(5.0, 5.0)), None);
        }
    }
}
