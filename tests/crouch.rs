use trickcal_shooting_balddagu::{game::*, movement};

fn actor() -> Actor {
    Actor {
        human: true,
        character: 3,
        ..Default::default()
    }
}

fn crouch() -> Input {
    Input {
        crouch: true,
        ..Default::default()
    }
}

fn ceiling() -> Block {
    Block {
        center: V3::new(0.0, 1.85, 0.0),
        size: V3::new(3.0, 0.6, 3.0),
        color: [0; 3],
        phaseable: true,
    }
}

#[test]
fn hold_lowers_eye_and_body_and_preserves_crouch_speed_with_shift() {
    for character in 0..15 {
        let mut a = Actor {
            character,
            dancing: true,
            ..actor()
        };
        let standing_eye = a.eye();
        movement::step_actor(
            &[],
            &mut a,
            Input {
                forward: 1.0,
                phase_shift: true,
                ..crouch()
            },
        );
        assert!(a.crouching && !a.dancing);
        assert!((a.pos.z - movement::CROUCH_SPEED * DT).abs() < 0.0001);
        assert_eq!(a.pos.y, 0.0);
        assert!((standing_eye.y - a.eye().y - CROUCH_DROP).abs() < 0.0001);
        assert!((a.collision_height() - 1.45).abs() < 0.0001);
        assert!((a.hit_height() - 1.55).abs() < 0.0001);
        movement::step_actor(&[], &mut a, Input::default());
        assert!(!a.crouching);
        assert_eq!(a.eye().y, standing_eye.y);
    }
}

#[test]
fn crouch_fits_tunnel_and_cannot_stand_until_clear() {
    for character in [3, 14] {
        let map = [ceiling()];
        let mut a = Actor {
            character,
            ..actor()
        };
        movement::step_actor(&map, &mut a, crouch());
        assert!(!movement::blocked_with_height(
            &map,
            a.pos,
            a.collision_height()
        ));
        for _ in 0..60 {
            movement::step_actor(
                &map,
                &mut a,
                Input {
                    forward: 1.0,
                    ..Default::default()
                },
            );
            assert!(a.crouching);
        }
        for _ in 0..60 {
            movement::step_actor(
                &map,
                &mut a,
                Input {
                    forward: 1.0,
                    ..Default::default()
                },
            );
        }
        assert!(!a.crouching);
        assert!(a.pos.z > 1.9);
    }
}

#[test]
fn crouched_jump_hits_ceiling_and_lands_without_clipping() {
    let map = [ceiling()];
    let mut a = actor();
    for frame in 0..45 {
        movement::step_actor(
            &map,
            &mut a,
            Input {
                jump: frame == 0,
                ..crouch()
            },
        );
        assert!(a.pos.y >= 0.0 && a.pos.y <= 0.1001);
        assert!(!movement::blocked_with_height(
            &map,
            a.pos,
            a.collision_height()
        ));
    }
    assert_eq!(a.pos.y, 0.0);
    assert_eq!(a.vy, 0.0);
}

#[test]
fn standing_cannot_enter_low_tunnel_but_crouching_can() {
    let map = [ceiling()];
    let mut standing = Actor {
        pos: V3::new(0.0, 0.0, -2.1),
        ..actor()
    };
    let mut crouching = standing.clone();
    for _ in 0..120 {
        movement::step_actor(
            &map,
            &mut standing,
            Input {
                forward: 1.0,
                ..Default::default()
            },
        );
        movement::step_actor(
            &map,
            &mut crouching,
            Input {
                forward: 1.0,
                ..crouch()
            },
        );
    }
    assert!(standing.pos.z < -1.89);
    assert!(crouching.pos.z > 0.7);
}

#[test]
fn respawn_resets_posture() {
    let mut world = World::room(Default::default(), true);
    world.phase = Phase::Playing;
    world.actors[0].crouching = true;
    world.actors[0].hp = 0;
    world.actors[0].respawn = DT;
    world.tick([Input::default(); PLAYERS]);
    assert!(world.actors[0].alive());
    assert!(!world.actors[0].crouching);
}
