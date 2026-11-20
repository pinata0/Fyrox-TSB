use super::*;
use crate::hilde_skill::{self as h, Phase as Action, State};
#[test]
fn medical_equipment_roundtrips_without_increasing_snapshot_size() {
    let mut w = World::default();
    let base = snapshot(&w, 42, 1).len();
    w.actors[0].character = 13;
    for (phase, elapsed) in [
        (Action::Off, 0.0),
        (Action::Skill, h::HEAL_RELEASE + 1.0),
        (Action::Equip, 1.0),
        (Action::Launcher, h::EQUIP_SECONDS),
        (Action::Launcher, h::EQUIP_SECONDS + 5.9),
        (Action::End, h::EQUIP_SECONDS + h::LAUNCHER_SECONDS + 0.4),
    ] {
        w.actors[0].combat.hilde = State {
            phase,
            elapsed,
            cooldown: if phase == Action::Launcher {
                h::FIRE_INTERVAL
            } else {
                0.0
            },
            firing: phase == Action::Launcher,
        };
        let bytes = snapshot(&w, 42, 1);
        assert_eq!(bytes.len(), base);
        assert!(bytes.len() < MAX_PACKET);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        let state = decoded.actors[0].combat.hilde;
        assert_eq!(state.phase, phase);
        assert!((state.elapsed - elapsed).abs() < DT + 0.0001);
        assert_eq!(state.firing, phase == Action::Launcher);
    }
    w.projectiles = (0..MAX_PROJECTILES)
        .map(|i| {
            Projectile::visual(
                i as u32,
                i % PLAYERS,
                ProjectileKind::MedicalGrenade,
                V3::new(1.0, 2.0, 3.0),
            )
        })
        .collect();
    let bytes = projectile_snapshot(&w, 42, 1);
    assert!(bytes.len() < MAX_PACKET);
    let (_, decoded) = read_projectiles(&bytes, 42).unwrap();
    assert!(decoded
        .iter()
        .all(|p| p.kind == ProjectileKind::MedicalGrenade));
}
#[test]
fn malformed_medical_clocks_are_rejected() {
    assert!(State::unpack(Action::Off, 1).is_none());
    assert!(State::unpack(Action::Skill, 1 << 15).is_none());
    assert!(State::unpack(Action::Launcher, 0).is_none());
    assert!(State::unpack(Action::Equip, 1 << 16).is_none());
}

#[test]
fn medical_explosions_roundtrip_with_contact_position_and_original_timestamp() {
    let mut w = World { tick: 100, ..Default::default() };
    for i in 0..MAX_IMPACTS {
        w.record_medical_explosion(V3::new(i as f32, 1.5, 3.0));
    }
    w.tick += 10;
    let bytes = impact_snapshot(&w, 42, 9);
    assert!(bytes.len() < MAX_PACKET);
    let decoded = read_impacts(&bytes, 42).unwrap();
    assert_eq!(decoded.impacts, w.impacts);
    assert!(decoded.impacts.iter().all(|hit|
        hit.kind == ImpactKind::MedicalExplosion && hit.tick == 100
    ));
    w.impacts[0].radius = h::GRENADE_RADIUS + 1.0;
    assert!(read_impacts(&impact_snapshot(&w, 42, 10), 42).is_none());
}
