use trickcal_shooting_balddagu::{
    combat::ProjectileKind,
    game::*,
    motion::{Motion, MotionClock},
    vivi_skill::{self as v, Phase as Action},
};
fn world() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.human = true;
        a.active = true;
        a.character = 5;
        a.pos = V3::new(60.0 + i as f32, 0.0, -60.0);
        a.shield = 0.0;
        a.barrier = 0.0;
        a.combat = Default::default();
        a.hp = MAX_HP;
    }
    w.actors[0].character = 0;
    w.actors[0].ammo = 0;
    w.actors[0].pos = V3::default();
    w.actors[6].pos = V3::new(0.0, 0.0, 2.5);
    w
}
fn step(w: &mut World, input: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = input;
    w.tick(inputs);
}
fn advance(w: &mut World, n: usize) {
    for _ in 0..n {
        step(w, Input::default());
    }
}
fn fire() -> Input {
    Input {
        fire: true,
        ..Default::default()
    }
}
fn cast(w: &mut World, ult: bool) {
    w.actors[0].combat.ultimate = 100.0;
    step(
        w,
        Input {
            ability: !ult,
            ultimate: ult,
            ..Default::default()
        },
    );
}
#[test]
fn tap_only_sweeps_left_to_right_and_a_later_tap_restarts_left() {
    let mut w = world();
    step(&mut w, fire());
    assert_eq!(w.actors[0].combat.vivi.phase, Action::Left);
    assert_eq!(w.actors[6].hp, MAX_HP);
    advance(&mut w, 23);
    assert_eq!(w.actors[6].hp, MAX_HP);
    advance(&mut w, 1);
    assert!(w.actors[6].hp < MAX_HP);
    let hp = w.actors[6].hp;
    advance(&mut w, 90);
    assert_eq!(w.actors[6].hp, hp);
    assert_eq!(w.actors[0].combat.vivi.phase, Action::Off);
    step(&mut w, fire());
    assert_eq!(w.actors[0].combat.vivi.phase, Action::Left);
    assert_eq!(w.actors[0].combat.vivi_combo, 1);
}
#[test]
fn holding_alternates_both_source_halves_without_a_three_hit_finisher() {
    let mut w = world();
    w.actors[6].pos = V3::new(0.0, 0.0, 50.0);
    let mut phases = Vec::new();
    let mut previous = Action::Off;
    for _ in 0..300 {
        step(&mut w, fire());
        let p = w.actors[0].combat.vivi.phase;
        if p != previous {
            phases.push(p);
            previous = p;
        }
    }
    assert_eq!(
        &phases[..5],
        &[
            Action::Left,
            Action::Right,
            Action::Left,
            Action::Right,
            Action::Left
        ]
    );
    advance(&mut w, 80);
    assert_eq!(w.actors[0].combat.vivi.phase, Action::Off);
    step(&mut w, fire());
    assert_eq!(w.actors[0].combat.vivi.phase, Action::Left);
}
#[test]
fn e_bubble_is_forty_percent_base_health_at_the_source_release_and_never_lifts_the_camera() {
    let mut w = world();
    w.actors[0].combat.fashion = 8.0;
    w.actors[0].hp = MAX_HP * 2;
    let eye = w.actors[0].eye();
    cast(&mut w, false);
    advance(&mut w, 39);
    assert_eq!(w.actors[0].combat.guard, 0.0);
    advance(&mut w, 1);
    assert_eq!(w.actors[0].combat.guard, v::SHIELD_HEALTH);
    assert_eq!(v::SHIELD_HEALTH, 48.0);
    assert!(w.actors[0].combat.mercury);
    assert_eq!(w.actors[0].eye(), eye);
    assert!(v::skill_lift(0.78) > 0.35);
    assert_eq!(v::skill_lift(v::SKILL_SECONDS), 0.0);
    assert!(v::raised_hand(0.28)[0] > 0.0);
    assert!(v::raised_hand(0.78)[1] > 0.3);
}
#[test]
fn q_lifts_the_actor_and_camera_and_returns_to_the_floor() {
    let mut w = world();
    let eye = w.actors[0].eye();
    cast(&mut w, true);
    advance(&mut w, 60);
    assert!(w.actors[0].pos.y > 1.7);
    assert!(w.actors[0].eye().y > eye.y + 1.7);
    advance(&mut w, 190);
    assert_eq!(w.actors[0].combat.vivi.phase, Action::Off);
    assert!(w.actors[0].pos.y.abs() < 0.001);
    assert!(w.actors[0].eye().y - eye.y < 0.001);
}
#[test]
fn landing_has_one_large_hit_and_late_entrants_only_receive_pool_control() {
    let mut w = world();
    w.actors[6].pos = V3::new(1.0, 0.0, 12.0);
    w.actors[7].pos = V3::new(0.0, 0.0, 12.0);
    w.actors[8].pos = V3::new(8.0, 0.0, 12.0);
    w.actors[1].pos = V3::new(0.0, 0.0, 12.0);
    for i in [6, 7] {
        w.actors[i].hp = MAX_HP * 2;
        w.actors[i].combat.fashion = 8.0;
    }
    cast(&mut w, true);
    advance(&mut w, 70);
    assert_eq!(w.projectiles[0].kind, ProjectileKind::MercuryField);
    assert_eq!(w.actors[6].hp, MAX_HP * 2);
    advance(&mut w, 54);
    let hp = w.actors[6].hp;
    assert!(hp > 0 && MAX_HP * 2 - hp >= 80);
    assert_eq!(
        w.actors[7].hp, hp,
        "direct hit and circle do not double damage"
    );
    assert_eq!(w.actors[1].hp, MAX_HP);
    assert!(w.actors[6].combat.weaken > 0.0);
    assert!(w.actors[6].combat.taunt > 0.0);
    w.actors[8].pos = V3::new(-2.0, 0.0, 12.0);
    advance(&mut w, 20);
    assert_eq!(w.actors[8].hp, MAX_HP);
    assert!(w.actors[8].combat.weaken > 0.0);
    assert!(w.actors[8].combat.taunt > 0.0);
    assert_eq!(w.actors[6].hp, hp);
    w.actors[8].pos = V3::new(9.0, 0.0, 12.0);
    advance(&mut w, 3);
    assert_eq!(w.actors[8].combat.weaken, 0.0);
    assert_eq!(w.actors[8].combat.taunt, 0.0);
    advance(&mut w, 310);
    assert!(w.projectiles.is_empty());
    assert_eq!(w.actors[6].combat.weaken, 0.0);
}
#[test]
fn interruption_cancels_pending_damage_shield_and_lance() {
    for action in 0..3 {
        let mut w = world();
        match action {
            0 => step(&mut w, fire()),
            1 => cast(&mut w, false),
            _ => cast(&mut w, true),
        };
        advance(&mut w, 5);
        w.actors[0].combat.stun = 1.0;
        advance(&mut w, 80);
        assert_eq!(w.actors[0].combat.vivi.phase, Action::Off);
        assert_eq!(w.actors[0].combat.guard, 0.0);
        assert!(w.projectiles.is_empty());
        assert_eq!(w.actors[6].hp, MAX_HP);
    }
}
#[test]
fn observer_clock_uses_authoritative_half_and_skill_age() {
    let mut w = world();
    let mut clock = MotionClock::default();
    for (phase, motion) in [
        (Action::Left, Motion::ViviLeft),
        (Action::Right, Motion::ViviRight),
        (Action::Skill, Motion::ViviSkill),
        (Action::Ultimate, Motion::ViviUltimate),
    ] {
        w.actors[0].combat.vivi = v::State {
            phase,
            elapsed: 0.55,
        };
        let p = clock.observe(&w.actors[0], DT, 100, true);
        assert_eq!(p.motion, motion);
        assert_eq!(p.age, 0.55);
    }
}
#[test]
fn pool_respects_circle_height_and_cover() {
    let w = world();
    let mut p = trickcal_shooting_balddagu::combat::Projectile::visual(
        1,
        0,
        ProjectileKind::MercuryField,
        V3::default(),
    );
    p.age = v::FALL_SECONDS;
    let mut a = w.actors[6].clone();
    a.pos = V3::new(3.9, 0.0, 0.0);
    assert!(v::in_pool(&[], &p, &a));
    a.pos = V3::new(3.0, 0.0, 3.0);
    assert!(!v::in_pool(&[], &p, &a));
    a.pos = V3::new(0.0, 3.0, 0.0);
    assert!(!v::in_pool(&[], &p, &a));
    a.pos = V3::new(3.0, 0.0, 0.0);
    let wall = Block {
        center: V3::new(1.5, 1.5, 0.0),
        size: V3::new(0.3, 3.0, 3.0),
        color: [0; 3],
        phaseable: false,
    };
    assert!(!v::in_pool(&[wall], &p, &a));
}
