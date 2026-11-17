use trickcal_shooting_balddagu::{
    game::*,
    movement,
    portal::{self, Portal},
    prediction::ClientMotion,
};
fn wall(x: f32, z: f32) -> Block {
    Block {
        center: V3::new(x, 2.5, z),
        size: V3::new(8.0, 5.0, 0.3),
        color: [0; 3],
        phaseable: true,
    }
}
fn world() -> World {
    let mut w = World::room(Default::default(), true);
    w.phase = Phase::Playing;
    w.map = vec![wall(0.0, 4.0), wall(12.0, 4.0)];
    w.actors[0].character = 3;
    w.actors[0].pos = V3::default();
    w.actors[0].shield = 0.0;
    w
}
fn tick(w: &mut World, input: Input, n: usize) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = input;
    for _ in 0..n {
        w.tick(inputs);
    }
}
fn yellow() -> Input {
    Input {
        phase_shift: true,
        fire: true,
        ..Default::default()
    }
}
fn blue() -> Input {
    Input {
        phase_shift: true,
        alt_fire: true,
        ..Default::default()
    }
}
fn pair() -> portal::Pair {
    [
        Some(Portal {
            center: V3::new(0.0, 1.5, 3.85),
            face: 5,
        }),
        Some(Portal {
            center: V3::new(12.0, 1.5, 3.85),
            face: 5,
        }),
    ]
}
#[test]
fn only_elves_place_and_shift_click_never_fires_or_phases() {
    for id in 0..15 {
        let mut w = world();
        w.actors[0].character = id;
        let shot = w.actors[0].shot;
        tick(&mut w, yellow(), 1);
        assert_eq!(
            w.actors[0].portals.ends[0].is_some(),
            portal::is_elf(&w.actors[0])
        );
        if portal::is_elf(&w.actors[0]) {
            assert_eq!(shot, w.actors[0].shot);
            assert!(!w.actors[0].phase_skill.active());
        }
    }
}
#[test]
fn colors_have_independent_five_second_cooldowns_and_click_edges() {
    let mut w = world();
    tick(&mut w, yellow(), 1);
    assert_eq!(w.actors[0].portals.cooldown, [5.0, 0.0]);
    let first = w.actors[0].portals.ends[0];
    w.actors[0].pos.x = 12.0;
    tick(&mut w, blue(), 1);
    assert_eq!(w.actors[0].portals.cooldown[1], 5.0);
    assert_eq!(first, w.actors[0].portals.ends[0]);
    tick(&mut w, blue(), 300);
    assert_eq!(w.actors[0].portals.cooldown, [0.0, 0.0]);
    w.actors[0].pos.x = 13.0;
    tick(&mut w, blue(), 10);
    assert_eq!(w.actors[0].portals.ends[1].unwrap().center.x, 12.0);
    tick(&mut w, Input::default(), 1);
    tick(&mut w, blue(), 1);
    assert_eq!(w.actors[0].portals.ends[1].unwrap().center.x, 13.0);
}
#[test]
fn failed_casts_do_not_consume_cooldown() {
    for state in 0..4 {
        let mut w = world();
        match state {
            0 => w.actors[0].combat.stun = 1.0,
            1 => w.actors[0].hp = 0,
            2 => w.map[0].phaseable = false,
            _ => w.map.clear(),
        }
        tick(&mut w, yellow(), 1);
        assert!(w.actors[0].portals.ends[0].is_none());
        assert_eq!(w.actors[0].portals.cooldown, [0.0, 0.0]);
    }
}
#[test]
fn full_aperture_fits_wall_floor_ceiling_and_rejects_occluders() {
    let eye = V3::new(0.0, 1.5, 0.0);
    assert_eq!(
        portal::placement(&[], eye, V3::new(0.0, -1.0, 0.0))
            .unwrap()
            .face,
        2
    );
    let ceiling = Block {
        center: V3::new(0.0, 6.0, 0.0),
        size: V3::new(10.0, 0.3, 10.0),
        ..wall(0.0, 0.0)
    };
    assert_eq!(
        portal::placement(&[ceiling], eye, V3::new(0.0, 1.0, 0.0))
            .unwrap()
            .face,
        3
    );
    let obstacle = Block {
        phaseable: false,
        ..wall(0.0, 2.0)
    };
    assert!(portal::placement(&[wall(0.0, 4.0), obstacle], eye, direction(0.0, 0.0)).is_none());
    let narrow = Block {
        size: V3::new(1.0, 2.0, 0.3),
        ..wall(0.0, 4.0)
    };
    assert!(portal::placement(&[narrow], eye, direction(0.0, 0.0)).is_none());
}
#[test]
fn both_colors_pass_without_bouncing_or_sticking() {
    for side in 0..2 {
        let mut w = world();
        w.actors[0].portals.ends = pair();
        w.actors[0].pos.x = side as f32 * 12.0;
        tick(
            &mut w,
            Input {
                forward: 1.0,
                ..Default::default()
            },
            42,
        );
        let a = &w.actors[0];
        assert!((a.pos.x - (1 - side) as f32 * 12.0).abs() < 0.01, "{a:?}");
        assert_eq!(a.portals.transits, 1);
        assert!(direction(a.yaw, a.pitch).z < -0.99);
        assert!(!movement::blocked(&w.map, a.pos));
    }
}
#[test]
fn single_entrance_and_backside_do_not_pass() {
    let mut w = world();
    w.actors[0].portals.ends[0] = pair()[0];
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
        80,
    );
    assert_eq!(w.actors[0].portals.transits, 0);
    assert!(w.actors[0].pos.z < 3.5);
    w.actors[0].portals.ends = pair();
    w.actors[0].pos = V3::new(0.0, 0.0, 5.0);
    tick(
        &mut w,
        Input {
            forward: -1.0,
            ..Default::default()
        },
        80,
    );
    assert_eq!(w.actors[0].portals.transits, 0);
}
#[test]
fn any_player_can_pass_but_blocked_exits_remain_solid() {
    let mut w = world();
    w.actors[0].character = 0;
    w.actors[1].active = true;
    w.actors[1].human = true;
    w.actors[1].character = 3;
    w.actors[1].hp = MAX_HP;
    w.actors[1].pos = V3::new(25.0, 0.0, 0.0);
    w.actors[1].portals.ends = pair();
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
        42,
    );
    assert_eq!(w.actors[0].portals.transits, 1);
    let mut w = world();
    w.actors[0].portals.ends = pair();
    w.map.push(Block {
        center: V3::new(12.0, 1.0, 3.0),
        size: V3::new(2.0, 2.0, 1.0),
        ..wall(0.0, 0.0)
    });
    tick(
        &mut w,
        Input {
            forward: 1.0,
            ..Default::default()
        },
        60,
    );
    assert_eq!(w.actors[0].portals.transits, 0);
}
#[test]
fn falling_into_floor_preserves_launch_speed_through_wall() {
    let mut w = world();
    w.actors[0].portals.ends = [
        Some(Portal {
            center: V3::default(),
            face: 2,
        }),
        pair()[1],
    ];
    w.actors[0].pos = V3::new(0.0, 2.0, 0.0);
    w.actors[0].vy = -12.0;
    tick(&mut w, Input::default(), 12);
    let a = &w.actors[0];
    assert_eq!(a.portals.transits, 1);
    assert!(a.portals.momentum.z < -12.0);
    assert!(a.pos.z < 3.3);
}
#[test]
fn prediction_matches_authority_across_passage_and_pending_aim() {
    let mut w = world();
    w.actors[0].portals.ends = pair();
    w.actors[0].pos.z = 3.3;
    let mut motion = ClientMotion::default();
    motion.observe(&w, 0, 0);
    for seq in 1..=9 {
        let input = Input {
            forward: 1.0,
            yaw: 0.01 * seq as f32,
            ..Default::default()
        };
        motion.advance(&w, seq, input);
        tick(&mut w, input, 1);
        let predicted = motion.actor(&w, 0, 0);
        assert!((predicted.pos - w.actors[0].pos).length() < 0.0001);
        assert!((predicted.yaw - w.actors[0].yaw).abs() < 0.0001);
    }
    assert_eq!(w.actors[0].portals.transits, 1);
}
#[test]
fn transforms_are_reversible_for_every_surface_orientation() {
    for a in 0..6 {
        for b in 0..6 {
            let entry = Portal {
                center: V3::new(2.0, 3.0, 4.0),
                face: a,
            };
            let exit = Portal {
                center: V3::new(-5.0, 9.0, 12.0),
                face: b,
            };
            let p = V3::new(1.1, 6.2, -2.0);
            assert!((exit.point_to(entry, entry.point_to(exit, p)) - p).length() < 0.0001);
            assert!((entry.vector_to(exit, p).length() - p.length()).abs() < 0.0001);
        }
    }
}

#[test]
fn death_disconnect_and_character_swap_remove_owned_portals() {
    for mode in 0..3 {
        let mut w = world();
        w.actors[0].portals.ends = pair();
        match mode {
            0 => {
                w.actors[0].hp = 0;
                w.actors[0].respawn = 2.0;
            }
            1 => w.actors[0].active = false,
            _ => {
                w.phase = Phase::Lobby;
                assert_eq!(
                    w.select_character(0, 7),
                    trickcal_shooting_balddagu::roster::SelectionResult::Accepted
                );
            }
        }
        tick(&mut w, Input::default(), 1);
        assert_eq!(w.actors[0].portals.ends, [None; 2]);
    }
}

#[test]
fn overhead_shot_cannot_place_past_a_sealed_map_boundary() {
    let boundary = Block {
        center: V3::new(5.0, 6.0, 0.0),
        size: V3::new(1.0, 12.0, 80.0),
        phaseable: false,
        color: [0; 3],
    };
    assert!(portal::placement(
        &[boundary],
        V3::new(0.0, 30.0, 0.0),
        V3::new(1.0, -1.0, 0.0).normalized()
    )
    .is_none());
}
