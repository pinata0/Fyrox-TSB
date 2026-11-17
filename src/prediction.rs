//! Visual movement prediction. Authoritative World is never mutated by a client.
use crate::{
    game::{Actor, Input, Phase, World, DT, PLAYERS, V3},
    level::MapId,
    movement,
};
use std::collections::VecDeque;

const HISTORY: usize = 30; // At most half a second of unacknowledged movement.
const INTERPOLATION_TICKS: f64 = 6.0; // Two snapshots at 20 Hz.
const TELEPORT_DISTANCE: f32 = 4.0;

#[derive(Debug)]
struct Sample {
    tick: u64,
    actors: [Actor; PLAYERS],
}

#[derive(Debug, Default)]
pub struct ClientMotion {
    pending: VecDeque<(u64, Input)>,
    predicted: Option<Actor>,
    correction: V3,
    key: Option<(MapId, u16, Phase, usize)>,
    samples: VecDeque<Sample>,
    render_tick: f64,
    age: f32,
}

fn discontinuity(a: &Actor, b: &Actor) -> bool {
    a.character != b.character
        || a.active != b.active
        || a.alive() != b.alive()
        || a.deaths != b.deaths
        || a.portals.transits != b.portals.transits
        || (a.pos - b.pos).length() > TELEPORT_DISTANCE
}

fn predict_step(world: &World, slot: usize, a: &mut Actor, input: Input) {
    if world.phase != Phase::Playing || !a.alive() {
        return;
    }
    // Skill movement (dash/knockback) is supplied by authority. Do not invent
    // damage or rerun ability activation while replaying movement inputs.
    let mut pairs = world.actors.each_ref().map(|actor| actor.portals.ends);
    // Replace the local owner's pair when replaying unacknowledged placement.
    pairs[slot] = a.portals.ends;
    crate::portal::advance(&world.map, a, input, &pairs);
    pairs[slot] = a.portals.ends;
    if a.combat.dash > 0.0 {
        crate::phase_skill::advance(&world.map, a, input.phase_shift);
        return;
    }
    for timer in [
        &mut a.combat.slow,
        &mut a.combat.speed_up,
        &mut a.combat.stun,
        &mut a.combat.sleep,
    ] {
        *timer = (*timer - DT).max(0.0);
    }
    if input.forward.abs() > 0.01
        || input.strafe.abs() > 0.01
        || input.jump
        || input.phase_shift
        || input.crouch
        || input.fire
        || input.ability
        || input.ultimate
    {
        a.dancing = false;
    }
    if a.combat.disabled() {
        a.combat.naia = Default::default();
    } else {
        a.combat.naia.advance(input);
    }
    crate::erpin::advance(a, input, movement::grounded(&world.map, a.pos));
    crate::phase_skill::advance(&world.map, a, input.phase_shift);
    movement::step_actor_with_portals(&world.map, a, input, &pairs);
}

impl ClientMotion {
    pub fn observe(&mut self, world: &World, slot: usize, acknowledged: u64) {
        let key = (world.map_id, world.round, world.phase, slot);
        let reset = self.key != Some(key)
            || self.age > 0.5
            || self.samples.back().is_some_and(|previous| {
                world.tick < previous.tick
                    || discontinuity(&previous.actors[slot], &world.actors[slot])
            });
        let old_position = self.predicted.as_ref().map(|a| a.pos + self.correction);
        if reset {
            self.pending.clear();
            self.samples.clear();
            self.render_tick = world.tick as f64 - INTERPOLATION_TICKS;
        }
        self.key = Some(key);
        self.age = 0.0;
        self.pending.retain(|(seq, _)| *seq > acknowledged);
        let mut predicted = world.actors[slot].clone();
        for &(_, input) in &self.pending {
            predict_step(world, slot, &mut predicted, input);
        }
        let error = old_position.map_or(V3::default(), |p| p - predicted.pos);
        // Hide small reconciliation errors over several frames. A teleport,
        // respawn or large correction must take effect immediately.
        self.correction = if !reset && error.length() <= 0.75 {
            error
        } else {
            V3::default()
        };
        self.predicted = Some(predicted);
        if self.samples.back().is_none_or(|s| s.tick < world.tick) {
            self.samples.push_back(Sample {
                tick: world.tick,
                actors: world.actors.clone(),
            });
            while self.samples.len() > 16 {
                self.samples.pop_front();
            }
        }
        if let Some(latest) = self.samples.back() {
            let desired = latest.tick as f64 - INTERPOLATION_TICKS;
            if (desired - self.render_tick).abs() > 18.0 {
                self.render_tick = desired;
            }
        }
    }

    /// Called once per client fixed update, even if no packet arrived.
    pub fn advance(&mut self, world: &World, sequence: u64, input: Input) {
        self.age += DT;
        self.correction = self.correction * (-20.0 * DT).exp();
        if let Some(latest) = self.samples.back() {
            let desired = latest.tick as f64 - INTERPOLATION_TICKS;
            let rate = 1.0 + ((desired - self.render_tick) * 0.02).clamp(-0.1, 0.1);
            self.render_tick = (self.render_tick + rate).min(latest.tick as f64);
        }
        if self.age > 0.5 || self.pending.len() >= HISTORY {
            // Bound work and stop extrapolating on a stalled/disconnected link.
            return;
        }
        if let Some(a) = &mut self.predicted {
            if world.phase == Phase::Playing && a.alive() {
                let input = input.sanitized();
                self.pending.push_back((sequence, input));
                predict_step(world, self.key.unwrap().3, a, input);
            }
        }
    }

    pub fn actor(&self, world: &World, slot: usize, local_slot: usize) -> Actor {
        let mut actor = world.actors[slot].clone();
        if slot == local_slot {
            if let Some(predicted) = &self.predicted {
                let target = predicted.pos + self.correction;
                // Cosmetic error smoothing must not move a solid character
                // through a wall or below the authoritative supporting floor.
                actor.pos = if predicted.phase_skill.active()
                    || !movement::blocked_with_height(
                        &world.map,
                        target,
                        predicted.collision_height(),
                    ) && target.y >= movement::support(&world.map, target, predicted.pos.y)
                {
                    target
                } else {
                    predicted.pos
                };
                actor.yaw = predicted.yaw;
                actor.pitch = predicted.pitch;
                actor.vy = predicted.vy;
                actor.crouching = predicted.crouching;
                actor.combat.naia = predicted.combat.naia;
                actor.combat.erpin = predicted.combat.erpin;
                actor.phase_skill = predicted.phase_skill;
                actor.portals = predicted.portals;
            }
            return actor;
        }
        let Some(first) = self.samples.front() else {
            return actor;
        };
        let mut earlier = first;
        for later in &self.samples {
            if later.tick as f64 >= self.render_tick {
                let a = &earlier.actors[slot];
                let b = &later.actors[slot];
                if discontinuity(a, b) || discontinuity(b, &actor) {
                    return actor;
                }
                let t = if later.tick == earlier.tick {
                    0.0
                } else {
                    ((self.render_tick - earlier.tick as f64) / (later.tick - earlier.tick) as f64)
                        .clamp(0.0, 1.0) as f32
                };
                actor.pos = a.pos + (b.pos - a.pos) * t;
                let yaw = (b.yaw - a.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                actor.yaw = a.yaw + yaw * t;
                actor.pitch = a.pitch + (b.pitch - a.pitch) * t;
                return actor;
            }
            earlier = later;
        }
        actor
    }
}

#[cfg(test)]
mod tests;
