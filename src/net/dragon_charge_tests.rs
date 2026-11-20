use super::*;
use crate::race_skill;

#[test]
fn dragon_charge_snapshot_roundtrips_timer_latch_cooldown_and_full_roster_mtu() {
    for active in [true, false] {
        let mut world = World::default();
        world.event = "용족 돌진 몸통박치기".repeat(20);
        for a in &mut world.actors {
            a.character = 0;
            a.ammo = kit(0).magazine;
            a.race_skill.remaining = if active {
                race_skill::CHARGE_DURATION
            } else {
                0.0
            };
            a.race_skill.cooldown = if active {
                0.0
            } else {
                race_skill::CHARGE_COOLDOWN
            };
            a.race_skill.down = true;
            a.race_skill.fire_latched = true; // Can persist after switching from a burrow.
            a.race_skill.hit_mask = 1 << 6;
        }
        let bytes = snapshot(&world, 42, 5);
        assert!(bytes.len() < MAX_PACKET);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        for (a, b) in world.actors.iter().zip(&decoded.actors) {
            assert_eq!(race_skill::charging(b), active);
            assert!((a.race_skill.remaining - b.race_skill.remaining).abs() < DT);
            assert!((a.race_skill.cooldown - b.race_skill.cooldown).abs() < DT);
            assert!(b.race_skill.down);
            assert!(b.race_skill.fire_latched);
            assert_eq!(b.race_skill.hit_mask, 0, "damage stays authoritative");
        }
    }
}

#[test]
fn dragon_charge_rejects_invalid_wire_state() {
    for state in 0..4 {
        let mut world = World::default();
        let a = &mut world.actors[0];
        a.character = 0;
        a.race_skill.remaining = race_skill::CHARGE_DURATION;
        match state {
            0 => a.race_skill.remaining += 1.0,
            1 => a.race_skill.cooldown = 1.0,
            2 => {
                a.race_skill.charge = 0.3;
                a.race_skill.fire_latched = true;
            }
            _ => {
                a.race_skill.remaining = 0.0;
                a.race_skill.cooldown = 9.0;
            }
        }
        assert!(
            read_snapshot(&snapshot(&world, 42, 5), 42).is_none(),
            "case {state}"
        );
    }
}

#[test]
fn udp_dragon_charge_reaches_owner_observer_and_late_joiner_then_cools_down() {
    let mut server = Server::bind("127.0.0.1:0", false).unwrap();
    let mut clients: Vec<_> = (0..2)
        .map(|_| Client::connect(server.addr().unwrap()).unwrap())
        .collect();
    fn pump(server: &mut Server, clients: &mut [Client], shift: bool, ticks: usize) {
        for _ in 0..ticks {
            for (i, c) in clients.iter_mut().enumerate() {
                c.update(Input {
                    phase_shift: i == 0 && shift,
                    ..Default::default()
                })
                .unwrap();
            }
            server.update(Input::default()).unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        for c in clients {
            c.update(Input::default()).unwrap();
        }
    }
    pump(&mut server, &mut clients, false, 20);
    let slot = clients[0].slot.unwrap();
    server.world.phase = Phase::Playing;
    server.world.map.clear();
    server.world.actors[slot].character = 0;
    server.world.actors[slot].ammo = kit(0).magazine;
    server.world.actors[slot].pos = V3::default();
    pump(&mut server, &mut clients, true, 8);
    assert!(race_skill::charging(&server.world.actors[slot]));
    clients.push(Client::connect(server.addr().unwrap()).unwrap());
    pump(&mut server, &mut clients, false, 12);
    for c in &clients {
        assert!(race_skill::charging(&c.world.actors[slot]));
    }
    pump(&mut server, &mut clients, false, 180);
    for c in &clients {
        assert!(!race_skill::charging(&c.world.actors[slot]));
        assert!(c.world.actors[slot].race_skill.cooldown > 7.0);
    }
}
