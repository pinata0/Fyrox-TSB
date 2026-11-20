//! Archived Picora gestures, aimed sleeve stickers and temporary maximum health.
use crate::{
    combat::{AttackClass, ProjectileKind},
    game::{direction, ray_box, team, Actor, World, DT, MAX_HP, PLAYERS, V3},
    motion::Motion,
};

pub const SKILL_SECONDS: f32 = 66.0 / 30.0;
pub const ULTIMATE_SECONDS: f32 = 60.0 / 30.0;
pub const STICKER_RELEASE: f32 = 25.0 / 30.0;
pub const ULTIMATE_RELEASE: f32 = 13.0 / 30.0;
pub const STICKER_SECONDS: f32 = 4.0;
pub const FASHION_SECONDS: f32 = 8.0;
pub const STICKER_HEAL: f32 = 44.0;
pub const DAMAGE_FACTOR: f32 = 0.75;
pub const RANGE: f32 = 18.0;

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
            Self::Skill => Some(Motion::PicoraSkill),
            Self::Ultimate => Some(Motion::PicoraUltimate),
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub elapsed: f32,
    pub target: Option<usize>,
    pub shot_delay: f32,
}
impl State {
    pub fn active(self) -> bool {
        self.phase != Phase::Off
    }
    pub fn pack(self) -> u32 {
        let age = if self.active() {
            (self.elapsed / DT)
                .round()
                .min(self.phase.duration() / DT - 1.0) as u32
        } else {
            0
        };
        age | (self.target.map_or(0, |i| i as u32 + 1) << 8)
            | (((self.shot_delay / DT).ceil() as u32).min(255) << 12)
    }
    pub fn unpack(phase: Phase, bits: u32) -> Option<Self> {
        let age = (bits & 255) as f32 * DT;
        let target = ((bits >> 8) & 15) as usize;
        let shot_delay = ((bits >> 12) & 255) as f32 * DT;
        if bits >> 20 != 0
            || target > PLAYERS
            || shot_delay > 0.5
            || (phase == Phase::Off && (age != 0.0 || target != 0))
            || (phase != Phase::Off
                && (age >= phase.duration() || target == 0 || shot_delay != 0.0))
        {
            return None;
        }
        Some(Self {
            phase,
            elapsed: age,
            target: target.checked_sub(1),
            shot_delay,
        })
    }
}
pub fn shot_delay(a: &Actor) -> f32 {
    // Attack1_1's frame-18 event, retimed with the existing 47-frame basic clip.
    crate::attack::seconds(5) * 18.0 / 47.0 / crate::attack::rate(a)
}
pub fn advance_actor(a: &mut Actor) {
    if a.character != 5
        || !a.alive()
        || a.combat.action_interrupted()
        || a.burrowed()
        || (a.combat.picora.active() && a.combat.skills_locked())
    {
        a.combat.picora = State::default();
        return;
    }
    let dt = DT * crate::renua_skill::time_factor(a);
    let s = &mut a.combat.picora;
    s.shot_delay = (s.shot_delay - dt).max(0.0);
    if s.active() {
        s.elapsed += dt;
        if s.elapsed + 0.00001 >= s.phase.duration() {
            *s = State::default();
        }
    }
}
pub fn advance_buffs(a: &mut Actor) {
    a.combat.sticker = (a.combat.sticker - DT).max(0.0);
    let expired = a.combat.fashion > 0.0 && a.combat.fashion <= DT + 0.00001;
    a.combat.fashion = (a.combat.fashion - DT).max(0.0);
    if expired {
        a.combat.fashion = 0.0;
        // Preserve the health percentage when the temporary capacity is removed.
        a.hp = a.hp.div_ceil(2).min(MAX_HP);
        a.barrier = a.barrier.min(MAX_HP as f32 * 0.08);
    }
}
impl World {
    /// Select the first visible ally intersecting the crosshair, including vertical aim.
    pub fn picora_target(&self, source: usize) -> Option<usize> {
        let a = &self.actors[source];
        let dir = direction(a.yaw, a.pitch);
        let wall = self
            .map
            .iter()
            .filter_map(|b| ray_box(a.eye(), dir, b.min(), b.max()))
            .fold(RANGE, f32::min);
        self.actors
            .iter()
            .enumerate()
            .filter(|(i, b)| *i != source && team(*i) == team(source) && b.alive() && !b.burrowed())
            .filter_map(|(i, b)| {
                let r = 0.48 * b.body_scale();
                ray_box(
                    a.eye(),
                    dir,
                    b.pos - V3::new(r, 0.0, r),
                    b.pos + V3::new(r, b.hit_height(), r),
                )
                .filter(|&t| t >= 0.0 && t < wall)
                .map(|t| (i, t))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|x| x.0)
    }
    pub(crate) fn start_picora(&mut self, source: usize, phase: Phase) {
        let target = self.picora_target(source);
        let a = &mut self.actors[source];
        a.combat.picora = State {
            phase,
            target,
            ..Default::default()
        };
        a.reload = 0.0;
        a.ability_flash = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        if phase == Phase::Ultimate {
            a.combat.ultimate_flash = ULTIMATE_SECONDS;
        }
    }
    pub(crate) fn advance_picora(&mut self, source: usize) {
        let before = self.actors[source].combat.picora;
        advance_actor(&mut self.actors[source]);
        let after = self.actors[source].combat.picora;
        if before.shot_delay > 0.0
            && after.shot_delay == 0.0
            && self.actors[source].alive()
            && !self.actors[source].combat.action_interrupted()
            && !self.actors[source].burrowed()
        {
            let a = &self.actors[source];
            self.projectile(
                source,
                ProjectileKind::Bolt,
                direction(a.yaw, a.pitch),
                24.0,
                crate::kits::kit(5).damage,
                AttackClass::Basic,
            );
        }
        if !after.active() || before.phase != after.phase {
            return;
        }
        let release = if after.phase == Phase::Skill {
            STICKER_RELEASE
        } else {
            ULTIMATE_RELEASE
        };
        if before.elapsed + 0.00001 >= release || after.elapsed + 0.00001 < release {
            return;
        }
        let Some(target) = after.target else {
            return;
        };
        let a = &self.actors[target];
        if !a.alive()
            || a.burrowed()
            || team(target) != team(source)
            || (a.eye() - self.actors[source].eye()).length() > RANGE
            || !self.visible_between(self.actors[source].eye(), a.eye())
        {
            return;
        }
        if after.phase == Phase::Skill {
            self.heal_from(source, target, STICKER_HEAL);
            self.actors[target].combat.sticker = STICKER_SECONDS;
        } else {
            let a = &mut self.actors[target];
            if a.combat.fashion <= 0.0 {
                a.hp = a.hp.saturating_mul(2).min(MAX_HP * 2);
            }
            a.combat.fashion = FASHION_SECONDS;
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
#[derive(serde::Deserialize)]
struct Data {
    clips: Vec<Clip>,
}
pub fn hand(motion: Motion, age: f32, first_person: bool) -> crate::attack::HandMotion {
    // Q is intentionally stationary in first person; the staff flash carries the cast.
    if !matches!(motion, Motion::PicoraSkill | Motion::PicoraUltimate)
        || (first_person && motion == Motion::PicoraUltimate)
    {
        return Default::default();
    }
    let p = hand_path(motion, age, false);
    let weight = (age / 0.15).clamp(0.0, 1.0) * ((motion.seconds(5) - age) / 0.2).clamp(0.0, 1.0);
    crate::attack::HandMotion {
        position: [
            0.0,
            (p[1] * 0.13).clamp(-0.12, 0.16) * weight,
            (p[0] * 0.10).clamp(-0.12, 0.12) * weight,
        ],
        angles: [p[2].sin() * 0.3 * weight, 0.0, -p[2].sin() * 0.12 * weight],
    }
}

pub fn hand_path(motion: Motion, age: f32, left: bool) -> [f32; 3] {
    if !matches!(motion, Motion::PicoraSkill | Motion::PicoraUltimate) {
        return [0.0; 3];
    }
    static DATA: std::sync::OnceLock<Data> = std::sync::OnceLock::new();
    let data = DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/skill/picora.json"))
            .expect("Picora source tracks")
    });
    let samples = &data.clips[usize::from(motion == Motion::PicoraUltimate)].samples;
    let f = (age * 30.0).clamp(0.0, (samples.len() - 1) as f32);
    let a = &samples[f as usize];
    let b = &samples[(f as usize + 1).min(samples.len() - 1)];
    let p: [f32; 3] = std::array::from_fn(|i| {
        a.hands[usize::from(left)][i]
            + (b.hands[usize::from(left)][i] - a.hands[usize::from(left)][i]) * f.fract()
    });
    p
}
