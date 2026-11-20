use super::*;
use crate::ui_skill::{Phase as Action, State};
#[test]
fn ui_basic_cycle_roundtrips_for_clover_beams_without_growing_packets() {
    let mut w = World::default();
    w.actors[0].character = 10;
    let original = snapshot(&w, 42, 1).len();
    for attacks in [0, 1, 2, 3, 4, 5, 8, 20, u32::MAX] {
        w.actors[0].combat.attacks = attacks;
        let bytes = snapshot(&w, 42, 1);
        assert_eq!(bytes.len(), original);
        assert!(bytes.len() < MAX_DATAGRAM);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        let cycle = decoded.actors[0].combat.attacks;
        assert_eq!(cycle, if attacks == 0 { 0 } else { (attacks - 1) % 4 + 1 });
        assert_eq!(cycle > 0 && cycle.is_multiple_of(4), attacks > 0 && attacks.is_multiple_of(4));
        assert_eq!(decoded.actors[0].combat.magic_ammo, 0);
    }
}

#[test]
fn viewport_is_transmitted_for_server_visibility_and_invalid_values_are_rejected() {
    let mut bytes = input_packet(
        42,
        7,
        Input {
            view_aspect: 2.4,
            ..Default::default()
        },
    );
    assert_eq!(read_input(&bytes).unwrap().2.view_aspect, 2.4);
    let n = bytes.len();
    bytes[n - 4..].copy_from_slice(&5.0_f32.to_le_bytes());
    assert!(read_input(&bytes).is_none());
}
#[test]
fn ui_casts_rain_and_frog_timers_roundtrip_without_growing_the_datagram() {
    let mut w = World::default();
    let original = snapshot(&w, 42, 1).len();
    w.actors[0].character = 10;
    w.actors[0].ammo = 20;
    for phase in [Action::Off, Action::Skill, Action::Ultimate] {
        w.actors[0].combat.ui = State {
            phase,
            elapsed: if phase == Action::Off {
                0.0
            } else {
                phase.duration() - DT
            },
        };
        for a in &mut w.actors {
            a.combat.polymorph = 3.0;
            a.combat.rain_slow = DT * 2.0;
            a.hurt = 0.25;
        }
        let bytes = snapshot(&w, 42, 1);
        assert_eq!(bytes.len(), original);
        assert!(bytes.len() < MAX_DATAGRAM);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        assert_eq!(decoded.actors[0].combat.ui.phase, phase);
        assert!((decoded.actors[0].combat.ui.elapsed - w.actors[0].combat.ui.elapsed).abs() < DT);
        for a in decoded.actors {
            assert!((a.combat.polymorph - 3.0).abs() < 0.00001);
            assert!(a.combat.rain_slow > 0.0);
            assert_eq!(a.hurt, 0.25);
            assert!(a.combat.disabled());
            assert!(!a.combat.movement_disabled());
        }
    }
    assert!(State::unpack(Action::Off, 1).is_none());
    assert!(State::unpack(Action::Ultimate, u32::MAX).is_none());
    assert!(Action::from_wire(3).is_none());
}
#[test]
fn all_rain_fields_fit_the_projectile_packet_and_keep_their_age() {
    let mut w = World::default();
    w.projectiles.clear();
    for i in 0..MAX_PROJECTILES {
        let mut p = Projectile::visual(
            i as u32,
            i % PLAYERS,
            ProjectileKind::RainField,
            V3::default(),
        );
        p.age = 2.35;
        w.projectiles.push(p);
    }
    let bytes = projectile_snapshot(&w, 42, 1);
    assert!(bytes.len() < MAX_DATAGRAM);
    let (_, decoded) = read_projectiles(&bytes, 42).unwrap();
    assert_eq!(decoded.len(), MAX_PROJECTILES);
    for p in decoded {
        assert_eq!(p.kind, ProjectileKind::RainField);
        assert!((p.age - 2.35).abs() < 0.026);
        assert!(p.life > 1.6);
    }
}
