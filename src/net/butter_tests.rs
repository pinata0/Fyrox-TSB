use super::*;
use crate::butter_skill::{Phase as Action, State};
#[test]
fn butter_hold_release_and_skills_roundtrip_without_packet_growth() {
    let mut w = World::default();
    let base = snapshot(&w, 42, 1).len();
    w.actors[0].character = 11;
    w.actors[0].ammo = kit(11).magazine;
    for phase in [
        Action::Off,
        Action::Draw,
        Action::Release,
        Action::Skill,
        Action::Ultimate,
    ] {
        w.actors[0].combat.butter = State {
            phase,
            elapsed: phase.duration() * 0.5,
            fire_down: true,
        };
        let bytes = snapshot(&w, 42, 1);
        assert_eq!(bytes.len(), base);
        assert!(bytes.len() < MAX_PACKET);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        let state = decoded.actors[0].combat.butter;
        assert_eq!(state.phase, phase);
        assert!(state.fire_down);
        assert!((state.elapsed - w.actors[0].combat.butter.elapsed).abs() < DT);
    }
}
#[test]
fn full_boulder_blast_packet_preserves_effect_age() {
    let mut w = World::default();
    w.projectiles = (0..MAX_PROJECTILES)
        .map(|i| {
            let mut p = Projectile::visual(
                i as u32,
                i % PLAYERS,
                ProjectileKind::BoulderBlast,
                V3::new(0.0, 1.0, 0.0),
            );
            p.age = (i % 13) as f32 * 0.05;
            p
        })
        .collect();
    let bytes = projectile_snapshot(&w, 42, 10);
    assert!(bytes.len() < MAX_PACKET);
    let (_, decoded) = read_projectiles(&bytes, 42).unwrap();
    for (a, b) in w.projectiles.iter().zip(decoded) {
        assert_eq!(a.kind, b.kind);
        assert!((a.age - b.age).abs() < 0.001);
    }
}
