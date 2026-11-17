use trickcal_shooting_balddagu::{
    game::*,
    level::MapId,
    net::{Client, Server},
    roster::SelectionResult,
};

#[test]
fn solo_can_move_without_bots_respawning_or_match_timing_out() {
    for map in MapId::ALL {
        let mut w = World::room(map, true);
        assert_eq!(w.team_count(0), 1);
        assert_eq!(w.team_count(1), 0);
        assert_eq!(w.damage_bonus(0), 0);
        assert_eq!(w.damage_bonus(1), 0);
        w.start();
        for _ in 0..500 {
            w.tick([Input::default(); PLAYERS]);
        }
        assert_eq!(w.phase, Phase::Playing);
        w.actors[0].pos = w.spawn_center(0);
        let origin = w.actors[0].pos;
        for _ in 0..60 {
            let mut inputs = [Input::default(); PLAYERS];
            inputs[0].forward = 1.0;
            w.tick(inputs);
        }
        assert!((w.actors[0].pos - origin).length() > 1.0);
        w.actors[0].pos = w.points()[0];
        for _ in 0..15000 {
            w.tick([Input::default(); PLAYERS]);
        }
        assert_eq!(w.phase, Phase::Playing);
        assert_eq!(w.remaining, 240.0);
        assert_eq!(w.progress, 0.0);
        assert!(w.actors[1..]
            .iter()
            .all(|a| !a.active && !a.alive() && a.hp == 0));
        assert!(w.projectiles.is_empty());
    }
}

#[test]
fn bot_configuration_obeys_team_rules_and_survives_map_start_restart() {
    let mut w = World::room(MapId::default(), true);
    assert_eq!(w.set_bot(0, None), SelectionResult::Locked);
    assert_eq!(w.set_bot(PLAYERS, Some(0)), SelectionResult::Invalid);
    assert_eq!(w.set_bot(1, Some(99)), SelectionResult::Invalid);
    assert_eq!(w.set_bot(1, Some(0)), SelectionResult::Taken);
    assert_eq!(w.set_bot(1, Some(3)), SelectionResult::Accepted);
    assert_eq!(w.set_bot(2, Some(6)), SelectionResult::Accepted);
    assert_eq!(w.set_bot(3, Some(9)), SelectionResult::PositionFull);
    assert!(!w.actors[3].active);
    assert_eq!(w.set_bot(6, Some(0)), SelectionResult::Accepted);
    assert_eq!(w.set_bot(1, Some(4)), SelectionResult::Accepted);
    assert_eq!(w.set_bot(2, None), SelectionResult::Accepted);
    assert_eq!(w.set_bot(2, Some(4)), SelectionResult::Taken);
    assert_eq!(w.position_counts(0), [1, 1, 0]);
    assert!(!w.can_select(2));
    assert_eq!(w.select_character(2, 1), SelectionResult::Invalid);
    assert!(w.select_map(MapId::ElfCity));
    for _ in 0..2 {
        w.start();
        assert_eq!(w.team_count(0), 2);
        assert_eq!(w.team_count(1), 1);
        assert_eq!(w.actors[1].character, 4);
        assert!(!w.actors[2].active);
        assert_eq!(w.set_bot(1, None), SelectionResult::Locked);
    }
    w.phase = Phase::Finished;
    assert_eq!(w.set_bot(1, None), SelectionResult::Accepted);
}

fn exchange(server: &mut Server, client: &mut Client) {
    for _ in 0..18 {
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn network_syncs_bot_add_change_remove_and_preserves_empty_slots() {
    let mut server = Server::bind("127.0.0.1:0", true).unwrap();
    assert_eq!(server.world.team_count(0), 1);
    assert_eq!(server.world.team_count(1), 0);
    assert_eq!(server.world.set_bot(6, Some(8)), SelectionResult::Accepted);
    let mut client = Client::connect(server.addr().unwrap()).unwrap();
    exchange(&mut server, &mut client);
    assert!(client.received);
    // Humans occupy empty slots before displacing an explicitly configured bot.
    assert_ne!(client.slot, Some(6));
    assert!(client.world.actors[6].active);
    assert_eq!(client.world.actors[6].character, 8);
    assert_eq!(server.world.set_bot(6, Some(14)), SelectionResult::Accepted);
    exchange(&mut server, &mut client);
    assert_eq!(client.world.actors[6].character, 14);
    assert_eq!(server.world.set_bot(6, None), SelectionResult::Accepted);
    exchange(&mut server, &mut client);
    assert!(!client.world.actors[6].active);
    assert!(!client.world.actors[6].alive());
    server.start();
    exchange(&mut server, &mut client);
    assert_eq!(client.world.actors.iter().filter(|a| a.active).count(), 2);
    assert_eq!(client.world.phase, Phase::Setup);
}

#[test]
fn joining_an_empty_slot_chooses_a_legal_hero_and_full_teams_keep_bot_hero() {
    let mut w = World::room(MapId::default(), true);
    assert_eq!(w.set_bot(2, Some(12)), SelectionResult::Accepted);
    w.join_player(1);
    assert!(w.actors[1].active && w.actors[1].human);
    assert_ne!(w.actors[1].character, 12);
    w.join_player(2);
    assert!(w.actors[2].human);
    assert_eq!(w.actors[2].character, 12);
    w.clear_slot(2);
    w.start();
    assert!(!w.actors[2].active);
}
