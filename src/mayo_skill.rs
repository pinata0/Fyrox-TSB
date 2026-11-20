//! Source-timed, aimable three/eight-shot volleys. Poison reduces outgoing damage.
use crate::{
    combat::{AttackClass, ProjectileKind},
    game::{direction, Actor, World, DT},
    motion::Motion,
};

pub const SKILL_SECONDS: f32 = 2.0;
pub const ULTIMATE_SECONDS: f32 = 100.0 / 30.0;
pub const SKILL_SHOTS: [f32; 3] = [24.0 / 30.0, 28.0 / 30.0, 32.0 / 30.0];
pub const ULTIMATE_SHOTS: [f32; 8] = [
    49.0 / 30.0,
    53.0 / 30.0,
    57.0 / 30.0,
    61.0 / 30.0,
    65.0 / 30.0,
    69.0 / 30.0,
    73.0 / 30.0,
    77.0 / 30.0,
];
pub const POISON_SECONDS: f32 = 4.0;
pub const POISON_DAMAGE_FACTOR: f32 = 0.8;
pub const DART_DAMAGE: f32 = 14.0;
pub const DART_SPEED: f32 = 44.0;

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
            Self::Skill => Some(Motion::MayoSkill),
            Self::Ultimate => Some(Motion::MayoUltimate),
        }
    }
    pub fn shots(self) -> &'static [f32] {
        match self {
            Self::Off => &[],
            Self::Skill => &SKILL_SHOTS,
            Self::Ultimate => &ULTIMATE_SHOTS,
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
    if a.character != 8 || !a.alive() || a.combat.action_interrupted() || a.burrowed() {
        a.combat.mayo = State::default();
    } else if a.combat.mayo.active() {
        a.combat.mayo.elapsed += DT * crate::renua_skill::time_factor(a);
        if a.combat.mayo.elapsed + 0.00001 >= a.combat.mayo.phase.duration() {
            a.combat.mayo = State::default();
        }
    }
}
impl World {
    pub(crate) fn start_mayo(&mut self, source: usize, phase: Phase) {
        let a = &mut self.actors[source];
        a.combat.mayo = State {
            phase,
            elapsed: 0.0,
        };
        a.combat.cast = None;
        a.reload = 0.0;
        a.shield = 0.0;
        a.ability_flash = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        if phase == Phase::Ultimate {
            a.combat.ultimate_flash = ULTIMATE_SECONDS;
        }
    }
    pub(crate) fn advance_mayo(&mut self, source: usize) {
        let before = self.actors[source].combat.mayo;
        advance_actor(&mut self.actors[source]);
        let after = self.actors[source].combat.mayo;
        if before.phase != after.phase || !after.active() {
            return;
        }
        for &at in after.phase.shots() {
            if before.elapsed + 0.00001 < at && after.elapsed + 0.00001 >= at {
                let a = &self.actors[source];
                let dir = direction(a.yaw, a.pitch);
                let class = if after.phase == Phase::Skill {
                    AttackClass::Skill
                } else {
                    AttackClass::Ultimate
                };
                self.projectile(
                    source,
                    ProjectileKind::Poison,
                    dir,
                    DART_SPEED,
                    DART_DAMAGE,
                    class,
                );
                self.actors[source].shot = self.actors[source].shot.wrapping_add(1);
            }
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
struct Source {
    clips: Vec<Clip>,
}
pub fn hand_sample(motion: Motion, age: f32, left: bool) -> [f32; 3] {
    static SOURCE: std::sync::OnceLock<Source> = std::sync::OnceLock::new();
    let source = SOURCE.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/skill/mayo.json"))
            .expect("Mayo source motion")
    });
    let index = match motion {
        Motion::MayoSkill => 0,
        Motion::MayoUltimate => 1,
        _ => return [0.0; 3],
    };
    let samples = &source.clips[index].samples;
    let frame = (age * 30.0).clamp(0.0, (samples.len() - 1) as f32);
    let lo = frame.floor() as usize;
    let hi = (lo + 1).min(samples.len() - 1);
    let hand = if left { 0 } else { 1 };
    std::array::from_fn(|i| {
        samples[lo].hands[hand][i] * (1.0 - frame.fract())
            + samples[hi].hands[hand][i] * frame.fract()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        combat::Projectile,
        game::{MAX_HP, V3},
    };
    fn world() -> World {
        let mut w = World::default();
        w.start();
        w.map.clear();
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = true;
            a.shield = 0.0;
            a.combat = Default::default();
            a.pos = V3::new(i as f32, 0.0, 0.0);
        }
        w
    }
    #[test]
    fn poison_reduces_all_attack_classes_including_fixed_health_ultimates() {
        for id in [0, 3, 7, 8] {
            for class in [
                AttackClass::Basic,
                AttackClass::Skill,
                AttackClass::Ultimate,
            ] {
                let mut normal = world();
                normal.actors[0].character = id;
                let mut poisoned = normal.clone();
                poisoned.actors[0].combat.poison = 4.0;
                normal.damage_hit(0, 6, 40.0, class, id);
                poisoned.damage_hit(0, 6, 40.0, class, id);
                assert!(
                    (MAX_HP - poisoned.actors[6].hp) < (MAX_HP - normal.actors[6].hp),
                    "{id} {class:?}"
                );
            }
        }
    }
    #[test]
    fn poison_respects_invulnerability_and_debuff_immunity() {
        for protection in 0..3 {
            let mut w = world();
            w.actors[0].character = 8;
            match protection {
                0 => w.actors[6].shield = 1.0,
                1 => w.actors[6].combat.id.invulnerable = 1.0,
                _ => w.actors[6].combat.sleep = 1.0,
            }
            let mut p = Projectile::visual(1, 0, ProjectileKind::Poison, V3::default());
            p.character = 8;
            p.damage = 14.0;
            w.projectile_hit(&mut p, 6);
            assert_eq!(w.actors[6].combat.poison, 0.0);
        }
    }
}
