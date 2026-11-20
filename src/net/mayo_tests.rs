use super::*;
use crate::mayo_skill::{Phase as Action, State};
#[test]
fn mayo_cast_and_poison_roundtrip_without_packet_growth() {
    let mut w = World::default();
    let original = snapshot(&w, 42, 1).len();
    w.actors[0].character = 8;
    w.actors[0].ammo = 0;
    for phase in [Action::Off, Action::Skill, Action::Ultimate] {
        w.actors[0].combat.mayo = State {
            phase,
            elapsed: if phase == Action::Off {
                0.0
            } else {
                phase.duration() - DT
            },
        };
        w.actors[6].combat.poison = 4.0;
        let bytes = snapshot(&w, 42, 1);
        assert_eq!(bytes.len(), original);
        assert!(bytes.len() < MAX_DATAGRAM);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        assert_eq!(decoded.actors[0].combat.mayo.phase, phase);
        assert!(
            (decoded.actors[0].combat.mayo.elapsed - w.actors[0].combat.mayo.elapsed).abs() < DT
        );
        assert!(decoded.actors[6].combat.poison > 0.0);
    }
    assert!(State::unpack(Action::Off, 1).is_none());
    assert!(State::unpack(Action::Ultimate, u32::MAX).is_none());
    assert!(State::unpack(Action::Skill, 120).is_none());
    assert!(Action::from_wire(3).is_none());
}
