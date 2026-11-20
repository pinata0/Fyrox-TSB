//! Butter's release-to-fire slingshot and source-timed airborne/giant shots.
use crate::{
    combat::{AttackClass, Projectile, ProjectileKind},
    game::{direction, ray_box, Actor, Input, World, DT, V3},
    motion::Motion,
};
#[cfg(test)]
mod tests;

pub const DRAW_SECONDS: f32 = 19.0 / 30.0;
pub const RELEASE_SECONDS: f32 = 0.20;
pub const SKILL_SECONDS: f32 = 70.0 / 30.0;
pub const TAKEOFF: f32 = 5.0 / 30.0;
pub const HOVER_END: f32 = 1.2;
pub const SKILL_RELEASE: f32 = 1.1;
pub const ULTIMATE_SECONDS: f32 = 118.0 / 30.0;
pub const ULTIMATE_RELEASE: f32 = 2.5;
pub const BOUNCES: u8 = 6;
pub const BOULDER_DAMAGE: f32 = 80.0;
pub const BOULDER_SPEED: f32 = 40.0;
pub const BOULDER_GRAVITY: f32 = 1.25;
pub const BOULDER_RADIUS: f32 = 0.30;
pub const BLAST_RADIUS: f32 = 4.5;
pub const STUN_SECONDS: f32 = 3.0;
pub const BLAST_SECONDS: f32 = 0.65;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Draw,
    Release,
    Skill,
    Ultimate,
}
impl Phase {
    pub fn from_wire(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Off,
            1 => Self::Draw,
            2 => Self::Release,
            3 => Self::Skill,
            4 => Self::Ultimate,
            _ => return None,
        })
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Draw => DRAW_SECONDS,
            Self::Release => RELEASE_SECONDS,
            Self::Skill => SKILL_SECONDS,
            Self::Ultimate => ULTIMATE_SECONDS,
        }
    }
    pub fn motion(self) -> Option<Motion> {
        match self {
            Self::Off => None,
            Self::Draw | Self::Release => Some(Motion::ButterAttack),
            Self::Skill => Some(Motion::ButterSkill),
            Self::Ultimate => Some(Motion::ButterUltimate),
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub elapsed: f32,
    pub fire_down: bool,
}
impl State {
    pub fn active(self) -> bool {
        self.phase != Phase::Off
    }
    pub fn locked(self) -> bool {
        matches!(self.phase, Phase::Release | Phase::Skill | Phase::Ultimate)
    }
    pub fn hovering(self) -> bool {
        self.phase == Phase::Skill && self.elapsed >= TAKEOFF && self.elapsed < HOVER_END
    }
    pub fn motion_age(self) -> f32 {
        if self.phase == Phase::Release {
            20.0 / 30.0 + self.elapsed / RELEASE_SECONDS * (1.5 - 20.0 / 30.0)
        } else {
            self.elapsed
        }
    }
    pub fn pack(self) -> u32 {
        ((self.elapsed / DT).round() as u32) | ((self.fire_down as u32) << 10)
    }
    pub fn unpack(phase: Phase, bits: u32) -> Option<Self> {
        let elapsed = (bits & 1023) as f32 * DT;
        (bits >> 11 == 0
            && if phase == Phase::Off {
                elapsed == 0.0
            } else if phase == Phase::Draw {
                elapsed <= DRAW_SECONDS + 0.00001
            } else {
                elapsed < phase.duration()
            })
        .then_some(Self {
            phase,
            elapsed,
            fire_down: bits & 1024 != 0,
        })
    }
}
pub fn smooth(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
pub fn lift(t: f32) -> f32 {
    0.85 * smooth((t - TAKEOFF) / 0.25)
}
/// Only the phase and clock are predicted; projectile creation stays authoritative.
pub fn advance_actor(a: &mut Actor, input: Input) -> bool {
    let old = a.combat.butter;
    let factor = crate::renua_skill::time_factor(a);
    if a.character != 11 || !a.alive() || a.combat.action_interrupted() || a.burrowed() {
        a.combat.butter = State {
            fire_down: input.fire,
            ..Default::default()
        };
        return false;
    }
    let s = &mut a.combat.butter;
    s.fire_down = input.fire;
    if old.phase == Phase::Draw {
        if input.reload
            || input.ability
            || input.ultimate
            || input.phase_shift
            || input.dance
            || a.reload > 0.0
        {
            s.phase = Phase::Off;
            s.elapsed = 0.0;
        } else if !input.fire && factor > 0.0 {
            s.phase = Phase::Release;
            s.elapsed = 0.0;
            return true;
        } else {
            s.elapsed = (s.elapsed + DT * factor).min(DRAW_SECONDS);
        }
    } else if old.locked() {
        s.elapsed += DT * factor;
        if s.elapsed + 0.00001 >= s.phase.duration() {
            s.phase = Phase::Off;
            s.elapsed = 0.0;
        }
    } else if input.fire
        && !old.fire_down
        && !input.ability
        && !input.ultimate
        && !input.reload
        && !input.phase_shift
        && a.reload <= 0.0
        && a.cooldown <= 0.0
        && (crate::kits::kit(a.character).magazine == 0 || a.ammo > 0)
        && !a.combat.disabled()
        && !a.combat.channeling()
    {
        a.combat.butter.phase = Phase::Draw;
        a.combat.butter.elapsed = 0.0;
        a.shield = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
    }
    false
}
impl World {
    pub(crate) fn start_butter(&mut self, source: usize, phase: Phase) {
        let a = &mut self.actors[source];
        a.combat.butter = State {
            phase,
            elapsed: 0.0,
            fire_down: a.combat.butter.fire_down,
        };
        a.reload = 0.0;
        a.shield = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.ability_flash = 0.0;
        a.combat.ultimate_flash = if phase == Phase::Ultimate {
            ULTIMATE_SECONDS
        } else {
            0.0
        };
        a.crouching = false;
        a.race_skill.cancel();
        a.portals.momentum = V3::default();
    }
    pub(crate) fn advance_butter(&mut self, source: usize, input: Input) {
        let before = self.actors[source].combat.butter;
        let release = advance_actor(&mut self.actors[source], input);
        let after = self.actors[source].combat.butter;
        if self.actors[source].character == 11
            && input.fire
            && crate::kits::kit(11).magazine > 0
            && self.actors[source].ammo == 0
            && !after.active()
        {
            self.reload(source);
        }
        if release {
            // The ordinary weapon owns ammo, haste, damage and the shot counter.
            self.actors[source].combat.butter.phase = Phase::Off;
            self.primary_attack(source);
            self.actors[source].combat.butter = after;
        }
        if before.phase != after.phase || !after.active() {
            return;
        }
        let crossed = |t| before.elapsed + 0.00001 < t && after.elapsed + 0.00001 >= t;
        let shot = if after.phase == Phase::Skill && crossed(SKILL_RELEASE) {
            Some((ProjectileKind::Ricochet, 30.0, 14.0, AttackClass::Skill))
        } else if after.phase == Phase::Ultimate && crossed(ULTIMATE_RELEASE) {
            Some((
                ProjectileKind::Boulder,
                BOULDER_SPEED,
                BOULDER_DAMAGE,
                AttackClass::Ultimate,
            ))
        } else {
            None
        };
        if let Some((kind, speed, damage, class)) = shot {
            let a = &self.actors[source];
            let dir = direction(a.yaw, a.pitch);
            if let Some(index) = self.projectile(source, kind, dir, speed, damage, class) {
                let p = &mut self.projectiles[index];
                p.bounces = if kind == ProjectileKind::Ricochet {
                    BOUNCES
                } else {
                    0
                };
                p.life = 4.0;
            }
            self.actors[source].shot = self.actors[source].shot.wrapping_add(1);
        }
    }
    /// Six physical reflections, shared by walls and bodies. A target becomes
    /// eligible again after the ball leaves its box, allowing a later return hit.
    pub(crate) fn advance_butter_ball(&mut self, p: &mut Projectile) {
        let radius = p.kind.radius();
        let pad = V3::new(radius, radius, radius);
        let bounds = |a: &Actor| {
            (
                a.pos + V3::new(-0.48 * a.body_scale(), 0.0, -0.48 * a.body_scale()) - pad,
                a.pos + V3::new(0.48 * a.body_scale(), a.hit_height(), 0.48 * a.body_scale()) + pad,
            )
        };
        for (i, a) in self.actors.iter().enumerate() {
            let (lo, hi) = bounds(a);
            if p.pos.x < lo.x - 0.01
                || p.pos.x > hi.x + 0.01
                || p.pos.y < lo.y - 0.01
                || p.pos.y > hi.y + 0.01
                || p.pos.z < lo.z - 0.01
                || p.pos.z > hi.z + 0.01
            {
                p.hit_mask &= !(1 << i);
            }
        }
        let mut remaining = p.velocity.length() * DT;
        for _ in 0..=BOUNCES {
            let dir = p.velocity.normalized();
            let wall = crate::impact::trace(&self.map, p.pos, dir, remaining, radius);
            let mut nearest = wall.map_or(remaining + 0.001, |h| h.distance);
            let mut normal = wall.map(|h| crate::impact::NORMALS[h.face as usize]);
            let mut target = None;
            for (i, a) in self.actors.iter().enumerate() {
                if i == p.owner || !a.alive() || a.burrowed() || p.hit_mask & (1 << i) != 0 {
                    continue;
                }
                let (lo, hi) = bounds(a);
                if let Some(t) =
                    ray_box(p.pos, dir, lo, hi).filter(|t| *t <= remaining && *t < nearest)
                {
                    let point = p.pos + dir * t;
                    let distances = [
                        point.x - lo.x,
                        point.x - hi.x,
                        point.y - lo.y,
                        point.y - hi.y,
                        point.z - lo.z,
                        point.z - hi.z,
                    ];
                    let face = (0..6)
                        .filter(|&f| dir.dot(crate::impact::NORMALS[f]) < 0.0)
                        .min_by(|&x, &y| distances[x].abs().total_cmp(&distances[y].abs()))
                        .unwrap();
                    nearest = t;
                    normal = Some(crate::impact::NORMALS[face]);
                    target = Some(i);
                }
            }
            let Some(normal) = normal else {
                p.pos = p.pos + dir * remaining;
                break;
            };
            p.pos = p.pos + dir * nearest;
            if let Some(i) = target {
                p.hit_mask |= 1 << i;
                self.projectile_hit(p, i);
            }
            if p.bounces == 0 {
                p.life = 0.0;
                return;
            }
            p.bounces -= 1;
            p.velocity = p.velocity - normal * (2.0 * p.velocity.dot(normal));
            p.pos = p.pos + normal * 0.006;
            remaining = (remaining - nearest).max(0.0);
            if remaining < 0.0001 {
                break;
            }
        }
    }
}

/// Source attack is held just before release; skill and ultimate retain source time.
pub fn pull(motion: Motion, t: f32) -> f32 {
    match motion {
        Motion::ButterAttack => smooth(t / DRAW_SECONDS) * (1.0 - smooth((t - 20.0 / 30.0) / 0.10)),
        Motion::ButterSkill => {
            smooth((t - 0.23) / 0.55) * (1.0 - smooth((t - SKILL_RELEASE) / 0.1))
        }
        Motion::ButterUltimate => {
            smooth((t - 1.1) / 1.2) * (1.0 - smooth((t - ULTIMATE_RELEASE) / 0.12))
        }
        _ => 0.0,
    }
}
pub fn giant(motion: Motion, t: f32) -> bool {
    motion == Motion::ButterUltimate && (0.95..3.3).contains(&t)
}
pub fn growth(motion: Motion, t: f32) -> f32 {
    if giant(motion, t) {
        smooth((t - 0.95) / 0.32) * (1.0 - smooth((t - 3.05) / 0.25))
    } else {
        0.0
    }
}
pub fn weapon_visible(motion: Motion, t: f32) -> bool {
    motion != Motion::ButterUltimate || t < 0.2 || t >= 0.95
}
pub fn loaded(motion: Motion, t: f32) -> bool {
    match motion {
        Motion::ButterAttack => t > 0.08 && t < 20.0 / 30.0,
        Motion::ButterSkill => t > 0.23 && t < SKILL_RELEASE,
        Motion::ButterUltimate => t >= 1.1 && t < ULTIMATE_RELEASE,
        _ => false,
    }
}
pub fn left_holding(motion: Motion, t: f32) -> bool {
    match motion {
        Motion::ButterAttack => t > 0.04 && t < 0.77,
        Motion::ButterSkill => t > 0.23 && t < 1.2,
        Motion::ButterUltimate => giant(motion, t),
        _ => false,
    }
}
