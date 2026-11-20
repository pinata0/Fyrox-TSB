//! F1 character guide, composited over the live scene without changing the session.
mod content;
pub(super) mod review;

use super::*;
use fyrox::{
    asset::manager::ResourceManager,
    core::algebra::Matrix3,
    gui::{
        canvas::CanvasBuilder,
        formatted_text::WrapMode,
        image::{ImageBuilder, ImageMessage},
        vector_image::{Primitive, VectorImage, VectorImageBuilder},
    },
};
use std::cell::Cell;
use trickcal_shooting_balddagu::roster::Race;

const WHITE: [u8; 3] = [245, 248, 252];
const MUTED: [u8; 3] = [181, 194, 207];
const GOLD: [u8; 3] = [255, 204, 114];

#[derive(Debug, Default)]
pub(super) struct Help {
    pub open: bool,
    restore_grab: bool,
}

fn available(rt: &Runtime) -> bool {
    !rt.home_open
        && !rt.lobby.open
        && !rt.selecting
        && !rt.settings_menu.open
        && !rt.chat.open
        && !rt.wheel.open
        && !rt.startup.active()
        && rt.error.is_none()
        && matches!(rt.session.world().phase, Phase::Setup | Phase::Playing)
        && rt.session.world().actors[rt.session.slot()].active
}

fn open(rt: &mut Runtime, window: &Window) {
    rt.character_help.restore_grab = rt.grabbed;
    rt.character_help.open = true;
    rt.keys = Keys::default();
    rt.grabbed = false;
    set_cursor(window, false);
}

fn close(rt: &mut Runtime, window: &Window, restore: bool) {
    rt.character_help.open = false;
    rt.keys = Keys::default();
    rt.grabbed = restore
        && rt.character_help.restore_grab
        && !rt.home_open
        && !rt.lobby.open
        && !rt.selecting
        && !rt.settings_menu.open
        && !rt.chat.open
        && !rt.startup.active()
        && rt.error.is_none();
    rt.character_help.restore_grab = false;
    set_cursor(window, rt.grabbed);
}

/// Run before Escape/settings dispatch, so closing help cannot open another menu.
pub(super) fn on_event(rt: &mut Runtime, event: &Event<()>, context: &PluginContext) -> bool {
    // Windows can deliver keyboard/focus events before the renderer is resumed.
    if matches!(
        context.graphics_context,
        fyrox::engine::GraphicsContext::Uninitialized(_)
    ) {
        return false;
    }
    if let Event::WindowEvent { event, .. } = event {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                return on_key(rt, event.physical_key, event.state, event.repeat, context);
            }
            WindowEvent::Focused(false) if rt.character_help.open => {
                close(
                    rt,
                    &context.graphics_context.as_initialized_ref().window,
                    false,
                );
                // Continue through the normal focus-loss cleanup as well.
                return false;
            }
            _ => {}
        }
    }
    rt.character_help.open
}

fn on_key(
    rt: &mut Runtime,
    key: PhysicalKey,
    state: ElementState,
    repeat: bool,
    context: &PluginContext,
) -> bool {
    let pressed = state == ElementState::Pressed && !repeat;
    if key == PhysicalKey::Code(KeyCode::F1) && (rt.character_help.open || available(rt)) {
        if pressed {
            let window = &context.graphics_context.as_initialized_ref().window;
            if rt.character_help.open {
                close(rt, window, true);
            } else {
                open(rt, window);
            }
        }
        return true;
    }
    if rt.character_help.open && pressed {
        match key {
            PhysicalKey::Code(KeyCode::Escape) => close(
                rt,
                &context.graphics_context.as_initialized_ref().window,
                true,
            ),
            PhysicalKey::Code(KeyCode::F10) => context.loop_controller.exit(),
            _ => {}
        }
        return true;
    }
    rt.character_help.open
}

pub(super) fn sync(rt: &mut Runtime, window: &Window) {
    if rt.character_help.open && !available(rt) {
        close(rt, window, true);
    }
}

#[derive(Debug, Default)]
pub(super) struct HelpUi {
    shade: Handle<UiNode>,
    root: Handle<UiNode>,
    portrait: Handle<UiNode>,
    title: Handle<UiNode>,
    name: Handle<UiNode>,
    traits: Handle<UiNode>,
    intro: Handle<UiNode>,
    footer: Handle<UiNode>,
    categories: [Handle<UiNode>; 4],
    names: [Handle<UiNode>; 4],
    descriptions: [Handle<UiNode>; 4],
    keys: [Handle<UiNode>; 4],
    icons: [Handle<UiNode>; 4],
    textures: Vec<TextureResource>,
    last_character: Cell<Option<u8>>,
}

fn wrapped(ctx: &mut BuildContext, size: f32, color: [u8; 3]) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(color)).into()),
    )
    .with_font_size(size.into())
    .with_wrap(WrapMode::Word)
    .with_shadow(true)
    .build(ctx)
    .to_base()
}

fn ultimate_icon() -> Vec<Primitive> {
    vec![
        Primitive::WireCircle {
            center: Vector2::new(32.0, 32.0),
            radius: 28.0,
            thickness: 2.5,
            segments: 48,
        },
        Primitive::Triangle {
            points: [(36.0, 10.0), (17.0, 35.0), (32.0, 33.0)].map(|(x, y)| Vector2::new(x, y)),
        },
        Primitive::Triangle {
            points: [(32.0, 28.0), (47.0, 26.0), (28.0, 53.0)].map(|(x, y)| Vector2::new(x, y)),
        },
    ]
}

fn race_icon(race: Race) -> Vec<Primitive> {
    match race {
        Race::Fairy | Race::Witch => combat_icons::surge(),
        Race::Beast => combat_icons::leap(),
        Race::Spirit => combat_icons::burrow(),
        Race::Ghost => combat_icons::phase(),
        Race::Dragon => combat_icons::charge(),
        Race::Elf => [18.0, 46.0]
            .into_iter()
            .flat_map(|x| {
                (0..48).map(move |i| {
                    let point = |n: f32| {
                        let angle = n * std::f32::consts::TAU / 48.0;
                        Vector2::new(x + angle.cos() * 12.0, 32.0 + angle.sin() * 25.0)
                    };
                    Primitive::Line {
                        begin: point(i as f32),
                        end: point(i as f32 + 1.0),
                        thickness: 3.0,
                    }
                })
            })
            .collect(),
    }
}

impl HelpUi {
    pub fn new(ui: &mut UserInterface, manager: &ResourceManager) -> Self {
        let textures = portraits::load(manager);
        let ctx = &mut ui.build_ctx();
        // Square corners cover the entire viewport, including ultrawide letterboxing.
        let shade = BorderBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_background(Brush::Solid(Color::from_rgba(0, 0, 0, 190)).into()),
        )
        .build(ctx)
        .to_base();
        let cards: [_; 4] = std::array::from_fn(|_| panel(ctx, Color::from_rgba(24, 30, 38, 200)));
        let portrait = ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
            .with_texture(textures[0].clone())
            .with_sync_with_texture_size(false)
            .build(ctx)
            .to_base();
        let title = label(ctx, "캐릭터 가이드", 17.0, MINT);
        let name = label(ctx, "", 36.0, WHITE);
        let traits = label(ctx, "", 16.0, GOLD);
        let intro = wrapped(ctx, 20.0, WHITE);
        let footer = label(
            ctx,
            "F1 / Esc  닫기     ·     안내를 보는 동안에도 경기는 계속됩니다.",
            16.0,
            MUTED,
        );
        let categories = std::array::from_fn(|_| label(ctx, "", 14.0, MUTED));
        let names = std::array::from_fn(|_| label(ctx, "", 21.0, WHITE));
        let descriptions = std::array::from_fn(|_| wrapped(ctx, 17.0, WHITE));
        let keys = std::array::from_fn(|_| label(ctx, "", 15.0, GOLD));
        let icons = std::array::from_fn(|_| {
            VectorImageBuilder::new(
                WidgetBuilder::new()
                    .with_hit_test_visibility(false)
                    .with_foreground(Brush::Solid(rgb(WHITE)).into()),
            )
            .build(ctx)
            .to_base()
        });
        let children: Vec<_> = cards
            .into_iter()
            .chain([portrait, title, name, traits, intro, footer])
            .chain(categories)
            .chain(names)
            .chain(descriptions)
            .chain(keys)
            .chain(icons)
            .collect();
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(1120.0)
                .with_height(660.0)
                .with_hit_test_visibility(false)
                .with_visibility(false)
                .with_children(children),
        )
        .build(ctx)
        .to_base();
        for (node, x, y, w, h) in [
            (portrait, 0.0, 34.0, 104.0, 120.0),
            (title, 132.0, 0.0, 960.0, 26.0),
            (name, 132.0, 30.0, 960.0, 52.0),
            (traits, 132.0, 87.0, 960.0, 27.0),
            (intro, 132.0, 123.0, 960.0, 65.0),
            (footer, 0.0, 630.0, 1120.0, 30.0),
        ] {
            place(ctx.inner(), node, x, y, w, h);
        }
        for i in 0..4 {
            ctx.inner().send(
                keys[i],
                TextMessage::HorizontalAlignment(fyrox::gui::HorizontalAlignment::Center),
            );
            let x = (i % 2) as f32 * 574.0;
            let y = 204.0 + (i / 2) as f32 * 210.0;
            for (node, dx, dy, w, h) in [
                (cards[i], 0.0, 0.0, 546.0, 194.0),
                (
                    icons[i],
                    if i == 0 { 16.0 } else { 40.0 },
                    40.0,
                    if i == 0 { 112.0 } else { 64.0 },
                    64.0,
                ),
                (keys[i], 20.0, 124.0, 125.0, 30.0),
                (categories[i], 150.0, 15.0, 376.0, 23.0),
                (names[i], 150.0, 44.0, 376.0, 32.0),
                (descriptions[i], 150.0, 87.0, 376.0, 100.0),
            ] {
                place(ctx.inner(), node, x + dx, y + dy, w, h);
            }
        }
        Self {
            shade,
            root,
            portrait,
            title,
            name,
            traits,
            intro,
            footer,
            categories,
            names,
            descriptions,
            keys,
            icons,
            textures,
            last_character: Cell::new(None),
        }
    }

    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> {
        [self.title, self.name, self.traits, self.intro, self.footer]
            .into_iter()
            .chain(self.categories)
            .chain(self.names)
            .chain(self.descriptions)
            .chain(self.keys)
    }

    pub fn render(&self, rt: &Runtime, ui: &mut UserInterface) {
        let show = rt.character_help.open && available(rt);
        visible(ui, self.shade, show);
        visible(ui, self.root, show);
        if !show {
            return;
        }
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0);
        place(ui, self.shade, 0.0, 0.0, size.x, size.y);
        // Preserve design-space clipping when scaling below 100%.
        ui.send(
            self.root,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        ui.send(
            self.root,
            WidgetMessage::DesiredPosition(Vector2::new(
                (size.x - 1120.0 * scale) * 0.5,
                (size.y - 660.0 * scale) * 0.5,
            )),
        );
        let id = rt.session.world().actors[rt.session.slot()].character;
        let c = character(id);
        ui.send(
            self.portrait,
            ImageMessage::Texture(Some(self.textures[id as usize].clone())),
        );
        ui.send(
            self.portrait,
            ImageMessage::UvRect(portraits::uv(
                id as usize,
                &self.textures[id as usize],
                104.0,
                120.0,
            )),
        );
        if self.last_character.get() == Some(id) {
            return;
        }
        self.last_character.set(Some(id));
        let k = kit(id);
        let guide = &content::GUIDES[id as usize];
        let (race_name, race_key, race_help) = content::race_guide(c.race);
        text(ui, self.name, c.korean);
        text(
            ui,
            self.traits,
            format!(
                "{}  ·  {}  ·  {}",
                ["순수", "냉정", "광기", "활발", "우울"][c.personality as usize],
                ["요정", "수인", "엘프", "정령", "유령", "용족", "마녀"][c.race as usize],
                if c.healer {
                    "지원"
                } else {
                    ["전열", "중열", "후열"][c.position as usize]
                }
            ),
        );
        text(ui, self.intro, guide.intro);
        let cooldown = k.cooldown * if c.race == Race::Witch { 0.95 } else { 1.0 };
        let categories = [
            format!("기본 공격  ·  사거리 {:.1}m", k.range),
            format!("일반 스킬  ·  쿨타임 {cooldown:.2}초"),
            "궁극기  ·  게이지 100% 필요".into(),
            "종족 스킬 / 특성".into(),
        ];
        let descriptions = [guide.attack, guide.skill, guide.ultimate, &race_help];
        let names = [guide.weapon, k.skill_ko, k.ultimate_ko, race_name];
        let keys = ["좌클릭", "E", "Q", race_key];
        let primitives = [
            combat_icons::weapon(id),
            combat_icons::skill(id),
            ultimate_icon(),
            race_icon(c.race),
        ];
        for (i, primitives) in primitives.into_iter().enumerate() {
            text(ui, self.categories[i], &categories[i]);
            text(ui, self.names[i], names[i]);
            text(ui, self.descriptions[i], descriptions[i]);
            text(ui, self.keys[i], keys[i]);
            let icon = ui[self.icons[i]].cast_mut::<VectorImage>().unwrap();
            icon.primitives.set_value_and_mark_modified(primitives);
            icon.invalidate_visual();
        }
    }
}
