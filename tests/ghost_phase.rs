use trickcal_shooting_balddagu::movement::WALK_SPEED;
use trickcal_shooting_balddagu::{
    game::*, ghost::wall_contacts, level::MapId, movement, roster::SelectionResult,
};

fn wall() -> Block {
    Block {
        center: V3::new(0.0, 2.0, 2.0),
        size: V3::new(8.0, 4.0, 0.3),
        color: [150; 3],
        phaseable: true,
    }
}

fn world(id: u8) -> World {
    let mut w = World {
        phase: Phase::Playing,
        map: vec![wall()],
        ..Default::default()
    };
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.human = true;
        a.pos = V3::new(10.0 + i as f32, 0.0, 0.0);
        a.shield = 0.0;
    }
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

#[test]
fn only_ghosts_with_shift_cross_without_damage_other_races_collide() {
    for id in 0..15 {
        let mut w = world(id);
        let mut saw_contact = false;
        for _ in 0..84 {
            tick(
                &mut w,
                Input {
                    forward: 1.0,
                    phase_shift: true,
                    ..Default::default()
                },
                1,
            );
            saw_contact |= w.actors[0].ghost_phase > 0.0;
        }
        if matches!(id, 9 | 14) {
            assert!(w.actors[0].pos.z > 4.0);
            assert!(saw_contact);
        } else {
            assert!(w.actors[0].pos.z < 1.46);
            assert!(!saw_contact);
        }
        assert_eq!(w.actors[0].hp, MAX_HP);
    }
}

#[test]
fn ghost_can_stop_inside_thick_wall_and_leave_in_reverse_or_diagonally() {
    for input in [
        Input {
            forward: -1.0,
            ..Default::default()
        },
        Input {
            forward: 1.0,
            strafe: 1.0,
            ..Default::default()
        },
    ] {
        let mut w = world(14);
        w.map[0].size.z = 2.0;
        tick(
            &mut w,
            Input {
                forward: 1.0,
                phase_shift: true,
                ..Default::default()
            },
            40,
        );
        assert!(movement::blocked(&w.map, w.actors[0].pos));
        let inside = w.actors[0].pos;
        tick(&mut w, Input::default(), 90);
        assert_eq!(inside, w.actors[0].pos);
        assert_eq!(w.actors[0].ghost_phase, 0.0);
        tick(&mut w, input, 60);
        assert!(!movement::blocked(&w.map, w.actors[0].pos));
    }
}

#[test]
fn ghost_stun_blocks_movement_and_death_and_respawn_clear_contact() {
    let mut w = world(9);
    w.actors[0].pos.z = 1.4;
    w.actors[0].combat.stun = 1.0;
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
        10,
    );
    assert_eq!(w.actors[0].pos.z, 1.4);
    assert_eq!(w.actors[0].ghost_phase, 0.0);
    w.actors[0].combat.stun = 0.0;
    tick(
        &mut w,
        Input {
            forward: 1.0,
            phase_shift: true,
            ..Default::default()
        },
        2,
    );
    assert!(w.actors[0].ghost_phase > 0.0);
    w.actors[0].hp = 0;
    w.actors[0].respawn = DT;
    tick(&mut w, Input::default(), 1);
    assert_eq!(w.actors[0].ghost_phase, 0.0);
}

#[test]
fn ghost_boundary_and_floor_and_ceiling_remain_solid() {
    let map = arena();
    for (start, delta) in [
        (V3::new(67.0, 0.0, 15.0), V3::new(8.0, 0.0, 0.0)),
        (V3::new(-21.0, 0.0, 0.0), V3::new(-8.0, 0.0, 0.0)),
        (V3::new(0.0, 0.0, -53.0), V3::new(0.0, 0.0, -8.0)),
        (V3::new(0.0, 0.0, 37.0), V3::new(0.0, 0.0, 8.0)),
    ] {
        let (end, contact) = movement::phase_walk(&map, start, delta, true);
        assert!((end - start).length() < 1.0);
        assert!(!contact);
    }
    let mut w = world(14);
    w.map = map;
    w.actors[0].pos = V3::new(-12.0, 0.0, 12.0);
    w.actors[0].vy = 100.0;
    tick(&mut w, Input::default(), 1);
    assert!(w.actors[0].pos.y <= 1.501);
    w.actors[0].pos.y = 5.0;
    w.actors[0].vy = -100.0;
    tick(&mut w, Input::default(), 1);
    assert!((w.actors[0].pos.y - 3.6).abs() < 0.001);
}

#[test]
fn ghost_stairs_and_airborne_wall_crossing_still_work() {
    let mut w = world(9);
    w.map = arena();
    w.actors[0].pos = V3::new(-12.0, 0.0, -6.3);
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
        310,
    );
    assert!((w.actors[0].pos.y - 3.6).abs() < 0.01);
    let mut w = world(14);
    w.actors[0].pos.z = 1.4;
    tick(
        &mut w,
        Input {
            jump: true,
            forward: 1.0,
            phase_shift: true,
            ..Default::default()
        },
        1,
    );
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
        30,
    );
    assert!(w.actors[0].pos.z > w.map[0].max().z + movement::BODY_RADIUS);
    assert!(w.actors[0].pos.y > 0.0);
}

#[test]
fn ghosts_stay_on_authored_stairs_when_headroom_runs_out() {
    for (map_id, json) in [
        (
            MapId::FairyKingdomCastle,
            include_str!("../assets/maps/fairy-kingdom-castle/layout.json"),
        ),
        (
            MapId::ElfCity,
            include_str!("../assets/maps/elf-city/layout.json"),
        ),
    ] {
        let layout: serde_json::Value = serde_json::from_str(json).unwrap();
        for stair in layout["stairs"].as_array().unwrap() {
            let point = |value: &serde_json::Value| {
                V3::new(
                    value[0].as_f64().unwrap() as f32,
                    value[1].as_f64().unwrap() as f32,
                    value[2].as_f64().unwrap() as f32,
                )
            };
            let start = point(&stair["start"]);
            let end = point(&stair["end"]);
            let direction = (end - start).flat().normalized();
            for id in [9, 14] {
                for low_ceiling in [false, true] {
                    let mut w = world(id);
                    w.map = map_id.arena();
                    w.actors[0].pos = start;
                    if low_ceiling {
                        // Fits above the fifth tread, but prevents the next step up.
                        w.map.push(Block {
                            center: V3::new(start.x, 3.6, start.z),
                            size: V3::new(40.0, 0.3, 40.0),
                            color: [0; 3],
                            phaseable: true,
                        });
                    }
                    let input = Input {
                        forward: 1.0,
                        yaw: direction.x.atan2(direction.z),
                        ..Default::default()
                    };
                    let frames = ((end - start).flat().length() / (WALK_SPEED * DT)).ceil() as usize;
                    let mut previous_height = start.y;
                    for _ in 0..frames {
                        tick(&mut w, input, 1);
                        let a = &w.actors[0];
                        assert!(
                            !movement::blocked(&w.map, a.pos) && a.pos.y >= previous_height - 0.001,
                            "{map_id:?} {} hero {id}, ceiling {low_ceiling}: {:?}",
                            stair["name"],
                            a.pos
                        );
                        assert_eq!(a.ghost_phase, 0.0);
                        previous_height = a.pos.y;
                    }
                    let height = if low_ceiling { 1.5 } else { end.y };
                    assert!((w.actors[0].pos.y - height).abs() < 0.001);
                    if low_ceiling {
                        // Continuing to push must stay stable, and backing out must still work.
                        let stopped = w.actors[0].pos;
                        tick(
                            &mut w,
                            Input {
                                phase_shift: true,
                                ..input
                            },
                            120,
                        );
                        assert_eq!(w.actors[0].pos, stopped);
                        tick(
                            &mut w,
                            Input {
                                forward: -1.0,
                                ..input
                            },
                            frames,
                        );
                        assert_eq!(w.actors[0].pos.y, 0.0);
                    }
                }
            }
        }
    }
}

#[test]
fn ghost_cannot_phase_into_a_step_when_either_lift_position_is_blocked() {
    for ceiling_z in [-0.3, 0.55] {
        let map = vec![
            Block {
                center: V3::new(0.0, 0.3, -0.5),
                size: V3::new(4.0, 0.6, 1.0),
                color: [0; 3],
                phaseable: true,
            },
            Block {
                center: V3::new(0.0, 0.45, 1.0),
                size: V3::new(4.0, 0.9, 1.0),
                color: [0; 3],
                phaseable: true,
            },
            Block {
                center: V3::new(0.0, 2.7, ceiling_z),
                size: V3::new(4.0, 0.3, 0.1),
                color: [0; 3],
                phaseable: true,
            },
        ];
        let start = V3::new(0.0, 0.6, 0.1);
        for delta in [
            V3::new(0.0, 0.0, 0.15),
            V3::new(0.0, 0.0, 3.0),
            V3::new(0.15, 0.0, 0.15),
        ] {
            let (end, contact) = movement::phase_walk(&map, start, delta, true);
            assert_eq!(end.z, start.z, "ceiling at {ceiling_z}, delta {delta:?}");
            assert_eq!(end.y, start.y);
            assert!((end.x - delta.x).abs() < 0.001, "can slide along the tread");
            assert!(!contact);
        }
    }
}

#[test]
fn wall_contacts_are_local_clipped_and_exclude_boundaries_and_floors() {
    let mut map = vec![wall()];
    let contacts = wall_contacts(&map, V3::new(0.0, 0.0, 2.0));
    assert_eq!(contacts.len(), 2);
    assert!(contacts
        .iter()
        .all(|c| c.face >= 2 && c.center.y > 0.0 && c.center.y < 1.8));
    assert!(wall_contacts(&map, V3::new(0.0, 0.0, 0.0)).is_empty());
    assert!(wall_contacts(&map, V3::new(0.0, 4.0, 2.0)).is_empty());
    map[0].phaseable = false;
    assert!(wall_contacts(&map, V3::new(0.0, 0.0, 2.0)).is_empty());
    // Thin arch segments can be phaseable walls. Check the actual top face,
    // rather than assuming every block shorter than STEP_HEIGHT is a floor.
    for map_id in MapId::ALL {
        for b in map_id.arena() {
            let feet = V3::new(b.center.x, b.max().y, b.center.z);
            assert!(wall_contacts(&[b], feet).is_empty());
        }
    }
}

#[test]
fn swapping_a_ghost_inside_a_wall_is_rejected_for_player_and_bot_exchange() {
    let mut w = world(9);
    w.phase = Phase::Setup;
    w.actors[0].pos.z = 2.0;
    assert_eq!(w.select_character(0, 4), SelectionResult::Locked);
    w.actors[0].pos.z = 0.0;
    w.actors[1].character = 14;
    w.actors[1].pos = V3::new(0.0, 0.0, 2.0);
    w.actors[1].human = false;
    assert_eq!(w.select_character(0, 14), SelectionResult::Locked);
}

#[test]
fn walls_still_block_ghost_shots() {
    let mut w = world(14);
    w.actors[6].pos = V3::new(0.0, 0.0, 4.0);
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(w.actors[6].hp, MAX_HP);
}
