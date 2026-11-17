use trickcal_shooting_balddagu::{game::*, level::UPPER_FLOOR, movement, navigation};

fn world() -> World {
    let mut w = World {
        phase: Phase::Playing,
        ..Default::default()
    };
    for a in &mut w.actors {
        a.human = true;
    }
    w
}

fn tick(w: &mut World, input: Input, frames: usize) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = input;
    for _ in 0..frames {
        w.tick(inputs);
    }
}

#[test]
fn walk_up_a_staircases_and_jump_from_each_height() {
    for (start, yaw, frames, height) in [
        (V3::new(-12.0, 0.0, -6.3), 0.0, 155, 3.6),
        (
            V3::new(24.28, 0.0, 4.0),
            -std::f32::consts::FRAC_PI_2,
            110,
            3.6,
        ),
    ] {
        let mut w = world();
        w.actors[0].pos = start;
        tick(
            &mut w,
            Input {
                forward: 1.0,
                yaw,
                ..Default::default()
            },
            frames,
        );
        let a = &w.actors[0];
        assert!(
            (a.pos.y - height).abs() < 0.01,
            "stair {start:?}: {:?}",
            a.pos
        );
        tick(
            &mut w,
            Input {
                jump: true,
                ..Default::default()
            },
            1,
        );
        assert!(w.actors[0].pos.y > height);
        tick(&mut w, Input::default(), 90);
        assert!((w.actors[0].pos.y - height).abs() < 0.01);
    }
}

#[test]
fn ground_floor_and_b_hall_remain_walkable_without_bridge() {
    let map = arena();
    for y in [0.0, UPPER_FLOOR] {
        let from = V3::new(-12.0, y, 12.0);
        let to = movement::walk(&map, from, V3::new(4.0, 0.0, 0.0), true);
        assert!((to.x + 8.0).abs() < 0.01, "door on floor {y}: {to:?}");
    }
    let through = movement::walk(&map, V3::new(36.0, 0.0, 15.0), V3::new(9.0, 0.0, 0.0), true);
    assert!((through.x - 45.0).abs() < 0.01);
    let across = movement::walk(&map, V3::new(40.0, 0.0, 6.0), V3::new(0.0, 0.0, 18.0), true);
    assert!((across.z - 24.0).abs() < 0.01, "B hall floor: {across:?}");
    // No invisible bridge, stair, landings or pier collision remains.
    for (x, z) in [
        (30.0, 2.0),
        (40.0, 3.75),
        (40.0, 11.5),
        (40.0, 15.0),
        (40.0, 18.5),
        (40.0, 25.8),
    ] {
        let mut p = V3::new(x, 5.0, z);
        let mut velocity = -100.0;
        movement::vertical(&map, &mut p, &mut velocity, 0.1);
        assert_eq!(p.y, 0.0, "removed bridge at {x}/{z}: {p:?}");
    }
}

#[test]
fn window_is_open_to_shots_but_sill_and_pillars_block() {
    let map = arena();
    let obstruction = |origin: V3| {
        map.iter()
            .filter_map(|b| ray_box(origin, V3::new(1.0, 0.0, 0.0), b.min(), b.max()))
            .fold(f32::INFINITY, f32::min)
    };
    assert!(obstruction(V3::new(-12.0, 5.1, 8.4)) > 10.0);
    assert!(obstruction(V3::new(-12.0, 3.9, 8.4)) < 3.0);
    assert!(obstruction(V3::new(-12.0, 5.1, 10.05)) < 3.0);
    assert!(movement::blocked(&map, V3::new(-9.0, UPPER_FLOOR, 8.4)));
    assert!(!movement::blocked(
        &map,
        V3::new(-9.0, UPPER_FLOOR + 0.8, 8.4)
    ));
}

#[test]
fn jump_through_window_then_land_on_gallery() {
    let mut w = world();
    w.actors[0].pos = V3::new(-9.9, UPPER_FLOOR, 8.4);
    tick(
        &mut w,
        Input {
            jump: true,
            ..Default::default()
        },
        1,
    );
    tick(
        &mut w,
        Input {
            forward: 1.0,
            yaw: std::f32::consts::FRAC_PI_2,
            ..Default::default()
        },
        24,
    );
    assert!(
        w.actors[0].pos.x > -8.3,
        "window traversal: {:?}",
        w.actors[0].pos
    );
    tick(&mut w, Input::default(), 90);
    assert!((w.actors[0].pos.y - UPPER_FLOOR).abs() < 0.01);
}

#[test]
fn balcony_drop_lands_on_ground_and_high_ground_does_not_capture() {
    let mut w = world();
    w.actors[0].pos = V3::new(-8.0, UPPER_FLOOR, 12.0);
    tick(
        &mut w,
        Input {
            forward: 1.0,
            yaw: std::f32::consts::FRAC_PI_2,
            ..Default::default()
        },
        35,
    );
    tick(&mut w, Input::default(), 90);
    assert_eq!(w.actors[0].pos.y, 0.0);
    w.phase = Phase::Playing;
    w.progress = 0.0;
    w.actors[0].pos = POINTS[0] + V3::new(0.0, UPPER_FLOOR, 0.0);
    w.update_objective();
    assert_eq!(w.progress, 0.0);
}

#[test]
fn vertical_sweep_stops_at_ceiling_and_floor_even_at_high_speed() {
    let map = vec![Block {
        center: V3::new(0.0, 3.45, 0.0),
        size: V3::new(10.0, 0.3, 10.0),
        color: [0; 3],
        phaseable: true,
    }];
    let mut p = V3::new(0.0, 1.0, 0.0);
    let mut velocity = 100.0;
    movement::vertical(&map, &mut p, &mut velocity, 0.1);
    assert!((p.y - 1.5).abs() < 0.01);
    assert_eq!(velocity, 0.0);
    p.y = 10.0;
    velocity = -200.0;
    movement::vertical(&map, &mut p, &mut velocity, 0.1);
    assert!((p.y - UPPER_FLOOR).abs() < 0.01);
    assert_eq!(velocity, 0.0);
}

#[test]
fn upper_floor_supports_emotes() {
    let mut w = world();
    w.actors[0].pos = V3::new(-12.0, UPPER_FLOOR, 12.0);
    tick(
        &mut w,
        Input {
            dance: true,
            ..Default::default()
        },
        1,
    );
    assert!(w.actors[0].dancing);
}

#[test]
fn navigation_climbs_stairs_and_returns_to_objective() {
    let mut w = world();
    w.actors[0].pos = V3::new(2.0, 0.0, -6.0);
    for target in [V3::new(-12.0, UPPER_FLOOR, 12.0), POINTS[0]] {
        for _ in 0..1000 {
            let pos = w.actors[0].pos;
            if (pos - target).length() < 0.3 {
                break;
            }
            let next = navigation::next_waypoint(&w.map, pos, target);
            let dir = (next - pos).flat();
            tick(
                &mut w,
                Input {
                    forward: 1.0,
                    yaw: dir.x.atan2(dir.z),
                    ..Default::default()
                },
                1,
            );
        }
        assert!(
            (w.actors[0].pos - target).length() < 0.4,
            "target {target:?}, actual {:?}",
            w.actors[0].pos
        );
    }
}
