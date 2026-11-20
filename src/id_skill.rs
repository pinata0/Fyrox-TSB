//! Id's independent protection, source-timed pod actions and Nata commands.
use crate::{
    combat::AttackClass,
    game::{team, Input, World, DT, MAX_HP},
    motion::Motion,
};

pub const PROTECTION_SECONDS: f32 = 3.0;
pub const HEALTH_FLOOR: u16 = MAX_HP.div_ceil(10);
pub const SKILL_SECONDS: f32 = 108.0 / 30.0;
pub const ULTIMATE_SECONDS: f32 = 160.0 / 30.0;
pub const ULTIMATE_RELEASE: f32 = 64.0 / 30.0;
pub const ULTIMATE_RADIUS: f32 = 10.0;
pub const PULSE_COOLDOWN: f32 = 3.0;
pub const PULSE_SECONDS: f32 = 0.7;
pub const PULSE_RADIUS: f32 = 6.0;
pub const PULSE_DAMAGE: f32 = 15.0;
pub const PULSE_PUSH: f32 = 1.25;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Skill,
    Pulse,
    Ultimate,
}
impl Phase {
    pub fn from_wire(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Off,
            1 => Self::Skill,
            2 => Self::Pulse,
            3 => Self::Ultimate,
            _ => return None,
        })
    }
    pub fn motion(self) -> Option<Motion> {
        match self {
            Self::Off => None,
            Self::Skill => Some(Motion::IdSkill),
            Self::Pulse => Some(Motion::IdPulse),
            Self::Ultimate => Some(Motion::IdUltimate),
        }
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Skill => SKILL_SECONDS,
            Self::Pulse => PULSE_SECONDS,
            Self::Ultimate => ULTIMATE_SECONDS,
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub elapsed: f32,
    pub invulnerable: f32,
    pub pulse_cooldown: f32,
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
    pub fn advance(&mut self) {
        self.tick_timers();
        self.advance_action();
    }
    pub fn tick_timers(&mut self) {
        for timer in [&mut self.invulnerable, &mut self.pulse_cooldown] {
            *timer = (*timer - DT).max(0.0);
            if *timer < DT * 0.01 {
                *timer = 0.0;
            }
        }
    }
    pub fn advance_action(&mut self) {
        self.advance_action_scaled(1.0);
    }
    pub fn advance_action_scaled(&mut self, factor: f32) {
        if self.active() {
            self.elapsed += DT * factor;
            if self.elapsed + 0.00001 >= self.phase.duration() {
                self.start(Phase::Off);
            }
        }
    }
    // Character-discriminated replacement of the existing four-byte flash slot.
    // No snapshot growth: 9 clock bits, 8 cooldown bits, 8 protection bits, one latch.
    pub fn pack(self) -> u32 {
        let ticks = |v: f32| (v / DT).round() as u32;
        (self.elapsed / DT + 0.0001).floor() as u32
            | (ticks(self.pulse_cooldown) << 9)
            | (ticks(self.invulnerable) << 17)
            | ((self.alt_down as u32) << 25)
    }
    pub fn unpack(phase: Phase, bits: u32) -> Option<Self> {
        let clock = bits & 511;
        let cooldown = (bits >> 9) & 255;
        let shield = (bits >> 17) & 255;
        if bits >> 26 != 0
            || cooldown > 180
            || shield > 180
            || (phase == Phase::Off && clock != 0)
            || (phase != Phase::Off && clock as f32 * DT >= phase.duration())
        {
            return None;
        }
        Some(Self {
            phase,
            elapsed: clock as f32 * DT,
            pulse_cooldown: cooldown as f32 * DT,
            invulnerable: shield as f32 * DT,
            alt_down: bits & (1 << 25) != 0,
        })
    }
}
pub fn smooth(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
pub fn eye_closure(motion: Motion, age: f32) -> f32 {
    if motion != Motion::IdUltimate {
        return 1.0;
    }
    1.0 - smooth((age - ULTIMATE_RELEASE + 0.10) / 0.10) * (1.0 - smooth((age - 4.65) / 0.25))
}
pub fn camera_lift(state: State) -> f32 {
    if state.phase != Phase::Skill {
        return 0.0;
    }
    0.065 * smooth(state.elapsed / 0.14) * (1.0 - smooth((state.elapsed - 0.42) / 0.32))
}
pub fn point_amount(age: f32) -> f32 {
    smooth(age / 0.055) * (1.0 - smooth((age - 0.14) / 0.16))
}
impl World {
    pub(crate) fn protect_id_team(&mut self, source: usize) {
        self.actors[source].combat.id.start(Phase::Skill);
        self.actors[source].combat.id.invulnerable = PROTECTION_SECONDS;
        self.actors[source].reload = 0.0;
        for (i, a) in self.actors.iter_mut().enumerate() {
            if i != source && team(i) == team(source) && a.alive() {
                a.combat.ward = PROTECTION_SECONDS;
                // Establish the floor even for an ally who was already below 10%.
                a.hp = a.hp.max(HEALTH_FLOOR);
            }
        }
    }
    pub(crate) fn advance_id(&mut self, source: usize, input: Input) {
        let a = &mut self.actors[source];
        if a.character != 3 || !a.alive() {
            a.combat.id = State::default();
            return;
        }
        let previous = a.combat.id;
        a.combat.id.advance_action_scaled(crate::renua_skill::time_factor(a));
        let pressed = input.alt_fire && !previous.alt_down;
        a.combat.id.alt_down = input.alt_fire;
        if a.combat.action_interrupted() || a.burrowed() {
            a.combat.id.start(Phase::Off);
            return;
        }
        let released = previous.phase == Phase::Ultimate
            && previous.elapsed + 0.00001 < ULTIMATE_RELEASE
            && a.combat.id.elapsed + 0.00001 >= ULTIMATE_RELEASE;
        let pulse = pressed
            && !a.combat.skills_locked()
            && a.combat.id.pulse_cooldown == 0.0
            && !a.combat.channeling()
            && !input.ability
            && !input.ultimate
            && !input.phase_shift;
        if pulse {
            a.combat.id.start(Phase::Pulse);
            a.combat.id.pulse_cooldown = PULSE_COOLDOWN;
            a.shield = 0.0;
            a.reload = 0.0;
        }
        let origin = a.eye();
        if pulse {
            self.burst(
                source,
                origin,
                PULSE_RADIUS,
                PULSE_DAMAGE,
                AttackClass::Skill,
                PULSE_PUSH,
            );
        }
        if released {
            for target in self.targets(source, origin, ULTIMATE_RADIUS, false) {
                self.damage_hit(
                    source,
                    target,
                    self.actors[target].max_hp() as f32 * 0.5,
                    AttackClass::Ultimate,
                    3,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::{Phase as MatchPhase, PLAYERS, V3},
        motion::MotionClock,
    };
    fn world() -> World {
        let mut w = World::default();
        w.start();
        w.phase = MatchPhase::Playing;
        w.map.clear();
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.human = true;
            a.active = true;
            a.shield = 0.0;
            a.combat = Default::default();
            a.barrier = 0.0;
            a.pos = V3::new(30.0 + i as f32 * 2.0, 0.0, 30.0);
        }
        w.actors[0].character = 3;
        w.actors[0].ammo = 0;
        w.actors[0].pos = V3::default();
        w.actors[6].pos = V3::new(0.0, 0.0, 3.0);
        w
    }
    fn tick(w: &mut World, input: Input) {
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0] = input;
        w.tick(inputs);
    }
    #[test]
    fn protection_is_teamwide_and_survives_unlimited_lethal_hits_then_expires() {
        let mut w = world();
        w.actors[1].hp = 1;
        w.actors[2].hp = 0;
        w.actors[2].respawn = 7.0;
        tick(
            &mut w,
            Input {
                ability: true,
                ..Default::default()
            },
        );
        assert_eq!(w.actors[1].hp, HEALTH_FLOOR);
        assert_eq!(w.actors[2].hp, 0);
        assert_eq!(w.actors[6].combat.ward, 0.0);
        for class in [
            AttackClass::Basic,
            AttackClass::Skill,
            AttackClass::Ultimate,
        ] {
            for _ in 0..8 {
                assert!(!w.damage_hit(6, 0, 1000.0, class, 8));
                w.damage_hit(6, 1, 1000.0, class, 8);
            }
        }
        assert_eq!(w.actors[0].hp, MAX_HP);
        assert_eq!(w.actors[1].hp, HEALTH_FLOOR);
        for _ in 0..179 {
            tick(&mut w, Input::default());
        }
        assert!(w.actors[0].combat.id.invulnerable > 0.0);
        tick(&mut w, Input::default());
        assert_eq!(w.actors[0].combat.id.invulnerable, 0.0);
        assert_eq!(w.actors[1].combat.ward, 0.0);
        w.damage_hit(6, 0, 1000.0, AttackClass::Basic, 8);
        w.damage_hit(6, 1, 1000.0, AttackClass::Basic, 8);
        assert_eq!(w.actors[0].hp, 0);
        assert_eq!(w.actors[1].hp, 0);
    }
    #[test]
    fn pulse_hits_all_directions_not_allies_or_cover_and_requires_a_new_press() {
        let mut w = world();
        w.actors[7].pos = V3::new(0.0, 0.0, -3.0);
        w.actors[8].pos = V3::new(3.0, 0.0, 0.0);
        w.actors[1].pos = V3::new(-2.0, 0.0, 0.0);
        w.map.push(crate::game::Block {
            center: V3::new(1.5, 1.5, 0.0),
            size: V3::new(0.2, 4.0, 2.0),
            color: [0; 3],
            phaseable: false,
        });
        tick(
            &mut w,
            Input {
                alt_fire: true,
                ..Default::default()
            },
        );
        assert_eq!(
            w.actors[6].hp,
            MAX_HP - (PULSE_DAMAGE * w.damage_multiplier(team(0))).round() as u16
        );
        assert_eq!(
            w.actors[7].hp,
            MAX_HP - (PULSE_DAMAGE * w.damage_multiplier(team(0))).round() as u16
        );
        assert!(w.actors[6].pos.z > 3.0);
        assert!(w.actors[7].pos.z < -3.0);
        assert_eq!(w.actors[8].hp, MAX_HP);
        assert_eq!(w.actors[1].hp, MAX_HP);
        assert_eq!(w.actors[0].combat.id.pulse_cooldown, 3.0);
        for _ in 0..190 {
            tick(
                &mut w,
                Input {
                    alt_fire: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!(w.actors[0].combat.id.pulse_cooldown, 0.0);
        let hp = w.actors[6].hp;
        tick(&mut w, Input::default());
        tick(
            &mut w,
            Input {
                alt_fire: true,
                ..Default::default()
            },
        );
        assert!(w.actors[6].hp < hp);
    }
    #[test]
    fn universe_charges_once_and_reduces_damage_only_during_the_motion() {
        let mut w = world();
        w.actors[0].combat.ultimate = 100.0;
        w.actors[7].pos = V3::new(0.0, 0.0, 11.0);
        tick(
            &mut w,
            Input {
                ultimate: true,
                ..Default::default()
            },
        );
        w.damage_hit(
            6,
            0,
            20.0 / w.damage_multiplier(team(6)),
            AttackClass::Basic,
            8,
        );
        assert_eq!(w.actors[0].hp, MAX_HP - 14);
        w.actors[0].hp = MAX_HP;
        for _ in 0..127 {
            tick(&mut w, Input::default());
        }
        assert_eq!(w.actors[6].hp, MAX_HP);
        tick(&mut w, Input::default());
        assert_eq!(w.actors[6].hp, MAX_HP / 2);
        for _ in 0..192 {
            tick(&mut w, Input::default());
        }
        assert_eq!(w.actors[6].hp, MAX_HP / 2);
        assert_eq!(w.actors[7].hp, MAX_HP);
        assert_eq!(w.actors[0].combat.id.phase, Phase::Off);
        w.damage_hit(
            6,
            0,
            20.0 / w.damage_multiplier(team(6)),
            AttackClass::Basic,
            8,
        );
        assert_eq!(w.actors[0].hp, MAX_HP - 20);
    }
    #[test]
    fn interrupted_charge_never_releases_and_e_protection_survives_q() {
        let mut w = world();
        tick(
            &mut w,
            Input {
                ability: true,
                ..Default::default()
            },
        );
        w.actors[0].combat.ultimate = 100.0;
        tick(
            &mut w,
            Input {
                ultimate: true,
                ..Default::default()
            },
        );
        assert!(w.actors[0].combat.id.invulnerable > 2.9);
        w.actors[0].combat.stun = 0.3;
        tick(&mut w, Input::default());
        assert_eq!(w.actors[0].combat.id.phase, Phase::Off);
        for _ in 0..330 {
            tick(&mut w, Input::default());
        }
        assert_eq!(w.actors[6].hp, MAX_HP);
    }
    #[test]
    fn pose_and_wire_follow_source_timing_with_bounded_clocks() {
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../assets/characters/skill/id.json")).unwrap();
        assert!((source["release"].as_f64().unwrap() as f32 - ULTIMATE_RELEASE).abs() < 1e-5);
        let mut w = world();
        let mut clock = MotionClock::default();
        for phase in [Phase::Skill, Phase::Pulse, Phase::Ultimate] {
            let state = State {
                phase,
                elapsed: 0.25,
                invulnerable: 1.5,
                pulse_cooldown: 2.75,
                alt_down: true,
            };
            let copy = State::unpack(phase, state.pack()).unwrap();
            assert_eq!(copy.phase, phase);
            assert_eq!(copy.elapsed, 0.25);
            assert!((copy.invulnerable - 1.5).abs() < 1e-5);
            assert!((copy.pulse_cooldown - 2.75).abs() < 1e-5);
            assert!(copy.alt_down);
            w.actors[0].combat.id = state;
            let pose = clock.observe(&w.actors[0], DT, 10, true);
            assert_eq!(pose.motion, phase.motion().unwrap());
            assert_eq!(pose.age, 0.25);
        }
        assert!(State::unpack(Phase::Skill, u32::MAX).is_none());
        assert!(State::unpack(Phase::Off, 1).is_none());
        assert_eq!(eye_closure(Motion::Idle, 2.0), 1.0);
        assert_eq!(eye_closure(Motion::IdUltimate, 2.5), 0.0);
        assert_eq!(eye_closure(Motion::IdUltimate, 5.2), 1.0);
    }
}
