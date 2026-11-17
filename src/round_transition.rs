//! Shared real-time result schedule. Only harmless motion continues after a result.
use crate::{
    game::{ray_box, Phase, World, DT, MATCH_RESULT_SECONDS},
    movement,
};

pub const ROUND_RESULT_SECONDS: f32 = 6.0;
pub const SLOW_MOTION_SECONDS: f32 = 0.9;
pub const SLOW_MOTION_SCALE: f32 = 0.16;
pub const REVEAL_SECONDS: f32 = 0.42;

impl World {
    pub fn in_round_transition(&self) -> bool {
        matches!(self.phase, Phase::Intermission | Phase::Finished)
    }

    /// Derived from the replicated phase countdown, including for late joiners.
    pub fn result_elapsed(&self) -> f32 {
        let duration = match self.phase {
            Phase::Intermission => ROUND_RESULT_SECONDS,
            Phase::Finished => MATCH_RESULT_SECONDS,
            _ => return 0.0,
        };
        (duration - self.phase_time).clamp(0.0, duration)
    }

    pub fn presentation_time_scale(&self) -> f32 {
        if !self.in_round_transition() {
            1.0
        } else if self.result_elapsed() < SLOW_MOTION_SECONDS {
            SLOW_MOTION_SCALE
        } else {
            0.0
        }
    }

    pub(crate) fn advance_round_aftermath(&mut self) {
        let dt = DT.min((SLOW_MOTION_SECONDS - self.result_elapsed()).max(0.0)) * SLOW_MOTION_SCALE;
        if dt == 0.0 {
            return;
        }
        for (actor, velocity) in self.actors.iter_mut().zip(self.result_velocity) {
            if !actor.active {
                continue;
            }
            if actor.alive() {
                let stepping = movement::grounded(&self.map, actor.pos);
                let height = actor.collision_height();
                actor.pos = movement::walk_with_height(
                    &self.map,
                    actor.pos,
                    velocity * dt,
                    stepping,
                    actor.phase_skill.active(),
                    height,
                )
                .0;
                actor.vy -= 18.0 * dt;
                movement::vertical_with_height(
                    &self.map,
                    &mut actor.pos,
                    &mut actor.vy,
                    dt,
                    height,
                );
            }
            for timer in [
                &mut actor.hit,
                &mut actor.hurt,
                &mut actor.ability_flash,
                &mut actor.combat.ultimate_flash,
                &mut actor.ghost_phase,
            ] {
                *timer = (*timer - dt).max(0.0);
            }
        }
        // No combat update: projectiles cannot deal late damage or change the winner.
        self.projectiles.retain_mut(|projectile| {
            let delta = projectile.velocity * dt;
            let distance = delta.length();
            if distance > 0.00001
                && self.map.iter().any(|block| {
                    ray_box(projectile.pos, delta.normalized(), block.min(), block.max())
                        .is_some_and(|hit| hit <= distance)
                })
            {
                return false;
            }
            projectile.pos = projectile.pos + delta;
            projectile.life -= dt;
            projectile.life > 0.0
        });
    }
}
