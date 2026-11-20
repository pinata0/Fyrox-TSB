//! Bottom-right regular skill and weapon HUD. Ultimate presentation is separate.
use super::*;
use fyrox::{
    core::algebra::Matrix3,
    gui::{
        canvas::CanvasBuilder,
        formatted_text::WrapMode,
        vector_image::{Primitive, VectorImageBuilder},
        HorizontalAlignment,
    },
};
use trickcal_shooting_balddagu::roster::Race;

const WHITE: [u8; 3] = [244, 249, 255];
const CYAN: [u8; 3] = [111, 222, 255];
const MUTED: [u8; 3] = [153, 170, 187];
const GOLD: [u8; 3] = [255, 182, 87];
const WIDTH: f32 = 340.0;
const HEIGHT: f32 = 164.0;

#[derive(Debug, Default)]
struct PortalHud {
    root: Handle<UiNode>,
    shades: [Handle<UiNode>; 2],
    bars: [Handle<UiNode>; 2],
    numbers: [Handle<UiNode>; 2],
    keys: [Handle<UiNode>; 2],
    name: Handle<UiNode>,
    shift: Handle<UiNode>,
}

impl PortalHud {
    fn new(ctx: &mut BuildContext) -> Self {
        let outline = block(ctx, Color::from_rgba(19, 31, 46, 240));
        let name = centered(ctx, 12.0);
        let shift = centered(ctx, 14.0);
        let shades = std::array::from_fn(|_| block(ctx, Color::from_rgba(8, 13, 23, 190)));
        let bars = std::array::from_fn(|i| block(ctx, rgb(portals::COLORS[i])));
        let numbers = std::array::from_fn(|_| centered(ctx, 25.0));
        let keys = std::array::from_fn(|_| centered(ctx, 11.0));
        let mut children = vec![outline, name, shift];
        place(ctx.inner(), outline, 2.0, 34.0, 82.0, 82.0);
        place(ctx.inner(), name, 0.0, 0.0, 88.0, 33.0);
        place(ctx.inner(), shift, 0.0, 121.0, 88.0, 24.0);
        for i in 0..2 {
            let x = 5.0 + i as f32 * 39.0;
            let bg = block(ctx, Color::from_rgba(30, 43, 60, 235));
            let ring = icon(
                ctx,
                (0..48)
                    .map(|s| {
                        let point = |n: f32| {
                            let a = n * std::f32::consts::TAU / 48.0;
                            Vector2::new(18.0 + a.cos() * 11.0, 25.0 + a.sin() * 20.0)
                        };
                        Primitive::Line {
                            begin: point(s as f32),
                            end: point(s as f32 + 1.0),
                            thickness: 2.4,
                        }
                    })
                    .collect(),
            );
            foreground(ctx.inner(), ring, portals::COLORS[i]);
            children.extend([bg, ring, shades[i], bars[i], numbers[i], keys[i]]);
            for (node, y, w, h) in [
                (bg, 37.0, 37.0, 76.0),
                (ring, 41.0, 37.0, 51.0),
                (shades[i], 37.0, 37.0, 76.0),
                (bars[i], 109.0, 37.0, 4.0),
                (numbers[i], 51.0, 37.0, 37.0),
                (keys[i], 91.0, 37.0, 17.0),
            ] {
                place(ctx.inner(), node, x, y, w, h);
            }
        }
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(88.0)
                .with_height(164.0)
                .with_hit_test_visibility(false)
                .with_visibility(false)
                .with_children(children),
        )
        .build(ctx)
        .to_base();
        Self {
            root,
            shades,
            bars,
            numbers,
            keys,
            name,
            shift,
        }
    }
    fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> {
        [self.name, self.shift]
            .into_iter()
            .chain(self.numbers)
            .chain(self.keys)
    }
    fn render(&self, rt: &Runtime, ui: &UserInterface, show: bool) {
        let w = rt.session.world();
        let slot = rt.session.slot();
        let actor = match &rt.session {
            Session::Join(c) => c.motion.actor(w, slot, slot),
            _ => w.actors[slot].clone(),
        };
        let show = show && trickcal_shooting_balddagu::portal::is_elf(&actor);
        visible(ui, self.root, show);
        if !show {
            return;
        }
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
        place(
            ui,
            self.root,
            size.x - 466.0 * scale,
            size.y - 216.0 * scale,
            88.0,
            164.0,
        );
        ui.send(
            self.root,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        text(
            ui,
            self.name,
            if rt.korean_font {
                "텔레포트"
            } else {
                "PORTALS"
            },
        );
        text(ui, self.shift, "SHIFT +");
        let blocked = !actor.alive() || actor.combat.skills_locked() || actor.combat.channeling() || actor.combat.komi.active();
        for i in 0..2 {
            let cd = actor.portals.cooldown[i];
            text_size(
                ui,
                self.numbers[i],
                if cd > 0.0 && cd < 1.0 { 19.0 } else { 25.0 },
            );
            let fraction = (cd / 5.0).clamp(0.0, 1.0);
            let x = 5.0 + i as f32 * 39.0;
            visible(ui, self.shades[i], cd > 0.0 || blocked);
            place(
                ui,
                self.shades[i],
                x,
                37.0,
                37.0,
                76.0 * if blocked { 1.0 } else { fraction },
            );
            place(ui, self.bars[i], x, 109.0, 37.0 * (1.0 - fraction), 4.0);
            text(
                ui,
                self.numbers[i],
                if cd >= 1.0 {
                    format!("{:.0}", cd.ceil())
                } else if cd > 0.0 {
                    format!("{:.1}", (cd * 10.0).ceil() / 10.0)
                } else if blocked {
                    "—".into()
                } else {
                    String::new()
                },
            );
            text(
                ui,
                self.keys[i],
                if rt.korean_font {
                    ["좌클", "우클"][i]
                } else {
                    ["LMB", "RMB"][i]
                },
            );
            foreground(ui, self.keys[i], portals::COLORS[i]);
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct CombatHud {
    chloe: super::chloe_hud::Hud,
    portals: PortalHud,
    phase: super::phase_hud::PhaseHud,
    root: Handle<UiNode>,
    tile: Handle<UiNode>,
    shade: Handle<UiNode>,
    progress: Handle<UiNode>,
    skill_icons: [Handle<UiNode>; 15],
    weapon_icons: [Handle<UiNode>; 15],
    skill_name: Handle<UiNode>,
    countdown: Handle<UiNode>,
    key: Handle<UiNode>,
    weapon_name: Handle<UiNode>,
    ammo: Handle<UiNode>,
    capacity: Handle<UiNode>,
    reload: Handle<UiNode>,
    magic: Handle<UiNode>,
}

fn block(ctx: &mut BuildContext, color: Color) -> Handle<UiNode> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_background(Brush::Solid(color).into()),
    )
    .with_corner_radius(2.0.into())
    .build(ctx)
    .to_base()
}
fn icon(ctx: &mut BuildContext, primitives: Vec<Primitive>) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(WHITE)).into()),
    )
    .with_primitives(primitives)
    .build(ctx)
    .to_base()
}
fn centered(ctx: &mut BuildContext, size: f32) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(WHITE)).into()),
    )
    .with_font_size(size.into())
    .with_horizontal_text_alignment(HorizontalAlignment::Center)
    .with_wrap(WrapMode::Letter)
    .with_shadow(true)
    .build(ctx)
    .to_base()
}
fn foreground(ui: &UserInterface, node: Handle<UiNode>, color: [u8; 3]) {
    ui.send(
        node,
        WidgetMessage::Foreground(Brush::Solid(rgb(color)).into()),
    );
}

impl CombatHud {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        let outline = block(ctx, Color::from_rgba(19, 31, 46, 230));
        let tile = block(ctx, rgb(WHITE));
        let weapon_bg = block(ctx, Color::from_rgba(20, 33, 49, 150));
        let top_rule = block(ctx, Color::from_rgba(111, 222, 255, 140));
        let bottom_rule = block(ctx, rgb(CYAN));
        let key_bg = block(ctx, Color::from_rgba(20, 33, 49, 230));
        let skill_icons = std::array::from_fn(|id| icon(ctx, super::combat_icons::skill(id as u8)));
        let weapon_icons =
            std::array::from_fn(|id| icon(ctx, super::combat_icons::weapon(id as u8)));
        // Dim the ready tile and sweep a dark area away as its cooldown expires.
        let shade = block(ctx, Color::from_rgba(14, 24, 37, 140));
        let progress = block(ctx, rgb(CYAN));
        let skill_name = centered(ctx, 12.0);
        let countdown = centered(ctx, 30.0);
        let key = centered(ctx, 16.0);
        let weapon_name = label(ctx, "", 12.0, MUTED);
        let ammo = centered(ctx, 35.0);
        let capacity = label(ctx, "", 19.0, MUTED);
        let reload = label(ctx, "", 13.0, WHITE);
        let magic = label(ctx, "", 12.0, CYAN);
        let children: Vec<_> = [outline, tile, weapon_bg, top_rule, bottom_rule, key_bg]
            .into_iter()
            .chain(skill_icons)
            .chain(weapon_icons)
            .chain([
                shade,
                progress,
                skill_name,
                countdown,
                key,
                weapon_name,
                ammo,
                capacity,
                reload,
                magic,
            ])
            .collect();
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(WIDTH)
                .with_height(HEIGHT)
                .with_hit_test_visibility(false)
                .with_children(children),
        )
        .build(ctx)
        .to_base();
        for (node, x, y, w, h) in [
            (outline, 2.0, 34.0, 82.0, 82.0),
            (tile, 5.0, 37.0, 76.0, 76.0),
            (shade, 5.0, 37.0, 76.0, 76.0),
            (progress, 5.0, 109.0, 76.0, 4.0),
            (skill_name, 0.0, 0.0, 88.0, 33.0),
            (countdown, 5.0, 52.0, 76.0, 44.0),
            (key_bg, 29.0, 121.0, 28.0, 24.0),
            (key, 29.0, 121.0, 28.0, 24.0),
            (weapon_name, 100.0, 12.0, 240.0, 22.0),
            (weapon_bg, 100.0, 37.0, 240.0, 76.0),
            (top_rule, 100.0, 34.0, 240.0, 2.0),
            (bottom_rule, 100.0, 114.0, 240.0, 2.0),
            (ammo, 225.0, 46.0, 57.0, 49.0),
            (capacity, 282.0, 62.0, 58.0, 30.0),
            (reload, 100.0, 122.0, 240.0, 23.0),
            (magic, 100.0, 145.0, 240.0, 19.0),
        ] {
            place(ctx.inner(), node, x, y, w, h);
        }
        for node in skill_icons {
            place(ctx.inner(), node, 11.0, 43.0, 64.0, 64.0);
        }
        for node in weapon_icons {
            place(ctx.inner(), node, 106.0, 43.0, 112.0, 64.0);
        }
        Self {
            chloe: super::chloe_hud::Hud::new(ctx),
            portals: PortalHud::new(ctx),
            phase: super::phase_hud::PhaseHud::new(ctx),
            root,
            tile,
            shade,
            progress,
            skill_icons,
            weapon_icons,
            skill_name,
            countdown,
            key,
            weapon_name,
            ammo,
            capacity,
            reload,
            magic,
        }
    }

    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> {
        [
            self.skill_name,
            self.countdown,
            self.key,
            self.weapon_name,
            self.ammo,
            self.capacity,
            self.reload,
            self.magic,
        ]
        .into_iter()
        .chain(self.chloe.labels())
        .chain(self.phase.labels())
        .chain(self.portals.labels())
    }

    pub fn render(&self, rt: &Runtime, ui: &mut UserInterface, show: bool) {
        self.chloe.render(rt,ui,show);
        self.portals.render(rt, ui, show);
        self.phase.render(rt, ui, show);
        visible(ui, self.root, show);
        if !show {
            return;
        }
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
        place(
            ui,
            self.root,
            size.x - (WIDTH + 26.0) * scale,
            size.y - 216.0 * scale,
            WIDTH,
            HEIGHT,
        );
        ui.send(
            self.root,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        let actor = &rt.session.world().actors[rt.session.slot()];
        let id = actor.character as usize;
        let kit = kit(actor.character);
        let cooling = actor.ability_cooldown > 0.0;
        let blocked = !actor.alive() || actor.combat.skills_locked() || actor.combat.channeling() || actor.combat.komi.active();
        let ready = !cooling && !blocked;
        for i in 0..self.skill_icons.len() {
            visible(ui, self.skill_icons[i], i == id);
            visible(ui, self.weapon_icons[i], i == id);
        }
        foreground(ui, self.skill_icons[id], if ready { INK } else { MUTED });
        foreground(
            ui,
            self.weapon_icons[id],
            if actor.alive() { WHITE } else { MUTED },
        );
        ui.send(
            self.tile,
            WidgetMessage::Background(
                Brush::Solid(rgb(if ready { WHITE } else { [64, 80, 98] })).into(),
            ),
        );
        let duration = kit.cooldown
            * if character(actor.character).race == Race::Witch {
                0.95
            } else {
                1.0
            };
        let remaining = (actor.ability_cooldown / duration).clamp(0.0, 1.0);
        visible(ui, self.shade, cooling);
        place(ui, self.shade, 5.0, 37.0, 76.0, 76.0 * remaining);
        place(
            ui,
            self.progress,
            5.0,
            109.0,
            76.0 * if cooling { 1.0 - remaining } else { 1.0 },
            4.0,
        );
        ui.send(
            self.progress,
            WidgetMessage::Background(
                Brush::Solid(rgb(if cooling {
                    GOLD
                } else if ready {
                    CYAN
                } else {
                    MUTED
                }))
                .into(),
            ),
        );
        text(
            ui,
            self.countdown,
            if cooling {
                if actor.ability_cooldown < 1.0 {
                    format!("{:.1}", (actor.ability_cooldown * 10.0).ceil() / 10.0)
                } else {
                    format!("{:.0}", actor.ability_cooldown.ceil())
                }
            } else if blocked {
                "—".into()
            } else {
                String::new()
            },
        );
        text(
            ui,
            self.skill_name,
            if rt.korean_font {
                kit.skill_ko
            } else {
                kit.skill
            },
        );
        text(ui, self.key, "E");
        foreground(ui, self.key, if ready { WHITE } else { MUTED });
        let medical = id == 13 && actor.combat.hilde.active();
        let launcher = medical && actor.combat.hilde.phase != trickcal_shooting_balddagu::hilde_skill::Phase::Skill;
        text(ui, self.weapon_name, if actor.combat.komi.active() {if rt.korean_font {"맨손"}else{"EMPTY HANDS"}} else if launcher { if rt.korean_font {"의료용 유탄발사기"}else{"MEDICAL LAUNCHER"} }else{kit.weapon});
        let melee = kit.mode == AttackMode::Melee;
        let unlimited = kit.magazine == 0 || launcher;
        let marksman = id == 14 && actor.combat.shion.active();
        let rounds = if marksman { actor.combat.magic_ammo } else { actor.ammo };
        let capacity = if marksman { trickcal_shooting_balddagu::shion_skill::ROUNDS } else { kit.magazine };
        let low = !unlimited && rounds <= capacity / 4;
        text(
            ui,
            self.ammo,
            if unlimited {
                "∞".into()
            } else {
                rounds.to_string()
            },
        );
        text(
            ui,
            self.capacity,
            if unlimited {
                String::new()
            } else {
                format!("/ {}", capacity)
            },
        );
        foreground(ui, self.ammo, if low { GOLD } else { WHITE });
        let hint = if !actor.alive() {
            if rt.korean_font {
                "부활 대기".into()
            } else {
                "RESPAWNING".into()
            }
        } else if actor.combat.komi.active() {
            use trickcal_shooting_balddagu::komi_skill::Phase as Action;
            let s=actor.combat.komi;let left=(s.phase.duration()-s.elapsed).max(0.0);
            match (s.phase,rt.korean_font) {
                (Action::Rest,true)=>format!("무적 · 자가 회복 {:.1}초 · 채팅 외 키로 기상",left),
                (Action::Rest,false)=>format!("INVULNERABLE · HEAL {:.1}s · ANY NON-CHAT KEY TO WAKE",left),
                (Action::Feast,true)=>"사료 먹는 중 · 거대화 준비".into(),
                (Action::Feast,false)=>"EATING · PREPARING GIANT FORM".into(),
                (_,true)=>format!("거대화 {:.1}초 · SHIFT 착지 공격 · 기본공격 불가",left),
                (_,false)=>format!("GIANT {:.1}s · SHIFT SLAM · BASIC ATTACK LOCKED",left),
            }
        } else if medical {
            use trickcal_shooting_balddagu::hilde_skill::{self as h, Phase as Action};
            match (actor.combat.hilde.phase,rt.korean_font) {
                (Action::Skill,true)=>"구급상자 · 주변 아군 회복".into(),
                (Action::Skill,false)=>"MEDICAL CASE · AREA HEAL".into(),
                (Action::Equip,true)=>"유탄발사기 준비 중".into(),
                (Action::Equip,false)=>"EQUIPPING LAUNCHER".into(),
                (Action::Launcher,true)=>format!("좌클릭 연사 · {:.1}초",(h::EQUIP_SECONDS+h::LAUNCHER_SECONDS-actor.combat.hilde.elapsed).max(0.0)),
                (Action::Launcher,false)=>format!("HOLD LMB · {:.1}s",(h::EQUIP_SECONDS+h::LAUNCHER_SECONDS-actor.combat.hilde.elapsed).max(0.0)),
                (_,true)=>"주사기총으로 전환 중".into(),
                (_,false)=>"RESTORING SYRINGE GUN".into(),
            }
        } else if marksman {
            use trickcal_shooting_balddagu::shion_skill::Phase as Action;
            match (actor.combat.shion.phase, rt.korean_font) {
                (Action::Load, true) => "마탄 충전 중".into(),
                (Action::Load, false) => "CHARGING MAGIC ROUNDS".into(),
                (Action::Ready | Action::Fire, true) => "고정 사격 · 좌클릭 발사".into(),
                (Action::Ready | Action::Fire, false) => "ANCHORED · LMB FIRE".into(),
                (Action::End, true) => "사격 자세 해제 중".into(),
                (Action::End, false) => "LEAVING MARKSMAN STANCE".into(),
                (_, true) => "종말의 폭발".into(),
                (_, false) => "DOOMSDAY EXPLOSION".into(),
            }
        } else if melee {
            if rt.korean_font {
                "근접 무기 · 탄약 없음".into()
            } else {
                "MELEE · NO AMMO".into()
            }
        } else if unlimited {
            if rt.korean_font {
                "무한 탄약".into()
            } else {
                "UNLIMITED AMMO".into()
            }
        } else if actor.reload > 0.0 {
            if rt.korean_font {
                format!("재장전 중  {:.1}s", actor.reload)
            } else {
                format!("RELOADING  {:.1}s", actor.reload)
            }
        } else if rt.korean_font {
            "R  재장전".into()
        } else {
            "R  RELOAD".into()
        };
        text(ui, self.reload, hint);
        foreground(
            ui,
            self.reload,
            if actor.reload > 0.0 || low {
                GOLD
            } else {
                MUTED
            },
        );
        visible(ui, self.magic, id == 14);
        text(
            ui,
            self.magic,
            if rt.korean_font {
                format!("마탄  {} / 5", actor.combat.magic_ammo)
            } else {
                format!("MAGIC  {} / 5", actor.combat.magic_ammo)
            },
        );
    }
}
