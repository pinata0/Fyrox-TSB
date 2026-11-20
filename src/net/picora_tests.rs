use super::*;
use crate::picora_skill::{Phase as Action, State};
#[test]
fn picora_cast_target_and_all_recipient_buffs_fit_the_existing_datagram() {
    let mut w = World::default();
    let baseline = snapshot(&w, 42, 1).len();
    w.actors[0].character = 5;
    w.actors[0].ammo = 18;
    for phase in [Action::Off, Action::Skill, Action::Ultimate] {
        w.actors[0].combat.picora = State {
            phase,
            elapsed: if phase == Action::Off {
                0.0
            } else {
                phase.duration() - DT
            },
            target: if phase == Action::Off { None } else { Some(1) },
            shot_delay: 0.0,
        };
        for a in &mut w.actors {
            a.combat.fashion = 8.0;
            a.combat.sticker = 4.0;
            a.hp = 240;
        }
        let bytes = snapshot(&w, 42, 1);
        assert_eq!(bytes.len(), baseline);
        assert!(bytes.len() < MAX_DATAGRAM);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        let s = decoded.actors[0].combat.picora;
        assert_eq!(s.phase, phase);
        assert_eq!(s.target, w.actors[0].combat.picora.target);
        assert!((s.elapsed - w.actors[0].combat.picora.elapsed).abs() < DT);
        for a in decoded.actors {
            assert_eq!(a.hp, 240);
            assert_eq!(a.max_hp(), 240);
            assert_eq!(a.combat.sticker, 4.0);
            assert_eq!(a.combat.fashion, 8.0);
        }
    }
}
#[test]
fn invalid_picora_states_and_unbuffed_overhealth_are_rejected() {
    assert!(State::unpack(Action::Off, 1).is_none());
    assert!(State::unpack(Action::Skill, 0).is_none());
    assert!(State::unpack(Action::Ultimate, u32::MAX).is_none());
    let mut w = World::default();
    w.actors[0].hp = 240;
    assert!(read_snapshot(&snapshot(&w, 42, 1), 42).is_none());
    w.actors[0].character = 5;
    w.actors[0].ammo = 18;
    w.actors[0].hp = 120;
    w.actors[0].combat.picora.shot_delay = 0.18;
    let (_, decoded, _) = read_snapshot(&snapshot(&w, 42, 1), 42).unwrap();
    assert!(decoded.actors[0].combat.picora.shot_delay >= 0.18);

    w.actors[0].hp = 190;
    w.actors[0].barrier = 19.2;
    w.actors[0].combat.fashion = DT;
    crate::picora_skill::advance_buffs(&mut w.actors[0]);
    let (_, decoded, _) = read_snapshot(&snapshot(&w, 42, 1), 42).unwrap();
    assert_eq!(decoded.actors[0].hp, 95);
    assert_eq!(decoded.actors[0].max_hp(), MAX_HP);
    assert!((decoded.actors[0].barrier - 9.6).abs() < 0.001);
}
