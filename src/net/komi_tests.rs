use super::*;
use crate::komi_skill::{Phase as Action, State};
#[test]
fn komi_actions_and_wake_input_roundtrip_without_packet_growth() {
    let mut w = World::default();
    let base = snapshot(&w, 42, 1).len();
    w.actors[0].character = 12;
    for phase in [Action::Off, Action::Rest, Action::Feast, Action::Giant] {
        w.actors[0].combat.komi = State {
            phase,
            elapsed: if phase == Action::Off { 0.0 } else { 1.2 },
            e_down: phase == Action::Rest,
            impact: if phase == Action::Giant { 0.35 } else { 0.0 },
        };
        let bytes = snapshot(&w, 42, 1);
        assert_eq!(bytes.len(), base);
        assert!(bytes.len() < MAX_PACKET);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        let s = decoded.actors[0].combat.komi;
        assert_eq!(s.phase, phase);
        assert!((s.elapsed - w.actors[0].combat.komi.elapsed).abs() < DT);
        assert_eq!(s.e_down, phase == Action::Rest);
        assert!((s.impact - w.actors[0].combat.komi.impact).abs() < DT);
    }
    let bytes = input_packet(
        42,
        1,
        Input {
            wake_rest: true,
            alt_fire: true,
            ..Default::default()
        },
    );
    let (_, _, input) = read_input(&bytes).unwrap();
    assert!(input.wake_rest && input.alt_fire);
}
#[test]
fn invalid_komi_state_is_rejected() {
    assert!(Action::from_wire(4).is_none());
    assert!(State::unpack(Action::Off, 1).is_none());
    assert!(State::unpack(Action::Rest, 240).is_none());
    assert!(State::unpack(Action::Feast, 264).is_none());
    assert!(State::unpack(Action::Giant, 480).is_none());
    assert!(State::unpack(Action::Giant, 1 << 20).is_none());
    assert!(State::unpack(Action::Rest, 1 << 11).is_none());
}
