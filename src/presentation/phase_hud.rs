//! Shared race skill tile and a duration ring surrounding the aiming point.
use super::*;
use fyrox::{
    core::algebra::Matrix3,
    gui::{
        canvas::CanvasBuilder,
        vector_image::{Primitive, VectorImage, VectorImageBuilder},
        HorizontalAlignment,
    },
};
use std::{cell::Cell, f32::consts::TAU};
use trickcal_shooting_balddagu::{phase_skill, race_skill, roster::Race};

const WHITE: [u8; 3] = [244, 249, 255];
const CYAN: [u8; 3] = [111, 222, 255];
const MUTED: [u8; 3] = [153, 170, 187];
const GOLD: [u8; 3] = [255, 182, 87];
const CENTER: Vector2<f32> = Vector2::new(54.0, 54.0);

#[derive(Debug, Default)]
pub(super) struct PhaseHud {
    tile_root: Handle<UiNode>,
    tile: Handle<UiNode>,
    icon: Handle<UiNode>,
    shade: Handle<UiNode>,
    progress: Handle<UiNode>,
    name: Handle<UiNode>,
    countdown: Handle<UiNode>,
    recharge: Handle<UiNode>,
    key: Handle<UiNode>,
    ring_root: Handle<UiNode>,
    arc: Handle<UiNode>,
    seconds: Handle<UiNode>,
    hint: Handle<UiNode>,
    last_remaining: Cell<Option<f32>>,
    last_race: Cell<Option<Race>>,
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

fn centered(ctx: &mut BuildContext, size: f32) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(WHITE)).into()),
    )
    .with_font_size(size.into())
    .with_horizontal_text_alignment(HorizontalAlignment::Center)
    .with_shadow(true)
    .build(ctx)
    .to_base()
}

fn vector(ctx: &mut BuildContext, primitives: Vec<Primitive>, color: Color) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(color).into()),
    )
    .with_primitives(primitives)
    .build(ctx)
    .to_base()
}

fn arc(fraction: f32) -> Vec<Primitive> {
    let fraction = fraction.clamp(0.0, 1.0);
    let point =
        |t: f32, radius: f32| CENTER + Vector2::new((t * TAU).sin(), -(t * TAU).cos()) * radius;
    let mut primitives = Vec::new();
    for i in 0..(fraction * 128.0).ceil() as usize {
        let start = i as f32 / 128.0;
        let end = ((i + 1) as f32 / 128.0).min(fraction);
        let a = point(start, 34.0);
        let b = point(start, 38.0);
        let c = point(end, 38.0);
        let d = point(end, 34.0);
        primitives.extend([
            Primitive::Triangle { points: [a, b, c] },
            Primitive::Triangle { points: [a, c, d] },
        ]);
    }
    primitives
}

impl PhaseHud {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let outline = block(ctx, Color::from_rgba(19, 31, 46, 230));
        let tile = block(ctx, rgb(WHITE));
        let icon = vector(ctx, super::combat_icons::phase(), rgb(INK));
        let shade = block(ctx, Color::from_rgba(14, 24, 37, 140));
        let progress = block(ctx, rgb(CYAN));
        let name = centered(ctx, 12.0);
        let countdown = centered(ctx, 30.0);
        let recharge = centered(ctx, 11.0);
        let key_bg = block(ctx, Color::from_rgba(20, 33, 49, 230));
        let key = centered(ctx, 14.0);
        let tile_root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(88.0)
                .with_height(164.0)
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_children([
                    outline, tile, icon, shade, progress, name, countdown, recharge, key_bg, key,
                ]),
        )
        .build(ctx)
        .to_base();
        for (node, x, y, w, h) in [
            (outline, 2.0, 34.0, 82.0, 82.0),
            (tile, 5.0, 37.0, 76.0, 76.0),
            (icon, 11.0, 43.0, 64.0, 64.0),
            (shade, 5.0, 37.0, 76.0, 76.0),
            (progress, 5.0, 109.0, 76.0, 4.0),
            (name, 0.0, 0.0, 88.0, 33.0),
            (countdown, 5.0, 52.0, 76.0, 44.0),
            (recharge, 0.0, 147.0, 88.0, 17.0),
            (key_bg, 14.0, 121.0, 58.0, 24.0),
            (key, 14.0, 121.0, 58.0, 24.0),
        ] {
            place(ctx.inner(), node, x, y, w, h);
        }
        let track = vector(
            ctx,
            vec![Primitive::WireCircle {
                center: CENTER,
                radius: 36.0,
                thickness: 7.0,
                segments: 128,
            }],
            Color::from_rgba(14, 24, 37, 210),
        );
        let arc = vector(ctx, vec![], rgb(CYAN));
        let seconds = centered(ctx, 14.0);
        let hint = centered(ctx, 15.0);
        let ring_root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(320.0)
                .with_height(164.0)
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_children([track, arc, seconds, hint]),
        )
        .build(ctx)
        .to_base();
        for node in [track, arc] {
            place(ctx.inner(), node, 106.0, 0.0, 108.0, 108.0);
        }
        place(ctx.inner(), seconds, 106.0, 96.0, 108.0, 28.0);
        place(ctx.inner(), hint, 0.0, 126.0, 320.0, 32.0);
        Self {
            tile_root,
            tile,
            icon,
            shade,
            progress,
            name,
            countdown,
            recharge,
            key,
            ring_root,
            arc,
            seconds,
            hint,
            last_remaining: Cell::new(None),
            last_race: Cell::new(None),
        }
    }

    pub fn labels(&self) -> [Handle<UiNode>; 6] {
        [
            self.name,
            self.countdown,
            self.recharge,
            self.key,
            self.seconds,
            self.hint,
        ]
    }

    pub fn render(&self, rt: &Runtime, ui: &mut UserInterface, show: bool) {
        let world = rt.session.world();
        let slot = rt.session.slot();
        let actor = match &rt.session {
            Session::Join(client) => client.motion.actor(world, slot, slot),
            _ => world.actors[slot].clone(),
        };
        let race = character(actor.character).race;
        let surge = race_skill::has_surge(race);
        let show = show
            && (surge
                || matches!(
                    race,
                    Race::Ghost | Race::Beast | Race::Spirit | Race::Dragon
                ));
        let (remaining, cooldown, duration, cooldown_length, ready) = if race == Race::Ghost {
            (
                actor.phase_skill.remaining,
                actor.phase_skill.cooldown,
                phase_skill::DURATION,
                phase_skill::COOLDOWN,
                phase_skill::can_activate(&actor),
            )
        } else if surge {
            (
                actor.race_skill.remaining,
                actor
                    .race_skill
                    .surge_recharge
                    .max(actor.race_skill.cooldown),
                race_skill::SURGE_DURATION,
                race_skill::SURGE_RECHARGE.max(actor.race_skill.cooldown),
                race_skill::can_activate(&world.map, &actor),
            )
        } else {
            (
                actor.race_skill.remaining,
                actor.race_skill.cooldown,
                if race == Race::Beast {
                    race_skill::LEAP_DURATION
                } else if race == Race::Dragon {
                    race_skill::CHARGE_DURATION
                } else {
                    race_skill::BURROW_DURATION
                },
                if race == Race::Beast {
                    if actor.combat.komi.giant() { trickcal_shooting_balddagu::komi_skill::LEAP_COOLDOWN } else { race_skill::LEAP_COOLDOWN }
                } else if race == Race::Dragon {
                    race_skill::CHARGE_COOLDOWN
                } else {
                    race_skill::BURROW_COOLDOWN
                },
                race_skill::can_activate(&world.map, &actor),
            )
        };
        let active = actor.alive() && remaining > 0.0;
        visible(ui, self.tile_root, show);
        let ring_visible = show
            && actor.alive()
            && active
            && race != Race::Beast
            && !surge
            && !rt.home_open
            && !rt.lobby.open
            && !rt.startup.active()
            && !rt.orbit.active
            && !rt.wheel.open
            && !rt.settings_menu.open
            && !rt.keys.score
            && !rt.chat.open;
        visible(ui, self.ring_root, ring_visible);
        if !show {
            return;
        }
        if self.last_race.get() != Some(race) {
            let icon = ui[self.icon]
                .cast_mut::<VectorImage>()
                .expect("Race skill icon");
            icon.primitives.set_value_and_mark_modified(match race {
                Race::Fairy | Race::Witch => super::combat_icons::surge(),
                Race::Beast => super::combat_icons::leap(),
                Race::Spirit => super::combat_icons::burrow(),
                Race::Dragon => super::combat_icons::charge(),
                _ => super::combat_icons::phase(),
            });
            icon.invalidate_visual();
            self.last_race.set(Some(race));
        }
        // Keep the charge count legible instead of drawing it over the arrows.
        ui.send(
            self.icon,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(if surge { 0.45 } else { 1.0 })),
        );
        place(
            ui,
            self.icon,
            if surge { 29.0 } else { 11.0 },
            if surge { 41.0 } else { 43.0 },
            64.0,
            64.0,
        );
        place(
            ui,
            self.countdown,
            5.0,
            if surge { 69.0 } else { 52.0 },
            76.0,
            if surge { 38.0 } else { 44.0 },
        );
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
        place(
            ui,
            self.tile_root,
            size.x - 466.0 * scale,
            size.y - 216.0 * scale,
            88.0,
            164.0,
        );
        ui.send(
            self.tile_root,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        let cooling = cooldown > 0.0;
        let color = if active || ready {
            CYAN
        } else if cooling {
            GOLD
        } else {
            MUTED
        };
        ui.send(
            self.tile,
            WidgetMessage::Background(
                Brush::Solid(rgb(if active {
                    [21, 104, 124]
                } else if ready {
                    WHITE
                } else {
                    [64, 80, 98]
                }))
                .into(),
            ),
        );
        ui.send(
            self.icon,
            WidgetMessage::Foreground(Brush::Solid(rgb(if ready { INK } else { MUTED })).into()),
        );
        let fraction = if active && race != Race::Beast && !surge {
            remaining / duration
        } else if cooling {
            1.0 - cooldown / cooldown_length
        } else {
            1.0
        };
        visible(
            ui,
            self.shade,
            cooling && (!surge || actor.race_skill.surge_charges == 0),
        );
        place(
            ui,
            self.shade,
            5.0,
            37.0,
            76.0,
            76.0 * (1.0 - fraction).clamp(0.0, 1.0),
        );
        place(
            ui,
            self.progress,
            5.0,
            109.0,
            76.0 * fraction.clamp(0.0, 1.0),
            4.0,
        );
        ui.send(
            self.progress,
            WidgetMessage::Background(Brush::Solid(rgb(color)).into()),
        );
        ui.send(
            self.key,
            WidgetMessage::Foreground(Brush::Solid(rgb(color)).into()),
        );
        text(ui, self.key, "SHIFT");
        visible(ui, self.recharge, surge && cooling);
        if surge && cooling {
            text(
                ui,
                self.recharge,
                format!("+1  {:.1}s", (cooldown * 10.0).ceil() / 10.0),
            );
        }
        text(
            ui,
            self.name,
            match (race, rt.korean_font, active) {
                (Race::Fairy | Race::Witch, true, _) => "쾌속 전진",
                (Race::Fairy | Race::Witch, false, _) => "SURGE",
                (Race::Beast, true, true) => "도약 중",
                (Race::Beast, true, false) => "도약",
                (Race::Beast, false, _) => "LEAP",
                (Race::Spirit, true, true) => "잠복 · 무적",
                (Race::Spirit, true, false) => "잠복",
                (Race::Spirit, false, true) => "BURROWED",
                (Race::Spirit, false, false) => "BURROW",
                (Race::Dragon, true, true) => "돌진 중",
                (Race::Dragon, true, false) => "몸통박치기",
                (Race::Dragon, false, _) => "CHARGE",
                (_, true, true) => "영체화 중",
                (_, true, false) => "영체화",
                (_, false, true) => "PHASING",
                (_, false, false) => "PHASE",
            },
        );
        let show_duration = active && race != Race::Beast && !surge;
        let countdown = if surge {
            format!(
                "{}/{}",
                actor.race_skill.surge_charges,
                race_skill::SURGE_CHARGES
            )
        } else if show_duration || cooling && cooldown < 1.0 {
            let seconds = if show_duration { remaining } else { cooldown };
            format!("{:.1}", (seconds * 10.0).ceil() / 10.0)
        } else if cooling {
            format!("{:.0}", cooldown.ceil())
        } else if !ready {
            "—".into()
        } else {
            String::new()
        };
        text(ui, self.countdown, countdown);
        if ring_visible {
            place(
                ui,
                self.ring_root,
                size.x * 0.5 - 160.0 * scale,
                size.y * 0.5 - CENTER.y * scale,
                320.0,
                164.0,
            );
            ui.send(
                self.ring_root,
                WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
            );
            let fraction = remaining / duration;
            if self.last_remaining.get() != Some(fraction) {
                let fill = ui[self.arc]
                    .cast_mut::<VectorImage>()
                    .expect("Phase duration arc");
                fill.primitives.set_value_and_mark_modified(arc(fraction));
                fill.invalidate_visual();
                self.last_remaining.set(Some(fraction));
            }
            ui.send(
                self.arc,
                WidgetMessage::Foreground(
                    Brush::Solid(rgb(if remaining <= 1.0 { GOLD } else { CYAN })).into(),
                ),
            );
            text(
                ui,
                self.seconds,
                format!("{:.1}s", (remaining * 10.0).ceil() / 10.0),
            );
            visible(ui, self.hint, matches!(race, Race::Spirit | Race::Dragon));
            if race == Race::Dragon {
                text(
                    ui,
                    self.hint,
                    if rt.korean_font {
                        "강제 전진 · 충돌 시 피해"
                    } else {
                        "AUTO FORWARD · CONTACT DAMAGE"
                    },
                );
            }
            if race == Race::Spirit {
                let charge = (actor.race_skill.charge / race_skill::EMERGE_HOLD * 100.0).round();
                text(
                    ui,
                    self.hint,
                    if rt.korean_font {
                        format!("좌클릭 길게: 나오기  {charge:.0}%")
                    } else {
                        format!("HOLD LMB TO EMERGE  {charge:.0}%")
                    },
                );
            }
        }
    }
}
