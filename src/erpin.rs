//! Erpin's held cake action and staff shot, shared by authority and prediction.
use crate::combat::{AttackClass, ProjectileKind};
use crate::game::{direction, ray_box, Actor, Input, World, DT, V3};

pub const EAT_SECONDS: f32 = 50.0 / 30.0;
pub const BITE_TIMES: [f32; 4] = [11.0 / 30.0, 17.0 / 30.0, 23.0 / 30.0, 29.0 / 30.0];
pub const HEAL_PER_BITE: f32 = 8.0;
pub const CAMERA_DROP: f32 = 0.24;
pub const CAKE_SECONDS: f32 = 3.0;
/// Attack1_1 Event at frame 13 of 44, retimed to the game's attack interval.
pub fn shot_delay() -> f32 {
    crate::attack::seconds(2) * 13.0 / 44.0
}

#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub eating: bool,
    pub elapsed: f32,
    pub alt_down: bool,
    pub shot_delay: f32,
}
impl State {
    pub fn bites(self) -> usize {
        BITE_TIMES
            .iter()
            .filter(|&&t| self.elapsed + 0.00001 >= t)
            .count()
    }
    pub fn remaining(self) -> u8 {
        4 - self.bites() as u8
    }
    pub fn seated(self) -> f32 {
        if !self.eating {
            return 0.0;
        }
        smooth(self.elapsed / 0.18) * smooth((EAT_SECONDS - self.elapsed) / 0.22)
    }
}
pub fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A release or another action cancels immediately; WASD stays locked while eating.
pub fn interrupted(a: &Actor, input: Input) -> bool {
    !input.alt_fire
        || input.fire
        || input.jump
        || input.reload
        || input.ability
        || input.ultimate
        || input.phase_shift
        || input.dance
        || input.crouch
        || !a.alive()
        || a.character != 2
        || a.combat.skills_locked()
}

/// Return (new bites, dropped portions). Prediction discards both effects.
pub fn advance(a: &mut Actor, input: Input, grounded: bool) -> (usize, u8) {
    let pressed = input.alt_fire && !a.combat.erpin.alt_down;
    a.combat.erpin.alt_down = input.alt_fire;
    if a.alive() && a.character == 2 && a.combat.temporal >= 1.0
        && !a.combat.action_interrupted() {
        return (0, 0);
    }
    if a.combat.erpin.eating {
        if interrupted(a, input) {
            let remaining = a.combat.erpin.remaining();
            a.combat.erpin.eating = false;
            a.combat.erpin.elapsed = 0.0;
            return (0, remaining);
        }
        let before = a.combat.erpin.bites();
        a.combat.erpin.elapsed = (a.combat.erpin.elapsed
            + DT * crate::renua_skill::time_factor(a)).min(EAT_SECONDS);
        let bites = a.combat.erpin.bites() - before;
        if a.combat.erpin.elapsed + 0.00001 >= EAT_SECONDS {
            a.combat.erpin.eating = false;
            a.combat.erpin.elapsed = 0.0;
        }
        return (bites, 0);
    }
    if pressed
        && !interrupted(a, input)
        && grounded
        && a.vy <= 0.0
        && a.cooldown <= 0.0
        && a.reload <= 0.0
        && !a.combat.channeling()
    {
        a.combat.erpin.eating = true;
        a.combat.erpin.elapsed = 0.0;
        a.combat.erpin.shot_delay = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.portals.momentum = V3::default();
    }
    (0, 0)
}

/// A camera-relative left-hand position; one mouthward pulse per source bite.
pub fn hand_position(age: f32) -> V3 {
    let lift = smooth(age / 0.20) * smooth((EAT_SECONDS - age) / 0.25);
    let bite = BITE_TIMES
        .iter()
        .map(|t| (1.0 - ((age - t) / 0.11).abs()).max(0.0))
        .fold(0.0_f32, f32::max);
    V3::new(
        0.30 - 0.06 * lift - 0.025 * bite,
        -0.34 + 0.12 * lift + 0.035 * bite,
        0.58 - 0.08 * lift - 0.035 * bite,
    )
}

impl World {
    pub(crate) fn cancel_erpin(&mut self, i: usize) {
        self.actors[i].combat.erpin.shot_delay = 0.0;
        if self.actors[i].combat.erpin.eating {
            self.advance_erpin(i, Input::default());
        }
    }
    pub(crate) fn advance_erpin(&mut self, i: usize, input: Input) {
        let grounded = crate::movement::grounded(&self.map, self.actors[i].pos);
        let (bites, remaining) = advance(&mut self.actors[i], input, grounded);
        if bites > 0 {
            self.heal_from(i, i, bites as f32 * HEAL_PER_BITE);
        }
        if remaining > 0 {
            let a = &self.actors[i];
            let f = direction(a.yaw, 0.0);
            let left = V3::new(f.z, 0.0, -f.x);
            let origin = a.pos + V3::new(0.0, 0.95, 0.0) + f * 0.35 + left * 0.20;
            let kind = ProjectileKind::cake(remaining);
            if let Some(index) = self.projectile(i, kind, f, 1.1, 0.0, AttackClass::Basic) {
                let p = &mut self.projectiles[index];
                p.pos = origin;
                p.origin = origin;
                p.velocity = f * 1.1 + left * 0.3 + V3::new(0.0, 0.6, 0.0);
                p.life = CAKE_SECONDS;
            }
        }
        let a = &mut self.actors[i];
        if a.character != 2 || !a.alive() || a.combat.action_interrupted() {
            a.combat.erpin.shot_delay = 0.0;
        }
        if a.combat.erpin.shot_delay > 0.0 {
            a.combat.erpin.shot_delay = (a.combat.erpin.shot_delay
                - DT * crate::renua_skill::time_factor(a)).max(0.0);
            if a.combat.erpin.shot_delay == 0.0 {
                let dir = direction(a.yaw, a.pitch);
                self.projectile(
                    i,
                    ProjectileKind::Bolt,
                    dir,
                    22.0,
                    crate::kits::kit(2).damage,
                    AttackClass::Basic,
                );
            }
        }
    }
}

/// Cosmetic debris never collides with actors or heals/damages them.
pub fn step_cake(p: &mut crate::combat::Projectile, map: &[crate::game::Block]) {
    if p.velocity.length() < 0.001 {
        return;
    }
    p.velocity.y -= 9.8 * DT;
    let delta = p.velocity * DT;
    let distance = delta.length();
    let dir = delta.normalized();
    let hit = map
        .iter()
        .filter_map(|b| {
            ray_box(
                p.pos,
                dir,
                b.min() - V3::new(0.06, 0.06, 0.06),
                b.max() + V3::new(0.06, 0.06, 0.06),
            )
            .map(|t| (t, b))
        })
        .filter(|(t, _)| *t <= distance)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((t, block)) = hit {
        p.pos = p.pos + dir * (t - 0.002).max(0.0);
        p.velocity = if dir.y < 0.0 && (p.pos.y - block.max().y - 0.06).abs() < 0.02 {
            V3::default()
        } else {
            V3::new(0.0, p.velocity.y.min(-0.1), 0.0)
        };
    } else {
        p.pos = p.pos + delta;
        if p.pos.y <= 0.065 {
            p.pos.y = 0.065;
            p.velocity = V3::default();
        }
    }
}
