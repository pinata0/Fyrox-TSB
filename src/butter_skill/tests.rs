use super::*;
use crate::game::{Block, Phase as MatchPhase, PLAYERS};
fn world() -> World {
    let mut w = World::default();
    w.start();
    w.phase = MatchPhase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.active = i == 0;
        a.human = true;
        a.shield = 0.0;
        a.hp = 100;
        a.pos = V3::new(50.0 + i as f32 * 3.0, 0.0, 50.0);
    }
    let a = &mut w.actors[0];
    a.character = 11;
    a.ammo = crate::kits::kit(11).magazine;
    a.pos = V3::default();
    w
}
fn tick(w: &mut World, input: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[0] = input;
    w.tick(inputs);
}
fn wait(w: &mut World, n: usize) {
    for _ in 0..n {
        tick(w, Input::default());
    }
}
#[test]
fn hold_never_autofires_and_release_fires_without_consuming_ammo() {
    let mut w = world();
    for _ in 0..180 {
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
    }
    assert_eq!(w.actors[0].ammo, 0);
    assert!(w.projectiles.is_empty());
    assert_eq!(w.actors[0].combat.butter.phase, Phase::Draw);
    tick(&mut w, Input::default());
    assert_eq!(w.actors[0].ammo, 0);
    assert_eq!(w.actors[0].shot, 1);
    assert_eq!(w.projectiles.len(), 1);
    assert_eq!(w.projectiles[0].kind, ProjectileKind::Stone);
    wait(&mut w, 90);
    assert_eq!(w.actors[0].shot, 1);
}
#[test]
fn repeated_shots_recover_in_point_two_seconds_without_reloading() {
    let mut w = world();
    for shot in 1..=24 {
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
        assert_eq!(w.actors[0].combat.butter.phase, Phase::Draw);
        tick(&mut w, Input::default());
        assert_eq!(w.actors[0].shot, shot);
        assert_eq!(w.actors[0].combat.butter.phase, Phase::Release);
        assert_eq!(w.actors[0].ammo, 0);
        assert_eq!(w.actors[0].reload, 0.0);
        wait(&mut w, 12);
        assert_eq!(w.actors[0].combat.butter.phase, Phase::Off);
    }
    tick(
        &mut w,
        Input {
            reload: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].reload, 0.0);
}
#[test]
fn quick_tap_fires_and_reload_stun_and_death_cancel_draw() {
    for cancel in 0..4 {
        let mut w = world();
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
        match cancel {
            1 => w.actors[0].combat.stun = 1.0,
            2 => w.actors[0].hp = 0,
            _ => {}
        }
        tick(
            &mut w,
            Input {
                reload: cancel == 3,
                ..Default::default()
            },
        );
        assert_eq!(w.actors[0].shot, if cancel == 0 { 1 } else { 0 });
        assert_ne!(w.actors[0].combat.butter.phase, Phase::Draw);
    }
}
#[test]
fn hover_uses_real_height_and_only_then_prevents_push() {
    let mut w = world();
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    assert_eq!(w.actors[0].combat.butter.phase, Phase::Skill);
    assert!(w.projectiles.is_empty());
    wait(&mut w, 40);
    assert!(
        (w.actors[0].pos.y - 0.85).abs() < 0.04,
        "{}",
        w.actors[0].pos.y
    );
    let pos = w.actors[0].pos;
    w.push(0, V3::new(1.0, 0.0, 0.0), 3.0);
    assert_eq!(w.actors[0].pos, pos);
    wait(&mut w, 25);
    assert!(w.projectiles.is_empty());
    tick(&mut w, Input::default());
    assert_eq!(w.projectiles.len(), 1);
    assert_eq!(w.projectiles[0].bounces, 6);
    wait(&mut w, 12);
    let x = w.actors[0].pos.x;
    w.push(0, V3::new(1.0, 0.0, 0.0), 3.0);
    assert!(w.actors[0].pos.x > x + 2.9);
    wait(&mut w, 120);
    assert!(w.actors[0].pos.y < 0.01);
}
#[test]
fn ceiling_collision_and_interrupt_do_not_leave_hover_stuck() {
    let mut w = world();
    w.map.push(Block {
        center: V3::new(0.0, 2.15, 0.0),
        size: V3::new(8.0, 0.2, 8.0),
        color: [100; 3],
        phaseable: false,
    });
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    wait(&mut w, 40);
    assert!(w.actors[0].pos.y < 0.30);
    w.actors[0].combat.stun = 1.0;
    wait(&mut w, 120);
    assert_eq!(w.actors[0].combat.butter.phase, Phase::Off);
    assert!(w.actors[0].pos.y < 0.01);
    assert_eq!(w.actors[0].shot, 0);
}
#[test]
fn ball_reflects_exactly_six_times_without_losing_speed() {
    let mut w = world();
    for x in [-1.0, 1.0] {
        w.map.push(Block {
            center: V3::new(x, 1.0, 0.0),
            size: V3::new(0.1, 3.0, 4.0),
            color: [100; 3],
            phaseable: false,
        });
    }
    let mut p = Projectile::visual(10, 0, ProjectileKind::Ricochet, V3::new(0.0, 1.0, 0.0));
    p.character = 11;
    p.velocity = V3::new(30.0, 0.0, 0.0);
    p.bounces = 6;
    p.life = 4.0;
    let mut reflected = 0;
    let mut last = 30.0_f32;
    for _ in 0..120 {
        w.advance_butter_ball(&mut p);
        if p.velocity.x.signum() != last.signum() {
            reflected += 1;
            last = p.velocity.x;
        }
        assert!((p.velocity.length() - 30.0).abs() < 0.001);
        if p.life <= 0.0 {
            break;
        }
    }
    assert_eq!(reflected, 6);
    assert_eq!(p.life, 0.0);
    assert_eq!(p.bounces, 0);
}
#[test]
fn a_player_bounce_and_a_wall_share_budget_and_return_hits_are_allowed() {
    let mut w = world();
    w.actors[6].active = true;
    w.actors[6].character = 0;
    w.actors[6].pos = V3::new(1.5, 0.0, 0.0);
    w.map.push(Block {
        center: V3::new(-1.0, 1.0, 0.0),
        size: V3::new(0.1, 3.0, 4.0),
        color: [100; 3],
        phaseable: false,
    });
    let mut p = Projectile::visual(10, 0, ProjectileKind::Ricochet, V3::new(0.0, 1.0, 0.0));
    p.character = 11;
    p.damage = 14.0;
    p.class = AttackClass::Skill;
    p.velocity = V3::new(30.0, 0.0, 0.0);
    p.bounces = 6;
    p.life = 4.0;
    for _ in 0..16 {
        w.advance_butter_ball(&mut p);
        if p.life <= 0.0 {
            break;
        }
    }
    assert!(p.bounces <= 3);
    assert!(
        w.actors[6].hp <= 72,
        "return hit missing: {}",
        w.actors[6].hp
    );
}
#[test]
fn ultimate_releases_on_source_event_and_blast_stuns_every_enemy_only_once() {
    let mut w = world();
    w.actors[0].combat.ultimate = 100.0;
    tick(
        &mut w,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    wait(&mut w, 149);
    assert!(w.projectiles.is_empty());
    tick(&mut w, Input::default());
    assert_eq!(w.projectiles.len(), 1);
    assert_eq!(w.projectiles[0].damage, 80.0);
    for i in [1, 6, 7, 8] {
        let a = &mut w.actors[i];
        a.active = true;
        a.character = 0;
        a.pos = V3::new(if i == 8 { 8.0 } else { i as f32 * 0.15 }, 0.0, 6.0);
    }
    let mut p = w.projectiles[0].clone();
    p.pos = V3::new(0.0, 1.0, 6.0);
    w.projectiles.clear();
    w.explode_projectile(&p);
    for i in [6, 7] {
        assert!(w.actors[i].hp <= 30);
        assert_eq!(w.actors[i].combat.stun, 3.0);
    }
    for i in [1, 8] {
        assert_eq!(w.actors[i].hp, 100);
        assert_eq!(w.actors[i].combat.stun, 0.0);
    }
    assert_eq!(w.projectiles[0].kind, ProjectileKind::BoulderBlast);
    let hp = w.actors[6].hp;
    wait(&mut w, 40);
    assert_eq!(w.actors[6].hp, hp);
    assert!(w.projectiles.is_empty());
}
#[test]
fn wire_clocks_and_source_events_agree() {
    for phase in [
        Phase::Off,
        Phase::Draw,
        Phase::Release,
        Phase::Skill,
        Phase::Ultimate,
    ] {
        for fire_down in [false, true] {
            let n = (phase.duration() / DT).ceil() as u32;
            for tick in 0..n.max(1) {
                let s = State {
                    phase,
                    elapsed: tick as f32 * DT,
                    fire_down,
                };
                let decoded = State::unpack(phase, s.pack()).unwrap();
                assert_eq!(decoded.phase, phase);
                assert_eq!(decoded.fire_down, fire_down);
                assert!((decoded.elapsed - s.elapsed).abs() < 0.001);
            }
        }
    }
    assert!(State::unpack(Phase::Skill, 1 << 31).is_none());
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../../assets/characters/skill/butter.json")).unwrap();
    for (clip, release) in [(1, SKILL_RELEASE), (2, ULTIMATE_RELEASE)] {
        assert!(data["clips"][clip]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["name"] == "Event"
                && (e["time"].as_f64().unwrap() as f32 - release).abs() < 0.0001));
    }
}

#[test]
fn interruption_cancels_both_delayed_skill_shots() {
    for phase in [Phase::Skill, Phase::Ultimate] {
        let mut w = world();
        w.start_butter(0, phase);
        wait(&mut w, 20);
        w.actors[0].combat.stun = 0.5;
        wait(&mut w, 240);
        assert_eq!(w.actors[0].shot, 0);
        assert!(w.projectiles.is_empty());
        assert_eq!(w.actors[0].combat.butter.phase, Phase::Off);
    }
}

#[test]
fn horizontal_boulder_reaches_distant_enemies_before_detonating() {
    let mut w = world();
    w.actors[6].active = true;
    w.actors[6].character = 0;
    w.actors[6].pos = V3::new(0.0, 0.0, 35.0);
    w.start_butter(0, Phase::Ultimate);
    wait(&mut w, 151);
    assert_eq!(w.actors[6].hp, 100);
    assert!(w
        .projectiles
        .iter()
        .any(|p| p.kind == ProjectileKind::Boulder));
    wait(&mut w, 60);
    assert!(w.actors[6].hp <= 30);
    assert!(w.actors[6].combat.stun > 2.5);
}

#[test]
fn horizontal_boulder_lands_farther_away_but_nearby_walls_still_stop_it() {
    for obstacle in [false, true] {
        let mut w = world();
        if obstacle {
            w.map.push(Block {
                center: V3::new(0.0, 1.0, 8.0),
                size: V3::new(6.0, 4.0, 0.2),
                color: [100; 3],
                phaseable: false,
            });
        }
        w.start_butter(0, Phase::Ultimate);
        let mut impact = None;
        for _ in 0..270 {
            tick(&mut w, Input::default());
            if let Some(p) = w
                .projectiles
                .iter()
                .find(|p| p.kind == ProjectileKind::BoulderBlast)
            {
                impact = Some(p.pos);
                break;
            }
        }
        let impact = impact.expect("boulder must explode at physical contact");
        if obstacle {
            assert!((7.0..8.0).contains(&impact.z));
        } else {
            assert!(
                (45.0..65.0).contains(&impact.z),
                "horizontal reach: {}",
                impact.z
            );
        }
        println!(
            "BUTTER_BOULDER_REACH: wall={obstacle} distance={:.2}m",
            impact.z
        );
    }
}

#[test]
fn slowed_hover_reaches_the_same_height_and_prediction_tracks_authority() {
    let mut w = world();
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    let mut predicted = w.actors[0].clone();
    for _ in 0..60 {
        crate::movement::step_actor(&w.map, &mut predicted, Input::default());
        advance_actor(&mut predicted, Input::default());
        tick(&mut w, Input::default());
        assert!((predicted.pos - w.actors[0].pos).length() < 0.001);
    }
    let mut a = world().actors[0].clone();
    a.combat.butter.phase = Phase::Skill;
    a.combat.temporal = 0.5;
    for _ in 0..110 {
        crate::movement::step_actor(&[], &mut a, Input::default());
        advance_actor(&mut a, Input::default());
    }
    assert!((a.pos.y - 0.85).abs() < 0.04, "slowed lift: {}", a.pos.y);
}
