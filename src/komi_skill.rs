//! Komi's interruptible blanket rest and source-timed feast / giant leap.
use crate::{
    combat::AttackClass,
    game::{Actor, Input, World, DT, V3},
    motion::Motion,
};

pub const REST_SECONDS: f32 = 4.0;
pub const REST_HEAL: f32 = 48.0;
pub const REST_PULSE: f32 = 0.25;
pub const FEAST_SECONDS: f32 = 132.0 / 30.0;
pub const GROW_START: f32 = 99.0 / 30.0;
pub const GIANT_SECONDS: f32 = 8.0;
pub const GIANT_SCALE: f32 = 1.8;
pub const LEAP_COOLDOWN: f32 = 1.0;
pub const LANDING_RADIUS: f32 = 5.0;
pub const LANDING_DAMAGE: f32 = 70.0;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Rest,
    Feast,
    Giant,
}
impl Phase {
    pub fn from_wire(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Off,
            1 => Self::Rest,
            2 => Self::Feast,
            3 => Self::Giant,
            _ => return None,
        })
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Rest => REST_SECONDS,
            Self::Feast => FEAST_SECONDS,
            Self::Giant => GIANT_SECONDS,
        }
    }
    pub fn motion(self) -> Option<Motion> {
        match self {
            Self::Rest => Some(Motion::KomiRest),
            Self::Feast => Some(Motion::KomiFeast),
            _ => None,
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub elapsed: f32,
    /// Prevent the activation E hold from immediately waking the caster.
    pub e_down: bool,
    pub impact: f32,
}
impl State {
    pub fn active(self) -> bool {
        self.phase != Phase::Off
    }
    pub fn resting(self) -> bool {
        self.phase == Phase::Rest
    }
    pub fn giant(self) -> bool {
        self.phase == Phase::Giant
    }
    pub fn locked(self) -> bool {
        matches!(self.phase, Phase::Rest | Phase::Feast)
    }
    pub fn scale(self) -> f32 {
        let growth = if self.giant() {
            1.0
        } else if self.phase == Phase::Feast {
            ((self.elapsed - GROW_START) / (FEAST_SECONDS - GROW_START)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        1.0 + (GIANT_SCALE - 1.0) * growth * growth * (3.0 - 2.0 * growth)
    }
    pub fn motion_age(self) -> f32 {
        if !self.resting() {
            return self.elapsed;
        }
        // Keep the source's entry/exit speed, shorten only its sleeping middle.
        if self.elapsed < 0.8 {
            self.elapsed
        } else if self.elapsed < 3.3 {
            0.8 + (self.elapsed - 0.8) * (5.466667 - 0.8) / 2.5
        } else {
            5.466667 + self.elapsed - 3.3
        }
    }
    pub fn pack(self) -> u32 {
        if !self.active() {
            return 0;
        }
        ((self.elapsed / DT).round() as u32).min((self.phase.duration() / DT).round() as u32 - 1)
            | ((self.e_down as u32) << 10)
            | (((self.impact / DT).round() as u32) << 11)
    }
    pub fn unpack(phase: Phase, bits: u32) -> Option<Self> {
        let s = Self {
            phase,
            elapsed: (bits & 1023) as f32 * DT,
            e_down: bits & 1024 != 0,
            impact: ((bits >> 11) & 31) as f32 * DT,
        };
        (bits >> 16 == 0
            && s.impact <= 0.35 + DT * 0.01
            && (s.impact == 0.0 || phase == Phase::Giant)
            && if phase == Phase::Off {
                bits == 0
            } else {
                s.elapsed < phase.duration()
            })
        .then_some(s)
    }
}

fn wakes(s: State, input: Input) -> bool {
    input.wake_rest
        || input.forward.abs() > 0.01
        || input.strafe.abs() > 0.01
        || input.fire
        || input.alt_fire
        || input.jump
        || input.reload
        || input.phase_shift
        || input.crouch
        || input.ultimate
        || input.dance
        || input.ability && !s.e_down
}

pub fn advance_actor(a: &mut Actor, input: Input) {
    if a.character != 12 || !a.alive() || a.combat.action_interrupted() {
        a.combat.komi = State::default();
        return;
    }
    let s = a.combat.komi;
    if !s.active() {
        return;
    }
    if s.resting() && wakes(s, input) {
        a.combat.komi = State::default();
        return;
    }
    a.combat.komi.e_down = input.ability;
    a.combat.komi.impact = (s.impact - DT).max(0.0);
    // Rest and giant are fixed wall-clock buffs; temporal fields do not extend immunity.
    a.combat.komi.elapsed += DT
        * if s.phase == Phase::Feast {
            crate::renua_skill::time_factor(a)
        } else {
            1.0
        };
    if a.combat.komi.elapsed + 0.0001 >= s.phase.duration() {
        a.combat.komi = if s.phase == Phase::Feast {
            a.race_skill.cooldown = a.race_skill.cooldown.min(LEAP_COOLDOWN);
            State {
                phase: Phase::Giant,
                ..Default::default()
            }
        } else {
            State::default()
        };
    }
}
impl World {
    pub(crate) fn start_komi(&mut self, source: usize, phase: Phase) {
        if phase == Phase::Rest {
            self.cleanse(source, true);
        }
        let a = &mut self.actors[source];
        a.combat.komi = State {
            phase,
            e_down: true,
            ..Default::default()
        };
        a.reload = 0.0;
        a.ability_flash = 0.0;
        a.combat.ultimate_flash = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.crouching = false;
        a.race_skill.cancel();
        a.portals.momentum = V3::default();
    }
    pub(crate) fn advance_komi(&mut self, source: usize, input: Input) {
        let before = self.actors[source].combat.komi;
        advance_actor(&mut self.actors[source], input);
        let after = self.actors[source].combat.komi;
        let completed = before.resting() && before.elapsed + DT + 0.0001 >= REST_SECONDS;
        if before.resting()
            && self.actors[source].character == 12
            && self.actors[source].alive()
            && !self.actors[source].combat.action_interrupted()
            && (after.resting() || completed && !wakes(before, input))
        {
            let end = if completed {
                REST_SECONDS
            } else {
                after.elapsed
            };
            let pulses = ((end + 0.0001) / REST_PULSE).floor()
                - ((before.elapsed + 0.0001) / REST_PULSE).floor();
            if pulses > 0.0 {
                self.heal_from(
                    source,
                    source,
                    REST_HEAL * REST_PULSE / REST_SECONDS * pulses,
                );
            }
        }
    }
    pub(crate) fn komi_landing(&mut self, source: usize, was_leaping: bool) {
        let a = &self.actors[source];
        if !was_leaping
            || a.race_skill.active()
            || !a.combat.komi.giant()
            || !a.alive()
            || a.combat.disabled()
            || a.vy > 0.0
            || !crate::movement::grounded(&self.map, a.pos)
        {
            return;
        }
        let origin = a.pos + V3::new(0.0, 0.5, 0.0);
        for target in self.targets(source, origin, LANDING_RADIUS, false) {
            self.damage_hit(source, target, LANDING_DAMAGE, AttackClass::Ultimate, 12);
        }
        self.actors[source].combat.komi.impact = 0.35;
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
#[derive(serde::Deserialize)]
struct Source {
    clips: Vec<Clip>,
}
pub fn hand(motion: Motion, age: f32, left: bool) -> [f32; 3] {
    if !matches!(motion, Motion::KomiRest | Motion::KomiFeast) {
        return [0.0; 3];
    }
    static SOURCE: std::sync::OnceLock<Source> = std::sync::OnceLock::new();
    let source = SOURCE.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/skill/komi.json"))
            .expect("Komi source tracks")
    });
    let samples = &source.clips[usize::from(motion == Motion::KomiFeast)].samples;
    let frame = (age * 30.0).clamp(0.0, (samples.len() - 1) as f32);
    let i = frame as usize;
    std::array::from_fn(|axis| {
        samples[i].hands[usize::from(!left)][axis] * (1.0 - frame.fract())
            + samples[(i + 1).min(samples.len() - 1)].hands[usize::from(!left)][axis]
                * frame.fract()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rest_rejects_basic_skill_ultimate_and_execution_damage() {
        let mut w = World::default();
        w.map.clear();
        w.actors[0].pos = V3::default();
        w.actors[6].pos = V3::new(0.0, 0.0, 2.0);
        w.actors[6].character = 12;
        w.actors[6].shield = 0.0;
        w.actors[6].combat.komi.phase = Phase::Rest;
        let hp = w.actors[6].hp;
        for (class, character) in [
            (AttackClass::Basic, 12),
            (AttackClass::Skill, 4),
            (AttackClass::Ultimate, 11),
            (AttackClass::Ultimate, 14),
        ] {
            assert!(!w.damage_hit(0, 6, 500.0, class, character));
            assert_eq!(w.actors[6].hp, hp);
        }
        advance_actor(
            &mut w.actors[6],
            Input {
                wake_rest: true,
                ..Default::default()
            },
        );
        assert!(w.damage_hit(0, 6, 10.0, AttackClass::Basic, 12));
        assert!(w.actors[6].hp < hp);
    }
}
