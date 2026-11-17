//! Ghost race skill tile and a duration ring surrounding the aiming point.
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
use trickcal_shooting_balddagu::{phase_skill, roster::Race};

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
    key: Handle<UiNode>,
    ring_root: Handle<UiNode>,
    arc: Handle<UiNode>,
    seconds: Handle<UiNode>,
    last_remaining: Cell<Option<f32>>,
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

fn arc(remaining: f32) -> Vec<Primitive> {
    let fraction = (remaining / phase_skill::DURATION).clamp(0.0, 1.0);
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
        let key_bg = block(ctx, Color::from_rgba(20, 33, 49, 230));
        let key = centered(ctx, 14.0);
        let tile_root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(88.0)
                .with_height(164.0)
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_children([
                    outline, tile, icon, shade, progress, name, countdown, key_bg, key,
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
        let ring_root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(108.0)
                .with_height(128.0)
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_children([track, arc, seconds]),
        )
        .build(ctx)
        .to_base();
        for node in [track, arc] {
            place(ctx.inner(), node, 0.0, 0.0, 108.0, 108.0);
        }
        place(ctx.inner(), seconds, 0.0, 96.0, 108.0, 28.0);
        Self {
            tile_root,
            tile,
            icon,
            shade,
            progress,
            name,
            countdown,
            key,
            ring_root,
            arc,
            seconds,
            last_remaining: Cell::new(None),
        }
    }

    pub fn labels(&self) -> [Handle<UiNode>; 4] {
        [self.name, self.countdown, self.key, self.seconds]
    }

    pub fn render(&self, rt: &Runtime, ui: &mut UserInterface, show: bool) {
        let world = rt.session.world();
        let slot = rt.session.slot();
        let actor = match &rt.session {
            Session::Join(client) => client.motion.actor(world, slot, slot),
            _ => world.actors[slot].clone(),
        };
        let show = show && character(actor.character).race == Race::Ghost;
        visible(ui, self.tile_root, show);
        let ring_visible = show
            && actor.alive()
            && actor.phase_skill.active()
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
        let state = actor.phase_skill;
        let active = actor.alive() && state.active();
        let cooling = state.cooldown > 0.0;
        let ready = phase_skill::can_activate(&actor);
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
        let fraction = if active {
            state.remaining / phase_skill::DURATION
        } else if cooling {
            1.0 - state.cooldown / phase_skill::COOLDOWN
        } else {
            1.0
        };
        visible(ui, self.shade, cooling);
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
        text(
            ui,
            self.name,
            match (rt.korean_font, active) {
                (true, true) => "영체화 중",
                (true, false) => "영체화",
                (false, true) => "PHASING",
                (false, false) => "PHASE",
            },
        );
        let countdown = if active || cooling && state.cooldown < 1.0 {
            let seconds = if active {
                state.remaining
            } else {
                state.cooldown
            };
            format!("{:.1}", (seconds * 10.0).ceil() / 10.0)
        } else if cooling {
            format!("{:.0}", state.cooldown.ceil())
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
                size.x * 0.5 - CENTER.x * scale,
                size.y * 0.5 - CENTER.y * scale,
                108.0,
                128.0,
            );
            ui.send(
                self.ring_root,
                WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
            );
            if self.last_remaining.get() != Some(state.remaining) {
                let fill = ui[self.arc]
                    .cast_mut::<VectorImage>()
                    .expect("Phase duration arc");
                fill.primitives
                    .set_value_and_mark_modified(arc(state.remaining));
                fill.invalidate_visual();
                self.last_remaining.set(Some(state.remaining));
            }
            ui.send(
                self.arc,
                WidgetMessage::Foreground(
                    Brush::Solid(rgb(if state.remaining <= 1.0 { GOLD } else { CYAN })).into(),
                ),
            );
            text(
                ui,
                self.seconds,
                format!("{:.1}s", (state.remaining * 10.0).ceil() / 10.0),
            );
        }
    }
}
