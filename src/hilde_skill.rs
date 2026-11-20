//! Hilde's source-timed medical case and temporary, player-aimed grenade launcher.
use crate::{
    combat::{AttackClass, Projectile, ProjectileKind},
    game::{direction, Actor, Input, World, DT},
    motion::Motion,
};

pub const HEAL_RELEASE: f32 = 41.0 / 30.0;
pub const HEAL_SECONDS: f32 = 2.0;
pub const HEAL_RADIUS: f32 = 9.0;
pub const HEAL_TOTAL: f32 = 40.0;
pub const HEAL_PULSES: u32 = 4;
pub const EQUIP_SECONDS: f32 = 63.0 / 30.0;
pub const LAUNCHER_SECONDS: f32 = 6.0;
pub const SOURCE_LOOP_END: f32 = 133.0 / 30.0;
pub const END_SECONDS: f32 = (185.0 - 133.0) / 30.0;
pub const FIRE_INTERVAL: f32 = 0.20;
pub const GRENADE_SPEED: f32 = 22.0;
pub const GRENADE_GRAVITY: f32 = 5.0;
pub const GRENADE_HEAL: f32 = 16.0;
pub const GRENADE_DAMAGE: f32 = 14.0;
pub const GRENADE_RADIUS: f32 = 2.2;
pub const BLAST_SECONDS: f32 = 0.65;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Skill,
    Equip,
    Launcher,
    End,
}
impl Phase {
    pub fn from_wire(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Off,
            1 => Self::Skill,
            2 => Self::Equip,
            3 => Self::Launcher,
            4 => Self::End,
            _ => return None,
        })
    }
    pub fn motion(self) -> Option<Motion> {
        match self {
            Self::Off => None,
            Self::Skill => Some(Motion::HildeSkill),
            _ => Some(Motion::HildeUltimate),
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub elapsed: f32,
    pub cooldown: f32,
    pub firing: bool,
}
impl State {
    pub fn active(self) -> bool {
        self.phase != Phase::Off
    }
    pub fn start(&mut self, phase: Phase) {
        *self = Self {
            phase,
            ..Self::default()
        };
    }
    pub fn duration(self) -> f32 {
        if self.phase == Phase::Skill {
            HEAL_RELEASE + HEAL_SECONDS
        } else {
            EQUIP_SECONDS + LAUNCHER_SECONDS + END_SECONDS
        }
    }
    pub fn motion_age(self) -> f32 {
        match self.phase {
            Phase::Skill => self.elapsed.min(99.0 / 30.0),
            Phase::Equip => self.elapsed,
            Phase::Launcher => {
                if self.firing {
                    EQUIP_SECONDS
                        + (self.elapsed - EQUIP_SECONDS).rem_euclid(SOURCE_LOOP_END - EQUIP_SECONDS)
                } else {
                    EQUIP_SECONDS
                }
            }
            Phase::End => SOURCE_LOOP_END + self.elapsed - EQUIP_SECONDS - LAUNCHER_SECONDS,
            Phase::Off => 0.0,
        }
    }
    pub fn pack(self) -> u32 {
        ((self.elapsed / DT + 0.0001).floor() as u32)
            | (((self.cooldown / DT).round() as u32) << 10)
            | ((self.firing as u32) << 15)
    }
    pub fn unpack(phase: Phase, bits: u32) -> Option<Self> {
        let s = Self {
            phase,
            elapsed: (bits & 1023) as f32 * DT,
            cooldown: ((bits >> 10) & 31) as f32 * DT,
            firing: bits & (1 << 15) != 0,
        };
        let valid = match phase {
            Phase::Off => bits == 0,
            Phase::Skill => {
                s.elapsed < HEAL_RELEASE + HEAL_SECONDS && !s.firing && s.cooldown == 0.0
            }
            Phase::Equip => s.elapsed < EQUIP_SECONDS && !s.firing,
            Phase::Launcher => {
                (EQUIP_SECONDS..EQUIP_SECONDS + LAUNCHER_SECONDS).contains(&s.elapsed)
            }
            Phase::End => {
                (EQUIP_SECONDS + LAUNCHER_SECONDS..s.duration()).contains(&s.elapsed) && !s.firing
            }
        };
        (bits >> 16 == 0 && s.cooldown <= FIRE_INTERVAL + DT * 0.01 && valid).then_some(s)
    }
}
/// Clock-only stepping is also used by client movement prediction.
pub fn advance_actor(a: &mut Actor, input: Input) {
    if a.character != 13
        || !a.alive()
        || a.combat.stun > 0.0
        || a.combat.sleep > 0.0
        || a.combat.frozen > 0.0
        || a.burrowed()
    {
        a.combat.hilde = State::default();
        return;
    }
    let factor = crate::renua_skill::time_factor(a);
    let s = &mut a.combat.hilde;
    if !s.active() {
        return;
    }
    s.elapsed += DT * factor;
    for boundary in [EQUIP_SECONDS, EQUIP_SECONDS + LAUNCHER_SECONDS] {
        if (s.elapsed - boundary).abs() < 0.00001 {
            s.elapsed = boundary;
        }
    }
    s.cooldown = (s.cooldown - DT * factor).max(0.0);
    if s.elapsed + 0.00001 >= s.duration() {
        *s = State::default();
        return;
    }
    if s.phase != Phase::Skill {
        s.phase = if s.elapsed + 0.00001 < EQUIP_SECONDS {
            Phase::Equip
        } else if s.elapsed + 0.00001 < EQUIP_SECONDS + LAUNCHER_SECONDS {
            Phase::Launcher
        } else {
            Phase::End
        };
        s.firing = s.phase == Phase::Launcher && input.fire && !input.phase_shift && factor > 0.0;
    }
}
impl World {
    pub(crate) fn start_hilde(&mut self, source: usize, phase: Phase) {
        let a = &mut self.actors[source];
        a.combat.hilde.start(phase);
        a.reload = 0.0;
        a.shield = 0.0;
    }
    pub(crate) fn advance_hilde(&mut self, source: usize, input: Input) {
        let before = self.actors[source].combat.hilde;
        advance_actor(&mut self.actors[source], input);
        let after = self.actors[source].combat.hilde;
        // Include the final pulse on the same tick the skill returns to idle.
        let completed = before.phase == Phase::Skill
            && before.elapsed
                + DT * crate::renua_skill::time_factor(&self.actors[source])
                + 0.00001
                >= before.duration();
        if before.phase == Phase::Skill
            && self.actors[source].character == 13
            && self.actors[source].alive()
            && !self.actors[source].burrowed()
            && !self.actors[source].combat.disabled()
            && (after.phase == Phase::Skill || completed)
        {
            let end = if completed {
                before.duration()
            } else {
                after.elapsed
            };
            for n in 1..=HEAL_PULSES {
                let at = HEAL_RELEASE + HEAL_SECONDS * n as f32 / HEAL_PULSES as f32;
                if before.elapsed + 0.00001 < at && end + 0.00001 >= at {
                    let origin = self.actors[source].eye();
                    for target in self.targets(source, origin, HEAL_RADIUS, true) {
                        self.heal_from(source, target, HEAL_TOTAL / HEAL_PULSES as f32);
                    }
                }
            }
        }
        if after.firing && after.cooldown <= 0.00001 {
            let a = &self.actors[source];
            let dir = direction(a.yaw, a.pitch);
            if self
                .projectile(
                    source,
                    ProjectileKind::MedicalGrenade,
                    dir,
                    GRENADE_SPEED,
                    GRENADE_DAMAGE,
                    AttackClass::Ultimate,
                )
                .is_some()
            {
                let a = &mut self.actors[source];
                a.combat.hilde.cooldown = FIRE_INTERVAL;
                a.shot = a.shot.wrapping_add(1);
            }
        }
    }
    pub(crate) fn explode_hilde_grenade(&mut self, p: &Projectile) {
        self.record_medical_explosion(p.pos);
        for target in self.targets(p.owner, p.pos, GRENADE_RADIUS, true) {
            self.heal_from(p.owner, target, GRENADE_HEAL);
        }
        for target in self.targets(p.owner, p.pos, GRENADE_RADIUS, false) {
            self.damage_hit(p.owner, target, p.damage, p.class, p.character);
        }
        self.actors[p.owner].ability_flash = 0.2;
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
        serde_json::from_str(include_str!("../assets/characters/skill/hilde.json"))
            .expect("Hilde source motion")
    });
    let index = match motion {
        Motion::HildeSkill => 0,
        Motion::HildeUltimate => 1,
        _ => return [0.0; 3],
    };
    let samples = &source.clips[index].samples;
    let frame = (age * 30.0).clamp(0.0, (samples.len() - 1) as f32);
    let lo = frame.floor() as usize;
    let hi = (lo + 1).min(samples.len() - 1);
    std::array::from_fn(|i| {
        samples[lo].hands[usize::from(left)][i] * (1.0 - frame.fract())
            + samples[hi].hands[usize::from(left)][i] * frame.fract()
    })
}
