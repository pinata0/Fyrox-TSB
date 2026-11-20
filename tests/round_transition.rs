use trickcal_shooting_balddagu::movement::WALK_SPEED;
use std::{thread, time::Duration};
use trickcal_shooting_balddagu::{
    combat::{Projectile, ProjectileKind},
    game::*,
    net::{Client, Server},
    round_transition::*,
};

fn fixture() -> World {
    let mut w = World::room(Default::default(), true);
    w.join_player(TEAM_SIZE);
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    w.actors[0].pos = V3::new(0.0, 0.0, -20.0);
    w.actors[TEAM_SIZE].pos = V3::new(10.0, 0.0, -20.0);
    w
}

fn advance(w: &mut World, ticks: usize) {
    for _ in 0..ticks {
        w.tick([Input::default(); PLAYERS]);
    }
}

#[test]
fn final_momentum_slows_then_freezes_without_processing_new_actions() {
    let mut w = fixture();
    let mut input = [Input::default(); PLAYERS];
    input[0].forward = 1.0;
    w.tick(input);
    w.finish_round(true);
    let pos = w.actors[0].pos;
    let hp = w.actors[TEAM_SIZE].hp;
    let shot = w.actors[0].shot;
    let scores = w.scores;
    let remaining = w.remaining;
    input[0].fire = true;
    input[0].ability = true;
    input[0].ultimate = true;
    input[0].jump = true;
    input[0].forward = -1.0;
    for _ in 0..30 {
        w.tick(input);
    }
    assert!((w.actors[0].pos.z - pos.z - WALK_SPEED * 0.5 * SLOW_MOTION_SCALE).abs() < 0.002);
    assert!((w.phase_time - (ROUND_RESULT_SECONDS - 0.5)).abs() < 0.001);
    assert_eq!(w.actors[0].shot, shot);
    assert_eq!(w.actors[TEAM_SIZE].hp, hp);
    assert_eq!(w.scores, scores);
    assert_eq!(w.remaining, remaining);
    advance(&mut w, 30);
    let frozen = w.actors[0].pos;
    assert_eq!(w.presentation_time_scale(), 0.0);
    advance(&mut w, 100);
    assert_eq!(w.actors[0].pos, frozen);
    assert_eq!(w.result, Some(0));
    advance(&mut w, 210);
    assert_eq!(w.phase, Phase::Setup);
    assert_eq!(w.attacker, 1);
    assert_eq!(w.result, None);
    assert_eq!(w.presentation_time_scale(), 1.0);
}

#[test]
fn aftermath_projectiles_move_without_damage_and_stop_at_walls() {
    let mut w = fixture();
    let target = w.actors[TEAM_SIZE].pos + V3::new(0.0, 1.0, 0.0);
    let mut projectile =
        Projectile::visual(1, 0, ProjectileKind::Stone, target - V3::new(0.0, 0.0, 0.1));
    projectile.velocity = V3::new(0.0, 0.0, 20.0);
    projectile.life = 3.0;
    projectile.damage = 999.0;
    w.projectiles.push(projectile);
    w.finish_round(false);
    let hp = w.actors[TEAM_SIZE].hp;
    advance(&mut w, 15);
    assert_eq!(w.actors[TEAM_SIZE].hp, hp);
    assert!(w.projectiles[0].pos.z > target.z);
    let position = w.projectiles[0].pos;
    w.map.push(Block {
        center: position + V3::new(0.0, 0.0, 0.1),
        size: V3::new(2.0, 2.0, 0.1),
        color: [0; 3],
        phaseable: false,
    });
    advance(&mut w, 4);
    assert!(w.projectiles.is_empty());
}

#[test]
fn round_winner_uses_capture_or_hold_even_when_extra_time_resets_round_records() {
    for attacker in 0..2 {
        for full in [false, true] {
            let mut w = fixture();
            w.attacker = attacker;
            w.finish_round(full);
            assert_eq!(w.result, Some(if full { attacker } else { 1 - attacker }));
        }
    }
    let mut w = fixture();
    w.finish_round(true);
    advance(&mut w, 361);
    w.finish_round(true);
    assert!(w.extra);
    assert_eq!(w.rounds[1].points, 0);
    assert_eq!(w.result, Some(1));
}

#[test]
fn final_draw_uses_same_schedule_and_returns_to_lobby_on_real_time() {
    let mut w = fixture();
    w.finish_round(false);
    advance(&mut w, 361);
    w.finish_round(false);
    assert_eq!(w.phase, Phase::Finished);
    assert_eq!(w.result, Some(2));
    assert_eq!(w.presentation_time_scale(), SLOW_MOTION_SCALE);
    advance(&mut w, 60);
    assert_eq!(w.presentation_time_scale(), 0.0);
    assert_eq!(w.result, Some(2));
    advance(&mut w, 241);
    assert_eq!(w.phase, Phase::Lobby);
    assert_eq!(w.result, None);
}

#[test]
fn udp_clients_receive_winner_and_phase_clock_including_late_join() {
    fn pump(server: &mut Server, client: &mut Client, ticks: usize) {
        for _ in 0..ticks {
            client.update(Input::default()).unwrap();
            server.update(Input::default()).unwrap();
            thread::sleep(Duration::from_millis(1));
        }
    }
    let mut server = Server::bind("127.0.0.1:0", true).unwrap();
    let mut client = Client::connect(server.addr().unwrap()).unwrap();
    pump(&mut server, &mut client, 24);
    server.start();
    server.world.finish_round(false);
    pump(&mut server, &mut client, 15);
    assert_eq!(client.world.phase, Phase::Intermission);
    assert_eq!(client.world.result, Some(1));
    assert!(client.world.result_elapsed() < SLOW_MOTION_SECONDS);
    assert!((client.world.phase_time - server.world.phase_time).abs() < 0.1);
    pump(&mut server, &mut client, 60);
    assert_eq!(client.world.presentation_time_scale(), 0.0);
    let mut late = Client::connect(server.addr().unwrap()).unwrap();
    pump(&mut server, &mut late, 18);
    assert_eq!(late.world.phase, Phase::Intermission);
    assert_eq!(late.world.result, Some(1));
    assert!(late.world.result_elapsed() > SLOW_MOTION_SECONDS);
    server.world.phase_time = DT;
    pump(&mut server, &mut client, 12);
    assert_eq!(client.world.phase, Phase::Setup);
    assert_eq!(client.world.result, None);
}
