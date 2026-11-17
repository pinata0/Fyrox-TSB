//! Shared animation clock for recorded actors and character-specific death clips.
use crate::game::Actor;

#[cfg(test)]
mod move_tests;

/// Original battle Die track lengths, in roster order.
pub const DEATH_SECONDS: [f32; 15] = [
    2.0,
    56.0 / 30.0,
    55.0 / 30.0,
    77.0 / 30.0,
    2.5,
    64.0 / 30.0,
    55.0 / 30.0,
    2.7,
    56.0 / 30.0,
    3.0,
    55.0 / 30.0,
    56.0 / 30.0,
    56.0 / 30.0,
    56.0 / 30.0,
    56.0 / 30.0,
];
pub fn death_seconds(character: u8) -> f32 {
    DEATH_SECONDS
        .get(character as usize)
        .copied()
        .unwrap_or(2.0)
}
/// Original battle Spawn track lengths, in roster order, sampled at 30 Hz.
pub const SPAWN_FRAMES: [u16; 15] = [40, 40, 40, 40, 55, 47, 40, 79, 40, 58, 40, 40, 40, 40, 54];
pub fn spawn_seconds(character: u8) -> f32 {
    SPAWN_FRAMES.get(character as usize).copied().unwrap_or(40) as f32 / 30.0
}

/// Archived battle Move periods, in roster order (30 Hz). Run uses the same
/// character-specific gait at 1.25x; floating/seated heroes keep their own pose.
pub const MOVE_FRAMES: [u16; 15] = [40, 40, 44, 80, 24, 28, 24, 30, 24, 198, 24, 24, 28, 24, 20];
pub fn move_seconds(character: u8) -> f32 {
    MOVE_FRAMES.get(character as usize).copied().unwrap_or(30) as f32 / 30.0
}

/// Selection and world spawns seek the very same clip at its source speed.
pub fn selection_pose(character: u8, elapsed: f32) -> Pose {
    let elapsed = elapsed.max(0.0);
    let duration = spawn_seconds(character);
    Pose {
        motion: if elapsed < duration {
            Motion::Spawn
        } else {
            Motion::Idle
        },
        age: if elapsed < duration {
            elapsed
        } else {
            elapsed - duration
        },
        shot_age: 100.0,
        locomotion: None,
    }
}
/// Verified standing Dance_1 periods, in simulation ticks (60 Hz).
/// Characters without a verified source retain their existing four-second dance.
pub const DANCE_TICKS: [u64; 15] = [
    240, 240, 240, 240, 96, 64, 64, 160, 240, 240, 240, 240, 240, 96, 240,
];
pub fn dance_ticks(character: u8) -> u64 {
    DANCE_TICKS.get(character as usize).copied().unwrap_or(240)
}
pub fn dance_age(character: u8, tick: u64) -> f32 {
    (tick % dance_ticks(character)) as f32 / 60.0
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    #[default]
    Idle,
    Walk,
    Run,
    Jump,
    Attack,
    Attack2,
    Skill,
    ErpinEat,
    NaiaStart,
    NaiaMove,
    NaiaFire,
    NaiaEnd,
    Ultimate,
    Dance,
    Hurt,
    Death,
    Spawn,
}
impl Motion {
    pub fn frames(self) -> (u16, u16) {
        match self {
            Self::Idle => (0, 60),
            Self::Walk => (63, 93),
            Self::Run => (96, 120),
            Self::Jump => (123, 153),
            Self::Attack => (156, 180),
            Self::Attack2 => (531, 555),
            Self::Skill => (183, 219),
            Self::ErpinEat => (531, 581),
            Self::NaiaStart => (558, 602),
            Self::NaiaMove => (605, 629),
            Self::NaiaFire => (632, 642),
            Self::NaiaEnd => (645, 672),
            Self::Ultimate => (222, 282),
            Self::Dance => (285, 405),
            Self::Hurt => (408, 420),
            Self::Death => (423, 453),
            Self::Spawn => (456, 528),
        }
    }
    pub fn looping(self) -> bool {
        matches!(
            self,
            Self::Idle | Self::Walk | Self::Run | Self::Dance | Self::NaiaMove | Self::NaiaFire
        )
    }
    pub fn is_attack(self) -> bool {
        matches!(self, Self::Attack | Self::Attack2)
    }
    pub fn is_naia_skill(self) -> bool {
        matches!(
            self,
            Self::NaiaStart | Self::NaiaMove | Self::NaiaFire | Self::NaiaEnd
        )
    }
    pub fn seconds(self, id: u8) -> f32 {
        if self == Self::ErpinEat {
            crate::erpin::EAT_SECONDS
        } else if self == Self::Death {
            death_seconds(id)
        } else if self == Self::Spawn {
            spawn_seconds(id)
        } else if self.is_attack() {
            crate::attack::seconds(id)
        } else if self == Self::Dance {
            dance_ticks(id) as f32 / 60.0
        } else if matches!(self, Self::Walk | Self::Run) {
            move_seconds(id) / if self == Self::Run { 1.25 } else { 1.0 }
        } else {
            let (s, e) = self.frames();
            (e - s) as f32 / 30.0
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct Pose {
    pub motion: Motion,
    pub age: f32,
    pub shot_age: f32,
    /// Independent ground movement clock, retained while basic attacks play.
    pub locomotion: Option<LocomotionPose>,
}
#[derive(Debug, Clone, Copy)]
pub struct LocomotionPose {
    pub motion: Motion,
    pub age: f32,
}
impl Pose {
    /// Select the full-body base. Attack remains the public action clock used by
    /// hands, weapons, facial animation and replays; only its arms are overlaid.
    pub fn base(self) -> (Motion, f32) {
        if self.motion.is_attack() {
            if let Some(movement) = self.locomotion {
                return (movement.motion, movement.age);
            }
        }
        (self.motion, self.age)
    }
    pub fn arm_attack_weight(self, id: u8) -> f32 {
        if !self.motion.is_attack() || self.locomotion.is_none() {
            return 0.0;
        }
        let phase = (self.age / self.motion.seconds(id)).clamp(0.0, 1.0);
        let smooth = |t: f32| {
            let t = t.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        smooth(phase / 0.08) * smooth((1.0 - phase) / 0.14)
    }
}
#[derive(Debug, Default, Clone)]
pub struct MotionClock {
    pub pose: Pose,
    previous: Option<Actor>,
    previous_tick: Option<u64>,
}
impl MotionClock {
    pub fn observe(&mut self, a: &Actor, dt: f32, tick: u64, grounded: bool) -> Pose {
        let p = self.previous.as_ref().unwrap_or(a);
        let changed = a.character != p.character
            || a.deaths != p.deaths
            || a.alive() != p.alive()
            || a.active != p.active;
        // The existing authoritative shield resets only on spawn. This also
        // catches a new round or a missed death snapshot without changing packets.
        let spawned = a.alive()
            && (changed
                || a.shield > p.shield + 0.01
                || self.previous_tick.is_some_and(|previous| tick < previous)
                || self.previous.is_none() && a.shield > 0.0);
        self.pose.age += dt
            * if self.pose.motion.is_attack() {
                crate::attack::rate(a)
            } else {
                1.0
            };
        self.pose.shot_age = if changed {
            100.0
        } else if a.shot != p.shot {
            0.0
        } else {
            self.pose.shot_age + dt
        };
        let triggered = if a.combat.ultimate_flash > 0.0 && p.combat.ultimate_flash <= 0.0 {
            Some(Motion::Ultimate)
        } else if a.ability_flash > 0.0 && p.ability_flash <= 0.0 {
            Some(Motion::Skill)
        } else if a.shot != p.shot && !changed {
            Some(
                if (a.character == 0 && a.combat.vivi_combo == 3)
                    || (a.character == 4 && a.combat.aya_combo == 3)
                {
                    Motion::Attack2
                } else {
                    Motion::Attack
                },
            )
        } else if a.hurt > 0.0 && p.hurt <= 0.0 {
            Some(Motion::Hurt)
        } else {
            None
        };
        let speed = (a.pos - p.pos).flat().length() / dt.max(0.001);
        let held = matches!(
            self.pose.motion,
            Motion::Attack | Motion::Attack2 | Motion::Skill | Motion::Ultimate | Motion::Hurt
        ) && self.pose.age < self.pose.motion.seconds(a.character);
        let next = if !a.alive() {
            Motion::Death
        } else if a.combat.erpin.eating {
            Motion::ErpinEat
        } else if let Some(motion) = a.combat.naia.phase.motion() {
            motion
        } else if a.dancing {
            Motion::Dance
        } else if let Some(m) = triggered {
            m
        } else if spawned
            || self.pose.motion == Motion::Spawn
                && self.pose.age < spawn_seconds(a.character)
                && grounded
                && speed <= 0.12
                && !a.crouching
                && a.wave_ticks == 0
                && a.reload <= 0.0
        {
            Motion::Spawn
        } else if held && !changed {
            self.pose.motion
        } else if !grounded {
            Motion::Jump
        } else if speed > 6.7 {
            Motion::Run
        } else if speed > 0.12 {
            Motion::Walk
        } else {
            Motion::Idle
        };
        if next != self.pose.motion
            || changed
            || spawned
            || triggered.is_some() && a.alive() && !a.dancing
        {
            self.pose.age = 0.0;
        }
        if self.previous.is_none() {
            self.pose.shot_age = 100.0;
        }
        self.pose.motion = next;
        // Advance independently of attack speed and attack retriggers. Preserve
        // stride phase when switching Walk/Run, including during an attack.
        self.pose.locomotion = if grounded
            && speed > 0.12
            && (matches!(next, Motion::Walk | Motion::Run) || next.is_attack())
        {
            let motion = if speed > 6.7 {
                Motion::Run
            } else {
                Motion::Walk
            };
            let duration = motion.seconds(a.character);
            let age = self
                .pose
                .locomotion
                .filter(|_| !changed && !spawned)
                .map_or(0.0, |old| {
                    old.age / old.motion.seconds(a.character) * duration + dt.max(0.0)
                });
            Some(LocomotionPose {
                motion,
                age: age.rem_euclid(duration),
            })
        } else {
            None
        };
        if matches!(next, Motion::Walk | Motion::Run) {
            self.pose.age = self.pose.locomotion.map_or(0.0, |movement| movement.age);
        }
        if next == Motion::ErpinEat {
            self.pose.age = a.combat.erpin.elapsed;
        }
        if next.is_naia_skill() {
            self.pose.age = a.combat.naia.motion_age();
        }
        if next == Motion::Dance {
            self.pose.age = dance_age(a.character, tick);
        }
        self.previous = Some(a.clone());
        self.previous_tick = Some(tick);
        self.pose
    }
}
pub fn interpolate_angle(a: f32, b: f32, t: f32) -> f32 {
    a + ((b - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI)
        * t
}

#[cfg(test)]
mod dance_tests {
    use super::*;

    #[test]
    fn dance_clock_wraps_at_each_source_period_without_long_session_drift() {
        for id in 0..15 {
            let period = dance_ticks(id);
            let duration = Motion::Dance.seconds(id);
            for tick in [0, 1, period - 1, period, period + 1, u64::MAX - 1] {
                let age = dance_age(id, tick);
                assert!(age >= 0.0 && age < duration);
                let baked_time = 9.5 + 4.0 * age / duration;
                assert!((9.5..13.5).contains(&baked_time));
            }
            assert_eq!(dance_age(id, period), 0.0);
            assert_eq!(dance_age(id, period + 1), dance_age(id, 1));
        }
    }

    #[test]
    fn verified_dances_keep_source_timing() {
        for (id, ticks) in [(4, 96), (5, 64), (6, 64), (7, 160), (13, 96)] {
            assert_eq!(dance_ticks(id), ticks);
        }
        assert_eq!(dance_ticks(255), 240);
    }

    #[test]
    fn late_join_character_switch_and_cancel_use_the_authoritative_dance_clock() {
        let mut actor = Actor {
            character: 7,
            dancing: true,
            ..Actor::default()
        };
        let mut live = MotionClock::default();
        let mut joined = MotionClock::default();
        live.observe(&actor, 0.0, 0, true);
        let a = live.observe(&actor, 0.6, 197, true);
        let b = joined.observe(&actor, 0.0, 197, true);
        assert_eq!(a.motion, Motion::Dance);
        assert_eq!(a.age, b.age);
        actor.character = 5;
        let p = live.observe(&actor, 0.02, 198, true);
        assert_eq!(p.age, dance_age(5, 198));
        actor.dancing = false;
        actor.shot += 1;
        assert_eq!(live.observe(&actor, 0.02, 199, true).motion, Motion::Attack);
        actor.hp = 0;
        assert_eq!(live.observe(&actor, 0.02, 200, true).motion, Motion::Death);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spawn_sources_cover_the_roster_and_selection_uses_the_same_clock() {
        let records: serde_json::Value =
            serde_json::from_str(include_str!("../assets/characters/spawn/motions.json")).unwrap();
        let records = records.as_array().unwrap();
        assert_eq!(records.len(), SPAWN_FRAMES.len());
        let mut signatures = std::collections::HashSet::new();
        for (id, record) in records.iter().enumerate() {
            assert_eq!(record["id"], id);
            assert_eq!(record["clip"], "Spawn");
            let duration = Motion::Spawn.seconds(id as u8);
            assert!((record["duration"].as_f64().unwrap() as f32 - duration).abs() < 0.0001);
            assert_eq!(
                record["samples"].as_array().unwrap().len(),
                SPAWN_FRAMES[id] as usize + 1
            );
            signatures.insert(record["samples"].to_string());
            for phase in [0.0, 0.3, 0.75, 0.99] {
                let preview = selection_pose(id as u8, phase * duration);
                assert_eq!(preview.motion, Motion::Spawn);
                assert_eq!(preview.age, phase * duration);
            }
            let idle = selection_pose(id as u8, duration);
            assert_eq!(idle.motion, Motion::Idle);
            assert_eq!(idle.age, 0.0);
            assert_eq!(
                selection_pose(id as u8, duration + 5.0).motion,
                Motion::Idle
            );
        }
        assert_eq!(signatures.len(), 15);
        assert!(!Motion::Spawn.looping());
    }

    #[test]
    fn spawn_plays_once_on_entry_respawn_switch_and_round_reset() {
        for id in 0..15 {
            let mut a = Actor {
                character: id,
                shield: 2.0,
                ..Actor::default()
            };
            let mut clock = MotionClock::default();
            assert_eq!(clock.observe(&a, 0.0, 0, true).motion, Motion::Spawn);
            let duration = spawn_seconds(id);
            a.shield = 1.0;
            let pose = clock.observe(&a, duration * 0.5, 1, true);
            assert_eq!(pose.motion, Motion::Spawn);
            assert_eq!(pose.age, selection_pose(id, duration * 0.5).age);
            assert_eq!(
                clock.observe(&a, duration * 0.51, 2, true).motion,
                Motion::Idle
            );
            a.hp = 0;
            a.deaths += 1;
            assert_eq!(clock.observe(&a, 0.01, 3, true).motion, Motion::Death);
            a.hp = 120;
            a.shield = 2.0;
            assert_eq!(clock.observe(&a, 0.01, 4, true).motion, Motion::Spawn);
            assert_eq!(clock.pose.age, 0.0);
            a.character = (id + 1) % 15;
            assert_eq!(clock.observe(&a, 0.1, 5, true).motion, Motion::Spawn);
            assert_eq!(clock.pose.age, 0.0);
            a.shield = 0.0;
            clock.observe(&a, 4.0, 6, true);
            a.shield = 2.0;
            assert_eq!(clock.observe(&a, 0.01, 7, true).motion, Motion::Spawn);
            assert_eq!(clock.pose.age, 0.0);
            // A match start resets time even when the lobby shield was still full.
            clock.observe(&a, 4.0, 8, true);
            assert_eq!(clock.observe(&a, 0.0, 0, true).motion, Motion::Spawn);
            assert_eq!(clock.pose.age, 0.0);
        }
    }

    #[test]
    fn spawn_gives_way_to_player_actions_and_does_not_restart_after_cancellation() {
        for action in 0..7 {
            let mut a = Actor {
                shield: 2.0,
                ..Actor::default()
            };
            let mut clock = MotionClock::default();
            clock.observe(&a, 0.0, 0, true);
            a.shield = 1.9;
            let expected = match action {
                0 => {
                    a.pos.x += 0.5;
                    Motion::Run
                }
                1 => {
                    a.shot += 1;
                    Motion::Attack
                }
                2 => {
                    a.ability_flash = 0.4;
                    Motion::Skill
                }
                3 => {
                    a.dancing = true;
                    Motion::Dance
                }
                4 => {
                    a.hp = 0;
                    Motion::Death
                }
                5 => Motion::Jump,
                _ => {
                    a.wave_ticks = 30;
                    Motion::Idle
                }
            };
            assert_eq!(clock.observe(&a, 0.02, 1, action != 5).motion, expected);
            if action == 0 || action == 6 {
                a.wave_ticks = 0;
                assert_eq!(clock.observe(&a, 0.02, 2, true).motion, Motion::Idle);
            }
        }
    }

    #[test]
    fn vivi_clock_selects_the_authoritative_combo_even_after_skipped_snapshots() {
        let mut a = Actor {
            character: 0,
            ..Actor::default()
        };
        let mut clock = MotionClock::default();
        clock.observe(&a, 0.0, 0, true);
        for (shot, stage, motion) in [
            (1, 1, Motion::Attack),
            (2, 2, Motion::Attack),
            (3, 3, Motion::Attack2),
            (4, 1, Motion::Attack),
            (6, 3, Motion::Attack2),
        ] {
            a.shot = shot;
            a.combat.vivi_combo = stage;
            let pose = clock.observe(&a, 0.2, shot as u64, true);
            assert_eq!(pose.motion, motion);
            assert_eq!(pose.age, 0.0);
        }
        a.combat.haste = 1.0;
        let pose = clock.observe(&a, 0.1, 7, false);
        assert_eq!(pose.motion, Motion::Attack2);
        assert!((pose.age - 0.1 / 0.75).abs() < 0.0001);
    }
    #[test]
    fn attack_finishes_before_repeat_and_survives_airborne_movement() {
        for id in 0..15 {
            for haste in [0.0, 2.0] {
                let mut a = Actor {
                    character: id,
                    ..Actor::default()
                };
                a.combat.haste = haste;
                let mut clock = MotionClock::default();
                clock.observe(&a, 0.0, 0, true);
                a.shot = 1;
                assert_eq!(clock.observe(&a, 0.0, 1, false).motion, Motion::Attack);
                let duration = crate::attack::seconds(id) / crate::attack::rate(&a);
                assert_eq!(
                    clock.observe(&a, duration * 0.5, 2, false).motion,
                    Motion::Attack
                );
                assert_eq!(
                    clock.observe(&a, duration * 0.51, 3, false).motion,
                    Motion::Jump
                );
                a.shot = 2;
                assert_eq!(clock.observe(&a, 0.01, 4, true).age, 0.0);
                a.character = (id + 1) % 15;
                a.shot = 0;
                let p = clock.observe(&a, 0.01, 5, true);
                assert_eq!(p.motion, Motion::Spawn);
                assert_eq!(p.shot_age, 100.0);
            }
        }
    }
    #[test]
    fn runtime_death_lengths_match_all_fifteen_researched_tracks() {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../assets/characters/death/motions.json")).unwrap();
        let records = data.as_array().unwrap();
        assert_eq!(records.len(), 15);
        let mut signatures = std::collections::HashSet::new();
        for (i, r) in records.iter().enumerate() {
            let duration = r["duration"].as_f64().unwrap() as f32;
            assert!((death_seconds(i as u8) - duration).abs() < 0.0001);
            let samples = r["samples"].as_array().unwrap();
            assert_eq!(samples.len(), (duration * 30.0).round() as usize + 1);
            signatures.insert(r["samples"].to_string());
        }
        assert_eq!(signatures.len(), 15);
    }
    #[test]
    fn shared_clock_keeps_attacks_death_and_respawn_on_their_own_timeline() {
        let mut clock = MotionClock::default();
        let mut a = Actor::default();
        clock.observe(&a, 0.0, 0, true);
        a.shot = 1;
        assert_eq!(clock.observe(&a, 0.02, 1, true).motion, Motion::Attack);
        let p = clock.observe(&a, 0.2, 13, true);
        assert_eq!(p.motion, Motion::Attack);
        assert!((p.age - 0.2).abs() < 0.001);
        a.hp = 0;
        a.deaths = 1;
        let p = clock.observe(&a, 0.02, 14, true);
        assert_eq!(p.motion, Motion::Death);
        assert_eq!(p.age, 0.0);
        assert_eq!(clock.observe(&a, 1.0, 74, true).age, 1.0);
        a.hp = 120;
        let p = clock.observe(&a, 0.02, 75, true);
        assert_eq!(p.motion, Motion::Spawn);
        assert_eq!(p.age, 0.0);
    }
}
