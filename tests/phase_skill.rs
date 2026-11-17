use trickcal_shooting_balddagu::{game::*, movement, phase_skill, roster::SelectionResult};

fn world(id: u8) -> World {
    let mut w = World::room(
        trickcal_shooting_balddagu::level::MapId::FairyKingdomCastle,
        true,
    );
    w.phase = Phase::Playing;
    w.map = vec![Block {
        center: V3::new(0.0, 2.0, 2.0),
        size: V3::new(8.0, 4.0, 2.0),
        color: [0; 3],
        phaseable: true,
    }];
    w.actors[0].character = id;
    w.actors[0].pos = V3::default();
    w
}
fn tick(w: &mut World, input: Input, count: usize) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = input;
    for _ in 0..count {
        w.tick(inputs);
    }
}
fn shift() -> Input {
    Input {
        phase_shift: true,
        ..Default::default()
    }
}
fn enter(w: &mut World) {
    tick(
        w,
        Input {
            forward: 1.0,
            ..shift()
        },
        20,
    );
    assert!(movement::blocked(&w.map, w.actors[0].pos));
}

#[test]
fn no_passive_wall_crossing_or_shift_speed_boost_for_any_race() {
    for id in 0..15 {
        let mut w = world(id);
        tick(
            &mut w,
            Input {
                forward: 1.0,
                ..Default::default()
            },
            45,
        );
        assert!(w.actors[0].pos.z < 0.601, "passive wall phase: {id}");
        w.map.clear();
        w.actors[0].pos = V3::default();
        tick(
            &mut w,
            Input {
                forward: 1.0,
                ..shift()
            },
            1,
        );
        assert!(
            (w.actors[0].pos.z - 5.8 * DT).abs() < 0.0001,
            "Shift acceleration: {id}"
        );
        assert_eq!(w.actors[0].phase_skill.active(), matches!(id, 9 | 14));
    }
}

#[test]
fn expiry_ejects_to_nearest_clear_face_and_hold_never_recasts() {
    for id in [9, 14] {
        let mut w = world(id);
        enter(&mut w);
        // Closer to the far face: do not teleport back to the entry position.
        w.actors[0].pos.z = 2.8;
        tick(&mut w, shift(), 161);
        let a = &w.actors[0];
        assert!(!a.phase_skill.active());
        assert!((a.phase_skill.cooldown - phase_skill::COOLDOWN).abs() < 0.001);
        assert!(!movement::blocked(&w.map, a.pos));
        assert!((a.pos.z - 3.42).abs() < 0.001);
        assert_eq!(a.pos.y, 0.0);
        tick(&mut w, shift(), 725);
        assert!(!w.actors[0].phase_skill.active());
        assert_eq!(w.actors[0].phase_skill.cooldown, 0.0);
        tick(&mut w, Input::default(), 1);
        tick(&mut w, shift(), 1);
        assert_eq!(w.actors[0].phase_skill.remaining, phase_skill::DURATION);
    }
}

#[test]
fn expiry_resolves_overlapping_walls_and_never_crosses_map_boundary() {
    let mut w = world(14);
    w.map.push(Block {
        center: V3::new(3.0, 2.0, 2.0),
        size: V3::new(2.0, 4.0, 8.0),
        color: [0; 3],
        phaseable: true,
    });
    w.map.push(Block {
        center: V3::new(0.0, 2.0, 3.6),
        size: V3::new(30.0, 4.0, 0.2),
        color: [0; 3],
        phaseable: false,
    });
    enter(&mut w);
    w.actors[0].pos = V3::new(2.5, 0.0, 2.8);
    w.actors[0].phase_skill.remaining = DT;
    tick(&mut w, Input::default(), 1);
    assert!(!movement::blocked(&w.map, w.actors[0].pos));
    assert!(w.actors[0].pos.z < 3.1);
    assert_eq!(w.actors[0].pos.y, 0.0);
}

#[test]
fn expiry_outside_walls_does_not_teleport_and_cooldown_blocks_presses() {
    let mut w = world(9);
    tick(&mut w, shift(), 1);
    w.actors[0].phase_skill.remaining = DT;
    let before = w.actors[0].pos;
    tick(&mut w, Input::default(), 1);
    assert_eq!(w.actors[0].pos, before);
    tick(&mut w, shift(), 1);
    assert!(!w.actors[0].phase_skill.active());
    assert!(w.actors[0].phase_skill.cooldown > 11.9);
}

#[test]
fn disabled_and_channeling_block_activation_but_stun_does_not_pause_expiry() {
    for kind in 0..4 {
        let mut w = world(14);
        match kind {
            0 => w.actors[0].combat.stun = 1.0,
            1 => w.actors[0].combat.sleep = 1.0,
            2 => w.actors[0].combat.dash = 1.0,
            _ => w.actors[0].hp = 0,
        }
        tick(&mut w, shift(), 1);
        assert!(!w.actors[0].phase_skill.active());
    }
    let mut w = world(9);
    enter(&mut w);
    w.actors[0].combat.stun = 4.0;
    tick(&mut w, Input::default(), 161);
    assert!(!movement::blocked(&w.map, w.actors[0].pos));
    assert!(!w.actors[0].phase_skill.active());
}

#[test]
fn crouched_expiry_uses_body_clearance_and_death_clears_skill() {
    let mut w = world(14);
    tick(
        &mut w,
        Input {
            crouch: true,
            ..shift()
        },
        1,
    );
    w.actors[0].pos.z = 2.8;
    w.actors[0].phase_skill.remaining = DT;
    tick(
        &mut w,
        Input {
            crouch: true,
            ..Default::default()
        },
        1,
    );
    assert!(!movement::blocked_with_height(
        &w.map,
        w.actors[0].pos,
        w.actors[0].collision_height()
    ));
    w.actors[0].phase_skill.cooldown = 0.0;
    tick(&mut w, shift(), 1);
    assert!(w.actors[0].phase_skill.active());
    w.actors[0].hp = 0;
    w.actors[0].respawn = DT;
    tick(&mut w, Input::default(), 1);
    assert_eq!(w.actors[0].phase_skill.remaining, 0.0);
    assert_eq!(w.actors[0].phase_skill.cooldown, 0.0);
}

#[test]
fn authored_map_wall_expiry_always_finds_clear_in_bounds_space() {
    for map_id in trickcal_shooting_balddagu::level::MapId::ALL {
        let map = map_id.arena();
        for wall in map
            .iter()
            .filter(|b| b.phaseable && b.min().y <= 0.0 && b.max().y > 1.8)
        {
            let mut a = Actor {
                character: 14,
                pos: V3::new(wall.center.x, 0.0, wall.center.z),
                ..Default::default()
            };
            a.phase_skill.remaining = DT;
            a.phase_skill.safe_position = map_id.spawn_center(true, false);
            phase_skill::advance(&map, &mut a, false);
            assert!(
                !movement::blocked(&map, a.pos),
                "{map_id:?}: {:?}",
                wall.center
            );
            assert_eq!(a.pos.y, 0.0);
            assert!(a.pos.x > -100.0 && a.pos.x < 100.0 && a.pos.z > -100.0 && a.pos.z < 100.0);
        }
    }
}

#[test]
fn character_change_ends_phase_without_refreshing_cooldown() {
    let mut w = world(9);
    w.actors[0].pos = w.spawn_center(0);
    tick(&mut w, shift(), 1);
    assert!(w.actors[0].phase_skill.active());
    assert_eq!(w.select_character(0, 3), SelectionResult::Accepted);
    assert!(!w.actors[0].phase_skill.active());
    assert_eq!(w.actors[0].phase_skill.cooldown, phase_skill::COOLDOWN);
}
