//! Screen-space Shion scope, kept behind the combat HUD and tied to camera zoom.
use super::*;
use fyrox::gui::{
    canvas::CanvasBuilder,
    vector_image::{Primitive, VectorImage, VectorImageBuilder},
};
use std::cell::Cell;

#[derive(Debug, Default)]
pub(super) struct ScopeUi {
    pub(super) root: Handle<UiNode>,
    pub(super) blur: scope_blur::ScopeEffect,
    border: Handle<UiNode>,
    rim: Handle<UiNode>,
    reticle_outline: Handle<UiNode>,
    reticle: Handle<UiNode>,
    dot: Handle<UiNode>,
    hit: Handle<UiNode>,
    last_size: Cell<Option<Vector2<f32>>>,
}

fn vector(ctx: &mut BuildContext, color: Color) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(color).into()),
    )
    .build(ctx)
    .to_base()
}

pub(super) fn lens_geometry(size: Vector2<f32>) -> (f32, f32) {
    let short_side = size.x.min(size.y);
    (short_side * 0.43, (short_side / 800.0).clamp(0.75, 2.0))
}

fn reticle(center: Vector2<f32>, radius: f32, scale: f32, thickness: f32) -> Vec<Primitive> {
    let mut lines = Vec::new();
    for axis in [Vector2::<f32>::x(), Vector2::<f32>::y()] {
        let normal = Vector2::new(-axis.y, axis.x);
        for sign in [-1.0, 1.0] {
            lines.push(Primitive::Line {
                begin: center + axis * (sign * 9.0 * scale),
                end: center + axis * (sign * radius * 0.93),
                thickness,
            });
            for tick in 1..=5 {
                let position = center + axis * (sign * radius * tick as f32 * 0.14);
                let half = if tick % 2 == 0 { 6.0 } else { 3.5 } * scale;
                lines.push(Primitive::Line {
                    begin: position - normal * half,
                    end: position + normal * half,
                    thickness,
                });
            }
        }
    }
    lines
}

impl ScopeUi {
    pub(super) fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        let border = vector(ctx, Color::from_rgba(12, 12, 22, 225));
        let rim = vector(ctx, Color::from_rgba(173, 158, 222, 170));
        let reticle_outline = vector(ctx, Color::from_rgba(240, 235, 255, 150));
        let reticle = vector(ctx, Color::opaque(16, 13, 27));
        let dot = vector(ctx, Color::opaque(203, 175, 255));
        let hit = vector(ctx, rgb(PINK));
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_children(vec![border, rim, reticle_outline, reticle, dot, hit]),
        )
        .build(ctx)
        .to_base();
        Self {
            root,
            blur: Default::default(),
            border,
            rim,
            reticle_outline,
            reticle,
            dot,
            hit,
            last_size: Cell::new(None),
        }
    }

    pub(super) fn render(&self, rt: &Runtime, ui: &mut UserInterface, show: bool) {
        visible(ui, self.root, show);
        self.blur.set_view(rt.scene, rt.camera, show);
        if !show {
            return;
        }
        let size = ui.screen_size();
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }
        if self.last_size.get() != Some(size) {
            let center = size * 0.5;
            let (radius, scale) = lens_geometry(size);
            let rim = [radius + 3.0 * scale, radius - 3.0 * scale]
                .into_iter()
                .map(|radius| Primitive::WireCircle {
                    center,
                    radius,
                    thickness: 1.0 * scale,
                    segments: 256,
                })
                .collect();
            let hit = [-1.0, 1.0]
                .into_iter()
                .flat_map(|x| {
                    [-1.0, 1.0].into_iter().map(move |y| {
                        let direction = Vector2::new(x, y);
                        Primitive::Line {
                            begin: center + direction * (6.0 * scale),
                            end: center + direction * (12.0 * scale),
                            thickness: 2.5 * scale,
                        }
                    })
                })
                .collect();
            place(ui, self.root, 0.0, 0.0, size.x, size.y);
            for (node, primitives) in [
                (
                    self.border,
                    vec![Primitive::WireCircle {
                        center,
                        radius,
                        thickness: 6.0 * scale,
                        segments: 256,
                    }],
                ),
                (self.rim, rim),
                (
                    self.reticle_outline,
                    reticle(center, radius, scale, 3.0 * scale),
                ),
                (self.reticle, reticle(center, radius, scale, 1.25 * scale)),
                (
                    self.dot,
                    vec![Primitive::Circle {
                        center,
                        radius: 2.0 * scale,
                        segments: 24,
                    }],
                ),
                (self.hit, hit),
            ] {
                place(ui, node, 0.0, 0.0, size.x, size.y);
                let image = ui[node].cast_mut::<VectorImage>().expect("Scope layer");
                image.primitives.set_value_and_mark_modified(primitives);
                image.invalidate_layout();
                image.invalidate_visual();
            }
            self.last_size.set(Some(size));
        }
        visible(
            ui,
            self.hit,
            rt.session.world().actors[rt.session.slot()].hit > 0.0,
        );
    }
}
