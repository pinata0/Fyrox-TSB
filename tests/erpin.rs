use trickcal_shooting_balddagu::movement::WALK_SPEED;
use trickcal_shooting_balddagu::{
    combat::ProjectileKind,
    erpin::*,
    game::*,
    motion::{Motion, MotionClock},
};
fn fixture() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.human = true;
        a.active = i == 0;
        a.shield = 0.0;
    }
    let a = &mut w.actors[0];
    a.character = 2;
    a.pos = V3::default();
    a.hp = 40;
    a.cooldown = 0.0;
    a.ammo = 20;
    w
}
fn tick(w: &mut World, input: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = input;
    w.tick(inputs);
}
fn eat() -> Input {
    Input {
        alt_fire: true,
        ..Default::default()
    }
}
#[test]
fn held_cake_roots_wasd_and_heals_only_on_four_bites() {
    let mut w = fixture();
    tick(&mut w, eat());
    let pos = w.actors[0].pos;
    let eye = w.actors[0].eye();
    for frame in 1..=100 {
        tick(
            &mut w,
            Input {
                forward: 1.0,
                strafe: 1.0,
                ..eat()
            },
        );
        let bites = BITE_TIMES
            .iter()
            .filter(|&&t| frame as f32 * DT + 0.00001 >= t)
            .count();
        assert_eq!(w.actors[0].hp, 40 + bites as u16 * 9, "frame {frame}");
        if frame < 100 {
            assert_eq!(w.actors[0].pos, pos);
        }
        if frame == 20 {
            assert!(eye.y - w.actors[0].eye().y > 0.23);
        }
    }
    assert!(!w.actors[0].combat.erpin.eating);
    for _ in 0..100 {
        tick(&mut w, eat());
    }
    assert_eq!(w.actors[0].hp, 76);
    assert!(w.projectiles.is_empty());
}
#[test]
fn every_cancel_action_releases_immediately_and_drops_remaining_cake_once() {
    for input in [
        Input::default(),
        Input {
            fire: true,
            ..eat()
        },
        Input {
            jump: true,
            ..eat()
        },
        Input {
            reload: true,
            ..eat()
        },
        Input {
            ability: true,
            ..eat()
        },
        Input {
            ultimate: true,
            ..eat()
        },
        Input {
            phase_shift: true,
            ..eat()
        },
        Input {
            dance: true,
            ..eat()
        },
        Input {
            crouch: true,
            ..eat()
        },
    ] {
        let mut w = fixture();
        tick(&mut w, eat());
        for _ in 0..35 {
            tick(&mut w, eat());
        }
        assert_eq!(w.actors[0].hp, 58);
        tick(
            &mut w,
            Input {
                forward: 1.0,
                ..input
            },
        );
        assert!(!w.actors[0].combat.erpin.eating);
        assert!(w.actors[0].pos.z > 0.0, "cancel resumes movement");
        let cake = w
            .projectiles
            .iter()
            .find(|p| p.kind.cake_remaining().is_some())
            .unwrap();
        assert_eq!(cake.kind, ProjectileKind::Cake2);
        let start = cake.pos;
        for _ in 0..50 {
            tick(&mut w, input);
        }
        let cakes: Vec<_> = w
            .projectiles
            .iter()
            .filter(|p| p.kind.cake_remaining().is_some())
            .collect();
        assert_eq!(cakes.len(), 1);
        assert!(cakes[0].pos.y < start.y);
        assert_eq!(w.actors[0].hp, 58);
        for _ in 0..150 {
            tick(&mut w, Input::default());
        }
        assert!(w
            .projectiles
            .iter()
            .all(|p| p.kind.cake_remaining().is_none()));
    }
}
#[test]
fn death_stun_and_input_loss_cancel_even_before_first_bite() {
    for kind in 0..3 {
        let mut w = fixture();
        tick(&mut w, eat());
        if kind == 0 {
            w.actors[0].hp = 0;
            w.actors[0].respawn = 5.0;
        }
        if kind == 1 {
            w.actors[0].combat.stun = 1.0;
        }
        tick(&mut w, if kind == 2 { Input::default() } else { eat() });
        assert!(!w.actors[0].combat.erpin.eating);
        assert_eq!(
            w.projectiles
                .iter()
                .filter(|p| p.kind == ProjectileKind::Cake4)
                .count(),
            1
        );
    }
}
#[test]
fn all_five_primary_attacks_fire_at_the_source_event_while_moving() {
    let mut w = fixture();
    let mut clock = MotionClock::default();
    clock.observe(&w.actors[0], DT, 0, true);
    for n in 1..=5 {
        w.actors[0].cooldown = 0.0;
        tick(
            &mut w,
            Input {
                fire: true,
                forward: 1.0,
                ..Default::default()
            },
        );
        assert_eq!(w.actors[0].shot, n);
        assert!(w.actors[0].combat.erpin.shot_delay > 0.0);
        assert_eq!(
            clock.observe(&w.actors[0], DT, w.tick, true).motion,
            Motion::Attack
        );
        let count = w.projectiles.len();
        for _ in 0..10 {
            tick(
                &mut w,
                Input {
                    forward: 1.0,
                    ..Default::default()
                },
            );
        }
        assert_eq!(w.projectiles.len(), count + 1);
    }
    assert!((w.actors[0].pos.z - 55.0 * WALK_SPEED * DT).abs() < 0.001);
    assert_eq!(w.actors[0].hp, 40);
    assert_eq!(w.actors[0].ammo, 15);
}
#[test]
fn cancel_does_not_leave_eat_motion_and_late_observers_use_authoritative_age() {
    let mut w = fixture();
    tick(&mut w, eat());
    for _ in 0..40 {
        tick(&mut w, eat());
    }
    let mut clock = MotionClock::default();
    let p = clock.observe(&w.actors[0], DT, w.tick, true);
    assert_eq!(p.motion, Motion::ErpinEat);
    assert_eq!(p.age, w.actors[0].combat.erpin.elapsed);
    assert!(p.locomotion.is_none());
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
    );
    assert_eq!(
        clock.observe(&w.actors[0], DT, w.tick, true).motion,
        Motion::Attack
    );
}
#[test]
fn prediction_shares_lock_cancel_and_does_not_heal() {
    let mut w = fixture();
    let mut a = w.actors[0].clone();
    advance(&mut a, eat(), true);
    trickcal_shooting_balddagu::movement::step_actor(
        &[],
        &mut a,
        Input {
            forward: 1.0,
            ..eat()
        },
    );
    assert_eq!(a.pos, V3::default());
    for _ in 0..50 {
        advance(&mut a, eat(), true);
    }
    assert_eq!(a.hp, 40);
    advance(&mut a, Input::default(), true);
    trickcal_shooting_balddagu::movement::step_actor(
        &[],
        &mut a,
        Input {
            forward: 1.0,
            ..Default::default()
        },
    );
    assert!(a.pos.z > 0.0);
    w.actors[0].hp = 119;
    tick(&mut w, eat());
    for _ in 0..80 {
        tick(&mut w, eat());
    }
    assert_eq!(w.actors[0].hp, MAX_HP);
}

#[test]
fn switching_hero_or_leaving_drops_cake_without_carrying_heal_state() {
    for leaving in [false, true] {
        let mut w = fixture();
        tick(&mut w, eat());
        for _ in 0..35 {
            tick(&mut w, eat());
        }
        if leaving {
            w.clear_slot(0);
        } else {
            w.phase = Phase::Lobby;
            assert_eq!(
                w.select_character(0, 1),
                trickcal_shooting_balddagu::roster::SelectionResult::Accepted
            );
        }
        assert!(!w.actors[0].combat.erpin.eating);
        assert_eq!(
            w.projectiles
                .iter()
                .filter(|p| p.kind == ProjectileKind::Cake2)
                .count(),
            1
        );
    }
}
