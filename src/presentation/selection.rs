//! Hero selection: a 3D bust preview, 2D illustration roster and authoritative confirmation.
use super::*;
use fyrox::{
    asset::manager::ResourceManager,
    gui::image::{ImageBuilder, ImageMessage},
    scene::light::point::PointLightBuilder,
};

const GOLD: [u8; 3] = [255, 181, 79];
const WHITE: [u8; 3] = [242, 247, 250];
pub const ORDER: [usize; 15] = [0, 3, 6, 9, 12, 1, 4, 7, 10, 13, 2, 5, 8, 11, 14];
pub fn showing(rt: &Runtime) -> bool {
    !rt.home_open
        && !rt.lobby.open
        && !rt.art_review
        && !rt.keys.score
        && (rt.selecting || (rt.smoke.is_some() && rt.capture_stage == 0))
}
pub fn rect(width: f32, height: f32, r: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let s = (width / 1280.0).min(height / 800.0);
    (
        (width - 1280.0 * s) * 0.5 + r.0 * s,
        (height - 800.0 * s) * 0.5 + r.1 * s,
        r.2 * s,
        r.3 * s,
    )
}
pub fn card_rect(width: f32, height: f32, id: usize) -> (f32, f32, f32, f32) {
    let group = id % 3;
    let col = id / 3;
    rect(
        width,
        height,
        (
            92.0 + group as f32 * 368.0 + col as f32 * 64.0,
            642.0,
            58.0,
            80.0,
        ),
    )
}
pub fn confirm_rect(w: f32, h: f32) -> (f32, f32, f32, f32) {
    rect(w, h, (527.0, 745.0, 226.0, 43.0))
}
pub fn back_rect(w: f32, h: f32) -> (f32, f32, f32, f32) {
    rect(w, h, (1120.0, 750.0, 120.0, 35.0))
}
pub fn contains(p: Vector2<f32>, r: (f32, f32, f32, f32)) -> bool {
    p.x >= r.0 && p.y >= r.1 && p.x <= r.0 + r.2 && p.y <= r.1 + r.3
}
pub fn navigate(id: usize, offset: isize) -> usize {
    let index = ORDER.iter().position(|v| *v == id).unwrap_or(0) as isize;
    ORDER[(index + offset).rem_euclid(15) as usize]
}

#[derive(Debug, Default)]
pub struct SelectionUi {
    bottom: Handle<UiNode>,
    info_bg: Handle<UiNode>,
    confirm: Handle<UiNode>,
    confirm_text: Handle<UiNode>,
    title: Handle<UiNode>,
    subtitle: Handle<UiNode>,
    name: Handle<UiNode>,
    traits: [Handle<UiNode>; 3],
    trait_icons: [Handle<UiNode>; 3],
    detail: Handle<UiNode>,
    status: Handle<UiNode>,
    back: Handle<UiNode>,
    synergy: Handle<UiNode>,
    synergy_labels: [Handle<UiNode>; 5],
    synergy_icons: [Handle<UiNode>; 5],
    roles: [Handle<UiNode>; 3],
    role_icons: [Handle<UiNode>; 3],
    cards: [Handle<UiNode>; 15],
    portraits: [Handle<UiNode>; 15],
    names: [Handle<UiNode>; 15],
    card_markers: [Handle<UiNode>; 15],
    team_cards: [Handle<UiNode>; 6],
    team_images: [Handle<UiNode>; 6],
    team_labels: [Handle<UiNode>; 6],
    team_markers: [Handle<UiNode>; 6],
    textures: Vec<TextureResource>,
    markers: trait_markers::TraitMarkers,
}
fn portrait(ctx: &mut BuildContext, texture: TextureResource) -> Handle<UiNode> {
    ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
        .with_texture(texture)
        .with_sync_with_texture_size(false)
        .build(ctx)
        .to_base()
}
fn wrapped(ctx: &mut BuildContext, color: [u8; 3]) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(color)).into()),
    )
    .with_wrap(fyrox::gui::formatted_text::WrapMode::Word)
    .with_shadow(true)
    .build(ctx)
    .to_base()
}
fn short_name(id: u8, korean: bool) -> &'static str {
    if id == 14 && korean {
        "시온"
    } else if korean {
        character(id).korean
    } else {
        character(id).name
    }
}
impl SelectionUi {
    pub fn new(ui: &mut UserInterface, manager: &ResourceManager) -> Self {
        let textures = portraits::load(manager);
        let markers = trait_markers::TraitMarkers::new(manager);
        let ctx = &mut ui.build_ctx();
        // All backgrounds precede their contents in the root canvas draw order.
        let bottom = panel(ctx, Color::from_rgba(20, 33, 49, 242));
        let info_bg = panel(ctx, Color::from_rgba(25, 44, 63, 215));
        let cards = std::array::from_fn(|_| panel(ctx, rgb([55, 75, 92])));
        let team_cards = std::array::from_fn(|_| panel(ctx, Color::from_rgba(25, 44, 63, 230)));
        let confirm = panel(ctx, rgb(GOLD));
        Self {
            bottom,
            info_bg,
            cards,
            team_cards,
            confirm,
            title: label(ctx, "", 34.0, WHITE),
            subtitle: label(ctx, "", 17.0, MINT),
            name: label(ctx, "", 45.0, WHITE),
            traits: std::array::from_fn(|_| label(ctx, "", 18.0, GOLD)),
            trait_icons: [
                portrait(ctx, markers.personality(ROSTER[0].personality)),
                portrait(ctx, markers.race(ROSTER[0].race)),
                portrait(ctx, markers.position(ROSTER[0].position)),
            ],
            detail: wrapped(ctx, WHITE),
            status: wrapped(ctx, GOLD),
            back: label(ctx, "", 15.0, WHITE),
            confirm_text: label(ctx, "", 20.0, INK),
            synergy: label(ctx, "", 15.0, MINT),
            synergy_labels: std::array::from_fn(|_| label(ctx, "", 16.0, WHITE)),
            synergy_icons: std::array::from_fn(|i| {
                portrait(ctx, markers.personality(ROSTER[i * 3].personality))
            }),
            roles: std::array::from_fn(|_| label(ctx, "", 17.0, WHITE)),
            role_icons: std::array::from_fn(|i| {
                portrait(ctx, markers.position(ROSTER[i].position))
            }),
            portraits: std::array::from_fn(|i| portrait(ctx, textures[i].clone())),
            names: std::array::from_fn(|_| label(ctx, "", 12.0, WHITE)),
            card_markers: std::array::from_fn(|i| {
                portrait(ctx, markers.personality(ROSTER[i].personality))
            }),
            team_images: std::array::from_fn(|_| portrait(ctx, textures[0].clone())),
            team_labels: std::array::from_fn(|_| label(ctx, "", 12.0, WHITE)),
            team_markers: std::array::from_fn(|_| {
                portrait(ctx, markers.personality(ROSTER[0].personality))
            }),
            textures,
            markers,
        }
    }
    pub fn labels(&self) -> Vec<Handle<UiNode>> {
        [
            self.title,
            self.subtitle,
            self.name,
            self.detail,
            self.status,
            self.back,
            self.confirm_text,
            self.synergy,
        ]
        .into_iter()
        .chain(self.traits)
        .chain(self.synergy_labels)
        .chain(self.roles)
        .chain(self.names)
        .chain(self.team_labels)
        .collect()
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface, show: bool) {
        for h in self
            .labels()
            .into_iter()
            .chain([self.bottom, self.info_bg, self.confirm])
            .chain(self.cards)
            .chain(self.portraits)
            .chain(self.team_cards)
            .chain(self.team_images)
            .chain(self.trait_icons)
            .chain(self.synergy_icons)
            .chain(self.role_icons)
            .chain(self.card_markers)
            .chain(self.team_markers)
        {
            visible(ui, h, show);
        }
        if !show {
            return;
        }
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0);
        let put = |h, r| {
            let (x, y, w, v) = rect(size.x, size.y, r);
            place(ui, h, x, y, w, v);
        };
        let txt = |h, r, size: f32, value: String| {
            put(h, r);
            text_size(ui, h, size * scale);
            text(ui, h, value);
        };
        let korean = rt.korean_font;
        let w = rt.session.world();
        let slot = rt.session.slot();
        let me = &w.actors[slot];
        let side = team(slot);
        let c = character(rt.selected as u8);
        let k = kit(rt.selected as u8);
        let counts = w.position_counts(side);
        let pc = w.personality_counts(side);
        put(self.bottom, (40.0, 608.0, 1200.0, 126.0));
        put(self.info_bg, (895.0, 180.0, 345.0, 310.0));
        txt(
            self.title,
            (42.0, 32.0, 420.0, 54.0),
            32.0,
            if korean {
                "경쟁전  /  6 대 6".into()
            } else {
                "COMPETITIVE / 6 vs 6".into()
            },
        );
        txt(
            self.subtitle,
            (44.0, 88.0, 420.0, 70.0),
            18.0,
            format!(
                "{}  /  FAIRY KINGDOM CASTLE\n2CP  ·  {}  ·  ROUND {}",
                if side == w.attacker {
                    "ATTACK"
                } else {
                    "DEFEND"
                },
                if w.point == 0 { "POINT A" } else { "POINT B" },
                w.round
            ),
        );
        txt(
            self.name,
            (895.0, 55.0, 360.0, 65.0),
            if c.korean.chars().count() > 7 {
                32.0
            } else {
                44.0
            },
            if korean {
                c.korean.into()
            } else {
                c.name.into()
            },
        );
        let personalities = ["순수", "냉정", "광기", "활발", "우울"];
        let positions = ["전열", "중열", "후열"];
        let races = ["요정", "수인", "엘프", "정령", "유령", "용족", "마녀"];
        let trait_names = if korean {
            [
                personalities[c.personality as usize],
                races[c.race as usize],
                positions[c.position as usize],
            ]
        } else {
            [c.personality.name(), c.race.name(), c.position.name()]
        };
        let trait_textures = [
            self.markers.personality(c.personality),
            self.markers.race(c.race),
            self.markers.position(c.position),
        ];
        for (i, texture) in trait_textures.into_iter().enumerate() {
            let x = 898.0 + i as f32 * 114.0;
            put(self.trait_icons[i], (x, 125.0, 32.0, 32.0));
            ui.send(self.trait_icons[i], ImageMessage::Texture(Some(texture)));
            txt(
                self.traits[i],
                (x + 38.0, 130.0, 76.0, 30.0),
                if korean { 18.0 } else { 13.0 },
                trait_names[i].into(),
            );
        }
        txt(
            self.detail,
            (915.0, 197.0, 310.0, 281.0),
            15.0,
            format!(
                "LMB  {}  /  {:0.1}m\n\nE  {}  ({:0.1}s)\n\nQ  {}\n\n{}\n\n{}",
                k.weapon,
                k.range,
                if korean { k.skill_ko } else { k.skill },
                k.cooldown
                    * if c.race == trickcal_shooting_balddagu::roster::Race::Witch {
                        0.95
                    } else {
                        1.0
                    },
                if korean { k.ultimate_ko } else { k.ultimate },
                c.race.passive(),
                k.help
            ),
        );
        txt(
            self.synergy,
            (44.0, 190.0, 340.0, 32.0),
            16.0,
            if korean {
                format!("{}  팀 공격력 +{}%", team_name(side), w.damage_bonus(side))
            } else {
                format!(
                    "{} / TEAM DAMAGE +{}%",
                    team_name(side),
                    w.damage_bonus(side)
                )
            },
        );
        for (i, count) in pc.iter().enumerate() {
            let x = 44.0 + (i % 3) as f32 * 108.0;
            let y = 232.0 + (i / 3) as f32 * 42.0;
            put(self.synergy_icons[i], (x, y, 30.0, 30.0));
            txt(
                self.synergy_labels[i],
                (x + 35.0, y + 5.0, 73.0, 26.0),
                if korean { 16.0 } else { 11.0 },
                format!(
                    "{} {}",
                    if korean {
                        personalities[i]
                    } else {
                        ROSTER[i * 3].personality.name()
                    },
                    count,
                ),
            );
        }
        for i in 0..6 {
            let a = &w.actors[side * TEAM_SIZE + i];
            let x = 350.0 + i as f32 * 96.0;
            put(self.team_cards[i], (x, 522.0, 86.0, 75.0));
            put(self.team_images[i], (x + 15.0, 524.0, 54.0, 54.0));
            visible(ui, self.team_images[i], a.active);
            visible(ui, self.team_markers[i], a.active);
            ui.send(
                self.team_images[i],
                ImageMessage::Texture(Some(self.textures[a.character as usize].clone())),
            );
            put(self.team_markers[i], (x + 3.0, 525.0, 22.0, 22.0));
            ui.send(
                self.team_markers[i],
                ImageMessage::Texture(Some(
                    self.markers.personality(character(a.character).personality),
                )),
            );
            ui.send(
                self.team_images[i],
                ImageMessage::UvRect(portraits::uv(
                    a.character as usize,
                    &self.textures[a.character as usize],
                    54.0,
                    54.0,
                )),
            );
            ui.send(
                self.team_cards[i],
                WidgetMessage::Background(
                    Brush::Solid(rgb(if side * TEAM_SIZE + i == slot {
                        [43, 113, 121]
                    } else {
                        [34, 55, 73]
                    }))
                    .into(),
                ),
            );
            txt(
                self.team_labels[i],
                (x + 4.0, 578.0, 82.0, 20.0),
                11.0,
                if !a.active {
                    if korean { "빈 자리" } else { "EMPTY" }.into()
                } else {
                    format!(
                        "{} {}",
                        if a.human { "●" } else { "BOT" },
                        short_name(a.character, korean)
                    )
                },
            );
        }
        for (i, count) in counts.iter().enumerate() {
            let x = 92.0 + i as f32 * 368.0;
            put(self.role_icons[i], (x, 612.0, 26.0, 26.0));
            txt(
                self.roles[i],
                (x + 33.0, 614.0, 287.0, 27.0),
                16.0,
                format!(
                    "{}   {}/3",
                    if korean {
                        positions[i]
                    } else {
                        ["FRONT", "MIDDLE", "REAR"][i]
                    },
                    count
                ),
            );
        }
        for (id, hero) in ROSTER.iter().enumerate() {
            let (x, y, cw, ch) = card_rect(size.x, size.y, id);
            place(ui, self.cards[id], x, y, cw, ch);
            place(
                ui,
                self.card_markers[id],
                x + 2.0 * scale,
                y + 2.0 * scale,
                20.0 * scale,
                20.0 * scale,
            );
            place(
                ui,
                self.portraits[id],
                x + 3.0 * scale,
                y + 3.0 * scale,
                cw - 6.0 * scale,
                54.0 * scale,
            );
            ui.send(
                self.portraits[id],
                ImageMessage::UvRect(portraits::uv(id, &self.textures[id], 52.0, 54.0)),
            );
            place(
                ui,
                self.names[id],
                x + 3.0 * scale,
                y + 59.0 * scale,
                cw - 6.0 * scale,
                20.0 * scale,
            );
            ui.send(
                self.names[id],
                TextMessage::FontSize(
                    (if hero.korean.chars().count() > 7 {
                        8.5
                    } else {
                        11.0
                    } * scale)
                        .into(),
                ),
            );
            text(ui, self.names[id], short_name(id as u8, korean));
            let check = w.selection_check(slot, id as u8);
            let blocked = check.is_err() && check != Err(SelectionResult::Locked);
            let color = if id == rt.selected {
                GOLD
            } else if me.character as usize == id {
                MINT
            } else if blocked {
                [75, 50, 65]
            } else {
                [65, 86, 108]
            };
            ui.send(
                self.cards[id],
                WidgetMessage::Background(Brush::Solid(rgb(color)).into()),
            );
            ui.send(
                self.portraits[id],
                WidgetMessage::Opacity(Some(if blocked { 0.3 } else { 1.0 })),
            );
            ui.send(
                self.card_markers[id],
                WidgetMessage::Opacity(Some(if blocked { 0.3 } else { 1.0 })),
            );
        }
        let pending = matches!(&rt.session,Session::Join(c) if c.selection_pending());
        let result = w.selection_check(slot, rt.selected as u8);
        let allowed = result.is_ok() && !pending;
        let same = me.character as usize == rt.selected;
        let status = if pending {
            if korean {
                "서버 확인 중…".into()
            } else {
                "Waiting for server…".into()
            }
        } else if let Err(reason) = result {
            if korean {
                match reason {
                    SelectionResult::Locked => "현재 아군 스폰 구역에서 교체할 수 있습니다",
                    SelectionResult::Taken => "팀원이 선택한 캐릭터입니다",
                    SelectionResult::PositionFull => "해당 포지션은 이미 3명입니다",
                    _ => "선택할 수 없습니다",
                }
                .into()
            } else {
                reason.message().into()
            }
        } else if rt.selection_message != SelectionResult::Accepted.message() {
            rt.selection_message.clone()
        } else {
            String::new()
        };
        // Leave room for the six-row team voice list under the synergies.
        txt(self.status, (44.0, 562.0, 285.0, 42.0), 15.0, status);
        let (x, y, cw, ch) = confirm_rect(size.x, size.y);
        place(ui, self.confirm, x, y, cw, ch);
        ui.send(
            self.confirm,
            WidgetMessage::Background(
                Brush::Solid(rgb(if allowed { GOLD } else { [96, 105, 115] })).into(),
            ),
        );
        txt(
            self.confirm_text,
            (559.0, 752.0, 190.0, 32.0),
            18.0,
            if korean {
                if pending {
                    "확인 중…"
                } else if same {
                    "준비 완료"
                } else {
                    "선택 확정"
                }
                .into()
            } else {
                if same { "READY" } else { "SELECT" }.into()
            },
        );
        txt(
            self.back,
            (1120.0, 755.0, 120.0, 32.0),
            14.0,
            if korean {
                "ESC  돌아가기".into()
            } else {
                "ESC  BACK".into()
            },
        );
    }
}

#[derive(Debug)]
pub struct Preview {
    root: Handle<Node>,
    stage: [Handle<Node>; 3],
    visual: characters::Visual,
    age: f32,
    serial: u64,
    visible: bool,
    pub cheeks: cheeks::Cheeks,
}
impl Preview {
    pub fn new(scene: &mut Scene, models: &[fyrox::resource::model::ModelResource]) -> Self {
        let g = &mut scene.graph;
        let s = Shapes::new();
        let floor = s.solid(
            g,
            "Hero stage floor",
            V3::new(120.0, -0.18, 0.0),
            V3::new(40.0, 0.25, 28.0),
            [158, 187, 198],
            0,
        );
        let backdrop = s.solid(
            g,
            "Hero stage backdrop",
            V3::new(120.0, 4.0, -6.0),
            V3::new(40.0, 12.0, 0.4),
            [115, 148, 174],
            0,
        );
        let podium = s.solid(
            g,
            "Hero podium",
            V3::new(120.0, -0.025, 0.0),
            V3::new(2.4, 0.05, 2.4),
            [99, 200, 187],
            2,
        );
        PointLightBuilder::new(
            BaseLightBuilder::new(
                BaseBuilder::new().with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(119.0, 3.5, 4.0))
                        .build(),
                ),
            )
            .with_intensity(4.0),
        )
        .with_radius(16.0)
        .build(g);
        let root = PivotBuilder::new(
            BaseBuilder::new()
                .with_name("Selection preview (client only)")
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(120.0, 0.0, 0.0))
                        .build(),
                ),
        )
        .build(g)
        .to_base();
        let visual = characters::Visual::spawn(scene, root, &models[0], 0, &s, None);
        let cheeks = cheeks::Cheeks::new(&mut scene.graph, &visual, root);
        Self {
            root,
            stage: [floor, backdrop, podium],
            visual,
            age: 0.0,
            serial: 0,
            visible: false,
            cheeks,
        }
    }
    pub fn update(
        &mut self,
        scene: &mut Scene,
        models: &[fyrox::resource::model::ModelResource],
        id: u8,
        serial: u64,
        show: bool,
        dt: f32,
    ) {
        if self.visual.id != id {
            let new = characters::Visual::spawn(
                scene,
                self.root,
                &models[id as usize],
                id,
                &Shapes::new(),
                None,
            );
            std::mem::replace(&mut self.visual, new).remove(&mut scene.graph);
            self.cheeks = cheeks::Cheeks::new(&mut scene.graph, &self.visual, self.root);
            self.age = 0.0;
        }
        if serial != self.serial || show && !self.visible {
            self.age = 0.0;
            self.cheeks.cancel();
        }
        if !show {
            self.cheeks.cancel();
            self.cheeks.radius = 0.0;
        }
        self.serial = serial;
        self.visible = show;
        scene.graph[self.root].set_visibility(show);
        for node in self.stage {
            scene.graph[node].set_visibility(show);
        }
        // The frog rider needs headroom above its separately rigged mount.
        scene.graph[self.root]
            .local_transform_mut()
            .set_scale(Vector3::repeat(match id {
                10 => 0.90,
                _ => 1.0,
            }));
        if show {
            self.cheeks.interaction.tick(dt, id);
            if self.cheeks.interaction.phase != trickcal_shooting_balddagu::cheeks::Phase::Idle {
                self.age = trickcal_shooting_balddagu::motion::spawn_seconds(id);
            }
            self.visual.selection_pose(&mut scene.graph, self.age);
            self.age += dt;
        }
    }
    pub fn react(&mut self, g: &mut Graph, camera: Handle<Node>, size: Vector2<f32>, dt: f32) {
        if self.visible {
            self.cheeks.apply(g, camera, size);
            self.visual.update_wings(g, dt);
        }
    }
    pub fn review_idle(&mut self, g: &mut Graph) {
        self.age = trickcal_shooting_balddagu::motion::spawn_seconds(self.visual.id);
        self.visual.selection_pose(g, self.age);
    }

    pub fn review_spawn(&mut self, g: &mut Graph, phase: f32) {
        self.age = trickcal_shooting_balddagu::motion::spawn_seconds(self.visual.id) * phase;
        self.visual.selection_pose(g, self.age);
    }

    pub fn review_face(&mut self, g: &mut Graph, closed: bool) {
        self.visual.review_face(g, closed);
    }

    pub fn review_blink(&mut self, g: &mut Graph, closure: f32) {
        self.visual.review_blink(g, closure);
    }
}
