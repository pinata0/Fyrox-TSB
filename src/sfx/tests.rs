use super::*;

fn arena() -> World {
    let mut w = World {
        phase: Phase::Playing,
        tick: 100,
        map: vec![],
        ..World::default()
    };
    for a in &mut w.actors {
        a.active = false;
    }
    w.actors[0] = Actor {
        active: true,
        human: true,
        pos: V3::default(),
        ..Actor::default()
    };
    w
}
fn cues(t: &mut Tracker, w: &World) -> Vec<Cue> {
    t.update(w, 0, true).iter().map(|e| e.cue).collect()
}

#[test]
fn snapshots_late_join_reset_and_hidden_frames_never_replay_old_audio() {
    let mut w = arena();
    let mut t = Tracker::default();
    w.actors[0].shot = 40;
    assert!(cues(&mut t, &w).is_empty());
    w.tick += 1;
    w.actors[0].shot += 1;
    assert_eq!(cues(&mut t, &w), vec![Cue::Fire(0)]);
    assert!(cues(&mut t, &w).is_empty());
    w.tick += 3;
    w.actors[0].shot += 4;
    assert_eq!(cues(&mut t, &w), vec![Cue::Fire(0)]);
    w.tick += 1;
    w.actors[0].shot += 1;
    assert!(t.update(&w, 0, false).is_empty());
    w.tick += 1;
    assert!(cues(&mut t, &w).is_empty());
    w.tick = 0;
    assert!(cues(&mut t, &w).is_empty());
    w.round += 1;
    w.actors[0].shot = 0;
    assert!(cues(&mut t, &w).is_empty());
    w.tick += 120;
    w.actors[0].shot = 8;
    assert!(cues(&mut t, &w).is_empty());
}

#[test]
fn skills_reload_damage_and_respawn_follow_successful_actions() {
    let mut w = arena();
    let mut t = Tracker::default();
    w.actors[0].ammo = 2;
    w.actors[0].combat.ultimate = 100.0;
    cues(&mut t, &w);
    w.tick += 1;
    w.actors[0].ability_cooldown = 10.0;
    w.actors[0].combat.ultimate = 0.0;
    w.actors[0].reload = 1.0;
    let c = cues(&mut t, &w);
    assert!(
        c.contains(&Cue::Skill(0)) && c.contains(&Cue::Ultimate(0)) && c.contains(&Cue::Reload)
    );
    w.tick += 1;
    assert!(
        cues(&mut t, &w).is_empty(),
        "held Q/E/reload must not repeat"
    );
    w.tick += 1;
    w.actors[0].reload = 0.0;
    assert!(
        cues(&mut t, &w).is_empty(),
        "cancelled reload must not complete"
    );
    w.tick += 1;
    w.actors[0].reload = 1.0;
    cues(&mut t, &w);
    w.tick += 1;
    w.actors[0].reload = 0.0;
    w.actors[0].ammo = 20;
    assert_eq!(cues(&mut t, &w), vec![Cue::ReloadDone]);
    w.tick += 1;
    w.actors[0].hp -= 5;
    w.actors[0].hit = 0.2;
    assert_eq!(cues(&mut t, &w), vec![Cue::Hit, Cue::Hurt]);
    w.tick += 1;
    w.actors[0].hp -= 5;
    assert!(cues(&mut t, &w).is_empty(), "rapid damage is rate limited");
    w.tick += 1;
    w.actors[0].hp = 0;
    assert_eq!(cues(&mut t, &w), vec![Cue::Death]);
    w.tick += 1;
    w.actors[0].hp = MAX_HP;
    assert_eq!(cues(&mut t, &w), vec![Cue::Respawn]);
}

#[test]
fn cast_voices_identify_the_caster_and_ignore_rejected_or_repeated_actions() {
    let mut w = arena();
    w.actors[1] = Actor { active: true, pos: V3::new(3.0, 0.0, 0.0),
                         ..Actor::default() };
    w.actors[1].combat.ultimate = 100.0;
    let mut tracker = Tracker::default();
    tracker.update(&w, 0, true);
    w.use_skill(0);
    assert!(w.use_ultimate(1));
    w.tick += 1;
    let events = tracker.update(&w, 0, true);
    let local = events.iter().find(|e| e.cue == Cue::Skill(0)).unwrap();
    assert_eq!(local.speaker, Some(0));
    assert!(local.position.is_none());
    let remote = events.iter().find(|e| e.cue == Cue::Ultimate(0)).unwrap();
    assert_eq!(remote.speaker, Some(1));
    assert_eq!(remote.position, Some(w.actors[1].eye()));
    // Held input, cooldown rejection, and duplicate snapshots cannot speak twice.
    w.use_skill(0);
    assert!(!w.use_ultimate(1));
    w.tick += 1;
    assert!(!tracker.update(&w, 0, true).iter().any(|e| e.speaker.is_some()));
    assert!(tracker.update(&w, 0, true).is_empty());
}

#[test]
fn swapped_heroes_and_inactive_slots_are_silent_and_remote_fire_is_spatial() {
    let mut w = arena();
    let mut t = Tracker::default();
    cues(&mut t, &w);
    w.tick += 1;
    w.actors[0].character = 14;
    w.actors[0].shot = 17;
    assert!(cues(&mut t, &w).is_empty());
    w.tick += 1;
    w.actors[1] = Actor {
        character: 3,
        active: true,
        ..Actor::default()
    };
    assert!(cues(&mut t, &w).is_empty());
    w.tick += 1;
    w.actors[1].shot += 1;
    let events = t.update(&w, 0, true);
    assert_eq!(events[0].cue, Cue::Fire(3));
    assert!(events[0].position.is_some());
    w.tick += 1;
    w.actors[1].active = false;
    w.actors[1].shot += 1;
    assert!(cues(&mut t, &w).is_empty());
}

#[test]
fn footsteps_require_ground_travel_and_teleport_is_not_walking() {
    let mut w = arena();
    let mut t = Tracker::default();
    cues(&mut t, &w);
    for _ in 0..2 {
        w.tick += 1;
        w.actors[0].pos.x += 0.7;
        assert!(cues(&mut t, &w).is_empty());
    }
    w.tick += 1;
    w.actors[0].pos.x += 0.7;
    assert!(cues(&mut t, &w).iter().any(|c| matches!(c, Cue::Step(_))));
    w.tick += 1;
    w.actors[0].pos.x += 30.0;
    assert!(cues(&mut t, &w).is_empty());
    w.tick += 1;
    w.actors[0].vy = 5.0;
    w.actors[0].pos.y = 0.1;
    assert_eq!(cues(&mut t, &w), vec![Cue::Jump]);
    w.tick += 1;
    w.actors[0].vy = -3.0;
    w.actors[0].pos.y = 0.2;
    assert!(cues(&mut t, &w).is_empty());
    w.tick += 1;
    w.actors[0].vy = 0.0;
    w.actors[0].pos.y = 0.0;
    assert_eq!(cues(&mut t, &w), vec![Cue::Land]);
}

#[test]
fn actual_simulation_fire_produces_cues_for_every_character() {
    for id in 0..15 {
        let mut w = arena();
        w.actors[0].character = id;
        w.actors[0].ammo = crate::kits::kit(id).magazine;
        let mut t = Tracker::default();
        cues(&mut t, &w);
        let mut fired = false;
        for tick in 0..120 {
            let mut inputs = [Input::default(); PLAYERS];
            inputs[0].fire = id != 11 || tick % 45 < 35;
            w.tick(inputs);
            fired |= cues(&mut t, &w).contains(&Cue::Fire(id));
        }
        assert!(
            fired,
            "character {id} never emitted a successful attack cue"
        );
    }
}

#[test]
fn impacts_are_deduplicated_and_stale_contacts_are_silent() {
    let mut w = arena();
    let mut t = Tracker::default();
    cues(&mut t, &w);
    let hit = crate::impact::Impact {
        id: 1,
        tick: w.tick,
        position: V3::default(),
        face: 3,
        color: [100; 3],
        radius: 0.1,
        kind: ImpactKind::Bullet,
        rotation: 0,
    };
    w.impacts.push(hit);
    w.tick += 1;
    assert_eq!(cues(&mut t, &w), vec![Cue::Wall]);
    w.tick += 1;
    assert!(cues(&mut t, &w).is_empty());
    w.impacts.push(crate::impact::Impact {
        id: 2,
        tick: 0,
        ..hit
    });
    w.tick += 1;
    assert!(cues(&mut t, &w).is_empty());
}
