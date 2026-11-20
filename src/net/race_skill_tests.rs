use super::*;

#[test]
fn surge_snapshots_roundtrip_charge_bank_for_all_races_and_reject_invalid_states() {
    for id in 0..ROSTER.len() as u8 {
        let mut world = World {
            event: "충전 상태와 이동 방향 동기화 검증".repeat(20),
            ..Default::default()
        };
        for a in &mut world.actors {
            a.character = id;
            a.ammo = kit(id).magazine;
            a.race_skill.surge_charges = 1;
            a.race_skill.surge_recharge = 1.5;
        }
        if crate::race_skill::has_surge(ROSTER[id as usize].race) {
            world.actors[0].race_skill.remaining = 0.1;
            world.actors[0].race_skill.velocity = V3::new(25.0, 0.0, 0.0);
            world.actors[0].race_skill.down = true;
        }
        let bytes = snapshot(&world, 42, 5);
        assert!(bytes.len() < MAX_PACKET);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        for a in &decoded.actors {
            assert_eq!(a.race_skill.surge_charges, 1);
            assert!((a.race_skill.surge_recharge - 1.5).abs() < DT);
        }
        if crate::race_skill::has_surge(ROSTER[id as usize].race) {
            assert!(decoded.actors[0].race_skill.active());
            assert!(decoded.actors[0].race_skill.down);
            assert_eq!(
                decoded.actors[0].race_skill.velocity,
                V3::new(25.0, 0.0, 0.0)
            );
            world.actors[0].race_skill.velocity.x = 100.0;
            assert!(read_snapshot(&snapshot(&world, 42, 5), 42).is_none());
            world.actors[0].race_skill.velocity.x = 25.0;
        }
        world.actors[0].race_skill.surge_recharge = 4.0;
        assert!(read_snapshot(&snapshot(&world, 42, 5), 42).is_none());
        world.actors[0].race_skill.surge_recharge = 0.0;
        assert!(read_snapshot(&snapshot(&world, 42, 5), 42).is_none());
    }
}

#[test]
fn udp_surge_spends_and_recharges_for_owner_observer_and_late_joiner() {
    let mut server = Server::bind("127.0.0.1:0", false).unwrap();
    let mut clients: Vec<_> = (0..2)
        .map(|_| Client::connect(server.addr().unwrap()).unwrap())
        .collect();
    pump(&mut server, &mut clients, 20);
    let slot = clients[0].slot.unwrap();
    server.world.phase = Phase::Playing;
    server.world.map.clear();
    server.world.actors[slot].character = 4;
    server.world.actors[slot].ammo = kit(4).magazine;
    server.world.actors[slot].pos = V3::default();
    for _ in 0..3 {
        for _ in 0..3 {
            clients[0]
                .update(Input {
                    phase_shift: true,
                    ..Default::default()
                })
                .unwrap();
            server.update(Input::default()).unwrap();
            clients[1].update(Input::default()).unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        pump(&mut server, &mut clients, 9);
    }
    assert_eq!(server.world.actors[slot].race_skill.surge_charges, 0);
    assert!((server.world.actors[slot].pos.z - 11.25).abs() < 0.001);
    clients.push(Client::connect(server.addr().unwrap()).unwrap());
    pump(&mut server, &mut clients, 12);
    for client in &clients {
        assert_eq!(client.world.actors[slot].race_skill.surge_charges, 0);
        assert!(client.world.actors[slot].race_skill.surge_recharge > 2.0);
        assert!((client.world.actors[slot].pos.z - 11.25).abs() < 0.001);
    }
    pump(&mut server, &mut clients, 150);
    for client in &clients {
        assert_eq!(client.world.actors[slot].race_skill.surge_charges, 1);
    }
}

fn pump(server: &mut Server, clients: &mut [Client], ticks: usize) {
    for _ in 0..ticks {
        for client in &mut *clients {
            client.update(Input::default()).unwrap();
        }
        server.update(Input::default()).unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    for client in clients {
        client.update(Input::default()).unwrap();
    }
}

#[test]
fn race_skill_snapshot_roundtrips_held_charge_leap_velocity_and_full_roster_mtu() {
    for local_id in [1, 10, 11, 12] {
        let mut world = World {
            event: "잠복과 도약 상태를 모든 플레이어에게 동기화합니다".repeat(10),
            ..Default::default()
        };
        for (slot, a) in world.actors.iter_mut().enumerate() {
            a.character = if slot == 0 {
                local_id
            } else {
                [1, 10, 11, 12][slot % 4]
            };
            a.ammo = kit(a.character).magazine;
            let beast = matches!(a.character, 11 | 12);
            a.race_skill = crate::race_skill::State {
                remaining: 3.0,
                cooldown: if beast { 5.0 } else { 0.0 },
                charge: if beast { 0.0 } else { 0.3 },
                down: true,
                fire_latched: !beast,
                velocity: if beast {
                    V3::new(8.0, 0.0, 8.0)
                } else {
                    V3::default()
                },
                ..Default::default()
            };
        }
        let packet = snapshot(&world, 42, 5);
        assert!(packet.len() < MAX_PACKET);
        let (_, decoded, _) = read_snapshot(&packet, 42).unwrap();
        for (a, b) in world.actors.iter().zip(&decoded.actors) {
            assert_eq!(a.race_skill.active(), b.race_skill.active());
            assert!((a.race_skill.remaining - b.race_skill.remaining).abs() < DT);
            assert!((a.race_skill.cooldown - b.race_skill.cooldown).abs() < DT);
            assert!((a.race_skill.charge - b.race_skill.charge).abs() < DT);
            assert_eq!(a.race_skill.fire_latched, b.race_skill.fire_latched);
            assert!(b.race_skill.down);
            assert!(!b.phase_skill.active());
        }
        assert_eq!(
            world.actors[0].race_skill.velocity,
            decoded.actors[0].race_skill.velocity
        );
        world.actors[0].race_skill.charge = 1.0;
        assert!(read_snapshot(&snapshot(&world, 42, 5), 42).is_none());
    }
}

#[test]
fn udp_burrow_and_leap_reach_owner_observer_and_late_joiner() {
    for id in [10, 11] {
        let mut server = Server::bind("127.0.0.1:0", false).unwrap();
        let mut clients: Vec<_> = (0..2)
            .map(|_| Client::connect(server.addr().unwrap()).unwrap())
            .collect();
        pump(&mut server, &mut clients, 20);
        let slot = clients[0].slot.unwrap();
        server.world.phase = Phase::Playing;
        server.world.map.clear();
        server.world.actors[slot].character = id;
        server.world.actors[slot].ammo = kit(id).magazine;
        server.world.actors[slot].pos = V3::default();
        for _ in 0..8 {
            clients[0]
                .update(Input {
                    phase_shift: true,
                    ..Default::default()
                })
                .unwrap();
            server.update(Input::default()).unwrap();
            clients[1].update(Input::default()).unwrap();
        }
        assert!(server.world.actors[slot].race_skill.active());
        clients.push(Client::connect(server.addr().unwrap()).unwrap());
        pump(&mut server, &mut clients, 12);
        for client in &clients {
            assert!(client.world.actors[slot].race_skill.active());
            assert_eq!(client.world.actors[slot].burrowed(), id == 10);
        }
        if id == 10 {
            for _ in 0..48 {
                clients[0]
                    .update(Input {
                        fire: true,
                        ..Default::default()
                    })
                    .unwrap();
                server.update(Input::default()).unwrap();
                for client in &mut clients[1..] {
                    client.update(Input::default()).unwrap();
                }
            }
            assert_eq!(server.world.actors[slot].shot, 0);
            for client in &clients {
                assert!(!client.world.actors[slot].burrowed());
                assert!(client.world.actors[slot].race_skill.cooldown > 7.0);
            }
        }
    }
}
