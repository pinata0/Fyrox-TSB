use trickcal_shooting_balddagu::movement::WALK_SPEED;
use trickcal_shooting_balddagu::{
    combat::ProjectileKind,
    game::*,
    motion::{Motion, MotionClock},
    ui_skill::{self as u, Phase as Action},
};
fn world() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.active = true;
        a.human = true;
        a.shield = 0.0;
        a.barrier = 0.0;
        a.combat = Default::default();
        a.character = 4;
        a.ammo = 0;
        a.pos = V3::new(60.0 + i as f32, 0.0, 60.0);
    }
    w.actors[0].character = 10;
    w.actors[0].ammo = 20;
    w.actors[0].pos = V3::default();
    w.actors[1].pos = V3::new(-2.0, 0.0, 3.0);
    w.actors[1].hp = 50;
    w.actors[6].pos = V3::new(2.0, 0.0, 3.0);
    w
}
fn tick(w: &mut World, input: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = input;
    w.tick(inputs);
}
fn advance(w: &mut World, n: usize) {
    for _ in 0..n {
        tick(w, Input::default());
    }
}
fn cast(w: &mut World, ultimate: bool) {
    w.actors[0].combat.ultimate = 100.0;
    tick(
        w,
        Input {
            ability: !ultimate,
            ultimate,
            ..Default::default()
        },
    );
}
#[test]
fn rain_releases_on_frame_22_and_ticks_eight_times_in_a_fixed_circle() {
    let mut w = world();
    w.actors[0].hp = 60;
    w.actors[1].ability_cooldown = 10.0;
    cast(&mut w, false);
    advance(&mut w, 43);
    assert!(w.projectiles.is_empty());
    advance(&mut w, 1);
    assert_eq!(w.projectiles[0].kind, ProjectileKind::RainField);
    assert_eq!(w.actors[1].hp, 50);
    let center = w.projectiles[0].pos;
    let damage = (u::RAIN_DAMAGE * w.damage_multiplier(team(0))).round() as u16;
    // Behind the caster is still in the all-direction rain zone.
    w.actors[7].pos = V3::new(-2.0, 0.0, -3.0);
    advance(&mut w, 30);
    assert_eq!(w.actors[1].hp, 54);
    assert_eq!(w.actors[6].hp, MAX_HP - damage);
    assert_eq!(w.actors[7].hp, MAX_HP - damage);
    assert!(w.actors[6].combat.rain_slow > 0.0);
    assert_eq!(w.actors[6].combat.slow, 0.0);
    assert!(w.actors[1].ability_cooldown > 8.0, "rain does not refund E");
    w.actors[0].pos = V3::new(20.0, 0.0, 0.0);
    assert_eq!(w.projectiles[0].pos, center);
    advance(&mut w, 210);
    assert_eq!(w.actors[1].hp, 82);
    assert_eq!(w.actors[6].hp, MAX_HP - 8 * damage);
    assert!(w.projectiles.is_empty());
    assert_eq!(w.actors[6].combat.rain_slow, 0.0);
}
#[test]
fn rain_applies_exactly_fifteen_percent_slow_and_stops_outside_or_behind_cover() {
    let mut w = world();
    cast(&mut w, false);
    advance(&mut w, 45);
    let before = w.actors[6].pos;
    let mut inputs = [Input::default(); PLAYERS];
    inputs[6].forward = 1.0;
    w.tick(inputs);
    assert!(((w.actors[6].pos - before).length() - WALK_SPEED * DT * 0.85).abs() < 0.0001);
    w.actors[6].pos = V3::new(8.0, 0.0, 0.0);
    w.tick(inputs);
    assert_eq!(w.actors[6].combat.rain_slow, 0.0);
    w.actors[6].pos = V3::new(0.0, 0.0, 3.0);
    w.map.push(Block {
        center: V3::new(0.0, 1.5, 1.5),
        size: V3::new(3.0, 3.0, 0.4),
        color: [0; 3],
        phaseable: true,
    });
    let hp = w.actors[6].hp;
    advance(&mut w, 31);
    assert_eq!(w.actors[6].hp, hp);
    assert_eq!(w.actors[6].combat.rain_slow, 0.0);
}
#[test]
fn wish_morphs_every_visible_enemy_then_resets_every_visible_ally_only_once() {
    let mut w = world();
    w.actors[2].pos = V3::new(8.0, 0.0, 25.0);
    w.actors[3].pos = V3::new(0.0, 0.0, -5.0);
    w.actors[7].pos = V3::new(-8.0, 0.0, 25.0);
    w.actors[8].pos = V3::new(0.0, 0.0, -5.0);
    for i in [1, 2, 3] {
        w.actors[i].ability_cooldown = 10.0;
    }
    cast(&mut w, true);
    advance(&mut w, 57);
    assert_eq!(w.actors[6].combat.polymorph, 0.0);
    advance(&mut w, 1);
    for i in [6, 7] {
        assert!(w.actors[i].combat.polymorph > 2.9);
        assert_eq!(w.actors[i].combat.stun, 0.0);
    }
    assert_eq!(w.actors[8].combat.polymorph, 0.0);
    for i in [1, 2] {
        assert_eq!(w.actors[i].ability_cooldown, 0.0);
    }
    assert!(w.actors[3].ability_cooldown > 8.0);
    assert_eq!(w.actors[1].hp, 50);
    w.actors[1].ability_cooldown = 9.0;
    advance(&mut w, 60);
    assert!(w.actors[1].ability_cooldown > 7.9);
}
#[test]
fn wish_respects_cover_pitch_immunity_and_cancels_pending_actions() {
    let mut w = world();
    w.actors[6].pos = V3::new(0.0, 0.0, 5.0);
    w.map.push(Block {
        center: V3::new(0.0, 1.5, 2.5),
        size: V3::new(2.0, 3.0, 0.4),
        color: [0; 3],
        phaseable: true,
    });
    cast(&mut w, true);
    advance(&mut w, 58);
    assert_eq!(w.actors[6].combat.polymorph, 0.0);
    let mut w = world();
    w.actors[6].shield = 2.0;
    cast(&mut w, true);
    advance(&mut w, 58);
    assert_eq!(w.actors[6].combat.polymorph, 0.0);
    let mut w = world();
    w.actors[6].pos = V3::new(2.0, 30.0, 3.0);
    cast(&mut w, true);
    advance(&mut w, 58);
    assert_eq!(w.actors[6].combat.polymorph, 0.0);
    let mut w = world();
    cast(&mut w, true);
    advance(&mut w, 57);
    w.actors[6].character = 13;
    w.actors[6]
        .combat
        .hilde
        .start(trickcal_shooting_balddagu::hilde_skill::Phase::Launcher);
    w.actors[6].combat.hilde.elapsed = 3.0;
    w.actors[6].reload = 1.5;
    advance(&mut w, 1);
    assert!(w.actors[6].combat.polymorph > 0.0);
    assert!(!w.actors[6].combat.hilde.active());
    assert_eq!(w.actors[6].reload, 0.0);
}
#[test]
fn frog_can_walk_and_look_but_cannot_jump_crouch_attack_reload_cast_or_shift() {
    let mut w = world();
    w.actors[0].combat.polymorph = 3.0;
    w.actors[0].crouching = true;
    w.actors[0].combat.ultimate = 100.0;
    w.actors[0].ammo = 10;
    let before = w.actors[0].pos;
    tick(
        &mut w,
        Input {
            forward: 1.0,
            yaw: 0.5,
            pitch: 0.2,
            fire: true,
            alt_fire: true,
            reload: true,
            ability: true,
            ultimate: true,
            phase_shift: true,
            jump: true,
            crouch: true,
            dance: true,
            ..Default::default()
        },
    );
    let a = &w.actors[0];
    assert!(((a.pos - before).length() - WALK_SPEED * DT).abs() < 0.0001);
    assert_eq!(a.pos.y, 0.0);
    assert_eq!(a.yaw, 0.5);
    assert_eq!(a.pitch, 0.2);
    assert_eq!(a.shot, 0);
    assert_eq!(a.ammo, 10);
    assert_eq!(a.reload, 0.0);
    assert_eq!(a.ability_cooldown, 0.0);
    assert_eq!(a.combat.ultimate, 100.0);
    assert!(!a.combat.ui.active());
    assert!(!a.race_skill.active());
    assert!(!a.crouching);
    assert!(!a.dancing);
    advance(&mut w, 180);
    assert_eq!(w.actors[0].combat.polymorph, 0.0);
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].combat.ui.phase, Action::Skill);
}
#[test]
fn interrupted_cast_does_not_emit_and_source_tracks_preserve_spin_and_pulse() {
    let mut w = world();
    cast(&mut w, false);
    w.actors[0].combat.polymorph = 1.0;
    advance(&mut w, 60);
    assert!(w.projectiles.is_empty());
    for phase in [Action::Skill, Action::Ultimate] {
        w.actors[0].combat.ui = u::State {
            phase,
            elapsed: 0.6,
        };
        let pose = MotionClock::default().observe(&w.actors[0], 0.0, 1, true);
        assert_eq!(pose.motion, phase.motion().unwrap());
        assert_eq!(pose.age, 0.6);
    }
    let samples: Vec<_> = (0..66)
        .map(|i| u::clover(Motion::UiSkill, i as f32 / 30.0)[0])
        .collect();
    assert!(samples.iter().copied().fold(f32::INFINITY, f32::min) < -12.0);
    let sizes: Vec<_> = (0..128)
        .map(|i| u::clover(Motion::UiUltimate, i as f32 / 30.0)[1])
        .collect();
    assert!(sizes.iter().any(|&x| x < 0.8));
    assert!(sizes.iter().any(|&x| x > 1.8));
}

#[test]
fn visibility_tracks_viewport_width_and_upward_aim() {
    let mut w = world();
    w.actors[6].pos = V3::new(5.0, 0.0, 5.0);
    w.actors[0].view_aspect = 1.0;
    assert!(!u::in_view(&w.map, &w.actors[0], &w.actors[6]));
    w.actors[0].view_aspect = 16.0 / 9.0;
    assert!(u::in_view(&w.map, &w.actors[0], &w.actors[6]));
    w.actors[6].pos = V3::new(0.0, 10.0, 5.0);
    assert!(!u::in_view(&w.map, &w.actors[0], &w.actors[6]));
    w.actors[0].pitch = 1.1;
    assert!(u::in_view(&w.map, &w.actors[0], &w.actors[6]));
}
