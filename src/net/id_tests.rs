use super::*;
use crate::{
    id_skill::{Phase as IdPhase, State},
    motion::{Motion, MotionClock},
};

#[test]
fn id_actions_protection_and_secondary_clock_reach_late_observers_without_packet_growth() {
    let mut w = World::default();
    w.actors[0].character = 3;
    w.actors[0].ammo = 0;
    let bytes = snapshot(&w, 42, 1).len();
    for phase in [
        IdPhase::Off,
        IdPhase::Skill,
        IdPhase::Pulse,
        IdPhase::Ultimate,
    ] {
        w.actors[0].combat.id = State {
            phase,
            elapsed: if phase == IdPhase::Off { 0.0 } else { 0.3 },
            invulnerable: 2.0,
            pulse_cooldown: 2.5,
            alt_down: true,
        };
        w.actors[1].combat.ward = 2.0;
        w.actors[1].hp = crate::id_skill::HEALTH_FLOOR;
        let packet = snapshot(&w, 42, 1);
        assert_eq!(packet.len(), bytes);
        assert!(bytes < MAX_DATAGRAM);
        let (_, decoded, _) = read_snapshot(&packet, 42).unwrap();
        let a = &decoded.actors[0];
        assert_eq!(a.combat.id.phase, phase);
        assert!((a.combat.id.elapsed - w.actors[0].combat.id.elapsed).abs() < DT);
        assert_eq!(a.combat.id.invulnerable, 2.0);
        assert!((a.combat.id.pulse_cooldown - 2.5).abs() < 0.0001);
        assert!(a.combat.id.alt_down);
        assert!(decoded.actors[1].combat.ward > 0.0);
        assert_eq!(decoded.actors[1].hp, crate::id_skill::HEALTH_FLOOR);
        if phase != IdPhase::Off {
            assert_eq!(
                MotionClock::default().observe(a, 0.0, w.tick, true).motion,
                phase.motion().unwrap()
            );
        }
    }
    let mut previous = snapshot(&w, 42, 1);
    previous[..4].copy_from_slice(b"TSBP");
    assert!(read_snapshot(&previous, 42).is_none());
    assert_eq!(IdPhase::Ultimate.motion(), Some(Motion::IdUltimate));
}
