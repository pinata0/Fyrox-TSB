//! Aya's archived cast clocks, snow butterflies and an authoritative ice prison.
use crate::{
    combat::{AttackClass, ProjectileKind},
    game::{direction, Actor, World, DT, V3},
    motion::Motion,
};
use serde::Deserialize;
use std::sync::OnceLock;

pub const SKILL_SECONDS: f32 = 94.0 / 30.0;
pub const ULTIMATE_SECONDS: f32 = 110.0 / 30.0;
pub const BUTTERFLY_RELEASE: f32 = 39.0 / 30.0;
pub const FREEZE_RELEASE: f32 = 57.0 / 30.0;
pub const BUTTERFLIES: usize = 5;
pub const BUTTERFLY_DAMAGE: f32 = 6.0;
pub const REFUND_SECONDS: f32 = 1.0;
pub const WEAKEN_SECONDS: f32 = 4.0;
pub const FREEZE_SECONDS: f32 = 3.0;
pub const SNOW_SECONDS: f32 = 5.5;
pub const ULTIMATE_RADIUS: f32 = 12.0;
pub const ULTIMATE_DAMAGE: f32 = 35.0;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Skill,
    Ultimate,
}
impl Phase {
    pub fn from_wire(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Off,
            1 => Self::Skill,
            2 => Self::Ultimate,
            _ => return None,
        })
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Skill => SKILL_SECONDS,
            Self::Ultimate => ULTIMATE_SECONDS,
        }
    }
    pub fn motion(self) -> Option<Motion> {
        match self {
            Self::Off => None,
            Self::Skill => Some(Motion::AyaSkill),
            Self::Ultimate => Some(Motion::AyaUltimate),
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
    pub fn start(&mut self, phase: Phase) {
        self.phase = phase;
        self.elapsed = 0.0;
    }
    pub fn advance(&mut self, factor: f32) {
        if self.active() {
            self.elapsed += DT * factor;
            if self.elapsed + 0.00001 >= self.phase.duration() {
                self.start(Phase::Off);
            }
        }
    }
    pub fn pack(self) -> u32 {
        if self.active() {
            ((self.elapsed / DT).round() as u32)
                .min((self.phase.duration() / DT).round() as u32 - 1)
        } else {
            0
        }
    }
    pub fn unpack(phase: Phase, ticks: u32) -> Option<Self> {
        let elapsed = ticks as f32 * DT;
        if (phase == Phase::Off && ticks != 0)
            || (phase != Phase::Off && elapsed >= phase.duration())
        {
            return None;
        }
        Some(Self { phase, elapsed })
    }
}

/// Share interruption rules between authority and prediction; hard frost arrests view too.
pub fn advance_actor(a: &mut Actor) {
    if a.character != 4 || !a.alive() || a.combat.action_interrupted() || a.burrowed() {
        a.combat.aya = State::default();
    } else {
        a.combat.aya.advance(crate::renua_skill::time_factor(a));
    }
}
impl World {
    pub(crate) fn start_aya(&mut self, source: usize, phase: Phase) {
        let a = &mut self.actors[source];
        a.combat.aya.start(phase);
        a.reload = 0.0;
        a.shield = 0.0;
        a.ability_flash = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        if phase == Phase::Ultimate {
            a.combat.ultimate_flash = ULTIMATE_SECONDS;
            let origin = a.pos;
            if let Some(index) = self.projectile(
                source,
                ProjectileKind::SnowGarden,
                V3::default(),
                0.0,
                0.0,
                AttackClass::Ultimate,
            ) {
                let p = &mut self.projectiles[index];
                p.pos = origin;
                p.origin = origin;
                p.life = SNOW_SECONDS;
            }
        }
    }
    pub(crate) fn advance_aya(&mut self, source: usize) {
        let previous = self.actors[source].combat.aya;
        advance_actor(&mut self.actors[source]);
        let current = self.actors[source].combat.aya;
        if previous.phase != current.phase || !current.active() {
            return;
        }
        let crossed =
            |time: f32| previous.elapsed + 0.00001 < time && current.elapsed + 0.00001 >= time;
        if current.phase == Phase::Skill && crossed(BUTTERFLY_RELEASE) {
            let a = &self.actors[source];
            let (yaw, pitch) = (a.yaw, a.pitch);
            for j in 0..BUTTERFLIES {
                let spread = (j as f32 - (BUTTERFLIES - 1) as f32 * 0.5) * 0.055;
                self.projectile(
                    source,
                    ProjectileKind::Butterfly,
                    direction(yaw + spread, pitch),
                    14.0,
                    BUTTERFLY_DAMAGE,
                    AttackClass::Skill,
                );
            }
        } else if current.phase == Phase::Ultimate && crossed(FREEZE_RELEASE) {
            for field in &mut self.projectiles {
                if field.owner == source && field.kind == ProjectileKind::SnowGarden {
                    field.step = 1;
                }
            }
            let center = self.actors[source].eye();
            for target in self.targets(source, center, ULTIMATE_RADIUS, false) {
                let susceptible = self.actors[target].combat.sleep <= 0.0 && !self.actors[target].combat.komi.resting();
                if self.damage_hit(source, target, ULTIMATE_DAMAGE, AttackClass::Ultimate, 4)
                    && susceptible
                    && self.actors[target].alive()
                {
                    let a = &mut self.actors[target];
                    a.combat.frozen = a.combat.frozen.max(FREEZE_SECONDS);
                    a.combat.cast = None;
                    a.combat.dash = 0.0;
                    a.combat.aya = State::default();
                    a.combat.naia = Default::default();
                    a.combat.id.start(crate::id_skill::Phase::Off);
                    a.combat.renua.start(crate::renua_skill::Phase::Off);
                    a.combat.erpin.eating = false;
                    a.reload = 0.0;
                    a.dancing = false;
                    a.wave_ticks = 0;
                    a.vy = 0.0;
                    a.portals.momentum = V3::default();
                    a.race_skill.cancel();
                }
            }
        }
    }
}

#[derive(Deserialize)]
pub struct Sample {
    pub hands: [[f32; 3]; 2],
    pub details: Vec<f32>,
}
#[derive(Deserialize)]
struct Clip {
    samples: Vec<Sample>,
}
#[derive(Deserialize)]
struct Data {
    clips: Vec<Clip>,
}
pub fn sample(motion: Motion, age: f32) -> (&'static Sample, &'static Sample, f32) {
    static DATA: OnceLock<Data> = OnceLock::new();
    let data = DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/skill/aya.json"))
            .expect("Aya source tracks")
    });
    let samples = &data.clips[usize::from(motion == Motion::AyaUltimate)].samples;
    let at = (age * 30.0).clamp(0.0, (samples.len() - 1) as f32);
    let i = at as usize;
    (
        &samples[i],
        &samples[(i + 1).min(samples.len() - 1)],
        at - i as f32,
    )
}
pub fn hand(motion: Motion, age: f32, left: bool) -> [f32; 3] {
    let (a, b, t) = sample(motion, age);
    let i = usize::from(!left);
    std::array::from_fn(|j| a.hands[i][j] + (b.hands[i][j] - a.hands[i][j]) * t)
}
pub fn bloom(age: f32) -> f32 {
    let t = ((age - FREEZE_RELEASE + 0.18) / 0.48).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
