use trickcal_shooting_balddagu::movement::WALK_SPEED;
use trickcal_shooting_balddagu::{game::*, level::MapId, movement, race_skill, roster::ROSTER};

fn world(id: u8) -> World {
    let mut w = World::room(MapId::default(), true);
    w.map.clear();
    w.phase = Phase::Playing;
    w.actors[0].character = id;
    w.actors[0].pos = V3::default();
    w.actors[0].shield = 0.0;
    w.actors[0].ammo = 20;
    w
}

fn tick(w: &mut World, input: Input, count: usize) {
    for _ in 0..count {
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0] = input;
        w.tick(inputs);
    }
}

fn shift() -> Input {
    Input {
        phase_shift: true,
        ..Default::default()
    }
}

#[test]
fn both_beasts_launch_in_aim_direction_land_and_require_a_fresh_press() {
    for id in [11, 12] {
        let mut w = world(id);
        let input = Input {
            yaw: std::f32::consts::FRAC_PI_2,
            ..shift()
        };
        tick(&mut w, input, 1);
        assert!(w.actors[0].pos.x > 0.19 && w.actors[0].pos.z.abs() < 0.001);
        assert!(w.actors[0].vy > 9.0 && w.actors[0].race_skill.active());
        let mut peak = 0.0_f32;
        for _ in 0..120 {
            tick(&mut w, input, 1);
            peak = peak.max(w.actors[0].pos.y);
        }
        assert!(peak > 2.5);
        assert_eq!(w.actors[0].pos.y, 0.0);
        assert!((12.0..14.0).contains(&w.actors[0].pos.x));
        assert!(!w.actors[0].race_skill.active());
        tick(&mut w, input, 400);
        assert_eq!(w.actors[0].race_skill.cooldown, 0.0);
        assert_eq!(w.actors[0].pos.y, 0.0);
        tick(&mut w, Input::default(), 1);
        tick(&mut w, input, 1);
        assert!(w.actors[0].vy > 9.0);
    }
}

#[test]
fn leap_obeys_walls_ceilings_and_raised_landings() {
    let mut w = world(12);
    w.map = vec![
        Block {
            center: V3::new(0.0, 2.5, 3.0),
            size: V3::new(8.0, 5.0, 0.2),
            color: [0; 3],
            phaseable: true,
        },
        Block {
            center: V3::new(0.0, 3.5, 0.0),
            size: V3::new(8.0, 0.2, 8.0),
            color: [0; 3],
            phaseable: true,
        },
    ];
    tick(&mut w, shift(), 1);
    for _ in 0..120 {
        tick(&mut w, Input::default(), 1);
        assert!(w.actors[0].pos.z < 2.51);
        assert!(w.actors[0].pos.y <= 1.601);
        assert!(!movement::blocked(&w.map, w.actors[0].pos));
    }
    let mut w = world(11);
    w.map.push(Block {
        center: V3::new(0.0, 0.5, 10.0),
        size: V3::new(6.0, 1.0, 8.0),
        color: [0; 3],
        phaseable: false,
    });
    tick(&mut w, shift(), 1);
    tick(&mut w, Input::default(), 100);
    assert_eq!(w.actors[0].pos.y, 1.0);
    assert!(!w.actors[0].race_skill.active());
}

#[test]
fn only_requested_races_activate_and_locks_prevent_activation() {
    for id in 0..ROSTER.len() as u8 {
        let mut w = world(id);
        tick(&mut w, shift(), 1);
        assert_eq!(
            w.actors[0].race_skill.active(),
            matches!(id, 0 | 1 | 10 | 11 | 12) || race_skill::has_surge(ROSTER[id as usize].race)
        );
    }
    for id in [0, 1, 10, 11, 12] {
        for lock in 0..3 {
            let mut w = world(id);
            match lock {
                0 => w.actors[0].combat.stun = 1.0,
                1 => w.actors[0].combat.sleep = 1.0,
                _ => w.actors[0].combat.dash = 1.0,
            }
            tick(&mut w, shift(), 1);
            assert!(!w.actors[0].race_skill.active());
        }
    }
    let mut w = world(1);
    w.actors[0].pos.y = 3.0;
    tick(&mut w, shift(), 1);
    assert!(!w.actors[0].burrowed());
}

#[test]
fn both_spirits_move_underground_expire_and_do_not_retrigger_held_shift() {
    for id in [1, 10] {
        let mut w = world(id);
        tick(&mut w, shift(), 1);
        assert!(w.actors[0].burrowed());
        assert_eq!(w.actors[0].eye().y, 0.55);
        tick(
            &mut w,
            Input {
                forward: 1.0,
                jump: true,
                ..shift()
            },
            60,
        );
        assert!((w.actors[0].pos.z - WALK_SPEED).abs() < 0.001);
        assert_eq!(w.actors[0].pos.y, 0.0);
        tick(&mut w, shift(), 179);
        assert!(w.actors[0].burrowed());
        tick(&mut w, shift(), 1);
        assert!(!w.actors[0].burrowed());
        assert_eq!(w.actors[0].race_skill.cooldown, race_skill::BURROW_COOLDOWN);
        assert!(w.actors[0].vy > 0.0);
        tick(&mut w, shift(), 485);
        assert!(!w.actors[0].burrowed());
        tick(&mut w, Input::default(), 1);
        tick(&mut w, shift(), 1);
        assert!(w.actors[0].burrowed());
    }
}

#[test]
fn emerge_requires_continuous_hold_and_consumes_fire_until_release() {
    for id in [1, 10] {
        let mut w = world(id);
        w.actors[0].combat.ultimate = 100.0;
        let ammo = w.actors[0].ammo;
        tick(&mut w, shift(), 1);
        let fire = Input {
            fire: true,
            ability: true,
            ultimate: true,
            reload: true,
            ..Default::default()
        };
        tick(&mut w, fire, 20);
        assert!(w.actors[0].burrowed());
        assert_eq!(w.actors[0].ammo, ammo);
        assert_eq!(w.actors[0].combat.ultimate, 100.0);
        assert_eq!(w.actors[0].ability_cooldown, 0.0);
        assert_eq!(w.actors[0].reload, 0.0);
        tick(&mut w, Input::default(), 1);
        assert_eq!(w.actors[0].race_skill.charge, 0.0);
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
            35,
        );
        assert!(w.actors[0].burrowed());
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
            1,
        );
        assert!(!w.actors[0].burrowed());
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
            10,
        );
        assert_eq!(w.actors[0].shot, 0);
        tick(&mut w, Input::default(), 1);
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
            1,
        );
        assert_eq!(w.actors[0].shot, 1);
    }
}

#[test]
fn burrow_cannot_cross_walls_and_exits_safely_on_ledge_departure() {
    let mut w = world(10);
    w.map.push(Block {
        center: V3::new(0.0, 2.0, 2.0),
        size: V3::new(8.0, 4.0, 0.2),
        color: [0; 3],
        phaseable: true,
    });
    tick(&mut w, shift(), 1);
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
        80,
    );
    assert!(w.actors[0].burrowed());
    assert!(w.actors[0].pos.z < 1.51);
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
        36,
    );
    assert!(!movement::blocked(&w.map, w.actors[0].pos));
    let mut w = world(10);
    w.map.push(Block {
        center: V3::new(0.0, 1.5, 0.0),
        size: V3::new(4.0, 3.0, 2.0),
        color: [0; 3],
        phaseable: false,
    });
    w.actors[0].pos.y = 3.0;
    tick(&mut w, shift(), 1);
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
        30,
    );
    assert!(!w.actors[0].burrowed());
    assert!(w.actors[0].pos.y >= 0.0);
}

#[test]
fn burrow_does_not_capture_and_selection_preserves_cooldown() {
    let mut w = world(10);
    w.actors[6].active = true;
    w.actors[6].human = true;
    w.actors[6].hp = MAX_HP;
    w.actors[0].pos = w.points()[0];
    tick(&mut w, shift(), 1);
    tick(&mut w, Input::default(), 60);
    assert_eq!(w.progress, 0.0);
    w.actors[0].pos = w.spawn_center(0);
    assert_eq!(
        w.select_character(0, 11),
        trickcal_shooting_balddagu::roster::SelectionResult::Accepted
    );
    assert!(!w.actors[0].race_skill.active());
    assert_eq!(w.actors[0].race_skill.cooldown, race_skill::BURROW_COOLDOWN);
    tick(&mut w, Input::default(), 1);
    tick(&mut w, shift(), 1);
    assert!(!w.actors[0].race_skill.active());
    w.start();
    assert_eq!(w.actors[0].race_skill.cooldown, 0.0);
}
