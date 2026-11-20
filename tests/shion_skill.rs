use trickcal_shooting_balddagu::{
    game::*,
    motion::{Motion, MotionClock},
    shion_skill::{self as s, Phase as Action},
};

fn world() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.human = true;
        a.active = true;
        a.shield = 0.0;
        a.barrier = 0.0;
        a.combat = Default::default();
        a.pos = V3::new(30.0 + i as f32 * 2.0, 0.0, 30.0);
    }
    w.actors[0].character = 14;
    w.actors[0].ammo = 6;
    w.actors[0].pos = V3::default();
    w.actors[6].character = 7;
    w.actors[6].pos = V3::new(0.0, 0.0, 6.0);
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
fn load(w: &mut World) {
    tick(
        w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    advance(w, 128);
}

#[test]
fn charges_for_64_frames_then_waits_anchored_until_all_five_rounds_are_spent() {
    let mut w = world();
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].combat.shion.phase, Action::Load);
    advance(&mut w, 127);
    assert_eq!(w.actors[0].combat.magic_ammo, 0);
    assert_eq!(w.actors[6].hp, MAX_HP);
    advance(&mut w, 1);
    assert_eq!(w.actors[0].combat.magic_ammo, 5);
    assert_eq!(w.actors[0].combat.shion.phase, Action::Ready);
    w.actors[0].combat.ultimate = 100.0;
    let origin = w.actors[0].pos;
    w.actors[0].portals.momentum = V3::new(20.0, 0.0, 0.0);
    for _ in 0..800 {
        tick(
            &mut w,
            Input {
                forward: 1.0,
                strafe: 1.0,
                jump: true,
                crouch: true,
                phase_shift: true,
                ultimate: true,
                ability: true,
                reload: true,
                yaw: 0.2,
                ..Default::default()
            },
        );
    }
    let a = &w.actors[0];
    assert_eq!(a.pos, origin);
    assert_eq!(a.combat.magic_ammo, 5);
    assert_eq!(a.reload, 0.0);
    assert_eq!(a.combat.ultimate, 100.0);
    assert_eq!(a.yaw, 0.2);
    assert!(!a.phase_skill.active());
    assert!(!a.crouching);
    w.actors[6].pos.x = 10.0;
    for remaining in (0..5).rev() {
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
        advance(&mut w, 2);
        assert_eq!(w.actors[0].combat.magic_ammo, remaining);
        advance(&mut w, 58);
        assert_eq!(
            w.actors[0].combat.shion.phase,
            if remaining == 0 {
                Action::End
            } else {
                Action::Ready
            }
        );
    }
    assert_eq!(w.actors[0].shot, 5);
    assert_eq!(w.actors[0].ammo, 6);
    advance(&mut w, 79);
    assert_eq!(w.actors[0].combat.shion.phase, Action::End);
    advance(&mut w, 1);
    assert_eq!(w.actors[0].combat.shion.phase, Action::Off);
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
    );
    assert_ne!(w.actors[0].pos, origin);
}
#[test]
fn source_loop_releases_one_strong_stunning_round_and_respects_cover_allies_and_immunity() {
    let mut w = world();
    load(&mut w);
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
    );
    advance(&mut w, 1);
    assert_eq!(w.actors[6].hp, MAX_HP);
    advance(&mut w, 1);
    assert_eq!(w.actors[0].shot, 1);
    assert_eq!(
        w.actors[6].hp,
        MAX_HP - (s::DAMAGE * w.damage_multiplier(0)).round() as u16
    );
    assert!(w.actors[6].combat.stun > 0.7);
    let p = w.actors[6].pos;
    let mut inputs = [Input::default(); PLAYERS];
    inputs[6] = Input {
        forward: 1.0,
        fire: true,
        jump: true,
        ..Default::default()
    };
    w.tick(inputs);
    assert_eq!(w.actors[6].pos, p);
    assert_eq!(w.actors[6].shot, 0);
    advance(&mut w, 46);
    assert_eq!(w.actors[6].combat.stun, 0.0);
    for mode in 0..4 {
        let mut w = world();
        load(&mut w);
        match mode {
            0 => w.map.push(Block {
                center: V3::new(0.0, 1.5, 3.0),
                size: V3::new(5.0, 4.0, 0.4),
                color: [0; 3],
                phaseable: false,
            }),
            1 => w.actors[1].pos = V3::new(0.0, 0.0, 3.0),
            2 => w.actors[6].shield = 2.0,
            _ => {
                w.actors[6].character = 3;
                w.actors[6].combat.id.invulnerable = 2.0;
            }
        }
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
        advance(&mut w, 3);
        assert_eq!(w.actors[6].hp, MAX_HP);
        assert_eq!(w.actors[6].combat.stun, 0.0);
        assert_eq!(w.actors[1].hp, MAX_HP);
        assert_eq!(w.actors[0].combat.magic_ammo, 4);
    }
}
#[test]
fn held_fire_uses_exactly_five_rounds_without_normal_ammo_or_reload() {
    let mut w = world();
    w.actors[0].ammo = 0;
    w.actors[6].pos.x = 20.0;
    load(&mut w);
    for _ in 0..305 {
        tick(
            &mut w,
            Input {
                fire: true,
                reload: true,
                ..Default::default()
            },
        );
    }
    assert_eq!(w.actors[0].shot, 5);
    assert_eq!(w.actors[0].combat.shion.phase, Action::End);
    assert_eq!(w.actors[0].combat.magic_ammo, 0);
    assert_eq!(w.actors[0].ammo, 0);
    assert_eq!(w.actors[0].reload, 0.0);
}
#[test]
fn ultimate_executes_at_frame_80_once_through_guard_resistance_and_ward() {
    let mut w = world();
    w.actors[0].combat.ultimate = 100.0;
    w.actors[6].barrier = 8.0;
    w.actors[6].combat.guard = 120.0;
    w.actors[6].combat.guard_time = 10.0;
    w.actors[6].combat.resist = 10.0;
    w.actors[6].combat.ward = 10.0;
    w.actors[7].character = 9;
    w.actors[7].pos = V3::new(2.0, 0.0, 6.0);
    w.actors[7].combat.revive_armed = true;
    w.actors[1].pos = V3::new(1.0, 0.0, 6.0);
    w.actors[8].pos = V3::new(0.0, 0.0, -2.0);
    tick(
        &mut w,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    advance(&mut w, 159);
    assert_eq!(w.actors[6].hp, MAX_HP);
    assert_eq!(w.actors[0].kills, 0);
    advance(&mut w, 1);
    assert_eq!(w.actors[6].hp, 0);
    assert_eq!(w.actors[7].hp, 0);
    assert_eq!(w.actors[6].killed_by, Some(0));
    assert_eq!(w.actors[0].kills, 2);
    assert_eq!(w.actors[1].hp, MAX_HP);
    assert_eq!(w.actors[8].hp, MAX_HP);
    assert_eq!(w.actors[0].combat.magic_ammo, 0);
    assert!(w.projectiles.is_empty());
    let origin = w.actors[0].shot_end;
    advance(&mut w, 53);
    assert_eq!(w.actors[0].kills, 2);
    assert_eq!(w.actors[0].shot_end, origin);
    assert_eq!(w.actors[0].combat.shion.phase, Action::Off);
}
#[test]
fn ultimate_respects_wall_range_rotation_and_spawn_immunity() {
    for mode in 0..4 {
        let mut w = world();
        w.actors[0].combat.ultimate = 100.0;
        match mode {
            0 => w.map.push(Block {
                center: V3::new(0.0, 1.5, 3.0),
                size: V3::new(10.0, 5.0, 0.5),
                color: [0; 3],
                phaseable: false,
            }),
            1 => w.actors[6].pos = V3::new(0.0, 0.0, 10.2),
            2 => w.actors[6].shield = 10.0,
            _ => w.actors[6].pos = V3::new(6.0, 0.0, 0.0),
        }
        let input = Input {
            ultimate: true,
            yaw: if mode == 3 {
                std::f32::consts::FRAC_PI_2
            } else {
                0.0
            },
            ..Default::default()
        };
        tick(&mut w, input);
        for _ in 0..160 {
            tick(&mut w, input);
        }
        assert_eq!(w.actors[6].hp, if mode == 3 { 0 } else { MAX_HP });
    }
}
#[test]
fn stun_cancels_unreleased_skills_death_resets_and_time_stop_pauses_the_clock() {
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
        advance(&mut w, 30);
        w.actors[0].combat.stun = 0.4;
        advance(&mut w, 250);
        assert_eq!(w.actors[0].combat.shion.phase, Action::Off);
        assert_eq!(w.actors[6].hp, MAX_HP);
    }
    let mut w = world();
    load(&mut w);
    w.actors[0].hp = 0;
    w.actors[0].respawn = 7.0;
    advance(&mut w, 1);
    assert_eq!(w.actors[0].combat.shion.phase, Action::Off);
    let mut a = world().actors[0].clone();
    a.combat.shion.start(Action::Ultimate);
    a.combat.temporal = 1.0;
    for _ in 0..250 {
        assert_eq!(s::advance(&mut a, Input::default()), (false, false));
    }
    assert_eq!(a.combat.shion.elapsed, 0.0);
}
#[test]
fn native_clips_have_continuous_stance_boundaries_and_authoritative_pose_clocks() {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../assets/characters/skill/shion.json")).unwrap();
    for (i, n) in [65, 31, 41, 107].into_iter().enumerate() {
        assert_eq!(data["clips"][i]["samples"].as_array().unwrap().len(), n);
    }
    let mut a = world().actors[0].clone();
    let mut clock = MotionClock::default();
    for phase in [
        Action::Load,
        Action::Ready,
        Action::Fire,
        Action::End,
        Action::Ultimate,
    ] {
        a.combat.shion = s::State {
            phase,
            elapsed: if phase == Action::Ready { 0.0 } else { 0.5 },
        };
        let p = clock.observe(&a, DT, 10, true);
        assert_eq!(p.motion, phase.motion().unwrap());
        assert_eq!(p.age, a.combat.shion.elapsed);
        assert!(s::State::unpack(phase, a.combat.shion.pack()).is_some());
    }
    for (motion, age) in [
        (Motion::ShionLoad, s::LOAD_SECONDS),
        (Motion::ShionLoop, 0.0),
        (Motion::ShionLoop, 1.0),
        (Motion::ShionEnd, 0.0),
    ] {
        let (sample, _, _) = s::sample(motion, age).unwrap();
        assert!((sample.root[1] + 0.356556).abs() < 0.0001);
    }
    assert!(s::State::unpack(Action::Off, 1).is_none());
    assert!(s::State::unpack(Action::Load, u32::MAX).is_none());
}

#[test]
fn client_prediction_keeps_anchor_and_releases_at_the_same_recovery_tick() {
    use trickcal_shooting_balddagu::prediction::ClientMotion;
    let mut w = world();
    load(&mut w);
    w.actors[6].pos.x = 20.0;
    let mut client = ClientMotion::default();
    client.observe(&w, 0, 0);
    for sequence in 1..=400 {
        let input = Input {
            forward: 1.0,
            jump: true,
            fire: sequence <= 305,
            ..Default::default()
        };
        client.advance(&w, sequence, input);
        tick(&mut w, input);
        let predicted = client.actor(&w, 0, 0);
        assert!(
            (predicted.pos - w.actors[0].pos).length() < 0.001,
            "sequence {sequence}"
        );
        assert_eq!(
            predicted.combat.shion.phase, w.actors[0].combat.shion.phase,
            "sequence {sequence}"
        );
        assert_eq!(predicted.combat.magic_ammo, w.actors[0].combat.magic_ammo);
        if sequence % 3 == 0 {
            client.observe(&w, 0, sequence);
        }
    }
}
