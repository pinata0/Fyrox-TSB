use trickcal_shooting_balddagu::movement::WALK_SPEED;
use trickcal_shooting_balddagu::game::*;

fn empty_match() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    for a in &mut w.actors {
        a.human = true;
        a.pos = V3::new(12.0, 0.0, -12.0);
        a.shield = 0.0;
    }
    w
}
fn advance_objective(w: &mut World, seconds: f32) {
    for _ in 0..(seconds / DT).ceil() as usize {
        w.update_objective();
    }
}
fn advance(w: &mut World, seconds: f32) {
    for _ in 0..(seconds / DT).ceil() as usize {
        w.tick([Input::default(); PLAYERS]);
    }
}

#[test]
fn b_is_locked_until_a_and_only_attackers_capture() {
    let mut w = empty_match();
    w.actors[0].pos = POINTS[1];
    advance_objective(&mut w, 2.0);
    assert_eq!(w.progress, 0.0);
    w.actors[TEAM_SIZE].pos = POINTS[0];
    advance_objective(&mut w, 2.0);
    assert_eq!(w.progress, 0.0);
    w.actors[TEAM_SIZE].pos.z = 44.0;
    w.actors[0].pos = POINTS[0];
    advance_objective(&mut w, 12.1);
    assert_eq!(w.point, 1);
    assert_eq!(w.scores, [1, 0]);
    assert!(w.remaining > 400.0);
}
#[test]
fn contest_stops_capture_and_checkpoints_survive_decay() {
    let mut w = empty_match();
    w.actors[0].pos = POINTS[0];
    advance_objective(&mut w, 5.0);
    let progress = w.progress;
    assert!(progress > 1.0 / 3.0);
    w.actors[TEAM_SIZE].pos = POINTS[0];
    advance_objective(&mut w, 3.0);
    assert!(w.contested);
    assert_eq!(w.progress, progress);
    w.actors[0].pos.z = -13.0;
    advance_objective(&mut w, 9.0);
    assert!((w.progress - 1.0 / 3.0).abs() < 0.0001);
    assert_eq!(w.peak, progress);
}
#[test]
fn overtime_ends_when_the_last_attacker_leaves() {
    let mut w = empty_match();
    w.remaining = 0.01;
    w.actors[0].pos = POINTS[0];
    advance_objective(&mut w, 0.1);
    assert!(w.overtime);
    w.actors[0].pos.z = -13.0;
    w.update_objective();
    assert_eq!(w.phase, Phase::Intermission);
    assert_eq!(w.result, Some(1));
    assert!(!w.overtime);
    advance(&mut w, 6.1);
    assert_eq!(w.attacker, 1);
    assert_eq!(w.point, 0);
    assert!(!w.overtime);
}
#[test]
fn both_b_captures_start_time_bank_without_a_bonus() {
    let mut w = empty_match();
    w.remaining = 200.0;
    w.finish_round(true);
    advance(&mut w, 6.1);
    w.remaining = 260.0;
    w.finish_round(true);
    assert!(w.extra);
    assert_eq!(w.banks, [120.0, 180.0]);
    advance(&mut w, 6.1);
    assert_eq!(w.attacker, 0);
    w.phase = Phase::Playing;
    w.progress = 0.999;
    w.actors[0].pos = POINTS[0];
    for i in 1..PLAYERS {
        w.actors[i].pos.z = -13.0;
    }
    let before = w.remaining;
    w.update_objective();
    assert_eq!(w.point, 1);
    assert!(w.remaining < before);
}
#[test]
fn partial_progress_tiebreaker_requires_first_tick() {
    let mut w = empty_match();
    w.peak = 0.2;
    w.finish_round(false);
    advance(&mut w, 6.1);
    w.peak = 0.3;
    w.finish_round(false);
    assert_eq!(w.result, Some(2));
    let mut w = empty_match();
    w.peak = 0.4;
    w.finish_round(false);
    advance(&mut w, 6.1);
    w.phase = Phase::Playing;
    w.peak = 0.41;
    w.update_objective();
    assert_eq!(w.result, Some(1));
}
#[test]
fn free_attack_wins_at_one_tick_or_draws() {
    for win in [false, true] {
        let mut w = empty_match();
        w.remaining = 0.0;
        w.finish_round(true);
        advance(&mut w, 6.1);
        w.remaining = 80.0;
        w.finish_round(true);
        assert!(w.free_attack);
        advance(&mut w, 6.1);
        assert_eq!(w.attacker, 1);
        if win {
            w.phase = Phase::Playing;
            w.peak = 1.0 / 3.0;
            w.update_objective();
            assert_eq!(w.result, Some(1));
        } else {
            w.finish_round(false);
            assert_eq!(w.result, Some(2));
        }
    }
}
#[test]
fn equal_empty_banks_draw_and_small_banks_get_equal_bonus() {
    let mut w = empty_match();
    w.remaining = 0.0;
    w.finish_round(true);
    advance(&mut w, 6.1);
    w.remaining = 0.0;
    w.finish_round(true);
    assert_eq!(w.result, Some(2));
    let mut w = empty_match();
    w.remaining = 20.0;
    w.finish_round(true);
    advance(&mut w, 6.1);
    w.remaining = 90.0;
    w.finish_round(true);
    assert_eq!(w.banks, [60.0, 130.0]);
}
#[test]
fn hitscan_damage_reload_respawn_and_spawn_shield() {
    let mut w = empty_match();
    w.actors[0].character = 3;
    w.actors[0].ammo = 40;
    w.actors[0].pos = V3::new(0.0, 0.0, -5.0);
    w.actors[TEAM_SIZE].pos = V3::new(0.0, 0.0, 0.0);
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0].fire = true;
    // A single Nata ray rounds damage once, unlike the old four sub-hits.
    // Allow the fourth shot to kill instead of relying on per-sub-hit rounding.
    for _ in 0..200 {
        w.tick(inputs);
    }
    assert!(!w.actors[TEAM_SIZE].alive());
    assert_eq!(w.actors[0].kills, 1);
    assert!(w.actors[0].ammo < 40);
    inputs[0].fire = false;
    inputs[0].reload = true;
    w.tick(inputs);
    assert!(w.actors[0].reload > 0.0);
    advance(&mut w, 1.7);
    assert_eq!(w.actors[0].ammo, 40);
    advance(&mut w, 5.5);
    assert!(w.actors[TEAM_SIZE].alive());
    assert!(w.actors[TEAM_SIZE].shield > 0.0);
}
#[test]
fn cover_blocks_bullets_and_teammates_take_no_damage() {
    let mut w = empty_match();
    w.actors[0].character = 3;
    w.actors[0].ammo = 40;
    w.actors[0].pos = V3::new(8.0, 0.0, -12.0);
    w.actors[TEAM_SIZE].pos = V3::new(8.0, 0.0, -4.0);
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0].fire = true;
    for _ in 0..60 {
        w.tick(inputs);
    }
    assert_eq!(w.actors[TEAM_SIZE].hp, 120);
    w.actors[0].pos = V3::new(0.0, 0.0, -8.0);
    w.actors[1].pos = V3::new(0.0, 0.0, -4.0);
    for _ in 0..60 {
        w.tick(inputs);
    }
    assert_eq!(w.actors[1].hp, 120);
}
#[test]
fn diagonal_speed_wall_collision_jump_and_restart() {
    let mut w = empty_match();
    w.actors[0].pos = V3::new(0.0, 0.0, -13.0);
    let before = w.actors[0].pos;
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = Input {
        forward: 1.0,
        strafe: 1.0,
        jump: true,
        ..Default::default()
    };
    w.tick(inputs);
    assert!((w.actors[0].pos - before).flat().length() <= WALK_SPEED * DT + 0.0001);
    assert!(w.actors[0].pos.y > 0.0);
    w.actors[0].pos = V3::new(-21.5, 0.0, -13.0);
    inputs[0].forward = 0.0;
    for _ in 0..120 {
        w.tick(inputs);
    }
    assert!(w.actors[0].pos.x > -21.7);
    w.start();
    assert_eq!(w.phase, Phase::Setup);
    assert_eq!(w.scores, [0, 0]);
    assert_eq!(w.actors[0].kills, 0);
    assert!(w.actors[0].human);
}

#[test]
fn bots_navigate_and_fight_without_leaving_the_arena() {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    let mut reached = false;
    for _ in 0..3600 {
        w.tick([Input::default(); PLAYERS]);
        reached |= w
            .actors
            .iter()
            .any(|a| a.alive() && (a.pos - POINTS[0]).flat().length() < RADIUS);
    }
    assert!(
        w.actors.iter().map(|a| a.kills as u32).sum::<u32>() > 0,
        "bots must engage each other"
    );
    assert!(w.actors.iter().all(|a| a.pos.finite()
        && a.pos.x > -22.0
        && a.pos.x < 68.0
        && a.pos.z > -54.0
        && a.pos.z < 38.0));
    assert!(reached, "bots must reach the objective");
}
#[test]
fn ray_parallel_and_inside_cases() {
    let min = V3::new(-1.0, -1.0, -1.0);
    let max = V3::new(1.0, 1.0, 1.0);
    assert_eq!(
        ray_box(V3::default(), V3::new(0.0, 0.0, 1.0), min, max),
        Some(0.0)
    );
    assert_eq!(
        ray_box(V3::new(2.0, 0.0, -2.0), V3::new(0.0, 0.0, 1.0), min, max),
        None
    );
}
