use std::{thread, time::Duration};
use trickcal_shooting_balddagu::{
    chat::ChatChannel,
    combat::{Projectile, ProjectileKind},
    game::*,
    kits::kit,
    level::MapId,
    net::{Client, Server},
    roster::SelectionResult,
};

fn advance(world: &mut World, seconds: f32) {
    for _ in 0..(seconds / DT).ceil() as usize {
        world.tick([Input::default(); PLAYERS]);
    }
}

fn finish_match(world: &mut World, winner: usize) {
    world.point = usize::from(winner == 0);
    world.finish_round(false);
    assert_eq!(world.phase, Phase::Intermission);
    advance(world, 6.1);
    world.point = usize::from(winner == 1);
    world.finish_round(false);
    assert_eq!(world.phase, Phase::Finished);
    assert_eq!(world.result, Some(winner));
}

#[test]
fn every_result_returns_to_the_same_lobby_and_allows_a_fresh_match() {
    for map in [MapId::FairyKingdomCastle, MapId::ElfCity] {
        for winner in 0..=2 {
            let mut world = World::room(map, true);
            world.join_player(TEAM_SIZE);
            assert_eq!(world.set_bot(1, Some(14)), SelectionResult::Accepted);
            let occupants = world
                .actors
                .each_ref()
                .map(|a| (a.active, a.human, a.character));
            world.start();
            finish_match(&mut world, winner);
            world.actors[0].kills = 8;
            world.actors[0].deaths = 3;
            world.actors[0].hp = 0;
            world.actors[0].killed_by = Some(TEAM_SIZE);
            world.actors[0].respawn = 7.0;
            world.actors[0].combat.ultimate = 100.0;
            world.actors[0].ability_cooldown = 20.0;
            world.projectiles.push(Projectile::visual(
                1,
                0,
                ProjectileKind::Stone,
                V3::default(),
            ));
            let result_tick = world.tick;
            advance(&mut world, MATCH_RESULT_SECONDS - 0.2);
            assert_eq!(world.phase, Phase::Finished);
            assert_eq!(world.result, Some(winner));
            assert_eq!(world.actors[0].kills, 8);
            advance(&mut world, 0.3);

            assert_eq!(world.phase, Phase::Lobby);
            assert_eq!(world.map_id, map);
            assert_eq!(
                world
                    .actors
                    .each_ref()
                    .map(|a| (a.active, a.human, a.character)),
                occupants
            );
            assert!(world.tick > result_tick);
            assert_eq!(world.result, None);
            assert_eq!(world.scores, [0; 2]);
            assert_eq!(world.round, 1);
            assert_eq!(world.attacker, 0);
            assert_eq!(world.point, 0);
            assert_eq!(world.progress, 0.0);
            assert_eq!(world.phase_time, 0.0);
            assert!(world.projectiles.is_empty());
            for actor in &world.actors {
                assert_eq!(actor.kills, 0);
                assert_eq!(actor.deaths, 0);
                assert_eq!(actor.combat.ultimate, 0.0);
                assert_eq!(actor.ability_cooldown, 0.0);
                assert_eq!(actor.killed_by, None);
                assert_eq!(actor.hp, if actor.active { MAX_HP } else { 0 });
                if actor.active {
                    assert_eq!(actor.ammo, kit(actor.character).magazine);
                }
            }
            advance(&mut world, 1.0);
            assert_eq!(
                world.phase,
                Phase::Lobby,
                "wait for the host to start again"
            );
            assert_eq!(world.set_bot(1, None), SelectionResult::Accepted);
            world.start();
            assert_eq!(world.phase, Phase::Setup);
            assert_eq!(world.map_id, map);
            assert!(!world.actors[1].active);
            assert!(world.actors[TEAM_SIZE].human);
        }
    }
}

#[test]
fn result_countdown_keeps_current_connections_instead_of_restoring_departed_players() {
    let mut world = World::room(MapId::ElfCity, true);
    world.join_player(TEAM_SIZE);
    world.start();
    finish_match(&mut world, 2);
    world.clear_slot(TEAM_SIZE);
    world.join_player(TEAM_SIZE + 1);
    advance(&mut world, MATCH_RESULT_SECONDS + 0.1);
    assert_eq!(world.phase, Phase::Lobby);
    assert!(!world.actors[TEAM_SIZE].active);
    assert!(world.actors[TEAM_SIZE + 1].human);
}

fn pump(server: &mut Server, clients: &mut [Client], ticks: usize) {
    for _ in 0..ticks {
        for client in clients.iter_mut() {
            client.update(Input::default()).unwrap();
        }
        server.update(Input::default()).unwrap();
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn connected_clients_return_together_keep_chat_and_receive_the_next_match() {
    let mut server = Server::bind("127.0.0.1:0", true).unwrap();
    server.world.select_map(MapId::ElfCity);
    let address = server.addr().unwrap();
    let mut clients = [
        Client::connect(address).unwrap(),
        Client::connect(address).unwrap(),
    ];
    pump(&mut server, &mut clients, 24);
    let slots = clients.each_ref().map(|c| c.slot.unwrap());
    assert_eq!(server.world.set_bot(2, Some(14)), SelectionResult::Accepted);
    server.send_chat(ChatChannel::Match, "same room").unwrap();
    server.start();
    pump(&mut server, &mut clients, 12);
    finish_match(&mut server.world, 0);
    pump(&mut server, &mut clients, 12);
    for client in &clients {
        assert_eq!(client.world.phase, Phase::Finished);
        assert_eq!(client.world.result, Some(0));
    }
    let sequences = clients.each_ref().map(|c| c.received_sequence);
    pump(
        &mut server,
        &mut clients,
        (MATCH_RESULT_SECONDS / DT).ceil() as usize + 12,
    );
    assert_eq!(server.addr().unwrap(), address);
    assert_eq!(server.peer_count(), 2);
    for (index, client) in clients.iter().enumerate() {
        assert_eq!(client.slot, Some(slots[index]));
        assert_eq!(client.world.phase, Phase::Lobby);
        assert_eq!(client.world.map_id, MapId::ElfCity);
        assert!(client.world.actors[2].active && !client.world.actors[2].human);
        assert_eq!(client.world.actors[2].character, 14);
        assert!(client.received_sequence > sequences[index]);
        assert!(client.error.is_none());
        assert!(client.chat.messages.iter().any(|m| m.text == "same room"));
    }
    clients[0]
        .send_chat(ChatChannel::Match, "back in lobby")
        .unwrap();
    pump(&mut server, &mut clients, 12);
    assert!(clients[1]
        .chat
        .messages
        .iter()
        .any(|m| m.text == "back in lobby"));
    server.start();
    pump(&mut server, &mut clients, 12);
    for client in &clients {
        assert_eq!(client.world.phase, Phase::Setup);
        assert_eq!(client.world.result, None);
        assert_eq!(client.world.scores, [0; 2]);
    }
}
