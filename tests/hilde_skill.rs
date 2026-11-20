use trickcal_shooting_balddagu::{
    combat::ProjectileKind,
    game::*,
    hilde_skill::{self as h, Phase as Action},
    impact::{ImpactKind, IMPACT_SECONDS},
};
fn world() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.active = true;
        a.human = true;
        a.character = 13;
        a.shield = 0.0;
        a.hp = 40;
        a.pos = V3::new(40.0 + i as f32 * 2.0, 0.0, 45.0);
    }
    w.actors[0].pos = V3::default();
    w.actors[6].pos = V3::new(0.0, 0.0, 15.0);
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
fn case_heals_every_nearby_ally_evenly_for_two_seconds() {
    let mut w = world();
    for i in 1..6 {
        w.actors[i].pos = V3::new(i as f32, 0.0, 1.0);
    }
    w.actors[6].pos = V3::new(0.0, 0.0, 5.0);
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    wait(&mut w, 82);
    assert!(w.actors[..6].iter().all(|a| a.hp == 40), "no upfront heal");
    wait(&mut w, 60);
    assert!(
        w.actors[..6].iter().all(|a| a.hp == 60),
        "half heal at one second"
    );
    wait(&mut w, 60);
    assert!(
        w.actors[..6].iter().all(|a| a.hp == 80),
        "all six get full 40"
    );
    assert_eq!(w.actors[6].hp, 40);
    assert_eq!(w.actors[0].combat.hilde.phase, Action::Off);
    wait(&mut w, 60);
    assert_eq!(w.actors[1].hp, 80);
}
#[test]
fn healing_respects_cover_range_death_and_cancellation() {
    let mut w = world();
    w.actors[1].pos = V3::new(0.0, 0.0, 4.0);
    w.actors[2].pos = V3::new(10.0, 0.0, 0.0);
    w.actors[3].pos = V3::new(1.0, 0.0, 0.0);
    w.actors[3].hp = 0;
    w.actors[3].respawn = 20.0;
    w.map.push(Block {
        center: V3::new(0.0, 1.5, 2.0),
        size: V3::new(3.0, 3.0, 0.5),
        color: [100, 100, 100],
        phaseable: true,
    });
    tick(
        &mut w,
        Input {
            ability: true,
            ..Default::default()
        },
    );
    wait(&mut w, 143);
    assert_eq!(w.actors[1].hp, 40);
    assert_eq!(w.actors[2].hp, 40);
    assert_eq!(w.actors[3].hp, 0);
    let hp = w.actors[0].hp;
    w.actors[0].combat.stun = 1.0;
    wait(&mut w, 120);
    assert_eq!(w.actors[0].hp, hp);
    assert!(!w.actors[0].combat.hilde.active());
}
#[test]
fn launcher_requires_fire_tracks_new_aim_and_expires_without_ammo_use() {
    let mut w = world();
    w.actors[0].combat.ultimate = 100.0;
    let ammo = w.actors[0].ammo;
    tick(
        &mut w,
        Input {
            ultimate: true,
            ..Default::default()
        },
    );
    wait(&mut w, 126);
    assert!(w.projectiles.is_empty());
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
    );
    assert_eq!(w.projectiles.len(), 1);
    assert_eq!(w.projectiles[0].kind, ProjectileKind::MedicalGrenade);
    let shots = w.actors[0].shot;
    wait(&mut w, 20);
    assert_eq!(w.actors[0].shot, shots);
    tick(
        &mut w,
        Input {
            fire: true,
            yaw: 1.0,
            ..Default::default()
        },
    );
    assert!(w.projectiles.last().unwrap().velocity.x > 10.0);
    assert_eq!(w.actors[0].ammo, ammo);
    wait(&mut w, 600);
    assert!(!w.actors[0].combat.hilde.active());
    assert!(w.projectiles.is_empty());
}
#[test]
fn grenades_heal_allies_damage_enemies_and_detonate_once() {
    for ally in [true, false] {
        let mut w = world();
        let target = if ally { 1 } else { 6 };
        w.actors[target].pos = V3::new(0.0, 0.0, 4.0);
        w.actors[0].combat.hilde.phase = Action::Launcher;
        w.actors[0].combat.hilde.elapsed = h::EQUIP_SECONDS;
        tick(
            &mut w,
            Input {
                fire: true,
                ..Default::default()
            },
        );
        wait(&mut w, 20);
        assert!(w.projectiles.is_empty());
        let hp = w.actors[target].hp;
        if ally {
            assert_eq!(hp, 56);
            assert_eq!(w.actors[0].stats.healing, 16.0);
        } else {
            assert!(hp < 40);
        }
        wait(&mut w, 30);
        assert_eq!(w.actors[target].hp, hp);
    }
}
#[test]
fn launcher_cannot_fire_during_stun_or_portal_input() {
    let mut w = world();
    w.actors[0].combat.hilde.phase = Action::Launcher;
    w.actors[0].combat.hilde.elapsed = h::EQUIP_SECONDS;
    tick(
        &mut w,
        Input {
            fire: true,
            phase_shift: true,
            ..Default::default()
        },
    );
    assert!(w.projectiles.is_empty());
    w.actors[0].combat.stun = 1.0;
    tick(
        &mut w,
        Input {
            fire: true,
            ..Default::default()
        },
    );
    assert!(w.projectiles.is_empty());
    assert!(!w.actors[0].combat.hilde.active());
}

#[test]
fn full_barrage_fires_thirty_rounds_and_keeps_valid_wire_state_every_tick() {
    let mut w=world();w.actors[0].combat.ultimate=100.0;
    w.actors[0].combat.attacks=2;
    tick(&mut w,Input{ultimate:true,..Default::default()});
    let first=w.actors[0].shot;
    for _ in 0..490 {
        tick(&mut w,Input{fire:true,..Default::default()});
        let s=w.actors[0].combat.hilde;
        assert!(h::State::unpack(s.phase,s.pack()).is_some(),"{:?} {}",s.phase,s.elapsed);
    }
    assert_eq!(w.actors[0].shot-first,30);
    assert_eq!(w.actors[0].combat.attacks,2,"ultimate preserves normal syringe combo");
    assert_eq!(w.actors[0].combat.hilde.phase,Action::End);
}

#[test]
fn grenade_splash_cannot_heal_or_damage_through_cover() {
    let mut w=world();w.actors[1].pos=V3::new(-0.6,0.0,3.0);w.actors[6].pos=V3::new(0.6,0.0,3.0);
    w.map.push(Block{center:V3::new(0.0,1.5,2.0),size:V3::new(5.0,3.0,0.5),color:[100;3],phaseable:true});
    w.actors[0].combat.hilde.phase=Action::Launcher;w.actors[0].combat.hilde.elapsed=h::EQUIP_SECONDS;
    tick(&mut w,Input{fire:true,..Default::default()});wait(&mut w,30);
    assert_eq!(w.actors[1].hp,40);assert_eq!(w.actors[6].hp,40);assert!(w.projectiles.is_empty());
}

#[test]
fn every_grenade_contact_records_one_green_explosion_at_the_contact() {
    for contact in ["wall", "ground", "platform", "ally", "enemy"] {
        let mut w = world();
        w.actors[6].pos = V3::new(40.0, 0.0, 40.0);
        let downward = matches!(contact, "ground" | "platform");
        if contact == "wall" {
            w.map.push(Block {
                center: V3::new(0.0, 1.5, 3.0),
                size: V3::new(5.0, 3.0, 0.5),
                color: [100; 3], phaseable: true,
            });
        } else if contact == "platform" {
            w.map.push(Block {
                center: V3::new(0.0, 1.0, 0.0),
                size: V3::new(10.0, 2.0, 10.0),
                color: [100; 3], phaseable: true,
            });
            w.actors[0].pos.y = 2.0;
        } else if matches!(contact, "ally" | "enemy") {
            w.actors[if contact == "ally" { 1 } else { 6 }].pos = V3::new(0.0, 0.0, 4.0);
        }
        w.actors[0].combat.hilde.phase = Action::Launcher;
        w.actors[0].combat.hilde.elapsed = h::EQUIP_SECONDS;
        tick(&mut w, Input {
            fire: true, pitch: if downward { -0.65 } else { 0.0 },
            ..Default::default()
        });
        for _ in 0..120 {
            if !w.impacts.is_empty() { break; }
            tick(&mut w, Input::default());
        }
        assert!(w.projectiles.is_empty(), "{contact}: grenade consumed");
        assert_eq!(w.impacts.len(), 1, "{contact}: exactly one explosion");
        let hit = w.impacts[0];
        assert_eq!(hit.kind, ImpactKind::MedicalExplosion, "{contact}");
        assert_eq!(hit.radius, h::GRENADE_RADIUS);
        assert!(hit.color[1] > hit.color[0] && hit.color[1] > hit.color[2]);
        match contact {
            "wall" => assert!((2.5..2.75).contains(&hit.position.z)),
            "ground" => assert!((0.05..0.10).contains(&hit.position.y)),
            "platform" => assert!((2.1..2.2).contains(&hit.position.y)),
            _ => assert!((3.0..4.0).contains(&hit.position.z)),
        }
        wait(&mut w, 60);
        assert_eq!(w.impacts, vec![hit], "{contact}: no repeated detonation");
        wait(&mut w, (IMPACT_SECONDS / DT).ceil() as usize);
        assert!(w.impacts.is_empty(), "{contact}: event expires");
    }
}
