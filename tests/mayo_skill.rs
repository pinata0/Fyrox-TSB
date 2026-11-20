use trickcal_shooting_balddagu::{
    combat::{ProjectileKind, ULT_PER_SECOND},
    game::*,
    mayo_skill::{self as m, Phase as Action},
    motion::{Motion, MotionClock},
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
        a.pos = V3::new(60.0 + i as f32, 0.0, 60.0);
    }
    w.actors[0].character = 8;
    w.actors[0].ammo = 0;
    w.actors[0].pos = V3::default();
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
fn cast(w: &mut World, q: bool) {
    w.actors[0].combat.ultimate = 100.0;
    tick(
        w,
        Input {
            ability: !q,
            ultimate: q,
            ..Default::default()
        },
    );
}
#[test]
fn basic_fire_continues_past_the_old_magazine_and_reload_does_not_interrupt() {
    let mut w = world();
    assert_eq!(trickcal_shooting_balddagu::kits::kit(8).magazine, 0);
    for shot in 1..=40 {
        tick(&mut w, Input { fire: true, reload: true, ..Default::default() });
        assert_eq!(w.actors[0].shot, shot);
        assert_eq!(w.actors[0].ammo, 0);
        assert_eq!(w.actors[0].reload, 0.0);
        assert_eq!(w.projectiles.last().unwrap().kind,
            if shot % 4 == 0 { ProjectileKind::Poison } else { ProjectileKind::Needle });
        advance(&mut w, 22);
    }
}

#[test]
fn shot_markers_and_clip_lengths_match_the_archived_source() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../assets/characters/skill/mayo.json")).unwrap();
    for (i, phase) in [Action::Skill, Action::Ultimate].into_iter().enumerate() {
        let clip = &source["clips"][i];
        assert!((clip["duration"].as_f64().unwrap() as f32 - phase.duration()).abs() < 0.00001);
        let events = clip["events"].as_array().unwrap();
        let shots = &events[events.len() - phase.shots().len()..];
        for (event, &expected) in shots.iter().zip(phase.shots()) {
            assert!((event.as_f64().unwrap() as f32 - expected).abs() < 0.00001);
        }
    }
}
#[test]
fn exact_source_timing_three_and_eight_shots_without_consuming_ammo() {
    for phase in [Action::Skill, Action::Ultimate] {
        let mut w = world();
        cast(&mut w, phase == Action::Ultimate);
        let start = w.actors[0].shot;
        for n in 1..=(phase.duration() / DT).round() as usize {
            tick(&mut w, Input::default());
            let expected = phase
                .shots()
                .iter()
                .filter(|&&t| t <= n as f32 * DT + 0.00001)
                .count();
            assert_eq!(
                w.actors[0].shot - start,
                expected as u32,
                "{phase:?} tick {n}"
            );
        }
        assert!(!w.actors[0].combat.mayo.active());
        assert_eq!(w.actors[0].ammo, 0);
        assert!(w
            .projectiles
            .iter()
            .all(|p| p.kind == ProjectileKind::Poison));
    }
}
#[test]
fn each_shot_uses_current_aim_without_a_fan_spread() {
    let mut w = world();
    cast(&mut w, true);
    advance(&mut w, 97);
    tick(
        &mut w,
        Input {
            yaw: 0.3,
            pitch: 0.2,
            ..Default::default()
        },
    );
    let first = w.projectiles.last().unwrap().velocity;
    for _ in 0..8 {
        tick(
            &mut w,
            Input {
                yaw: -0.5,
                pitch: -0.1,
                ..Default::default()
            },
        );
    }
    let second = w.projectiles.last().unwrap().velocity;
    assert!((first - direction(0.3, 0.2) * m::DART_SPEED).length() < 0.001);
    assert!((second - direction(-0.5, -0.1) * m::DART_SPEED).length() < 0.001);
}
#[test]
fn hard_interruptions_cancel_the_remaining_volley() {
    for condition in 0..6 {
        let mut w = world();
        cast(&mut w, true);
        advance(&mut w, 99);
        let fired = w.actors[0].shot;
        match condition {
            0 => w.actors[0].combat.stun = 1.0,
            1 => w.actors[0].combat.sleep = 1.0,
            2 => w.actors[0].combat.frozen = 1.0,
            3 => w.actors[0].combat.polymorph = 1.0,
            4 => {
                w.actors[0].hp = 0;
                w.actors[0].respawn = 20.0;
            }
            _ => w.actors[0].character = 4,
        }
        advance(&mut w, 150);
        assert!(!w.actors[0].combat.mayo.active());
        assert_eq!(w.actors[0].shot, fired);
    }
}
#[test]
fn temporal_freeze_pauses_without_losing_or_duplicating_shots() {
    let mut a = world().actors[0].clone();
    a.combat.mayo = m::State {
        phase: Action::Ultimate,
        elapsed: 1.5,
    };
    a.combat.temporal = 1.0;
    for _ in 0..60 {
        m::advance_actor(&mut a);
    }
    assert_eq!(a.combat.mayo.elapsed, 1.5);
    a.combat.temporal = 0.0;
    m::advance_actor(&mut a);
    assert!((a.combat.mayo.elapsed - 1.5 - DT).abs() < 0.00001);
}
#[test]
fn active_volley_blocks_reloading_and_competing_actions() {
    let mut w = world();
    cast(&mut w, false);
    tick(
        &mut w,
        Input {
            reload: true,
            fire: true,
            ultimate: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].combat.mayo.phase, Action::Skill);
    assert_eq!(w.actors[0].reload, 0.0);
    assert_eq!(w.actors[0].ammo, 0);
    assert_eq!(w.actors[0].combat.ultimate, 100.0);
}
#[test]
fn poison_refreshes_without_stack_damage_or_damage_over_time() {
    let mut w = world();
    w.actors[6].pos = V3::new(0.0, 0.0, 3.0);
    w.actors[6].combat.guard = 150.0;
    w.actors[6].combat.guard_time = 20.0;
    cast(&mut w, false);
    advance(&mut w, 54);
    let remaining = w.actors[6].combat.poison;
    assert!(remaining > 3.8);
    advance(&mut w, 14);
    assert!(
        w.actors[6].combat.poison > remaining - 0.03,
        "third hit refreshes duration"
    );
    let guard = w.actors[6].combat.guard;
    let hp = w.actors[6].hp;
    advance(&mut w, 120);
    assert_eq!(w.actors[6].hp, hp);
    assert_eq!(w.actors[6].combat.guard, guard);
    advance(&mut w, 150);
    assert_eq!(w.actors[6].combat.poison, 0.0);
}
#[test]
fn ultimate_hits_never_charge_itself_and_motion_uses_authoritative_age() {
    let mut w = world();
    w.actors[6].pos = V3::new(0.0, 0.0, 3.0);
    w.actors[6].combat.guard = 500.0;
    w.actors[6].combat.guard_time = 20.0;
    cast(&mut w, true);
    advance(&mut w, 160);
    assert!((w.actors[0].combat.ultimate - 160.0 * DT * ULT_PER_SECOND).abs() < 0.001);
    let mut clock = MotionClock::default();
    let p = clock.observe(&w.actors[0], DT, w.tick, true);
    assert_eq!(p.motion, Motion::MayoUltimate);
    assert!((p.age - w.actors[0].combat.mayo.elapsed).abs() < 0.00001);
}
