use super::*;
use crate::erpin_skill::{self as e, Phase as Action, State};
#[test]
fn erpin_skill_phases_and_clock_fit_existing_snapshot_without_losing_cake_latch() {
    let mut w = World::default();
    let len = snapshot(&w, 42, 1).len();
    for a in &mut w.actors {
        a.character = 2;
        a.ammo = 20;
    }
    for phase in [
        Action::Skill,
        Action::Start,
        Action::Charge,
        Action::Recover,
    ] {
        for a in &mut w.actors {
            a.combat.erpin_skill = State {
                phase,
                elapsed: phase.duration() - DT,
            };
            a.combat.erpin.alt_down = true;
            a.combat.stun = if phase == Action::Recover { DT } else { 0.0 };
        }
        let packet = snapshot(&w, 42, 1);
        assert_eq!(packet.len(), len);
        assert!(packet.len() < MAX_PACKET);
        let (_, decoded, _) = read_snapshot(&packet, 42).unwrap();
        for a in decoded.actors {
            assert_eq!(a.combat.erpin_skill.phase, phase);
            assert!((a.combat.erpin_skill.elapsed - (phase.duration() - DT)).abs() < 0.001);
            assert!(a.combat.erpin.alt_down);
            assert!(!a.combat.erpin.eating);
            assert_eq!(a.combat.stun > 0.0, phase == Action::Recover);
        }
    }
    assert!(State::unpack(0).is_none());
    assert!(State::unpack(7).is_none());
    assert!(State::unpack(u32::MAX).is_none());
    assert!(State::unpack(
        State {
            phase: Action::Recover,
            elapsed: 3.0
        }
        .pack(false)
    )
    .is_none());
}
#[test]
fn erpin_explosion_ages_roundtrip_at_full_projectile_capacity() {
    let mut w = World::default();
    for i in 0..MAX_PROJECTILES {
        let kind = if i % 2 == 0 {
            ProjectileKind::ErpinCrash
        } else {
            ProjectileKind::ErpinBlast
        };
        let mut p = Projectile::visual(i as u32, i % PLAYERS, kind, V3::new(1.0, 1.0, 1.0));
        p.age = 0.35;
        w.projectiles.push(p);
    }
    let packet = projectile_snapshot(&w, 42, 1);
    assert!(packet.len() < MAX_PACKET);
    let (_, ps) = read_projectiles(&packet, 42).unwrap();
    assert_eq!(ps.len(), MAX_PROJECTILES);
    for p in ps {
        assert!((p.age - 0.35).abs() < 0.001);
        assert!((p.life - (e::BLAST_SECONDS - 0.35)).abs() < 0.001);
    }
}
