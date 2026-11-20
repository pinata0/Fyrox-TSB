use super::*;
use crate::vivi_skill::{self as v, Phase as Action, State};
#[test]
fn vivi_actions_and_mercury_fields_roundtrip_without_growing_packets() {
    let mut w = World::default();
    w.actors[0].character = 0;
    w.actors[0].ammo = 0;
    let size = snapshot(&w, 42, 1).len();
    for phase in [
        Action::Off,
        Action::Left,
        Action::Right,
        Action::Skill,
        Action::Ultimate,
    ] {
        w.actors[0].combat.vivi = State {
            phase,
            elapsed: if phase == Action::Off {
                0.0
            } else {
                phase.duration() - DT
            },
        };
        w.actors[0].combat.guard = 48.0;
        let bytes = snapshot(&w, 42, 1);
        assert_eq!(bytes.len(), size);
        assert!(bytes.len() < MAX_DATAGRAM);
        let (_, d, _) = read_snapshot(&bytes, 42).unwrap();
        assert_eq!(d.actors[0].combat.vivi.phase, phase);
        assert!((d.actors[0].combat.vivi.elapsed - w.actors[0].combat.vivi.elapsed).abs() < DT);
        assert!(d.actors[0].combat.mercury);
    }
    assert!(State::unpack(Action::Off, 1).is_none());
    assert!(State::unpack(Action::Ultimate, 240).is_none());
    assert!(Action::from_wire(5).is_none());
    w.projectiles = (0..MAX_PROJECTILES)
        .map(|i| {
            let mut p = Projectile::visual(
                i as u32,
                i % PLAYERS,
                ProjectileKind::MercuryField,
                V3::default(),
            );
            p.age = 5.75;
            p
        })
        .collect();
    let bytes = projectile_snapshot(&w, 42, 1);
    assert!(bytes.len() < MAX_DATAGRAM);
    let (_, d) = read_projectiles(&bytes, 42).unwrap();
    assert_eq!(d.len(), MAX_PROJECTILES);
    assert!((d[0].age - 5.75).abs() < 0.026);
    assert!((d[0].life - (v::FIELD_SECONDS - 5.75)).abs() < 0.026);
}
