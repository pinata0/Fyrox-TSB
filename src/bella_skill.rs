//! Bella: source-timed violet fire, skill-locking taunt and a lantern healing finale.
use crate::{
    combat::{AttackClass, Projectile, ProjectileKind},
    game::{direction, team, Actor, World, DT, PLAYERS, V3},
    motion::Motion,
};

pub const SKILL_SECONDS: f32 = 100.0 / 30.0;
pub const ULTIMATE_SECONDS: f32 = 150.0 / 30.0;
pub const FIRE_RELEASE: f32 = 58.0 / 30.0;
pub const TAUNT_RELEASE: f32 = 64.0 / 30.0;
pub const TAUNT_SECONDS: f32 = 3.0;
pub const FIRE_LENGTH: f32 = 10.0;
pub const FIRE_WIDTH: f32 = 3.0;
pub const FIRE_SECONDS: f32 = 4.0;
pub const FIRE_INTERVAL: f32 = 0.5;
pub const FIRE_DAMAGE: f32 = 6.0;
pub const HEAL_RADIUS: f32 = 8.0;
pub const HEAL_FRACTION: f32 = 0.40;
pub const WAVE_SECONDS: f32 = 0.9;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Skill,
    Ultimate,
}
impl Phase {
    pub fn from_wire(v: u8) -> Option<Self> {
        Some(match v {
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
            Self::Skill => Some(Motion::BellaSkill),
            Self::Ultimate => Some(Motion::BellaUltimate),
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub elapsed: f32,
    pub targets: u16,
}
impl State {
    pub fn active(self) -> bool {
        self.phase != Phase::Off
    }
    pub fn pack(self) -> u32 {
        if !self.active() {
            return 0;
        }
        ((self.elapsed / DT).round() as u32).min((self.phase.duration() / DT).round() as u32 - 1)
            | ((self.targets as u32) << 9)
    }
    pub fn unpack(phase: Phase, packed: u32) -> Option<Self> {
        let elapsed = (packed & 511) as f32 * DT;
        let mask = packed >> 9;
        if mask >= 1 << PLAYERS || (phase != Phase::Ultimate && mask != 0) {
            return None;
        }
        ((phase == Phase::Off && packed == 0)
            || (phase != Phase::Off && elapsed < phase.duration()))
        .then_some(Self {
            phase,
            elapsed,
            targets: mask as u16,
        })
    }
}
pub fn interrupted(a: &Actor) -> bool {
    a.character != 9
        || !a.alive()
        || a.combat.action_interrupted()
        || a.combat.taunt > 0.0
        || a.burrowed()
}
pub fn advance_actor(a: &mut Actor) {
    if interrupted(a) {
        a.combat.bella = State::default();
    } else if a.combat.bella.active() {
        a.combat.bella.elapsed += DT * crate::renua_skill::time_factor(a);
        if a.combat.bella.elapsed + 0.00001 >= a.combat.bella.phase.duration() {
            a.combat.bella = State::default();
        }
    }
}
/// The oriented rectangle is fixed at release. Height and occlusion exclude other floors/cover.
pub fn in_fire(map: &[crate::game::Block], p: &Projectile, a: &Actor) -> bool {
    let forward = p.velocity.flat().normalized();
    let right = V3::new(forward.z, 0.0, -forward.x);
    let d = a.pos - p.pos;
    let point = a.pos + V3::new(0.0, 0.8, 0.0);
    let from = p.pos + V3::new(0.0, 0.8, 0.0);
    let ray = point - from;
    a.alive()
        && !a.burrowed()
        && d.y.abs() <= 1.5
        && (0.0..=FIRE_LENGTH).contains(&d.dot(forward))
        && d.dot(right).abs() <= FIRE_WIDTH * 0.5
        && !map.iter().any(|b| {
            crate::game::ray_box(from, ray.normalized(), b.min(), b.max())
                .is_some_and(|t| t < ray.length() - 0.02)
        })
}
impl World {
    pub(crate) fn start_bella(&mut self, source: usize, phase: Phase) {
        let a = &mut self.actors[source];
        a.combat.bella = State {
            phase,
            ..Default::default()
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
    pub(crate) fn advance_bella(&mut self, source: usize) {
        let before = self.actors[source].combat.bella;
        let canceled = interrupted(&self.actors[source]);
        let elapsed = before.elapsed + DT * crate::renua_skill::time_factor(&self.actors[source]);
        advance_actor(&mut self.actors[source]);
        if canceled || !before.active() {
            return;
        }
        let crossed = |t| before.elapsed + 0.00001 < t && elapsed + 0.00001 >= t;
        if before.phase == Phase::Skill && crossed(FIRE_RELEASE) {
            let a = &self.actors[source];
            let origin = a.pos;
            let forward = direction(a.yaw, 0.0);
            if let Some(i) = self.projectile(
                source,
                ProjectileKind::BellaFire,
                forward,
                1.0,
                FIRE_DAMAGE,
                AttackClass::Skill,
            ) {
                let p = &mut self.projectiles[i];
                p.pos = origin;
                p.origin = origin;
                p.velocity = forward;
                p.life = FIRE_SECONDS;
            }
        }
        if before.phase == Phase::Ultimate && crossed(TAUNT_RELEASE) {
            let targets: Vec<_> = (0..PLAYERS)
                .filter(|&i| {
                    team(i) != team(source)
                        && self.actors[i].alive()
                        && crate::ui_skill::in_view(
                            &self.map,
                            &self.actors[source],
                            &self.actors[i],
                        )
                })
                .collect();
            let mut mask = 0;
            for i in targets {
                if self.apply_taunt(source, i, TAUNT_SECONDS) {
                    mask |= 1 << i;
                }
            }
            self.actors[source].combat.bella.targets = mask;
        }
        // Completion is checked against the pre-reset clock: interrupted casts never heal.
        if before.phase == Phase::Ultimate && crossed(ULTIMATE_SECONDS) {
            let origin = self.actors[source].pos;
            for i in 0..PLAYERS {
                if team(i) == team(source)
                    && self.actors[i].alive()
                    && !self.actors[i].burrowed()
                    && (self.actors[i].pos - origin).length() <= HEAL_RADIUS
                {
                    self.heal_from(source, i, self.actors[i].max_hp() as f32 * HEAL_FRACTION);
                }
            }
            if let Some(i) = self.projectile(
                source,
                ProjectileKind::BellaHeal,
                V3::default(),
                0.0,
                0.0,
                AttackClass::Ultimate,
            ) {
                let p = &mut self.projectiles[i];
                p.pos = origin;
                p.origin = origin;
                p.life = WAVE_SECONDS;
            }
        }
    }
    pub(crate) fn advance_bella_fire(&mut self, p: &mut Projectile) {
        let pulse = ((p.age.min(FIRE_SECONDS) + 0.00001) / FIRE_INTERVAL).floor() as u8;
        let tick = pulse > p.step;
        p.step = pulse;
        for i in 0..PLAYERS {
            if team(i) == team(p.owner) || !in_fire(&self.map, p, &self.actors[i]) {
                continue;
            }
            // Three seconds per successful application, not a continuously refreshed silence.
            if p.hit_mask & (1 << i) == 0 && self.apply_taunt(p.owner, i, TAUNT_SECONDS) {
                p.hit_mask |= 1 << i;
            }
            if tick {
                self.damage_hit(p.owner, i, p.damage, p.class, 9);
            }
        }
    }
}
#[derive(serde::Deserialize)]
struct Sample {
    hands: [[f32; 3]; 2],
    lantern: [f32; 3],
}
#[derive(serde::Deserialize)]
struct Clip {
    samples: Vec<Sample>,
}
#[derive(serde::Deserialize)]
struct Data {
    clips: Vec<Clip>,
}
pub fn hand(m: Motion, age: f32, left: bool) -> [f32; 3] {
    sample(m, age, |s| s.hands[usize::from(!left)])
}
pub fn lantern(m: Motion, age: f32) -> [f32; 3] {
    sample(m, age, |s| s.lantern)
}
fn sample(m: Motion, age: f32, field: impl Fn(&Sample) -> [f32; 3]) -> [f32; 3] {
    if !matches!(m, Motion::BellaSkill | Motion::BellaUltimate) {
        return [0.0; 3];
    }
    static DATA: std::sync::OnceLock<Data> = std::sync::OnceLock::new();
    let d = DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/skill/bella.json"))
            .expect("Bella source tracks")
    });
    let s = &d.clips[usize::from(m == Motion::BellaUltimate)].samples;
    let f = (age * 30.0).clamp(0.0, (s.len() - 1) as f32);
    let i = f as usize;
    let a = field(&s[i]);
    let b = field(&s[(i + 1).min(s.len() - 1)]);
    std::array::from_fn(|j| a[j] + (b[j] - a[j]) * f.fract())
}
