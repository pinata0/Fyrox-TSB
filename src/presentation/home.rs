//! Illustrated home screen with native social and profile screens.
use super::*;
use fyrox::{
    asset::manager::ResourceManager,
    core::math::Rect,
    gui::{
        brush::GradientPoint,
        canvas::CanvasBuilder,
        image::{ImageBuilder, ImageMessage},
    },
};

const WHITE: [u8; 3] = [255, 250, 236];
const GOLD: [u8; 3] = [255, 214, 135];
const MUTED: [u8; 3] = [190, 200, 193];
const MENU_KO: [&str; 9] = [
    "플레이",
    "전투훈련",
    "사도갤러리",
    "전리품 상자",
    "하이라이트",
    "소셜",
    "프로필",
    "설정",
    "게임 종료",
];
const MENU_EN: [&str; 9] = [
    "PLAY",
    "TRAINING",
    "SAINT GALLERY",
    "LOOT BOXES",
    "HIGHLIGHTS",
    "SOCIAL",
    "PROFILE",
    "OPTIONS",
    "QUIT GAME",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Play,
    Training,
    Social,
    Profile,
    Quit,
}
impl Action {
    pub fn menu_index(self) -> usize {
        match self {
            Self::Play => 0,
            Self::Training => 1,
            Self::Social => 5,
            Self::Profile => 6,
            Self::Quit => 8,
        }
    }
    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Training,
            5 => Self::Social,
            6 => Self::Profile,
            8 => Self::Quit,
            _ => Self::Play,
        }
    }
}

fn scale(size: Vector2<f32>) -> f32 {
    (size.x / 1280.0).min(size.y / 720.0)
}
fn menu_y(index: usize) -> f32 {
    if index < 4 {
        202.0 + index as f32 * 47.0
    } else {
        408.0 + (index - 4) as f32 * 28.0
    }
}

/// Apply home layout even while Windows' interactive resize loop pauses fixed updates.
pub fn resize(rt: &Runtime, ui: &mut UserInterface, size: Vector2<f32>) -> bool {
    if !rt.home_open || rt.hud.home.root.is_none() || size.x <= 0.0 || size.y <= 0.0 {
        return false;
    }

    ui.set_screen_size(size);
    rt.hud.home.render(rt, ui);
    rt.hud.account.render(rt, ui);
    // Home input is handled by on_os_event, not UI messages. Flush the layout
    // messages here because the executor's usual UI dispatch is also paused.
    while ui.poll_message().is_some() {}
    // update_layout alone leaves drawing_context at the old size. A zero-time
    // UI update also rebuilds draw data without advancing gameplay or UI timers.
    ui.update(size, 0.0, &Default::default());
    true
}

pub fn hit_action(size: Vector2<f32>, cursor: Vector2<f32>) -> Option<Action> {
    let s = scale(size);
    let right = size.x / s - 42.0;
    if selection::contains(cursor, ((right - 288.0) * s, 30.0 * s, 48.0 * s, 48.0 * s)) {
        return Some(Action::Social);
    }
    if selection::contains(cursor, ((right - 232.0) * s, 30.0 * s, 232.0 * s, 48.0 * s)) {
        return Some(Action::Profile);
    }
    [
        (0, Action::Play),
        (1, Action::Training),
        (5, Action::Social),
        (6, Action::Profile),
        (8, Action::Quit),
    ]
    .into_iter()
    .find_map(|(i, a)| {
        selection::contains(
            cursor,
            (
                48.0 * s,
                menu_y(i) * s,
                290.0 * s,
                if i <= 1 { 47.0 * s } else { 28.0 * s },
            ),
        )
        .then_some(a)
    })
}

#[derive(Debug, Default)]
pub struct HomeUi {
    root: Handle<UiNode>,
    background: Handle<UiNode>,
    shade: Handle<UiNode>,
    logo: Handle<UiNode>,
    menu: [Handle<UiNode>; 9],
    highlight: Handle<UiNode>,
    marker: Handle<UiNode>,
    rule: Handle<UiNode>,
    friend_bg: Handle<UiNode>,
    friend_icon: Handle<UiNode>,
    profile_bg: Handle<UiNode>,
    avatar_bg: Handle<UiNode>,
    avatar: Handle<UiNode>,
    avatar_initial: Handle<UiNode>,
    player: Handle<UiNode>,
    player_detail: Handle<UiNode>,
    chat_bg: Handle<UiNode>,
    chat_rule: Handle<UiNode>,
    chat_title: Handle<UiNode>,
    chat_status: Handle<UiNode>,
    chat_empty: Handle<UiNode>,
    chat_input_bg: Handle<UiNode>,
    chat_input: Handle<UiNode>,
    footer: Handle<UiNode>,
    textures: Vec<TextureResource>,
}

fn image(ctx: &mut BuildContext, texture: TextureResource) -> Handle<UiNode> {
    ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
        .with_texture(texture)
        .with_sync_with_texture_size(false)
        .build(ctx)
        .to_base()
}
fn flat(ctx: &mut BuildContext, color: Color, radius: f32) -> Handle<UiNode> {
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

impl HomeUi {
    pub fn new(ui: &mut UserInterface, manager: &ResourceManager) -> Self {
        let textures = [
            "assets/ui/home/elias-picnic-soft.png",
            "assets/ui/home/trickcal-shooting-balddagu-logo.png",
            "assets/characters/erpin-portrait.png",
        ]
        .map(|p| manager.request::<Texture>(p))
        .to_vec();
        Self::with_textures(ui, textures)
    }
    fn with_textures(ui: &mut UserInterface, textures: Vec<TextureResource>) -> Self {
        let ctx = &mut ui.build_ctx();
        let background = image(ctx, textures[0].clone());
        let shade = BorderBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_background(
                    Brush::LinearGradient {
                        from: Vector2::new(0.0, 0.0),
                        to: Vector2::new(1.0, 0.0),
                        stops: vec![
                            GradientPoint {
                                stop: 0.0,
                                color: Color::from_rgba(24, 41, 36, 180),
                            },
                            GradientPoint {
                                stop: 0.23,
                                color: Color::from_rgba(24, 41, 36, 132),
                            },
                            GradientPoint {
                                stop: 0.46,
                                color: Color::from_rgba(24, 41, 36, 0),
                            },
                            GradientPoint {
                                stop: 0.70,
                                color: Color::TRANSPARENT,
                            },
                            GradientPoint {
                                stop: 1.0,
                                color: Color::TRANSPARENT,
                            },
                        ],
                    }
                    .into(),
                ),
        )
        .with_stroke_thickness(fyrox::gui::Thickness::uniform(0.0).into())
        .build(ctx)
        .to_base();
        let highlight = flat(ctx, Color::from_rgba(255, 230, 168, 22), 3.0);
        let marker = flat(ctx, rgb(GOLD), 0.0);
        let rule = flat(ctx, Color::from_rgba(237, 232, 202, 65), 0.0);
        let friend_bg = flat(ctx, Color::from_rgba(24, 47, 45, 215), 5.0);
        let profile_bg = flat(ctx, Color::from_rgba(24, 40, 38, 224), 5.0);
        let avatar_bg = flat(ctx, rgb([217, 222, 177]), 3.0);
        let chat_bg = flat(ctx, Color::from_rgba(16, 31, 30, 176), 5.0);
        let chat_rule = flat(ctx, Color::from_rgba(229, 234, 213, 40), 0.0);
        let chat_input_bg = flat(ctx, Color::from_rgba(9, 22, 22, 130), 3.0);
        let mut home = Self {
            background,
            shade,
            highlight,
            marker,
            rule,
            friend_bg,
            profile_bg,
            avatar_bg,
            chat_bg,
            chat_rule,
            chat_input_bg,
            logo: image(ctx, textures[1].clone()),
            avatar: image(ctx, textures[2].clone()),
            avatar_initial: label(ctx, "", 23.0, WHITE),
            friend_icon: label(ctx, "+", 32.0, WHITE),
            menu: std::array::from_fn(|_| label(ctx, "", 28.0, WHITE)),
            player: label(ctx, "PLAYER 1", 16.0, WHITE),
            player_detail: label(ctx, "", 10.0, MUTED),
            chat_title: label(ctx, "", 13.0, WHITE),
            chat_status: label(ctx, "", 10.0, MUTED),
            chat_empty: label(ctx, "", 12.0, MUTED),
            chat_input: label(ctx, "", 12.0, MUTED),
            footer: label(ctx, "TSB  /  FAN GAME  /  0.1", 10.0, WHITE),
            textures,
            ..Default::default()
        };
        let children = [
            background,
            shade,
            highlight,
            marker,
            rule,
            friend_bg,
            profile_bg,
            avatar_bg,
            chat_bg,
            chat_rule,
            chat_input_bg,
            home.logo,
            home.avatar,
        ]
        .into_iter()
        .chain(home.labels())
        .collect::<Vec<_>>();
        home.root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_children(children)
                .with_hit_test_visibility(false)
                .with_visibility(false),
        )
        .build(ctx)
        .to_base();
        home
    }
    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> + '_ {
        self.menu.into_iter().chain([
            self.friend_icon,
            self.avatar_initial,
            self.player,
            self.player_detail,
            self.chat_title,
            self.chat_status,
            self.chat_empty,
            self.chat_input,
            self.footer,
        ])
    }
    pub fn ready(&self) -> bool {
        self.textures.iter().all(|t| t.is_ok())
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface) {
        visible(ui, self.root, rt.home_open);
        if !rt.home_open {
            return;
        }
        let size = ui.screen_size();
        let s = scale(size);
        place(ui, self.root, 0.0, 0.0, size.x, size.y);
        place(ui, self.background, 0.0, 0.0, size.x, size.y);
        place(ui, self.shade, 0.0, 0.0, size.x, size.y);
        // Cover without distortion; keep the character group on the right.
        let art_aspect = 1672.0 / 941.0;
        let window_aspect = size.x / size.y;
        let uv = if window_aspect < art_aspect {
            let width = window_aspect / art_aspect;
            Rect::new(1.0 - width, 0.0, width, 1.0)
        } else {
            let height = art_aspect / window_aspect;
            Rect::new(0.0, (1.0 - height) * 0.5, 1.0, height)
        };
        ui.send(self.background, ImageMessage::UvRect(uv));
        let put = |h, x, y, w, v| place(ui, h, x * s, y * s, w * s, v * s);
        let txt = |h, x, y, w, v, font: f32, value: &str| {
            put(h, x, y, w, v);
            text_size(ui, h, font * s);
            text(ui, h, value);
        };
        let ko = rt.korean_font;
        put(self.logo, 45.0, 25.0, 280.0, 280.0 * 941.0 / 1672.0);
        let active = rt.home_focus;
        put(
            self.highlight,
            46.0,
            menu_y(active),
            283.0,
            if active <= 1 { 47.0 } else { 28.0 },
        );
        put(
            self.marker,
            46.0,
            menu_y(active) + 7.0,
            3.0,
            if active == 0 { 32.0 } else { 16.0 },
        );
        put(self.rule, 58.0, 396.0, 36.0, 1.0);
        for (i, &h) in self.menu.iter().enumerate() {
            txt(
                h,
                if i == active { 67.0 } else { 58.0 },
                menu_y(i),
                280.0,
                if i < 4 { 49.0 } else { 29.0 },
                if i < 4 { 35.0 } else { 20.0 },
                if ko { MENU_KO[i] } else { MENU_EN[i] },
            );
            ui.send(
                h,
                WidgetMessage::Foreground(
                    Brush::Solid(rgb(if i == active {
                        GOLD
                    } else if i == 8 {
                        WHITE
                    } else {
                        [226, 231, 217]
                    }))
                    .into(),
                ),
            );
        }
        let right = size.x / s - 42.0;
        put(self.friend_bg, right - 288.0, 30.0, 48.0, 48.0);
        txt(self.friend_icon, right - 280.0, 28.0, 40.0, 48.0, 32.0, "+");
        put(self.profile_bg, right - 232.0, 30.0, 232.0, 48.0);
        put(self.avatar_bg, right - 229.0, 33.0, 42.0, 42.0);
        put(self.avatar, right - 228.0, 34.0, 40.0, 40.0);
        let profile = rt.browser.linked.as_ref();
        let initials = profile.is_some() && rt.browser.avatar.is_none();
        visible(ui, self.avatar, !initials);
        visible(ui, self.avatar_initial, initials);
        ui.send(
            self.avatar,
            ImageMessage::Texture(Some(
                rt.browser
                    .avatar
                    .as_ref()
                    .unwrap_or(&self.textures[2])
                    .clone(),
            )),
        );
        let color = match profile.and_then(|p| p.avatar.as_deref()) {
            Some("mint") => [47, 125, 107],
            Some("blue") => [62, 102, 171],
            Some("violet") => [116, 82, 165],
            _ if profile.is_some() => [181, 93, 103],
            _ => [217, 222, 177],
        };
        ui.send(
            self.avatar_bg,
            WidgetMessage::Background(Brush::Solid(rgb(color)).into()),
        );
        txt(
            self.avatar_initial,
            right - 221.0,
            36.0,
            33.0,
            36.0,
            23.0,
            &profile
                .and_then(|p| p.display_name.chars().next())
                .unwrap_or(' ')
                .to_string(),
        );
        txt(
            self.player,
            right - 176.0,
            34.0,
            170.0,
            24.0,
            16.0,
            &profile.map_or_else(
                || rt.session.player_name(rt.session.slot()),
                |p| p.display_name.clone(),
            ),
        );
        txt(
            self.player_detail,
            right - 176.0,
            59.0,
            170.0,
            15.0,
            10.0,
            &rt.browser.profile_detail(ko),
        );
        let bottom = size.y / s;
        let cy = bottom - 160.0;
        put(self.chat_bg, 48.0, cy, 318.0, 126.0);
        put(self.chat_rule, 62.0, cy + 35.0, 290.0, 1.0);
        txt(
            self.chat_title,
            62.0,
            cy + 9.0,
            200.0,
            22.0,
            13.0,
            if ko { "친구 채팅" } else { "FRIEND CHAT" },
        );
        txt(
            self.chat_status,
            297.0,
            cy + 12.0,
            60.0,
            18.0,
            10.0,
            if ko { "준비 중" } else { "SOON" },
        );
        txt(
            self.chat_empty,
            62.0,
            cy + 48.0,
            290.0,
            23.0,
            12.0,
            if ko {
                "친구와 함께할 이야기를 기다리고 있어요."
            } else {
                "A place for conversations with friends."
            },
        );
        put(self.chat_input_bg, 58.0, cy + 84.0, 298.0, 31.0);
        txt(
            self.chat_input,
            70.0,
            cy + 90.0,
            280.0,
            23.0,
            12.0,
            if ko {
                "친구에게 메시지 보내기..."
            } else {
                "Message a friend..."
            },
        );
        txt(
            self.footer,
            right - 170.0,
            bottom - 29.0,
            185.0,
            18.0,
            10.0,
            "TSB  /  FAN GAME  /  0.1",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_resize_rebuilds_draw_data_without_a_game_update() {
        let texture = TextureResource::new_embedded(
            Texture::from_bytes(
                TextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                TexturePixelKind::RGBA8,
                vec![255; 4],
            )
            .unwrap(),
        );
        let mut ui = UserInterface::new(Vector2::new(1280.0, 800.0));
        let mut rt = Runtime::new(Session::Solo(Box::default()), None, false);
        rt.home_open = true;
        rt.hud.home = HomeUi::with_textures(&mut ui, vec![texture; 3]);

        // No engine/game update between resizes, as in Windows' modal size loop.
        for size in [
            Vector2::new(960.0, 640.0),
            Vector2::new(1920.0, 800.0),
            Vector2::new(1024.0, 900.0),
        ] {
            assert!(resize(&rt, &mut ui, size));
            assert_eq!(ui.screen_size(), size);
            assert_eq!(ui[rt.hud.home.background].actual_local_size(), size);
            // Checking layout alone misses stale drawing_context geometry.
            assert!(ui
                .drawing_context
                .get_vertices()
                .iter()
                .any(|vertex| (vertex.pos - size).norm() < 0.01));
            let menu_center = ui[rt.hud.home.menu[0]].screen_bounds().center();
            assert_eq!(hit_action(size, menu_center), Some(Action::Play));
        }

        let last_size = ui.screen_size();
        assert!(!resize(&rt, &mut ui, Vector2::zeros()));
        assert_eq!(ui.screen_size(), last_size);
        rt.home_open = false;
        assert!(!resize(&rt, &mut ui, Vector2::new(1280.0, 800.0)));
        assert_eq!(ui.screen_size(), last_size);
    }
    #[test]
    fn home_uses_linked_name_handle_and_photo_before_joining_and_after_leaving_a_room() {
        use fyrox::gui::image::Image;
        use trickcal_shooting_balddagu::room_directory::PlayerProfile;
        let texture = TextureResource::new_embedded(
            Texture::from_bytes(
                TextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                TexturePixelKind::RGBA8,
                vec![20, 180, 100, 255],
            )
            .unwrap(),
        );
        let mut ui = UserInterface::new(Vector2::new(1280.0, 800.0));
        let home = HomeUi::with_textures(&mut ui, vec![texture.clone(); 3]);
        let mut rt = Runtime::new(Session::Solo(Box::default()), None, false);
        rt.home_open = true;
        rt.browser.linked = Some(PlayerProfile {
            display_name: "로그인한 친구".into(),
            handle: Some("dotg_friend".into()),
            avatar: Some("mint".into()),
            avatar_url: Some("https://example.com/photo.webp".into()),
        });
        rt.browser.avatar = Some(texture.clone());
        for size in [Vector2::new(1280.0, 800.0), Vector2::new(960.0, 640.0)] {
            ui.set_screen_size(size);
            home.render(&rt, &ui);
            while ui.poll_message().is_some() {}
            assert_eq!(
                ui[home.player].cast::<Text>().unwrap().text(),
                "로그인한 친구"
            );
            assert_eq!(
                ui[home.player_detail].cast::<Text>().unwrap().text(),
                "@dotg_friend"
            );
            assert_eq!(
                ui[home.avatar].cast::<Image>().unwrap().texture.as_ref(),
                Some(&texture)
            );
            assert!(*ui[home.avatar].visibility);
            assert!(!*ui[home.avatar_initial].visibility);
            rt.replace_room(Session::Solo(Box::default()), false);
            rt.home_open = true;
        }
        rt.browser.avatar = None;
        home.render(&rt, &ui);
        while ui.poll_message().is_some() {}
        assert!(!*ui[home.avatar].visibility);
        assert!(*ui[home.avatar_initial].visibility);
        assert_eq!(ui[home.avatar_initial].cast::<Text>().unwrap().text(), "로");
        rt.browser.linked = None;
        home.render(&rt, &ui);
        while ui.poll_message().is_some() {}
        assert_eq!(ui[home.player].cast::<Text>().unwrap().text(), "PLAYER 1");
        assert!(*ui[home.avatar].visibility);
        assert!(!*ui[home.avatar_initial].visibility);
    }
    #[test]
    fn play_social_profile_and_quit_are_actionable_at_supported_sizes() {
        for size in [
            Vector2::new(1280.0, 800.0),
            Vector2::new(960.0, 640.0),
            Vector2::new(1920.0, 1080.0),
            Vector2::new(1920.0, 800.0),
        ] {
            let s = scale(size);
            for i in 0..9 {
                let action = hit_action(size, Vector2::new(100.0 * s, (menu_y(i) + 12.0) * s));
                assert_eq!(
                    action,
                    match i {
                        0 => Some(Action::Play),
                        1 => Some(Action::Training),
                        5 => Some(Action::Social),
                        6 => Some(Action::Profile),
                        8 => Some(Action::Quit),
                        _ => None,
                    }
                );
            }
            assert_eq!(
                hit_action(size, Vector2::new(size.x - 100.0 * s, 50.0 * s)),
                Some(Action::Profile)
            );
            assert_eq!(hit_action(size, Vector2::new(100.0, size.y - 80.0)), None);
        }
    }
}
