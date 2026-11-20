use super::*;
use crate::{
    aya_skill::{self as aya, Phase as AyaPhase, State},
    motion::MotionClock,
};
#[test]
fn aya_actions_frost_and_weaken_roundtrip_without_growing_full_roster_snapshot() {
    let mut world = World::default();
    let original = snapshot(&world, 42, 1).len();
    world.actors[0].character = 4;
    world.actors[0].ammo = 0;
    for phase in [AyaPhase::Off, AyaPhase::Skill, AyaPhase::Ultimate] {
        world.actors[0].combat.aya = State {
            phase,
            elapsed: if phase == AyaPhase::Off {
                0.0
            } else {
                phase.duration() - DT
            },
        };
        world.actors[6].combat.frozen = aya::FREEZE_SECONDS;
        world.actors[6].combat.weaken = aya::WEAKEN_SECONDS;
        let bytes = snapshot(&world, 42, 1);
        assert_eq!(bytes.len(), original);
        assert!(bytes.len() < MAX_DATAGRAM);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        let a = &decoded.actors[0];
        assert_eq!(a.combat.aya.phase, phase);
        assert!((a.combat.aya.elapsed - world.actors[0].combat.aya.elapsed).abs() < DT);
        assert!((decoded.actors[6].combat.frozen - aya::FREEZE_SECONDS).abs() < 0.00001);
        assert!(decoded.actors[6].combat.weaken > 0.0);
        assert!(decoded.actors[6].combat.view_locked());
        if phase != AyaPhase::Off {
            assert_eq!(
                MotionClock::default().observe(a, 0.0, 1, true).motion,
                phase.motion().unwrap()
            );
        }
    }
    for phase in [AyaPhase::Skill, AyaPhase::Ultimate] {
        assert!(State::unpack(phase, u32::MAX).is_none());
        assert!(State::unpack(phase, (phase.duration() / DT).round() as u32).is_none());
    }
    assert!(State::unpack(AyaPhase::Off, 1).is_none());
    assert!(AyaPhase::from_wire(3).is_none());
}
#[test]
fn aya_snow_age_and_butterfly_heading_fit_the_ninety_six_projectile_packet() {
    let mut world = World::default();
    world.projectiles.clear();
    for i in 0..MAX_PROJECTILES {
        let mut p = Projectile::visual(
            i as u32,
            i % PLAYERS,
            if i % 2 == 0 {
                ProjectileKind::SnowGarden
            } else {
                ProjectileKind::Butterfly
            },
            V3::default(),
        );
        p.age = 2.35;
        p.velocity = direction(0.7, 0.0);
        world.projectiles.push(p);
    }
    let bytes = projectile_snapshot(&world, 42, 1);
    assert!(bytes.len() < MAX_DATAGRAM);
    let (_, decoded) = read_projectiles(&bytes, 42).unwrap();
    for (a, b) in world.projectiles.iter().zip(&decoded) {
        assert_eq!(a.kind, b.kind);
        if a.kind == ProjectileKind::SnowGarden {
            assert!((a.age - b.age).abs() < 0.026);
            assert!(b.life > 0.0);
        } else {
            assert!(a.velocity.normalized().dot(b.velocity.normalized()) > 0.999);
        }
    }
}
#[test]
fn aya_cast_and_statuses_reach_existing_and_late_udp_clients() {
    let mut server = Server::bind("127.0.0.1:0", false).unwrap();
    let mut clients = vec![Client::connect(server.addr().unwrap()).unwrap()];
    fn pump(server: &mut Server, clients: &mut [Client]) {
        for _ in 0..20 {
            for c in clients.iter_mut() {
                c.update(Input::default()).unwrap();
            }
            server.update(Input::default()).unwrap();
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    pump(&mut server, &mut clients);
    server.world.actors[0].character = 4;
    server.world.actors[0].ammo = 0;
    server.world.actors[0].combat.aya = State {
        phase: AyaPhase::Ultimate,
        elapsed: 2.1,
    };
    server.world.actors[8].combat.frozen = 2.4;
    server.world.actors[8].combat.weaken = 3.0;
    let mut p = Projectile::visual(81, 0, ProjectileKind::SnowGarden, V3::default());
    p.age = 2.1;
    p.life = 3.4;
    server.world.projectiles.push(p);
    clients.push(Client::connect(server.addr().unwrap()).unwrap());
    pump(&mut server, &mut clients);
    for c in clients {
        assert!(c.slot.is_some());
        assert_eq!(c.world.actors[0].combat.aya.phase, AyaPhase::Ultimate);
        assert!((c.world.actors[8].combat.frozen - 2.4).abs() < DT);
        assert!(c.world.actors[8].combat.weaken > 0.0);
        assert!(c
            .world
            .projectiles
            .iter()
            .any(|p| p.kind == ProjectileKind::SnowGarden && (p.age - 2.1).abs() < 0.026));
    }
}
