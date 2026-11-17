use super::*;
use trickcal_shooting_balddagu::chat::{ChatHistory, ChatInput, MAX_CHAT_CHARS};

const ROWS: usize = 8;

pub(super) struct Layout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    height: f32,
    pub scale: f32,
    rows: usize,
}
impl Layout {
    pub fn input_y(&self) -> f32 {
        self.y + self.height - 43.0 * self.scale
    }
}
pub(super) fn layout(size: Vector2<f32>) -> Layout {
    let width = (size.x * 0.36).clamp(320.0, 460.0);
    let height = 228.0;
    // Keep chat above the bottom controls in every menu and during gameplay.
    let bottom_margin = 96.0;
    Layout {
        x: 22.0,
        y: (size.y - height - bottom_margin).max(0.0),
        width,
        height,
        scale: 1.0,
        rows: ROWS,
    }
}

fn available(rt: &Runtime) -> bool {
    !rt.home_open
        && !rt.art_review
        && !rt.keys.score
        && (!rt.lobby.open || (!rt.browser.open && rt.lobby.chat_available()))
}

/// Chat owns input before the room, map picker, hero picker and gameplay.
pub(super) fn on_event(rt: &mut Runtime, event: &Event<()>, context: &PluginContext) -> bool {
    let Event::WindowEvent { event, .. } = event else {
        return rt.chat.open;
    };
    let window = &context.graphics_context.as_initialized_ref().window;
    if let WindowEvent::CursorMoved { position, .. } = event {
        rt.cursor = Vector2::new(position.x as f32, position.y as f32);
    }
    if !rt.chat.open {
        if !available(rt) || rt.wheel.open {
            return false;
        }
        let open = match event {
            WindowEvent::KeyboardInput { event, .. } => {
                event.state == ElementState::Pressed
                    && !event.repeat
                    && matches!(
                        event.physical_key,
                        PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter)
                    )
            }
            _ => false,
        };
        if open {
            rt.open_chat(window);
        }
        return open;
    }
    match event {
        WindowEvent::Focused(false) => rt.close_chat(window, false),
        WindowEvent::KeyboardInput { event, .. } => {
            if event.state != ElementState::Pressed {
                rt.chat_ime_commit = false;
                return true;
            }
            rt.chat.touch();
            rt.chat_error.clear();
            if let PhysicalKey::Code(code) = event.physical_key {
                match code {
                    KeyCode::Enter | KeyCode::NumpadEnter
                        if !event.repeat && rt.chat.preedit.is_empty() && !rt.chat_ime_commit =>
                    {
                        rt.send_chat(window);
                    }
                    KeyCode::Enter | KeyCode::NumpadEnter => {}
                    KeyCode::Escape if !event.repeat => {
                        rt.chat.clear();
                        rt.close_chat(window, true);
                    }
                    KeyCode::F10 => context.loop_controller.exit(),
                    _ if !rt.chat.preedit.is_empty() => {}
                    KeyCode::Tab if !event.repeat => rt.chat.channel.toggle(),
                    KeyCode::Backspace => rt.chat.backspace(),
                    KeyCode::Delete => rt.chat.delete(),
                    KeyCode::ArrowLeft => rt.chat.left(),
                    KeyCode::ArrowRight => rt.chat.right(),
                    KeyCode::Home => rt.chat.caret = 0,
                    KeyCode::End => rt.chat.caret = rt.chat.draft.len(),
                    KeyCode::PageUp => rt.chat_scroll = (rt.chat_scroll + 4).min(1024),
                    KeyCode::PageDown => rt.chat_scroll = rt.chat_scroll.saturating_sub(4),
                    _ if !rt.modifiers.control_key() && !rt.modifiers.super_key() => {
                        if let Some(value) = &event.text {
                            rt.chat.insert(value);
                        }
                    }
                    _ => {}
                }
            } else if rt.chat.preedit.is_empty() {
                if let Some(value) = &event.text {
                    rt.chat.insert(value);
                }
            }
        }
        WindowEvent::Ime(ime) => {
            match ime {
                Ime::Preedit(value, _) => rt.chat.preedit = value.clone(),
                Ime::Commit(value) => {
                    rt.chat.insert(value);
                    rt.chat.preedit.clear();
                    // Committing an IME syllable must not also submit the message.
                    rt.chat_ime_commit = true;
                }
                Ime::Disabled => rt.chat.preedit.clear(),
                _ => {}
            }
            rt.chat.touch();
        }
        _ => {}
    }
    true
}

#[derive(Debug, Default)]
pub(super) struct ChatUi {
    background: Handle<UiNode>,
    heading: Handle<UiNode>,
    rows: [Handle<UiNode>; ROWS],
    input_background: Handle<UiNode>,
    input: Handle<UiNode>,
    hint: Handle<UiNode>,
}
impl ChatUi {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        Self {
            background: panel(ctx, Color::from_rgba(16, 24, 36, 170)),
            heading: label(ctx, "", 13.0, CREAM),
            rows: std::array::from_fn(|_| label(ctx, "", 15.0, CREAM)),
            input_background: panel(ctx, Color::from_rgba(12, 19, 30, 235)),
            input: label(ctx, "", 15.0, CREAM),
            hint: label(ctx, "", 12.0, CREAM),
        }
    }
    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> + '_ {
        self.rows
            .iter()
            .copied()
            .chain([self.heading, self.input, self.hint])
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface, history: &ChatHistory) {
        let input = &rt.chat;
        let layout = layout(ui.screen_size());
        let Layout {
            x,
            y,
            width,
            height,
            scale,
            rows,
        } = layout;
        let menu = rt.selecting || rt.lobby.open;
        let opacity = input.opacity();
        let show = available(rt)
            && opacity > 0.0
            && (!rt.session.world().in_round_transition() || input.open);
        for node in self
            .labels()
            .chain([self.background, self.input_background])
        {
            visible(ui, node, show);
            ui.send(node, WidgetMessage::Opacity(Some(opacity)));
        }
        if !show {
            return;
        }
        visible(ui, self.input_background, input.open);
        visible(ui, self.input, input.open);
        let padding = 10.0 * scale;
        let content_width = width - padding * 2.0;
        let font_scale = 1.0;
        let wrap_width = content_width / (scale * font_scale);
        for node in self.rows.into_iter().chain([self.input]) {
            text_size(ui, node, 15.0 * scale * font_scale);
        }
        text_size(ui, self.heading, 12.0 * scale);
        text_size(ui, self.hint, 10.0 * scale);
        place(ui, self.background, x, y, width, height);
        place(
            ui,
            self.heading,
            x + padding,
            y + 5.0 * scale,
            content_width,
            19.0 * scale,
        );
        let destination = if rt.lobby.open && !rt.lobby.room_active {
            if rt.korean_font {
                "방 연결 대기"
            } else {
                "NO ROOM"
            }
        } else if matches!(rt.session, Session::Solo(_)) {
            if rt.korean_font {
                "연습"
            } else {
                "PRACTICE"
            }
        } else if menu {
            if rt.korean_font {
                "현재 방"
            } else {
                "CURRENT ROOM"
            }
        } else if rt.korean_font {
            "현재 경기"
        } else {
            "CURRENT MATCH"
        };
        text(
            ui,
            self.heading,
            format!(
                "{} · {destination}",
                if rt.korean_font { "채팅" } else { "CHAT" }
            ),
        );

        let mut lines = Vec::new();
        for message in &history.messages {
            let hero = character(message.character);
            let content = format!(
                "[{}] P{} {}: {}",
                message.channel.label(rt.korean_font),
                message.sender + 1,
                if rt.korean_font {
                    hero.korean
                } else {
                    hero.name
                },
                message.text
            );
            for line in wrap(&content, wrap_width) {
                lines.push((line, message.channel.color()));
            }
        }
        let end = lines
            .len()
            .saturating_sub(rt.chat_scroll.min(lines.len().saturating_sub(rows)));
        let start = end.saturating_sub(rows);
        for (i, &node) in self.rows.iter().enumerate() {
            visible(ui, node, i < rows);
            place(
                ui,
                node,
                x + padding,
                y + (25.0 + i as f32 * 18.0) * scale,
                content_width,
                20.0 * scale,
            );
            if let Some((line, color)) = lines.get(start + i).filter(|_| start + i < end) {
                text(ui, node, line.clone());
                ui.send(
                    node,
                    WidgetMessage::Foreground(Brush::Solid(rgb(*color)).into()),
                );
            } else {
                text(ui, node, "");
            }
        }
        place(
            ui,
            self.input_background,
            x + 5.0 * scale,
            layout.input_y(),
            width - 10.0 * scale,
            24.0 * scale,
        );
        place(
            ui,
            self.input,
            x + padding,
            layout.input_y() + 2.0 * scale,
            content_width,
            22.0 * scale,
        );
        let prefix = format!("[{}] ", input.channel.label(rt.korean_font));
        text(
            ui,
            self.input,
            if input.open {
                format!(
                    "{prefix}{}",
                    draft_view(input, wrap_width - measure(&prefix))
                )
            } else {
                format!(
                    "{prefix}{}",
                    if rt.korean_font {
                        "Enter 또는 클릭으로 입력"
                    } else {
                        "Enter / click to chat"
                    }
                )
            },
        );
        ui.send(
            self.input,
            WidgetMessage::Foreground(Brush::Solid(rgb(input.channel.color())).into()),
        );
        place(
            ui,
            self.hint,
            x + padding,
            y + height - 17.0 * scale,
            content_width,
            16.0 * scale,
        );
        let hint = if !rt.chat_error.is_empty() {
            rt.chat_error.clone()
        } else if input.open {
            if rt.korean_font {
                format!(
                    "Tab 팀/경기 · Esc 취소 · PgUp/Dn 기록 {}/{}",
                    input.draft.chars().count(),
                    MAX_CHAT_CHARS
                )
            } else {
                format!(
                    "Tab CHANNEL · Esc CANCEL · PgUp/Dn {}/{}",
                    input.draft.chars().count(),
                    MAX_CHAT_CHARS
                )
            }
        } else if rt.korean_font {
            "팀: 파랑 · 경기 전체: 주황 · 기록 유지".into()
        } else {
            "Enter CHAT · TEAM: BLUE / MATCH: ORANGE".into()
        };
        text(ui, self.hint, hint);
    }
}

// Conservative widths keep mixed Korean/Latin lines inside the panel with the system font.
fn glyph_width(c: char) -> f32 {
    if c.is_ascii() {
        if "MW@%".contains(c) {
            15.0
        } else {
            9.0
        }
    } else {
        16.0
    }
}
fn measure(text: &str) -> f32 {
    text.chars().map(glyph_width).sum()
}
fn wrap(text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut used = 0.0;
    for c in text.chars() {
        let advance = glyph_width(c);
        if used + advance > width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            used = 0.0;
        }
        line.push(c);
        used += advance;
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
fn draft_view(input: &ChatInput, width: f32) -> String {
    let composing = if input.preedit.is_empty() {
        "|".into()
    } else {
        format!("[{}]|", input.preedit)
    };
    let mut before = String::new();
    let mut used = measure(&composing);
    for c in input.draft[..input.caret].chars().rev() {
        if used + glyph_width(c) > width {
            break;
        }
        before.insert(0, c);
        used += glyph_width(c);
    }
    before.push_str(&composing);
    for c in input.draft[input.caret..].chars() {
        if used + glyph_width(c) > width {
            break;
        }
        before.push(c);
        used += glyph_width(c);
    }
    before
}
