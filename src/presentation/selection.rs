//! Hero selection: a 3D bust preview, 2D illustration roster and authoritative confirmation.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    asset::manager::ResourceManager,
    gui::{
        image::{ImageBuilder, ImageMessage},
        widget::WidgetMaterial,
    },
    material::shader::{Shader, ShaderResource},
    scene::light::point::PointLightBuilder,
};
use trickcal_shooting_balddagu::roster::POSITION_LIMIT;

const GOLD: [u8; 3] = [255, 181, 79];
const WHITE: [u8; 3] = [242, 247, 250];
fn button_color(color: [u8; 3], enabled: bool) -> [u8; 3] {
    if enabled {
        color
    } else {
        color.map(|channel| (u16::from(channel) * 90 / 255) as u8)
    }
}
const CENTER_X: f32 = 640.0;
const RIGHT_EDGE: f32 = 1240.0;
const CONFIRM_RECT: (f32, f32, f32, f32) = (CENTER_X - 113.0, 745.0, 226.0, 43.0);
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
    let row_width = 2.0 * 368.0 + 4.0 * 64.0 + 58.0;
    rect(
        width,
        height,
        (
            CENTER_X - row_width * 0.5 + group as f32 * 368.0 + col as f32 * 64.0,
            642.0,
            58.0,
            80.0,
        ),
    )
}
pub fn confirm_rect(w: f32, h: f32) -> (f32, f32, f32, f32) {
    rect(w, h, CONFIRM_RECT)
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

impl Runtime {
    pub(super) fn click_selection_card(&mut self, id: usize, now: Instant) {
        let double_click = self
            .last_selection_click
            .take()
            .is_some_and(|(last_id, at)| {
                last_id == id && now.duration_since(at) <= std::time::Duration::from_millis(500)
            });
        self.selected = id;
        self.preview_serial += 1;
        if let Some(preview) = &mut self.preview {
            preview.cheeks.cancel();
        }
        self.selection_message.clear();
        if double_click {
            self.confirm_selection();
        } else {
            self.last_selection_click = Some((id, now));
        }
    }

    /// Consume both edges and repeats so confirmation cannot also become a jump.
    pub(super) fn confirm_selection_key(&mut self, code: KeyCode, down: bool, repeat: bool) -> bool {
        if !self.selecting || !matches!(code, KeyCode::Space | KeyCode::F5) {
            return false;
        }
        if down && !repeat && showing(self) {
            self.confirm_selection();
        }
        true
    }
}

fn start_text(world: &World, korean: bool) -> String {
    match world.phase {
        Phase::Setup => {
            let seconds = world.phase_time.max(0.0).ceil() as u32;
            if korean {
                format!("게임 시작까지 {seconds}초")
            } else {
                format!("GAME STARTS IN {seconds}s")
            }
        }
        Phase::Playing => if korean {
            "게임 시작! 영웅을 선택하세요"
        } else {
            "MATCH STARTED / SELECT A HERO"
        }
        .into(),
        _ => String::new(),
    }
}

// --selection-review --selection-capacity-review captures each row from an
// outsider, an existing member, a reopened slot, and the opposing team.
pub(super) fn review_capacity(rt: &mut Runtime) {
    let position = rt.review_capture / 4;
    let stage = rt.review_capture % 4;
    if rt.review_frames == 0 {
        let mut world = World::default();
        let other = (position + 1) % 3;
        let rear = (position + 2) % 3;
        let first = if stage == 3 { TEAM_SIZE } else { 0 };
        for (offset, id) in [other, position, position + 3, position + 6, rear, rear + 3]
            .into_iter()
            .enumerate()
        {
            world.actors[first + offset].character = id;
        }
        if stage == 1 {
            world.actors[0].character = position;
            world.actors[1].character = other;
        } else if stage == 2 {
            world.clear_slot(3);
        }
        for actor in &mut world.actors {
            actor.human = true;
        }
        rt.session = Session::Solo(Box::new(world));
        rt.preview_serial += 1;
    }
    // Previewing an unavailable hero must not grant membership of its row.
    rt.selected = (position + 9) as usize;
}

// --selection-review --selection-synergy-review: mixed, paired, unpaired,
// one member, and an empty team. The first frame matches the 3/2/1 example.
pub(super) fn review_synergy(rt: &mut Runtime) {
    if rt.review_frames == 0 {
        let heroes: &[u8] = match rt.review_capture {
            0 => &[0, 1, 2, 6, 7, 3],
            1 => &[0, 1, 3, 4, 6, 7],
            2 => &[0, 1, 3, 7, 11, 14],
            3 => &[0],
            _ => &[],
        };
        let mut world = World::default();
        for slot in 0..TEAM_SIZE {
            world.clear_slot(slot);
            if let Some(&id) = heroes.get(slot) {
                world.actors[slot].active = true;
                world.actors[slot].human = true;
                world.actors[slot].character = id;
            }
        }
        rt.session = Session::Solo(Box::new(world));
    }
    rt.selected = 0;
}

#[derive(Debug, Default)]
pub struct SelectionUi {
    confirm: Handle<UiNode>,
    confirm_text: Handle<UiNode>,
    title: Handle<UiNode>,
    subtitle: Handle<UiNode>,
    countdown: Handle<UiNode>,
    name: Handle<UiNode>,
    trait_icons: [Handle<UiNode>; 3],
    status: Handle<UiNode>,
    back: Handle<UiNode>,
    synergy_icons: [Handle<UiNode>; TEAM_SIZE],
    synergy_materials: [MaterialResource; TEAM_SIZE],
    roles: [Handle<UiNode>; 3],
    role_boxes: [Handle<UiNode>; 3],
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
    card_image(ctx, texture, WidgetMaterial::default())
}
fn card_image(
    ctx: &mut BuildContext,
    texture: TextureResource,
    material: WidgetMaterial,
) -> Handle<UiNode> {
    ImageBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_material(material),
    )
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
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("selection_card.shader"))
                .expect("selection card shader"),
        );
        let synergy_materials: [MaterialResource; TEAM_SIZE] =
            std::array::from_fn(|_| MaterialResource::new(Material::from_shader(shader.clone())));
        let ctx = &mut ui.build_ctx();
        // All backgrounds precede their contents in the root canvas draw order.
        let role_boxes = std::array::from_fn(|_| panel(ctx, rgb([25, 44, 63])));
        let cards = std::array::from_fn(|_| panel(ctx, rgb([55, 75, 92])));
        let team_cards = std::array::from_fn(|_| panel(ctx, Color::from_rgba(25, 44, 63, 230)));
        let confirm = panel(ctx, rgb(GOLD));
        let selection = Self {
            role_boxes,
            cards,
            team_cards,
            confirm,
            title: label(ctx, "", 34.0, WHITE),
            subtitle: label(ctx, "", 17.0, MINT),
            countdown: label(ctx, "", 26.0, GOLD),
            name: label(ctx, "", 72.0, WHITE),
            trait_icons: [
                portrait(ctx, markers.race(ROSTER[0].race)),
                portrait(ctx, markers.position(ROSTER[0].position)),
                portrait(ctx, markers.personality(ROSTER[0].personality)),
            ],
            status: wrapped(ctx, GOLD),
            back: label(ctx, "", 15.0, WHITE),
            confirm_text: label(ctx, "", 20.0, INK),
            synergy_icons: std::array::from_fn(|i| {
                card_image(
                    ctx,
                    markers.personality(ROSTER[0].personality),
                    WidgetMaterial(synergy_materials[i].clone()),
                )
            }),
            synergy_materials,
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
        };
        ui.send(
            selection.name,
            TextMessage::HorizontalAlignment(fyrox::gui::HorizontalAlignment::Right),
        );
        for h in selection
            .names
            .into_iter()
            .chain(selection.team_labels)
            .chain([selection.confirm_text])
        {
            ui.send(
                h,
                TextMessage::HorizontalAlignment(fyrox::gui::HorizontalAlignment::Center),
            );
        }
        ui.send(
            selection.confirm_text,
            TextMessage::VerticalAlignment(fyrox::gui::VerticalAlignment::Center),
        );
        selection
    }
    pub fn labels(&self) -> Vec<Handle<UiNode>> {
        [
            self.title,
            self.subtitle,
            self.countdown,
            self.name,
            self.status,
            self.back,
            self.confirm_text,
        ]
        .into_iter()
        .chain(self.roles)
        .chain(self.names)
        .chain(self.team_labels)
        .collect()
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface, show: bool) {
        for h in self
            .labels()
            .into_iter()
            .chain([self.confirm])
            .chain(self.cards)
            .chain(self.portraits)
            .chain(self.team_cards)
            .chain(self.team_images)
            .chain(self.trait_icons)
            .chain(self.synergy_icons)
            .chain(self.role_icons)
            .chain(self.role_boxes)
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
        let counts = w.position_counts(side);
        let pc = w.personality_counts(side);
        txt(
            self.title,
            (42.0, 32.0, 420.0, 54.0),
            32.0,
            if korean {
                "경쟁전".into()
            } else {
                "COMPETITIVE".into()
            },
        );
        txt(
            self.subtitle,
            (44.0, 88.0, 420.0, 32.0),
            18.0,
            format!(
                "{} / {}",
                if korean {
                    w.map_id.display_name()
                } else {
                    match w.map_id {
                        MapId::FairyKingdomCastle => "FAIRY KINGDOM CASTLE",
                        MapId::ElfCity => "ELF CITY",
                    }
                },
                if side == w.attacker {
                    "ATTACK"
                } else {
                    "DEFEND"
                },
            ),
        );
        txt(
            self.name,
            (RIGHT_EDGE - 600.0, 55.0, 600.0, 100.0),
            72.0,
            if korean {
                c.korean.into()
            } else {
                c.name.into()
            },
        );
        txt(
            self.countdown,
            (470.0, 38.0, 410.0, 48.0),
            if korean { 26.0 } else { 22.0 },
            start_text(w, korean),
        );
        ui.send(
            self.countdown,
            TextMessage::HorizontalAlignment(fyrox::gui::HorizontalAlignment::Center),
        );
        let trait_textures = [
            self.markers.race(c.race),
            self.markers.position(c.position),
            self.markers.personality(c.personality),
        ];
        for (i, texture) in trait_textures.into_iter().enumerate() {
            let x = RIGHT_EDGE - 48.0 - (2 - i) as f32 * 64.0;
            put(self.trait_icons[i], (x, 166.0, 48.0, 48.0));
            ui.send(self.trait_icons[i], ImageMessage::Texture(Some(texture)));
        }
        // Group larger sets first, keeping roster order for ties. A leftover
        // member follows its colored pairs in grayscale, with no text/counts.
        let mut order: [usize; 5] = std::array::from_fn(|i| i);
        order.sort_by_key(|&i| std::cmp::Reverse(pc[i]));
        let mut members = order.into_iter().flat_map(|personality| {
            (0..pc[personality]).map(move |member| (personality, member < pc[personality] / 2 * 2))
        });
        let icon_size = 32.0;
        let step = icon_size * 0.5;
        let start_x = 44.0;
        for (i, &icon) in self.synergy_icons.iter().enumerate() {
            let Some((personality, paired)) = members.next() else {
                visible(ui, icon, false);
                continue;
            };
            put(
                icon,
                (start_x + i as f32 * step, 170.0, icon_size, icon_size),
            );
            ui.send(
                icon,
                ImageMessage::Texture(Some(
                    self.markers
                        .personality(ROSTER[personality * 3].personality),
                )),
            );
            self.synergy_materials[i]
                .data_ref()
                .set_property("grayscale", if paired { 0.0_f32 } else { 1.0_f32 });
        }
        let team_width = (TEAM_SIZE - 1) as f32 * 96.0 + 86.0;
        for i in 0..TEAM_SIZE {
            let a = &w.actors[side * TEAM_SIZE + i];
            let x = CENTER_X - team_width * 0.5 + i as f32 * 96.0;
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
            // Center each capacity box above its corresponding hero group.
            let (group_x, card_y, _, _) = card_rect(1280.0, 800.0, i);
            let x = group_x + (4.0 * 64.0 + 58.0 - 140.0) * 0.5;
            let y = card_y - 40.0;
            let full = *count >= POSITION_LIMIT;
            put(self.role_boxes[i], (x, y, 140.0, 36.0));
            ui.send(
                self.role_boxes[i],
                WidgetMessage::Background(
                    Brush::Solid(rgb(if full { [83, 57, 34] } else { [25, 44, 63] })).into(),
                ),
            );
            put(self.role_icons[i], (x + 10.0, y + 5.0, 26.0, 26.0));
            ui.send(
                self.roles[i],
                WidgetMessage::Foreground(
                    Brush::Solid(rgb(if full { GOLD } else { WHITE })).into(),
                ),
            );
            txt(
                self.roles[i],
                (x + 48.0, y + 3.0, 82.0, 30.0),
                22.0,
                format!("{count} / {POSITION_LIMIT}"),
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
            let enabled = w.selection_check(slot, id as u8).is_ok();
            let color = if !enabled {
                [65, 86, 108]
            } else if id == rt.selected {
                GOLD
            } else if me.character as usize == id {
                MINT
            } else {
                [65, 86, 108]
            };
            ui.send(
                self.cards[id],
                WidgetMessage::Background(Brush::Solid(rgb(button_color(color, enabled))).into()),
            );
            ui.send(
                self.names[id],
                WidgetMessage::Foreground(Brush::Solid(rgb(button_color(WHITE, enabled))).into()),
            );
            // Tint RGB without reducing alpha or removing the original hues.
            for image in [self.portraits[id], self.card_markers[id]] {
                ui.send(
                    image,
                    WidgetMessage::Background(
                        Brush::Solid(rgb(button_color([255; 3], enabled))).into(),
                    ),
                );
            }
        }
        let pending = matches!(&rt.session,Session::Join(c) if c.selection_pending());
        let result = w.selection_check(slot, rt.selected as u8);
        let allowed = result.is_ok() && !pending;
        let same = me.character as usize == rt.selected;
        let waiting = rt.selection_confirmed() && w.phase == Phase::Setup;
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
        } else if waiting {
            String::new()
        } else if rt.selection_message != SelectionResult::Accepted.message() {
            rt.selection_message.clone()
        } else {
            String::new()
        };
        // Leave room for the six-row team voice list under the synergy icons.
        txt(self.status, (44.0, 562.0, 285.0, 42.0), 15.0, status);
        let (x, y, cw, ch) = confirm_rect(size.x, size.y);
        place(ui, self.confirm, x, y, cw, ch);
        ui.send(
            self.confirm,
            WidgetMessage::Background(Brush::Solid(rgb(button_color(GOLD, allowed))).into()),
        );
        txt(
            self.confirm_text,
            CONFIRM_RECT,
            18.0,
            if korean {
                if pending {
                    "확인 중…"
                } else if waiting && same {
                    "시작 대기 중"
                } else if same {
                    "준비 완료"
                } else {
                    "선택 확정"
                }
                .into()
            } else {
                if pending {
                    "CONFIRMING…"
                } else if waiting && same {
                    "WAITING FOR START"
                } else if same {
                    "READY"
                } else {
                    "SELECT"
                }
                .into()
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
    lighting: Handle<Node>,
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
        let lighting = PivotBuilder::new(
            BaseBuilder::new()
                .with_name("Selection studio dome")
                .with_enabled(false)
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(120.0, 0.0, 0.0))
                        .build(),
                ),
        )
        .build(g)
        .to_base();
        // A shadowless dome of broad fills lifts faces, hair and the underside of
        // hats without changing the map's exposure or washing out painted colors.
        for (name, position, color, intensity) in [
            ("Warm key", [-2.5, 3.8, 4.5], [255, 248, 238], 4.5),
            ("Cool front fill", [3.0, 2.8, 4.0], [244, 250, 255], 4.0),
            ("Overhead fill", [-1.0, 5.0, 0.5], [255, 255, 255], 2.4),
            ("Left wrap", [-4.0, 1.8, 0.3], [255, 248, 242], 2.4),
            ("Right rim", [4.0, 2.0, -1.0], [240, 248, 255], 2.4),
            ("Lower bounce", [0.0, -2.0, 2.0], [255, 244, 235], 1.4),
        ] {
            let light: Handle<Node> = PointLightBuilder::new(
                BaseLightBuilder::new(
                    BaseBuilder::new()
                        .with_name(format!("Selection dome / {name}"))
                        .with_cast_shadows(false)
                        .with_local_transform(
                            TransformBuilder::new()
                                .with_local_position(Vector3::from(position))
                                .build(),
                        ),
                )
                .with_color(rgb(color))
                .with_intensity(intensity),
            )
            .with_radius(12.0)
            .build(g)
            .to_base();
            g.link_nodes(light, lighting);
        }
        selection_backdrop::isolate_preview(g, lighting);
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
        selection_backdrop::isolate_preview(&mut scene.graph, root);
        Self {
            root,
            lighting,
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
            selection_backdrop::isolate_preview(&mut scene.graph, self.root);
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
        scene.graph[self.root].set_visible(show);
        scene.graph[self.root].set_active(show);
        scene.graph[self.lighting].set_enabled(show);
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
            self.visual.prepare_chloe_arms(g, [self.cheeks.interaction.phase != trickcal_shooting_balddagu::cheeks::Phase::Idle; 2]);
            self.cheeks.apply(g, camera, size);
            self.visual.update_wings(g, dt);
            if self.visual.id == 3
                && self.cheeks.interaction.phase == trickcal_shooting_balddagu::cheeks::Phase::Idle
            {
                self.visual.attach_weapons(g);
            }
        }
    }
    pub fn review_idle(&mut self, g: &mut Graph) {
        self.age = trickcal_shooting_balddagu::motion::spawn_seconds(self.visual.id);
        self.visual.selection_pose(g, self.age);
    }
    pub fn review_cheek_arms(&self, g: &Graph) {
        self.visual.review_chloe_arms(g);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn selecting_runtime() -> Runtime {
        let mut rt = Runtime::new(
            Session::Solo(Box::new(World::room(MapId::default(), true))),
            None,
            false,
        );
        rt.open_lobby_selection(false);
        rt
    }

    #[test]
    fn double_click_confirms_but_single_slow_and_different_card_clicks_only_preview() {
        let mut rt = selecting_runtime();
        let now = Instant::now();
        let original = rt.session.world().actors[0].character;
        rt.click_selection_card(4, now);
        assert_eq!(rt.selected, 4);
        assert_eq!(rt.session.world().actors[0].character, original);
        assert!(rt.pending_ready.is_none());

        rt.click_selection_card(4, now + Duration::from_millis(501));
        assert!(rt.pending_ready.is_none());
        rt.click_selection_card(7, now + Duration::from_millis(600));
        assert_eq!(rt.selected, 7);
        assert!(rt.pending_ready.is_none());
        rt.click_selection_card(7, now + Duration::from_millis(800));
        assert_eq!(rt.session.world().actors[0].character, 7);
        assert_eq!(rt.pending_ready, Some(7));
        assert!(rt.last_selection_click.is_none());
        assert!(rt.sync_selection());
        assert!(rt.lobby.open && !rt.selecting);

        rt.open_lobby_selection(false);
        rt.click_selection_card(4, now + Duration::from_millis(900));
        rt.open_lobby_selection(false);
        rt.click_selection_card(4, now + Duration::from_millis(1000));
        assert!(rt.pending_ready.is_none());
    }

    #[test]
    fn space_and_f5_confirm_only_on_first_press_without_jumping() {
        for code in [KeyCode::Space, KeyCode::F5] {
            let mut rt = selecting_runtime();
            rt.click_selection_card(4, Instant::now());
            assert!(rt.confirm_selection_key(code, false, false));
            assert!(rt.confirm_selection_key(code, true, true));
            assert!(rt.pending_ready.is_none());
            rt.keys.score = true;
            assert!(rt.confirm_selection_key(code, true, false));
            assert!(rt.pending_ready.is_none());
            rt.keys.score = false;
            assert!(rt.confirm_selection_key(code, true, false));
            assert_eq!(rt.session.world().actors[0].character, 4);
            assert_eq!(rt.pending_ready, Some(4));
            assert!(!rt.keys.jump);
            assert!(rt.sync_selection());
            assert!(!rt.confirm_selection_key(code, true, false));
        }
    }

    #[test]
    fn shortcuts_reject_a_teammates_hero() {
        for double_click in [false, true] {
            let mut rt = selecting_runtime();
            let Session::Solo(world) = &mut rt.session else {
                unreachable!()
            };
            world.actors[1].active = true;
            world.actors[1].human = true;
            world.actors[1].character = 4;
            let original = world.actors[0].character;
            let now = Instant::now();
            rt.click_selection_card(4, now);
            if double_click {
                rt.click_selection_card(4, now + Duration::from_millis(100));
            } else {
                assert!(rt.confirm_selection_key(KeyCode::Space, true, false));
            }
            assert_eq!(rt.session.world().actors[0].character, original);
            assert!(rt.pending_ready.is_none());
            assert_eq!(rt.selection_message, SelectionResult::Taken.message());
        }
    }

    #[test]
    fn countdown_rounds_up_until_the_authoritative_match_start() {
        let mut world = World::default();
        assert!(start_text(&world, true).is_empty());
        world.start();
        for (time, seconds) in [(8.0, 8), (7.01, 8), (1.0, 1), (0.01, 1), (0.0, 0)] {
            world.phase_time = time;
            assert_eq!(
                start_text(&world, true),
                format!("게임 시작까지 {seconds}초")
            );
            assert_eq!(
                start_text(&world, false),
                format!("GAME STARTS IN {seconds}s")
            );
        }
        world.tick([Input::default(); PLAYERS]);
        assert_eq!(start_text(&world, true), "게임 시작! 영웅을 선택하세요");
    }

    #[test]
    fn client_requires_server_acceptance_and_matching_hero_even_after_countdown() {
        let server = Server::bind("127.0.0.1:0", true).unwrap();
        for result in [
            None,
            Some(SelectionResult::Taken),
            Some(SelectionResult::Accepted),
        ] {
            let mut client = Client::connect(server.addr().unwrap()).unwrap();
            client.slot = Some(0);
            client.world.phase = Phase::Playing;
            client.selection_result = result;
            let id = client.world.actors[0].character;
            let mut rt = Runtime::new(Session::Join(Box::new(client)), None, false);
            rt.home_open = false;
            rt.selecting = true;
            rt.pending_ready = Some(id);
            assert_eq!(
                rt.sync_selection(),
                result == Some(SelectionResult::Accepted)
            );
            assert_eq!(rt.selecting, result != Some(SelectionResult::Accepted));
            if result == Some(SelectionResult::Taken) {
                assert!(rt.pending_ready.is_none());
                assert_eq!(rt.selection_message, SelectionResult::Taken.message());
            }
        }
        let mut client = Client::connect(server.addr().unwrap()).unwrap();
        client.slot = Some(0);
        client.world.phase = Phase::Playing;
        client.selection_result = Some(SelectionResult::Accepted);
        let mut rt = Runtime::new(Session::Join(Box::new(client)), None, false);
        rt.selecting = true;
        rt.pending_ready = Some(4);
        assert!(!rt.sync_selection());
        assert!(rt.selecting);
    }
}
