//! Bottom-center ultimate charge, sweeping clockwise from twelve o'clock.
use super::*;
use fyrox::{
    core::algebra::Matrix3,
    gui::{
        canvas::CanvasBuilder,
        formatted_text::WrapMode,
        vector_image::{Primitive, VectorImage, VectorImageBuilder},
        HorizontalAlignment,
    },
};
use std::{cell::Cell, f32::consts::TAU};

const WHITE: [u8; 3] = [244, 249, 255];
const GOLD: [u8; 3] = [255, 194, 77];
const CYAN: [u8; 3] = [94, 225, 255];
const MUTED: [u8; 3] = [153, 170, 187];
const WIDTH: f32 = 260.0;
const HEIGHT: f32 = 194.0;
const BOTTOM: f32 = 238.0;
const CENTER: Vector2<f32> = Vector2::new(WIDTH * 0.5, 66.0);
const RADIUS: f32 = 48.0;
const THICKNESS: f32 = 7.0;
const SEGMENTS: usize = 160;

#[derive(Debug, Default)]
pub(super) struct UltimateHud {
    root: Handle<UiNode>,
    disc: Handle<UiNode>,
    fill: Handle<UiNode>,
    tip: Handle<UiNode>,
    halo: Handle<UiNode>,
    bolt: Handle<UiNode>,
    amount: Handle<UiNode>,
    percent: Handle<UiNode>,
    key_bg: Handle<UiNode>,
    key: Handle<UiNode>,
    name: Handle<UiNode>,
    hint: Handle<UiNode>,
    last_charge: Cell<Option<f32>>,
}

fn point(fraction: f32, radius: f32) -> Vector2<f32> {
    // Screen Y increases downwards: top -> right -> bottom -> left.
    let angle = fraction * TAU;
    CENTER + Vector2::new(angle.sin(), -angle.cos()) * radius
}

fn charge_arc(charge: f32) -> Vec<Primitive> {
    let fraction = (charge / 100.0).clamp(0.0, 1.0);
    let count = (fraction * SEGMENTS as f32).ceil() as usize;
    let mut triangles = Vec::with_capacity(count * 2);
    for i in 0..count {
        let start = i as f32 / SEGMENTS as f32;
        let end = ((i + 1) as f32 / SEGMENTS as f32).min(fraction);
        let a = point(start, RADIUS - THICKNESS * 0.5);
        let b = point(start, RADIUS + THICKNESS * 0.5);
        let c = point(end, RADIUS + THICKNESS * 0.5);
        let d = point(end, RADIUS - THICKNESS * 0.5);
        triangles.push(Primitive::Triangle { points: [a, b, c] });
        triangles.push(Primitive::Triangle { points: [a, c, d] });
    }
    triangles
}

fn vector(ctx: &mut BuildContext, primitives: Vec<Primitive>, color: Color) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_width(WIDTH)
            .with_height(HEIGHT)
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(color).into()),
    )
    .with_primitives(primitives)
    .build(ctx)
    .to_base()
}

fn circle(radius: f32, thickness: f32) -> Vec<Primitive> {
    vec![Primitive::WireCircle {
        center: CENTER,
        radius,
        thickness,
        segments: SEGMENTS,
    }]
}

fn centered(ctx: &mut BuildContext, size: f32) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(WHITE)).into()),
    )
    .with_font_size(size.into())
    .with_horizontal_text_alignment(HorizontalAlignment::Center)
    .with_wrap(WrapMode::NoWrap)
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

impl UltimateHud {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        let halo = vector(ctx, circle(57.0, 8.0), Color::from_rgba(94, 225, 255, 65));
        let shadow = vector(ctx, circle(53.0, 5.0), Color::from_rgba(10, 20, 32, 220));
        let bezel = vector(ctx, circle(55.0, 1.5), Color::from_rgba(244, 249, 255, 185));
        let disc = vector(
            ctx,
            vec![Primitive::Circle {
                center: CENTER,
                radius: 43.0,
                segments: SEGMENTS,
            }],
            Color::from_rgba(18, 30, 45, 210),
        );
        let track = vector(
            ctx,
            circle(RADIUS, THICKNESS),
            Color::from_rgba(92, 108, 125, 180),
        );
        let fill = vector(ctx, vec![], rgb(GOLD));
        let inner = vector(ctx, circle(40.5, 1.0), Color::from_rgba(244, 249, 255, 65));
        let ticks = vector(
            ctx,
            (0..4)
                .map(|i| Primitive::Line {
                    begin: point(i as f32 / 4.0, 57.0),
                    end: point(i as f32 / 4.0, 61.0),
                    thickness: 2.0,
                })
                .collect(),
            rgb(WHITE),
        );
        let tip = vector(
            ctx,
            vec![Primitive::Circle {
                center: Vector2::new(3.0, 3.0),
                radius: 2.3,
                segments: 16,
            }],
            rgb(WHITE),
        );
        // An original lightning glyph makes the fully charged state distinct at a glance.
        let bolt = vector(
            ctx,
            vec![
                Primitive::Triangle {
                    points: [(134.0, 35.0), (115.0, 60.0), (130.0, 58.0)]
                        .map(|(x, y)| Vector2::new(x, y)),
                },
                Primitive::Triangle {
                    points: [(130.0, 53.0), (145.0, 51.0), (126.0, 78.0)]
                        .map(|(x, y)| Vector2::new(x, y)),
                },
            ],
            rgb(WHITE),
        );
        let amount = centered(ctx, 35.0);
        let percent = centered(ctx, 13.0);
        let key_bg = BorderBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_background(Brush::Solid(rgb(WHITE)).into()),
        )
        .with_corner_radius(3.0.into())
        .build(ctx)
        .to_base();
        let key = centered(ctx, 17.0);
        let name = centered(ctx, 14.0);
        let hint = centered(ctx, 12.0);
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(WIDTH)
                .with_height(HEIGHT)
                .with_hit_test_visibility(false)
                .with_children(vec![
                    halo, shadow, bezel, disc, track, fill, inner, ticks, tip, bolt, amount,
                    percent, key_bg, key, name, hint,
                ]),
        )
        .build(ctx)
        .to_base();
        for (node, x, y, w, h) in [
            (amount, 85.0, 42.0, 90.0, 47.0),
            (percent, 85.0, 83.0, 90.0, 21.0),
            (key_bg, 113.0, 126.0, 34.0, 24.0),
            (key, 113.0, 125.0, 34.0, 25.0),
            (name, 0.0, 153.0, WIDTH, 23.0),
            (hint, 0.0, 176.0, WIDTH, 18.0),
        ] {
            place(ctx.inner(), node, x, y, w, h);
        }
        Self {
            root,
            disc,
            fill,
            tip,
            halo,
            bolt,
            amount,
            percent,
            key_bg,
            key,
            name,
            hint,
            last_charge: Cell::new(None),
        }
    }

    pub fn labels(&self) -> [Handle<UiNode>; 5] {
        [self.amount, self.percent, self.key, self.name, self.hint]
    }

    pub fn render(&self, rt: &Runtime, ui: &mut UserInterface, show: bool) {
        visible(ui, self.root, show);
        if !show {
            return;
        }
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
        place(
            ui,
            self.root,
            (size.x - WIDTH * scale) * 0.5,
            size.y - BOTTOM * scale,
            WIDTH,
            HEIGHT,
        );
        ui.send(
            self.root,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        // Keep combat status above the ring, clear of its key and hero-specific name.
        place(
            ui,
            rt.hud.status,
            (size.x - 380.0 * scale) * 0.5,
            size.y - (BOTTOM + 30.0) * scale,
            380.0,
            28.0,
        );
        ui.send(
            rt.hud.status,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        ui.send(
            rt.hud.status,
            TextMessage::HorizontalAlignment(HorizontalAlignment::Center),
        );

        let actor = &rt.session.world().actors[rt.session.slot()];
        let charge = actor.combat.ultimate.clamp(0.0, 100.0);
        let charged = charge >= 100.0;
        let blocked = !actor.alive() || actor.combat.skills_locked() || actor.combat.komi.active();
        let ready = charged && !blocked;
        let active = actor.combat.ultimate_flash > 0.0 || matches!(actor.combat.komi.phase,trickcal_shooting_balddagu::komi_skill::Phase::Feast | trickcal_shooting_balddagu::komi_skill::Phase::Giant);
        let color = if blocked {
            MUTED
        } else if charged || active {
            CYAN
        } else {
            GOLD
        };
        // Only rebuild the single arc mesh when charge actually changes. Its last
        // segment ends at the exact fraction, including values just below 100%.
        if self.last_charge.get() != Some(charge) {
            let fill = ui[self.fill]
                .cast_mut::<VectorImage>()
                .expect("Ultimate arc");
            fill.primitives
                .set_value_and_mark_modified(charge_arc(charge));
            fill.invalidate_visual();
            self.last_charge.set(Some(charge));
        }
        foreground(ui, self.fill, color);
        let end = point(charge / 100.0, RADIUS);
        place(ui, self.tip, end.x - 3.0, end.y - 3.0, 6.0, 6.0);
        visible(ui, self.tip, charge > 0.0 && !charged && !blocked);
        visible(ui, self.halo, ready || (active && !blocked));
        let pulse = 0.55 + 0.25 * (rt.started.elapsed().as_secs_f32() * 3.0).sin();
        ui.send(self.halo, WidgetMessage::Opacity(Some(pulse)));
        ui.send(
            self.disc,
            WidgetMessage::Foreground(
                Brush::Solid(if ready {
                    Color::from_rgba(12, 78, 99, 225)
                } else {
                    Color::from_rgba(18, 30, 45, 210)
                })
                .into(),
            ),
        );
        visible(ui, self.bolt, ready);
        visible(ui, self.amount, !ready);
        text(ui, self.amount, format!("{:.0}", charge.floor()));
        foreground(ui, self.amount, if blocked { MUTED } else { WHITE });
        text(ui, self.percent, if ready { "100%" } else { "%" });
        foreground(ui, self.percent, if ready { CYAN } else { MUTED });
        ui.send(
            self.key_bg,
            WidgetMessage::Background(
                Brush::Solid(rgb(if ready {
                    CYAN
                } else if blocked {
                    MUTED
                } else {
                    WHITE
                }))
                .into(),
            ),
        );
        text(ui, self.key, "Q");
        foreground(ui, self.key, INK);
        let kit = kit(actor.character);
        text(
            ui,
            self.name,
            if rt.korean_font {
                kit.ultimate_ko
            } else {
                kit.ultimate
            },
        );
        foreground(ui, self.name, if blocked { MUTED } else { WHITE });
        let hint = if !actor.alive() {
            if rt.korean_font {
                "부활 대기"
            } else {
                "RESPAWNING"
            }
        } else if actor.combat.skills_locked() {
            if rt.korean_font {
                "사용 불가"
            } else {
                "DISABLED"
            }
        } else if active {
            if rt.korean_font {
                "궁극기 발동"
            } else {
                "ACTIVATED"
            }
        } else if ready {
            if rt.korean_font {
                "궁극기 준비 완료"
            } else {
                "ULTIMATE READY"
            }
        } else {
            if rt.korean_font {
                "충전 중"
            } else {
                "CHARGING"
            }
        };
        text(ui, self.hint, hint);
        foreground(ui, self.hint, color);
    }
}
