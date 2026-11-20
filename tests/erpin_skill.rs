use trickcal_shooting_balddagu::{
    combat::{AttackClass, Projectile, ProjectileKind},
    erpin_skill::{self as e, Phase as Action},
    game::*,
    motion::{Motion, MotionClock},
    movement,
};
fn world() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.human = true;
        a.active = i == 0;
        a.shield = 0.0;
        a.barrier = 0.0;
        a.combat = Default::default();
    }
    let a = &mut w.actors[0];
    a.character = 2;
    a.pos = V3::default();
    a.hp = MAX_HP;
    a.ammo = 20;
    a.cooldown = 0.0;
    w
}
fn tick(w: &mut World, input: Input, n: usize) {
    for _ in 0..n {
        let mut ins = [Input::default(); PLAYERS];
        ins[0] = input;
        w.tick(ins);
    }
}
fn start(w: &mut World, ultimate: bool) {
    w.actors[0].combat.ultimate = 100.0;
    tick(
        w,
        Input {
            ability: !ultimate,
            ultimate,
            ..Default::default()
        },
        1,
    );
}
fn enemy(w: &mut World, i: usize, pos: V3) {
    let a = &mut w.actors[i];
    a.active = true;
    a.hp = MAX_HP;
    a.pos = pos;
    a.shield = 0.0;
    a.barrier = 0.0;
    a.combat = Default::default();
}
#[test]
fn four_orbs_release_at_source_frames_and_never_on_cast() {
    let mut w = world();
    start(&mut w, false);
    assert!(w.projectiles.is_empty());
    for frame in 1..=170 {
        tick(&mut w, Input::default(), 1);
        let expected = e::RELEASES
            .iter()
            .filter(|&&t| frame as f32 * DT + 0.00001 >= t)
            .count();
        assert_eq!(
            w.projectiles
                .iter()
                .filter(|p| p.kind == ProjectileKind::MagicBomb)
                .count(),
            expected,
            "tick {frame}"
        );
    }
    assert_eq!(w.actors[0].combat.erpin_skill.phase, Action::Off);
}
#[test]
fn orb_contact_explodes_once_and_hurts_multiple_enemies() {
    let mut w = world();
    enemy(&mut w, 6, V3::new(0.0, 0.0, 3.0));
    enemy(&mut w, 7, V3::new(1.1, 0.0, 3.0));
    let mut p = Projectile::visual(999, 0, ProjectileKind::MagicBomb, V3::new(0.0, 1.2, 0.0));
    p.character = 2;
    p.velocity = V3::new(0.0, 0.0, 20.0);
    p.life = 3.0;
    p.damage = 14.0;
    p.class = AttackClass::Skill;
    w.projectiles.push(p);
    tick(&mut w, Input::default(), 10);
    assert!(w.actors[6].hp < MAX_HP);
    assert!(w.actors[7].hp < MAX_HP);
    assert!(w
        .projectiles
        .iter()
        .any(|p| p.kind == ProjectileKind::ErpinBlast));
    let hp = w.actors[6].hp;
    tick(&mut w, Input::default(), 40);
    assert_eq!(w.actors[6].hp, hp);
    assert!(w.projectiles.is_empty());
}
#[test]
fn charge_has_no_timeout_cannot_turn_cancel_or_be_slowed() {
    let mut w = world();
    start(&mut w, true);
    tick(&mut w, Input::default(), 70);
    assert_eq!(w.actors[0].combat.erpin_skill.phase, Action::Charge);
    let begin = w.actors[0].pos;
    for _ in 0..240 {
        let c = &mut w.actors[0].combat;
        c.slow = 3.0;
        c.rain_slow = 3.0;
        c.temporal = 0.8;
        c.stun = 1.0;
        tick(
            &mut w,
            Input {
                forward: -1.0,
                strafe: 1.0,
                yaw: 1.0,
                fire: true,
                alt_fire: true,
                jump: true,
                reload: true,
                crouch: true,
                phase_shift: true,
                ability: true,
                ultimate: true,
                ..Default::default()
            },
            1,
        );
        assert_eq!(w.actors[0].combat.erpin_skill.phase, Action::Charge);
        assert_eq!(w.actors[0].combat.slow, 0.0);
        assert_eq!(w.actors[0].ammo, 20);
    }
    assert!((w.actors[0].pos.z - begin.z - e::CHARGE_SPEED * 240.0 * DT).abs() < 0.002);
    assert!(w.actors[0].pos.x.abs() < 0.001);
}
#[test]
fn body_collision_deals_heavy_damage_recoils_self_and_stuns_until_clip_end() {
    let mut w = world();
    enemy(&mut w, 6, V3::new(0.0, 0.0, 4.0));
    start(&mut w, true);
    for _ in 0..100 {
        tick(&mut w, Input::default(), 1);
        if w.actors[0].combat.erpin_skill.phase == Action::Recover {
            break;
        }
    }
    assert_eq!(w.actors[0].combat.erpin_skill.phase, Action::Recover);
    assert!(MAX_HP - w.actors[6].hp >= 60);
    assert!(w
        .projectiles
        .iter()
        .any(|p| p.kind == ProjectileKind::ErpinCrash));
    let point = w.actors[0].pos;
    let hp = w.actors[6].hp;
    let mut clock = MotionClock::default();
    for n in 1..180 {
        tick(
            &mut w,
            Input {
                forward: 1.0,
                fire: true,
                ..Default::default()
            },
            1,
        );
        assert!(w.actors[0].combat.stun > 0.0, "tick {n}");
        assert_eq!(
            clock.observe(&w.actors[0], DT, w.tick, true).motion,
            Motion::ErpinRecover
        );
    }
    assert!(point.z - w.actors[0].pos.z > 5.0);
    assert_eq!(hp, w.actors[6].hp);
    assert_eq!(w.actors[0].ammo, 20);
    tick(&mut w, Input::default(), 2);
    assert_eq!(w.actors[0].combat.erpin_skill.phase, Action::Off);
    assert_eq!(w.actors[0].combat.stun, 0.0);
}
#[test]
fn thin_wall_stops_charge_without_tunneling_and_blocks_recoil() {
    let mut w = world();
    w.map.push(Block {
        center: V3::new(0.0, 1.5, 4.0),
        size: V3::new(10.0, 3.0, 0.03),
        color: [128; 3],
        phaseable: true,
    });
    start(&mut w, true);
    tick(&mut w, Input::default(), 84);
    assert_eq!(w.actors[0].combat.erpin_skill.phase, Action::Recover);
    assert!(w.actors[0].pos.z < 3.6);
    assert!(!movement::blocked(&w.map, w.actors[0].pos));
}
#[test]
fn interrupted_cast_cancels_unreleased_orbs_and_death_cancels_charge() {
    let mut w = world();
    start(&mut w, false);
    tick(&mut w, Input::default(), 80);
    w.actors[0].combat.stun = 1.0;
    tick(&mut w, Input::default(), 100);
    assert!(w.projectiles.is_empty());
    assert_eq!(w.actors[0].combat.erpin_skill.phase, Action::Off);
    w.actors[0].combat.stun = 0.0;
    start(&mut w, true);
    tick(&mut w, Input::default(), 75);
    w.actors[0].hp = 0;
    w.actors[0].respawn = 5.0;
    let pos = w.actors[0].pos;
    tick(&mut w, Input::default(), 2);
    assert_eq!(w.actors[0].combat.erpin_skill.phase, Action::Off);
    assert_eq!(pos, w.actors[0].pos);
}
#[test]
fn shared_prediction_matches_wall_contact_and_recoil_without_damage() {
    let mut w = world();
    enemy(&mut w, 6, V3::new(0.0, 0.0, 5.0));
    start(&mut w, true);
    let mut predicted = w.actors[0].clone();
    for _ in 0..160 {
        predicted.combat.stun = (predicted.combat.stun - DT).max(0.0);
        e::advance_actor(&w.map, &w.actors, 0, &mut predicted);
        movement::step_actor(&w.map, &mut predicted, Input::default());
        tick(&mut w, Input::default(), 1);
        assert!((predicted.pos - w.actors[0].pos).length() < 0.001);
        assert_eq!(
            predicted.combat.erpin_skill.phase,
            w.actors[0].combat.erpin_skill.phase
        );
    }
    assert_eq!(predicted.hp, MAX_HP);
}

#[test]
fn stopped_recovery_keeps_stun_until_the_source_clip_finishes() {
    let w = world();
    let mut a = w.actors[0].clone();
    a.combat.erpin_skill = e::State {
        phase: Action::Recover,
        elapsed: 1.0,
    };
    for frozen in [false, true] {
        a.combat.temporal = if frozen { 0.0 } else { 1.0 };
        a.combat.frozen = if frozen { 4.0 } else { 0.0 };
        for _ in 0..240 {
            a.combat.stun = (a.combat.stun - DT).max(0.0);
            e::advance_actor(&w.map, &w.actors, 0, &mut a);
            assert_eq!(a.combat.erpin_skill.elapsed, 1.0);
            assert!(a.combat.stun >= 2.0);
        }
    }
}
