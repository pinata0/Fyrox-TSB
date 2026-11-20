//! Renua's source-timed empty-hand attacks and team-neutral temporal field.
use crate::{
    combat::{AttackClass, Projectile, ProjectileKind},
    game::{direction, Actor, Input, World, DT, V3},
    motion::Motion,
};
use serde::Deserialize;
use std::sync::OnceLock;

pub const BASIC_SECONDS: f32 = 81.0 / 30.0;
pub const ENHANCED_SECONDS: f32 = 86.0 / 30.0;
pub const SKILL_SECONDS: f32 = 149.0 / 30.0;
pub const ULTIMATE_SECONDS: f32 = 481.0 / 30.0;
pub const BASIC_RELEASE: f32 = 39.0 / 30.0;
pub const ENHANCED_RELEASE: f32 = 41.0 / 30.0;
pub const SKILL_RELEASES: [f32; 5] = [1.1, 1.6, 2.1, 2.6, 3.6];
pub const FIELD_RELEASE: f32 = 127.0 / 30.0;
pub const FIELD_SECONDS: f32 = 10.0;
pub const FIELD_NEAR: f32 = 1.5;
pub const FIELD_DEPTH: f32 = 12.0;
pub const FIELD_HALF_WIDTH: f32 = 4.0;
pub const FIELD_HEIGHT: f32 = 4.0;
pub const FREEZE_SECONDS: f32 = 2.0;
pub const SKILL_HEAL: f32 = 9.0;
pub const BEAM_SECONDS: f32 = 0.20;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Basic,
    Enhanced,
    Skill,
    Ultimate,
}
impl Phase {
    pub fn from_wire(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Off,
            1 => Self::Basic,
            2 => Self::Enhanced,
            3 => Self::Skill,
            4 => Self::Ultimate,
            _ => return None,
        })
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Basic => BASIC_SECONDS,
            Self::Enhanced => ENHANCED_SECONDS,
            Self::Skill => SKILL_SECONDS,
            Self::Ultimate => ULTIMATE_SECONDS,
        }
    }
    pub fn motion(self) -> Option<Motion> {
        Some(match self {
            Self::Off => return None,
            Self::Basic => Motion::RenuaBasic,
            Self::Enhanced => Motion::RenuaEnhanced,
            Self::Skill => Motion::RenuaSkill,
            Self::Ultimate => Motion::RenuaUltimate,
        })
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub elapsed: f32,
    pub alt_down: bool,
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
        (self.elapsed / DT + 0.0001).floor() as u32 | ((self.alt_down as u32) << 10)
    }
    pub fn unpack(phase: Phase, bits: u32) -> Option<Self> {
        let elapsed = (bits & 1023) as f32 * DT;
        if bits >> 11 != 0
            || (phase == Phase::Off && elapsed != 0.0)
            || (phase != Phase::Off && elapsed >= phase.duration())
        {
            return None;
        }
        Some(Self {
            phase,
            elapsed,
            alt_down: bits & (1 << 10) != 0,
        })
    }
}
pub fn time_factor(a: &Actor) -> f32 {
    if crate::erpin_skill::unstoppable(a) { 1.0 } else { (1.0 - a.combat.temporal).clamp(0.0, 1.0) }
}
pub fn contains(field: &Projectile, pos: V3) -> bool {
    let dir = field.velocity.flat().normalized();
    let right = V3::new(dir.z, 0.0, -dir.x);
    let d = pos - field.pos;
    (0.0..=FIELD_DEPTH).contains(&d.dot(dir))
        && d.dot(right).abs() <= FIELD_HALF_WIDTH
        && (-0.15..=FIELD_HEIGHT).contains(&d.y)
}
pub fn temporal_step(map: &[crate::game::Block], fields: &[Projectile], a: &mut Actor) {
    if crate::erpin_skill::unstoppable(a) { crate::erpin_skill::clear_charge_control(a); return; }
    let inside = a.alive()
        && !a.burrowed()
        && fields.iter().any(|p| {
            p.kind == ProjectileKind::TimeField && p.life > 0.0 && contains(p, a.pos) && {
                let origin =
                    p.pos - p.velocity.flat().normalized() * FIELD_NEAR + V3::new(0.0, 1.0, 0.0);
                let delta = a.eye() - origin;
                let distance = delta.length();
                !map.iter().any(|b| {
                    crate::game::ray_box(origin, delta.normalized(), b.min(), b.max())
                        .is_some_and(|t| t < distance - 0.05)
                })
            }
        });
    a.combat.temporal = if inside {
        (a.combat.temporal + DT / FREEZE_SECONDS).min(1.0)
    } else {
        0.0
    };
}
impl World {
    pub(crate) fn start_renua(&mut self, source: usize, phase: Phase) {
        let a = &mut self.actors[source];
        a.combat.renua.start(phase);
        a.reload = 0.0;
        a.shield = 0.0;
        if matches!(phase, Phase::Basic | Phase::Enhanced) {
            a.cooldown = phase.duration();
            a.combat.attacks = a.combat.attacks.wrapping_add(1);
        }
    }
    pub(crate) fn advance_renua(&mut self, source: usize, input: Input) {
        let a = &mut self.actors[source];
        if a.character != 7 || !a.alive() {
            a.combat.renua = State::default();
            return;
        }
        let previous = a.combat.renua;
        let pressed = input.alt_fire && !previous.alt_down;
        a.combat.renua.alt_down = input.alt_fire;
        if a.combat.stun > 0.0 || a.combat.sleep > 0.0 || a.combat.frozen > 0.0 || a.burrowed() {
            a.combat.renua.start(Phase::Off);
            return;
        }
        a.combat.renua.advance(time_factor(a));
        let elapsed = a.combat.renua.elapsed;
        let crossed = |time: f32| previous.elapsed + 0.00001 < time && elapsed + 0.00001 >= time;
        let phase = previous.phase;
        let alt = pressed
            && a.cooldown <= 0.0
            && !a.combat.disabled()
            && !a.combat.channeling()
            && !input.ability
            && !input.ultimate
            && !input.phase_shift;
        if phase == Phase::Basic && crossed(BASIC_RELEASE) {
            let dir = direction(a.yaw, a.pitch);
            a.shot = a.shot.wrapping_add(1);
            self.projectile(
                source,
                ProjectileKind::RenuaOrb,
                dir,
                40.0,
                15.0,
                AttackClass::Basic,
            );
        } else if phase == Phase::Enhanced && crossed(ENHANCED_RELEASE) {
            self.renua_beam(source, 26.0, AttackClass::Basic, false);
        } else if phase == Phase::Skill {
            for time in SKILL_RELEASES {
                if crossed(time) {
                    self.renua_beam(source, 12.0, AttackClass::Skill, true);
                }
            }
        } else if phase == Phase::Ultimate && crossed(FIELD_RELEASE) {
            let a = &self.actors[source];
            let dir = direction(a.yaw, 0.0);
            let origin = a.pos + dir * FIELD_NEAR;
            if let Some(index) = self.projectile(
                source,
                ProjectileKind::TimeField,
                dir,
                1.0,
                0.0,
                AttackClass::Ultimate,
            ) {
                let p = &mut self.projectiles[index];
                p.pos = origin;
                p.origin = origin;
                p.life = FIELD_SECONDS;
            }
        }
        if alt {
            self.start_renua(source, Phase::Enhanced);
        }
    }
    fn renua_beam(&mut self, source: usize, damage: f32, class: AttackClass, heal: bool) {
        let health_layers = |a: &Actor| a.hp as f32 + a.combat.guard + a.barrier;
        let before = self.actors.each_ref().map(health_layers);
        self.actors[source].shot = self.actors[source].shot.wrapping_add(1);
        let target = self.ray(source, 40.0, damage, 1, class, false);
        if heal && target.is_some_and(|i| health_layers(&self.actors[i]) < before[i]) {
            self.heal_from(source, source, SKILL_HEAL);
        }
    }
}

#[derive(Deserialize)]
pub struct Sample {
    pub hands: [[f32; 3]; 2],
    pub root: [f32; 2],
    pub details: Vec<f32>,
    pub constructs: Vec<[f32; 6]>,
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
        serde_json::from_str(include_str!("../assets/characters/skill/renua.json"))
            .expect("Renua source tracks")
    });
    let index = match motion {
        Motion::RenuaBasic => 1,
        Motion::RenuaEnhanced => 2,
        Motion::RenuaSkill => 3,
        Motion::RenuaUltimate => 4,
        _ => 0,
    };
    let samples = &data.clips[index].samples;
    let time = if index == 0 { age.rem_euclid(2.0) } else { age };
    let at = (time * 30.0).clamp(0.0, (samples.len() - 1) as f32);
    let i = at as usize;
    (
        &samples[i],
        &samples[(i + 1).min(samples.len() - 1)],
        at - i as f32,
    )
}
pub fn hand(motion: Motion, age: f32, left: bool) -> [f32; 3] {
    let (a, b, t) = sample(motion, age);
    let i = usize::from(left);
    std::array::from_fn(|j| a.hands[i][j] + (b.hands[i][j] - a.hands[i][j]) * t)
}
