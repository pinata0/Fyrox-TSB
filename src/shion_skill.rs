//! Shion's stationary marksman magazine and source-timed execution blast.
use crate::{
    combat::AttackClass,
    game::{direction, Actor, Input, World, DT, V3},
    motion::Motion,
};
use serde::Deserialize;
use std::sync::OnceLock;

pub const ROUNDS: u8 = 5;
pub const LOAD_SECONDS: f32 = 64.0 / 30.0;
pub const SHOT_SECONDS: f32 = 1.0;
pub const SHOT_RELEASE: f32 = 1.0 / 30.0;
pub const END_SECONDS: f32 = 40.0 / 30.0;
pub const ULTIMATE_SECONDS: f32 = 106.0 / 30.0;
pub const ULTIMATE_RELEASE: f32 = 80.0 / 30.0;
pub const DAMAGE: f32 = 65.0;
pub const STUN_SECONDS: f32 = 0.75;
pub const BLAST_DISTANCE: f32 = 6.0;
pub const BLAST_RADIUS: f32 = 4.0;
pub const BLAST_SECONDS: f32 = 0.8;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Load,
    Ready,
    Fire,
    End,
    Ultimate,
}
impl Phase {
    pub fn from_wire(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Off,
            1 => Self::Load,
            2 => Self::Ready,
            3 => Self::Fire,
            4 => Self::End,
            5 => Self::Ultimate,
            _ => return None,
        })
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Off | Self::Ready => 0.0,
            Self::Load => LOAD_SECONDS,
            Self::Fire => SHOT_SECONDS,
            Self::End => END_SECONDS,
            Self::Ultimate => ULTIMATE_SECONDS,
        }
    }
    pub fn motion(self) -> Option<Motion> {
        Some(match self {
            Self::Off => return None,
            Self::Load => Motion::ShionLoad,
            Self::Ready | Self::Fire => Motion::ShionLoop,
            Self::End => Motion::ShionEnd,
            Self::Ultimate => Motion::ShionUltimate,
        })
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
    pub fn start(&mut self, phase: Phase) {
        self.phase = phase;
        self.elapsed = 0.0;
    }
    pub fn pack(self) -> u32 {
        (self.elapsed / DT).floor() as u32
    }
    pub fn unpack(phase: Phase, ticks: u32) -> Option<Self> {
        let elapsed = ticks as f32 * DT;
        if ticks > 255
            || (matches!(phase, Phase::Off | Phase::Ready) && ticks != 0)
            || (phase.duration() > 0.0 && elapsed >= phase.duration())
        {
            return None;
        }
        Some(Self { phase, elapsed })
    }
}
/// Shared clock, ammunition and movement state for authority and prediction.
/// The returned events are applied only by the server.
pub fn advance(a: &mut Actor, input: Input) -> (bool, bool) {
    if a.character != 14 || !a.alive() {
        a.combat.shion = State::default();
        return (false, false);
    }
    if a.combat.stun > 0.0 || a.combat.sleep > 0.0 || a.burrowed() {
        if a.combat.shion.active() {
            a.combat.magic_ammo = 0;
        }
        a.combat.shion = State::default();
        return (false, false);
    }
    let factor = crate::renua_skill::time_factor(a);
    if factor <= 0.0 || a.combat.frozen > 0.0 {
        return (false, false);
    }
    let previous = a.combat.shion;
    if previous.phase == Phase::Ready {
        if input.fire {
            a.combat.shion.start(Phase::Fire);
        }
        return (false, false);
    }
    if !previous.active() {
        return (false, false);
    }
    a.combat.shion.elapsed += DT * factor;
    let crossed =
        |time: f32| previous.elapsed + 0.00001 < time && a.combat.shion.elapsed + 0.00001 >= time;
    let shot = previous.phase == Phase::Fire && crossed(SHOT_RELEASE);
    let blast = previous.phase == Phase::Ultimate && crossed(ULTIMATE_RELEASE);
    if shot {
        a.combat.magic_ammo = a.combat.magic_ammo.saturating_sub(1);
    }
    if a.combat.shion.elapsed + 0.00001 >= previous.phase.duration() {
        let next = match previous.phase {
            Phase::Load => {
                a.combat.magic_ammo = ROUNDS;
                Phase::Ready
            }
            Phase::Fire if a.combat.magic_ammo > 0 => Phase::Ready,
            Phase::Fire => Phase::End,
            _ => Phase::Off,
        };
        a.combat.shion.start(next);
    }
    (shot, blast)
}
impl World {
    pub(crate) fn start_shion(&mut self, source: usize, phase: Phase) {
        let a = &mut self.actors[source];
        a.combat.shion.start(phase);
        a.combat.magic_ammo = 0;
        a.reload = 0.0;
        a.cooldown = 0.0;
        a.race_skill.cancel();
        a.portals.momentum = V3::default();
    }
    pub(crate) fn advance_shion(&mut self, source: usize, input: Input) {
        let (shot, blast) = advance(&mut self.actors[source], input);
        if shot || blast {
            self.actors[source].yaw = input.yaw;
            self.actors[source].pitch = input.pitch;
        }
        if shot {
            let before = self.actors.each_ref().map(|a| (a.hp, a.shield_health()));
            self.actors[source].shot = self.actors[source].shot.wrapping_add(1);
            if let Some(target) = self.ray(source, 60.0, DAMAGE, 1, AttackClass::Skill, false) {
                let a = &mut self.actors[target];
                if a.alive()
                    && (a.hp < before[target].0 || a.shield_health() < before[target].1)
                    && a.combat.sleep <= 0.0
                {
                    a.combat.stun = a.combat.stun.max(STUN_SECONDS);
                }
            }
        }
        if blast {
            let a = &self.actors[source];
            let eye = a.eye();
            let forward = direction(a.yaw, 0.0);
            // Stop the center at cover so the explosion cannot be placed through walls.
            let distance = self
                .map
                .iter()
                .filter_map(|b| crate::game::ray_box(eye, forward, b.min(), b.max()))
                .fold(BLAST_DISTANCE, f32::min);
            let origin = eye + forward * (distance - 0.05).max(0.0);
            self.actors[source].shot_end = origin;
            for target in self.targets(source, origin, BLAST_RADIUS, false) {
                // Execution is handled by the common damage path for deaths and credit.
                self.damage_hit(source, target, 0.0, AttackClass::Ultimate, 14);
            }
        }
    }
}

#[derive(Deserialize)]
pub struct Sample {
    pub hands: [[f32; 3]; 2],
    pub root: [f32; 2],
}
#[derive(Deserialize)]
struct Clip {
    samples: Vec<Sample>,
}
#[derive(Deserialize)]
struct Data {
    clips: Vec<Clip>,
}
pub fn sample(motion: Motion, age: f32) -> Option<(&'static Sample, &'static Sample, f32)> {
    let index = match motion {
        Motion::ShionLoad => 0,
        Motion::ShionLoop => 1,
        Motion::ShionEnd => 2,
        Motion::ShionUltimate => 3,
        _ => return None,
    };
    static DATA: OnceLock<Data> = OnceLock::new();
    let data = DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/skill/shion.json"))
            .expect("Shion source tracks")
    });
    let samples = &data.clips[index].samples;
    let at = (age * 30.0).clamp(0.0, (samples.len() - 1) as f32);
    let i = at as usize;
    Some((
        &samples[i],
        &samples[(i + 1).min(samples.len() - 1)],
        at - i as f32,
    ))
}
pub fn hand(motion: Motion, age: f32) -> crate::attack::HandMotion {
    let Some((a, b, t)) = sample(motion, age) else {
        return Default::default();
    };
    let path: [f32; 3] =
        std::array::from_fn(|j| a.hands[0][j] + (b.hands[0][j] - a.hands[0][j]) * t);
    crate::attack::HandMotion {
        position: [
            0.0,
            (path[1] * 0.10).clamp(-0.06, 0.10),
            (path[0] * 0.06).clamp(-0.08, 0.08),
        ],
        // Periodic source angles avoid a discontinuity when the rifle crosses 180 degrees.
        angles: [-path[2].sin() * 0.26, 0.0, 0.0],
    }
}
