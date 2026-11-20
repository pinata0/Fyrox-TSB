use trickcal_shooting_balddagu::{
    game::*, level::MapId, movement, race_skill::*, roster::SelectionResult,
};

fn world() -> World {
    let mut w = World::room(MapId::default(), true);
    w.map.clear();
    w.phase = Phase::Playing;
    w.actors[0] = Actor {
        character: 0,
        human: true,
        ..Default::default()
    };
    w
}

fn tick(w: &mut World, input: Input, count: usize) {
    for _ in 0..count {
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0] = input;
        w.tick(inputs);
    }
}

fn shift() -> Input {
    Input {
        phase_shift: true,
        ..Default::default()
    }
}

fn enemy(w: &mut World, slot: usize, pos: V3) {
    w.actors[slot] = Actor {
        character: 3,
        human: true,
        pos,
        ..Default::default()
    };
}

#[test]
fn charge_forces_forward_even_with_back_input_and_steers_with_view_and_strafe() {
    let mut w = world();
    tick(
        &mut w,
        Input {
            forward: -1.0,
            ..shift()
        },
        1,
    );
    assert!((w.actors[0].pos.z - CHARGE_SPEED * DT).abs() < 0.0001);
    let before = w.actors[0].pos;
    tick(
        &mut w,
        Input {
            yaw: std::f32::consts::FRAC_PI_2,
            pitch: 1.4,
            ..Default::default()
        },
        1,
    );
    let delta = w.actors[0].pos - before;
    assert!((delta.x - CHARGE_SPEED * DT).abs() < 0.0001);
    assert!(delta.z.abs() < 0.0001);
    let before = w.actors[0].pos;
    tick(
        &mut w,
        Input {
            strafe: 1.0,
            ..Default::default()
        },
        1,
    );
    let delta = w.actors[0].pos - before;
    assert!(delta.x < 0.0 && delta.z > 0.0);
    assert!((delta.length() - CHARGE_SPEED * DT).abs() < 0.0001);
}

#[test]
fn charge_expires_starts_cooldown_and_requires_fresh_shift() {
    let mut w = world();
    tick(&mut w, shift(), 180);
    assert!(charging(&w.actors[0]));
    assert!((w.actors[0].pos.z - CHARGE_SPEED * CHARGE_DURATION).abs() < 0.001);
    tick(&mut w, shift(), 1);
    assert!(!charging(&w.actors[0]));
    assert_eq!(w.actors[0].race_skill.cooldown, CHARGE_COOLDOWN);
    let end = w.actors[0].pos;
    tick(&mut w, shift(), 500);
    assert_eq!(w.actors[0].pos, end);
    assert_eq!(w.actors[0].race_skill.cooldown, 0.0);
    tick(&mut w, Input::default(), 1);
    tick(&mut w, shift(), 1);
    assert!(charging(&w.actors[0]));
}

#[test]
fn charge_hits_each_enemy_once_and_can_hit_again_on_next_cast() {
    let mut w = world();
    enemy(&mut w, 6, V3::new(0.0, 0.0, 1.0));
    enemy(&mut w, 7, V3::new(0.0, 0.0, 3.0));
    tick(&mut w, shift(), 1);
    tick(&mut w, Input::default(), 25);
    assert_eq!(w.actors[6].hp, MAX_HP - CHARGE_DAMAGE as u16);
    assert_eq!(w.actors[7].hp, MAX_HP - CHARGE_DAMAGE as u16);
    assert!(w.actors[0].combat.ultimate > 0.0);
    w.actors[0].pos = w.actors[6].pos;
    tick(&mut w, Input::default(), 3);
    assert_eq!(w.actors[6].hp, MAX_HP - CHARGE_DAMAGE as u16);
    tick(&mut w, Input::default(), 660);
    w.actors[0].pos = w.actors[6].pos;
    tick(&mut w, shift(), 1);
    assert_eq!(w.actors[6].hp, MAX_HP - 2 * CHARGE_DAMAGE as u16);
}

#[test]
fn charge_respects_allies_dead_burrowed_spawn_protection_and_vertical_separation() {
    for case in 0..6 {
        let mut w = world();
        let target = if case == 0 { 1 } else { 6 };
        enemy(&mut w, target, V3::new(0.0, 0.0, 0.6));
        match case {
            1 => w.actors[target].hp = 0,
            2 => {
                w.actors[target].character = 1;
                w.actors[target].race_skill.remaining = 3.0;
            }
            3 => w.actors[target].shield = 2.0,
            4 => w.actors[target].pos.y = 3.0,
            5 => {
                let spawn = w.spawn_center(team(target));
                w.actors[target].pos = spawn;
                w.actors[0].pos = spawn - V3::new(0.0, 0.0, 0.6);
            }
            _ => {}
        }
        let hp = w.actors[target].hp;
        // Do not let a deliberately dead test actor respawn this tick.
        w.actors[target].respawn = 7.0;
        tick(&mut w, shift(), 1);
        assert_eq!(w.actors[target].hp, hp, "case {case}");
        assert_eq!(w.actors[0].race_skill.hit_mask, 0, "case {case}");
    }
}

#[test]
fn charge_obeys_walls_and_cannot_hit_through_thin_geometry() {
    let mut w = world();
    w.actors[0].pos.x = -0.41;
    enemy(&mut w, 6, V3::new(0.35, 0.0, 0.1));
    w.map.push(Block {
        center: V3::new(0.0, 2.0, 0.0),
        size: V3::new(0.02, 4.0, 20.0),
        color: [0; 3],
        phaseable: true,
    });
    tick(&mut w, shift(), 1);
    assert_eq!(w.actors[6].hp, MAX_HP);
    tick(
        &mut w,
        Input {
            yaw: std::f32::consts::FRAC_PI_2,
            ..Default::default()
        },
        30,
    );
    assert!(w.actors[0].pos.x < -0.4);
    assert!(!movement::blocked(&w.map, w.actors[0].pos));
    assert_eq!(w.actors[6].hp, MAX_HP);
}

#[test]
fn enemy_movement_into_charge_deals_damage_and_stationary_overlap_does_not_repeat() {
    let mut w = world();
    tick(&mut w, shift(), 1);
    let target_pos = w.actors[0].pos
        + V3::new(2.0 * movement::BODY_RADIUS + movement::WALK_SPEED * DT * 0.5, 0.0, 0.0);
    enemy(&mut w, 6, target_pos);
    let mut inputs = [Input::default(); PLAYERS];
    inputs[6].strafe = 1.0;
    w.tick(inputs);
    assert_eq!(w.actors[6].hp, MAX_HP - CHARGE_DAMAGE as u16);
    for _ in 0..3 {
        w.tick(inputs);
    }
    assert_eq!(w.actors[6].hp, MAX_HP - CHARGE_DAMAGE as u16);
}

#[test]
fn stun_or_sleep_interrupts_charge_and_stops_contact_damage() {
    for sleep in [false, true] {
        let mut w = world();
        tick(&mut w, shift(), 1);
        if sleep {
            w.actors[0].combat.sleep = 1.0;
        } else {
            w.actors[0].combat.stun = 1.0;
        }
        let pos = w.actors[0].pos;
        enemy(&mut w, 6, pos);
        tick(&mut w, Input::default(), 1);
        assert_eq!(w.actors[0].pos, pos);
        assert!(!charging(&w.actors[0]));
        assert_eq!(w.actors[0].race_skill.cooldown, CHARGE_COOLDOWN);
        assert_eq!(w.actors[6].hp, MAX_HP);
    }
}

#[test]
fn charge_kill_uses_normal_kill_credit_and_swap_preserves_cooldown() {
    let mut w = world();
    enemy(&mut w, 6, V3::new(0.0, 0.0, 0.6));
    w.actors[6].hp = CHARGE_DAMAGE as u16;
    tick(&mut w, shift(), 1);
    assert_eq!(w.actors[6].hp, 0);
    assert_eq!(w.actors[6].killed_by, Some(0));
    assert_eq!(w.actors[0].kills, 1);
    w.actors[0].pos = w.spawn_center(team(0));
    assert_eq!(w.select_character(0, 3), SelectionResult::Accepted);
    assert_eq!(w.actors[0].race_skill.cooldown, CHARGE_COOLDOWN);
    assert_eq!(w.select_character(0, 0), SelectionResult::Accepted);
    assert!(!charging(&w.actors[0]));
    assert_eq!(w.actors[0].race_skill.cooldown, CHARGE_COOLDOWN);
}

#[test]
fn charge_uses_normal_shield_mitigation_and_slow_without_losing_forced_forward() {
    let mut w = world();
    enemy(&mut w, 6, V3::new(0.0, 0.0, 0.6));
    w.actors[6].combat.guard = 10.0;
    w.actors[6].combat.guard_time = 2.0;
    w.actors[0].combat.slow = 2.0;
    tick(&mut w, shift(), 1);
    assert!((w.actors[0].pos.z - CHARGE_SPEED * 0.65 * DT).abs() < 0.0001);
    assert_eq!(w.actors[6].combat.guard, 0.0);
    assert_eq!(w.actors[6].hp, MAX_HP - 5);
    assert!(w.actors[0].hit > 0.0);
}

#[test]
fn portal_ends_charge_and_never_sweeps_body_damage_across_teleport() {
    use trickcal_shooting_balddagu::portal::Portal;
    let mut w = world();
    for x in [0.0, 12.0] {
        w.map.push(Block {
            center: V3::new(x, 2.5, 4.0),
            size: V3::new(8.0, 5.0, 0.3),
            color: [0; 3],
            phaseable: true,
        });
    }
    w.actors[1] = Actor {
        character: 3,
        human: true,
        pos: V3::new(25.0, 0.0, 0.0),
        ..Default::default()
    };
    w.actors[1].portals.ends = [0.0, 12.0].map(|x| {
        Some(Portal {
            center: V3::new(x, 1.5, 3.85),
            face: 5,
        })
    });
    enemy(&mut w, 6, V3::new(6.0, 0.0, 3.3));
    tick(&mut w, shift(), 1);
    for _ in 0..30 {
        tick(&mut w, Input::default(), 1);
        if w.actors[0].portals.transits > 0 {
            break;
        }
    }
    assert_eq!(w.actors[0].portals.transits, 1);
    assert!(!charging(&w.actors[0]));
    assert_eq!(w.actors[0].race_skill.cooldown, CHARGE_COOLDOWN);
    assert_eq!(w.actors[6].hp, MAX_HP);
}

#[test]
fn charge_cannot_be_replaced_by_dance_after_shift_is_released() {
    let mut w = world();
    tick(&mut w, shift(), 1);
    tick(
        &mut w,
        Input {
            dance: true,
            ..Default::default()
        },
        1,
    );
    assert!(!w.actors[0].dancing);
    assert!(w.actors[0].pos.z > CHARGE_SPEED * DT);
}
