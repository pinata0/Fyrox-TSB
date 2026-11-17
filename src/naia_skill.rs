//! Naia E: source-timed wind-up, mobile stance, held healing stream, recovery.
use crate::game::{direction, ray_box, team, Actor, Input, World, DT, V3};
use crate::motion::Motion;

pub const START_SECONDS: f32 = 44.0 / 30.0;
pub const ACTIVE_SECONDS: f32 = 3.0;
pub const END_SECONDS: f32 = 27.0 / 30.0;
pub const TOTAL_SECONDS: f32 = START_SECONDS + ACTIVE_SECONDS + END_SECONDS;
pub const SPEED_MULTIPLIER: f32 = 1.3;
pub const RANGE: f32 = 15.0;
pub const PULSE_SECONDS: f32 = 0.1;
pub const HEAL_PER_SECOND: f32 = 60.0;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Start,
    Move,
    Fire,
    End,
}
impl Phase {
    pub fn from_wire(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Off,
            1 => Self::Start,
            2 => Self::Move,
            3 => Self::Fire,
            4 => Self::End,
            _ => return None,
        })
    }
    pub fn motion(self) -> Option<Motion> {
        Some(match self {
            Self::Off => return None,
            Self::Start => Motion::NaiaStart,
            Self::Move => Motion::NaiaMove,
            Self::Fire => Motion::NaiaFire,
            Self::End => Motion::NaiaEnd,
        })
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    /// Shared absolute skill clock; independent of cooldown reductions.
    pub elapsed: f32,
    pub pulse: f32,
}
impl State {
    pub fn active(self) -> bool {
        self.phase != Phase::Off
    }
    pub fn start(&mut self) {
        *self = Self {
            phase: Phase::Start,
            ..Self::default()
        };
    }
    pub fn advance(&mut self, input: Input) {
        if !self.active() {
            return;
        }
        self.elapsed += DT;
        self.phase = if self.elapsed + 0.00001 >= TOTAL_SECONDS {
            Phase::Off
        } else if self.elapsed + 0.00001 >= START_SECONDS + ACTIVE_SECONDS {
            Phase::End
        } else if self.elapsed + 0.00001 < START_SECONDS {
            Phase::Start
        } else if input.fire {
            Phase::Fire
        } else {
            Phase::Move
        };
        if !self.active() {
            *self = Self::default();
        }
    }
    pub fn motion_age(self) -> f32 {
        match self.phase {
            Phase::Start => self.elapsed,
            Phase::Move | Phase::Fire => (self.elapsed - START_SECONDS).max(0.0),
            Phase::End => (self.elapsed - START_SECONDS - ACTIVE_SECONDS).max(0.0),
            Phase::Off => 0.0,
        }
    }
}

/// Shared by authority and rendering: the stream ends at the first solid surface.
pub fn stream_end(map: &[crate::game::Block], actor: &Actor) -> V3 {
    let origin = actor.eye();
    let dir = direction(actor.yaw, actor.pitch);
    let distance = map
        .iter()
        .filter_map(|b| ray_box(origin, dir, b.min(), b.max()))
        .fold(RANGE, f32::min);
    origin + dir * distance.max(0.0)
}
impl World {
    pub(crate) fn advance_naia_skill(&mut self, i: usize, input: Input) {
        let a = &mut self.actors[i];
        if a.character != 1 || !a.alive() || a.combat.disabled() {
            a.combat.naia = State::default();
            return;
        }
        a.combat.naia.advance(input);
    }
    pub(crate) fn fire_naia_stream(&mut self, source: usize) {
        let a = &self.actors[source];
        if a.combat.naia.phase != Phase::Fire {
            return;
        }
        let origin = a.eye();
        let dir = direction(a.yaw, a.pitch);
        let end = stream_end(&self.map, a);
        let distance = (end - origin).length();
        self.actors[source].shot_end = end;
        let state = &mut self.actors[source].combat.naia;
        state.pulse -= DT;
        if state.pulse > 0.0 {
            return;
        }
        state.pulse += PULSE_SECONDS;
        // Heal the nearest ally actually intersecting the directed water, never behind a wall.
        let target = self
            .actors
            .iter()
            .enumerate()
            .filter(|(i, a)| *i != source && team(*i) == team(source) && a.alive())
            .filter_map(|(i, a)| {
                let center = a.pos + V3::new(0.0, a.collision_height() * 0.55, 0.0);
                let along = (center - origin).dot(dir);
                let nearest = origin + dir * along;
                (along > 0.0
                    && along < distance
                    && (center - nearest).length() < 0.95
                    && self.visible_between(origin, a.eye()))
                .then_some((i, along))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|t| t.0);
        if let Some(target) = target {
            self.heal(target, HEAL_PER_SECOND * PULSE_SECONDS);
            self.actors[target].combat.stun = 0.0;
        }
    }
}

/// First-person weapon follows the same sampled wrist gesture as the body.
pub fn hand(motion: Motion, age: f32) -> crate::attack::HandMotion {
    use serde::Deserialize;
    use std::sync::OnceLock;
    #[derive(Deserialize)]
    struct Sample {
        hands: [[f32; 3]; 2],
    }
    #[derive(Deserialize)]
    struct Clip {
        samples: Vec<Sample>,
    }
    #[derive(Deserialize)]
    struct Data {
        clips: Vec<Clip>,
    }
    static DATA: OnceLock<Data> = OnceLock::new();
    let index = match motion {
        Motion::NaiaStart => 0,
        Motion::NaiaMove => 1,
        Motion::NaiaFire => 2,
        Motion::NaiaEnd => 3,
        _ => return Default::default(),
    };
    let data = DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/skill/naia.json"))
            .expect("Naia skill samples")
    });
    let samples = &data.clips[index].samples;
    let duration = motion.seconds(1);
    let phase = if motion.looping() {
        age.rem_euclid(duration) / duration
    } else {
        (age / duration).clamp(0.0, 1.0)
    };
    let at = phase * (samples.len() - 1) as f32;
    let a = at as usize;
    let b = (a + 1).min(samples.len() - 1);
    let t = at - a as f32;
    let path: [f32; 3] =
        std::array::from_fn(|j| samples[a].hands[0][j] * (1.0 - t) + samples[b].hands[0][j] * t);
    crate::attack::HandMotion {
        position: [
            0.0,
            (path[1] * 0.23).clamp(-0.12, 0.18),
            (-path[0] * 0.15).clamp(-0.12, 0.16),
        ],
        angles: [(-path[2] * 0.22).clamp(-0.45, 0.45), 0.0, 0.0],
    }
}
