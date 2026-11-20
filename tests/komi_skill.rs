use trickcal_shooting_balddagu::{
    game::*,
    komi_skill::{self as k, Phase as Action},
    race_skill,
};

fn world() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.active = i == 0 || i == 1 || i == 6;
        a.human = true;
        a.shield = 0.0;
        a.hp = 100;
        a.pos = V3::new(40.0 + i as f32 * 3.0, 0.0, 40.0);
    }
    w.actors[0].character = 12;
    w.actors[0].pos = V3::default();
    w.actors[0].hp = 20;
    w.actors[6].character = 12;
    w.actors[6].pos = V3::new(0.0, 0.0, 2.0);
    w.actors[6].yaw = std::f32::consts::PI;
    w
}
fn tick(w: &mut World, input: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = input;
    w.tick(inputs);
}
fn wait(w: &mut World, n: usize) {
    for _ in 0..n {
        tick(w, Input::default());
    }
}
fn rest(w: &mut World) {
    tick(
        w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
}
fn giant(w: &mut World) {
    w.actors[0].combat.ultimate = 100.0;
    tick(
        w,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    wait(w, (k::FEAST_SECONDS / DT).round() as usize);
    assert!(w.actors[0].combat.komi.giant());
}

#[test]
fn rest_is_invulnerable_and_only_self_heals_for_exact_duration() {
    let mut w = world();
    w.actors[1].pos = V3::new(-5.0, 0.0, 0.0);
    w.actors[1].hp = 20;
    rest(&mut w);
    for _ in 0..239 {
        let mut inputs = [Input::default(); PLAYERS];
        inputs[6] = Input {
            fire: true,
            yaw: std::f32::consts::PI,
            ..Default::default()
        };
        w.tick(inputs);
    }
    assert!(w.actors[0].combat.komi.resting());
    assert_eq!(w.actors[0].hp, 65);
    tick(&mut w, Input::default());
    assert_eq!(w.actors[0].hp, 68);
    assert!(!w.actors[0].combat.komi.active());
    assert_eq!(w.actors[1].hp, 20);
    assert_eq!(w.actors[0].stats.healing, 48.0);
    wait(&mut w, 60);
    assert_eq!(
        w.actors[0].hp, 68,
        "rest heal cannot leak beyond its lifetime"
    );
}
#[test]
fn every_gameplay_input_and_unbound_key_wakes_immediately_but_look_and_e_hold_do_not() {
    let inputs = [
        Input {
            forward: 1.0,
            ..Default::default()
        },
        Input {
            strafe: 1.0,
            ..Default::default()
        },
        Input {
            fire: true,
            ..Default::default()
        },
        Input {
            alt_fire: true,
            ..Default::default()
        },
        Input {
            jump: true,
            ..Default::default()
        },
        Input {
            reload: true,
            ..Default::default()
        },
        Input {
            phase_shift: true,
            ..Default::default()
        },
        Input {
            crouch: true,
            ..Default::default()
        },
        Input {
            ultimate: true,
            ..Default::default()
        },
        Input {
            dance: true,
            ..Default::default()
        },
        Input {
            wake_rest: true,
            ..Default::default()
        },
    ];
    for input in inputs {
        let mut w = world();
        rest(&mut w);
        wait(&mut w, 20);
        let hp = w.actors[0].hp;
        tick(&mut w, input);
        assert!(!w.actors[0].combat.komi.resting(), "{input:?}");
        wait(&mut w, 20);
        assert_eq!(w.actors[0].hp, hp);
    }
    let mut w = world();
    rest(&mut w);
    for _ in 0..20 {
        tick(
            &mut w,
            Input {
                ability: true,
                yaw: 0.8,
                pitch: 0.5,
                ..Default::default()
            },
        );
    }
    assert!(w.actors[0].combat.komi.resting());
    tick(&mut w, Input::default());
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    assert!(!w.actors[0].combat.komi.active());
}
#[test]
fn feast_grows_then_giant_locks_basics_and_restores_normal_state() {
    let mut w = world();
    w.actors[0].combat.ultimate = 100.0;
    tick(
        &mut w,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    let origin = w.actors[0].pos;
    let shots = w.actors[0].shot;
    for _ in 0..210 {
        tick(
            &mut w,
            Input {
                forward: 1.0,
                fire: true,
                phase_shift: true,
                ..Default::default()
            },
        );
    }
    assert_eq!(w.actors[0].pos, origin);
    assert_eq!(w.actors[0].shot, shots);
    assert!(w.actors[0].body_scale() > 1.0 && w.actors[0].body_scale() < k::GIANT_SCALE);
    wait(&mut w, 54);
    assert!(w.actors[0].combat.komi.giant());
    assert!((w.actors[0].body_scale() - k::GIANT_SCALE).abs() < 0.001);
    for _ in 0..479 {
        tick(
            &mut w,
            Input {
                fire: true,
                ability: true,
                ..Default::default()
            },
        );
    }
    assert_eq!(w.actors[0].shot, shots);
    assert!(w.actors[0].combat.komi.giant());
    tick(&mut w, Input::default());
    assert!(!w.actors[0].combat.komi.active());
    assert_eq!(w.actors[0].body_scale(), 1.0);
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
    );
    assert!(w.actors[0].shot > shots);
}
#[test]
fn empowered_shift_hits_once_on_landing_and_never_on_plain_jump() {
    let mut w = world();
    giant(&mut w);
    let expected = 100 - (k::LANDING_DAMAGE * w.damage_multiplier(0)).round() as u16;
    w.actors[6].pos = V3::new(0.0, 0.0, 12.0);
    tick(
        &mut w,
        Input {
            phase_shift: true,
            ..Default::default()
        },
    );
    assert!((w.actors[0].race_skill.cooldown - k::LEAP_COOLDOWN).abs() < DT);
    assert_eq!(w.actors[6].hp, 100);
    assert!(w.actors[0].race_skill.active());
    for _ in 0..160 {
        tick(&mut w, Input::default());
        if !w.actors[0].race_skill.active() {
            break;
        }
    }
    assert_eq!(w.actors[6].hp, expected);
    assert!(w.actors[0].combat.komi.impact > 0.0);
    wait(&mut w, 30);
    assert_eq!(w.actors[6].hp, expected);
    assert_eq!(w.actors[0].race_skill.cooldown, 0.0);
    tick(
        &mut w,
        Input {
            jump: true,
            ..Default::default()
        },
    );
    wait(&mut w, 60);
    assert_eq!(w.actors[6].hp, expected);
}
#[test]
fn landing_respects_cover_allies_and_buff_expiry() {
    let mut w = world();
    giant(&mut w);
    w.actors[0].combat.komi.elapsed = k::GIANT_SECONDS - DT * 2.0;
    tick(
        &mut w,
        Input {
            phase_shift: true,
            ..Default::default()
        },
    );
    w.actors[6].pos = V3::new(0.0, 0.0, 12.0);
    wait(&mut w, 100);
    assert_eq!(w.actors[6].hp, 100);
    assert!(!w.actors[0].combat.komi.active());
    let mut w = world();
    giant(&mut w);
    w.actors[1].pos = V3::new(0.0, 0.0, 12.0);
    w.actors[6].pos = V3::new(3.5, 0.0, 12.0);
    w.map.push(Block {
        center: V3::new(2.0, 1.5, 12.0),
        size: V3::new(0.4, 3.0, 8.0),
        color: [100; 3],
        phaseable: true,
    });
    tick(
        &mut w,
        Input {
            phase_shift: true,
            ..Default::default()
        },
    );
    wait(&mut w, 100);
    assert_eq!(w.actors[1].hp, 100);
    assert_eq!(w.actors[6].hp, 100);
    w.actors[0].combat.komi = Default::default();
    w.actors[0].race_skill = Default::default();
    tick(
        &mut w,
        Input {
            phase_shift: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].race_skill.cooldown, race_skill::LEAP_COOLDOWN);
}
#[test]
fn death_swap_and_polymorph_clear_both_actions() {
    for action in [Action::Rest, Action::Feast, Action::Giant] {
        let mut w = world();
        w.actors[0].combat.komi.phase = action;
        w.actors[0].hp = 0;
        w.actors[0].respawn = 20.0;
        tick(&mut w, Input::default());
        assert!(!w.actors[0].combat.komi.active());
        let mut w = world();
        w.actors[0].combat.komi.phase = action;
        w.actors[0].character = 0;
        tick(&mut w, Input::default());
        assert!(!w.actors[0].combat.komi.active());
        let mut w = world();
        w.actors[0].combat.komi.phase = action;
        w.actors[0].combat.polymorph = 1.0;
        tick(&mut w, Input::default());
        assert!(!w.actors[0].combat.komi.active());
    }
}
