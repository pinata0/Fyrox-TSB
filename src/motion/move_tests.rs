use super::*;

#[test]
fn move_periods_match_all_archived_tracks() {
    let records: serde_json::Value =
        serde_json::from_str(include_str!("../../assets/characters/move/motions.json")).unwrap();
    let records = records.as_array().unwrap();
    assert_eq!(records.len(), 15);
    let mut signatures = std::collections::HashSet::new();
    for (id, record) in records.iter().enumerate() {
        assert_eq!(record["id"], id);
        assert_eq!(record["clip"], "Move");
        assert!(
            (record["duration"].as_f64().unwrap() as f32 - move_seconds(id as u8)).abs() < 0.00001
        );
        assert_eq!(
            record["samples"].as_array().unwrap().len(),
            MOVE_FRAMES[id] as usize + 1
        );
        signatures.insert(record["samples"].to_string());
        assert!(
            (Motion::Run.seconds(id as u8) * 1.25 - Motion::Walk.seconds(id as u8)).abs() < 0.00001
        );
    }
    assert_eq!(signatures.len(), 15);
}

#[test]
fn walking_attack_retriggers_do_not_reset_stride_or_use_attack_haste() {
    for id in 0..15 {
        let mut actor = Actor {
            character: id,
            ..Actor::default()
        };
        actor.combat.haste = 2.0;
        let mut clock = MotionClock::default();
        let mut walk = MotionClock::default();
        clock.observe(&actor, 0.0, 0, true);
        walk.observe(&actor, 0.0, 0, true);
        for tick in 1..=150 {
            actor.pos.x += 0.04;
            if tick % 12 == 2 {
                actor.shot += 1;
            }
            let pose = clock.observe(&actor, 0.01, tick, true);
            let mut walking_actor = actor.clone();
            walking_actor.shot = 0;
            let expected = walk.observe(&walking_actor, 0.01, tick, true);
            let movement = pose.locomotion.unwrap();
            assert_eq!(movement.motion, Motion::Walk);
            assert!(
                (movement.age - expected.age).abs() < 0.00001,
                "stride restarted for {id} at {tick}"
            );
            assert_eq!(pose.base().0, Motion::Walk);
            if tick % 12 == 2 {
                assert_eq!(pose.motion, Motion::Attack);
                assert_eq!(pose.age, 0.0);
            }
        }
        actor.pos.x += 0.08;
        let old = clock.pose.locomotion.unwrap();
        let pose = clock.observe(&actor, 0.01, 151, true);
        let running = pose.locomotion.unwrap();
        assert_eq!(running.motion, Motion::Run);
        let expected = (old.age / 1.25 + 0.01).rem_euclid(Motion::Run.seconds(id));
        assert!((running.age - expected).abs() < 0.00001);
        // Stopping mid-attack returns to full-body attack without losing its clock.
        actor.shot += 1;
        let pose = clock.observe(&actor, 0.01, 152, true);
        assert!(pose.locomotion.is_none());
        assert_eq!(pose.base().0, Motion::Attack);
    }
}

#[test]
fn movement_layers_clear_on_airborne_skill_death_and_character_switch() {
    for transition in 0..6 {
        let mut actor = Actor::default();
        let mut clock = MotionClock::default();
        clock.observe(&actor, 0.0, 0, true);
        actor.pos.x += 0.05;
        actor.shot = 1;
        assert!(clock.observe(&actor, 0.01, 1, true).locomotion.is_some());
        actor.pos.x += 0.05;
        match transition {
            0 => {}
            1 => actor.ability_flash = 0.5,
            2 => actor.combat.ultimate_flash = 0.5,
            3 => actor.hp = 0,
            4 => actor.character = 1,
            _ => actor.dancing = true,
        }
        let pose = clock.observe(&actor, 0.01, 2, transition != 0);
        assert!(pose.locomotion.is_none());
        if transition == 0 {
            assert_eq!(pose.motion, Motion::Attack);
        }
    }
}

#[test]
fn normal_and_combo_arm_layers_fade_at_edges() {
    for id in 0..15 {
        for motion in [Motion::Attack, Motion::Attack2] {
            let mut pose = Pose {
                motion,
                locomotion: Some(LocomotionPose {
                    motion: Motion::Walk,
                    age: 0.3,
                }),
                ..Pose::default()
            };
            assert_eq!(pose.arm_attack_weight(id), 0.0);
            pose.age = motion.seconds(id) * 0.4;
            assert_eq!(pose.arm_attack_weight(id), 1.0);
            assert_eq!(pose.base(), (Motion::Walk, 0.3));
            pose.age = motion.seconds(id);
            assert_eq!(pose.arm_attack_weight(id), 0.0);
        }
    }
}
