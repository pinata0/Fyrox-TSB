use trickcal_shooting_balddagu::{
    combat::{Projectile, ProjectileKind},
    game::*,
    renua_skill::{self as r, Phase as Action},
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
        a.pos = V3::new(50.0 + i as f32 * 2.0, 0.0, 45.0);
    }
    w.actors[0].character = 7;
    w.actors[0].pos = V3::default();
    w.actors[0].ammo = 0;
    w.actors[6].character = 7;
    w.actors[6].pos = V3::new(0.0, 0.0, 8.0);
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
fn field() -> Projectile {
    let mut p = Projectile::visual(1, 0, ProjectileKind::TimeField, V3::new(0.0, 0.0, 1.5));
    p.velocity = V3::new(0.0, 0.0, 1.0);
    p.life = r::FIELD_SECONDS;
    p
}
#[test]
fn basic_waits_for_source_release_and_never_turns_into_an_automatic_laser() {
    let mut w = world();
    w.actors[6].pos.x = 20.0;
    for attack in 1..=4 {
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
        assert_eq!(w.actors[0].combat.renua.phase, Action::Basic);
        advance(&mut w, 76);
        assert!(w.projectiles.is_empty());
        advance(&mut w, 3);
        assert_eq!(w.actors[0].shot, attack);
        assert_eq!(w.projectiles[0].kind, ProjectileKind::RenuaOrb);
        advance(&mut w, 85);
        assert_eq!(w.actors[0].ammo, 0);
    }
}
#[test]
fn right_click_releases_one_hand_laser_and_does_not_repeat_while_held() {
    let mut w = world();
    let input = Input {
        alt_fire: true,
        ..Default::default()
    };
    tick(&mut w, input);
    assert_eq!(w.actors[0].combat.renua.phase, Action::Enhanced);
    for _ in 0..80 {
        tick(&mut w, input);
    }
    assert_eq!(w.actors[6].hp, MAX_HP);
    for _ in 0..200 {
        tick(&mut w, input);
    }
    assert_eq!(w.actors[0].shot, 1);
    assert!(w.actors[6].hp < MAX_HP);
    assert!(w.projectiles.is_empty());
}
#[test]
fn moving_skill_fires_exactly_five_beams_and_rejects_other_attacks() {
    let mut w = world();
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    for _ in 0..298 {
        tick(
            &mut w,
            Input {
                strafe: 0.2,
                fire: true,
                alt_fire: true,
                ..Default::default()
            },
        );
    }
    let distance = trickcal_shooting_balddagu::movement::WALK_SPEED * 0.2 * DT * 298.0;
    assert!((w.actors[0].pos.x.abs() - distance).abs() < 0.001);
    assert_eq!(w.actors[0].shot, 5);
    assert!(w.projectiles.is_empty());
}
#[test]
fn skill_heals_only_real_hits_and_clamps_health() {
    let mut w = world();
    w.actors[0].hp = 30;
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    advance(&mut w, 300);
    assert_eq!(w.actors[0].shot, 5);
    assert_eq!(w.actors[0].hp, 30 + 5 * r::SKILL_HEAL as u16);
    assert!(w.actors[6].hp < MAX_HP);
    let mut guarded = world();
    guarded.actors[0].hp = MAX_HP - 2;
    guarded.actors[6].combat.guard = 100.0;
    guarded.actors[6].combat.guard_time = 8.0;
    tick(
        &mut guarded,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    advance(&mut guarded, 70);
    assert_eq!(guarded.actors[0].hp, MAX_HP);
    assert_eq!(guarded.actors[6].hp, MAX_HP);
    assert!(guarded.actors[6].combat.guard < 100.0);
    for mode in 0..3 {
        let mut w = world();
        w.actors[0].hp = 30;
        match mode {
            0 => w.actors[6].pos.x = 10.0,
            1 => w.actors[6].shield = 10.0,
            _ => w.map.push(Block {
                center: V3::new(0.0, 1.5, 3.0),
                size: V3::new(10.0, 4.0, 0.5),
                color: [0; 3],
                phaseable: false,
            }),
        }
        tick(
            &mut w,
            Input {
                ability: true,
                ..Default::default()
            },
        );
        advance(&mut w, 300);
        assert_eq!(w.actors[0].hp, 30);
        assert_eq!(w.actors[6].hp, MAX_HP);
    }
}
#[test]
fn ultimate_delays_rectangular_field_then_slows_both_teams_equally() {
    let mut w = world();
    w.actors[1].pos = V3::new(1.0, 0.0, 8.0);
    w.actors[2].pos = V3::new(4.2, 0.0, 8.0);
    w.actors[0].combat.ultimate = 100.0;
    tick(
        &mut w,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    advance(&mut w, 252);
    assert!(w.projectiles.is_empty());
    advance(&mut w, 3);
    assert_eq!(w.projectiles[0].kind, ProjectileKind::TimeField);
    advance(&mut w, 60);
    let mid = w.actors[6].combat.temporal;
    assert!((0.45..0.55).contains(&mid));
    assert_eq!(w.actors[1].combat.temporal, mid);
    assert_eq!(w.actors[2].combat.temporal, 0.0);
    advance(&mut w, 65);
    assert_eq!(w.actors[1].combat.temporal, 1.0);
    assert_eq!(w.actors[6].combat.temporal, 1.0);
    assert_eq!(w.actors[1].combat.haste, 0.0);
    assert_eq!(w.actors[6].hp, MAX_HP);
}
#[test]
fn frozen_targets_cannot_move_jump_fire_or_escape_with_momentum_and_expiry_releases() {
    let mut w = world();
    w.projectiles.push(field());
    w.actors[1].pos = V3::new(1.0, 0.0, 5.0);
    advance(&mut w, 125);
    let position = w.actors[1].pos;
    w.actors[1].portals.momentum = V3::new(30.0, 20.0, 0.0);
    let mut inputs = [Input::default(); PLAYERS];
    inputs[1] = Input {
        forward: 1.0,
        jump: true,
        fire: true,
        ..Default::default()
    };
    w.tick(inputs);
    assert_eq!(w.actors[1].pos, position);
    assert_eq!(w.actors[1].shot, 0);
    w.projectiles.clear();
    w.tick(inputs);
    assert_eq!(w.actors[1].combat.temporal, 0.0);
    assert_ne!(w.actors[1].pos, position);
}
#[test]
fn exposure_resets_on_exit_does_not_stack_and_respects_cover_and_rotation() {
    let mut w = world();
    w.projectiles.push(field());
    w.projectiles.push(field());
    advance(&mut w, 60);
    assert!((w.actors[6].combat.temporal - 0.5).abs() < 0.01);
    w.actors[6].pos.x = 5.0;
    advance(&mut w, 1);
    assert_eq!(w.actors[6].combat.temporal, 0.0);
    w.actors[6].pos.x = 0.0;
    w.map.push(Block {
        center: V3::new(0.0, 1.5, 3.0),
        size: V3::new(10.0, 4.0, 0.5),
        color: [0; 3],
        phaseable: false,
    });
    advance(&mut w, 120);
    assert_eq!(w.actors[6].combat.temporal, 0.0);
    let mut p = field();
    p.velocity = V3::new(1.0, 0.0, 0.0);
    assert!(r::contains(&p, V3::new(8.0, 0.0, 2.0)));
    assert!(!r::contains(&p, V3::new(0.0, 0.0, 8.0)));
}
#[test]
fn death_or_stun_cancels_unreleased_actions_and_owner_death_clears_field() {
    let mut w = world();
    tick(
        &mut w,
        Input {
            alt_fire: true,
            ..Default::default()
        },
    );
    w.actors[0].combat.stun = 2.0;
    advance(&mut w, 100);
    assert_eq!(w.actors[0].shot, 0);
    w.actors[0].hp = 0;
    w.projectiles.push(field());
    advance(&mut w, 1);
    assert!(w.projectiles.is_empty());
    assert_eq!(w.actors[0].combat.renua.phase, Action::Off);
}
#[test]
fn action_clock_packing_handles_slow_motion_final_ticks_and_rejects_invalid_data() {
    for phase in [
        Action::Basic,
        Action::Enhanced,
        Action::Skill,
        Action::Ultimate,
    ] {
        for age in [0.0, phase.duration() * 0.5, phase.duration() - 0.001] {
            let s = r::State {
                phase,
                elapsed: age,
                alt_down: true,
            };
            let d = r::State::unpack(phase, s.pack()).unwrap();
            assert!((d.elapsed - age).abs() < DT + 0.0001);
            assert!(d.alt_down);
        }
    }
    assert!(r::State::unpack(Action::Off, 1).is_none());
    assert!(r::State::unpack(Action::Skill, 1 << 18).is_none());
}

#[test]
fn time_field_slows_and_pauses_existing_casts_then_resumes_them() {
    use trickcal_shooting_balddagu::{
        aya_skill, erpin, hilde_skill, id_skill, naia_skill, shion_skill,
    };
    let mut w = world();
    w.actors[9].character = 7;
    let mut area = field();
    area.owner = 9;
    w.projectiles.push(area);
    for (slot, character) in [1, 2, 3, 4, 7, 13, 14].into_iter().enumerate() {
        let a = &mut w.actors[slot];
        a.character = character;
        a.pos = V3::new(0.0, 0.0, 8.0);
        a.combat.temporal = 0.5;
    }
    w.actors[0].combat.naia.phase = naia_skill::Phase::Start;
    w.actors[0].combat.naia.elapsed = 0.1;
    w.actors[1].combat.erpin = erpin::State {
        eating: true,
        elapsed: 0.1,
        ..Default::default()
    };
    w.actors[2].combat.id.start(id_skill::Phase::Ultimate);
    w.actors[2].combat.id.elapsed = 0.1;
    w.actors[3].combat.aya.start(aya_skill::Phase::Skill);
    w.actors[3].combat.aya.elapsed = 0.1;
    w.actors[4].combat.renua.start(Action::Skill);
    w.actors[4].combat.renua.elapsed = 0.1;
    w.actors[5].combat.hilde.start(hilde_skill::Phase::Skill);
    w.actors[5].combat.hilde.elapsed = 0.1;
    w.actors[6].combat.shion.start(shion_skill::Phase::Load);
    w.actors[6].combat.shion.elapsed = 0.1;
    let clocks = |w: &World| {
        [
            w.actors[0].combat.naia.elapsed,
            w.actors[1].combat.erpin.elapsed,
            w.actors[2].combat.id.elapsed,
            w.actors[3].combat.aya.elapsed,
            w.actors[4].combat.renua.elapsed,
            w.actors[5].combat.hilde.elapsed,
            w.actors[6].combat.shion.elapsed,
        ]
    };
    let mut inputs = [Input::default(); PLAYERS];
    inputs[1].alt_fire = true;
    w.tick(inputs);
    let slow = clocks(&w);
    let expected = 0.1 + DT * (0.5 - DT / r::FREEZE_SECONDS);
    assert!(
        slow.iter().all(|age| (age - expected).abs() < 0.00001),
        "{slow:?}"
    );
    for a in &mut w.actors[..7] {
        a.combat.temporal = 1.0;
    }
    for _ in 0..10 {
        w.tick(inputs);
    }
    assert_eq!(clocks(&w), slow);
    assert!(w.actors[..7].iter().all(|a| a.shot == 0));
    w.projectiles.clear();
    w.tick(inputs);
    assert!(clocks(&w)
        .iter()
        .zip(slow)
        .all(|(age, before)| (age - before - DT).abs() < 0.00001));
}
