//! Elf portals. Placement, passage and view transforms are shared by host and prediction.
use crate::{
    game::{direction, ray_box, Actor, Block, Input, DT, V3},
    movement::{blocked_with_height, BODY_RADIUS},
    roster::{character, Race},
};

pub const COOLDOWN: f32 = 5.0;
pub const REOPEN_DELAY: f32 = 1.0;
pub const RANGE: f32 = 45.0;
pub const HALF_WIDTH: f32 = 0.95;
pub const HALF_HEIGHT: f32 = 1.45;
pub const EXIT_GAP: f32 = 0.06;
pub type Pair = [Option<Portal>; 2];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Portal {
    pub center: V3,
    /// Outward surface normal, encoded as an axis index on the wire.
    pub face: u8,
}

impl Portal {
    pub fn normal(self) -> V3 {
        [
            V3::new(1.0, 0.0, 0.0),
            V3::new(-1.0, 0.0, 0.0),
            V3::new(0.0, 1.0, 0.0),
            V3::new(0.0, -1.0, 0.0),
            V3::new(0.0, 0.0, 1.0),
            V3::new(0.0, 0.0, -1.0),
        ][self.face as usize]
    }
    pub fn up(self) -> V3 {
        if self.face == 2 {
            V3::new(0.0, 0.0, -1.0)
        } else if self.face == 3 {
            V3::new(0.0, 0.0, 1.0)
        } else {
            V3::new(0.0, 1.0, 0.0)
        }
    }
    pub fn right(self) -> V3 {
        let u = self.up();
        let n = self.normal();
        V3::new(
            u.y * n.z - u.z * n.y,
            u.z * n.x - u.x * n.z,
            u.x * n.y - u.y * n.x,
        )
    }
    pub fn vector_to(self, exit: Self, v: V3) -> V3 {
        exit.right() * -v.dot(self.right())
            + exit.up() * v.dot(self.up())
            + exit.normal() * -v.dot(self.normal())
    }
    pub fn point_to(self, exit: Self, p: V3) -> V3 {
        exit.center + self.vector_to(exit, p - self.center)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct State {
    pub ends: Pair,
    pub cooldown: [f32; 2],
    pub down: [bool; 2],
    pub lock: f32,
    /// Shared lockout for both owned endpoints after any player passes through.
    pub reopen: f32,
    pub momentum: V3,
    /// Cumulative offsets keep unacknowledged mouse inputs in a stable coordinate frame.
    pub yaw_offset: f32,
    pub pitch_offset: f32,
    pub transits: u16,
}

impl State {
    pub fn passable_ends(&self) -> Pair {
        if self.reopen > 0.0 {
            [None; 2]
        } else {
            self.ends
        }
    }
}

pub fn advance_reopen(remaining: &mut f32) {
    *remaining = (*remaining - DT).max(0.0);
    if *remaining < DT * 0.01 {
        *remaining = 0.0;
    }
}

pub fn is_elf(a: &Actor) -> bool {
    character(a.character).race == Race::Elf
}
pub fn aimed_input(a: &Actor, mut input: Input) -> Input {
    input.yaw += a.portals.yaw_offset;
    input.pitch = (input.pitch + a.portals.pitch_offset).clamp(-1.45, 1.45);
    input
}

fn extent(axis: V3, height: f32) -> f32 {
    (axis.x.abs() + axis.z.abs()) * BODY_RADIUS + axis.y.abs() * height * 0.5
}

/// A full aperture must fit on one solid face, with a clear standing exit.
pub fn placement(map: &[Block], origin: V3, aim: V3) -> Option<Portal> {
    let mut distance = RANGE;
    let mut hit = None;
    for (index, b) in map.iter().enumerate() {
        if let Some(t) = ray_box(origin, aim, b.min(), b.max()) {
            if t < distance {
                distance = t;
                hit = Some(index);
            }
        }
    }
    // The arena's implicit ground plane is also portalable.
    if aim.y < -0.0001 {
        let t = -origin.y / aim.y;
        if t > 0.0 && t < distance {
            distance = t;
            hit = None;
        }
    }
    if distance >= RANGE || distance <= 0.02 {
        return None;
    }
    let point = origin + aim * distance;
    let mut portal = Portal {
        center: point,
        face: 2,
    };
    if let Some(index) = hit {
        let b = map[index];
        // Outer boundaries stay sealed, even when approached from inside the arena.
        if !b.phaseable && b.size.y >= 1.0 {
            return None;
        }
        let lo = b.min();
        let hi = b.max();
        let faces = [
            (hi.x - point.x).abs(),
            (lo.x - point.x).abs(),
            (hi.y - point.y).abs(),
            (lo.y - point.y).abs(),
            (hi.z - point.z).abs(),
            (lo.z - point.z).abs(),
        ];
        portal.face = faces
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))?
            .0 as u8;
        // Snap a low wall hit up so that an eye-height shot makes a walk-in doorway.
        let right = portal.right();
        let up = portal.up();
        for (axis, half) in [(right, HALF_WIDTH), (up, HALF_HEIGHT)] {
            let size = b
                .size
                .dot(V3::new(axis.x.abs(), axis.y.abs(), axis.z.abs()));
            if size < half * 2.0 + 0.04 {
                return None;
            }
            let local = (portal.center - b.center).dot(axis);
            let limit = size * 0.5 - half - 0.02;
            portal.center = portal.center + axis * (local.clamp(-limit, limit) - local);
        }
    } else if point.y.abs() > 0.01 {
        return None;
    }
    // Keep overhead shots from placing an exit beyond the sealed arena walls.
    if map
        .iter()
        .filter(|b| !b.phaseable && b.size.y >= 10.0)
        .any(|b| {
            (b.size.x <= 2.0
                && b.size.z >= 40.0
                && (origin.x - b.center.x) * (portal.center.x - b.center.x) < 0.0)
                || (b.size.z <= 2.0
                    && b.size.x >= 40.0
                    && (origin.z - b.center.z) * (portal.center.z - b.center.z) < 0.0)
        })
    {
        return None;
    }
    // Probe the entire opening and the space immediately outside it. This also
    // rejects adjacent props, corners and apertures overhanging a raised floor.
    for x in [-0.8, 0.0, 0.8] {
        for y in [-1.3, 0.0, 1.3] {
            let probe =
                portal.center + portal.right() * x + portal.up() * y + portal.normal() * 0.08;
            if probe.y < -0.001
                || map.iter().any(|b| {
                    let lo = b.min();
                    let hi = b.max();
                    probe.x > lo.x
                        && probe.x < hi.x
                        && probe.y > lo.y
                        && probe.y < hi.y
                        && probe.z > lo.z
                        && probe.z < hi.z
                })
            {
                return None;
            }
        }
    }
    exit_position(map, portal, V3::default(), 1.8)?;
    Some(portal)
}

pub fn advance(map: &[Block], a: &mut Actor, input: Input, occupied: &[Pair]) {
    for cd in &mut a.portals.cooldown {
        *cd = (*cd - DT).max(0.0);
        if *cd < DT * 0.01 {
            *cd = 0.0;
        }
    }
    advance_reopen(&mut a.portals.lock);
    let buttons = [input.fire, input.alt_fire];
    for (color, button) in buttons.into_iter().enumerate() {
        let pressed = button && !a.portals.down[color];
        a.portals.down[color] = button;
        if !is_elf(a)
            || !input.phase_shift
            || !pressed
            || a.portals.cooldown[color] > 0.0
            || !a.alive()
            || a.combat.skills_locked()
            || a.combat.channeling()
        {
            continue;
        }
        let aim = aimed_input(a, input);
        if let Some(p) = placement(map, a.eye(), direction(aim.yaw, aim.pitch)) {
            // Different owners and the other endpoint must not share an aperture.
            if occupied
                .iter()
                .flatten()
                .flatten()
                .any(|other| Some(*other) != a.portals.ends[color] && overlaps(p, *other))
                || a.portals.ends[1 - color].is_some_and(|other| overlaps(p, other))
            {
                continue;
            }
            a.portals.ends[color] = Some(p);
            a.portals.cooldown[color] = COOLDOWN;
            a.dancing = false;
            a.wave_ticks = 0;
        }
    }
}

fn overlaps(a: Portal, b: Portal) -> bool {
    a.face == b.face
        && (a.center - b.center).dot(a.normal()).abs() < 0.1
        && (a.center - b.center).dot(a.right()).abs() < HALF_WIDTH * 2.0
        && (a.center - b.center).dot(a.up()).abs() < HALF_HEIGHT * 2.0
}

fn exit_position(map: &[Block], exit: Portal, offset: V3, height: f32) -> Option<V3> {
    let right = exit.right();
    let up = exit.up();
    let normal = exit.normal();
    let x = offset.dot(right).clamp(
        -HALF_WIDTH + extent(right, height),
        HALF_WIDTH - extent(right, height),
    );
    let y = offset.dot(up).clamp(
        -HALF_HEIGHT + extent(up, height),
        HALF_HEIGHT - extent(up, height),
    );
    let center = exit.center + right * x + up * y + normal * (extent(normal, height) + EXIT_GAP);
    let foot = center - V3::new(0.0, height * 0.5, 0.0);
    (foot.y >= -0.001 && !blocked_with_height(map, foot, height)).then_some(foot)
}

/// Test the swept body before solid collision prevents it reaching the portal plane.
/// Returns the pair owner so both endpoints can be closed after a successful transit.
pub fn traverse(map: &[Block], pairs: &[Pair], a: &mut Actor, velocity: V3) -> Option<usize> {
    if a.portals.lock > 0.0 || !a.alive() || a.combat.disabled() {
        return None;
    }
    let height = a.collision_height();
    let center = a.pos + V3::new(0.0, height * 0.5, 0.0);
    for (owner, pair) in pairs.iter().enumerate() {
        let [Some(first), Some(second)] = *pair else {
            continue;
        };
        for (entry, exit) in [(first, second), (second, first)] {
            let n = entry.normal();
            let approach = velocity.dot(n);
            if approach >= -0.01 {
                continue;
            }
            let d = (center - entry.center).dot(n) - extent(n, height);
            if d < -0.08 || d + approach * DT > 0.025 {
                continue;
            }
            let t = (d / -approach).clamp(0.0, DT);
            let offset = center + velocity * t - entry.center;
            if offset.dot(entry.right()).abs() + extent(entry.right(), height) > HALF_WIDTH - 0.03
                || offset.dot(entry.up()).abs() + extent(entry.up(), height) > HALF_HEIGHT + 0.06
            {
                continue;
            }
            let Some(pos) = exit_position(map, exit, entry.vector_to(exit, offset), height) else {
                continue;
            };
            let look = entry.vector_to(exit, direction(a.yaw, a.pitch));
            let yaw = if look.flat().length() > 0.001 {
                look.x.atan2(look.z)
            } else {
                a.yaw
            };
            let pitch = look.y.clamp(-1.0, 1.0).asin().clamp(-1.45, 1.45);
            a.portals.yaw_offset += (yaw - a.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            a.portals.pitch_offset += pitch - a.pitch;
            a.yaw = yaw;
            a.pitch = pitch;
            a.pos = pos;
            let v = entry.vector_to(exit, velocity);
            a.vy = v.y;
            a.portals.momentum = v.flat();
            a.portals.lock = REOPEN_DELAY;
            a.portals.transits = a.portals.transits.wrapping_add(1);
            a.think = 0.0;
            a.dancing = false;
            a.wave_ticks = 0;
            return Some(owner);
        }
    }
    None
}
