use super::*;
use crate::chloe_skill::{self as c, Phase as Action, State};

#[test]
fn chloe_forms_duration_and_hit_counter_roundtrip_without_growing_packets() {
    let mut w = World::default();
    let baseline = snapshot(&w, 42, 1).len();
    w.actors[0].character = 6;
    for phase in [
        Action::Off,
        Action::Mount,
        Action::Ride,
        Action::Roar,
        Action::Dismount,
        Action::Attack,
        Action::RideAttack,
    ] {
        let riding = matches!(phase, Action::Ride | Action::Roar | Action::RideAttack);
        w.actors[0].combat.chloe = State {
            phase,
            elapsed: if matches!(phase, Action::Off | Action::Ride) {
                0.0
            } else {
                phase.duration() - DT
            },
            remaining: if riding { 11.25 } else { 0.0 },
            hits: if riding { 8 } else { 0 },
        };
        let bytes = snapshot(&w, 42, 1);
        assert_eq!(bytes.len(), baseline);
        assert!(bytes.len() < MAX_DATAGRAM);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        let s = decoded.actors[0].combat.chloe;
        assert_eq!(s.phase, phase);
        assert_eq!(s.hits, w.actors[0].combat.chloe.hits);
        assert!((s.elapsed - w.actors[0].combat.chloe.elapsed).abs() < DT);
        assert!((s.remaining - w.actors[0].combat.chloe.remaining).abs() < DT);
    }
    assert!(State::unpack(Action::Ride, u32::MAX).is_none());
    assert!(State::unpack(Action::Off, 1).is_none());
    assert!(State::unpack(Action::Attack, 9 << 17).is_none());
}
#[test]
fn chloe_confetti_age_and_doll_heading_survive_projectile_snapshots() {
    let mut w = World::default();
    w.actors[0].character = 6;
    let mut doll = crate::combat::Projectile::visual(
        101,
        0,
        crate::combat::ProjectileKind::Doll,
        V3::new(1.0, 0.5, 3.0),
    );
    doll.velocity = V3::new(7.0, 0.0, 7.0);
    doll.life = c::DOLL_SECONDS;
    let mut burst = doll.clone();
    burst.id = 102;
    burst.kind = crate::combat::ProjectileKind::Confetti;
    burst.age = 0.3;
    burst.life = 0.4;
    burst.velocity = V3::default();
    w.projectiles = vec![doll, burst];
    let (_, got) = read_projectiles(&projectile_snapshot(&w, 42, 1), 42).unwrap();
    assert_eq!(got[0].kind, crate::combat::ProjectileKind::Doll);
    assert!(got[0].velocity.x > 0.0 && got[0].velocity.z > 0.0);
    assert_eq!(got[1].kind, crate::combat::ProjectileKind::Confetti);
    assert!((got[1].age - 0.3).abs() < 0.02);
}
