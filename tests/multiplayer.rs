use std::{thread, time::Duration};
use trickcal_shooting_balddagu::{
    game::*,
    movement::WALK_SPEED,
    net::{Client, Server},
};

#[test]
fn client_predicts_between_udp_snapshots_then_accepts_server_correction() {
    let mut server = Server::bind("127.0.0.1:0", true).unwrap();
    let mut client = Client::connect(server.addr().unwrap()).unwrap();
    for _ in 0..30 {
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        thread::sleep(Duration::from_millis(1));
    }
    let slot = client.slot.unwrap();
    server.start();
    server.world.phase = Phase::Playing;
    server.world.actors[slot].pos = server.world.spawn_center(team(slot));
    for _ in 0..12 {
        server.update(Input::default()).unwrap();
        client.update(Input::default()).unwrap();
        thread::sleep(Duration::from_millis(1));
    }
    let start = client.world.actors[slot].pos;
    let tick = client.world.tick;
    // Stop servicing the host: the next six client frames receive no reply.
    for _ in 0..6 {
        client
            .update(Input {
                forward: 1.0,
                ..Default::default()
            })
            .unwrap();
    }
    let drawn = client.motion.actor(&client.world, slot, slot).pos;
    assert!(drawn.z > start.z + WALK_SPEED * DT * 5.0);
    assert_eq!(client.world.tick, tick);
    assert_eq!(client.world.actors[slot].pos, start);
    for _ in 0..45 {
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        thread::sleep(Duration::from_millis(1));
    }
    assert!(client.rtt().is_some());
    let drawn = client.motion.actor(&client.world, slot, slot).pos;
    assert!((drawn - server.world.actors[slot].pos).length() < 0.03);
}

#[test]
fn two_udp_clients_join_opposing_teams_move_and_receive_restart() {
    let mut server = Server::bind("127.0.0.1:0", false).unwrap();
    let mut a = Client::connect(server.addr().unwrap()).unwrap();
    let mut b = Client::connect(server.addr().unwrap()).unwrap();
    for _ in 0..30 {
        a.update(Input::default()).unwrap();
        b.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        thread::sleep(Duration::from_millis(1));
    }
    let ai = a.slot.unwrap();
    let bi = b.slot.unwrap();
    assert_ne!(team(ai), team(bi));
    assert_eq!(server.peer_count(), 2);
    assert!(a.received && b.received);
    server.start();
    server.world.phase = Phase::Playing;
    // Exercise network movement along the castle spawn's open central doorway.
    server.world.actors[ai].pos = server.world.spawn_center(team(ai));
    let start = server.world.actors[ai].pos;
    for _ in 0..65 {
        a.update(Input {
            forward: 1.0,
            ..Default::default()
        })
        .unwrap();
        b.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        thread::sleep(Duration::from_millis(1));
    }
    assert!(server.world.actors[ai].pos.z > start.z + WALK_SPEED * DT * 45.0);
    for _ in 0..8 {
        a.update(Input::default()).unwrap();
        b.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
    }
    assert!((a.world.actors[ai].pos - server.world.actors[ai].pos).length() < 0.3);
    assert!((b.world.actors[ai].pos - server.world.actors[ai].pos).length() < 0.3);
    let old_sequence = a.received_sequence;
    server.start();
    for _ in 0..8 {
        a.update(Input::default()).unwrap();
        b.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
    }
    assert!(a.received_sequence > old_sequence);
    assert_eq!(a.world.phase, Phase::Setup);
    assert!(a.world.tick < 20);
}

#[test]
fn host_reserves_slot_and_malformed_datagrams_do_not_change_state() {
    use std::net::UdpSocket;
    let mut server = Server::bind("127.0.0.1:0", true).unwrap();
    let mut client = Client::connect(server.addr().unwrap()).unwrap();
    let raw = UdpSocket::bind("127.0.0.1:0").unwrap();
    raw.send_to(&[0; 1300], server.addr().unwrap()).unwrap();
    raw.send_to(b"TSB3\x03", server.addr().unwrap()).unwrap();
    for _ in 0..12 {
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
    }
    assert_eq!(client.slot, Some(TEAM_SIZE));
    assert_eq!(server.world.phase, Phase::Lobby);
    assert_eq!(server.world.actors[0].hp, 120);
}
