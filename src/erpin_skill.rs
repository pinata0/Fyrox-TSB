//! Archived Erpin E/Q timing, shared by server simulation and movement prediction.
use crate::{
    combat::{AttackClass, ProjectileKind},
    game::{direction, ray_box, Actor, Block, World, DT, V3},
    motion::Motion,
    movement,
};

pub const SKILL_SECONDS: f32 = 85.0 / 30.0;
pub const RELEASES: [f32; 4] = [51.0 / 30.0, 54.0 / 30.0, 59.0 / 30.0, 64.0 / 30.0];
pub const START_SECONDS: f32 = 35.0 / 30.0;
pub const LOOP_SECONDS: f32 = 16.0 / 30.0;
pub const RECOVER_SECONDS: f32 = 3.0;
pub const CHARGE_SPEED: f32 = 18.0;
pub const IMPACT_DAMAGE: f32 = 70.0;
pub const RECOIL_SPEED: f32 = 14.0;
pub const RECOIL_SECONDS: f32 = 0.8;
pub const BLAST_SECONDS: f32 = 0.6;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Skill,
    Start,
    Charge,
    Recover,
}
impl Phase {
    pub fn motion(self) -> Option<Motion> {
        Some(match self {
            Self::Off => return None,
            Self::Skill => Motion::ErpinSkill,
            Self::Start => Motion::ErpinStart,
            Self::Charge => Motion::ErpinCharge,
            Self::Recover => Motion::ErpinRecover,
        })
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Skill => SKILL_SECONDS,
            Self::Start => START_SECONDS,
            Self::Charge => LOOP_SECONDS,
            Self::Recover => RECOVER_SECONDS,
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub elapsed: f32,
}
impl State {
    pub fn active(self) -> bool {
        self.phase != Phase::Off
    }
    pub fn owns_movement(self) -> bool {
        matches!(self.phase, Phase::Start | Phase::Charge | Phase::Recover)
    }
    pub fn pack(self, alt_down: bool) -> u32 {
        self.phase as u32 | (((self.elapsed / DT).round() as u32) << 3) | ((alt_down as u32) << 16)
    }
    pub fn unpack(bits: u32) -> Option<(Self, bool)> {
        if bits >> 17 != 0 {
            return None;
        }
        let phase = match bits & 7 {
            1 => Phase::Skill,
            2 => Phase::Start,
            3 => Phase::Charge,
            4 => Phase::Recover,
            _ => return None,
        };
        let elapsed = ((bits >> 3) & 8191) as f32 * DT;
        (elapsed < phase.duration()).then_some((Self { phase, elapsed }, bits & (1 << 16) != 0))
    }
}

/// A charge cannot be steered, canceled or slowed. Its own collision is the only stop.
pub fn unstoppable(a: &Actor) -> bool {
    a.character == 2 && a.alive() && a.combat.erpin_skill.phase == Phase::Charge
}
pub fn clear_charge_control(a: &mut Actor) {
    if unstoppable(a) {
        let c = &mut a.combat;
        c.slow = 0.0;
        c.rain_slow = 0.0;
        c.temporal = 0.0;
        c.stun = 0.0;
        c.sleep = 0.0;
        c.frozen = 0.0;
        c.polymorph = 0.0;
    }
}

/// Swept body contacts choose the nearest wall or actor, including allies.
fn contact(map: &[Block], actors: &[Actor], slot: usize, a: &Actor, distance: f32) -> Option<f32> {
    let dir = direction(a.yaw, 0.0);
    let pad = movement::BODY_RADIUS;
    let walls = map
        .iter()
        .filter(|b| b.max().y > a.pos.y + 0.001 && b.min().y < a.pos.y + a.collision_height())
        .filter_map(|b| {
            ray_box(
                a.pos,
                dir,
                b.min() - V3::new(pad, a.collision_height() - 0.001, pad),
                b.max() + V3::new(pad, -0.001, pad),
            )
        });
    let bodies = actors
        .iter()
        .enumerate()
        .filter(|(i, b)| *i != slot && b.alive() && !b.burrowed())
        .filter_map(|(_, b)| {
            let radius = pad + 0.48 * b.body_scale();
            ray_box(
                a.pos,
                dir,
                b.pos - V3::new(radius, a.collision_height() - 0.001, radius),
                b.pos + V3::new(radius, b.hit_height(), radius),
            )
        });
    walls
        .chain(bodies)
        .filter(|t| *t <= distance)
        .min_by(f32::total_cmp)
}

/// Returns the collision point once. Prediction discards damage and visual events.
pub fn advance_actor(map: &[Block], actors: &[Actor], slot: usize, a: &mut Actor) -> Option<V3> {
    let state = a.combat.erpin_skill;
    if !state.active() {
        return None;
    }
    if a.character != 2
        || !a.alive()
        || (matches!(state.phase, Phase::Skill | Phase::Start)
            && (a.combat.action_interrupted() || a.combat.taunt > 0.0))
    {
        a.combat.erpin_skill = State::default();
        return None;
    }
    clear_charge_control(a);
    let dt =
        DT * crate::renua_skill::time_factor(a) * if a.combat.frozen > 0.0 { 0.0 } else { 1.0 };
    if state.phase == Phase::Recover {
        // Status countdowns use wall time, but a paused recovery must remain stunned.
        a.combat.stun = a
            .combat
            .stun
            .max((RECOVER_SECONDS - state.elapsed - dt).max(0.0));
    }
    if dt == 0.0 {
        return None;
    }
    let mut impact = None;
    let dir = direction(a.yaw, 0.0);
    if state.phase == Phase::Charge {
        let distance = CHARGE_SPEED * dt;
        if let Some(t) = contact(map, actors, slot, a, distance) {
            a.pos = a.pos + dir * (t - 0.002).max(0.0);
            impact = Some(a.pos + dir * 0.35 + V3::new(0.0, 0.9, 0.0));
            a.combat.erpin_skill = State {
                phase: Phase::Recover,
                elapsed: 0.0,
            };
            a.combat.stun = a.combat.stun.max(RECOVER_SECONDS);
            a.vy = 6.0;
        } else {
            a.pos = a.pos + dir * distance;
        }
    } else if state.phase == Phase::Recover {
        let speed = RECOIL_SPEED * (1.0 - state.elapsed / RECOIL_SECONDS).clamp(0.0, 1.0);
        a.pos = movement::walk_with_height(
            map,
            a.pos,
            dir * (-speed * dt),
            false,
            false,
            a.collision_height(),
        )
        .0;
    }
    if state.owns_movement() {
        a.portals.momentum = V3::default();
        a.vy -= 18.0 * dt;
        let height = a.collision_height();
        movement::vertical_with_height(map, &mut a.pos, &mut a.vy, dt, height);
    }
    if impact.is_none() {
        let next = state.elapsed + dt;
        a.combat.erpin_skill.elapsed = next;
        if next + 0.00001 >= state.phase.duration() {
            a.combat.erpin_skill = match state.phase {
                Phase::Start => State {
                    phase: Phase::Charge,
                    elapsed: 0.0,
                },
                Phase::Charge => State {
                    phase: Phase::Charge,
                    elapsed: next.rem_euclid(LOOP_SECONDS),
                },
                _ => State::default(),
            };
        }
    }
    impact
}

impl World {
    pub(crate) fn start_erpin_skill(&mut self, slot: usize, phase: Phase) {
        self.cancel_erpin(slot);
        let a = &mut self.actors[slot];
        a.combat.erpin_skill = State {
            phase,
            elapsed: 0.0,
        };
        a.reload = 0.0;
        a.shield = 0.0;
        a.ability_flash = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.race_skill.cancel();
        a.portals.momentum = V3::default();
        if phase == Phase::Start {
            a.crouching = false;
            a.combat.ultimate_flash = START_SECONDS;
        }
    }
    pub(crate) fn advance_erpin_skill(&mut self, slot: usize) {
        let before = self.actors[slot].combat.erpin_skill;
        if !before.active() {
            return;
        }
        let actors = self.actors.clone();
        let elapsed = before.elapsed + DT * crate::renua_skill::time_factor(&actors[slot]);
        let impact = advance_actor(&self.map, &actors, slot, &mut self.actors[slot]);
        if before.phase == Phase::Skill
            && self.actors[slot].combat.erpin_skill.phase == Phase::Skill
        {
            for (n, time) in RELEASES.into_iter().enumerate() {
                if before.elapsed + 0.00001 < time && elapsed + 0.00001 >= time {
                    let a = &self.actors[slot];
                    let dir = direction(a.yaw + (n as f32 - 1.5) * 0.045, a.pitch);
                    self.projectile(
                        slot,
                        ProjectileKind::MagicBomb,
                        dir,
                        20.0,
                        14.0,
                        AttackClass::Skill,
                    );
                }
            }
        }
        if let Some(point) = impact {
            self.burst(slot, point, 2.2, IMPACT_DAMAGE, AttackClass::Ultimate, 0.0);
            self.erpin_blast(slot, point, true);
        }
    }
    pub(crate) fn erpin_blast(&mut self, slot: usize, point: V3, ultimate: bool) {
        let kind = if ultimate {
            ProjectileKind::ErpinCrash
        } else {
            ProjectileKind::ErpinBlast
        };
        if let Some(i) = self.projectile(slot, kind, V3::default(), 0.0, 0.0, AttackClass::Ultimate)
        {
            let p = &mut self.projectiles[i];
            p.pos = point;
            p.origin = point;
            p.life = BLAST_SECONDS;
        }
    }
}

#[derive(serde::Deserialize)]
struct Sample {
    hands: [[f32; 3]; 2],
}
#[derive(serde::Deserialize)]
struct Clip {
    samples: Vec<Sample>,
}
pub fn hand(motion: Motion, age: f32, left: bool) -> [f32; 3] {
    static DATA: std::sync::OnceLock<Vec<Clip>> = std::sync::OnceLock::new();
    let index = match motion {
        Motion::ErpinSkill => 0,
        Motion::ErpinStart => 1,
        Motion::ErpinCharge => 2,
        Motion::ErpinRecover => 3,
        _ => return [0.0; 3],
    };
    let data = DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/skill/erpin.json"))
            .expect("Erpin source tracks")
    });
    let samples = &data[index].samples;
    let time = if index == 2 {
        age.rem_euclid(LOOP_SECONDS)
    } else {
        age
    };
    let f = (time * 30.0).clamp(0.0, (samples.len() - 1) as f32);
    let i = f as usize;
    std::array::from_fn(|j| {
        samples[i].hands[usize::from(left)][j]
            + (samples[(i + 1).min(samples.len() - 1)].hands[usize::from(left)][j]
                - samples[i].hands[usize::from(left)][j])
                * (f - i as f32)
    })
}

pub fn hand_motion(motion: Motion, age: f32) -> crate::attack::HandMotion {
    if motion == Motion::ErpinSkill {
        // Camera-space +Y lifts the grip and +X pitch tips the staff forward.
        // Raise it upright, then strike down at the first orb's release instead
        // of importing the source wrist rotation that bends it behind the view.
        let lift = crate::erpin::smooth((age - 0.12) / 0.90);
        let strike = crate::erpin::smooth((age - 1.38) / (RELEASES[0] - 1.38));
        let recover = 1.0 - crate::erpin::smooth((age - 2.28) / (SKILL_SECONDS - 2.28));
        return crate::attack::HandMotion {
            position: [
                (0.08 * lift + 0.10 * strike) * recover,
                (0.56 * lift * (1.0 - strike) - 0.12 * strike) * recover,
                (0.08 * lift + 0.16 * strike) * recover,
            ],
            angles: [(0.04 * lift + 0.96 * strike) * recover, 0.0, 0.0],
        };
    }
    let p = hand(motion, age, false);
    crate::attack::HandMotion {
        position: [
            0.0,
            (p[1] * 0.22).clamp(-0.22, 0.22),
            (p[0] * 0.24).clamp(-0.15, 0.25),
        ],
        angles: [
            (-p[2] * 0.55).clamp(-0.85, 0.85),
            0.0,
            (p[0] * 0.25).clamp(-0.25, 0.25),
        ],
    }
}
pub fn flustered(motion: Motion, age: f32) -> f32 {
    match motion {
        Motion::ErpinSkill => {
            crate::erpin::smooth((age - 0.78) / 0.12) * crate::erpin::smooth((2.60 - age) / 0.18)
        }
        Motion::ErpinStart => crate::erpin::smooth((age - 0.45) / 0.12),
        Motion::ErpinCharge => 1.0,
        Motion::ErpinRecover => crate::erpin::smooth((RECOVER_SECONDS - age) / 0.35),
        _ => 0.0,
    }
}
