use trickcal_shooting_balddagu::{
    chloe_skill::{self as c, Phase as Action, State},
    combat::{AttackClass, Projectile, ProjectileKind},
    game::*,
    motion::{Motion, MotionClock},
    sfx::{Cue, Tracker},
};

fn world() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.active = i == 0;
        a.human = true;
        a.character = 6;
        a.hp = MAX_HP;
        a.shield = 0.0;
        a.barrier = 0.0;
        a.combat = Default::default();
        a.pos = V3::default();
        a.cooldown = 0.0;
        a.ammo = 20;
    }
    w
}
fn tick(w: &mut World, input: Input, n: usize) {
    for _ in 0..n {
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0] = input;
        w.tick(inputs);
    }
}
fn advance(w: &mut World, n: usize) {
    tick(w, Input::default(), n);
}
fn enemy(w: &mut World, index: usize, pos: V3) {
    w.actors[index].active = true;
    w.actors[index].pos = pos;
    w.actors[index].hp = MAX_HP;
    w.actors[index].shield = 0.0;
    w.actors[index].barrier = 0.0;
    w.actors[index].combat = Default::default();
}
fn ride(w: &mut World) {
    w.actors[0].combat.chloe = State {
        phase: Action::Ride,
        remaining: c::RIDE_SECONDS,
        ..Default::default()
    };
}
#[test]
fn attack_waits_then_shoves_while_locomotion_continues() {
    let mut w = world();
    enemy(&mut w, TEAM_SIZE, V3::new(0.0, 0.0, 2.0));
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(w.actors[0].combat.chloe.phase, Action::Attack);
    assert_eq!(w.actors[TEAM_SIZE].hp, MAX_HP);
    advance(&mut w, 13);
    assert_eq!(w.actors[TEAM_SIZE].hp, MAX_HP);
    tick(
        &mut w,
        Input {
            strafe: 1.0,
            ..Default::default()
        },
        1,
    );
    let expected_step = trickcal_shooting_balddagu::movement::WALK_SPEED * DT;
    assert!((w.actors[0].pos.x.abs() - expected_step).abs() < 0.001);
    assert_eq!(w.actors[TEAM_SIZE].hp, MAX_HP - 27);
    assert!(w.actors[TEAM_SIZE].pos.z > 2.5);
    advance(&mut w, 35);
    assert_eq!(w.actors[TEAM_SIZE].hp, MAX_HP - 27);
    assert_eq!(w.actors[0].combat.chloe.phase, Action::Off);
}
#[test]
fn mount_raises_camera_and_expiration_forces_dismount() {
    let mut w = world();
    let low = w.actors[0].eye().y;
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(w.actors[0].combat.chloe.phase, Action::Mount);
    tick(
        &mut w,
        Input {
            forward: 1.0,
            jump: true,
            ..Default::default()
        },
        120,
    );
    assert!(w.actors[0].pos.length() < 0.001);
    assert_eq!(w.actors[0].combat.chloe.phase, Action::Ride);
    assert!((w.actors[0].eye().y - low - c::RIDE_LIFT).abs() < 0.001);
    assert_eq!(w.actors[0].combat.chloe.remaining, c::RIDE_SECONDS);
    advance(&mut w, 721);
    assert_eq!(w.actors[0].combat.chloe.phase, Action::Dismount);
    advance(&mut w, 120);
    assert_eq!(w.actors[0].combat.chloe.phase, Action::Off);
    assert!((w.actors[0].eye().y - low).abs() < 0.001);
}
#[test]
fn nine_landed_swings_reset_timer_and_roar_once_in_every_direction() {
    let mut w = world();
    ride(&mut w);
    for hit in 1..=9 {
        enemy(&mut w, TEAM_SIZE, V3::new(0.0, 0.0, 2.0));
        enemy(&mut w, TEAM_SIZE + 1, V3::new(0.3, 0.0, 2.0));
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
            1,
        );
        advance(&mut w, 14);
        assert_eq!(
            w.actors[0].combat.chloe.hits,
            if hit == 9 { 0 } else { hit }
        );
        if hit < 9 {
            advance(&mut w, 35);
        }
    }
    assert_eq!(w.actors[0].combat.chloe.phase, Action::Roar);
    assert_eq!(w.actors[0].combat.chloe.remaining, c::RIDE_SECONDS);
    for (i, pos) in [
        (TEAM_SIZE, V3::new(0.0, 0.0, -3.0)),
        (TEAM_SIZE + 1, V3::new(3.0, 0.0, 0.0)),
        (1, V3::new(0.0, 0.0, 2.0)),
    ] {
        enemy(&mut w, i, pos);
    }
    let start = w.actors[0].pos;
    let damage = (c::ROAR_DAMAGE * w.damage_multiplier(0)).round() as u16;
    tick(
        &mut w,
        Input {
            forward: 1.0,
            strafe: 1.0,
            jump: true,
            fire: true,
            phase_shift: true,
            ..Default::default()
        },
        38,
    );
    assert!((w.actors[0].pos - start).length() < 0.001);
    assert_eq!(w.actors[TEAM_SIZE].hp, MAX_HP - damage);
    assert_eq!(w.actors[TEAM_SIZE + 1].hp, MAX_HP - damage);
    assert_eq!(w.actors[1].hp, MAX_HP);
    advance(&mut w, 82);
    assert_eq!(w.actors[0].combat.chloe.phase, Action::Ride);
    assert_eq!(w.actors[TEAM_SIZE].hp, MAX_HP - damage);
}
#[test]
fn misses_invulnerability_allies_and_hard_control_do_not_grant_hits() {
    for case in 0..4 {
        let mut w = world();
        ride(&mut w);
        enemy(
            &mut w,
            if case == 2 { 1 } else { TEAM_SIZE },
            V3::new(0.0, 0.0, if case == 0 { -2.0 } else { 2.0 }),
        );
        if case == 1 {
            w.actors[TEAM_SIZE].shield = 3.0;
        }
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
            1,
        );
        if case == 3 {
            w.actors[0].combat.stun = 1.0;
        }
        advance(&mut w, 20);
        assert_eq!(w.actors[0].combat.chloe.hits, 0, "case {case}");
        assert!(w.actors[0].combat.chloe.remaining < c::RIDE_SECONDS);
    }
}
#[test]
fn death_swap_and_polymorph_clear_form_and_camera_lift() {
    for case in 0..3 {
        let mut w = world();
        ride(&mut w);
        match case {
            0 => w.actors[0].hp = 0,
            1 => w.actors[0].character = 1,
            _ => w.actors[0].combat.polymorph = 2.0,
        }
        c::advance_actor(&mut w.actors[0]);
        assert_eq!(w.actors[0].combat.chloe.phase, Action::Off);
        assert_eq!(w.actors[0].combat.chloe.lift(), 0.0);
    }
}
#[test]
fn nine_dolls_spawn_behind_chase_burst_on_contact_and_expire_without_blasting() {
    let mut w = world();
    w.actors[0].combat.ultimate = 100.0;
    tick(
        &mut w,
        Input {
            ultimate: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(w.projectiles.len(), 9);
    assert!(w
        .projectiles
        .iter()
        .all(|p| p.kind == ProjectileKind::Doll && p.pos.z < 0.0 && p.life == c::DOLL_SECONDS));
    enemy(&mut w, TEAM_SIZE, V3::new(3.0, 0.0, 3.0));
    let initial = w.projectiles[4].velocity.x;
    advance(&mut w, 8);
    assert!(w.projectiles[4].velocity.x > initial);
    let mut contact = false;
    for _ in 0..90 {
        advance(&mut w, 1);
        if w.projectiles
            .iter()
            .any(|p| p.kind == ProjectileKind::Confetti)
        {
            contact = true;
            break;
        }
    }
    assert!(contact);
    assert!(w.actors[TEAM_SIZE].hp < MAX_HP);
    let mut w = world();
    w.actors[0].combat.ultimate = 100.0;
    tick(
        &mut w,
        Input {
            ultimate: true,
            ..Default::default()
        },
        1,
    );
    advance(&mut w, 302);
    assert!(w.projectiles.is_empty());
}
#[test]
fn rider_head_rays_work_from_front_side_and_rear_bear_head_is_not_critical() {
    let mut w = world();
    ride(&mut w);
    let a = &mut w.actors[0];
    for yaw in [0.0, 1.3, 3.0] {
        a.yaw = yaw;
        let center = c::head_center(a);
        for dir in [
            V3::new(0.0, 0.0, 1.0),
            V3::new(1.0, 0.0, 0.0),
            V3::new(0.0, 0.0, -1.0),
        ] {
            let origin = center - dir * 5.0;
            let hit = c::ray_hit(a, origin, dir, 0.0).expect("offset rider head must be hittable");
            assert!(c::headshot(a, origin, dir, hit, 0.0));
        }
    }
    let origin = V3::new(0.0, 1.65, -5.0);
    let dir = V3::new(0.0, 0.0, 1.0);
    let hit = c::ray_hit(a, origin, dir, 0.0).unwrap();
    assert!(!c::headshot(a, origin, dir, hit, 0.0));
}
#[test]
fn moving_attack_preserves_mounted_locomotion_and_first_person_form() {
    let mut w = world();
    ride(&mut w);
    let mut clock = MotionClock::default();
    clock.observe(&w.actors[0], DT, 0, true);
    w.actors[0].combat.chloe.phase = Action::RideAttack;
    w.actors[0].combat.chloe.elapsed = 0.3;
    w.actors[0].pos.z = 0.1;
    clock.observe(&w.actors[0], DT, 1, true);
    assert_eq!(clock.pose.motion, Motion::ChloeRideAttack);
    assert_eq!(clock.pose.base().0, Motion::ChloeRideMove);
    assert!(!c::view_hands(Motion::Idle, 0.0));
    assert!(c::view_hands(Motion::ChloeMount, 1.4));
    assert!(c::spread(1.43) > 0.9);
    assert!(!c::view_hands(Motion::ChloeDismount, 1.9));
}

#[test]
fn rider_hands_stay_still_while_sebastian_walks_and_attacks() {
    for left in [false, true] {
        let rest = c::view_palm(Motion::ChloeRideIdle, 0.0, left);
        for frame in 0..=48 {
            let age = frame as f32 / 60.0;
            assert_eq!(c::view_palm(Motion::ChloeRideMove, age, left), rest);
            assert_eq!(c::view_palm(Motion::ChloeRideAttack, age, left), rest);
        }
    }
}

#[test]
fn sebastian_swings_widely_and_recovers_in_both_forms() {
    for motion in [Motion::ChloeAttack, Motion::ChloeRideAttack] {
        let rest = c::view_arm(motion, 0.0, false);
        let hit = c::view_arm(motion, c::ATTACK_RELEASE, false);
        assert!(hit.tip[0] - rest.tip[0] > 0.40);
        assert!(hit.tip[1] - rest.tip[1] > 0.35);
        assert_eq!(c::view_arm(motion, c::ATTACK_SECONDS, false).tip, rest.tip);
        let mut previous = rest.tip;
        for frame in 1..=48 {
            let arm = c::view_arm(motion, frame as f32 / 60.0, false);
            let step = arm
                .tip
                .iter()
                .zip(previous)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f32>()
                .sqrt();
            assert!(step < 0.20, "view arm snapped: {step}");
            assert!(arm.corner[0] < arm.tip[0] && arm.corner[1] < arm.tip[1]);
            previous = arm.tip;
        }
    }
    assert!(
        c::view_arm(Motion::ChloeRideIdle, 0.0, false).scale
            > c::view_arm(Motion::Idle, 0.0, false).scale * 1.3
    );
}

#[test]
fn sebastian_size_blends_across_mount_and_dismount_seams() {
    for left in [false, true] {
        let off = c::view_arm(Motion::Idle, 0.0, left);
        let on = c::view_arm(Motion::ChloeRideIdle, 0.0, left);
        assert_eq!(c::view_arm(Motion::ChloeMount, 0.0, left).tip, off.tip);
        assert_eq!(
            c::view_arm(Motion::ChloeMount, c::MOUNT_SECONDS, left).tip,
            on.tip
        );
        assert_eq!(c::view_arm(Motion::ChloeDismount, 0.0, left).tip, on.tip);
        assert_eq!(
            c::view_arm(Motion::ChloeDismount, c::DISMOUNT_SECONDS, left).tip,
            off.tip
        );
    }
}
#[test]
fn roar_sound_uses_release_edge_without_snapshot_repeats_or_late_join_replay() {
    let mut w = world();
    ride(&mut w);
    let mut tracker = Tracker::default();
    tracker.update(&w, 0, true);
    w.actors[0].combat.chloe.phase = Action::Roar;
    w.actors[0].combat.chloe.elapsed = 0.7;
    w.tick += 1;
    assert_eq!(
        tracker
            .update(&w, 0, true)
            .iter()
            .filter(|e| e.cue == Cue::SebastianRoar)
            .count(),
        1
    );
    w.tick += 1;
    assert!(tracker
        .update(&w, 0, true)
        .iter()
        .all(|e| e.cue != Cue::SebastianRoar));
    assert!(Tracker::default().update(&w, 0, true).is_empty());
}

#[test]
fn actual_bullets_crit_the_rider_but_not_sebastian() {
    for (head, damage) in [(true, 30), (false, 20)] {
        let mut w = world();
        w.actors[0].character = 14;
        enemy(&mut w, TEAM_SIZE, V3::new(0.0, 0.0, 4.0));
        w.actors[TEAM_SIZE].combat.chloe = State {
            phase: Action::Ride,
            remaining: 12.0,
            ..Default::default()
        };
        let center = if head {
            c::head_center(&w.actors[TEAM_SIZE])
        } else {
            V3::new(0.0, 1.65, 4.0)
        };
        let mut p = Projectile::visual(
            100,
            0,
            ProjectileKind::Bolt,
            center - V3::new(0.0, 0.0, 2.0),
        );
        p.character = 14;
        p.class = AttackClass::Basic;
        p.damage = 20.0;
        p.velocity = V3::new(0.0, 0.0, 30.0);
        p.life = 1.0;
        w.projectiles.push(p);
        advance(&mut w, 5);
        assert_eq!(w.actors[TEAM_SIZE].hp, MAX_HP - damage, "head={head}");
    }
}

#[test]
fn cover_blocks_roar_and_doll_contact_never_blasts_through_a_wall() {
    let mut w = world();
    ride(&mut w);
    enemy(&mut w, TEAM_SIZE, V3::new(0.0, 0.0, 3.0));
    w.map.push(Block {
        center: V3::new(0.0, 2.0, 1.5),
        size: V3::new(5.0, 4.0, 0.2),
        color: [0; 3],
        phaseable: false,
    });
    w.actors[0].combat.chloe.phase = Action::Roar;
    w.actors[0].combat.chloe.elapsed = 0.60;
    advance(&mut w, 4);
    assert_eq!(w.actors[TEAM_SIZE].hp, MAX_HP);
    let mut p = Projectile::visual(101, 0, ProjectileKind::Doll, V3::new(0.0, 0.45, 0.0));
    p.velocity = V3::new(0.0, 0.0, 10.0);
    p.life = 5.0;
    p.damage = 14.0;
    p.character = 6;
    p.class = AttackClass::Ultimate;
    w.projectiles.push(p);
    advance(&mut w, 20);
    assert_eq!(w.actors[TEAM_SIZE].hp, MAX_HP);
    assert!(w.projectiles.is_empty());
}
