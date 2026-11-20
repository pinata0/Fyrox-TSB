//! Ui's source-timed clover rain and visible-target frog transformation.
use crate::{
    combat::{AttackClass, Projectile, ProjectileKind},
    game::{direction, team, Actor, World, DT, PLAYERS, V3},
    motion::Motion,
};

pub const SKILL_SECONDS: f32 = 65.0 / 30.0;
pub const ULTIMATE_SECONDS: f32 = 127.0 / 30.0;
pub const RAIN_RELEASE: f32 = 22.0 / 30.0;
pub const ULTIMATE_RELEASE: f32 = 29.0 / 30.0;
pub const RAIN_SECONDS: f32 = 4.0;
pub const RAIN_RADIUS: f32 = 7.0;
pub const RAIN_INTERVAL: f32 = 0.5;
pub const RAIN_DAMAGE: f32 = 3.0;
pub const RAIN_HEAL: f32 = 4.0;
pub const RAIN_SPEED: f32 = 0.85;
pub const MORPH_SECONDS: f32 = 3.0;

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
            Self::Skill => Some(Motion::UiSkill),
            Self::Ultimate => Some(Motion::UiUltimate),
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
        ((phase == Phase::Off && ticks == 0) || (phase != Phase::Off && elapsed < phase.duration()))
            .then_some(Self { phase, elapsed })
    }
}
pub fn advance_actor(a: &mut Actor) {
    if a.character != 10 || !a.alive() || a.combat.action_interrupted() || a.burrowed() {
        a.combat.ui = State::default();
    } else if a.combat.ui.active() {
        a.combat.ui.elapsed += DT * crate::renua_skill::time_factor(a);
        if a.combat.ui.elapsed + 0.00001 >= a.combat.ui.phase.duration() {
            a.combat.ui = State::default();
        }
    }
}

/// The shared gameplay view: 80 degrees vertically, the caster's viewport horizontally.
/// Samples the target's head and torso; cover can hide either independently.
pub fn in_view(map: &[crate::game::Block], source: &Actor, target: &Actor) -> bool {
    let forward = direction(source.yaw, source.pitch);
    let right = V3::new(source.yaw.cos(), 0.0, -source.yaw.sin());
    let up = V3::new(
        -source.yaw.sin() * source.pitch.sin(),
        source.pitch.cos(),
        -source.yaw.cos() * source.pitch.sin(),
    );
    [
        target.eye(),
        target.pos + V3::new(0.0, target.hit_height() * 0.5, 0.0),
    ]
    .into_iter()
    .any(|point| {
        let d = point - source.eye();
        let depth = d.dot(forward);
        let half = 40.0_f32.to_radians().tan();
        depth > 0.01
            && d.dot(right).abs() <= depth * half * source.view_aspect
            && d.dot(up).abs() <= depth * half
            && clear(map, source.eye(), point)
    })
}
fn clear(map: &[crate::game::Block], from: V3, to: V3) -> bool {
    let d = to - from;
    !map.iter().any(|b| {
        crate::game::ray_box(from, d.normalized(), b.min(), b.max())
            .is_some_and(|t| t < d.length() - 0.02)
    })
}
pub fn in_rain(map: &[crate::game::Block], p: &Projectile, a: &Actor) -> bool {
    a.alive()
        && !a.burrowed()
        && (a.pos - p.pos).flat().length() <= RAIN_RADIUS
        && (a.pos.y - p.pos.y).abs() <= 2.5
        && clear(
            map,
            p.pos + V3::new(0.0, 0.8, 0.0),
            a.pos + V3::new(0.0, 0.8, 0.0),
        )
}
pub fn predict_rain(map: &[crate::game::Block], fields: &[Projectile], slot: usize, a: &mut Actor) {
    a.combat.rain_slow = if a.shield <= 0.0
        && a.combat.sleep <= 0.0
        && !a.combat.komi.resting()
        && fields.iter().any(|p| {
            p.kind == ProjectileKind::RainField
                && p.life > 0.0
                && team(p.owner) != team(slot)
                && in_rain(map, p, a)
        }) {
        DT * 2.0
    } else {
        0.0
    };
}
impl World {
    pub(crate) fn start_ui(&mut self, source: usize, phase: Phase) {
        let a = &mut self.actors[source];
        a.combat.ui = State {
            phase,
            elapsed: 0.0,
        };
        a.reload = 0.0;
        a.shield = 0.0;
        a.ability_flash = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        if phase == Phase::Ultimate {
            a.combat.ultimate_flash = ULTIMATE_SECONDS;
        }
    }
    pub(crate) fn advance_ui(&mut self, source: usize) {
        let before = self.actors[source].combat.ui;
        advance_actor(&mut self.actors[source]);
        let after = self.actors[source].combat.ui;
        if before.phase != after.phase || !after.active() {
            return;
        }
        let crossed = |t| before.elapsed + 0.00001 < t && after.elapsed + 0.00001 >= t;
        if after.phase == Phase::Skill && crossed(RAIN_RELEASE) {
            let origin = self.actors[source].pos;
            if let Some(i) = self.projectile(
                source,
                ProjectileKind::RainField,
                V3::default(),
                0.0,
                RAIN_DAMAGE,
                AttackClass::Skill,
            ) {
                let p = &mut self.projectiles[i];
                p.pos = origin;
                p.origin = origin;
                p.life = RAIN_SECONDS;
            }
        } else if after.phase == Phase::Ultimate && crossed(ULTIMATE_RELEASE) {
            // Snapshot both lists before changing anybody's form or camera height.
            let targets: Vec<_> = (0..PLAYERS)
                .filter(|&i| {
                    i != source
                        && self.actors[i].alive()
                        && !self.actors[i].burrowed()
                        && in_view(&self.map, &self.actors[source], &self.actors[i])
                })
                .collect();
            for &i in &targets {
                if team(i) != team(source)
                    && !self.in_spawn(i)
                    && self.actors[i].shield <= 0.0
                    && self.actors[i].combat.sleep <= 0.0
                    && !self.actors[i].combat.komi.resting()
                {
                    self.morph_ui_target(i);
                }
            }
            for i in targets {
                if team(i) == team(source) {
                    self.actors[i].ability_cooldown = 0.0;
                }
            }
        }
    }
    fn morph_ui_target(&mut self, i: usize) {
        let a = &mut self.actors[i];
        // End phasing using its ordinary safe ejection, including an overlapping wall.
        if a.phase_skill.active() {
            a.phase_skill.remaining = DT;
            crate::phase_skill::advance(&self.map, a, false);
        }
        a.combat.polymorph = a.combat.polymorph.max(MORPH_SECONDS);
        a.combat.cast = None;
        a.combat.dash = 0.0;
        a.combat.ui = Default::default();
        a.combat.chloe = Default::default();
        a.combat.vivi = Default::default();
        a.combat.picora = Default::default();
        a.combat.mayo = Default::default();
        a.combat.bella = Default::default();
        a.combat.naia = Default::default();
        a.combat.id.start(crate::id_skill::Phase::Off);
        a.combat.renua.start(crate::renua_skill::Phase::Off);
        a.combat.komi = Default::default();
        a.combat.butter = Default::default();
        a.combat.hilde = Default::default();
        a.combat.shion = Default::default();
        a.combat.aya = Default::default();
        a.combat.erpin.eating = false;
        a.combat.erpin.shot_delay = 0.0;
        a.combat.magic_ammo = 0;
        a.reload = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.race_skill.cancel();
        a.portals.momentum = V3::default();
    }
    pub(crate) fn advance_ui_rain(&mut self, p: &mut Projectile) {
        let pulse = (p.age.min(RAIN_SECONDS) + 0.00001) / RAIN_INTERVAL;
        let pulse = pulse.floor() as u8;
        let tick = pulse > p.step;
        p.step = pulse;
        for i in 0..PLAYERS {
            if !in_rain(&self.map, p, &self.actors[i]) {
                continue;
            }
            if team(i) == team(p.owner) {
                if tick {
                    self.heal_from(p.owner, i, RAIN_HEAL);
                }
            } else if !self.in_spawn(i) {
                let a = &mut self.actors[i];
                if a.shield <= 0.0
                    && a.combat.sleep <= 0.0
                    && !a.combat.komi.resting()
                    && p.life > 0.00001
                {
                    a.combat.rain_slow = DT * 2.0;
                }
                if tick {
                    self.damage_hit(p.owner, i, p.damage, p.class, 10);
                }
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct Sample {
    pub clover: [f32; 3],
    pub hands: [[f32; 3]; 2],
}
#[derive(serde::Deserialize)]
struct Clip {
    samples: Vec<Sample>,
}
#[derive(serde::Deserialize)]
struct Data {
    clips: Vec<Clip>,
}
pub fn sample(motion: Motion, age: f32) -> Option<(&'static Sample, &'static Sample, f32)> {
    if !matches!(motion, Motion::UiSkill | Motion::UiUltimate) {
        return None;
    }
    static DATA: std::sync::OnceLock<Data> = std::sync::OnceLock::new();
    let data = DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/skill/ui.json"))
            .expect("Ui source tracks")
    });
    let samples = &data.clips[usize::from(motion == Motion::UiUltimate)].samples;
    let frame = (age * 30.0).clamp(0.0, (samples.len() - 1) as f32);
    let i = frame as usize;
    Some((
        &samples[i],
        &samples[(i + 1).min(samples.len() - 1)],
        frame.fract(),
    ))
}
pub fn clover(motion: Motion, age: f32) -> [f32; 3] {
    let Some((a, b, t)) = sample(motion, age) else {
        return [0.0, 1.0, 1.0];
    };
    std::array::from_fn(|i| a.clover[i] + (b.clover[i] - a.clover[i]) * t)
}
pub fn hand(motion: Motion, age: f32) -> crate::attack::HandMotion {
    let Some((a, b, t)) = sample(motion, age) else {
        return Default::default();
    };
    let p: [f32; 3] = std::array::from_fn(|i| a.hands[1][i] + (b.hands[1][i] - a.hands[1][i]) * t);
    crate::attack::HandMotion {
        position: [
            -0.10,
            (p[1] * 0.08).clamp(-0.06, 0.1),
            (p[0] * 0.05).clamp(-0.06, 0.06),
        ],
        angles: [p[2].sin() * 0.15, 0.0, 0.12],
    }
}
