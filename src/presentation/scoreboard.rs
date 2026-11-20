//! Hold-Tab match overlay, with a team-relative roster and authoritative totals.
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
        HorizontalAlignment,
    },
};
use std::cell::Cell;

const WIDTH: f32 = 1100.0;
const HEIGHT: f32 = 666.0;
const WHITE: [u8; 3] = [246, 250, 255];
const MUTED: [u8; 3] = [173, 188, 206];
const CYAN: [u8; 3] = [66, 222, 244];
const RED: [u8; 3] = [255, 104, 131];
const GOLD: [u8; 3] = [255, 206, 111];
const COLS: [(f32, f32); 6] = [
    (328.0, 44.0),
    (372.0, 44.0),
    (416.0, 44.0),
    (466.0, 90.0),
    (562.0, 90.0),
    (658.0, 90.0),
];

#[derive(Debug)]
struct Row {
    root: Handle<UiNode>,
    background: Handle<UiNode>,
    accent: Handle<UiNode>,
    portrait: Handle<UiNode>,
    roles: [Handle<UiNode>; 3],
    name: Handle<UiNode>,
    detail: Handle<UiNode>,
    track: Handle<UiNode>,
    ring: Handle<UiNode>,
    charge: Handle<UiNode>,
    values: [Handle<UiNode>; 6],
    last_charge: Cell<u8>,
}

#[derive(Debug, Default)]
pub(super) struct ScoreboardUi {
    root: Handle<UiNode>,
    dim: Handle<UiNode>,
    card: Handle<UiNode>,
    heading: Handle<UiNode>,
    time: Handle<UiNode>,
    teams: [Handle<UiNode>; 2],
    headers: [[Handle<UiNode>; 6]; 2],
    rows: Vec<Row>,
    versus: Handle<UiNode>,
    portrait: Handle<UiNode>,
    hero: Handle<UiNode>,
    role: Handle<UiNode>,
    personal: [Handle<UiNode>; 5],
    captions: [Handle<UiNode>; 5],
    state: Handle<UiNode>,
    hint: Handle<UiNode>,
    textures: Vec<TextureResource>,
}

fn available(rt: &Runtime) -> bool {
    rt.session.world().phase != Phase::Lobby
        && !rt.home_open
        && !rt.lobby.open
        && !rt.startup.active()
        && !rt.selecting
        && !rt.chat.open
        && !rt.wheel.open
        && !rt.settings_menu.open
        && !rt.character_help.open
}

pub(super) fn showing(rt: &Runtime) -> bool {
    rt.keys.score && available(rt)
}

pub(super) fn on_tab(rt: &mut Runtime, down: bool) {
    rt.keys.score = down && available(rt);
}

fn block(ctx: &mut BuildContext, color: Color) -> Handle<UiNode> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_background(Brush::Solid(color).into()),
    )
    .with_corner_radius(2.0.into())
    .with_stroke_thickness(fyrox::gui::Thickness::uniform(0.0).into())
    .build(ctx)
    .to_base()
}

fn caption(ctx: &mut BuildContext, size: f32, color: [u8; 3], centered: bool) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(color)).into()),
    )
    .with_font_size(size.into())
    .with_wrap(WrapMode::NoWrap)
    .with_trim_text(true)
    .with_horizontal_text_alignment(if centered {
        HorizontalAlignment::Center
    } else {
        HorizontalAlignment::Left
    })
    .build(ctx)
    .to_base()
}

fn picture(ctx: &mut BuildContext, texture: &TextureResource) -> Handle<UiNode> {
    ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
        .with_texture(texture.clone())
        .with_sync_with_texture_size(false)
        .build(ctx)
        .to_base()
}

fn vector(ctx: &mut BuildContext, primitives: Vec<Primitive>, color: [u8; 3]) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(color)).into()),
    )
    .with_primitives(primitives)
    .build(ctx)
    .to_base()
}

fn line(a: (f32, f32), b: (f32, f32), thickness: f32) -> Primitive {
    Primitive::Line {
        begin: Vector2::new(a.0, a.1),
        end: Vector2::new(b.0, b.1),
        thickness,
    }
}

fn ring(charge: u8) -> Vec<Primitive> {
    let point = |fraction: f32| {
        let a = fraction * std::f32::consts::TAU;
        (18.0 + a.sin() * 14.0, 18.0 - a.cos() * 14.0)
    };
    (0..48)
        .filter_map(|i| {
            let from = i as f32 / 48.0;
            let to = ((i + 1) as f32 / 48.0).min(charge as f32 / 100.0);
            (to > from).then(|| line(point(from), point(to), 2.5))
        })
        .collect()
}

fn number(value: f32) -> String {
    let raw = (value.round().max(0.0) as u64).to_string();
    let mut result = String::new();
    for (i, c) in raw.chars().enumerate() {
        if i > 0 && (raw.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

impl ScoreboardUi {
    pub fn new(ui: &mut UserInterface, manager: &ResourceManager) -> Self {
        let textures = portraits::load(manager);
        let ctx = &mut ui.build_ctx();
        let dim = block(ctx, Color::from_rgba(5, 10, 21, 218));
        let heading = caption(ctx, 24.0, WHITE, false);
        let time = caption(ctx, 15.0, MUTED, false);
        let mut children = vec![heading, time];
        let teams = std::array::from_fn(|_| caption(ctx, 14.0, WHITE, false));
        let headers =
            std::array::from_fn(|_| std::array::from_fn(|_| caption(ctx, 12.0, WHITE, true)));
        let mut rows = Vec::new();
        for side in 0..2 {
            let y = if side == 0 { 60.0 } else { 368.0 };
            let header = block(ctx, Color::from_rgba(33, 42, 59, 238));
            children.push(header);
            children.push(teams[side]);
            children.extend(headers[side]);
            place(ctx.inner(), header, 0.0, y, 760.0, 26.0);
            place(ctx.inner(), teams[side], 12.0, y + 3.0, 302.0, 22.0);
            for (node, (x, width)) in headers[side].iter().zip(COLS) {
                place(ctx.inner(), *node, x, y + 4.0, width, 20.0);
            }
            for index in 0..TEAM_SIZE {
                let background = block(ctx, Color::TRANSPARENT);
                let accent = block(ctx, rgb(if side == 0 { CYAN } else { RED }));
                let portrait = picture(ctx, &textures[0]);
                let roles = [
                    vector(
                        ctx,
                        vec![
                            Primitive::Triangle {
                                points: [
                                    Vector2::new(1.0, 2.0),
                                    Vector2::new(17.0, 2.0),
                                    Vector2::new(9.0, 18.0),
                                ],
                            },
                            Primitive::Triangle {
                                points: [
                                    Vector2::new(1.0, 2.0),
                                    Vector2::new(9.0, 18.0),
                                    Vector2::new(1.0, 10.0),
                                ],
                            },
                            Primitive::Triangle {
                                points: [
                                    Vector2::new(17.0, 2.0),
                                    Vector2::new(17.0, 10.0),
                                    Vector2::new(9.0, 18.0),
                                ],
                            },
                        ],
                        WHITE,
                    ),
                    vector(
                        ctx,
                        vec![
                            line((3.0, 3.0), (3.0, 16.0), 3.0),
                            line((9.0, 3.0), (9.0, 16.0), 3.0),
                            line((15.0, 3.0), (15.0, 16.0), 3.0),
                        ],
                        WHITE,
                    ),
                    vector(
                        ctx,
                        vec![
                            line((9.0, 1.0), (9.0, 17.0), 5.0),
                            line((1.0, 9.0), (17.0, 9.0), 5.0),
                        ],
                        WHITE,
                    ),
                ];
                let name = caption(ctx, 17.0, WHITE, false);
                let detail = caption(ctx, 10.0, [211, 234, 245], false);
                let track = vector(
                    ctx,
                    vec![Primitive::WireCircle {
                        center: Vector2::new(18.0, 18.0),
                        radius: 14.0,
                        thickness: 2.0,
                        segments: 48,
                    }],
                    [44, 107, 125],
                );
                let ring = vector(ctx, ring(0), GOLD);
                let charge = caption(ctx, 11.0, WHITE, true);
                let values = std::array::from_fn(|_| caption(ctx, 16.0, WHITE, true));
                let mut nodes = vec![
                    background, accent, portrait, name, detail, track, ring, charge,
                ];
                nodes.extend(roles);
                nodes.extend(values);
                let root = CanvasBuilder::new(
                    WidgetBuilder::new()
                        .with_hit_test_visibility(false)
                        .with_width(760.0)
                        .with_height(40.0)
                        .with_children(nodes),
                )
                .build(ctx)
                .to_base();
                children.push(root);
                place(
                    ctx.inner(),
                    root,
                    0.0,
                    y + 26.0 + index as f32 * 40.0,
                    760.0,
                    40.0,
                );
                for (node, x, yy, w, h) in [
                    (background, 0.0, 0.0, 760.0, 39.0),
                    (accent, 0.0, 0.0, 4.0, 39.0),
                    (portrait, 34.0, 0.0, 40.0, 39.0),
                    (name, 86.0, 2.0, 181.0, 23.0),
                    (detail, 87.0, 25.0, 180.0, 14.0),
                    (track, 276.0, 1.0, 36.0, 36.0),
                    (ring, 276.0, 1.0, 36.0, 36.0),
                    (charge, 278.0, 11.0, 32.0, 18.0),
                ] {
                    place(ctx.inner(), node, x, yy, w, h);
                }
                for role in roles {
                    place(ctx.inner(), role, 10.0, 10.0, 18.0, 20.0);
                }
                for (node, (x, width)) in values.iter().zip(COLS) {
                    place(ctx.inner(), *node, x, 10.0, width, 26.0);
                }
                // Opposing ultimate charge is intentionally private.
                visible(ctx.inner(), track, side == 0);
                rows.push(Row {
                    root,
                    background,
                    accent,
                    portrait,
                    roles,
                    name,
                    detail,
                    track,
                    ring,
                    charge,
                    values,
                    last_charge: Cell::new(255),
                });
            }
        }
        let versus = caption(ctx, 22.0, WHITE, true);
        let left_line = block(ctx, Color::from_rgba(175, 198, 220, 65));
        let right_line = block(ctx, Color::from_rgba(175, 198, 220, 65));
        let profile = block(ctx, Color::from_rgba(35, 43, 61, 239));
        let profile_accent = block(ctx, rgb(CYAN));
        let portrait = picture(ctx, &textures[0]);
        let hero = caption(ctx, 31.0, CYAN, false);
        let role = caption(ctx, 14.0, MUTED, false);
        let state = caption(ctx, 14.0, CYAN, false);
        let hint = caption(ctx, 13.0, MUTED, false);
        let personal =
            std::array::from_fn(|i| caption(ctx, if i < 2 { 30.0 } else { 27.0 }, WHITE, i < 2));
        let captions = std::array::from_fn(|i| caption(ctx, 12.0, MUTED, i < 2));
        children.extend([
            versus,
            left_line,
            right_line,
            profile,
            profile_accent,
            portrait,
            hero,
            role,
            state,
            hint,
        ]);
        for i in 0..5 {
            if i < 2 {
                let tile = block(ctx, Color::from_rgba(17, 25, 40, 210));
                children.push(tile);
                place(
                    ctx.inner(),
                    tile,
                    928.0,
                    72.0 + i as f32 * 72.0,
                    160.0,
                    64.0,
                );
                place(
                    ctx.inner(),
                    personal[i],
                    928.0,
                    73.0 + i as f32 * 72.0,
                    160.0,
                    40.0,
                );
                place(
                    ctx.inner(),
                    captions[i],
                    928.0,
                    115.0 + i as f32 * 72.0,
                    160.0,
                    19.0,
                );
            } else {
                let y = 325.0 + (i - 2) as f32 * 76.0;
                place(ctx.inner(), personal[i], 802.0, y, 272.0, 40.0);
                place(ctx.inner(), captions[i], 802.0, y + 41.0, 272.0, 22.0);
            }
        }
        children.extend(personal);
        children.extend(captions);
        let card = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_width(WIDTH)
                .with_height(HEIGHT)
                .with_children(children),
        )
        .build(ctx)
        .to_base();
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_visibility(false)
                .with_children(vec![dim, card]),
        )
        .build(ctx)
        .to_base();
        for (node, x, y, w, h) in [
            (heading, 0.0, 0.0, 760.0, 33.0),
            (time, 1.0, 36.0, 760.0, 22.0),
            (versus, 220.0, 334.0, 320.0, 32.0),
            (left_line, 0.0, 347.0, 210.0, 1.0),
            (right_line, 550.0, 347.0, 210.0, 1.0),
            (profile, 778.0, 60.0, 322.0, 574.0),
            (profile_accent, 778.0, 60.0, 322.0, 3.0),
            (portrait, 790.0, 72.0, 126.0, 136.0),
            (hero, 802.0, 234.0, 274.0, 45.0),
            (role, 802.0, 282.0, 274.0, 24.0),
            (state, 802.0, 586.0, 274.0, 26.0),
            (hint, 0.0, 647.0, WIDTH, 19.0),
        ] {
            place(ui, node, x, y, w, h);
        }
        Self {
            root,
            dim,
            card,
            heading,
            time,
            teams,
            headers,
            rows,
            versus,
            portrait,
            hero,
            role,
            personal,
            captions,
            state,
            hint,
            textures,
        }
    }

    pub fn labels(&self) -> Vec<Handle<UiNode>> {
        let mut labels = vec![
            self.heading,
            self.time,
            self.versus,
            self.hero,
            self.role,
            self.state,
            self.hint,
        ];
        labels.extend(self.teams);
        labels.extend(self.headers.into_iter().flatten());
        labels.extend(self.personal);
        labels.extend(self.captions);
        for row in &self.rows {
            labels.extend([row.name, row.detail, row.charge]);
            labels.extend(row.values);
        }
        labels
    }

    pub fn render(&self, rt: &Runtime, ui: &mut UserInterface) {
        let show = showing(rt);
        visible(ui, self.root, show);
        if !show {
            return;
        }
        let size = ui.screen_size();
        let scale = ((size.x - 64.0) / WIDTH)
            .min((size.y - 64.0) / HEIGHT)
            .min(1.4)
            .max(0.1);
        place(ui, self.root, 0.0, 0.0, size.x, size.y);
        place(ui, self.dim, 0.0, 0.0, size.x, size.y);
        place(
            ui,
            self.card,
            (size.x - WIDTH * scale) * 0.5,
            (size.y - HEIGHT * scale) * 0.5,
            WIDTH,
            HEIGHT,
        );
        ui.send(
            self.card,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        let world = rt.session.world();
        let local = rt.session.slot();
        let friendly = team(local);
        let korean = rt.korean_font;
        let hero_name = |id| {
            let c = character(id);
            if korean {
                c.korean
            } else {
                c.name
            }
        };
        let time = world.remaining.max(0.0).ceil() as u32;
        text(
            ui,
            self.heading,
            if korean {
                format!("거점 점령  |  {}", world.map_id.display_name())
            } else {
                format!("CONTROL  |  {}", world.map_id.slug().to_uppercase())
            },
        );
        text(
            ui,
            self.time,
            if korean {
                format!(
                    "ROUND {:02}   ·   {}   ·   남은 시간 {}:{:02}",
                    world.round,
                    if world.attacker == friendly {
                        "공격"
                    } else {
                        "수비"
                    },
                    time / 60,
                    time % 60
                )
            } else {
                format!(
                    "ROUND {:02}   ·   {}   ·   TIME {}:{:02}",
                    world.round,
                    if world.attacker == friendly {
                        "ATTACK"
                    } else {
                        "DEFEND"
                    },
                    time / 60,
                    time % 60
                )
            },
        );
        for side in 0..2 {
            let actual_team = if side == 0 { friendly } else { 1 - friendly };
            text(
                ui,
                self.teams[side],
                format!(
                    "{}  ·  {}   {}/{}",
                    if side == 0 {
                        if korean {
                            "아군"
                        } else {
                            "ALLIES"
                        }
                    } else if korean {
                        "적군"
                    } else {
                        "ENEMIES"
                    },
                    team_name(actual_team),
                    world.team_count(actual_team),
                    TEAM_SIZE
                ),
            );
            for (node, title) in self.headers[side].into_iter().zip(if korean {
                ["처치", "도움", "죽음", "피해", "치유", "방어"]
            } else {
                ["E", "A", "D", "DMG", "H", "MIT"]
            }) {
                text(ui, node, title);
            }
            for index in 0..TEAM_SIZE {
                let slot = actual_team * TEAM_SIZE + index;
                let actor = &world.actors[slot];
                let row = &self.rows[side * TEAM_SIZE + index];
                let color = match (side, slot == local, index % 2) {
                    (0, true, _) => Color::from_rgba(22, 188, 214, 239),
                    (0, _, 0) => Color::from_rgba(8, 122, 150, 218),
                    (0, _, _) => Color::from_rgba(8, 108, 136, 218),
                    (1, _, 0) => Color::from_rgba(170, 35, 64, 218),
                    _ => Color::from_rgba(149, 29, 55, 218),
                };
                ui.send(
                    row.background,
                    WidgetMessage::Background(Brush::Solid(color).into()),
                );
                ui.send(
                    row.accent,
                    WidgetMessage::Background(
                        Brush::Solid(if slot == local {
                            Color::WHITE
                        } else {
                            rgb(if side == 0 { CYAN } else { RED })
                        })
                        .into(),
                    ),
                );
                ui.send(
                    row.root,
                    WidgetMessage::Opacity(Some(if actor.active { 1.0 } else { 0.38 })),
                );
                let c = character(actor.character);
                let role = if c.healer {
                    2
                } else if c.position == trickcal_shooting_balddagu::roster::Position::Front {
                    0
                } else {
                    1
                };
                for (i, node) in row.roles.iter().enumerate() {
                    visible(ui, *node, actor.active && i == role);
                }
                for node in [row.portrait, row.detail].into_iter().chain(row.values) {
                    visible(ui, node, actor.active);
                }
                visible(ui, row.charge, actor.active && side == 0);
                visible(ui, row.track, actor.active && side == 0);
                visible(ui, row.ring, actor.active && side == 0);
                if !actor.active {
                    text(ui, row.name, if korean { "빈 슬롯" } else { "EMPTY SLOT" });
                    continue;
                }
                let texture = &self.textures[actor.character as usize];
                ui.send(row.portrait, ImageMessage::Texture(Some(texture.clone())));
                ui.send(
                    row.portrait,
                    ImageMessage::UvRect(portraits::uv(
                        actor.character as usize,
                        texture,
                        40.0,
                        39.0,
                    )),
                );
                ui.send(
                    row.portrait,
                    WidgetMessage::Opacity(Some(if actor.alive() { 1.0 } else { 0.35 })),
                );
                text(
                    ui,
                    row.name,
                    if actor.human {
                        rt.session.player_name(slot)
                    } else {
                        hero_name(actor.character).to_owned()
                    },
                );
                let tag = if slot == local {
                    if korean {
                        "나"
                    } else {
                        "YOU"
                    }
                } else if actor.human {
                    "PLAYER"
                } else {
                    "BOT"
                };
                text(
                    ui,
                    row.detail,
                    if actor.alive() {
                        format!("{} · {tag}", hero_name(actor.character))
                    } else {
                        format!(
                            "{} · {} {:.0}s",
                            hero_name(actor.character),
                            if korean { "부활" } else { "RESPAWN" },
                            actor.respawn.ceil()
                        )
                    },
                );
                let charge = actor.combat.ultimate.floor().clamp(0.0, 100.0) as u8;
                text(
                    ui,
                    row.charge,
                    if charge == 100 {
                        "Q".to_string()
                    } else {
                        charge.to_string()
                    },
                );
                if row.last_charge.replace(charge) != charge {
                    let arc = ui[row.ring].cast_mut::<VectorImage>().unwrap();
                    arc.primitives.set_value_and_mark_modified(ring(charge));
                    arc.invalidate_visual();
                }
                let stats = actor.stats;
                for (node, value) in row.values.into_iter().zip([
                    (actor.kills as u32 + stats.assists as u32).to_string(),
                    stats.assists.to_string(),
                    actor.deaths.to_string(),
                    number(stats.damage),
                    number(stats.healing),
                    number(stats.mitigated),
                ]) {
                    text(ui, node, value);
                }
            }
        }
        text(
            ui,
            self.versus,
            format!(
                "{}     VS     {}",
                world.scores[friendly],
                world.scores[1 - friendly]
            ),
        );
        let actor = &world.actors[local];
        let c = character(actor.character);
        let texture = &self.textures[actor.character as usize];
        ui.send(self.portrait, ImageMessage::Texture(Some(texture.clone())));
        ui.send(
            self.portrait,
            ImageMessage::UvRect(portraits::uv(
                actor.character as usize,
                texture,
                126.0,
                136.0,
            )),
        );
        text(ui, self.hero, hero_name(actor.character));
        let position = if korean {
            ["전열", "중열", "후열"][c.position as usize]
        } else {
            c.position.name()
        };
        text(
            ui,
            self.role,
            format!(
                "{position}   /   {}",
                if korean {
                    "경기 누적 기록"
                } else {
                    "MATCH TOTALS"
                }
            ),
        );
        for (node, value) in self.personal.into_iter().zip([
            actor.kills.to_string(),
            actor.stats.solo_kills.to_string(),
            number(actor.stats.damage),
            number(actor.stats.healing),
            number(actor.stats.mitigated),
        ]) {
            text(ui, node, value);
        }
        for (node, title) in self.captions.into_iter().zip(if korean {
            [
                "마무리 일격",
                "단독 처치",
                "가한 피해",
                "유효 치유",
                "막은 피해",
            ]
        } else {
            [
                "FINAL BLOWS",
                "SOLO KILLS",
                "DAMAGE DEALT",
                "HEALING DONE",
                "DAMAGE MITIGATED",
            ]
        }) {
            text(ui, node, title);
        }
        text(
            ui,
            self.state,
            if actor.alive() {
                format!(
                    "{}   ·   Q  {}%",
                    if korean { "생존 중" } else { "ALIVE" },
                    actor.combat.ultimate.floor() as u32
                )
            } else {
                format!(
                    "{}   {:.0}s",
                    if korean { "부활까지" } else { "RESPAWN IN" },
                    actor.respawn.ceil()
                )
            },
        );
        text(
            ui,
            self.hint,
            if korean {
                "TAB 놓으면 닫기   ·   처치 = 마무리 + 도움   ·   아군 원형 숫자 = 궁극기 충전"
            } else {
                "RELEASE TAB TO CLOSE   ·   E = FINAL BLOWS + ASSISTS   ·   ALLY RING = ULTIMATE CHARGE"
            },
        );
    }
}
