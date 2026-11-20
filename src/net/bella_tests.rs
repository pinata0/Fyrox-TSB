use super::*;
use crate::bella_skill::{self as b, Phase as Action, State};
#[test]
fn bella_clock_and_soul_mask_roundtrip_in_the_existing_snapshot_budget() {
    let mut w = World::default();
    let baseline = snapshot(&w, 42, 1).len();
    w.actors[0].character = 9;
    w.actors[0].ammo = 0;
    for phase in [Action::Off, Action::Skill, Action::Ultimate] {
        w.actors[0].combat.bella = State {
            phase,
            elapsed: if phase == Action::Off {
                0.0
            } else {
                phase.duration() - DT
            },
            targets: if phase == Action::Ultimate {
                (1 << 6) | (1 << 11)
            } else {
                0
            },
        };
        w.actors[6].combat.taunt = 3.0;
        let bytes = snapshot(&w, 42, 1);
        assert_eq!(bytes.len(), baseline);
        assert!(bytes.len() < MAX_DATAGRAM);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        let state = decoded.actors[0].combat.bella;
        assert_eq!(state.phase, phase);
        assert_eq!(state.targets, w.actors[0].combat.bella.targets);
        assert!((state.elapsed - w.actors[0].combat.bella.elapsed).abs() < DT);
        assert!(decoded.actors[6].combat.skills_locked());
        assert!(!decoded.actors[6].combat.disabled());
    }
}
#[test]
fn rectangle_heading_and_healing_wave_age_fit_a_full_projectile_datagram() {
    let mut w = World::default();
    for i in 0..MAX_PROJECTILES {
        let kind = if i % 2 == 0 {
            ProjectileKind::BellaFire
        } else {
            ProjectileKind::BellaHeal
        };
        let mut p = Projectile::visual(i as u32, i % PLAYERS, kind, V3::new(3.0, 0.0, 5.0));
        p.velocity = direction(1.2, 0.0);
        p.age = 0.35;
        w.projectiles.push(p);
    }
    let bytes = projectile_snapshot(&w, 42, 1);
    assert!(bytes.len() < MAX_DATAGRAM);
    let (_, decoded) = read_projectiles(&bytes, 42).unwrap();
    assert_eq!(decoded.len(), MAX_PROJECTILES);
    for p in decoded {
        if p.kind == ProjectileKind::BellaFire {
            assert!(p.velocity.dot(direction(1.2, 0.0)) > 0.999);
            assert_eq!(p.life, b::FIRE_SECONDS);
        } else {
            assert!((p.age - 0.35).abs() < 0.001);
        }
    }
}
