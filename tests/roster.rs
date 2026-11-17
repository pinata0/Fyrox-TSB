use trickcal_shooting_balddagu::{game::*, roster::*};

fn arena_test() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.human = true;
        a.shield = 0.0;
        a.pos = V3::new(40.0 + i as f32 * 2.0, 0.0, 40.0);
    }
    w.actors[0].pos = V3::default();
    w
}
fn tick(w: &mut World, actor: usize, input: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[actor] = input;
    w.tick(inputs);
}
fn ability(w: &mut World, actor: usize) {
    tick(
        w,
        actor,
        Input {
            ability: true,
            ..Default::default()
        },
    );
}
fn shoot(w: &mut World, actor: usize) {
    tick(
        w,
        actor,
        Input {
            fire: true,
            ..Default::default()
        },
    );
}

#[test]
fn approved_roster_and_default_six_person_teams() {
    let w = World::default();
    assert_eq!(PLAYERS, 12);
    assert_eq!(ROSTER.len(), 15);
    let mut races = [false; 7];
    let mut grid = [[0; 3]; 5];
    for c in ROSTER {
        races[c.race as usize] = true;
        grid[c.personality as usize][c.position as usize] += 1;
    }
    assert!(races.into_iter().all(|v| v));
    assert_eq!(grid, [[1; 3]; 5]);
    for side in 0..2 {
        assert_eq!(w.position_counts(side), [2, 2, 2]);
        assert_eq!(w.damage_bonus(side), 60);
        for i in side * TEAM_SIZE..(side + 1) * TEAM_SIZE {
            assert_eq!(team(i), side);
            for j in i + 1..(side + 1) * TEAM_SIZE {
                assert_ne!(w.actors[i].character, w.actors[j].character);
                assert!((w.actors[i].pos - w.actors[j].pos).length() > 1.0);
            }
        }
    }
}
#[test]
fn selection_is_atomic_and_enforces_position_duplicate_and_phase_rules() {
    let mut w = World::default();
    w.actors[0].human = true;
    let old = w.actors[0].character;
    assert_eq!(w.select_character(0, 10), SelectionResult::Accepted);
    assert_eq!(w.actors[3].character, old); // bot exchange preserves the team
    assert_eq!(w.position_counts(0), [2, 2, 2]);
    w.actors[1].human = true;
    assert_eq!(w.select_character(0, 12), SelectionResult::Taken);
    assert_eq!(w.actors[0].character, 10);
    assert_eq!(w.select_character(4, 3), SelectionResult::Accepted); // third front
    assert_eq!(w.select_character(5, 6), SelectionResult::PositionFull);
    assert_eq!(w.position_counts(0), [3, 2, 1]);
    assert_eq!(w.select_character(6, 10), SelectionResult::Accepted); // enemy mirror allowed
    assert_eq!(w.select_character(0, 255), SelectionResult::Invalid);
    assert_eq!(w.select_character(PLAYERS, 0), SelectionResult::Invalid);
    w.phase = Phase::Playing;
    w.actors[0].pos = V3::default();
    assert_eq!(w.select_character(0, 4), SelectionResult::Locked);
    w.actors[0].hp = 0;
    w.actors[0].respawn = 5.0;
    w.actors[0].ability_cooldown = 6.0;
    assert_eq!(w.select_character(0, 4), SelectionResult::Accepted);
    assert_eq!(w.actors[0].hp, 0);
    assert_eq!(w.actors[0].respawn, 5.0);
    assert_eq!(w.actors[0].ability_cooldown, 6.0);
}
#[test]
fn character_choices_survive_restart_round_switch_and_respawn() {
    let mut w = World::default();
    w.select_character(0, 4);
    w.actors[0].human = true;
    w.start();
    assert_eq!(w.actors[0].character, 4);
    w.phase = Phase::Playing;
    w.actors[0].hp = 0;
    w.actors[0].respawn = DT;
    w.tick([Input::default(); PLAYERS]);
    assert_eq!(w.actors[0].character, 4);
    assert_eq!(w.actors[0].hp, MAX_HP);
    w.finish_round(false);
    for _ in 0..362 {
        w.tick([Input::default(); PLAYERS]);
    }
    assert_eq!(w.attacker, 1);
    assert_eq!(w.actors[0].character, 4);
}
#[test]
fn teamwide_additive_synergy_uses_roster_including_dead_players() {
    let mut w = arena_test();
    for (ids, expected) in [
        ([0, 1, 3, 4, 6, 7], 60),
        ([0, 1, 2, 3, 4, 5], 40),
        ([0, 1, 3, 6, 9, 14], 20),
    ] {
        for (a, id) in w.actors[..TEAM_SIZE].iter_mut().zip(ids) {
            a.character = id;
        }
        assert_eq!(w.damage_bonus(0), expected);
        w.actors[1].hp = 0;
        assert_eq!(w.damage_bonus(0), expected);
    }
    // A lone Gloomy shooter also receives the Pure pair's team bonus.
    w.actors[5].pos = V3::default();
    w.actors[0].pos = V3::new(30.0, 0.0, 0.0);
    w.actors[6].pos = V3::new(0.0, 0.0, 4.0);
    shoot(&mut w, 5);
    assert_eq!(w.actors[6].hp, MAX_HP - 34);
}
#[test]
fn all_maximum_synergy_compositions_respect_position_cap() {
    let mut maximum = 0;
    let mut balanced = 0;
    for mask in 0u32..(1 << ROSTER.len()) {
        if mask.count_ones() != 6 {
            continue;
        }
        let mut p = [0; 5];
        let mut pos = [0; 3];
        for (i, c) in ROSTER.iter().enumerate() {
            if mask & (1 << i) != 0 {
                p[c.personality as usize] += 1;
                pos[c.position as usize] += 1;
            }
        }
        if p.into_iter().map(|n| n / 2).sum::<usize>() == 3 {
            maximum += 1;
            assert!(pos.into_iter().all(|n| n <= 3));
            if pos == [2, 2, 2] {
                balanced += 1;
            }
        }
    }
    assert_eq!((maximum, balanced), (270, 60));
}
#[test]
fn fairy_receives_extra_healing_and_heal_obeys_team_death_cover_and_cap() {
    let mut w = arena_test();
    w.actors[0].character = 13; // Hilde pulse
    w.actors[1].character = 2; // Erpin
    w.actors[1].pos = V3::new(0.0, 0.0, 3.0);
    w.actors[1].hp = 50;
    w.actors[2].hp = 0;
    w.actors[2].respawn = 5.0;
    w.actors[2].pos = V3::new(1.0, 0.0, 3.0);
    w.actors[6].hp = 50;
    w.actors[6].pos = V3::new(-1.0, 0.0, 3.0);
    ability(&mut w, 0);
    assert_eq!(w.actors[1].hp, 72);
    assert_eq!(w.actors[2].hp, 0);
    assert_eq!(w.actors[6].hp, 50);
    w.actors[0].ability_cooldown = 0.0;
    w.actors[1].combat.regen = 0.0;
    w.map.push(Block {
        center: V3::new(0.0, 1.0, 1.5),
        size: V3::new(5.0, 3.0, 0.5),
        color: [0; 3],
        phaseable: true,
    });
    ability(&mut w, 0);
    assert_eq!(w.actors[1].hp, 72);
    w.map.clear();
    w.actors[0].ability_cooldown = 0.0;
    w.actors[1].hp = 110;
    ability(&mut w, 0);
    assert_eq!(w.actors[1].hp, MAX_HP);
}
#[test]
fn shift_has_no_beast_speed_bonus_and_elf_reload_is_faster() {
    let mut w = arena_test();
    w.actors[0].character = 11;
    tick(
        &mut w,
        0,
        Input {
            forward: 1.0,
            phase_shift: true,
            ..Default::default()
        },
    );
    assert!((w.actors[0].pos.z - 5.8 * DT).abs() < 0.0001);
    assert!(!w.actors[0].phase_skill.active());
    w.actors[0].pos = V3::default();
    tick(
        &mut w,
        0,
        Input {
            forward: 1.0,
            ..Default::default()
        },
    );
    assert!((w.actors[0].pos.z - 5.8 * DT).abs() < 0.0001);
    w.actors[0].character = 3;
    w.actors[0].ammo = 1;
    tick(
        &mut w,
        0,
        Input {
            reload: true,
            ..Default::default()
        },
    );
    assert!((w.actors[0].reload - 1.44).abs() < 0.001);
    for _ in 0..87 {
        tick(&mut w, 0, Input::default());
    }
    assert_eq!(w.actors[0].ammo, 40);
}
#[test]
fn spirit_barrier_waits_four_seconds_absorbs_damage_and_never_stacks() {
    let mut w = arena_test();
    w.actors[6].character = 1;
    w.actors[6].pos = V3::new(0.0, 0.0, 3.0);
    for _ in 0..239 {
        tick(&mut w, 0, Input::default());
    }
    assert_eq!(w.actors[6].barrier, 0.0);
    for _ in 0..5 {
        tick(&mut w, 0, Input::default());
    }
    assert!((w.actors[6].barrier - 9.6).abs() < 0.001);
    shoot(&mut w, 0);
    // First blade: 21 / 2 - 9.6 barrier = 0.9 -> 1 HP.
    // Second blade: the barrier is gone, so all 21 damage reaches HP.
    assert_eq!(w.actors[6].hp, 98);
    assert_eq!(w.actors[6].barrier, 0.0);
    for _ in 0..300 {
        tick(&mut w, 0, Input::default());
    }
    assert!((w.actors[6].barrier - 9.6).abs() < 0.001);
}
#[test]
fn ghost_damage_no_longer_grants_speed() {
    let mut w = arena_test();
    w.actors[6].character = 9;
    w.actors[6].pos = V3::new(0.0, 0.0, 3.0);
    shoot(&mut w, 0);
    assert_eq!(w.actors[6].ghost_phase, 0.0);
    let before = w.actors[6].pos;
    tick(
        &mut w,
        6,
        Input {
            strafe: 1.0,
            ..Default::default()
        },
    );
    assert!(((w.actors[6].pos - before).length() - 5.8 * DT).abs() < 0.0001);
}
fn ultimate(w: &mut World) {
    w.actors[0].combat.ultimate = 100.0;
    w.actors[0].combat.q_down = false;
    tick(
        w,
        0,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
}
#[test]
fn dragon_resists_shockwave_and_witch_cooldown_applies_without_repeat_cast() {
    for (id, distance) in [(0, 1.4), (3, 2.0)] {
        let mut w = arena_test();
        w.actors[0].character = 12;
        w.actors[6].character = id;
        w.actors[6].pos = V3::new(0.0, 0.0, 3.0);
        ultimate(&mut w);
        assert!((w.actors[6].pos.z - 3.0 - distance).abs() < 0.001);
    }
    let mut w = arena_test();
    w.actors[0].character = 4;
    ability(&mut w, 0);
    assert!((w.actors[0].ability_cooldown - 8.55).abs() < 0.001);
    assert_eq!(w.projectiles.len(), 1);
    ability(&mut w, 0);
    assert_eq!(w.projectiles.len(), 1);
}
#[test]
fn shockwave_respects_spawn_protection_cover_and_swept_collision() {
    let mut w = arena_test();
    w.actors[0].character = 12;
    w.actors[6].pos = V3::new(0.0, 0.0, 3.0);
    w.actors[6].shield = 1.0;
    ultimate(&mut w);
    assert_eq!(w.actors[6].hp, MAX_HP);
    assert_eq!(w.actors[6].pos.z, 3.0);
    w.actors[6].shield = 0.0;
    w.map.push(Block {
        center: V3::new(0.0, 1.0, 4.5),
        size: V3::new(4.0, 3.0, 0.2),
        color: [0; 3],
        phaseable: true,
    });
    ultimate(&mut w);
    assert!(w.actors[6].pos.z > 3.0 && w.actors[6].pos.z < 4.1);
    assert!(w.actors[6].hp < MAX_HP);
    w.map[0].center.z = 1.5;
    let hp = w.actors[6].hp;
    ultimate(&mut w);
    assert_eq!(w.actors[6].hp, hp);
}
