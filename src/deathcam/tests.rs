use crate::{combat::AttackClass, deathcam::DeathCamera, game::*};

fn arena() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    for a in &mut w.actors {
        a.human = true;
        a.shield = 0.0;
        // Combat fixtures stand outside the active spawn sanctuary.
        a.pos.z += 10.0;
    }
    w
}

fn kill(w: &mut World, source: usize, target: usize) {
    assert!(w.damage_hit(
        source,
        target,
        1000.0,
        AttackClass::Basic,
        w.actors[source].character
    ));
    assert!(!w.actors[target].alive());
}

#[test]
fn lethal_source_survives_selection_and_clears_on_respawn_and_restart() {
    let mut w = arena();
    w.damage_hit(6, 0, 1.0, AttackClass::Skill, 3);
    assert_eq!(w.actors[0].killed_by, None);
    kill(&mut w, 7, 0);
    assert_eq!(w.actors[0].killed_by, Some(7));
    assert!(!w.damage_hit(6, 0, 1000.0, AttackClass::Basic, 3));
    w.select_character(0, 4);
    assert_eq!(w.actors[0].killed_by, Some(7));
    w.actors[0].respawn = DT;
    w.tick([Input::default(); PLAYERS]);
    assert!(w.actors[0].alive());
    assert_eq!(w.actors[0].killed_by, None);
    w.actors[0].shield = 0.0;
    w.actors[0].pos = POINTS[0];
    kill(&mut w, 6, 0);
    w.start();
    assert!(w.actors.iter().all(|a| a.killed_by.is_none()));
}

#[test]
fn bella_revive_does_not_start_death_sequence() {
    let mut w = arena();
    w.actors[0].character = 9;
    w.actors[0].combat.revive_armed = true;
    w.damage_hit(6, 0, 1000.0, AttackClass::Skill, 3);
    assert!(w.actors[0].alive());
    assert_eq!(w.actors[0].killed_by, None);
    assert!(DeathCamera::default().update(&w, 0, DT).is_none());
}

fn record(w: &mut World, c: &mut DeathCamera, n: u64) {
    for i in 0..=n {
        w.tick = i;
        w.actors[6].pos = V3::new(i as f32 * 0.01, 0.0, -8.0);
        w.actors[6].yaw = i as f32 * 0.002;
        w.actors[6].pitch = i as f32 * 0.001;
        c.update(w, 0, DT);
    }
}
fn replay(c: &mut DeathCamera, w: &World) {
    c.update(w, 0, c.death_duration() + super::DEATH_HOLD + 0.001);
    assert_eq!(c.stage, super::Stage::Replay);
}
#[test]
fn every_hero_finishes_its_own_death_before_replay() {
    for id in 0..15 {
        let mut w = arena();
        w.actors[0].character = id;
        let mut c = DeathCamera::default();
        record(&mut w, &mut c, 180);
        w.tick += 1;
        kill(&mut w, 6, 0);
        let view = c.update(&w, 0, DT).unwrap();
        assert_eq!(c.stage, super::Stage::TopDown);
        assert_eq!(view.target, 0);
        assert!(view.position.y > w.actors[0].pos.y + 4.0);
        assert!(view.pitch < -1.2);
        c.update(&w, 0, crate::motion::death_seconds(id) - 0.01);
        assert_eq!(c.stage, super::Stage::TopDown);
        c.update(&w, 0, 0.01 + super::DEATH_HOLD + 0.001);
        assert_eq!(c.stage, super::Stage::Replay);
        assert_eq!(c.view.unwrap().target, 6);
        assert!(c.scene().unwrap().actors[0].alive());
    }
}
#[test]
fn playback_rewinds_all_actors_shots_projectiles_and_stops_at_lethal_tick() {
    use crate::combat::{Projectile, ProjectileKind};
    let mut w = arena();
    let mut c = DeathCamera::default();
    for tick in 0..=180 {
        w.tick = tick;
        w.actors[6].pos.x = tick as f32 * 0.02;
        w.actors[1].pos.x = -(tick as f32) * 0.03;
        w.actors[6].shot = if tick >= 120 { 1 } else { 0 };
        w.actors[6].shot_end = w.actors[0].eye();
        w.projectiles = if (110..130).contains(&tick) {
            vec![Projectile::visual(
                77,
                6,
                ProjectileKind::Water,
                V3::new(tick as f32, 2.0, 0.0),
            )]
        } else {
            vec![]
        };
        c.update(&w, 0, DT);
    }
    w.tick += 1;
    kill(&mut w, 6, 0);
    c.update(&w, 0, DT);
    // Continue the live match somewhere entirely different while the captured clip plays.
    w.actors[6].pos.x = 99.0;
    w.actors[1].pos.x = 88.0;
    w.actors[6].shot = 99;
    replay(&mut c, &w);
    assert!(c.scene().unwrap().actors[6].pos.x < 0.1);
    assert!(c.scene().unwrap().actors[1].pos.x > -0.1);
    c.update(&w, 0, 119.0 * DT);
    let scene = c.scene().unwrap();
    assert_eq!(scene.actors[6].shot, 1);
    assert_eq!(scene.projectiles[0].id, 77);
    assert!((scene.projectiles[0].pos.x - 120.0).abs() < 0.1);
    c.update(&w, 0, 3.0);
    assert_eq!(c.stage, super::Stage::Finished);
    assert!(!c.scene().unwrap().actors[0].alive());
    assert!((c.scene().unwrap().actors[6].pos.x - 3.6).abs() < 0.001);
    assert_eq!(w.actors[6].pos.x, 99.0);
    assert_eq!(w.actors[1].pos.x, 88.0);
    let end = c.view;
    c.update(&w, 0, 10.0);
    assert_eq!(c.view, end);
}

#[test]
fn vivi_replay_keeps_the_recorded_sweep_after_live_direction_changes() {
    use crate::motion::Motion;
    use crate::vivi_skill::{Phase, State};
    let mut w = arena();
    w.actors[6].character = 0;
    let mut c = DeathCamera::default();
    for tick in 0..=180 {
        w.tick = tick;
        w.actors[6].shot = if tick >= 120 { 3 } else { 2 };
        w.actors[6].combat.vivi_combo = if tick >= 120 { 2 } else { 1 };
        w.actors[6].combat.vivi = State {
            phase: if tick >= 120 { Phase::Right } else { Phase::Left },
            elapsed: 0.3,
        };
        c.update(&w, 0, DT);
    }
    w.tick += 1;
    kill(&mut w, 6, 0);
    c.update(&w, 0, DT);
    w.actors[6].combat.vivi_combo = 1;
    w.actors[6].combat.vivi = State { phase: Phase::Left, elapsed: 0.1 };
    w.actors[6].shot = 4;
    replay(&mut c, &w);
    c.update(&w, 0, 119.0 * DT);
    assert_eq!(c.scene().unwrap().actors[6].combat.vivi_combo, 2);
    assert_eq!(c.poses()[6].motion, Motion::ViviRight);
    assert_eq!(c.scene().unwrap().actors[6].combat.vivi.elapsed, 0.3);
    assert!((0.299..=0.3 + DT * 1.4).contains(&c.poses()[6].age));
}
#[test]
fn aya_replay_keeps_transformation_after_live_actor_changes() {
    use crate::motion::Motion;
    let mut w = arena();
    w.actors[6].character = 4;
    let mut c = DeathCamera::default();
    for tick in 0..=180 {
        w.tick = tick;
        w.actors[6].shot = if tick >= 120 { 3 } else { 2 };
        w.actors[6].combat.aya_combo = if tick >= 120 { 3 } else { 2 };
        c.update(&w, 0, DT);
    }
    w.tick += 1;
    kill(&mut w, 6, 0);
    c.update(&w, 0, DT);
    w.actors[6].combat.aya_combo = 1;
    w.actors[6].shot = 4;
    replay(&mut c, &w);
    c.update(&w, 0, 119.0 * DT);
    assert_eq!(c.scene().unwrap().actors[6].combat.aya_combo, 3);
    assert_eq!(c.poses()[6].motion, Motion::Attack2);
    assert_eq!(crate::aya_attack::weapon(0.7, true).blade, 3);
}

#[test]
fn replay_interpolates_movement_and_attack_on_independent_clocks() {
    use crate::motion::Motion;
    let mut w = arena();
    w.actors[6].character = 4;
    let mut c = DeathCamera::default();
    for tick in 0..=180 {
        w.tick = tick;
        w.actors[6].pos = V3::new(tick as f32 * 0.04, 0.0, -8.0);
        w.actors[6].shot = if tick >= 120 { 3 } else { 2 };
        w.actors[6].combat.aya_combo = if tick >= 120 { 3 } else { 2 };
        c.update(&w, 0, DT);
    }
    w.tick += 1;
    kill(&mut w, 6, 0);
    c.update(&w, 0, DT);
    w.actors[6].character = 11;
    w.actors[6].shot = 100;
    replay(&mut c, &w);
    c.update(&w, 0, 119.5 * DT);
    let pose = c.poses()[6];
    assert_eq!(pose.motion, Motion::Attack2);
    let movement = pose.locomotion.unwrap();
    assert_eq!(movement.motion, Motion::Walk);
    assert!(movement.age > 0.0);
    let age = pose.age;
    c.update(&w, 0, 0.25 * DT);
    let later = c.poses()[6];
    assert!((later.age - age - 0.25 * DT).abs() < 0.0001);
    assert!((later.locomotion.unwrap().age - movement.age - 0.25 * DT).abs() < 0.0001);
    assert_eq!(c.scene().unwrap().actors[6].character, 4);
}
#[test]
fn killer_disconnect_death_and_hero_change_cannot_change_recorded_clip() {
    let mut w = arena();
    let mut c = DeathCamera::default();
    record(&mut w, &mut c, 180);
    w.tick += 1;
    kill(&mut w, 6, 0);
    c.update(&w, 0, DT);
    let original = w.actors[0].character;
    let killer_character = w.actors[6].character;
    w.actors[0].character = 14;
    w.actors[6].character = 13;
    w.actors[6].human = false;
    w.actors[6].hp = 0;
    c.update(&w, 0, 0.5);
    assert_eq!(c.scene().unwrap().actors[0].character, original);
    replay(&mut c, &w);
    assert_eq!(c.scene().unwrap().actors[6].character, killer_character);
    assert!(c.scene().unwrap().actors[6].alive());
    assert!(!c.view.unwrap().frozen);
}
#[test]
fn sparse_network_snapshots_interpolate_shortest_yaw_and_preserve_discrete_death() {
    let mut w = arena();
    let mut c = DeathCamera::default();
    w.actors[6].yaw = std::f32::consts::TAU - 0.02;
    w.actors[6].pos.x = 0.0;
    c.update(&w, 0, DT);
    w.tick = 6;
    w.actors[6].yaw = 0.02;
    w.actors[6].pos.x = 1.0;
    c.update(&w, 0, DT);
    w.tick = 12;
    kill(&mut w, 6, 0);
    c.update(&w, 0, DT);
    c.update(&w, 0, c.death_duration() + super::DEATH_HOLD + 3.0 * DT);
    let view = c.view.unwrap();
    assert!((view.position.x - 0.5).abs() < 0.001);
    assert!((view.yaw - std::f32::consts::TAU).abs() < 0.001);
    assert!(c.scene().unwrap().actors[0].alive());
    c.update(&w, 0, 20.0 * DT);
    assert!(!c.scene().unwrap().actors[0].alive());
}
#[test]
fn no_history_or_invalid_killer_keeps_overhead_instead_of_live_spectating() {
    for target in [None, Some(0), Some(1), Some(PLAYERS), Some(6)] {
        let mut w = arena();
        kill(&mut w, 6, 0);
        w.actors[0].killed_by = target;
        let mut c = DeathCamera::default();
        c.update(&w, 0, DT);
        c.update(&w, 0, 5.0);
        assert_eq!(c.stage, super::Stage::TopDown);
        assert_eq!(c.view.unwrap().target, 0);
    }
}
#[test]
fn respawn_round_map_phase_and_clock_reset_discard_playback() {
    let mut w = arena();
    let mut c = DeathCamera::default();
    record(&mut w, &mut c, 300);
    assert!(c.history.len() <= 182);
    assert_eq!(
        c.history.back().unwrap().tick - c.history.front().unwrap().tick,
        180
    );
    w.tick += 1;
    kill(&mut w, 6, 0);
    c.update(&w, 0, DT);
    replay(&mut c, &w);
    w.actors[0].hp = MAX_HP;
    w.actors[0].killed_by = None;
    assert!(c.update(&w, 0, DT).is_none());
    w.tick += 1;
    kill(&mut w, 7, 0);
    c.update(&w, 0, DT);
    assert_eq!(c.stage, super::Stage::TopDown);
    assert_eq!(c.killer, Some(7));
    w.phase = Phase::Intermission;
    assert!(c.update(&w, 0, DT).is_none());
    assert!(c.history.is_empty());
    w.phase = Phase::Playing;
    w.actors[0].hp = MAX_HP;
    record(&mut w, &mut c, 30);
    w.round += 1;
    c.update(&w, 0, DT);
    assert_eq!(c.history.len(), 1);
    w.tick = 0;
    c.update(&w, 0, DT);
    assert_eq!(c.history.len(), 1);
    w.map_id = crate::level::MapId::ElfCity;
    c.update(&w, 0, DT);
    assert_eq!(c.history.len(), 1);
}

#[test]
fn overhead_camera_faces_the_victim_and_stays_below_a_ceiling() {
    let mut w = arena();
    w.map.clear();
    w.actors[0].pos = V3::default();
    w.actors[0].yaw = 1.0;
    w.actors[0].hp = 0;
    w.actors[0].deaths = 1;
    w.actors[0].killed_by = Some(6);
    w.map.push(Block {
        center: V3::new(0.0, 3.0, 0.0),
        size: V3::new(20.0, 0.2, 20.0),
        color: [0; 3],
        phaseable: false,
    });
    let view = DeathCamera::default().update(&w, 0, DT).unwrap();
    assert!(view.position.y < 2.8 && view.position.y > 1.0);
    assert!((view.yaw - w.actors[0].yaw - std::f32::consts::PI).abs() < 0.001);
    assert!(view.position.x > 0.0);
}
