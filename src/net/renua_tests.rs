use super::*;
use crate::{
    motion::MotionClock,
    renua_skill::{self as r, Phase as Action, State},
};

#[test]
fn renua_actions_temporal_exposure_and_hurt_share_existing_snapshot_budget() {
    let mut w = World::default();
    let base = snapshot(&w, 42, 1).len();
    w.actors[0].character = 7;
    w.actors[0].ammo = 0;
    for phase in [
        Action::Off,
        Action::Basic,
        Action::Enhanced,
        Action::Skill,
        Action::Ultimate,
    ] {
        w.actors[0].combat.renua = State {
            phase,
            elapsed: if phase == Action::Off {
                0.0
            } else {
                phase.duration() - 0.002
            },
            alt_down: true,
        };
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.combat.temporal = i as f32 / (PLAYERS - 1) as f32;
            a.hurt = 0.35;
        }
        let bytes = snapshot(&w, 42, 2);
        assert_eq!(bytes.len(), base);
        assert!(bytes.len() < MAX_PACKET);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        let a = &decoded.actors[0];
        assert_eq!(a.combat.renua.phase, phase);
        assert!((a.combat.renua.elapsed - w.actors[0].combat.renua.elapsed).abs() < DT + 0.0001);
        assert!(a.combat.renua.alt_down);
        if phase != Action::Off {
            assert_eq!(
                MotionClock::default().observe(a, 0.0, 1, true).motion,
                phase.motion().unwrap()
            );
        }
        for (a, b) in w.actors.iter().zip(&decoded.actors) {
            assert!((a.combat.temporal - b.combat.temporal).abs() < 0.002);
            assert!((a.hurt - b.hurt).abs() < 0.001);
        }
        assert!(decoded.actors[11].combat.disabled());
    }
}
#[test]
fn ninety_six_fields_roundtrip_heading_without_packet_growth() {
    let mut w = World::default();
    w.projectiles.clear();
    for i in 0..MAX_PROJECTILES {
        let mut p = Projectile::visual(
            i as u32,
            i % PLAYERS,
            ProjectileKind::TimeField,
            V3::new(0.0, 0.0, 1.5),
        );
        p.velocity = direction(i as f32 * 0.17, 0.0);
        p.life = r::FIELD_SECONDS;
        w.projectiles.push(p);
    }
    let bytes = projectile_snapshot(&w, 42, 7);
    assert!(bytes.len() < MAX_PACKET);
    let (_, decoded) = read_projectiles(&bytes, 42).unwrap();
    for (a, b) in w.projectiles.iter().zip(decoded) {
        assert_eq!(b.kind, ProjectileKind::TimeField);
        assert!(b.life > 0.0);
        assert!(a.velocity.normalized().dot(b.velocity.normalized()) > 0.999);
    }
}
#[test]
fn local_udp_observers_and_late_joiners_receive_renua_cast_and_team_neutral_freeze() {
    let mut server = Server::bind("127.0.0.1:0", false).unwrap();
    let mut clients: Vec<_> = (0..2)
        .map(|_| Client::connect(server.addr().unwrap()).unwrap())
        .collect();
    fn pump(server: &mut Server, clients: &mut [Client]) {
        for _ in 0..18 {
            for c in clients.iter_mut() {
                c.update(Input::default()).unwrap();
            }
            server.update(Input::default()).unwrap();
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    pump(&mut server, &mut clients);
    server.world.actors[0].character = 7;
    server.world.actors[0].ammo = 0;
    server.world.actors[0].combat.renua = State {
        phase: Action::Skill,
        elapsed: 2.1,
        alt_down: true,
    };
    server.world.actors[2].combat.temporal = 0.5;
    server.world.actors[6].combat.temporal = 1.0;
    let mut p = Projectile::visual(81, 0, ProjectileKind::TimeField, V3::new(0.0, 0.0, 1.5));
    p.velocity = direction(0.7, 0.0);
    p.life = 5.0;
    server.world.projectiles.push(p);
    clients.push(Client::connect(server.addr().unwrap()).unwrap());
    pump(&mut server, &mut clients);
    for c in clients {
        assert!(c.slot.is_some());
        assert_eq!(c.world.actors[0].combat.renua.phase, Action::Skill);
        assert!((c.world.actors[2].combat.temporal - 0.5).abs() < 0.002);
        assert_eq!(c.world.actors[6].combat.temporal, 1.0);
        assert!(c
            .world
            .projectiles
            .iter()
            .any(|p| p.kind == ProjectileKind::TimeField));
    }
}
