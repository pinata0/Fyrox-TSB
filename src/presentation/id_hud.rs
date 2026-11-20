//! Brief monochrome engineering overlays; the aiming area remains unobstructed.
use super::*;
use fyrox::gui::{
    canvas::CanvasBuilder,
    vector_image::{Primitive, VectorImage, VectorImageBuilder},
};
use trickcal_shooting_balddagu::id_skill::{self as id, Phase as IdPhase};

#[derive(Debug, Default)]
pub(super) struct Hud {
    pub root: Handle<UiNode>,
    lines: Handle<UiNode>,
    title: Handle<UiNode>,
    sub: Handle<UiNode>,
    code: Handle<UiNode>,
    warning: Handle<UiNode>,
    cooldown: Handle<UiNode>,
}
impl Hud {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let lines = VectorImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
            .with_primitives(vec![])
            .build(ctx)
            .to_base();
        let title = label(ctx, "", 32.0, [255, 222, 32]);
        let sub = label(ctx, "", 12.0, [255, 222, 32]);
        let code = label(ctx, "", 12.0, [255, 222, 32]);
        let warning = label(ctx, "", 13.0, [255, 222, 32]);
        let cooldown = label(ctx, "", 14.0, [245, 245, 245]);
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_children([lines, title, sub, code, warning]),
        )
        .build(ctx)
        .to_base();
        Self {
            root,
            lines,
            title,
            sub,
            code,
            warning,
            cooldown,
        }
    }
    pub fn render(&self, rt: &Runtime, ui: &mut UserInterface, actor: &Actor, show: bool) {
        let s = actor.combat.id;
        let allowed = show
            && actor.character == 3
            && actor.alive()
            && !actor.burrowed()
            && !rt.home_open
            && !rt.lobby.open
            && !rt.startup.active()
            && !rt.settings_menu.open
            && !rt.orbit.active
            && !rt.keys.score
            && !rt.wheel.open
            && !rt.chat.open;
        visible(ui, self.cooldown, allowed);
        let screen = ui.screen_size();
        let scale = (screen.x / 1280.0).min(screen.y / 800.0).clamp(0.6, 1.4);
        if allowed {
            text(
                ui,
                self.cooldown,
                &if s.pulse_cooldown > 0.0 {
                    format!("RMB  {:0.1}s", s.pulse_cooldown)
                } else {
                    "RMB  PULSE / READY".into()
                },
            );
            place(
                ui,
                self.cooldown,
                screen.x * 0.5 + 70.0 * scale,
                screen.y - 48.0 * scale,
                180.0 * scale,
                24.0 * scale,
            );
            text_size(ui, self.cooldown, 13.0 * scale);
        }
        let duration = match s.phase {
            IdPhase::Skill => 0.78,
            IdPhase::Pulse => 0.50,
            _ => 0.0,
        };
        let active = allowed && duration > 0.0 && s.elapsed < duration;
        visible(ui, self.root, active);
        if !active {
            return;
        }
        let white = s.phase == IdPhase::Pulse;
        let fade =
            id::smooth(s.elapsed / 0.06) * (1.0 - id::smooth((s.elapsed - duration + 0.22) / 0.22));
        let flicker = if (s.elapsed * 45.0) as u32 % 7 == 2 {
            0.52
        } else {
            1.0
        };
        let color = if white {
            Color::from_rgba(255, 255, 255, (215.0 * fade * flicker) as u8)
        } else {
            Color::from_rgba(255, 220, 20, (215.0 * fade * flicker) as u8)
        };
        for n in [self.lines, self.title, self.sub, self.code, self.warning] {
            ui.send(n, WidgetMessage::Foreground(Brush::Solid(color).into()));
        }
        let slide = (1.0 - id::smooth(s.elapsed / 0.14)) * 90.0
            - id::smooth((s.elapsed - duration + 0.18) / 0.18) * 70.0;
        let glitch = if (s.elapsed * 60.0) as u32 % 9 < 2 {
            11.0
        } else {
            0.0
        };
        let width = 920.0 * scale;
        let height = 430.0 * scale;
        place(
            ui,
            self.root,
            (screen.x - width) * 0.5 + (slide + glitch) * scale,
            screen.y * 0.21,
            width,
            height,
        );
        place(ui, self.lines, 0.0, 0.0, width, height);
        text(
            ui,
            self.title,
            if white {
                "OMNI / DISCHARGE"
            } else {
                "BOUNDARY // LOCK"
            },
        );
        text(
            ui,
            self.sub,
            if white {
                "NATTAH : RADIAL EMISSION   /   R-03"
            } else {
                "LLEL / PROTECTION PROTOCOL    03.00 SEC"
            },
        );
        text(
            ui,
            self.code,
            if white {
                "0xA3  > ALL AZIMUTHS\nPULSE.OUT [360]\nRECOIL = 0x01\nDISCHARGE : CONFIRMED"
            } else {
                "0xE1 > BARRIER.SET\nSELF.DAMAGE = 0\nALLY.HP >= 0.10\nSYNC // COMPLETE"
            },
        );
        text(
            ui,
            self.warning,
            if white {
                "/// CAUTION   ENERGY RELEASE   ///"
            } else {
                "/// WARNING   BOUNDARY SEALED   ///"
            },
        );
        for (n, x, y, w, h, size) in [
            (self.title, 18.0, 34.0, 600.0, 46.0, 30.0),
            (self.sub, 20.0, 86.0, 590.0, 24.0, 12.0),
            (self.code, 695.0, 245.0, 225.0, 90.0, 12.0),
            (self.warning, 36.0, 374.0, 530.0, 28.0, 14.0),
        ] {
            place(ui, n, x * scale, y * scale, w * scale, h * scale);
            text_size(ui, n, size * scale);
        }
        let mut primitives = Vec::new();
        let mut line = |x: f32, y: f32, x2: f32, y2: f32, thickness: f32| {
            primitives.push(Primitive::Line {
                begin: Vector2::new(x * scale, y * scale),
                end: Vector2::new(x2 * scale, y2 * scale),
                thickness: thickness * scale,
            })
        };
        // Open, sharply cut brackets leave the center 45% of the view clear.
        for (x, y, x2, y2, w) in [
            (0., 110., 0., 32., 2.),
            (0., 32., 32., 0., 2.),
            (32., 0., 565., 0., 2.),
            (565., 0., 590., 25., 2.),
            (920., 216., 920., 356., 2.),
            (920., 356., 892., 384., 2.),
            (892., 384., 666., 384., 2.),
            (0., 360., 30., 412., 2.),
            (30., 412., 550., 412., 2.),
            (640., 140., 690., 90., 1.),
            (690., 90., 880., 90., 1.),
            (34., 116., 208., 116., 1.),
            (770., 232., 910., 232., 1.),
        ] {
            line(x, y, x2, y2, w);
        }
        for i in 0..12 {
            let x = 35.0 + i as f32 * 16.0;
            line(x, 351.0, x + 12.0, 334.0, 4.0);
        }
        for i in 0..22 {
            let x = 710.0 + i as f32 * 8.5;
            line(
                x,
                347.0,
                x,
                355.0 + (i % 3) as f32 * 7.0,
                if i % 3 == 0 { 3.0 } else { 1.0 },
            );
        }
        // Deterministic sparse scan fragments, never a full-screen flash.
        for i in 0..5 {
            let k = ((s.elapsed * 40.0) as u32 * 37 + i * 71) % 211;
            line(
                30.0 + k as f32,
                126.0 + i as f32 * 4.0,
                110.0 + k as f32,
                126.0 + i as f32 * 4.0,
                1.0,
            );
        }
        let vector = ui[self.lines].cast_mut::<VectorImage>().unwrap();
        vector.primitives.set_value_and_mark_modified(primitives);
        vector.invalidate_visual();
    }
}
