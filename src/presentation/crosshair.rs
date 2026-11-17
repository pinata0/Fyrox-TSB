//! Font-independent aiming reticle, centered on the current viewport.
use super::*;
use fyrox::gui::{
    canvas::CanvasBuilder,
    vector_image::{Primitive, VectorImageBuilder},
};

const SIZE: f32 = 24.0;

#[derive(Debug, Default)]
pub(super) struct Crosshair {
    pub(super) root: Handle<UiNode>,
    cross: Handle<UiNode>,
}

fn cross(ctx: &mut BuildContext, length: f32, thickness: f32, color: Color) -> Handle<UiNode> {
    let start = (SIZE - length) * 0.5;
    let center = SIZE * 0.5;
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_width(SIZE)
            .with_height(SIZE)
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(color).into()),
    )
    .with_primitives(vec![
        Primitive::Line {
            begin: Vector2::new(start, center),
            end: Vector2::new(start + length, center),
            thickness,
        },
        Primitive::Line {
            begin: Vector2::new(center, start),
            end: Vector2::new(center, start + length),
            thickness,
        },
    ])
    .build(ctx)
    .to_base()
}

impl Crosshair {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let outline = cross(ctx, SIZE, 6.0, rgb([20, 25, 32]));
        let cross = cross(ctx, 20.0, 2.0, rgb(CREAM));
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(SIZE)
                .with_height(SIZE)
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_children([outline, cross]),
        )
        .build(ctx)
        .to_base();
        Self { root, cross }
    }

    pub fn render(&self, ui: &UserInterface, show: bool, hit: bool) {
        visible(ui, self.root, show);
        if !show {
            return;
        }
        let center = ui.screen_size() * 0.5;
        place(
            ui,
            self.root,
            center.x - SIZE * 0.5,
            center.y - SIZE * 0.5,
            SIZE,
            SIZE,
        );
        // Keep the aiming point stable while retaining hit confirmation.
        ui.send(
            self.cross,
            WidgetMessage::Foreground(Brush::Solid(rgb(if hit { PINK } else { CREAM })).into()),
        );
    }
}
