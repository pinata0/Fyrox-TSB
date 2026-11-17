//! Escape overlay. Capture and blur only our own rendered game, once per opening.
use super::*;
use fyrox::gui::image::{ImageBuilder, ImageMessage};
use trickcal_shooting_balddagu::settings::{
    settings_path, Settings, DEFAULT_SENSITIVITY, MAX_SENSITIVITY, MIN_SENSITIVITY,
};

type Rect = (f32, f32, f32, f32);
const CARD: Rect = (380.0, 175.0, 520.0, 450.0);
const SLIDER: Rect = (455.0, 365.0, 370.0, 40.0);
const MENU_BUTTONS: [Rect; 4] = [
    (420.0, 312.0, 440.0, 54.0),
    (420.0, 382.0, 440.0, 54.0),
    (420.0, 452.0, 440.0, 54.0),
    (420.0, 522.0, 440.0, 54.0),
];
const SETTINGS_BUTTONS: [Rect; 4] = [
    (420.0, 430.0, 80.0, 48.0),
    (510.0, 430.0, 80.0, 48.0),
    (610.0, 430.0, 250.0, 48.0),
    (420.0, 535.0, 440.0, 48.0),
];

#[derive(Debug)]
pub(super) struct Menu {
    pub open: bool,
    pub settings: Settings,
    editing: bool,
    dragging: bool,
    dirty: bool,
    focus: usize,
    path: Option<std::path::PathBuf>,
    status: Option<String>,
    background: Option<TextureResource>,
}

impl Default for Menu {
    fn default() -> Self {
        let path = if std::env::args().any(|a| a == "--settings-review") {
            // Review fixtures must never overwrite the player's preferences.
            Ok(std::path::PathBuf::from(
                "artifacts/settings-menu/review-settings.json",
            ))
        } else {
            settings_path()
        };
        let loaded = path
            .as_ref()
            .map_err(|e| e.to_string())
            .and_then(|p| Settings::load(p).map_err(|e| e.to_string()));
        let status = loaded.as_ref().err().cloned();
        if let Some(error) = &status {
            eprintln!("SETTINGS_LOAD_ERROR: {error}");
        }
        Self {
            open: false,
            settings: loaded.unwrap_or_default(),
            editing: false,
            dragging: false,
            dirty: false,
            focus: 0,
            path: path.ok(),
            status,
            background: None,
        }
    }
}

impl Menu {
    fn change(&mut self, value: f32) {
        let previous = self.settings;
        self.settings.set_sensitivity(value);
        self.dirty |= self.settings != previous || self.status.is_some();
    }

    fn save(&mut self) {
        if !self.dirty {
            return;
        }
        let result = self
            .path
            .as_ref()
            .ok_or_else(|| std::io::Error::other("Settings directory unavailable"))
            .and_then(|path| self.settings.save(path));
        match result {
            Ok(()) => {
                self.dirty = false;
                self.status = None;
            }
            Err(error) => {
                eprintln!("SETTINGS_SAVE_ERROR: {error}");
                self.status = Some(error.to_string());
            }
        }
    }

    fn drag(&mut self, size: Vector2<f32>, cursor: Vector2<f32>) {
        let (x, _, width, _) = selection::rect(size.x, size.y, SLIDER);
        let fraction = ((cursor.x - x) / width).clamp(0.0, 1.0);
        self.change(MIN_SENSITIVITY + fraction * (MAX_SENSITIVITY - MIN_SENSITIVITY));
    }
}

fn buttons(rt: &Runtime) -> &'static [Rect] {
    if rt.settings_menu.editing {
        &SETTINGS_BUTTONS
    } else if rt.lobby.room_active || !rt.home_open {
        // Command-line solo play also has a room, even before visiting its lobby.
        &MENU_BUTTONS
    } else {
        &MENU_BUTTONS[..2]
    }
}

pub(super) fn open(rt: &mut Runtime, context: &mut PluginContext) {
    // Capture before making the overlay visible so it never feeds back into itself.
    rt.settings_menu.background = match capture_frame(rt.scene, rt.home_open, context) {
        Ok((width, height, pixels)) => {
            let source =
                image::RgbaImage::from_raw(width, height, pixels).expect("validated framebuffer");
            let small = image::imageops::resize(
                &source,
                (width / 4).max(1),
                (height / 4).max(1),
                image::imageops::FilterType::Triangle,
            );
            let blurred = image::imageops::blur(&small, 3.0);
            Texture::from_bytes(
                TextureKind::Rectangle {
                    width: blurred.width(),
                    height: blurred.height(),
                },
                TexturePixelKind::RGBA8,
                blurred.into_raw(),
            )
            .map(TextureResource::new_embedded)
        }
        Err(error) => {
            fyrox::core::log::Log::warn(format!("Menu background capture failed: {error}"));
            None
        }
    };
    context.user_interfaces.first().send(
        rt.hud.settings.background,
        ImageMessage::Texture(rt.settings_menu.background.clone()),
    );
    rt.settings_menu.open = true;
    rt.settings_menu.editing = false;
    rt.settings_menu.focus = 0;
    rt.settings_menu.dragging = false;
    rt.grabbed = false;
    rt.keys = Keys::default();
    set_cursor(&context.graphics_context.as_initialized_ref().window, false);
}

fn close(rt: &mut Runtime, context: &PluginContext) {
    rt.settings_menu.save();
    rt.settings_menu.open = false;
    rt.settings_menu.dragging = false;
    rt.settings_menu.background = None;
    context
        .user_interfaces
        .first()
        .send(rt.hud.settings.background, ImageMessage::Texture(None));
    rt.keys = Keys::default();
    rt.grabbed = !rt.home_open && !rt.lobby.open && !rt.selecting && rt.error.is_none();
    set_cursor(
        &context.graphics_context.as_initialized_ref().window,
        rt.grabbed,
    );
}

fn activate(rt: &mut Runtime, action: usize, context: &PluginContext) {
    if action >= buttons(rt).len() {
        return;
    }
    let menu = &mut rt.settings_menu;
    if menu.editing {
        match action {
            0 => menu.change(menu.settings.mouse_sensitivity - 0.05),
            1 => menu.change(menu.settings.mouse_sensitivity + 0.05),
            2 => menu.change(DEFAULT_SENSITIVITY),
            _ => {
                menu.editing = false;
                menu.focus = 0;
            }
        }
        menu.save();
    } else if action == 0 {
        menu.editing = true;
        menu.focus = 0;
    } else {
        match action {
            2 => rt.return_to_game_lobby(),
            3 => rt.leave_room_to_home(),
            _ => {}
        }
        close(rt, context);
    }
}

/// Modal input ownership prevents clicks/keys from leaking into weapons or chat.
pub(super) fn on_event(rt: &mut Runtime, event: &Event<()>, context: &mut PluginContext) -> bool {
    if !rt.settings_menu.open {
        let escape = matches!(event, Event::WindowEvent { event: WindowEvent::KeyboardInput { event, .. }, .. }
            if event.physical_key == PhysicalKey::Code(KeyCode::Escape) && event.state == ElementState::Pressed && !event.repeat);
        // Existing chat/communication/selection/lobby cancellation keeps priority.
        if escape && !rt.chat.open && !rt.wheel.open && !rt.selecting && !rt.lobby.open {
            open(rt, context);
            return true;
        }
        return false;
    }
    let size = context.user_interfaces.first().screen_size();
    if let Event::WindowEvent { event, .. } = event {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                rt.cursor = Vector2::new(position.x as f32, position.y as f32);
                if rt.settings_menu.dragging {
                    rt.settings_menu.drag(size, rt.cursor);
                } else if let Some(index) = buttons(rt).iter().position(|&r| {
                    selection::contains(rt.cursor, selection::rect(size.x, size.y, r))
                }) {
                    rt.settings_menu.focus = index;
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                if *state == ElementState::Released {
                    rt.settings_menu.dragging = false;
                    rt.settings_menu.save();
                } else if rt.settings_menu.editing
                    && selection::contains(rt.cursor, selection::rect(size.x, size.y, SLIDER))
                {
                    rt.settings_menu.dragging = true;
                    rt.settings_menu.drag(size, rt.cursor);
                } else if let Some(index) = buttons(rt).iter().position(|&r| {
                    selection::contains(rt.cursor, selection::rect(size.x, size.y, r))
                }) {
                    activate(rt, index, context);
                }
            }
            WindowEvent::Focused(false) => {
                rt.settings_menu.dragging = false;
                rt.settings_menu.save();
                rt.keys = Keys::default();
                set_cursor(&context.graphics_context.as_initialized_ref().window, false);
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) if !event.repeat => {
                        if rt.settings_menu.editing {
                            rt.settings_menu.save();
                            rt.settings_menu.editing = false;
                            rt.settings_menu.dragging = false;
                            rt.settings_menu.focus = 0;
                        } else {
                            close(rt, context);
                        }
                    }
                    PhysicalKey::Code(KeyCode::F10) => {
                        rt.settings_menu.save();
                        context.loop_controller.exit();
                    }
                    PhysicalKey::Code(KeyCode::Tab | KeyCode::ArrowDown | KeyCode::ArrowUp) => {
                        let count = buttons(rt).len();
                        let menu = &mut rt.settings_menu;
                        let backwards = event.physical_key == PhysicalKey::Code(KeyCode::ArrowUp)
                            || (event.physical_key == PhysicalKey::Code(KeyCode::Tab)
                                && rt.modifiers.shift_key());
                        menu.focus = (menu.focus + if backwards { count - 1 } else { 1 }) % count;
                    }
                    PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::ArrowRight)
                        if rt.settings_menu.editing =>
                    {
                        let step = if event.physical_key == PhysicalKey::Code(KeyCode::ArrowLeft) {
                            -0.05
                        } else {
                            0.05
                        };
                        rt.settings_menu
                            .change(rt.settings_menu.settings.mouse_sensitivity + step);
                        rt.settings_menu.save();
                    }
                    PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) if !event.repeat => {
                        activate(rt, rt.settings_menu.focus, context)
                    }
                    _ => {}
                }
            }
            WindowEvent::CloseRequested => rt.settings_menu.save(),
            _ => {}
        }
    }
    true
}

#[derive(Debug, Default)]
pub(super) struct SettingsUi {
    background: Handle<UiNode>,
    shade: Handle<UiNode>,
    card: Handle<UiNode>,
    title: Handle<UiNode>,
    subtitle: Handle<UiNode>,
    value: Handle<UiNode>,
    track: Handle<UiNode>,
    fill: Handle<UiNode>,
    thumb: Handle<UiNode>,
    buttons: [Handle<UiNode>; 4],
    button_labels: [Handle<UiNode>; 4],
    status: Handle<UiNode>,
    hint: Handle<UiNode>,
}

impl SettingsUi {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        let result = Self {
            background: ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
                .with_sync_with_texture_size(false)
                .with_flip(true)
                .build(ctx)
                .to_base(),
            shade: panel(ctx, Color::from_rgba(10, 19, 30, 145)),
            card: panel(ctx, Color::from_rgba(24, 38, 54, 246)),
            track: panel(ctx, rgb([65, 83, 98])),
            fill: panel(ctx, rgb(MINT)),
            thumb: panel(ctx, rgb(CREAM)),
            buttons: std::array::from_fn(|_| panel(ctx, rgb([46, 65, 82]))),
            title: label(ctx, "", 34.0, CREAM),
            subtitle: label(ctx, "", 17.0, MINT),
            value: label(ctx, "", 27.0, CREAM),
            button_labels: std::array::from_fn(|_| label(ctx, "", 21.0, CREAM)),
            status: label(ctx, "", 15.0, MINT),
            hint: label(ctx, "", 14.0, CREAM),
        };
        for handle in result.nodes() {
            ctx[handle].set_visibility(false);
        }
        result
    }

    pub fn labels(&self) -> Vec<Handle<UiNode>> {
        [
            self.title,
            self.subtitle,
            self.value,
            self.status,
            self.hint,
        ]
        .into_iter()
        .chain(self.button_labels)
        .collect()
    }

    fn nodes(&self) -> Vec<Handle<UiNode>> {
        self.labels()
            .into_iter()
            .chain([
                self.background,
                self.shade,
                self.card,
                self.track,
                self.fill,
                self.thumb,
            ])
            .chain(self.buttons)
            .collect()
    }

    pub fn render(&self, rt: &Runtime, ui: &UserInterface) {
        let menu = &rt.settings_menu;
        for node in self.nodes() {
            visible(ui, node, menu.open);
        }
        if !menu.open {
            return;
        }
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0);
        let put = |h, r| {
            let (x, y, w, hgt) = selection::rect(size.x, size.y, r);
            place(ui, h, x, y, w, hgt);
        };
        let kr = rt.korean_font;
        let txt = |handle, bounds, points: f32, value: &str| {
            put(handle, bounds);
            text_size(ui, handle, points * scale);
            text(ui, handle, value);
        };
        place(ui, self.background, 0.0, 0.0, size.x, size.y);
        visible(ui, self.background, menu.background.is_some());
        place(ui, self.shade, -12.0, -12.0, size.x + 24.0, size.y + 24.0);
        put(self.card, CARD);
        txt(
            self.title,
            (420.0, 208.0, 440.0, 50.0),
            34.0,
            if menu.editing {
                if kr {
                    "설정"
                } else {
                    "SETTINGS"
                }
            } else if kr {
                "게임 메뉴"
            } else {
                "GAME MENU"
            },
        );
        txt(
            self.subtitle,
            (420.0, 265.0, 440.0, 30.0),
            17.0,
            if menu.editing {
                if kr {
                    "마우스 감도"
                } else {
                    "MOUSE SENSITIVITY"
                }
            } else if kr {
                "잠시 조작을 멈췄어요"
            } else {
                "CONTROLS PAUSED"
            },
        );
        for node in [self.value, self.track, self.fill, self.thumb, self.status] {
            visible(ui, node, menu.editing);
        }
        if menu.editing {
            txt(
                self.value,
                (420.0, 310.0, 440.0, 45.0),
                27.0,
                &format!("{:.2}x", menu.settings.mouse_sensitivity),
            );
            let fraction = (menu.settings.mouse_sensitivity - MIN_SENSITIVITY)
                / (MAX_SENSITIVITY - MIN_SENSITIVITY);
            put(self.track, (SLIDER.0, 381.0, SLIDER.2, 8.0));
            put(
                self.fill,
                (SLIDER.0, 381.0, (SLIDER.2 * fraction).max(1.0), 8.0),
            );
            put(
                self.thumb,
                (SLIDER.0 + SLIDER.2 * fraction - 7.0, 371.0, 14.0, 28.0),
            );
            txt(
                self.status,
                (420.0, 490.0, 440.0, 30.0),
                15.0,
                if menu.status.is_some() {
                    if kr {
                        "저장 실패 · 감도를 다시 조절하면 재시도합니다"
                    } else {
                        "Save failed. Adjust again to retry."
                    }
                } else if menu.dirty {
                    if kr {
                        "조절 중 · 놓으면 자동 저장"
                    } else {
                        "Release to save"
                    }
                } else if kr {
                    "자동 저장 · 다음 실행에도 유지됩니다"
                } else {
                    "Saved locally · kept after restart"
                },
            );
            ui.send(
                self.status,
                WidgetMessage::Foreground(
                    Brush::Solid(rgb(if menu.status.is_some() { PINK } else { MINT })).into(),
                ),
            );
        }
        let labels = if menu.editing {
            if kr {
                ["-", "+", "기본값으로", "뒤로"]
            } else {
                ["-", "+", "RESET TO DEFAULT", "BACK"]
            }
        } else if kr {
            [
                "설정",
                "게임으로 돌아가기",
                "게임 로비로 돌아가기",
                "방 나가기",
            ]
        } else {
            [
                "SETTINGS",
                "RESUME GAME",
                "RETURN TO GAME LOBBY",
                "LEAVE ROOM",
            ]
        };
        for (i, label) in labels.iter().enumerate() {
            let Some(&bounds) = buttons(rt).get(i) else {
                visible(ui, self.buttons[i], false);
                visible(ui, self.button_labels[i], false);
                continue;
            };
            put(self.buttons[i], bounds);
            ui.send(
                self.buttons[i],
                WidgetMessage::Background(
                    Brush::Solid(rgb(if menu.focus == i {
                        [65, 105, 112]
                    } else {
                        [46, 65, 82]
                    }))
                    .into(),
                ),
            );
            txt(
                self.button_labels[i],
                (
                    bounds.0 + 16.0,
                    bounds.1 + 10.0,
                    bounds.2 - 24.0,
                    bounds.3 - 10.0,
                ),
                if menu.editing { 18.0 } else { 21.0 },
                label,
            );
        }
        visible(ui, self.hint, menu.editing);
        if menu.editing {
            txt(
                self.hint,
                (400.0, 646.0, 540.0, 55.0),
                14.0,
                if kr {
                    "드래그 / ← → 감도 조절 · Esc 뒤로"
                } else {
                    "Drag / Left-Right: sensitivity · Esc: back"
                },
            );
        }
    }
}

// The review uses the same menu actions as the event handler and captures only the
// application's GPU output. Preferences are isolated in artifacts/settings-menu.
pub(super) fn review_update(rt: &mut Runtime, context: &mut PluginContext) {
    if rt.review_frames != 0 {
        return;
    }
    if rt.review_capture == 0 && std::env::args().any(|a| a == "--settings-review-reload") {
        assert!((rt.settings_menu.settings.mouse_sensitivity - 2.35).abs() < 0.001);
    }
    match rt.review_capture {
        0 => {
            rt.grabbed = true;
            rt.yaw = 0.35;
            rt.pitch = 0.0;
        }
        1 => {
            rt.keys.w = true;
            rt.keys.fire = true;
            open(rt, context);
            assert!(
                rt.settings_menu.background.is_some(),
                "Menu must have a blurred background"
            );
            assert!(!rt.controls_active() && !rt.keys.w && !rt.keys.fire && !rt.keys.zoom);
        }
        2 => activate(rt, 0, context),
        3 => {
            let size = context.user_interfaces.first().screen_size();
            let (x, y, width, _) = selection::rect(size.x, size.y, SLIDER);
            let fraction = (2.35 - MIN_SENSITIVITY) / (MAX_SENSITIVITY - MIN_SENSITIVITY);
            rt.settings_menu
                .drag(size, Vector2::new(x + width * fraction, y));
            rt.settings_menu.save();
            let loaded = Settings::load(rt.settings_menu.path.as_ref().unwrap()).unwrap();
            assert!((loaded.mouse_sensitivity - 2.35).abs() < 0.001);
            assert!(rt.settings_menu.status.is_none() && !rt.settings_menu.dirty);
        }
        4 => {
            activate(rt, 2, context);
            assert_eq!(
                rt.settings_menu.settings.mouse_sensitivity,
                DEFAULT_SENSITIVITY
            );
        }
        5 => {
            rt.settings_menu.change(2.35);
            activate(rt, 3, context);
            assert!(rt.settings_menu.open && !rt.settings_menu.editing);
        }
        6 => {
            activate(rt, 1, context);
            assert!(!rt.settings_menu.open && rt.controls_active());
            assert!(!rt.keys.w && !rt.keys.fire);
            assert!((rt.look_sensitivity() - 0.0022 * 2.35).abs() < 0.000001);
        }
        7 => open(rt, context),
        8 => {
            let phase = rt.session.world().phase;
            let map = rt.session.world().map_id;
            activate(rt, 2, context);
            assert!(rt.lobby.open && rt.lobby.room_active);
            assert!(!rt.home_open && !rt.browser.open && !rt.settings_menu.open);
            assert!(!rt.grabbed && !rt.controls_active());
            assert_eq!(rt.session.world().phase, phase);
            assert_eq!(rt.session.world().map_id, map);
        }
        9 => {
            rt.lobby.open = false;
            rt.close_selection(&context.graphics_context.as_initialized_ref().window);
            assert!(rt.controls_active());
            open(rt, context);
        }
        10 => {
            activate(rt, 3, context);
            assert!(rt.home_open && !rt.lobby.room_active && !rt.lobby.open);
            assert!(!rt.browser.open && !rt.settings_menu.open && !rt.grabbed);
            assert!(matches!(rt.session, Session::Solo(_)));
            assert_eq!(rt.session.world().phase, Phase::Lobby);
            assert!(!rt.sync_match_end() && !rt.sync_match_start());
        }
        11 => {
            open(rt, context);
            assert_eq!(buttons(rt).len(), 2);
            activate(rt, 2, context);
            activate(rt, 3, context);
            assert!(rt.home_open && !rt.lobby.room_active && rt.settings_menu.open);
        }
        _ => {
            println!("SETTINGS_REVIEW_OK: blur, menu, slider, save, reset, back, resume, return to lobby, leave room, home actions; sensitivity=2.35");
            context.loop_controller.exit();
        }
    }
    rt.review_frames = 1;
}

pub(super) fn review_capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if rt.review_frames == 0 {
        return Ok(());
    }
    rt.review_frames += 1;
    if rt.review_frames < 20 {
        return Ok(());
    }
    let tag = format!("settings-{:02}-", rt.review_capture);
    capture_preview_impl(
        rt.scene,
        1,
        &tag,
        rt.settings_menu.open || rt.home_open || rt.lobby.open,
        context,
    )
    .map_err(|error| fyrox::plugin::error::GameError::str(error.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
