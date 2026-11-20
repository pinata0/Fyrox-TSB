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
        advance(&mut w, if id == 7 { 100 } else { 25 });
        assert!(w.actors[6].hp < MAX_HP, "primary must hit: {id}");
        if kit(id).magazine == 0 {
            assert_eq!(w.actors[0].ammo, 0);
        } else {
            assert_eq!(w.actors[0].ammo, start_ammo - 1);
        }
    }
}

#[test]
fn vivi_taps_reset_to_left_and_do_not_repeat_during_cooldown() {
    let mut w=world(0);
    let mut damage=Vec::new();
    for _ in 0..3 {
        w.actors[6].hp=MAX_HP;
        fire(&mut w);
        assert_eq!(w.actors[0].combat.vivi_combo,1);
        let shot=w.actors[0].shot;
        fire(&mut w);
        assert_eq!(w.actors[0].shot,shot);
        advance(&mut w,50);
        damage.push(MAX_HP-w.actors[6].hp);
        assert_eq!(w.actors[0].ammo,0);
    }
    assert!(damage[0]>0);
    assert!(damage.iter().all(|d|*d==damage[0]));
    w.clear_combat_on_spawn(0);
    assert_eq!(w.actors[0].combat.vivi_combo,0);
}

#[test]
fn vivi_misses_finish_the_gesture_but_disabled_inputs_do_not_start_another() {
    let mut w=world(0);w.actors[6].pos.z=30.0;fire(&mut w);
    for _ in 0..48 {fire(&mut w);}
    assert_eq!(w.actors[0].combat.vivi_combo,2);
    assert_eq!(w.actors[6].hp,MAX_HP);
    w.actors[0].combat.stun=1.0;
    let shot=w.actors[0].shot;fire(&mut w);
    assert_eq!(w.actors[0].shot,shot);
    assert_eq!(w.actors[0].combat.status_text(),"STUN");
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
            // Only ghosts preserve grounded basic attacks while using Shift.
            // Elves place portals, spirits charge emergence, and beasts launch airborne.
            if phase_shift && !matches!(id, 9 | 14) {
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
                if id == 11 {
                    assert_eq!(w.actors[0].shot, 0, "Butter holds until release");
                    input(&mut w, 0, Input {forward, strafe, ..Default::default()});
                }
                assert_eq!(
                    w.actors[0].shot, if id == 7 { 0 } else { 1 },
                    "must attack: {id}, phase_shift={phase_shift}"
                );
                assert_eq!(w.actors[0].ammo, start_ammo.saturating_sub(1));
                advance(&mut w, if id == 7 { 100 } else { 25 });
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
        for tick in 0..if id == 7 { 300 } else { 90 } {
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
                    fire: id != 11 || tick % 60 < 20,
                    ..movement
                },
            );
            input(&mut moving_only, 0, movement);
            input(&mut firing_only, 0, Input { fire: id != 11 || tick % 60 < 20, ..Default::default() });
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
        if id == 5 { advance(&mut w, 11); } // Picora frame-18 release.
        if id == 7 { advance(&mut w, 79); }
        if id == 11 { advance(&mut w, 1); } // Butter fires on button release.
        assert!(!w.projectiles.is_empty(), "projectile release {id}");
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
    advance(&mut w, 70);
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
fn poison_refresh_is_independent_of_aya_freeze_and_has_no_periodic_damage() {
    let mut w = world(8);
    w.actors[6].combat.guard = 120.0;
    w.actors[6].combat.guard_time = 10.0;
    skill(&mut w);
    advance(&mut w, 70);
    assert!(w.actors[6].combat.poison > 3.8);
    w.actors[1].character = 4;
    w.actors[1].pos = V3::new(0.5, 0.0, 0.0);
    w.actors[1].yaw = (-0.5_f32).atan2(3.0);
    w.actors[1].combat.ultimate = 100.0;
    input(&mut w, 1, Input {ultimate: true, ..Default::default()});
    advance(&mut w, 114);
    assert!(w.actors[6].combat.frozen > 0.0);
    assert!(w.actors[6].combat.poison > 0.0);
    let hp = w.actors[6].hp;
    let guard = w.actors[6].combat.guard;
    let charge = w.actors[0].combat.ultimate;
    advance(&mut w, 70);
    assert_eq!(w.actors[6].hp, hp);
    assert_eq!(w.actors[6].combat.guard, guard);
    assert!((w.actors[0].combat.ultimate - charge - 70.0 * DT * ULT_PER_SECOND).abs() < 0.001);
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
    assert!(!w.actors[0].combat.komi.active());
    w.actors[0].combat.ultimate = 100.0;
    input(
        &mut w,
        0,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    assert!(!w.actors[0].combat.komi.active());
    advance(&mut w, 1);
    input(
        &mut w,
        0,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].combat.komi.phase,trickcal_shooting_balddagu::komi_skill::Phase::Feast);
    assert_eq!(w.actors[0].combat.ultimate, 0.0);
    assert_eq!(w.actors[6].hp,MAX_HP,"feast does not damage before a leap lands");
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
        if id == 5 { w.actors[1].pos.x = 0.0; }
        if id==0 {w.actors[6].pos.z=12.0;}
        ult(&mut w);
        assert_eq!(w.actors[0].combat.ultimate, 0.0, "ultimate {id}");
        assert!(w.actors[0].combat.ultimate_flash > 0.0 || w.actors[0].combat.komi.active()
            || id == 6 && w.projectiles.iter().any(|p|p.kind==ProjectileKind::Doll));
        assert!(w.event.contains(kit(id).ultimate));
        advance(&mut w, if id == 7 { 385 } else if id == 9 { 300 } else { 180 });
        if id == 13 {fire(&mut w);advance(&mut w,20);}
        let changed = w.actors[6].hp < MAX_HP
            || w.actors[1].hp > 30
            || w.actors
                .iter()
                .any(|a| a.combat.guard > 0.0 || a.combat.komi.active() || a.combat.giant > 0.0 || a.combat.stun > 0.0 || a.combat.polymorph > 0.0 || a.combat.temporal > 0.0);
        assert!(changed, "ultimate has gameplay effect: {id}");
    }
}
#[test]
fn vivi_guard_explodes_and_spear_weakens_and_taunts() {
    let mut w = world(0);
    skill(&mut w);
    advance(&mut w, 40);
    assert_eq!(w.actors[0].combat.guard, trickcal_shooting_balddagu::vivi_skill::SHIELD_HEALTH);
    advance(&mut w, 365);
    assert_eq!(w.actors[0].combat.guard, 0.0);
    assert!(w.actors[6].combat.vulnerable > 0.0);
    assert!(w.actors[6].hp < MAX_HP);
    let mut w = world(0);
    w.actors[6].pos.z=12.0;w.actors[6].hp=MAX_HP*2;w.actors[6].combat.fashion=8.0;
    ult(&mut w);
    advance(&mut w, 125);
    assert!(w.actors[6].combat.taunt > 0.0);
    assert!(w.actors[6].combat.weaken > 0.0);
}
#[test]
fn naia_wash_cleanses_stun_and_wave_shields_allies_and_hits_enemies() {
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
    advance(&mut w, 160);
    assert_eq!(w.actors[1].hp, 30);
    assert_eq!(w.actors[1].combat.guard, 100.0);
    assert!(w.actors[6].hp < MAX_HP);
}
#[test]
fn erpin_fires_four_bombs_and_charge_cannot_cross_walls() {
    let mut w = world(2);
    skill(&mut w);
    assert!(w.projectiles.is_empty(), "source windup must precede release");
    advance(&mut w, 101);
    assert_eq!(w.actors[6].hp, MAX_HP);
    advance(&mut w, 40);
    assert!(w.actors[6].hp < MAX_HP);
    let mut w = world(2);
    w.map.push(wall(1.5));
    ult(&mut w);
    advance(&mut w, 80);
    assert!(w.actors[0].pos.z < 1.0);
    assert_eq!(w.actors[0].combat.erpin_skill.phase, trickcal_shooting_balddagu::erpin_skill::Phase::Recover);
    assert_eq!(w.actors[6].hp, MAX_HP);
}
#[test]
fn aya_butterflies_return_and_snow_garden_freezes_nearby_enemies() {
    let mut w = world(4);
    w.actors[6].pos.z = 7.0;
    skill(&mut w);
    advance(&mut w, 110);
    let hp = w.actors[6].hp;
    assert!(hp < MAX_HP);
    assert!(w.actors[6].combat.weaken > 0.0);
    advance(&mut w, 95);
    assert!(w.actors[6].hp < hp);
    let mut w = world(4);
    w.actors[7].pos = V3::new(5.0, 0.0, 3.0);
    ult(&mut w);
    advance(&mut w, 114);
    assert!(w.actors[6].combat.frozen > 0.0);
    assert!(w.actors[7].combat.frozen > 0.0);
}
#[test]
fn picora_sticker_heals_aimed_ally_and_fashion_doubles_health() {
    let mut w = world(5);
    ally(&mut w, 1, 30, 4.0);
    w.actors[1].pos.x = 0.0;
    skill(&mut w);
    advance(&mut w, 50);
    assert!(w.actors[1].hp > 30);
    assert!(w.actors[1].combat.sticker > 0.0);
    assert_eq!(w.actors[1].combat.physical_resist, 0.0);
    advance(&mut w, 83);
    let hp = w.actors[1].hp;
    ult(&mut w);
    advance(&mut w, 26);
    assert_eq!(w.actors[1].combat.guard, 0.0);
    assert_eq!(w.actors[1].max_hp(), MAX_HP * 2);
    assert_eq!(w.actors[1].hp, hp * 2);
    advance(&mut w, 481);
    assert_eq!(w.actors[1].max_hp(), MAX_HP);
    assert_eq!(w.actors[1].hp, hp);
}
#[test]
fn chloe_mounts_then_attacks_and_summons_nine_explosive_dolls() {
    let mut w = world(6);
    skill(&mut w);
    assert_eq!(w.actors[0].combat.chloe.phase, trickcal_shooting_balddagu::chloe_skill::Phase::Mount);
    advance(&mut w,120);
    fire(&mut w);
    advance(&mut w,14);
    assert_eq!(w.actors[0].combat.chloe.hits, 1);
    assert!(w.actors[6].hp < MAX_HP);
    let mut w = world(6);
    ult(&mut w);
    assert_eq!(w.projectiles.len(), 9);
    assert!(w.projectiles.iter().all(|p| p.kind == ProjectileKind::Doll));
    advance(&mut w, 90);
    assert!(w.actors[6].hp < MAX_HP);
}
#[test]
fn renua_skill_hits_restore_hp_without_stripping_buffs_and_field_affects_allies() {
    let mut w = world(7);
    w.actors[0].hp = 50;
    w.actors[6].combat.haste = 10.0;
    skill(&mut w);
    advance(&mut w, 225);
    assert!(w.actors[6].combat.haste > 0.0);
    assert_eq!(w.actors[0].combat.haste, 0.0);
    assert!(w.actors[0].hp > 50);
    assert_eq!(w.actors[0].shot, 5);
    let mut w = world(7);
    ally(&mut w, 1, 100, 2.0);
    ult(&mut w);
    assert_eq!(w.actors[6].combat.temporal, 0.0);
    advance(&mut w, 385);
    assert_eq!(w.actors[6].combat.temporal, 1.0);
    assert_eq!(w.actors[1].combat.temporal, 1.0);
    assert_eq!(w.actors[1].combat.haste, 0.0);
    assert_eq!(w.actors[6].hp, MAX_HP);
}

#[test]
fn mayo_poison_debuff_and_ultimate_volley_does_not_recharge_ultimate() {
    let mut w = world(8);
    skill(&mut w);
    advance(&mut w, 70);
    assert!(w.actors[6].combat.poison > 0.0);
    let hp = w.actors[6].hp;
    advance(&mut w, 65);
    assert_eq!(w.actors[6].hp, hp);
    let mut w = world(8);
    ult(&mut w);
    advance(&mut w, 120);
    assert!(w.actors[6].hp < MAX_HP);
    assert!((w.actors[0].combat.ultimate - 2.0 * ULT_PER_SECOND).abs() < 0.01);
}
#[test]
fn bella_fire_taunts_and_lantern_heals_only_after_the_complete_cast() {
    let mut w = world(9);
    ally(&mut w, 1, 30, 2.0);
    skill(&mut w);
    assert_eq!(w.actors[1].combat.damage_up, 0.0);
    advance(&mut w, 150);
    assert!(w.actors[6].hp < MAX_HP);
    assert!(w.actors[6].combat.taunt > 0.0);
    let mut w = world(9);
    ally(&mut w, 1, 30, 2.0);
    ult(&mut w);
    advance(&mut w, 299);
    assert_eq!(w.actors[1].hp, 30);
    advance(&mut w, 1);
    assert!(w.actors[1].hp >= 78);
    assert_eq!(w.actors[6].hp, MAX_HP);
}
#[test]
fn ui_rain_and_wish_apply_the_requested_team_effects() {
    let mut w = world(10);
    ally(&mut w, 1, 30, 2.0);
    w.actors[1].ability_cooldown = 8.0;
    skill(&mut w);
    advance(&mut w, 90);
    assert!(w.actors[1].hp > 30);
    assert!(w.actors[6].hp < MAX_HP);
    let mut w = world(10);
    ally(&mut w, 1, 30, 2.0);
    ally(&mut w, 2, 80, 3.0);
    w.actors[1].pos = V3::new(-1.0, 0.0, 3.0);
    w.actors[2].pos = V3::new(1.0, 0.0, 3.0);
    w.actors[1].ability_cooldown = 8.0;
    w.actors[2].ability_cooldown = 8.0;
    ult(&mut w);
    advance(&mut w, 58);
    assert_eq!(w.actors[1].hp, 30);
    assert_eq!(w.actors[1].combat.guard, 0.0);
    assert_eq!(w.actors[1].ability_cooldown, 0.0);
    assert_eq!(w.actors[2].ability_cooldown, 0.0);
    assert!(w.actors[6].combat.polymorph > 0.0);
    assert_eq!(w.actors[6].combat.stun, 0.0);
}
#[test]
fn butter_hover_shot_reflects_off_players_and_boulder_explodes_after_windup() {
    let mut w = world(11);
    skill(&mut w);
    // Track the grounded target from the actual airborne camera.
    for _ in 0..80 {
        input(&mut w, 0, Input { pitch: -0.28, ..Default::default() });
    }
    assert!(w.actors[6].hp < MAX_HP);
    assert!(w.projectiles.iter().any(|p| p.kind == ProjectileKind::Ricochet && p.bounces < 6 && p.velocity.z < 0.0));
    let mut w = world(11);
    ult(&mut w);
    advance(&mut w, 180);
    assert!(w.actors[6].hp < MAX_HP);
    assert!(w.actors[6].combat.stun > 2.0);
    assert!(w.actors[6].pos.z > 3.0 || !w.actors[6].alive());
}
#[test]
fn komi_rest_cleanses_and_can_be_cancelled_then_feast_grows_without_basic_attack() {
    let mut w=world(12);w.actors[0].hp=30;w.actors[0].combat.poison=4.0;
    skill(&mut w);advance(&mut w,60);assert_eq!(w.actors[0].hp,42);assert_eq!(w.actors[0].combat.poison,0.0);
    input(&mut w,0,Input{forward:1.0,..Default::default()});assert!(!w.actors[0].combat.komi.active());assert!(w.actors[0].pos.z>0.0);
    let mut w=world(12);ult(&mut w);advance(&mut w,264);assert!(w.actors[0].body_scale()>1.0);
    fire(&mut w);assert_eq!(w.actors[6].hp,MAX_HP);assert!(w.actors[0].combat.komi.giant());
}

#[test]
fn hilde_syringes_heal_the_first_ally_on_impact_for_both_teams() {
    for source in [0, TEAM_SIZE] {
        let mut w = world(13);
        w.actors.swap(0, source);
        w.actors[source].hp = 40;
        let near = source + 2;
        let far = source + 1;
        let enemy = (source + TEAM_SIZE) % PLAYERS;
        for (target, z) in [(near, 5.0), (far, 7.0)] {
            w.actors[target].character = 12;
            w.actors[target].hp = 30;
            w.actors[target].pos = V3::new(0.0, 0.0, z);
        }
        w.actors[enemy].pos = V3::new(0.0, 0.0, 9.0);
        input(
            &mut w,
            source,
            Input {
                fire: true,
                ..Default::default()
            },
        );
        assert_eq!(w.actors[near].hp, 30, "healing waits for impact");
        assert_eq!(w.projectiles.len(), 1);
        assert_eq!(w.actors[source].ammo, kit(13).magazine - 1);
        advance(&mut w, 15);
        assert_eq!(w.actors[near].hp, 38);
        assert_eq!(w.actors[far].hp, 30, "syringes do not pierce allies");
        assert_eq!(w.actors[enemy].hp, MAX_HP);
        assert_eq!(w.actors[source].hp, 40, "syringes cannot heal their owner");
        assert_eq!(w.actors[near].hurt, 0.0);
        assert!(w.projectiles.is_empty());
    }
}

#[test]
fn hilde_syringe_healing_respects_max_hp_and_received_healing_bonuses() {
    for (character, hp, boosted, expected) in [
        (12, MAX_HP - 3, false, MAX_HP),
        (12, MAX_HP, false, MAX_HP),
        (2, 30, false, 39),
        (12, 30, true, 40),
    ] {
        let mut w = world(13);
        w.actors[1].character = character;
        w.actors[1].hp = hp;
        w.actors[1].combat.healing_up = if boosted { 2.0 } else { 0.0 };
        w.actors[1].pos = V3::new(0.0, 0.0, 3.0);
        w.actors[6].pos.z = 6.0;
        fire(&mut w);
        advance(&mut w, 12);
        assert_eq!(w.actors[1].hp, expected);
        assert_eq!(
            w.actors[6].hp, MAX_HP,
            "even full-health allies stop syringes"
        );
        assert!(w.projectiles.is_empty());
    }
}

#[test]
fn hilde_syringes_cannot_heal_through_walls() {
    let mut w = world(13);
    w.actors[1].hp = 30;
    w.actors[1].pos = V3::new(0.0, 0.0, 5.0);
    w.actors[6].pos.z = 8.0;
    w.map.push(wall(2.0));
    fire(&mut w);
    advance(&mut w, 15);
    assert_eq!(w.actors[1].hp, 30);
    assert_eq!(w.actors[6].hp, MAX_HP);
    assert!(w.projectiles.is_empty());
}

#[test]
fn hilde_syringes_ignore_dead_inactive_and_burrowed_allies() {
    for state in 0..3 {
        let mut w = world(13);
        w.actors[1].character = 1;
        w.actors[1].hp = 30;
        w.actors[1].pos = V3::new(0.0, 0.0, 2.0);
        match state {
            0 => {
                w.actors[1].hp = 0;
                w.actors[1].respawn = 7.0;
            }
            1 => w.actors[1].active = false,
            _ => w.actors[1].race_skill.remaining = 2.0,
        }
        w.actors[6].pos.z = 5.0;
        fire(&mut w);
        advance(&mut w, 12);
        assert_eq!(w.actors[1].hp, if state == 0 { 0 } else { 30 });
        assert!(w.actors[6].hp < MAX_HP);
        assert!(w.projectiles.is_empty());
    }
}

#[test]
fn other_primary_projectiles_still_pass_through_allies() {
    for id in [1, 5, 7, 8, 10, 11] {
        let mut w = world(id);
        w.actors[1].hp = 30;
        w.actors[1].pos = V3::new(0.0, 0.0, 2.0);
        w.actors[6].pos.z = 5.0;
        fire(&mut w);
        advance(&mut w, if id == 7 { 100 } else if id == 5 { 32 } else { 20 });
        assert_eq!(w.actors[1].hp, 30, "hero {id} must not heal or hurt allies");
        assert!(w.actors[6].hp < MAX_HP, "hero {id} still hits enemies");
    }
}

#[test]
fn hilde_third_attack_heals_and_skills_use_source_timed_medical_equipment() {
    let mut w = world(13);
    ally(&mut w, 1, 30, 2.0);
    w.actors[0].combat.attacks = 2;
    fire(&mut w);
    assert_eq!(w.actors[1].hp, 54);
    assert_eq!(w.actors[6].hp, MAX_HP);
    skill(&mut w);
    assert_eq!(w.actors[1].hp, 54);
    advance(&mut w, 203);
    assert!(w.actors[1].hp >= 94);
    assert_eq!(w.actors[1].combat.regen, 0.0);
    let mut w = world(13);
    ally(&mut w, 1, 60, 2.0);
    ult(&mut w);
    assert_eq!(w.actors[0].combat.hilde.phase, trickcal_shooting_balddagu::hilde_skill::Phase::Equip);
    assert_eq!(w.actors[1].combat.haste, 0.0);
    assert_eq!(w.actors[6].combat.physical_weaken, 0.0);
}

#[test]
fn shion_basic_round_bonus_and_delayed_marksman_charge_are_separate() {
    let mut w = world(14);
    w.actors[0].combat.attacks = 3;
    fire(&mut w);
    assert_eq!(w.actors[0].combat.magic_ammo, 1);
    assert!(w.actors[6].combat.blind > 0.0);
    skill(&mut w);
    assert_eq!(w.actors[0].combat.magic_ammo, 0);
    advance(&mut w, 45);
    assert_eq!(w.actors[0].combat.magic_ammo, 0);
    advance(&mut w, 83);
    assert_eq!(w.actors[0].combat.magic_ammo, 5);
    assert_eq!(w.actors[0].combat.shion.phase, trickcal_shooting_balddagu::shion_skill::Phase::Ready);
    let mut w = world(14);
    ult(&mut w);
    assert_eq!(w.actors[0].combat.magic_ammo, 0);
    advance(&mut w, 30);
    assert_eq!(w.actors[6].hp, MAX_HP);
    advance(&mut w, 130);
    assert_eq!(w.actors[6].hp, 0);
}
#[test]
fn every_skill_has_a_cooldown_and_stun_rejects_skill_and_ultimate() {
    for id in 0..15 {
        let mut w = world(id);
        if id == 5 { w.actors[1].pos = V3::new(0.0, 0.0, 2.0); }
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
