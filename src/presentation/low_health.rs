//! Critical-health vignette beneath the HUD, using the local player's live health.
use super::*;
use fyrox::gui::{brush::GradientPoint, Thickness};

#[derive(Debug, Default)]
pub(super) struct LowHealthUi {
    pub(super) root: Handle<UiNode>,
}

impl LowHealthUi {
    pub(super) fn new(ui: &mut UserInterface) -> Self {
        let root = BorderBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_background(
                    Brush::RadialGradient {
                        // Normalized coordinates keep the vignette fitted to any aspect ratio.
                        center: Vector2::new(0.5, 0.5),
                        stops: [
                            (0.0, [80, 0, 10, 0]),
                            (0.28, [80, 0, 10, 0]),
                            (0.38, [80, 0, 10, 20]),
                            (0.50, [75, 0, 12, 135]),
                            (0.63, [40, 0, 7, 215]),
                            (1.0, [24, 0, 4, 230]),
                        ]
                        .into_iter()
                        .map(|(stop, [r, g, b, a])| GradientPoint {
                            stop,
                            color: Color::from_rgba(r, g, b, a),
                        })
                        .collect(),
                    }
                    .into(),
                ),
        )
        .with_stroke_thickness(Thickness::uniform(0.0).into())
        .build(&mut ui.build_ctx())
        .to_base();
        Self { root }
    }

    pub(super) fn render(&self, rt: &Runtime, ui: &UserInterface, show: bool) {
        let world = rt.session.world();
        let actor = &world.actors[rt.session.slot()];
        // Integer comparison keeps exactly 10% outside the critical range.
        let active = show
            && matches!(world.phase, Phase::Setup | Phase::Playing)
            && actor.alive()
            && u32::from(actor.hp) * 10 < u32::from(actor.max_hp())
            && !rt.home_open
            && !rt.lobby.open
            && !rt.startup.active()
            && !rt.settings_menu.open
            && rt.error.is_none();
        visible(ui, self.root, active);
        if active {
            let size = ui.screen_size();
            place(ui, self.root, 0.0, 0.0, size.x, size.y);
        }
    }
}
