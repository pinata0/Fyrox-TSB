use trickcal_shooting_balddagu::{
    bella_skill::{self as b, Phase as Action},
    combat::ProjectileKind,
    game::*,
    kits::kit,
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
        a.character = 5;
        a.shield = 0.0;
        a.barrier = 0.0;
        a.combat = Default::default();
        a.ammo = kit(5).magazine;
        a.pos = V3::new(60.0 + i as f32, 0.0, -60.0);
    }
    w.actors[0].character = 9;
    w.actors[0].ammo = 0;
    w.actors[0].pos = V3::default();
    w.actors[6].pos = V3::new(0.0, 0.0, 5.0);
    w
}
fn input(w: &mut World, slot: usize, input: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[slot] = input;
    w.tick(inputs);
}
fn advance(w: &mut World, ticks: usize) {
    for _ in 0..ticks {
        w.tick([Input::default(); PLAYERS]);
    }
}
fn cast(w: &mut World, ult: bool) {
    w.actors[0].combat.ultimate = 100.0;
    input(
        w,
        0,
        Input {
            ability: !ult,
            ultimate: ult,
            ..Default::default()
        },
    );
}
#[test]
fn fire_releases_at_source_frame_58_then_ticks_only_inside_the_fixed_rectangle() {
    let mut w = world();
    w.actors[7].pos = V3::new(1.6, 0.0, 5.0);
    w.actors[8].pos = V3::new(0.0, 0.0, -1.0);
    w.actors[9].pos = V3::new(0.0, 0.0, 10.1);
    cast(&mut w, false);
    advance(&mut w, 115);
    assert!(w.projectiles.is_empty());
    advance(&mut w, 1);
    assert_eq!(w.projectiles[0].kind, ProjectileKind::BellaFire);
    assert_eq!(w.actors[6].hp, MAX_HP);
    advance(&mut w, 1);
    assert!(w.actors[6].combat.taunt > 2.99);
    let point = w.projectiles[0].pos;
    w.actors[0].pos = V3::new(-20.0, 0.0, 0.0);
    advance(&mut w, 29);
    let damage = (b::FIRE_DAMAGE * w.damage_multiplier(team(0))).round() as u16;
    assert_eq!(w.actors[6].hp, MAX_HP - damage);
    assert_eq!(w.projectiles[0].pos, point);
    advance(&mut w, 210);
    assert!(w.projectiles.is_empty());
    assert_eq!(w.actors[6].hp, MAX_HP - damage * 8);
    assert_eq!(
        w.actors[6].combat.taunt, 0.0,
        "three seconds is not extended by damage ticks"
    );
    for i in [7, 8, 9] {
        assert_eq!(w.actors[i].hp, MAX_HP);
        assert_eq!(w.actors[i].combat.taunt, 0.0);
    }
}
#[test]
fn fire_respects_yaw_cover_height_and_damage_stops_after_leaving() {
    let mut w = world();
    cast(&mut w, false);
    advance(&mut w, 116);
    let mut p = w.projectiles[0].clone();
    p.velocity = V3::new(1.0, 0.0, 0.0);
    let mut a = w.actors[6].clone();
    a.pos = V3::new(5.0, 0.0, 0.0);
    assert!(b::in_fire(&[], &p, &a));
    a.pos.z = 1.6;
    assert!(!b::in_fire(&[], &p, &a));
    a.pos = V3::new(5.0, 2.0, 0.0);
    assert!(!b::in_fire(&[], &p, &a));
    let wall = Block {
        center: V3::new(0.0, 1.0, 2.0),
        size: V3::new(3.0, 3.0, 0.4),
        color: [0; 3],
        phaseable: true,
    };
    w.map.push(wall);
    advance(&mut w, 30);
    assert_eq!(w.actors[6].hp, MAX_HP);
    assert_eq!(w.actors[6].combat.taunt, 0.0);
    w.map.clear();
    advance(&mut w, 30);
    assert!(w.actors[6].hp < MAX_HP);
    let hp = w.actors[6].hp;
    w.actors[6].pos.x = 2.0;
    advance(&mut w, 90);
    assert_eq!(w.actors[6].hp, hp);
}
#[test]
fn taunt_blocks_all_hero_and_race_activations_but_keeps_basic_damage_and_movement() {
    for id in 0..15 {
        let mut w = world();
        let a = &mut w.actors[0];
        a.character = id;
        a.ammo = kit(id).magazine;
        a.combat.taunt = 3.0;
        a.combat.ultimate = 100.0;
        input(
            &mut w,
            0,
            Input {
                ability: true,
                phase_shift: true,
                ..Default::default()
            },
        );
        assert_eq!(w.actors[0].ability_cooldown, 0.0, "E for {id}");
        assert!(!w.actors[0].race_skill.active());
        assert!(!w.actors[0].phase_skill.active());
        input(
            &mut w,
            0,
            Input {
                ultimate: true,
                ..Default::default()
            },
        );
        assert_eq!(w.actors[0].combat.ultimate, 100.0, "Q for {id}");
    }
    let mut w = world();
    w.actors[6].pos.z = 3.0;
    w.actors[0].combat.taunt = 3.0;
    w.actors[0].combat.taunt_source = 7;
    input(
        &mut w,
        0,
        Input {
            fire: true,
            forward: 1.0,
            ..Default::default()
        },
    );
    let expected = (kit(9).damage * w.damage_multiplier(team(0))).round() as u16;
    assert_eq!(
        w.actors[6].hp,
        MAX_HP - expected,
        "no old off-target damage penalty"
    );
    assert!(w.actors[0].pos.z > 0.0);
    advance(&mut w, 180);
    assert_eq!(w.actors[0].combat.taunt, 0.0);
    cast(&mut w, false);
    assert_eq!(w.actors[0].combat.bella.phase, Action::Skill);
}
#[test]
fn alternate_skills_are_locked_and_bella_cast_cancels_on_taunt() {
    for id in [2, 3] {
        let mut w = world();
        w.actors[0].character = id;
        w.actors[0].combat.taunt = 3.0;
        input(
            &mut w,
            0,
            Input {
                alt_fire: true,
                ..Default::default()
            },
        );
        assert!(!w.actors[0].combat.erpin.eating);
        assert!(!w.actors[0].combat.id.active());
    }
    let mut w = world();
    cast(&mut w, true);
    w.actors[0].combat.taunt = 3.0;
    advance(&mut w, 310);
    assert!(w.projectiles.is_empty());
    assert_eq!(w.actors[6].combat.taunt, 0.0);
}
#[test]
fn lantern_selects_only_visible_susceptible_enemies_and_heals_once_on_completion() {
    let mut w = world();
    w.actors[0].hp = 40;
    for (i, p) in [
        (1, V3::new(2.0, 0.0, 0.0)),
        (2, V3::new(8.1, 0.0, 0.0)),
        (7, V3::new(-3.0, 0.0, 8.0)),
        (8, V3::new(0.0, 0.0, -5.0)),
        (9, V3::new(2.0, 0.0, 5.0)),
    ] {
        w.actors[i].pos = p;
    }
    w.actors[1].hp = 40;
    w.actors[2].hp = 40;
    w.actors[9].shield = 6.0;
    cast(&mut w, true);
    advance(&mut w, 127);
    assert_eq!(w.actors[6].combat.taunt, 0.0);
    advance(&mut w, 1);
    assert_eq!(w.actors[0].combat.bella.targets, (1 << 6) | (1 << 7));
    assert!(w.actors[6].combat.taunt > 2.98);
    for i in [8, 9] {
        assert_eq!(w.actors[i].combat.taunt, 0.0);
    }
    advance(&mut w, 171);
    assert_eq!(w.actors[1].hp, 40);
    advance(&mut w, 1);
    assert_eq!(w.actors[0].hp, 88);
    assert_eq!(w.actors[1].hp, 88);
    assert_eq!(w.actors[2].hp, 40);
    assert_eq!(w.actors[6].hp, MAX_HP);
    assert_eq!(w.actors[0].stats.healing, 96.0);
    assert_eq!(w.projectiles[0].kind, ProjectileKind::BellaHeal);
    advance(&mut w, 60);
    assert_eq!(w.actors[1].hp, 88);
    assert!(w.projectiles.is_empty());
}
#[test]
fn completion_without_targets_still_heals_but_death_and_stun_cancel_it() {
    for interrupt in [0, 1, 2] {
        let mut w = world();
        w.actors[6].pos.z = -6.0;
        w.actors[1].pos = V3::new(2.0, 0.0, 0.0);
        w.actors[1].hp = 80;
        cast(&mut w, true);
        advance(&mut w, 250);
        assert_eq!(w.actors[0].combat.bella.targets, 0);
        if interrupt == 1 {
            w.actors[0].combat.stun = 2.0;
        } else if interrupt == 2 {
            w.actors[0].hp = 0;
            w.actors[0].respawn = 10.0;
        }
        advance(&mut w, 50);
        assert_eq!(w.actors[1].hp, if interrupt == 0 { MAX_HP } else { 80 });
    }
}
#[test]
fn motion_clock_and_wire_state_keep_source_time_and_target_count() {
    let mut w = world();
    for phase in [Action::Skill, Action::Ultimate] {
        let targets = if phase == Action::Ultimate {
            (1 << 6) | (1 << 11)
        } else {
            0
        };
        w.actors[0].combat.bella = b::State {
            phase,
            elapsed: 2.5,
            targets,
        };
        let p = MotionClock::default().observe(&w.actors[0], 0.0, 1, true);
        assert_eq!(p.motion, phase.motion().unwrap());
        assert_eq!(p.age, 2.5);
        let decoded = b::State::unpack(phase, w.actors[0].combat.bella.pack()).unwrap();
        assert_eq!(decoded.targets, targets);
        assert!((decoded.elapsed - 2.5).abs() < 0.00001);
    }
    assert_eq!(Motion::BellaSkill.seconds(9), b::SKILL_SECONDS);
    assert_eq!(Motion::BellaUltimate.seconds(9), b::ULTIMATE_SECONDS);
    assert!(b::State::unpack(Action::Off, 1).is_none());
    assert!(b::State::unpack(Action::Skill, 1 << 9).is_none());
    assert!(b::State::unpack(Action::Ultimate, u32::MAX).is_none());
}
