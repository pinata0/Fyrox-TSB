use trickcal_shooting_balddagu::{combat::*, game::*, kits::*};
fn world(id: u8) -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.human = true;
        a.shield = 0.0;
        a.pos = V3::new(50.0 + i as f32 * 2.0, 0.0, 45.0);
    }
    w.actors[0].character = id;
    w.actors[0].ammo = kit(id).magazine;
    w.actors[0].pos = V3::default();
    w.actors[6].pos = V3::new(0.0, 0.0, 3.0);
    w
}
fn input(w: &mut World, i: usize, v: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[i] = v;
    w.tick(inputs);
}
fn advance(w: &mut World, n: usize) {
    for _ in 0..n {
        w.tick([Input::default(); PLAYERS]);
    }
}
fn fire(w: &mut World) {
    input(
        w,
        0,
        Input {
            fire: true,
            ..Default::default()
        },
    );
}
fn skill(w: &mut World) {
    input(
        w,
        0,
        Input {
            ability: true,
            ..Default::default()
        },
    );
}
fn ult(w: &mut World) {
    w.actors[0].combat.ultimate = 100.0;
    input(
        w,
        0,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
}
fn ally(w: &mut World, i: usize, hp: u16, z: f32) {
    w.actors[i].pos = V3::new(1.0, 0.0, z);
    w.actors[i].hp = hp;
}
fn wall(z: f32) -> Block {
    Block {
        center: V3::new(0.0, 1.5, z),
        size: V3::new(15.0, 4.0, 0.25),
        color: [0; 3],
        phaseable: true,
    }
}

#[test]
fn every_character_has_a_functional_primary_and_melee_has_no_ammo() {
    for id in 0..15 {
        let mut w = world(id);
        let start_ammo = w.actors[0].ammo;
        fire(&mut w);
        advance(&mut w, 25);
        assert!(w.actors[6].hp < MAX_HP, "primary must hit: {id}");
        if kit(id).mode == AttackMode::Melee {
            assert_eq!(w.actors[0].ammo, 0);
        } else {
            assert_eq!(w.actors[0].ammo, start_ammo - 1);
        }
    }
}

#[test]
fn vivi_repeats_two_normal_attacks_then_a_stronger_finisher() {
    let mut w = world(0);
    let mut damage = Vec::new();
    for stage in [1, 2, 3, 1, 2, 3] {
        w.actors[6].hp = MAX_HP;
        w.actors[0].cooldown = 0.0;
        fire(&mut w);
        damage.push(MAX_HP - w.actors[6].hp);
        assert_eq!(w.actors[0].combat.vivi_combo, stage);
        assert_eq!(w.actors[0].ammo, 0);
        let shot = w.actors[0].shot;
        fire(&mut w);
        assert_eq!(w.actors[0].shot, shot, "cooldown must not advance combo");
        assert_eq!(w.actors[0].combat.vivi_combo, stage);
    }
    assert_eq!(damage[0], damage[1]);
    assert!(damage[2] > damage[1]);
    assert_eq!(&damage[..3], &damage[3..]);
    w.clear_combat_on_spawn(0);
    assert_eq!(w.actors[0].combat.vivi_combo, 0);
    w.actors[0].cooldown = 0.0;
    fire(&mut w);
    assert_eq!(w.actors[0].combat.vivi_combo, 1);
}

#[test]
fn vivi_misses_advance_combo_but_disabled_inputs_do_not() {
    let mut w = world(0);
    w.actors[6].pos.z = 30.0;
    for stage in 1..=3 {
        w.actors[0].cooldown = 0.0;
        fire(&mut w);
        assert_eq!(w.actors[0].combat.vivi_combo, stage);
    }
    assert_eq!(w.actors[6].hp, MAX_HP);
    w.actors[0].cooldown = 0.0;
    w.actors[0].combat.stun = 1.0;
    fire(&mut w);
    assert_eq!(w.actors[0].combat.vivi_combo, 3);
    assert_eq!(w.actors[0].combat.status_text(), "STUN / MORPH");
}

#[test]
fn aya_visual_stage_tracks_existing_damage_even_when_attacks_miss_or_are_blocked() {
    let mut w = world(4);
    let mut damage = Vec::new();
    for stage in [1, 2, 3, 1] {
        w.actors[0].cooldown = 0.0;
        w.actors[6].hp = MAX_HP;
        fire(&mut w);
        assert_eq!(w.actors[0].combat.aya_combo, stage);
        damage.push(MAX_HP - w.actors[6].hp);
        fire(&mut w);
        assert_eq!(w.actors[0].combat.aya_combo, stage);
    }
    assert_eq!(damage[0], damage[1]);
    assert_eq!(damage[0], damage[3]);
    assert!(damage[2] > damage[1]);
    w.actors[6].pos.z = 30.0;
    w.actors[0].cooldown = 0.0;
    fire(&mut w);
    assert_eq!(w.actors[0].combat.aya_combo, 2);
    w.actors[0].cooldown = 0.0;
    w.actors[0].combat.stun = 1.0;
    fire(&mut w);
    assert_eq!(w.actors[0].combat.aya_combo, 2);
    w.clear_combat_on_spawn(0);
    assert_eq!(w.actors[0].combat.aya_combo, 0);
}
#[test]
fn every_character_can_move_and_attack_while_walking_or_phase_shifting() {
    for id in 0..15 {
        for phase_shift in [false, true] {
            // Elves reserve Shift+fire for portal placement; this checks basic attacks.
            if phase_shift && trickcal_shooting_balddagu::portal::is_elf(&world(id).actors[0]) {
                continue;
            }
            for (forward, strafe) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (1.0, 1.0)]
            {
                let mut w = world(id);
                w.actors[6].pos.z = 2.0;
                let start_ammo = w.actors[0].ammo;
                input(
                    &mut w,
                    0,
                    Input {
                        forward,
                        strafe,
                        phase_shift,
                        fire: true,
                        ..Default::default()
                    },
                );
                assert!(w.actors[0].pos.flat().length() > 0.0, "must move: {id}");
                assert_eq!(
                    w.actors[0].shot, 1,
                    "must attack: {id}, phase_shift={phase_shift}"
                );
                assert_eq!(w.actors[0].ammo, start_ammo.saturating_sub(1));
                advance(&mut w, 25);
                assert!(w.actors[6].hp < MAX_HP, "moving attack must hit: {id}, phase_shift={phase_shift}, forward={forward}, strafe={strafe}");
            }
        }
    }
}

#[test]
fn sustained_phase_shift_attacks_preserve_movement_speed_and_attack_rate() {
    for id in 0..15 {
        let mut moving_fire = world(id);
        moving_fire.actors[6].pos = V3::new(40.0, 0.0, 40.0);
        let mut moving_only = moving_fire.clone();
        let mut firing_only = moving_fire.clone();
        for _ in 0..90 {
            let movement = Input {
                forward: 1.0,
                strafe: 1.0,
                phase_shift: matches!(id, 9 | 14),
                ..Default::default()
            };
            input(
                &mut moving_fire,
                0,
                Input {
                    fire: true,
                    ..movement
                },
            );
            input(&mut moving_only, 0, movement);
            fire(&mut firing_only);
        }
        assert_eq!(
            moving_fire.actors[0].pos, moving_only.actors[0].pos,
            "attack must preserve movement: {id}"
        );
        assert!(
            moving_fire.actors[0].shot > 1,
            "held attack must repeat: {id}"
        );
        assert_eq!(
            moving_fire.actors[0].shot, firing_only.actors[0].shot,
            "movement must preserve attack rate: {id}"
        );
        assert_eq!(moving_fire.actors[0].ammo, firing_only.actors[0].ammo);
    }
}

#[test]
fn melee_rejects_far_behind_and_occluded_targets() {
    for id in [0, 4, 6, 9, 12] {
        for pos in [V3::new(0.0, 0.0, 10.0), V3::new(0.0, 0.0, -2.0)] {
            let mut w = world(id);
            w.actors[6].pos = pos;
            fire(&mut w);
            assert_eq!(w.actors[6].hp, MAX_HP);
        }
        let mut w = world(id);
        w.map.push(wall(1.5));
        fire(&mut w);
        assert_eq!(w.actors[6].hp, MAX_HP);
    }
}
#[test]
fn projectile_travel_cover_and_spawn_protection_are_authoritative() {
    for id in [1, 2, 5, 7, 8, 10, 11, 13] {
        let mut w = world(id);
        w.actors[6].pos.z = 8.0;
        fire(&mut w);
        assert_eq!(w.actors[6].hp, MAX_HP);
        if id == 2 {
            assert!(w.actors[0].combat.erpin.shot_delay > 0.0);
            advance(&mut w, 10); // Attack1_1 releases the bolt at its source event.
        }
        assert!(!w.projectiles.is_empty());
        advance(&mut w, 60);
        assert!(w.actors[6].hp < MAX_HP, "projectile {id}");
        let mut w = world(id);
        w.map.push(wall(1.5));
        fire(&mut w);
        advance(&mut w, 60);
        assert_eq!(w.actors[6].hp, MAX_HP, "cover {id}");
    }
    let mut w = world(8);
    w.actors[6].shield = 2.0;
    skill(&mut w);
    advance(&mut w, 45);
    assert_eq!(w.actors[6].hp, MAX_HP);
    assert_eq!(w.actors[6].combat.poison, 0.0);
}
#[test]
fn spawn_protection_rejects_enhanced_melee_and_time_stop_debuffs() {
    for (id, attacks) in [(4, 2), (6, 2), (12, 3)] {
        let mut w = world(id);
        w.actors[6].shield = 2.0;
        w.actors[0].combat.attacks = attacks;
        fire(&mut w);
        let target = &w.actors[6];
        assert_eq!(target.hp, MAX_HP);
        assert_eq!(
            target.combat.slow + target.combat.stun + target.combat.taunt,
            0.0
        );
    }
    let mut w = world(7);
    w.actors[6].shield = 2.0;
    ult(&mut w);
    assert_eq!(w.actors[6].combat.slow, 0.0);
    advance(&mut w, 150);
    assert_eq!(w.actors[6].hp, MAX_HP);
}
#[test]
fn poison_refresh_does_not_delay_ticks_and_frost_keeps_its_ultimate_source() {
    let mut w = world(8);
    w.actors[6].combat.guard = 120.0;
    w.actors[6].combat.guard_time = 10.0;
    skill(&mut w);
    advance(&mut w, 30);
    assert!(w.actors[6].combat.poison > 0.0);
    let next_tick = w.actors[6].combat.dots[0].tick;
    assert!(
        next_tick < 0.75,
        "subsequent poison darts must preserve tick timing"
    );
    // A different ally applies a frost ultimate while poison remains active.
    w.actors[1].character = 4;
    w.actors[1].pos = V3::new(0.5, 0.0, 0.0);
    w.actors[1].yaw = (-0.5_f32).atan2(3.0);
    w.actors[1].combat.ultimate = 100.0;
    input(
        &mut w,
        1,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    assert!(w.actors[6].combat.frost > 0.0);
    assert_eq!(w.actors[6].combat.dots[0].owner, 0);
    assert_eq!(w.actors[6].combat.dots[1].owner, 1);
    let poison_charge = w.actors[0].combat.ultimate;
    advance(&mut w, 70);
    assert!(w.actors[0].combat.ultimate > poison_charge + 70.0 * DT * ULT_PER_SECOND);
    assert!((w.actors[1].combat.ultimate - 70.0 * DT * ULT_PER_SECOND).abs() < 0.001);
}
#[test]
fn charge_is_actual_enemy_damage_plus_time_not_overkill_or_invulnerability() {
    let mut w = world(14);
    w.actors[6].hp = 3;
    fire(&mut w);
    assert!(
        (w.actors[0].combat.ultimate - (ULT_PER_SECOND * DT + 3.0 * ULT_PER_DAMAGE)).abs() < 0.001
    );
    let old = w.actors[0].combat.ultimate;
    w.actors[0].cooldown = 0.0;
    fire(&mut w);
    assert!((w.actors[0].combat.ultimate - old - ULT_PER_SECOND * DT).abs() < 0.001);
    let mut w = world(14);
    w.actors[6].shield = 1.0;
    fire(&mut w);
    assert!((w.actors[0].combat.ultimate - ULT_PER_SECOND * DT).abs() < 0.001);
    let mut w = world(14);
    w.actors[6].pos.z = 30.0;
    ally(&mut w, 1, 50, 3.0);
    w.actors[1].pos.x = 0.0;
    fire(&mut w);
    assert_eq!(w.actors[1].hp, 50);
    assert!((w.actors[0].combat.ultimate - ULT_PER_SECOND * DT).abs() < 0.001);
    let mut w = world(14);
    w.phase = Phase::Lobby;
    advance(&mut w, 120);
    assert_eq!(w.actors[0].combat.ultimate, 0.0);
    w.phase = Phase::Playing;
    w.actors[0].combat.ultimate = 99.9;
    advance(&mut w, 60);
    assert_eq!(w.actors[0].combat.ultimate, 100.0);
}
#[test]
fn basic_projectiles_expire_at_their_listed_weapon_range() {
    for id in [1, 2, 5, 7, 8, 10, 11, 13] {
        let mut w = world(id);
        w.actors[6].pos.z = kit(id).range + 5.0;
        fire(&mut w);
        advance(&mut w, 180);
        assert_eq!(w.actors[6].hp, MAX_HP, "range cap for {id}");
        assert!(w.projectiles.is_empty());
    }
}
#[test]
fn q_requires_full_charge_and_a_new_press_and_never_charges_itself() {
    let mut w = world(12);
    w.actors[0].combat.ultimate = 99.0;
    input(
        &mut w,
        0,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].combat.giant, 0.0);
    w.actors[0].combat.ultimate = 100.0;
    input(
        &mut w,
        0,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].combat.giant, 0.0);
    advance(&mut w, 1);
    input(
        &mut w,
        0,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    assert!(w.actors[0].combat.giant > 0.0);
    assert_eq!(w.actors[0].combat.ultimate, 0.0);
    assert!(w.actors[6].hp < MAX_HP);
    advance(&mut w, 30);
    assert!((w.actors[0].combat.ultimate - 30.0 * DT * ULT_PER_SECOND).abs() < 0.001);
}
#[test]
fn death_preserves_charge_change_caps_it_and_round_switch_resets_it() {
    let mut w = world(0);
    w.actors[0].combat.ultimate = 58.0;
    w.actors[0].hp = 0;
    w.actors[0].respawn = 2.0;
    advance(&mut w, 60);
    assert_eq!(w.actors[0].combat.ultimate, 58.0);
    advance(&mut w, 62);
    assert!(w.actors[0].alive());
    assert!((w.actors[0].combat.ultimate - 58.0).abs() < 0.1);
    w.actors[0].hp = 0;
    w.actors[0].respawn = 3.0;
    w.select_character(0, 4);
    assert_eq!(w.actors[0].combat.ultimate, 15.0);
    w.actors[0].combat.ultimate = 80.0;
    w.finish_round(false);
    advance(&mut w, 362);
    assert!(w.actors[0].combat.ultimate < 1.0);
}
#[test]
fn all_fifteen_ultimates_consume_charge_and_emit_an_effect() {
    for id in 0..15 {
        let mut w = world(id);
        ally(&mut w, 1, 30, 2.0);
        w.actors[1].ability_cooldown = 8.0;
        ult(&mut w);
        assert_eq!(w.actors[0].combat.ultimate, 0.0, "ultimate {id}");
        assert!(w.actors[0].combat.ultimate_flash > 0.0);
        assert!(w.event.contains(kit(id).ultimate));
        advance(&mut w, 180);
        let changed = w.actors[6].hp < MAX_HP
            || w.actors[1].hp > 30
            || w.actors
                .iter()
                .any(|a| a.combat.guard > 0.0 || a.combat.giant > 0.0 || a.combat.stun > 0.0);
        assert!(changed, "ultimate has gameplay effect: {id}");
    }
}
#[test]
fn vivi_guard_explodes_and_spear_weakens_and_taunts() {
    let mut w = world(0);
    skill(&mut w);
    assert_eq!(w.actors[0].combat.guard, 45.0);
    advance(&mut w, 365);
    assert_eq!(w.actors[0].combat.guard, 0.0);
    assert!(w.actors[6].combat.vulnerable > 0.0);
    assert!(w.actors[6].hp < MAX_HP);
    let mut w = world(0);
    ult(&mut w);
    advance(&mut w, 20);
    assert!(w.actors[6].combat.taunt > 0.0);
    assert!(w.actors[6].combat.weaken > 0.0);
}
#[test]
fn naia_wash_cleanses_stun_and_wave_heals_allies_and_hits_enemies() {
    let mut w = world(1);
    ally(&mut w, 1, 30, 2.0);
    w.actors[1].pos.x = 0.0;
    w.actors[1].combat.stun = 4.0;
    skill(&mut w);
    advance(&mut w, 90);
    fire(&mut w);
    assert!(w.actors[1].hp > 30);
    assert_eq!(w.actors[1].combat.stun, 0.0);
    w.map.push(wall(1.0));
    let hp = w.actors[1].hp;
    for _ in 0..20 {
        fire(&mut w);
    }
    assert_eq!(w.actors[1].hp, hp);
    let mut w = world(1);
    ally(&mut w, 1, 30, 2.0);
    w.actors[6].pos.z = 6.0;
    ult(&mut w);
    advance(&mut w, 40);
    assert!(w.actors[1].hp >= 90);
    assert!(w.actors[6].hp < MAX_HP);
}
#[test]
fn erpin_fires_four_bombs_and_charge_cannot_cross_walls() {
    let mut w = world(2);
    skill(&mut w);
    assert_eq!(w.projectiles.len(), 4);
    advance(&mut w, 20);
    assert!(w.actors[6].hp < MAX_HP);
    let mut w = world(2);
    w.map.push(wall(1.5));
    ult(&mut w);
    advance(&mut w, 60);
    assert!(w.actors[0].pos.z < 1.0);
    assert_eq!(w.actors[6].hp, MAX_HP);
}
#[test]
fn id_wards_negate_two_lethal_hits_and_universe_boosts_allied_skills() {
    let mut w = world(3);
    ally(&mut w, 1, 1, 2.0);
    w.actors[1].pos.x = 0.0;
    w.actors[6].pos.z = 5.0;
    w.actors[6].character = 14;
    w.actors[6].ammo = 6;
    skill(&mut w);
    assert_eq!(w.actors[1].combat.ward_saves, 2);
    for left in [1, 0] {
        w.actors[6].cooldown = 0.0;
        w.actors[1].combat.guard = 0.0;
        input(
            &mut w,
            6,
            Input {
                fire: true,
                yaw: std::f32::consts::PI,
                ..Default::default()
            },
        );
        assert_eq!(w.actors[1].combat.ward_saves, left);
        assert_eq!(w.actors[1].hp, 1);
    }
    w.actors[6].cooldown = 0.0;
    w.actors[1].combat.guard = 0.0;
    input(
        &mut w,
        6,
        Input {
            fire: true,
            yaw: std::f32::consts::PI,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[1].hp, 0);
    let mut w = world(3);
    ally(&mut w, 1, 100, 2.0);
    ult(&mut w);
    assert!(w.actors[1].combat.skill_up > 0.0);
    advance(&mut w, 90);
    assert!(w.actors[6].ability_cooldown > 0.0);
}
#[test]
fn aya_butterfly_returns_and_snow_blooms_chain_frost() {
    let mut w = world(4);
    w.actors[6].pos.z = 7.0;
    skill(&mut w);
    advance(&mut w, 35);
    let hp = w.actors[6].hp;
    assert!(hp < MAX_HP);
    assert!(w.actors[6].combat.skill_weaken > 0.0);
    advance(&mut w, 95);
    assert!(w.actors[6].hp < hp);
    let mut w = world(4);
    w.actors[7].pos = V3::new(5.0, 0.0, 3.0);
    ult(&mut w);
    assert!(w.actors[6].combat.frost > 0.0);
    assert!(w.actors[7].combat.frost > 0.0);
}
#[test]
fn picora_stickers_heal_three_and_fashion_adds_guard_then_heals() {
    let mut w = world(5);
    for i in 1..5 {
        ally(&mut w, i, 20 + i as u16, i as f32 + 1.0);
    }
    skill(&mut w);
    assert!(w.actors[1].hp > 21 && w.actors[2].hp > 22 && w.actors[3].hp > 23);
    assert_eq!(w.actors[4].hp, 24);
    assert!(w.actors[1].combat.physical_resist > 0.0);
    let mut w = world(5);
    ally(&mut w, 1, 30, 2.0);
    ult(&mut w);
    assert_eq!(w.actors[1].combat.guard, 70.0);
    advance(&mut w, 485);
    assert!(w.actors[1].hp >= 65);
}
#[test]
fn chloe_empowers_melee_and_summons_seven_explosive_dolls() {
    let mut w = world(6);
    skill(&mut w);
    assert!(w.actors[0].combat.empowered > 0.0);
    assert_eq!(w.actors[0].combat.guard, 50.0);
    fire(&mut w);
    assert_eq!(w.actors[0].combat.empower_stacks, 1);
    assert!(w.actors[6].hp < MAX_HP);
    let mut w = world(6);
    ult(&mut w);
    assert_eq!(w.projectiles.len(), 7);
    assert!(w.projectiles.iter().all(|p| p.kind == ProjectileKind::Doll));
    advance(&mut w, 90);
    assert!(w.actors[6].hp < MAX_HP);
}
#[test]
fn renewa_echo_strips_buffs_and_time_break_has_a_delayed_stop() {
    let mut w = world(7);
    w.actors[0].hp = 50;
    w.actors[6].combat.haste = 10.0;
    w.actors[6].combat.guard = 100.0;
    w.actors[6].combat.guard_time = 10.0;
    skill(&mut w);
    advance(&mut w, 45);
    assert_eq!(w.actors[6].combat.haste, 0.0);
    assert!(w.actors[0].combat.haste > 0.0);
    assert!(w.actors[0].hp > 50);
    assert_eq!(w.actors[6].combat.guard, 0.0);
    let mut w = world(7);
    ally(&mut w, 1, 100, 2.0);
    ult(&mut w);
    assert!(w.actors[6].combat.slow > 0.0);
    assert_eq!(w.actors[6].combat.stun, 0.0);
    assert!(w.actors[1].combat.haste > 0.0);
    advance(&mut w, 140);
    assert!(w.actors[6].combat.stun > 0.0);
    assert!(w.actors[6].hp < MAX_HP);
}
#[test]
fn mayo_poison_ticks_and_ultimate_volley_does_not_recharge_ultimate() {
    let mut w = world(8);
    skill(&mut w);
    advance(&mut w, 45);
    assert!(w.actors[6].combat.poison > 0.0);
    let hp = w.actors[6].hp;
    advance(&mut w, 65);
    assert!(w.actors[6].hp < hp);
    let mut w = world(8);
    ult(&mut w);
    advance(&mut w, 120);
    assert!(w.actors[6].hp < MAX_HP);
    assert!((w.actors[0].combat.ultimate - 2.0 * ULT_PER_SECOND).abs() < 0.01);
}
#[test]
fn bella_flames_buff_allies_and_one_revive_is_preserved_across_respawns() {
    let mut w = world(9);
    ally(&mut w, 1, 60, 2.0);
    skill(&mut w);
    assert!(w.actors[1].combat.damage_up > 0.0);
    assert!(w.actors[0].combat.revive_armed);
    w.actors[0].hp = 1;
    w.actors[0].combat.guard = 0.0;
    w.actors[6].character = 14;
    w.actors[6].ammo = 6;
    input(
        &mut w,
        6,
        Input {
            fire: true,
            yaw: std::f32::consts::PI,
            ..Default::default()
        },
    );
    assert!(w.actors[0].alive());
    assert!(w.actors[0].combat.revive_used);
    w.actors[0].hp = 0;
    w.actors[0].respawn = DT;
    advance(&mut w, 1);
    assert!(w.actors[0].combat.revive_used);
    w.actors[0].shield = 0.0;
    skill(&mut w);
    assert!(!w.actors[0].combat.revive_armed);
    let mut w = world(9);
    ally(&mut w, 1, 30, 2.0);
    ult(&mut w);
    advance(&mut w, 100);
    assert!(w.actors[1].hp > 30);
    assert!(w.actors[6].hp < MAX_HP);
}
#[test]
fn ui_rain_restores_skills_and_wish_heals_refreshes_and_morphs() {
    let mut w = world(10);
    ally(&mut w, 1, 30, 2.0);
    w.actors[1].ability_cooldown = 8.0;
    skill(&mut w);
    advance(&mut w, 60);
    assert!(w.actors[1].hp > 30);
    assert!(w.actors[1].ability_cooldown < 6.8);
    assert!(w.actors[6].hp < MAX_HP);
    let mut w = world(10);
    ally(&mut w, 1, 30, 2.0);
    ally(&mut w, 2, 80, 3.0);
    w.actors[2].ability_cooldown = 8.0;
    ult(&mut w);
    assert_eq!(w.actors[1].hp, MAX_HP);
    assert_eq!(w.actors[1].combat.guard, 25.0);
    assert_eq!(w.actors[2].ability_cooldown, 0.0);
    assert!(w.actors[6].combat.stun > 0.0);
}
#[test]
fn butter_ricochets_to_second_enemy_and_boulder_explodes() {
    let mut w = world(11);
    w.actors[7].pos = V3::new(4.0, 0.0, 3.0);
    skill(&mut w);
    advance(&mut w, 50);
    assert!(w.actors[6].hp < MAX_HP);
    assert!(w.actors[7].hp < MAX_HP);
    let mut w = world(11);
    ult(&mut w);
    advance(&mut w, 30);
    assert!(w.actors[6].hp < MAX_HP);
    assert!(w.actors[6].pos.z > 3.0 || !w.actors[6].alive());
}
#[test]
fn komi_sleep_heals_blocks_input_and_giant_changes_size_and_attack() {
    let mut w = world(12);
    w.actors[0].hp = 30;
    w.actors[0].combat.poison = 4.0;
    skill(&mut w);
    let pos = w.actors[0].pos;
    for _ in 0..60 {
        input(
            &mut w,
            0,
            Input {
                fire: true,
                forward: 1.0,
                ..Default::default()
            },
        );
    }
    assert_eq!(w.actors[0].pos, pos);
    assert!(w.actors[0].hp > 30);
    assert_eq!(w.actors[6].hp, MAX_HP);
    assert_eq!(w.actors[0].combat.poison, 0.0);
    let mut w = world(12);
    w.actors[0].hp = 30;
    ult(&mut w);
    assert!(w.actors[0].body_scale() > 1.0);
    assert!(w.actors[0].hp >= 75);
    assert!(w.actors[6].hp < MAX_HP);
}
#[test]
fn hilde_third_attack_heals_and_overdose_hastes_allies() {
    let mut w = world(13);
    ally(&mut w, 1, 30, 2.0);
    w.actors[0].combat.attacks = 2;
    fire(&mut w);
    assert_eq!(w.actors[1].hp, 54);
    assert_eq!(w.actors[6].hp, MAX_HP);
    skill(&mut w);
    assert!(w.actors[1].hp > 54);
    assert!(w.actors[1].combat.regen > 0.0);
    let mut w = world(13);
    ally(&mut w, 1, 60, 2.0);
    ult(&mut w);
    assert!(w.actors[1].combat.haste > 0.0);
    assert!(w.actors[6].combat.physical_weaken > 0.0);
}
#[test]
fn shion_magic_rounds_are_loaded_consumed_and_restored() {
    let mut w = world(14);
    w.actors[0].combat.attacks = 3;
    fire(&mut w);
    assert_eq!(w.actors[0].combat.magic_ammo, 1);
    assert!(w.actors[6].combat.blind > 0.0);
    skill(&mut w);
    assert_eq!(w.actors[0].combat.magic_ammo, 5);
    advance(&mut w, 45);
    assert_eq!(w.actors[0].combat.magic_ammo, 0);
    let mut w = world(14);
    ult(&mut w);
    assert_eq!(w.actors[0].combat.magic_ammo, 3);
    advance(&mut w, 30);
    assert!(w.actors[6].hp < MAX_HP);
}
#[test]
fn every_skill_has_a_cooldown_and_stun_rejects_skill_and_ultimate() {
    for id in 0..15 {
        let mut w = world(id);
        skill(&mut w);
        assert!(w.actors[0].ability_cooldown > 0.0, "skill {id}");
        let cd = w.actors[0].ability_cooldown;
        skill(&mut w);
        assert!(w.actors[0].ability_cooldown <= cd);
        let mut w = world(id);
        w.actors[0].combat.stun = 1.0;
        w.actors[0].combat.ultimate = 100.0;
        input(
            &mut w,
            0,
            Input {
                fire: true,
                ability: true,
                ultimate: true,
                ..Default::default()
            },
        );
        assert_eq!(w.actors[0].combat.ultimate, 100.0);
        assert_eq!(w.actors[0].ability_cooldown, 0.0);
        assert_eq!(w.actors[6].hp, MAX_HP);
    }
}
