use trickcal_shooting_balddagu::{
    aya_skill::{self as aya, Phase as AyaPhase},
    combat::ProjectileKind,
    game::*,
    prediction::ClientMotion,
};
fn world() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.active = i == 0 || i == 6;
        a.human = true;
        a.shield = 0.0;
        a.barrier = 0.0;
        a.pos = V3::new(30.0 + i as f32 * 2.0, 0.0, 0.0);
        a.combat = Default::default();
    }
    w.actors[0].character = 4;
    w.actors[0].ammo = 0;
    w.actors[0].pos = V3::default();
    w.actors[6].character = 14;
    w.actors[6].ammo = 6;
    w.actors[6].pos = V3::new(0.0, 0.0, 4.0);
    w
}
fn tick(w: &mut World, input: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = input;
    w.tick(inputs);
}
#[test]
fn butterflies_wait_for_source_release_then_each_outbound_and_return_hit_refunds() {
    let mut w = world();
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].combat.aya.phase, AyaPhase::Skill);
    for _ in 0..77 {
        tick(&mut w, Input::default());
        assert!(w.projectiles.is_empty());
    }
    tick(&mut w, Input::default());
    assert_eq!(
        w.projectiles
            .iter()
            .filter(|p| p.kind == ProjectileKind::Butterfly)
            .count(),
        aya::BUTTERFLIES
    );
    let before = w.actors[0].ability_cooldown;
    for _ in 0..25 {
        tick(&mut w, Input::default());
    }
    assert!(w.actors[6].combat.weaken > 3.0);
    assert_eq!(w.actors[6].combat.skill_weaken, 0.0);
    assert!((before - w.actors[0].ability_cooldown - 25.0 * DT - 5.0).abs() < 0.03);
    for _ in 0..140 {
        tick(&mut w, Input::default());
    }
    assert_eq!(
        w.actors[0].ability_cooldown, 0.0,
        "return hits refund independently"
    );
}
#[test]
fn ultimate_freezes_only_visible_nearby_enemies_at_the_arm_spread() {
    let mut w = world();
    w.actors[0].combat.ultimate = 100.0;
    w.actors[1].active = true;
    w.actors[1].pos = V3::new(-2.0, 0.0, 4.0);
    w.actors[7].active = true;
    w.actors[7].pos = V3::new(5.0, 0.0, 4.0);
    w.actors[8].active = true;
    w.actors[8].pos = V3::new(0.0, 0.0, 13.0);
    w.map.push(Block {
        center: V3::new(2.5, 1.5, 2.0),
        size: V3::new(1.0, 3.0, 3.0),
        color: [0; 3],
        phaseable: false,
    });
    tick(
        &mut w,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    assert!(w
        .projectiles
        .iter()
        .any(|p| p.kind == ProjectileKind::SnowGarden));
    for _ in 0..113 {
        tick(&mut w, Input::default());
        assert_eq!(w.actors[6].combat.frozen, 0.0);
    }
    tick(&mut w, Input::default());
    assert_eq!(w.actors[6].combat.frozen, aya::FREEZE_SECONDS);
    for i in [0, 1, 7, 8] {
        assert_eq!(w.actors[i].combat.frozen, 0.0, "slot {i}");
    }
    assert_eq!(
        w.actors[6].combat.frost, 0.0,
        "ice prison is not damage over time"
    );
}
#[test]
fn frost_blocks_view_posture_all_gameplay_and_prediction_then_releases() {
    let mut w = world();
    let a = &mut w.actors[6];
    a.combat.frozen = aya::FREEZE_SECONDS;
    a.combat.ultimate = 100.0;
    a.yaw = 0.4;
    a.pitch = 0.2;
    a.vy = 5.0;
    let before = a.clone();
    let input = Input {
        yaw: 2.0,
        pitch: -1.0,
        forward: 1.0,
        strafe: 1.0,
        jump: true,
        crouch: true,
        fire: true,
        alt_fire: true,
        reload: true,
        ability: true,
        ultimate: true,
        phase_shift: true,
        dance: true,
        ..Default::default()
    };
    let mut prediction = ClientMotion::default();
    prediction.observe(&w, 6, 0);
    for seq in 1..60 {
        prediction.advance(&w, seq, input);
    }
    let predicted = prediction.actor(&w, 6, 6);
    assert_eq!(predicted.pos, before.pos);
    assert_eq!(predicted.yaw, before.yaw);
    assert_eq!(predicted.pitch, before.pitch);
    assert!(!predicted.crouching);
    for _ in 0..120 {
        let mut inputs = [Input::default(); PLAYERS];
        inputs[6] = input;
        w.tick(inputs);
    }
    let a = &w.actors[6];
    assert_eq!(a.pos, before.pos);
    assert_eq!(a.yaw, before.yaw);
    assert_eq!(a.pitch, before.pitch);
    assert!(!a.crouching && !a.dancing);
    assert_eq!(a.shot, before.shot);
    assert_eq!(a.reload, 0.0);
    assert_eq!(a.combat.ultimate, 100.0);
    assert!(w.projectiles.is_empty());
    for _ in 0..62 {
        let mut inputs = [Input::default(); PLAYERS];
        inputs[6] = Input {
            yaw: 1.0,
            forward: 1.0,
            ..Default::default()
        };
        w.tick(inputs);
    }
    assert_eq!(w.actors[6].combat.frozen, 0.0);
    assert_ne!(w.actors[6].pos, before.pos);
    assert_eq!(w.actors[6].yaw, 1.0);
}
#[test]
fn interrupted_cast_does_not_fire_or_freeze_and_recast_resets_clock() {
    for ultimate in [false, true] {
        let mut w = world();
        w.actors[0].combat.ultimate = 100.0;
        tick(
            &mut w,
            Input {
                ability: !ultimate,
                ultimate,
                ..Default::default()
            },
        );
        w.actors[0].combat.stun = 0.1;
        tick(&mut w, Input::default());
        assert_eq!(w.actors[0].combat.aya.phase, AyaPhase::Off);
        for _ in 0..200 {
            tick(&mut w, Input::default());
        }
        assert!(w.projectiles.is_empty());
        assert_eq!(w.actors[6].hp, MAX_HP);
        assert_eq!(w.actors[6].combat.frozen, 0.0);
    }
}

#[test]
fn snow_garden_cancels_on_release_boundary_but_survives_after_a_real_freeze() {
    for released in [false, true] {
        let mut w = world();
        w.actors[0].combat.ultimate = 100.0;
        tick(
            &mut w,
            Input {
                ultimate: true,
                ..Default::default()
            },
        );
        for _ in 0..if released { 114 } else { 113 } {
            tick(&mut w, Input::default());
        }
        w.actors[0].combat.stun = 1.0;
        tick(&mut w, Input::default());
        assert_eq!(
            w.projectiles
                .iter()
                .any(|p| p.kind == ProjectileKind::SnowGarden),
            released
        );
        assert_eq!(w.actors[6].combat.frozen > 0.0, released);
    }
}
