//! Naia E healing stream and source-timed Q tidal wave with temporary allied shields.
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
pub const ULTIMATE_SECONDS: f32 = 121.0 / 30.0;
pub const WAVE_RELEASE: f32 = 1.5;
pub const WAVE_SPEED: f32 = 10.0;
pub const WAVE_LIFE: f32 = 3.2;
pub const WAVE_BEHIND: f32 = 4.5;
pub const WAVE_HALF_WIDTH: f32 = 5.0;
pub const WAVE_HEIGHT: f32 = 3.8;
pub const WAVE_DEPTH: f32 = 0.7;
pub const WAVE_GUARD: f32 = 100.0;
pub const WAVE_GUARD_SECONDS: f32 = 5.0;
pub const WAVE_DAMAGE: f32 = 25.0;
pub const WAVE_PUSH: f32 = 2.0;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Start,
    Move,
    Fire,
    End,
    Ultimate,
}
impl Phase {
    pub fn from_wire(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Off,
            1 => Self::Start,
            2 => Self::Move,
            3 => Self::Fire,
            4 => Self::End,
            5 => Self::Ultimate,
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
            Self::Ultimate => Motion::NaiaUltimate,
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
        self.advance_scaled(input, 1.0);
    }
    pub fn advance_scaled(&mut self, input: Input, factor: f32) {
        if !self.active() || factor <= 0.0 {
            return;
        }
        self.elapsed += DT * factor;
        if self.phase == Phase::Ultimate {
            if self.elapsed + 0.00001 >= ULTIMATE_SECONDS {
                *self = Self::default();
            }
            return;
        }
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
            Phase::Start | Phase::Ultimate => self.elapsed,
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
    /// Swept, oriented water wall. Each target is affected once, including the caster.
    pub(crate) fn advance_naia_wave(&mut self, p: &mut crate::combat::Projectile) {
        if p.life <= 0.0 {
            return;
        }
        let dir = p.velocity.flat().normalized();
        let right = V3::new(dir.z, 0.0, -dir.x);
        let travel = WAVE_SPEED * DT;
        let center = p.pos + V3::new(0.0, WAVE_HEIGHT * 0.5, 0.0);
        let wall = self
            .map
            .iter()
            .filter_map(|b| ray_box(center, dir, b.min(), b.max()))
            .fold(travel, f32::min);
        let end = p.pos + dir * wall;
        for target in 0..self.actors.len() {
            let a = &self.actors[target];
            if !a.alive() || a.burrowed() || p.hit_mask & (1 << target) != 0 {
                continue;
            }
            let delta = a.pos - p.pos;
            let along = delta.dot(dir);
            let across = delta.dot(right);
            let radius = 0.48 * a.body_scale();
            if along < -WAVE_DEPTH - radius
                || along > wall + WAVE_DEPTH + radius
                || across.abs() > WAVE_HALF_WIDTH + radius
                || a.pos.y > p.pos.y + WAVE_HEIGHT
                || a.pos.y + a.hit_height() < p.pos.y
            {
                continue;
            }
            // Test the target's lane, so a wide crest never grants effects through cover.
            let lane = p.origin + right * across + V3::new(0.0, a.eye().y - p.origin.y, 0.0);
            if !self.visible_between(lane, a.eye()) {
                continue;
            }
            p.hit_mask |= 1 << target;
            if team(target) == team(p.owner) {
                self.add_guard(target, WAVE_GUARD, WAVE_GUARD_SECONDS);
            } else if self.damage_hit(p.owner, target, p.damage, p.class, p.character) {
                self.push(target, dir, WAVE_PUSH);
            }
        }
        p.pos = end;
        if wall < travel {
            p.life = 0.0;
        }
    }
    pub(crate) fn advance_naia_skill(&mut self, i: usize, input: Input) {
        let a = &mut self.actors[i];
        if a.character != 1 || !a.alive() || a.combat.action_interrupted() || a.burrowed() {
            a.combat.naia = State::default();
            return;
        }
        let previous = a.combat.naia;
        a.combat.naia.advance_scaled(input, crate::renua_skill::time_factor(a));
        if previous.phase == Phase::Ultimate
            && previous.elapsed + 0.00001 < WAVE_RELEASE
            && a.combat.naia.elapsed + 0.00001 >= WAVE_RELEASE
        {
            let dir = direction(a.yaw, 0.0);
            let origin = a.pos - dir * WAVE_BEHIND;
            if let Some(index) = self.projectile(
                i,
                crate::combat::ProjectileKind::Wave,
                dir,
                WAVE_SPEED,
                WAVE_DAMAGE,
                crate::combat::AttackClass::Ultimate,
            ) {
                let p = &mut self.projectiles[index];
                p.origin = origin;
                p.pos = origin;
                p.life = WAVE_LIFE;
            }
        }
    }
    pub(crate) fn fire_naia_stream(&mut self, source: usize) {
        let a = &self.actors[source];
        if a.combat.naia.phase != Phase::Fire || a.combat.disabled() {
            return;
        }
        let origin = a.eye();
        let dir = direction(a.yaw, a.pitch);
        let end = stream_end(&self.map, a);
        let distance = (end - origin).length();
        let action_dt = DT * crate::renua_skill::time_factor(a);
        self.actors[source].shot_end = end;
        let state = &mut self.actors[source].combat.naia;
        state.pulse -= action_dt;
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
            self.heal_from(source, target, HEAL_PER_SECOND * PULSE_SECONDS);
            self.actors[target].combat.stun = 0.0;
        }
    }
}

/// Source samples shared by the body, both viewmodel hands and the whale school.
#[derive(serde::Deserialize)]
struct Sample {
    hands: [[f32; 3]; 2],
    whales: [[f32; 4]; 3],
}
#[derive(serde::Deserialize)]
struct Clip {
    samples: Vec<Sample>,
}
#[derive(serde::Deserialize)]
struct Data {
    clips: Vec<Clip>,
}
fn samples(motion: Motion, age: f32) -> Option<(&'static Sample, &'static Sample, f32)> {
    static DATA: std::sync::OnceLock<Data> = std::sync::OnceLock::new();
    let index = match motion {
        Motion::NaiaStart => 0,
        Motion::NaiaMove => 1,
        Motion::NaiaFire => 2,
        Motion::NaiaEnd => 3,
        Motion::NaiaUltimate => 4,
        _ => return None,
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
    Some((&samples[a], &samples[b], at - a as f32))
}
pub fn hand_path(motion: Motion, age: f32, left: bool) -> [f32; 3] {
    let Some((a, b, t)) = samples(motion, age) else {
        return [0.0; 3];
    };
    // Coordinate-side L is anatomical right in the shipped rig, exactly as in the bake.
    let i = usize::from(left);
    std::array::from_fn(|j| a.hands[i][j] * (1.0 - t) + b.hands[i][j] * t)
}
pub fn whale_path(age: f32, index: usize) -> [f32; 4] {
    let (a, b, t) = samples(Motion::NaiaUltimate, age).unwrap();
    let i = index % 3;
    std::array::from_fn(|j| {
        if j == 2 {
            crate::motion::interpolate_angle(a.whales[i][j], b.whales[i][j], t)
        } else {
            a.whales[i][j] * (1.0 - t) + b.whales[i][j] * t
        }
    })
}
pub fn whale_form(motion: Motion, age: f32) -> f32 {
    let smooth = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    match motion {
        Motion::NaiaStart => smooth(age / 0.45),
        Motion::NaiaMove | Motion::NaiaFire | Motion::NaiaUltimate => 1.0,
        Motion::NaiaEnd => smooth((END_SECONDS - age) / 0.4),
        _ => 0.0,
    }
}
pub fn hand(motion: Motion, age: f32) -> crate::attack::HandMotion {
    let path = hand_path(motion, age, false);
    crate::attack::HandMotion {
        position: [
            0.0,
            (path[1] * 0.23).clamp(-0.12, 0.18),
            (-path[0] * 0.15).clamp(-0.12, 0.16),
        ],
        // E keeps the muzzle aligned with aim while the hands follow the source bob.
        angles: if motion == Motion::NaiaUltimate {
            [(-path[2] * 0.22).clamp(-0.45, 0.45), 0.0, 0.0]
        } else {
            [0.0; 3]
        },
    }
}
