use trickcal_shooting_balddagu::{game::*, level::MapId, movement, navigation};

#[test]
fn map_selection_preserves_roster_resets_round_and_survives_restart() {
    let mut w = World::default();
    w.actors[0].human = true;
    w.actors[0].character = 7;
    assert!(w.select_map(MapId::ElfCity));
    assert_eq!(w.map_id, MapId::ElfCity);
    assert_eq!(w.points()[0], V3::new(12.0, 0.0, -2.0));
    assert!(w.actors[0].human);
    assert_eq!(w.actors[0].character, 7);
    for i in 0..PLAYERS {
        assert!(!movement::blocked(&w.map, w.actors[i].pos));
        assert!(w.in_spawn(i));
    }
    w.start();
    assert_eq!(w.map_id, MapId::ElfCity);
    assert!(!w.select_map(MapId::FairyKingdomCastle));
    w.phase = Phase::Finished;
    w.start();
    assert_eq!(w.map_id, MapId::ElfCity);
    w.phase = Phase::Finished;
    assert!(w.select_map(MapId::FairyKingdomCastle));
    assert_eq!(w.phase, Phase::Lobby);
    assert_eq!(w.map.len(), arena().len());
}

#[test]
fn elf_objectives_use_the_selected_map_and_spawn_protection_follows_sides() {
    let mut w = World::for_map(MapId::ElfCity);
    w.start();
    w.phase = Phase::Playing;
    for a in &mut w.actors {
        a.pos = V3::new(-2.0, 0.0, -38.0);
    }
    w.actors[0].pos = w.points()[0];
    for _ in 0..120 {
        w.update_objective();
    }
    assert!(w.progress > 0.0);
    for attacker in [0, 1] {
        w.attacker = attacker;
        for point in [0, 1] {
            w.point = point;
            for i in 0..PLAYERS {
                w.actors[i].pos = w.spawn_center(team(i));
                assert!(w.in_spawn(i));
                assert!(!movement::blocked(&w.map, w.actors[i].pos));
            }
        }
    }
}

fn walk_to(map: &[Block], start: V3, target: V3) {
    let mut p = start;
    for _ in 0..2200 {
        if (p - target).length() < 0.35 {
            return;
        }
        let next = navigation::next_waypoint(map, p, target);
        let delta = (next - p).flat();
        p = movement::walk(map, p, delta.normalized() * delta.length().min(0.16), true);
        let mut vy = -6.0;
        movement::vertical(map, &mut p, &mut vy, DT);
    }
    panic!(
        "route {start:?} -> {target:?} stopped at {p:?}, next {:?}",
        navigation::next_waypoint(map, p, target)
    );
}

#[test]
fn bots_walk_between_spawns_objectives_and_all_stairs() {
    let id = MapId::ElfCity;
    let map = id.arena();
    for b in [false, true] {
        for attack in [true, false] {
            walk_to(&map, id.spawn_center(attack, b), id.points()[b as usize]);
        }
    }
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../assets/maps/elf-city/layout.json")).unwrap();
    let v = |value: &serde_json::Value| {
        V3::new(
            value[0].as_f64().unwrap() as f32,
            value[1].as_f64().unwrap() as f32,
            value[2].as_f64().unwrap() as f32,
        )
    };
    for stair in data["stairs"].as_array().unwrap() {
        let start = v(&stair["start"]);
        let end = v(&stair["end"]);
        assert!(!movement::blocked(&map, start), "{} start", stair["name"]);
        assert!(
            !movement::blocked(&map, end),
            "{} end {end:?}",
            stair["name"]
        );
        walk_to(&map, start, end);
        walk_to(&map, end, start);
    }
    for upper in data["high_ground"].as_array().unwrap() {
        walk_to(&map, id.points()[0], v(upper));
    }
}

#[test]
fn source_layout_points_bounds_and_geometry_match_runtime() {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../assets/maps/elf-city/layout.json")).unwrap();
    assert_eq!(
        data["blocks"].as_array().unwrap().len(),
        MapId::ElfCity.arena().len()
    );
    assert_eq!(data["slug"], MapId::ElfCity.slug());
    for (point, json) in MapId::ElfCity
        .points()
        .into_iter()
        .zip(data["points"].as_array().unwrap())
    {
        assert_eq!(
            [point.x, point.y, point.z],
            std::array::from_fn(|i| json[i].as_f64().unwrap() as f32)
        );
    }
    assert_eq!(MapId::from_wire(1), Some(MapId::ElfCity));
    assert_eq!(MapId::from_wire(2), None);
}
