use trickcal_shooting_balddagu::{
    combat::AttackClass, game::*, ground_effect::GroundEffect, level::MapId, movement,
    race_skill::*, roster::SelectionResult,
};

fn world(id: u8) -> World {
    let mut w = World::room(MapId::default(), true);
    w.map.clear();
    w.phase = Phase::Playing;
    w.actors[0].character = id;
    w.actors[0].ammo = trickcal_shooting_balddagu::kits::kit(id).magazine;
    w.actors[0].pos = V3::default();
    w.actors[0].shield = 0.0;
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

fn field(z: f32, damage: f32, stun: f32) -> GroundEffect {
    GroundEffect {
        owner: 6,
        character: 4,
        min: V3::new(-2.0, -0.01, z),
        max: V3::new(2.0, 0.05, z + 0.001),
        damage,
        stun,
        remaining: 10.0,
        interval: 1.0,
        class: AttackClass::Skill,
        next_hit: [0; PLAYERS],
    }
}

#[test]
fn every_fairy_and_witch_moves_over_nine_ticks_in_the_original_facing_direction() {
    for id in [2, 4, 5, 6, 8] {
        let mut w = world(id);
        tick(
            &mut w,
            Input {
                yaw: std::f32::consts::FRAC_PI_2,
                pitch: 1.5,
                ..shift()
            },
            1,
        );
        assert!((w.actors[0].pos.x - SURGE_SPEED * DT).abs() < 0.001);
        assert_eq!(w.actors[0].race_skill.surge_charges, 2);
        tick(
            &mut w,
            Input {
                forward: -1.0,
                strafe: 1.0,
                ..Default::default()
            },
            8,
        );
        assert!((w.actors[0].pos.x - 3.75).abs() < 0.001);
        assert!(w.actors[0].pos.z.abs() < 0.001);
        assert_eq!(w.actors[0].pos.y, 0.0);
        tick(&mut w, Input::default(), 1);
        assert!(!w.actors[0].race_skill.active());
        assert!((w.actors[0].pos.x - 3.75).abs() < 0.001);
    }
}

#[test]
fn every_fairy_and_witch_can_surge_during_basic_attacks() {
    for id in [2, 4, 5, 6, 8] {
        // Cover simultaneous input, windup, and an attack already in progress.
        for attack_ticks in [0, 1, 12] {
            let mut w = world(id);
            if attack_ticks > 0 {
                tick(
                    &mut w,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                    1,
                );
                tick(&mut w, Input::default(), attack_ticks - 1);
                assert_eq!(w.actors[0].combat.attacks, 1);
                assert!(w.actors[0].cooldown > 0.0);
                assert!(
                    can_activate(&w.map, &w.actors[0]),
                    "hero {id}, attack tick {attack_ticks}"
                );
            }
            tick(
                &mut w,
                Input {
                    fire: attack_ticks == 0,
                    ..shift()
                },
                1,
            );
            assert!(
                w.actors[0].race_skill.active(),
                "hero {id}, attack tick {attack_ticks}"
            );
            assert_eq!(w.actors[0].race_skill.surge_charges, 2);
            tick(&mut w, shift(), 8);
            assert!(
                (w.actors[0].pos.z - 3.75).abs() < 0.001,
                "hero {id}, attack tick {attack_ticks}"
            );
            assert_eq!(w.actors[0].combat.attacks, 1);
        }
    }
}

#[test]
fn chloe_surge_preserves_normal_and_mounted_attack_hits() {
    use trickcal_shooting_balddagu::chloe_skill::{Phase as Action, State, RIDE_SECONDS};
    for mounted in [false, true] {
        let mut w = world(6);
        w.join_player(TEAM_SIZE);
        w.actors[TEAM_SIZE].character = 6;
        w.actors[TEAM_SIZE].pos = V3::new(0.0, 0.0, 5.75);
        w.actors[TEAM_SIZE].shield = 0.0;
        if mounted {
            w.actors[0].combat.chloe = State {
                phase: Action::Ride,
                remaining: RIDE_SECONDS,
                ..Default::default()
            };
        }
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
            1,
        );
        let attack = if mounted {
            Action::RideAttack
        } else {
            Action::Attack
        };
        assert_eq!(w.actors[0].combat.chloe.phase, attack);
        tick(&mut w, shift(), 9);
        assert!((w.actors[0].pos.z - 3.75).abs() < 0.001);
        assert_eq!(w.actors[0].combat.chloe.phase, attack);
        assert_eq!(w.actors[TEAM_SIZE].hp, MAX_HP);
        tick(&mut w, Input::default(), 5);
        let hp = w.actors[TEAM_SIZE].hp;
        assert!(
            hp < MAX_HP,
            "the original swing must still hit after the surge"
        );
        tick(&mut w, Input::default(), 40);
        assert_eq!(w.actors[TEAM_SIZE].hp, hp, "the swing must hit only once");
        assert_eq!(
            w.actors[0].combat.chloe.phase,
            if mounted { Action::Ride } else { Action::Off }
        );
        assert_eq!(w.actors[0].race_skill.surge_charges, 2);
    }
}

#[test]
fn chloe_non_basic_actions_still_block_surge() {
    use trickcal_shooting_balddagu::chloe_skill::{Phase as Action, State, RIDE_SECONDS};
    for phase in [Action::Mount, Action::Roar, Action::Dismount] {
        let mut w = world(6);
        w.actors[0].combat.chloe = State {
            phase,
            remaining: if phase == Action::Roar {
                RIDE_SECONDS
            } else {
                0.0
            },
            ..Default::default()
        };
        assert!(!can_activate(&w.map, &w.actors[0]));
        tick(&mut w, shift(), 1);
        assert_eq!(w.actors[0].race_skill.surge_charges, 3);
        assert_eq!(w.actors[0].pos, V3::default());
    }
}

#[test]
fn three_presses_spend_three_charges_and_recharge_sequentially_without_resetting_timer() {
    let mut w = world(4);
    for left in (0..3).rev() {
        tick(&mut w, shift(), 1);
        assert_eq!(w.actors[0].race_skill.surge_charges, left);
        tick(&mut w, Input::default(), 9);
    }
    let end = w.actors[0].pos;
    tick(&mut w, shift(), 1);
    assert_eq!(w.actors[0].pos, end);
    assert!(!w.actors[0].race_skill.active());
    tick(&mut w, shift(), 149);
    assert_eq!(w.actors[0].race_skill.surge_charges, 0);
    tick(&mut w, shift(), 1);
    assert_eq!(w.actors[0].race_skill.surge_charges, 1);
    assert_eq!(w.actors[0].pos, end, "held Shift must not auto-spend");
    tick(&mut w, shift(), 180);
    assert_eq!(w.actors[0].race_skill.surge_charges, 2);
    tick(&mut w, shift(), 360);
    assert_eq!(w.actors[0].race_skill.surge_charges, 3);
    assert_eq!(w.actors[0].race_skill.surge_recharge, 0.0);
    tick(&mut w, Input::default(), 1);
    tick(&mut w, shift(), 1);
    assert_eq!(w.actors[0].race_skill.surge_charges, 2);
}

#[test]
fn stun_sleep_and_channeling_prevent_spending_and_interrupt_without_resuming() {
    for lock in 0..3 {
        let mut w = world(2);
        match lock {
            0 => w.actors[0].combat.stun = 1.0,
            1 => w.actors[0].combat.sleep = 1.0,
            _ => w.actors[0].combat.dash = 1.0,
        }
        tick(&mut w, shift(), 1);
        assert_eq!(w.actors[0].race_skill.surge_charges, 3);
    }
    for sleep in [false, true] {
        let mut w = world(4);
        tick(&mut w, shift(), 2);
        let stopped = w.actors[0].pos;
        if sleep {
            w.actors[0].combat.sleep = 0.05;
        } else {
            w.actors[0].combat.stun = 0.05;
        }
        tick(&mut w, shift(), 20);
        assert_eq!(w.actors[0].pos, stopped);
        assert!(!w.actors[0].race_skill.active());
        assert_eq!(w.actors[0].race_skill.surge_charges, 2);
    }
}

#[test]
fn thin_stun_field_stops_at_contact_before_damage_further_along_the_same_tick() {
    let mut w = world(4);
    // Intentionally reversed insertion order; both fields fit within one tick.
    w.ground_effects = vec![field(0.38, 120.0, 0.0), field(0.31, 0.0, 1.0)];
    tick(&mut w, shift(), 1);
    assert!((w.actors[0].pos.z - 0.31).abs() < 0.0001);
    assert!(w.actors[0].combat.stun > 0.99);
    assert_eq!(w.actors[0].hp, MAX_HP);
    assert!(!w.actors[0].race_skill.active());
    assert_eq!(w.actors[0].race_skill.surge_charges, 2);
    tick(&mut w, Input::default(), 10);
    assert!((w.actors[0].pos.z - 0.31).abs() < 0.0001);
}

#[test]
fn damage_fields_hit_in_path_once_per_interval_and_lethal_damage_stops_the_surge() {
    let mut w = world(5);
    let mut area = field(0.2, 12.0, 0.0);
    area.max.z = 8.0;
    w.ground_effects.push(area);
    tick(&mut w, shift(), 10);
    assert_eq!(w.actors[0].hp, MAX_HP - 12);
    assert!((w.actors[0].pos.z - 3.75).abs() < 0.001);
    tick(&mut w, Input::default(), 51);
    assert_eq!(w.actors[0].hp, MAX_HP - 24);

    let mut w = world(8);
    w.ground_effects.push(field(1.13, 999.0, 0.0));
    tick(&mut w, shift(), 3);
    assert!(!w.actors[0].alive());
    assert!((w.actors[0].pos.z - 1.13).abs() < 0.0001);
    assert_eq!(w.actors[0].killed_by, Some(6));
}

#[test]
fn walls_block_fields_behind_them_and_airborne_surge_keeps_gravity() {
    let mut w = world(6);
    w.map.push(Block {
        center: V3::new(0.0, 2.0, 1.5),
        size: V3::new(8.0, 4.0, 0.02),
        color: [0; 3],
        phaseable: true,
    });
    w.ground_effects.push(field(1.51, 999.0, 1.0));
    tick(&mut w, shift(), 10);
    assert!(w.actors[0].pos.z < 1.1);
    assert!(!movement::blocked(&w.map, w.actors[0].pos));
    assert_eq!(w.actors[0].hp, MAX_HP);
    assert!(!w.actors[0].race_skill.active());

    let mut w = world(2);
    w.actors[0].pos.y = 2.0;
    w.ground_effects.push(field(0.31, 999.0, 1.0));
    tick(&mut w, shift(), 9);
    assert!(w.actors[0].pos.y < 2.0 && w.actors[0].pos.y > 1.0);
    assert!((w.actors[0].pos.z - 3.75).abs() < 0.001);
    assert_eq!(w.actors[0].hp, MAX_HP);
}

#[test]
fn normal_walking_uses_the_same_field_contacts_and_friendly_or_expired_fields_do_not_hit() {
    let mut w = world(4);
    w.ground_effects.push(field(0.04, 0.0, 1.0));
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
        1,
    );
    assert!((w.actors[0].pos.z - 0.04).abs() < 0.0001);
    assert!(w.actors[0].combat.stun > 0.0);
    let mut w = world(4);
    let mut friendly = field(0.1, 999.0, 1.0);
    friendly.owner = 1;
    let mut expired = field(0.3, 999.0, 1.0);
    expired.remaining = 0.0;
    w.ground_effects = vec![friendly, expired];
    tick(&mut w, shift(), 9);
    assert_eq!(w.actors[0].hp, MAX_HP);
    assert!((w.actors[0].pos.z - 3.75).abs() < 0.001);
}

#[test]
fn hero_swaps_preserve_charge_bank_and_respawn_and_new_round_refill_it() {
    let mut w = world(2);
    tick(&mut w, shift(), 1);
    let recharge = w.actors[0].race_skill.surge_recharge;
    w.actors[0].pos = w.spawn_center(0);
    for id in [4, 14, 5] {
        assert_eq!(w.select_character(0, id), SelectionResult::Accepted);
        assert_eq!(w.actors[0].race_skill.surge_charges, 2);
        assert_eq!(w.actors[0].race_skill.surge_recharge, recharge);
        assert!(!w.actors[0].race_skill.active());
    }
    w.actors[0].hp = 0;
    w.actors[0].respawn = DT;
    tick(&mut w, Input::default(), 1);
    assert_eq!(w.actors[0].race_skill.surge_charges, 3);
    w.actors[0].race_skill.surge_charges = 0;
    w.start();
    assert_eq!(w.actors[0].race_skill.surge_charges, 3);
}
