use super::*;

#[test]
fn every_melee_hero_leaves_one_cut_per_swing_with_vivi_finisher_variation() {
    use crate::impact::ImpactKind;
    for id in [0, 4, 6, 9, 12] {
        let mut w = shield_world(0.0, 0.0);
        for actor in &mut w.actors {
            actor.active = false;
        }
        w.actors[0].active = true;
        w.actors[0].character = id;
        w.actors[0].pos = V3::default();
        w.actors[0].yaw = 0.0;
        w.actors[0].pitch = 0.0;
        w.map = vec![Block {
            center: V3::new(0.0, 1.5, 3.0),
            size: V3::new(4.0, 3.0, 1.0),
            color: [180, 170, 150],
            phaseable: false,
        }];
        for swing in 0..3 {
            w.actors[0].cooldown = 0.0;
            w.primary_attack(0);
            assert_eq!(
                w.impacts.len(),
                swing + 1,
                "hero {id}: multi-hit attacks make one surface cut"
            );
            let hit = w.impacts[swing];
            assert_eq!(hit.kind, ImpactKind::Slash);
            assert!((hit.position.z - 2.5).abs() < 0.001);
            assert_eq!(hit.face, 4);
        }
        assert_ne!(w.impacts[0].rotation, w.impacts[1].rotation);
        if id == 0 {
            assert!(w.impacts[2].radius > w.impacts[0].radius);
        }
        w.impacts.clear();
        w.map[0].center.z = 7.0;
        w.actors[0].cooldown = 0.0;
        w.primary_attack(0);
        assert!(
            w.impacts.is_empty(),
            "hero {id}: out-of-reach wall cannot be cut"
        );
        w.map[0].center.z = -3.0;
        w.actors[0].cooldown = 0.0;
        w.primary_attack(0);
        assert!(
            w.impacts.is_empty(),
            "hero {id}: wall behind the swing cannot be cut"
        );
        w.map.clear();
        w.actors[0].pitch = -std::f32::consts::FRAC_PI_3;
        w.actors[0].cooldown = 0.0;
        w.primary_attack(0);
        assert_eq!(w.impacts.len(), 1);
        assert_eq!(w.impacts[0].face, 3);
        assert_eq!(w.impacts[0].kind, ImpactKind::Slash);
    }
}

#[test]
fn surface_impacts_follow_wall_hits_but_not_misses_or_intercepting_players() {
    let mut w = shield_world(0.0, 0.0);
    for actor in &mut w.actors {
        actor.active = false;
    }
    w.actors[0].active = true;
    w.actors[0].pos = V3::new(0.0, 0.0, 0.0);
    w.actors[0].yaw = 0.0;
    w.actors[0].pitch = 0.0;
    w.map = vec![Block {
        center: V3::new(0.0, 1.5, 5.0),
        size: V3::new(4.0, 3.0, 1.0),
        color: [180, 170, 150],
        phaseable: false,
    }];
    w.ray(0, 3.0, 10.0, 1, AttackClass::Basic, false);
    assert!(w.impacts.is_empty(), "range expiry is not a collision");
    w.ray(0, 10.0, 10.0, 1, AttackClass::Basic, false);
    assert_eq!(w.impacts.len(), 1);
    assert_eq!(
        w.impacts[0].position,
        V3::new(0.0, w.actors[0].eye().y, 4.5)
    );
    assert_eq!(w.impacts[0].face, 4);
    w.actors[6].active = true;
    w.actors[6].pos = V3::new(0.0, 0.0, 2.0);
    w.ray(0, 10.0, 10.0, 1, AttackClass::Basic, false);
    assert_eq!(
        w.impacts.len(),
        1,
        "a blocked ray cannot scar the wall behind its target"
    );
    w.actors[6].active = false;
    w.projectile(
        0,
        ProjectileKind::Bolt,
        V3::new(0.0, 0.0, 1.0),
        60.0,
        10.0,
        AttackClass::Skill,
    );
    for _ in 0..6 {
        w.advance_projectiles();
    }
    assert!(w.projectiles.is_empty());
    assert_eq!(w.impacts.len(), 2);
    assert!(
        (w.impacts[1].position.z - 4.5).abs() < 0.001,
        "projectile padding must not offset the surface mark"
    );
    w.projectile(
        0,
        ProjectileKind::Bolt,
        V3::new(1.0, 0.0, 0.0),
        60.0,
        10.0,
        AttackClass::Basic,
    );
    w.projectiles[0].life = DT;
    w.advance_projectiles();
    assert_eq!(
        w.impacts.len(),
        2,
        "expired airborne projectiles create no debris"
    );
}

#[test]
fn crouching_ducks_under_head_height_shots_and_can_return_fire() {
    let mut w = shield_world(0.0, 0.0);
    w.actors[0].pos.y = 0.15;
    w.actors[6].crouching = true;
    assert_eq!(w.ray(0, 10.0, 10.0, 1, AttackClass::Basic, false), None);
    assert_eq!(w.actors[6].hp, MAX_HP);
    w.actors[0].crouching = true;
    assert_eq!(w.ray(0, 10.0, 10.0, 1, AttackClass::Basic, false), Some(6));
    assert!(w.actors[6].hp < MAX_HP);
    w.actors[0].crouching = false;
    w.actors[6].crouching = false;
    assert_eq!(w.ray(0, 10.0, 10.0, 1, AttackClass::Basic, false), Some(6));
}

#[test]
fn sanctuary_blocks_all_damage_and_displacement_for_both_sides_and_objectives() {
    for attacker in 0..2 {
        for point in 0..2 {
            for target in [0, 6] {
                let mut w = shield_world(0.0, 0.0);
                w.attacker = attacker;
                w.point = point;
                let source = if target == 0 { 6 } else { 0 };
                w.actors[target].pos = w.spawn_center(team(target));
                w.actors[target].barrier = 9.0;
                w.actors[target].combat.guard = 20.0;
                let origin = w.actors[target].pos;
                for class in [
                    AttackClass::Basic,
                    AttackClass::Skill,
                    AttackClass::Ultimate,
                ] {
                    assert!(!w.damage_hit(source, target, 1000.0, class, 0));
                }
                w.push(target, V3::new(0.0, 0.0, 1.0), 10.0);
                w.taunt(source, target, 4.0);
                w.dot(source, target, false, 5.0, AttackClass::Skill);
                assert_eq!(w.actors[target].hp, MAX_HP);
                assert_eq!(w.actors[target].shield_health(), 29.0);
                assert_eq!(w.actors[target].pos, origin);
                assert_eq!(w.actors[target].combat.taunt, 0.0);
                assert_eq!(w.actors[target].combat.poison, 0.0);
                assert_eq!(w.actors[source].combat.ultimate, 0.0);
                assert_eq!(w.actors[target].killed_by, None);
                w.actors[target].pos.z += SPAWN_HALF_DEPTH + 0.01;
                assert!(w.damage_hit(source, target, 10.0, AttackClass::Basic, 0));
                w.actors[target].pos = origin;
                assert!(!w.damage_hit(source, target, 1000.0, AttackClass::Basic, 0));
            }
        }
    }
}

#[test]
fn sanctuary_is_positional_even_after_firing_and_dots_pause_damage_inside() {
    let mut w = shield_world(0.0, 0.0);
    w.phase = Phase::Playing;
    w.actors[6].pos = w.spawn_center(1);
    w.actors[0].pos = w.actors[6].pos + V3::new(0.0, 0.0, -4.0);
    w.actors[6].character = 14;
    w.actors[6].ammo = 30;
    w.primary_attack(6); // An attack consumes the old timed shield, never zone protection.
    assert!(w.in_spawn(6));
    assert!(!w.damage_hit(0, 6, 1000.0, AttackClass::Basic, 0));
    w.actors[6].combat.poison = 4.0;
    w.actors[6].combat.dots[0] = DamageOverTime {
        tick: 0.0,
        owner: 0,
        character: 8,
        class: Some(AttackClass::Skill),
    };
    w.advance_combat();
    assert_eq!(w.actors[6].hp, MAX_HP);
    w.actors[6].pos.z -= SPAWN_HALF_DEPTH + 0.1;
    w.actors[6].combat.dots[0].tick = 0.0;
    w.advance_combat();
    assert!(w.actors[6].hp < MAX_HP);
    // Enemy and inactive spawn rooms grant no immunity.
    w.actors[6].pos = w.spawn_center(0);
    assert!(w.damage_hit(0, 6, 1.0, AttackClass::Basic, 0));
    w.actors[6].pos = w.spawn_center(1);
    w.point = 1;
    // This layout deliberately shares the defender room between A and B.
    assert!(!w.damage_hit(0, 6, 1.0, AttackClass::Basic, 0));
    w.point = 0;
    w.actors[0].pos = w.spawn_center(0);
    w.point = 1;
    assert!(w.damage_hit(6, 0, 1.0, AttackClass::Basic, 0));
}

fn shield_world(guard: f32, barrier: f32) -> World {
    let mut w = World::default();
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.human = true;
        a.shield = 0.0;
        a.pos = V3::new(40.0 + i as f32 * 2.0, 0.0, 40.0);
    }
    w.actors[0].character = 0;
    w.actors[6].character = 0;
    w.actors[0].pos = V3::default();
    w.actors[6].pos = V3::new(0.0, 0.0, 3.0);
    w.actors[6].combat.guard = guard;
    w.actors[6].combat.guard_time = 6.0;
    w.actors[6].barrier = barrier;
    w
}

fn hit(w: &mut World, damage: f32, class: AttackClass) {
    // Express fixtures in final pre-shield damage regardless of roster synergy.
    let base = damage / w.damage_multiplier(team(0));
    assert!(w.damage_hit(0, 6, base, class, 0));
}

#[test]
fn shield_threshold_applies_to_every_attack_class_and_shield_type() {
    for class in [
        AttackClass::Basic,
        AttackClass::Skill,
        AttackClass::Ultimate,
    ] {
        for (guard, barrier) in [(45.0, 0.0), (0.0, 9.6), (45.0, 9.6)] {
            for (damage, absorbed) in [(4.0, 4.0), (10.0, 10.0), (11.0, 5.5), (20.0, 10.0)] {
                let mut w = shield_world(guard, barrier);
                hit(&mut w, damage, class);
                assert!(
                    (w.actors[6].shield_health() - (guard + barrier - absorbed).max(0.0)).abs()
                        < 0.001
                );
                let overflow = (absorbed - guard - barrier).max(0.0).ceil() as u16;
                assert_eq!(w.actors[6].hp, MAX_HP - overflow);
            }
        }
    }
}

#[test]
fn unshielded_hits_keep_full_damage() {
    for damage in [4.0, 10.0, 11.0, 20.0, 60.0] {
        let mut w = shield_world(0.0, 0.0);
        hit(&mut w, damage, AttackClass::Basic);
        assert_eq!(w.actors[6].hp, MAX_HP - damage as u16);
    }
}

#[test]
fn shield_overflow_is_reduced_once_and_next_unshielded_hit_is_full() {
    let mut w = shield_world(3.0, 2.0);
    hit(&mut w, 40.0, AttackClass::Basic);
    assert_eq!(w.actors[6].shield_health(), 0.0);
    assert_eq!(w.actors[6].hp, MAX_HP - 15);
    hit(&mut w, 20.0, AttackClass::Basic);
    assert_eq!(w.actors[6].hp, MAX_HP - 35);
}

#[test]
fn skill_guard_absorbs_before_racial_barrier_without_double_reduction() {
    let mut w = shield_world(3.0, 9.6);
    hit(&mut w, 20.0, AttackClass::Skill);
    assert_eq!(w.actors[6].combat.guard, 0.0);
    assert!((w.actors[6].barrier - 2.6).abs() < 0.001);
    assert_eq!(w.actors[6].hp, MAX_HP);
}

#[test]
fn threshold_uses_damage_after_buffs_and_debuffs() {
    let mut w = shield_world(45.0, 0.0);
    w.actors[6].combat.vulnerable = 4.0;
    hit(&mut w, 10.0, AttackClass::Basic); // 10 * 1.15 -> 12 -> 6.
    assert_eq!(w.actors[6].combat.guard, 39.0);
    let mut w = shield_world(45.0, 0.0);
    w.actors[0].combat.weaken = 4.0;
    hit(&mut w, 12.0, AttackClass::Basic); // 12 * 0.8 -> 10, no reduction.
    assert_eq!(w.actors[6].combat.guard, 35.0);
}

#[test]
fn ultimate_charge_counts_only_absorbed_damage() {
    let mut w = shield_world(45.0, 0.0);
    hit(&mut w, 20.0, AttackClass::Basic);
    assert!((w.actors[0].combat.ultimate - 10.0 * ULT_PER_DAMAGE).abs() < 0.001);
    hit(&mut w, 20.0, AttackClass::Ultimate);
    assert!((w.actors[0].combat.ultimate - 10.0 * ULT_PER_DAMAGE).abs() < 0.001);
}

#[test]
fn mercury_break_still_bursts_once_and_expiry_removes_mitigation() {
    let mut w = shield_world(5.0, 0.0);
    w.actors[6].combat.mercury = true;
    hit(&mut w, 20.0, AttackClass::Basic);
    assert_eq!(w.actors[6].hp, MAX_HP - 5);
    assert!(!w.actors[6].combat.mercury);
    assert!(w.actors[0].hp < MAX_HP);
    assert!(w.actors[0].combat.vulnerable > 0.0);
    let attacker_hp = w.actors[0].hp;
    hit(&mut w, 20.0, AttackClass::Basic);
    assert_eq!(w.actors[0].hp, attacker_hp);

    let mut w = shield_world(45.0, 0.0);
    w.phase = Phase::Playing;
    w.actors[6].combat.guard_time = DT;
    w.tick([Input::default(); PLAYERS]);
    assert_eq!(w.actors[6].shield_health(), 0.0);
    hit(&mut w, 20.0, AttackClass::Basic);
    assert_eq!(w.actors[6].hp, MAX_HP - 20);
}

#[test]
fn spawn_invulnerability_still_rejects_damage() {
    let mut w = shield_world(45.0, 0.0);
    w.actors[6].shield = 2.0;
    assert!(!w.damage_hit(0, 6, 100.0, AttackClass::Basic, 0));
    assert_eq!(w.actors[6].combat.guard, 45.0);
    assert_eq!(w.actors[6].hp, MAX_HP);
    assert_eq!(w.actors[0].combat.ultimate, 0.0);
}
