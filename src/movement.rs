//! Headless character collision, including floors, ceilings and walkable steps.
use crate::game::{direction, Actor, Block, Input, DT, V3};

/// One fixed movement step, shared by authority and client prediction. Combat,
/// damage, abilities, objectives and respawns remain exclusively authoritative.
pub fn step_actor(map: &[Block], a: &mut Actor, input: Input) {
    step_actor_with_portals(map, a, input, &[]);
}

/// Returns the owner of a traversed pair, if any.
pub fn step_actor_with_portals(
    map: &[Block],
    a: &mut Actor,
    input: Input,
    pairs: &[crate::portal::Pair],
) -> Option<usize> {
    step_actor_traced(map, a, input, pairs, &mut |_, _| {})
}

/// Reports only the route actually walked, including collision slides and steps.
/// Portal exits are isolated contacts, never a swept segment across the map.
pub(crate) fn step_actor_traced(
    map: &[Block],
    a: &mut Actor,
    input: Input,
    pairs: &[crate::portal::Pair],
    contact: &mut dyn FnMut(V3, V3),
) -> Option<usize> {
    if a.combat.erpin_skill.owns_movement() { return None; }
    if a.combat.view_locked() { return None; }
    a.view_aspect = input.sanitized().view_aspect;
    contact(a.pos, a.pos);
    let input = crate::portal::aimed_input(a, input);
    // Releasing Ctrl only stands up when the full body has headroom.
    if a.combat.polymorph > 0.0 {
        a.crouching = a.crouching && !can_stand(map, a);
    } else if !a.combat.shion.active() && !a.combat.komi.locked() {
        a.crouching = input.crouch || (a.crouching && !can_stand(map, a));
    }
    if a.crouching {
        a.dancing = false;
    }
    let height = a.collision_height();
    // Time stop also arrests jumps, portal momentum and race movement.
    if !a.dancing {
        a.yaw = input.yaw;
        a.pitch = input.pitch;
    }
    let f = direction(a.yaw, 0.0);
    let right = V3::new(-f.z, 0.0, f.x);
    let charging = crate::race_skill::charging(a) && !a.combat.disabled() && !a.combat.channeling();
    let forward = if charging { 1.0 } else { input.forward };
    let mut movement = f * forward + right * input.strafe;
    if movement.length() > 1.0 {
        movement = movement.normalized();
    }
    let mut speed = if charging {
        crate::race_skill::CHARGE_SPEED
    } else if a.crouching {
        CROUCH_SPEED
    } else {
        WALK_SPEED
    };
    if a.combat.slow > 0.0 {
        speed *= 0.65;
    } else if a.combat.rain_slow > 0.0 {
        speed *= crate::ui_skill::RAIN_SPEED;
    }
    if a.combat.speed_up > 0.0 {
        speed *= 1.2;
    }
    if a.combat.naia.phase == crate::naia_skill::Phase::Move {
        speed *= crate::naia_skill::SPEED_MULTIPLIER;
    }
    let water_lock = matches!(
        a.combat.naia.phase,
        crate::naia_skill::Phase::Fire | crate::naia_skill::Phase::Ultimate
    ) || a.combat.erpin.eating || a.combat.aya.active() || a.combat.shion.active() || a.combat.komi.locked() || matches!(a.combat.butter.phase, crate::butter_skill::Phase::Skill | crate::butter_skill::Phase::Ultimate);
    let water_lock = water_lock || a.combat.vivi.casting() || a.combat.chloe.locked();
    if water_lock {
        speed = 0.0;
    }
    if a.combat.movement_disabled() || a.combat.dash > 0.0 {
        speed = 0.0;
    }
    let on_ground = grounded(map, a.pos) && a.vy <= 0.0;
    a.portals.momentum = a.portals.momentum * if on_ground { 0.72 } else { 0.99 };
    let mut velocity = movement * speed;
    let surging = a.race_skill.active()
        && crate::race_skill::has_surge(crate::roster::character(a.character).race);
    if a.race_skill.active()
        && !crate::race_skill::charging(a)
        && !a.burrowed()
        && !a.combat.disabled()
        && !water_lock
    {
        velocity = a.race_skill.velocity + velocity * if surging { 0.0 } else { 0.15 };
    }
    if !a.combat.disabled()
        && !surging
        && !charging
        && a.combat.dash <= 0.0
        && !water_lock
        && a.portals.momentum.length() > speed
    {
        velocity = a.portals.momentum + velocity * 0.15;
    }
    if input.jump && on_ground && !a.combat.disabled() && !water_lock && !a.burrowed() {
        a.vy = 6.0;
    }
    if a.combat.vivi.phase == crate::vivi_skill::Phase::Ultimate && !a.combat.action_interrupted() {
        let t=a.combat.vivi.elapsed;
        let action_dt=DT*crate::renua_skill::time_factor(a);
        a.vy=(crate::vivi_skill::ultimate_lift(t+action_dt)-crate::vivi_skill::ultimate_lift(t))/action_dt.max(0.00001);
    } else if a.combat.butter.hovering() && !a.combat.action_interrupted() {
        let t = a.combat.butter.elapsed;
        let action_dt = DT * crate::renua_skill::time_factor(a);
        a.vy = (crate::butter_skill::lift(t + action_dt) - crate::butter_skill::lift(t)) / action_dt.max(0.00001);
    } else {
        a.vy -= 18.0 * DT * crate::renua_skill::time_factor(a);
    }
    let transit = if a.burrowed() {
        None
    } else {
        crate::portal::traverse(map, pairs, a, velocity + V3::new(0.0, a.vy, 0.0))
    };
    if let Some(owner) = transit {
        // The portal now owns the rotated momentum; do not reapply the old launch direction.
        crate::race_skill::finish_charge(a);
        a.race_skill.remaining = 0.0;
        a.race_skill.velocity = V3::default();
        contact(a.pos, a.pos);
        return Some(owner);
    }
    let time_factor = crate::renua_skill::time_factor(a);
    let delta = velocity * (DT * time_factor);
    let before = a.pos;
    if a.phase_skill.active() {
        let (position, touched_wall) =
            walk_traced(map, a.pos, delta, on_ground, true, height, contact);
        a.pos = position;
        if touched_wall {
            a.ghost_phase = PHASE_LINGER;
        }
    } else {
        a.pos = walk_traced(map, a.pos, delta, on_ground, false, height, contact).0;
    }
    if surging && (a.pos - before).flat().dot(delta.normalized()) + 0.001 < delta.length() {
        a.race_skill.cancel();
    }
    let before = a.pos;
    vertical_with_height(map, &mut a.pos, &mut a.vy, DT * time_factor, height);
    contact(before, a.pos);
    crate::race_skill::after_movement(map, a);
    crate::phase_skill::remember_clear_position(map, a);
    None
}

pub const BODY_HEIGHT: f32 = 1.8;
pub const WALK_SPEED: f32 = 3.48;
pub const CROUCH_SPEED: f32 = WALK_SPEED * 0.5;
pub const BODY_RADIUS: f32 = 0.4;
pub const STEP_HEIGHT: f32 = 0.32;
/// Long enough for a 20 Hz snapshot to carry a brief wall contact.
pub const PHASE_LINGER: f32 = 0.2;
const EPS: f32 = 0.001;

fn overlaps_xz(b: &Block, p: V3) -> bool {
    let lo = b.min();
    let hi = b.max();
    p.x > lo.x - BODY_RADIUS + EPS
        && p.x < hi.x + BODY_RADIUS - EPS
        && p.z > lo.z - BODY_RADIUS + EPS
        && p.z < hi.z + BODY_RADIUS - EPS
}

pub fn blocked(map: &[Block], p: V3) -> bool {
    blocked_with_height(map, p, BODY_HEIGHT)
}

pub fn blocked_with_height(map: &[Block], p: V3, height: f32) -> bool {
    map.iter().any(|b| overlaps_body(b, p, height))
}

fn can_stand(map: &[Block], a: &Actor) -> bool {
    let phasing = a.phase_skill.active();
    !map.iter().any(|b| {
        overlaps_body(b, a.pos, BODY_HEIGHT)
            // Phasing allows standing inside side walls, but not through an underside.
            && !(phasing && b.phaseable && b.min().y < a.pos.y + a.collision_height() - EPS)
    })
}

fn overlaps_body(b: &Block, p: V3, height: f32) -> bool {
    overlaps_xz(b, p) && p.y < b.max().y - EPS && p.y + height > b.min().y + EPS
}

/// Highest supporting surface below the specified foot height (ground plane included).
pub fn support(map: &[Block], p: V3, max_height: f32) -> f32 {
    map.iter()
        .filter(|b| overlaps_xz(b, p) && b.max().y <= max_height + EPS)
        .map(|b| b.max().y)
        .fold(0.0, f32::max)
}

pub fn grounded(map: &[Block], p: V3) -> bool {
    (p.y - support(map, p, p.y)).abs() <= EPS
}

pub fn walk(map: &[Block], p: V3, delta: V3, stepping: bool) -> V3 {
    walk_with_height(map, p, delta, stepping, false, BODY_HEIGHT).0
}

/// Normal steps first, then horizontal phasing. A blocked step stays solid even for ghosts.
pub fn phase_walk(map: &[Block], p: V3, delta: V3, stepping: bool) -> (V3, bool) {
    walk_with_height(map, p, delta, stepping, true, BODY_HEIGHT)
}

pub fn walk_with_height(
    map: &[Block],
    p: V3,
    delta: V3,
    stepping: bool,
    phase: bool,
    height: f32,
) -> (V3, bool) {
    walk_traced(map, p, delta, stepping, phase, height, &mut |_, _| {})
}

fn walk_traced(
    map: &[Block],
    mut p: V3,
    delta: V3,
    stepping: bool,
    phase: bool,
    height: f32,
    trace: &mut dyn FnMut(V3, V3),
) -> (V3, bool) {
    let mut contact = false;
    // Swept subdivision also protects callers such as dash/knockback from thin walls.
    let count = (delta.flat().length() / 0.15).ceil().max(1.0) as usize;
    let delta = delta * (1.0 / count as f32);
    for _ in 0..count {
        for axis in [V3::new(delta.x, 0.0, 0.0), V3::new(0.0, 0.0, delta.z)] {
            if axis.length() < EPS {
                continue;
            }
            let candidate = p + axis;
            if !blocked_with_height(map, candidate, height) {
                trace(p, candidate);
                p = candidate;
            } else if stepping && grounded(map, p) {
                let top = support(map, candidate, p.y + STEP_HEIGHT);
                let lifted = V3::new(candidate.x, top, candidate.z);
                let overhead = V3::new(p.x, top, p.z);
                if top > p.y + EPS {
                    if !blocked_with_height(map, lifted, height)
                        && !blocked_with_height(map, overhead, height)
                    {
                        trace(p, overhead);
                        trace(overhead, lifted);
                        p = lifted;
                    }
                    // A walkable rise is floor geometry. If there is no headroom,
                    // stop here instead of phasing into the tread and losing support.
                    continue;
                }
            }
            if phase
                && blocked_with_height(map, candidate, height)
                && !map
                    .iter()
                    .any(|b| !b.phaseable && overlaps_body(b, candidate, height))
            {
                trace(p, candidate);
                p = candidate;
                contact = true;
            }
        }
    }
    (p, contact)
}

pub fn vertical(map: &[Block], p: &mut V3, velocity: &mut f32, dt: f32) {
    vertical_with_height(map, p, velocity, dt, BODY_HEIGHT);
}

pub fn vertical_with_height(map: &[Block], p: &mut V3, velocity: &mut f32, dt: f32, height: f32) {
    let old = p.y;
    let mut next = old + *velocity * dt;
    if *velocity <= 0.0 {
        let floor = support(map, *p, old);
        if next <= floor {
            next = floor;
            *velocity = 0.0;
        }
    } else {
        for b in map.iter().filter(|b| overlaps_xz(b, *p)) {
            let ceiling = b.min().y - height;
            if ceiling >= old - EPS && next > ceiling {
                next = ceiling;
                *velocity = 0.0;
            }
        }
    }
    p.y = next;
}

/// Sample an actual supported walking route, allowing directed drops but no climbing walls.
pub fn walkable_line(map: &[Block], from: V3, to: V3) -> bool {
    let count = ((to - from).flat().length() / 0.18).ceil().max(1.0) as usize;
    let mut previous = from;
    for n in 1..=count {
        let mut next = from + (to - from) * (n as f32 / count as f32);
        next.y = support(map, next, previous.y + STEP_HEIGHT);
        if previous.y - next.y > 4.0
            || blocked(map, next)
            || blocked(map, V3::new(next.x, previous.y.max(next.y), next.z))
        {
            return false;
        }
        previous = next;
    }
    (previous.y - to.y).abs() < 0.05
}
