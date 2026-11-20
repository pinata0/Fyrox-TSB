use super::*;
use fyrox::gui::canvas::CanvasBuilder;
#[derive(Debug, Default)]
pub(super) struct Hud {
    root: Handle<UiNode>,
    bar: Handle<UiNode>,
    label: Handle<UiNode>,
}
impl Hud {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let background = BorderBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_background(Brush::Solid(Color::from_rgba(25, 23, 40, 210)).into()),
        )
        .with_corner_radius(4.0.into())
        .build(ctx)
        .to_base();
        let bar = BorderBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_background(Brush::Solid(rgb([255, 173, 205])).into()),
        )
        .with_corner_radius(3.0.into())
        .build(ctx)
        .to_base();
        let label = TextBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_foreground(Brush::Solid(rgb([255, 244, 250])).into()),
        )
        .with_font_size(14.0.into())
        .with_horizontal_text_alignment(fyrox::gui::HorizontalAlignment::Center)
        .with_shadow(true)
        .build(ctx)
        .to_base();
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_visibility(false)
                .with_children([background, bar, label]),
        )
        .build(ctx)
        .to_base();
        place(ctx.inner(), background, 0.0, 25.0, 260.0, 13.0);
        place(ctx.inner(), bar, 3.0, 28.0, 254.0, 7.0);
        place(ctx.inner(), label, 0.0, 0.0, 260.0, 23.0);
        Self { root, bar, label }
    }
    pub fn labels(&self) -> [Handle<UiNode>; 1] {
        [self.label]
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface, show: bool) {
        let a = &rt.session.world().actors[rt.session.slot()];
        let s = a.combat.chloe;
        visible(
            ui,
            self.root,
            show && a.alive()
                && a.character == 6
                && s.active()
                && s.phase != trickcal_shooting_balddagu::chloe_skill::Phase::Attack,
        );
        let size = ui.screen_size();
        place(
            ui,
            self.root,
            size.x * 0.5 - 130.0,
            size.y * 0.66,
            260.0,
            40.0,
        );
        let f =
            (s.remaining / trickcal_shooting_balddagu::chloe_skill::RIDE_SECONDS).clamp(0.0, 1.0);
        place(ui, self.bar, 3.0, 28.0, 254.0 * f, 7.0);
        use trickcal_shooting_balddagu::chloe_skill::Phase as P;
        text(
            ui,
            self.label,
            match (s.phase, rt.korean_font) {
                (P::Mount, true) => "세바스티안 탑승 중".into(),
                (P::Dismount, true) => "탑승 종료 · 내려오는 중".into(),
                (P::Roar, true) => format!("울부짖기 · {:0.1}초", s.remaining),
                (_, true) => format!("세바스티안  {:0.1}초   명중 {} / 9", s.remaining, s.hits),
                (P::Mount, false) => "MOUNTING SEBASTIAN".into(),
                (P::Dismount, false) => "DISMOUNTING".into(),
                (_, false) => format!("SEBASTIAN  {:0.1}s   HITS {} / 9", s.remaining, s.hits),
            },
        );
    }
}
