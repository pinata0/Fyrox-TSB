use trickcal_shooting_balddagu::movement::WALK_SPEED;
use trickcal_shooting_balddagu::{
    game::*,
    motion::{Motion, MotionClock},
    naia_skill::*,
};
fn fixture() -> World {
    let mut w = World::default();
    w.start();
    w.phase = trickcal_shooting_balddagu::game::Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.human = true;
        a.active = true;
        a.shield = 0.0;
        a.pos = V3::new(40.0 + i as f32 * 2.0, 0.0, 40.0);
    }
    w.actors[0].character = 1;
    w.actors[0].pos = V3::default();
    w.actors[0].ammo = 24;
    w.actors[1].pos = V3::new(0.0, 0.0, 4.0);
    w.actors[1].hp = 1;
    w.actors[1].character = 1;
    w
}
fn tick(w: &mut World, input: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = input;
    w.tick(inputs);
}
fn start(w: &mut World) {
    tick(
        w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
}
fn windup(w: &mut World) {
    for _ in 0..88 {
        tick(w, Input::default());
    }
}
#[test]
fn held_stream_roots_and_release_moves_immediately_with_bonus() {
    let mut w = fixture();
    start(&mut w);
    assert_eq!(
        w.actors[0].combat.naia.phase,
        trickcal_shooting_balddagu::naia_skill::Phase::Start
    );
    windup(&mut w);
    let pos = w.actors[0].pos;
    let ammo = w.actors[0].ammo;
    let shot = w.actors[0].shot;
    for _ in 0..60 {
        tick(
            &mut w,
            Input {
                fire: true,
                forward: 1.0,
                strafe: 1.0,
                jump: true,
                phase_shift: true,
                reload: true,
                ..Default::default()
            },
        );
    }
    assert_eq!(w.actors[0].pos, pos);
    assert_eq!(w.actors[0].ammo, ammo);
    assert_eq!(w.actors[0].shot, shot);
    assert_eq!(w.actors[0].reload, 0.0);
    assert!(
        (55..=67).contains(&w.actors[1].hp),
        "rapid healing: {}",
        w.actors[1].hp
    );
    let hp = w.actors[1].hp;
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
    );
    assert!((w.actors[0].pos.z - pos.z - WALK_SPEED * SPEED_MULTIPLIER * DT).abs() < 0.0001);
    for _ in 0..12 {
        tick(&mut w, Input::default());
    }
    assert_eq!(w.actors[1].hp, hp, "release stops healing");
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
    );
    assert_eq!(
        w.actors[0].combat.naia.phase,
        trickcal_shooting_balddagu::naia_skill::Phase::Fire
    );
}
#[test]
fn all_four_clips_follow_authority_and_expire_while_mouse_is_held() {
    let mut w = fixture();
    let mut clock = MotionClock::default();
    start(&mut w);
    assert_eq!(
        clock.observe(&w.actors[0], DT, w.tick, true).motion,
        Motion::NaiaStart
    );
    windup(&mut w);
    assert_eq!(
        clock.observe(&w.actors[0], DT, w.tick, true).motion,
        Motion::NaiaMove
    );
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
    );
    assert_eq!(
        clock.observe(&w.actors[0], DT, w.tick, true).motion,
        Motion::NaiaFire
    );
    for _ in 0..179 {
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
    }
    let p = clock.observe(&w.actors[0], DT, w.tick, true);
    assert_eq!(p.motion, Motion::NaiaEnd);
    let hp = w.actors[1].hp;
    let ammo = w.actors[0].ammo;
    for _ in 0..50 {
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
    }
    assert_eq!(w.actors[1].hp, hp);
    assert_eq!(w.actors[0].ammo, ammo);
    for _ in 0..5 {
        tick(&mut w, Input::default());
    }
    assert!(!w.actors[0].combat.naia.active());
    assert_eq!(
        clock.observe(&w.actors[0], DT, w.tick, true).motion,
        Motion::Idle
    );
    assert!(w.actors[0].ability_cooldown > 0.0);
}
#[test]
fn water_respects_aim_walls_team_and_stun_interrupts() {
    let mut w = fixture();
    start(&mut w);
    windup(&mut w);
    for _ in 0..15 {
        tick(
            &mut w,
            Input {
                fire: true,
                yaw: 1.0,
                ..Default::default()
            },
        );
    }
    assert_eq!(w.actors[1].hp, 1, "aim required");
    w.map.push(Block {
        center: V3::new(0.0, 1.5, 2.0),
        size: V3::new(5.0, 4.0, 0.25),
        color: [0; 3],
        phaseable: true,
    });
    for _ in 0..15 {
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
    }
    assert_eq!(w.actors[1].hp, 1);
    assert!(w.actors[0].shot_end.z <= 1.875);
    w.map.clear();
    w.actors[1].pos.x = 10.0;
    w.actors[6].pos = V3::new(0.0, 0.0, 4.0);
    w.actors[6].hp = 20;
    for _ in 0..15 {
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
    }
    assert_eq!(w.actors[6].hp, 20, "enemies are neither healed nor damaged");
    w.actors[0].combat.stun = 2.0;
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
    );
    assert!(!w.actors[0].combat.naia.active());
}
#[test]
fn source_clips_and_appended_ranges_preserve_the_original_timing() {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../assets/characters/skill/naia.json")).unwrap();
    for (i, m) in [
        Motion::NaiaStart,
        Motion::NaiaMove,
        Motion::NaiaFire,
        Motion::NaiaEnd,
        Motion::NaiaUltimate,
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            (data["clips"][i]["duration"].as_f64().unwrap() as f32 - m.seconds(1)).abs() < 0.00001
        );
        assert!(m.frames().0 > 555);
    }
}

fn ultimate(w: &mut World) {
    w.actors[0].combat.ultimate = 100.0;
    tick(
        w,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
}
fn advance(w: &mut World, ticks: usize) {
    for _ in 0..ticks {
        tick(w, Input::default());
    }
}

#[test]
fn ultimate_releases_from_behind_on_source_cue_and_locks_actions() {
    let mut w = fixture();
    ultimate(&mut w);
    assert_eq!(
        w.actors[0].combat.naia.phase,
        trickcal_shooting_balddagu::naia_skill::Phase::Ultimate
    );
    let mut clock = MotionClock::default();
    assert_eq!(
        clock.observe(&w.actors[0], DT, w.tick, true).motion,
        Motion::NaiaUltimate
    );
    advance(&mut w, 89);
    assert!(w.projectiles.is_empty());
    let pos = w.actors[0].pos;
    let ammo = w.actors[0].ammo;
    tick(
        &mut w,
        Input {
            forward: 1.0,
            jump: true,
            fire: true,
            ability: true,
            reload: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].pos, pos);
    assert_eq!(w.actors[0].ammo, ammo);
    assert_eq!(w.projectiles.len(), 1);
    assert!((w.projectiles[0].origin.z + WAVE_BEHIND).abs() < 0.001);
    advance(&mut w, 40);
    assert_eq!(
        w.actors[0].combat.guard, WAVE_GUARD,
        "caster is crossed by the wave"
    );
    advance(&mut w, 120);
    assert!(!w.actors[0].combat.naia.active());
}

#[test]
fn broad_wave_grants_temporary_guard_and_hits_enemies_only_once() {
    let mut w = fixture();
    w.actors[1].pos = V3::new(4.5, 0.0, 3.0);
    w.actors[6].pos = V3::new(-4.5, 0.0, 3.0);
    w.actors[6].barrier = 0.0;
    w.actors[6].character = 14;
    let hp = w.actors[6].hp;
    ultimate(&mut w);
    advance(&mut w, 160);
    assert_eq!(w.actors[1].hp, 1, "Q gives guard rather than healing");
    assert_eq!(w.actors[1].combat.guard, WAVE_GUARD);
    assert!(w.actors[6].hp < hp);
    assert!((w.actors[6].pos.z - 5.0).abs() < 0.01);
    let damaged = w.actors[6].hp;
    advance(&mut w, 80);
    assert_eq!(w.actors[6].hp, damaged);
    advance(&mut w, 300);
    assert_eq!(w.actors[1].combat.guard, 0.0);
}

#[test]
fn wave_respects_cover_width_height_and_burrow() {
    let mut w = fixture();
    w.actors[1].pos = V3::new(3.0, 0.0, 4.0);
    w.actors[2].pos = V3::new(6.0, 0.0, 4.0);
    w.actors[3].pos = V3::new(0.0, 8.0, 4.0);
    w.actors[3].vy = 0.0;
    w.map.push(Block {
        center: V3::new(0.0, 7.75, 4.0),
        size: V3::new(2.0, 0.5, 2.0),
        color: [0; 3],
        phaseable: false,
    });
    w.actors[6].pos = V3::new(-3.0, 0.0, 4.0);
    w.actors[6].character = 1;
    w.actors[6].race_skill.remaining = trickcal_shooting_balddagu::race_skill::BURROW_DURATION;
    w.map.push(Block {
        center: V3::new(3.0, 2.0, 2.0),
        size: V3::new(2.0, 4.0, 0.3),
        color: [0; 3],
        phaseable: true,
    });
    let hp = w.actors[6].hp;
    ultimate(&mut w);
    advance(&mut w, 90);
    w.actors[6].race_skill.remaining = trickcal_shooting_balddagu::race_skill::BURROW_DURATION;
    advance(&mut w, 60);
    assert_eq!(w.actors[1].combat.guard, 0.0, "covered lane");
    assert_eq!(w.actors[2].combat.guard, 0.0, "outside crest");
    assert_eq!(w.actors[3].combat.guard, 0.0, "above crest");
    assert_eq!(w.actors[6].hp, hp, "burrowed");
}

#[test]
fn ultimate_interrupts_low_skill_and_stun_before_release_cancels_wave() {
    let mut w = fixture();
    start(&mut w);
    ultimate(&mut w);
    advance(&mut w, 50);
    w.actors[0].combat.stun = 2.0;
    advance(&mut w, 80);
    assert!(!w.actors[0].combat.naia.active());
    assert!(w.projectiles.is_empty());
    assert!(
        w.actors[0].combat.ultimate < 10.0,
        "interruption does not refund charge"
    );
}
