//! Ghost-only shared Shift skill. Authority and prediction share expiry and ejection.
use crate::{
    game::{ray_box, Actor, Block, DT, V3},
    movement::{blocked_with_height, BODY_RADIUS},
    roster::{character, Race},
};

pub const DURATION: f32 = 3.0;
pub const COOLDOWN: f32 = 12.0;

#[derive(Debug, Clone, Copy, Default)]
pub struct State {
    pub remaining: f32,
    pub cooldown: f32,
    pub down: bool,
    /// Last clear position, also acknowledged to the predicting client.
    pub safe_position: V3,
}

impl State {
    pub fn active(&self) -> bool {
        self.remaining > 0.0
    }
}

pub fn can_activate(a: &Actor) -> bool {
    a.alive()
        && character(a.character).race == Race::Ghost
        && !a.phase_skill.active()
        && a.phase_skill.cooldown <= 0.0
        && !a.combat.disabled()
        && !a.combat.channeling()
}

/// Called exactly once before each living actor's movement step.
pub fn advance(map: &[Block], a: &mut Actor, down: bool) {
    let pressed = down && !a.phase_skill.down;
    a.phase_skill.down = down;
    if a.phase_skill.active() {
        a.phase_skill.remaining = (a.phase_skill.remaining - DT).max(0.0);
        if a.phase_skill.remaining < DT * 0.01 {
            a.phase_skill.remaining = 0.0;
            a.phase_skill.cooldown = COOLDOWN;
            eject(map, a);
        }
    } else {
        a.phase_skill.cooldown = (a.phase_skill.cooldown - DT).max(0.0);
        if a.phase_skill.cooldown < DT * 0.01 {
            a.phase_skill.cooldown = 0.0;
        }
    }
    if pressed && can_activate(a) && !blocked_with_height(map, a.pos, a.collision_height()) {
        a.phase_skill.remaining = DURATION;
        a.phase_skill.safe_position = a.pos;
        a.think = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
    }
}

pub fn remember_clear_position(map: &[Block], a: &mut Actor) {
    if a.phase_skill.active() && !blocked_with_height(map, a.pos, a.collision_height()) {
        a.phase_skill.safe_position = a.pos;
    }
}

/// Resolve the union of overlapping walls, not just the first contact. Candidate
/// coordinates are faces of the expanded collision boxes, including corners.
/// Never eject through a non-phaseable map boundary or change the foot height.
fn eject(map: &[Block], a: &mut Actor) {
    a.ghost_phase = 0.0;
    let height = a.collision_height();
    let p = a.pos;
    if !blocked_with_height(map, p, height) {
        return;
    }
    const GAP: f32 = 0.02;
    let walls: Vec<_> = map
        .iter()
        .filter(|b| p.y < b.max().y - 0.001 && p.y + height > b.min().y + 0.001)
        .collect();
    let mut xs = vec![p.x];
    let mut zs = vec![p.z];
    for b in &walls {
        xs.extend([b.min().x - BODY_RADIUS - GAP, b.max().x + BODY_RADIUS + GAP]);
        zs.extend([b.min().z - BODY_RADIUS - GAP, b.max().z + BODY_RADIUS + GAP]);
    }
    for values in [&mut xs, &mut zs] {
        values.sort_by(f32::total_cmp);
        values.dedup();
    }
    let mut best = None;
    let mut distance = f32::INFINITY;
    for x in xs {
        for &z in &zs {
            let candidate = V3::new(x, p.y, z);
            let delta = candidate - p;
            let length = delta.length();
            if length >= distance || blocked_with_height(map, candidate, height) {
                continue;
            }
            let crosses_boundary = walls.iter().filter(|b| !b.phaseable).any(|b| {
                ray_box(
                    p,
                    delta.normalized(),
                    b.min() - V3::new(BODY_RADIUS, height - 0.001, BODY_RADIUS),
                    b.max() + V3::new(BODY_RADIUS, -0.001, BODY_RADIUS),
                )
                .is_some_and(|hit| hit <= length)
            });
            if !crosses_boundary {
                distance = length;
                best = Some(candidate);
            }
        }
    }
    // A falling actor may be enclosed at its current height. Return to the last
    // known clear position in that case, instead of leaving it stuck in scenery.
    a.pos = best.unwrap_or(a.phase_skill.safe_position);
    a.vy = 0.0;
    a.think = 0.0;
}
