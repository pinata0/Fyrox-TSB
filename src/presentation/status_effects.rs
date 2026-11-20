//! Negative effects above the crosshair; slows use world/screen effects instead.
use super::*;
use fyrox::gui::{formatted_text::WrapMode, HorizontalAlignment};
use trickcal_shooting_balddagu::naia_skill::Phase as NaiaPhase;

mod review;
pub(super) use review::{capture as review_capture, update as review_update};

#[derive(Debug, Default)]
pub(super) struct StatusEffectsUi {
    label: Handle<UiNode>,
    details: Handle<UiNode>,
}

type Effect = (&'static str, &'static str, [u8; 3]);

fn effects(actor: &Actor) -> Vec<Effect> {
    let c = &actor.combat;
    let mut effects = Vec::new();
    // Hard control stays prominent even when damage debuffs are also active.
    for (active, effect) in [
        (c.frozen > 0.0, ("동상", "FROZEN", [160, 225, 255])),
        (
            c.temporal >= 1.0,
            ("시간 정지", "TIME STOP", [169, 188, 255]),
        ),
        (c.stun > 0.0, ("기절", "STUNNED", PINK)),
        (c.sleep > 0.0, ("수면", "ASLEEP", [197, 176, 255])),
        (
            c.polymorph > 0.0,
            (
                "변이 · 이동만 가능",
                "FROG · MOVEMENT ONLY",
                [156, 235, 134],
            ),
        ),
        (
            c.frost > 0.0 && c.frozen <= 0.0,
            ("동상", "FROSTBITE", [160, 225, 255]),
        ),
        (c.poison > 0.0, ("중독 · 주는 피해 -20%", "POISONED · DAMAGE -20%", [166, 225, 114])),
        (
            c.weaken.max(c.skill_weaken).max(c.physical_weaken) > 0.0,
            ("약화", "WEAKENED", [197, 123, 255]),
        ),
        (c.taunt > 0.0, ("도발", "TAUNTED", [255, 183, 123])),
        (c.blind > 0.0, ("실명", "BLINDED", [190, 180, 220])),
    ] {
        if active {
            effects.push(effect);
        }
    }
    // Use the predicted channel phase so releasing the stream clears this immediately.
    if effects.is_empty()
        && actor.character == 1
        && c.naia.phase == NaiaPhase::Fire
        && !c.disabled()
    {
        effects.push(("정신집중", "CONCENTRATING", PINK));
    }
    effects
}

/// The old small status line only owns buffs, avoiding duplicate debuff/slow text.
pub(super) fn buff_text(actor: &Actor) -> &'static str {
    let c = &actor.combat;
    if c.fashion > 0.0 && c.sticker > 0.0 {
        "MAX HP x2 / STICKER -25% DAMAGE"
    } else if c.fashion > 0.0 {
        "MAX HP x2"
    } else if c.sticker > 0.0 {
        "STICKER -25% DAMAGE"
    } else if c.giant > 0.0 {
        "GIANT FORM"
    } else if c.empowered > 0.0 {
        "DOLL WILL"
    } else if c.ward > 0.0 {
        "HP FLOOR 10%"
    } else if c.haste > 0.0 {
        "ATTACK HASTE"
    } else {
        ""
    }
}

impl StatusEffectsUi {
    pub(super) fn new(ctx: &mut BuildContext) -> Self {
        // ONE Mobile POP supplies the heavy Korean letterforms; a dark outline
        // keeps them readable over both bright water and dark scenery.
        let label = TextBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_hit_test_visibility(false),
        )
        .with_font_size(36.0.into())
        .with_horizontal_text_alignment(HorizontalAlignment::Center)
        .with_wrap(WrapMode::NoWrap)
        .with_shadow(true)
        .with_shadow_brush(Brush::Solid(Color::from_rgba(15, 23, 34, 240)))
        .with_shadow_dilation(2.0)
        .with_shadow_offset(Vector2::new(0.0, 2.0))
        .build(ctx)
        .to_base();
        let details = TextBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_foreground(Brush::Solid(Color::opaque(231, 234, 255)).into()),
        )
        .with_font_size(22.0.into())
        .with_horizontal_text_alignment(HorizontalAlignment::Center)
        .with_wrap(WrapMode::Word)
        .with_shadow(true)
        .with_shadow_brush(Brush::Solid(Color::from_rgba(15, 23, 34, 240)))
        .with_shadow_dilation(1.5)
        .with_shadow_offset(Vector2::new(0.0, 2.0))
        .build(ctx)
        .to_base();
        Self { label, details }
    }

    pub(super) fn labels(&self) -> [Handle<UiNode>; 2] {
        [self.label, self.details]
    }

    pub(super) fn render(&self, rt: &Runtime, ui: &UserInterface, actor: &Actor, show: bool) {
        let effects = effects(actor);
        let show = show
            && !effects.is_empty()
            && actor.alive()
            && matches!(rt.session.world().phase, Phase::Setup | Phase::Playing)
            && !rt.home_open
            && !rt.lobby.open
            && !rt.startup.active()
            && !rt.settings_menu.open
            && !rt.character_help.open
            && !rt.keys.score
            && !rt.wheel.open
            && rt.error.is_none();
        visible(ui, self.label, show);
        visible(ui, self.details, show && effects.len() > 1);
        if !show {
            return;
        }
        let (korean, english, color) = effects[0];
        text(
            ui,
            self.label,
            if rt.korean_font { korean } else { english },
        );
        ui.send(
            self.label,
            WidgetMessage::Foreground(Brush::Solid(rgb(color)).into()),
        );
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
        text_size(ui, self.label, 36.0 * scale);
        place(
            ui,
            self.label,
            (size.x - 420.0 * scale) * 0.5,
            size.y * 0.5 - if effects.len() > 1 { 120.0 } else { 82.0 } * scale,
            420.0 * scale,
            56.0 * scale,
        );
        if effects.len() > 1 {
            text(
                ui,
                self.details,
                effects[1..]
                    .iter()
                    .map(|(ko, en, _)| if rt.korean_font { *ko } else { *en })
                    .collect::<Vec<_>>()
                    .join(" · "),
            );
            text_size(ui, self.details, 22.0 * scale);
            place(
                ui,
                self.details,
                (size.x - 720.0 * scale) * 0.5,
                size.y * 0.5 - 68.0 * scale,
                720.0 * scale,
                60.0 * scale,
            );
        }
    }
}
