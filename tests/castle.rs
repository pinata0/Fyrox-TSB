use trickcal_shooting_balddagu::{game::*, level, movement, navigation};

#[test]
fn all_spawn_slots_are_clear_and_sanctuaries_do_not_overlap_objectives() {
    for attacker in 0..2 {
        for point in 0..2 {
            let mut w = World {
                attacker,
                point,
                ..Default::default()
            };
            for slot in 0..PLAYERS {
                let center = w.spawn_center(team(slot));
                let p = center + V3::new((slot % TEAM_SIZE) as f32 * 1.8 - 4.5, 0.0, 0.0);
                w.actors[slot].pos = p;
                assert!(
                    !movement::blocked(&w.map, p),
                    "blocked spawn {attacker}/{point}/{slot}: {p:?}"
                );
                assert!(w.in_spawn(slot));
                for objective in POINTS {
                    w.actors[slot].pos = objective;
                    assert!(!w.in_spawn(slot));
                }
            }
        }
    }
}

#[test]
fn outer_gate_arch_has_open_headroom_and_solid_shoulders() {
    let map = arena();
    let crosses_gate = |x, y| {
        let from = V3::new(x, y, -8.6);
        map.iter().any(|b| {
            ray_box(from, V3::new(0.0, 0.0, 1.0), b.min(), b.max())
                .is_some_and(|distance| distance < 1.2)
        })
    };
    // The former rectangular lintel blocked the central opening above 3 m.
    assert!(!crosses_gate(2.0, 4.8));
    assert!(crosses_gate(2.0, 5.2));
    assert!(crosses_gate(-1.9, 3.7));
    assert!(crosses_gate(5.9, 3.7));
    assert!(!movement::blocked(&map, V3::new(2.0, 2.5, -8.0)));
    for x in [-1.5, 2.0, 5.5] {
        assert!(movement::walkable_line(
            &map,
            V3::new(x, 0.0, -8.6),
            V3::new(x, 0.0, -7.4)
        ));
    }
}

#[test]
fn shared_blender_layout_matches_game_metadata_and_anubis_topology() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../assets/maps/fairy-kingdom-castle/layout.json"
    ))
    .unwrap();
    assert_eq!(data["slug"], level::SLUG);
    assert_eq!(data["name"], level::NAME);
    for (i, bound) in [level::MIN_X, level::MAX_X, level::MIN_Z, level::MAX_Z]
        .iter()
        .enumerate()
    {
        assert_eq!(data["bounds"][i].as_f64().unwrap() as f32, *bound);
    }
    for (i, p) in POINTS.iter().enumerate() {
        for (axis, value) in [p.x, p.y, p.z].iter().enumerate() {
            assert_eq!(data["points"][i][axis].as_f64().unwrap() as f32, *value);
        }
    }
    let map = arena();
    // The street and temple axes form a real quarter-turn, not a straight chain.
    let approach = (POINTS[0] - level::spawn_center(true, false))
        .flat()
        .normalized();
    let continuation = (POINTS[1] - POINTS[0]).flat().normalized();
    assert!((approach.x * continuation.x + approach.z * continuation.z).abs() < 0.25);
    assert!(movement::blocked(&map, V3::new(28.0, 0.0, 9.0)));
    assert!(!movement::blocked(&map, V3::new(28.0, 0.0, 15.0)));
    // Actual building mass, rather than a little screen, separates the two courts.
    for from in [POINTS[0], level::spawn_center(true, false)] {
        let to = POINTS[1];
        let direction = (to - from).normalized();
        assert!(map.iter().any(|b| ray_box(
            from + V3::new(0.0, EYE_HEIGHT, 0.0),
            direction,
            b.min(),
            b.max()
        )
        .is_some_and(|distance| distance < (to - from).length())));
    }
    // Every drawn route is checked against the real movement envelope.
    for route in data["routes"].as_array().unwrap() {
        let points: Vec<V3> = route["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                V3::new(
                    p[0].as_f64().unwrap() as f32,
                    p[1].as_f64().unwrap() as f32,
                    p[2].as_f64().unwrap() as f32,
                )
            })
            .collect();
        for pair in points.windows(2) {
            assert!(
                movement::walkable_line(&map, pair[0], pair[1]),
                "{}: {:?} -> {:?}",
                route["name"],
                pair[0],
                pair[1]
            );
        }
    }
}

#[test]
fn navigation_reaches_each_objective_from_all_active_spawns() {
    let map = arena();
    let mut routes = Vec::new();
    for b in [false, true] {
        for attack in [false, true] {
            routes.push((level::spawn_center(attack, b), POINTS[b as usize]));
        }
    }
    for (start, target) in routes {
        let mut p = start;
        for _ in 0..1800 {
            if (p - target).length() < 0.35 {
                break;
            }
            let next = navigation::next_waypoint(&map, p, target);
            let remaining = (next - p).flat();
            let delta = remaining.normalized() * remaining.length().min(0.12);
            p = movement::walk(&map, p, delta, true);
            let mut vy = -6.0;
            movement::vertical(&map, &mut p, &mut vy, DT);
        }
        assert!(
            (p - target).length() < 0.4,
            "route {start:?} -> {target:?} stopped at {p:?}"
        );
    }
}
