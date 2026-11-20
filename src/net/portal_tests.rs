use super::*;
use crate::portal::{Portal, State};

fn pump(server: &mut Server, clients: &mut [Client], ticks: usize) {
    for _ in 0..ticks {
        for client in clients.iter_mut() {
            client.update(Input::default()).unwrap();
        }
        server.update(Input::default()).unwrap();
    }
    for client in clients {
        client.update(Input::default()).unwrap();
    }
}

#[test]
fn portal_packet_roundtrip_fits_mtu_and_rejects_bad_faces_timers_and_tokens() {
    let mut world = World::default();
    for (i, a) in world.actors.iter_mut().enumerate() {
        a.portals = State {
            ends: [
                Some(Portal {
                    center: V3::new(i as f32, 1.5, 4.0),
                    face: 5,
                }),
                Some(Portal {
                    center: V3::new(i as f32, 0.0, -4.0),
                    face: 2,
                }),
            ],
            cooldown: [5.0, 2.0],
            down: [true, false],
            lock: crate::portal::REOPEN_DELAY,
            reopen: crate::portal::REOPEN_DELAY,
            momentum: V3::new(1.0, 0.0, -12.0),
            yaw_offset: 3.0,
            pitch_offset: -0.3,
            transits: 7,
        };
    }
    let packet = portal_snapshot(&world, 42, 9);
    assert!(packet.len() < MAX_DATAGRAM);
    let (seq, states) = read_portals(&packet, 42).unwrap();
    assert_eq!(seq, 9);
    for (a, s) in world.actors.iter().zip(states) {
        assert_eq!(a.portals.ends, s.ends);
        assert_eq!(a.portals.cooldown, s.cooldown);
        assert_eq!(a.portals.lock, s.lock);
        assert_eq!(a.portals.reopen, s.reopen);
        assert_eq!(s.down, [true, false]);
        assert_eq!(s.transits, 7);
        assert_eq!(s.yaw_offset, 3.0);
    }
    assert!(read_portals(&packet, 43).is_none());
    for len in 0..packet.len() {
        assert!(read_portals(&packet[..len], 42).is_none());
    }
    let mut bad = packet.clone();
    bad[21] = 6;
    assert!(read_portals(&bad, 42).is_none());
    let mut bad = packet.clone();
    bad[47..49].copy_from_slice(&301_u16.to_le_bytes());
    assert!(read_portals(&bad, 42).is_none());
    for offset in [52, 53] {
        let mut bad = packet.clone();
        bad[offset] = 61;
        assert!(read_portals(&bad, 42).is_none());
    }
}

#[test]
fn right_click_input_survives_wire_and_rejects_unknown_flags() {
    let packet = input_packet(
        42,
        7,
        Input {
            phase_shift: true,
            alt_fire: true,
            ..Default::default()
        },
    );
    let (_, _, input) = read_input(&packet).unwrap();
    assert!(input.alt_fire && input.phase_shift && !input.fire);
    let mut bad = packet;
    *bad.last_mut().unwrap() = 2;
    assert!(read_input(&bad).is_none());
}

#[test]
fn movement_and_portals_commit_together_after_packet_reordering() {
    let mut client = Client::connect("127.0.0.1:28080").unwrap();
    let mut world = World::default();
    world.tick = 81;
    world.actors[0].pos = V3::new(10.0, 0.0, 4.0);
    let mut states = [State::default(); PLAYERS];
    states[0].transits = 1;
    states[0].yaw_offset = 3.14;
    client.pending_worlds.push_back((
        9,
        world.clone(),
        InputAck {
            slot: 0,
            sequence: 0,
        },
    ));
    client.commit_world();
    assert_eq!(client.received_sequence, 0);
    client
        .pending_portals
        .push_back((12, [State::default(); PLAYERS]));
    client.commit_world();
    assert_eq!(client.received_sequence, 0);
    client.pending_portals.push_back((9, states));
    client.commit_world();
    assert_eq!(client.received_sequence, 9);
    assert_eq!(client.world.actors[0].portals.transits, 1);
    assert_eq!(client.world.actors[0].pos, world.actors[0].pos);
    client.pending_worlds.push_back((
        12,
        world,
        InputAck {
            slot: 0,
            sequence: 0,
        },
    ));
    client.commit_world();
    assert_eq!(client.received_sequence, 12);
    assert_eq!(client.world.actors[0].portals.transits, 0);
}

#[test]
fn udp_elf_portals_reach_owner_observer_and_late_joiner() {
    let mut server = Server::bind("127.0.0.1:0", false).unwrap();
    let mut clients: Vec<_> = (0..2)
        .map(|_| Client::connect(server.addr().unwrap()).unwrap())
        .collect();
    pump(&mut server, &mut clients, 20);
    let slot = clients[0].slot.unwrap();
    server.world.phase = Phase::Playing;
    server.world.map = vec![Block {
        center: V3::new(0.0, 2.5, 4.0),
        size: V3::new(8.0, 5.0, 0.3),
        color: [0; 3],
        phaseable: true,
    }];
    server.world.actors[slot].character = 3;
    server.world.actors[slot].pos = V3::default();
    for _ in 0..8 {
        clients[0]
            .update(Input {
                phase_shift: true,
                alt_fire: true,
                ..Default::default()
            })
            .unwrap();
        server.update(Input::default()).unwrap();
        clients[1].update(Input::default()).unwrap();
    }
    assert!(server.world.actors[slot].portals.ends[1].is_some());
    server.world.actors[slot].portals.reopen = crate::portal::REOPEN_DELAY;
    clients.push(Client::connect(server.addr().unwrap()).unwrap());
    pump(&mut server, &mut clients, 20);
    for c in &clients {
        assert_eq!(
            c.world.actors[slot].portals.ends,
            server.world.actors[slot].portals.ends
        );
        assert!(c.world.actors[slot].portals.cooldown[1] > 0.0);
        assert!(c.world.actors[slot].portals.reopen > 0.0);
        assert_eq!(c.world.actors[slot].portals.passable_ends(), [None; 2]);
    }
    pump(&mut server, &mut clients, 65);
    for c in &clients {
        assert_eq!(c.world.actors[slot].portals.reopen, 0.0);
    }
}
