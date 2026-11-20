use super::*;

#[test]
fn komi_wake_and_giant_leaps_match_authority_through_reconciliation() {
    use crate::komi_skill::{Phase as Action, State};
    for phase in [Action::Rest,Action::Feast,Action::Giant] {
        let mut w=fixture();w.actors[0].character=12;w.actors[0].combat.komi=State{phase,..Default::default()};
        let mut motion=ClientMotion::default();motion.observe(&w,0,0);
        for sequence in 1..=560 {
            let input=Input{wake_rest:sequence==10,phase_shift:sequence%100==50,forward:if sequence>10{0.3}else{0.0},..Default::default()};
            motion.advance(&w,sequence,input);let mut inputs=[Input::default();PLAYERS];inputs[0]=input;w.tick(inputs);
            let p=motion.predicted.as_ref().unwrap();let a=&w.actors[0];
            assert!((p.pos-a.pos).length()<0.001,"{phase:?} tick {sequence}");
            assert_eq!(p.combat.komi.phase,a.combat.komi.phase);
            assert!((p.body_scale()-a.body_scale()).abs()<0.001);
            assert!((p.race_skill.cooldown-a.race_skill.cooldown).abs()<0.001);
            if sequence%3==0 {motion.observe(&w,0,sequence);}
        }
    }
}

#[test]
fn surge_prediction_matches_direction_charges_recharge_and_replayed_inputs() {
    for id in [2, 4, 5, 6, 8] {
        let mut world = fixture();
        world.actors[0].character = id;
        let mut motion = ClientMotion::default();
        motion.observe(&world, 0, 0);
        for sequence in 1..=560 {
            let input = Input {
                phase_shift: matches!(sequence, 1 | 11 | 21 | 191),
                yaw: 0.6,
                ..Default::default()
            };
            motion.advance(&world, sequence, input);
            let mut inputs = [Input::default(); PLAYERS];
            inputs[0] = input;
            world.tick(inputs);
            let predicted = motion.predicted.as_ref().unwrap();
            assert!((predicted.pos - world.actors[0].pos).length() < 0.001);
            assert_eq!(
                predicted.race_skill.surge_charges,
                world.actors[0].race_skill.surge_charges
            );
            assert!(
                (predicted.race_skill.surge_recharge - world.actors[0].race_skill.surge_recharge)
                    .abs()
                    < 0.001
            );
            if sequence % 3 == 0 {
                motion.observe(&world, 0, sequence);
            }
        }
    }
}

#[test]
fn surge_during_basic_attacks_matches_authority_and_reconciliation() {
    for (id, mounted) in [
        (2, false), (4, false), (5, false), (6, false), (6, true), (8, false),
    ] {
        let mut world = fixture();
        world.actors[0].character = id;
        world.actors[0].ammo = crate::kits::kit(id).magazine;
        if mounted {
            world.actors[0].combat.chloe = crate::chloe_skill::State {
                phase: crate::chloe_skill::Phase::Ride,
                remaining: crate::chloe_skill::RIDE_SECONDS,
                ..Default::default()
            };
        }
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0].fire = true;
        world.tick(inputs);
        assert_eq!(world.actors[0].combat.attacks, 1);
        let mut motion = ClientMotion::default();
        motion.observe(&world, 0, 0);
        for sequence in 1..=12 {
            let input = Input {
                phase_shift: sequence == 1,
                ..Default::default()
            };
            motion.advance(&world, sequence, input);
            inputs[0] = input;
            world.tick(inputs);
            let predicted = motion.predicted.as_ref().unwrap();
            let authoritative = &world.actors[0];
            assert!((predicted.pos - authoritative.pos).length() < 0.001);
            assert_eq!(predicted.race_skill.surge_charges, 2);
            assert_eq!(authoritative.race_skill.surge_charges, 2);
            assert!(
                (predicted.race_skill.remaining - authoritative.race_skill.remaining).abs() < 0.001
            );
            assert_eq!(predicted.combat.chloe.phase, authoritative.combat.chloe.phase);
            if sequence % 3 == 0 {
                motion.observe(&world, 0, sequence);
            }
        }
        assert!((world.actors[0].pos.z - 3.75).abs() < 0.001);
    }
}

#[test]
fn authoritative_stun_discards_visual_offset_and_pending_surge_movement() {
    let mut world = fixture();
    world.actors[0].character = 4;
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    motion.advance(
        &world,
        1,
        Input {
            phase_shift: true,
            ..Default::default()
        },
    );
    motion.advance(&world, 2, Input::default());
    world.tick = 1;
    world.actors[0].pos.z = 1.1;
    world.actors[0].combat.stun = 1.0;
    world.actors[0].race_skill.surge_charges = 2;
    world.actors[0].race_skill.surge_recharge = 3.0;
    motion.observe(&world, 0, 1);
    assert_eq!(motion.actor(&world, 0, 0).pos, world.actors[0].pos);
    assert!(!motion.actor(&world, 0, 0).race_skill.active());
    assert_eq!(motion.actor(&world, 0, 0).race_skill.surge_charges, 2);
}

#[test]
fn race_skills_match_authority_through_reconciliation_and_emergence() {
    for id in [1, 10, 11, 12] {
        let mut world = fixture();
        world.actors[0].character = id;
        let mut motion = ClientMotion::default();
        motion.observe(&world, 0, 0);
        for sequence in 1..=300 {
            let input = Input {
                phase_shift: sequence < 5,
                forward: 1.0,
                yaw: 0.4,
                fire: (40..90).contains(&sequence),
                ..Default::default()
            };
            motion.advance(&world, sequence, input);
            let mut inputs = [Input::default(); PLAYERS];
            inputs[0] = input;
            world.tick(inputs);
            let predicted = motion.actor(&world, 0, 0);
            assert!(
                (predicted.pos - world.actors[0].pos).length() < 0.001,
                "hero {id}, tick {sequence}"
            );
            assert_eq!(predicted.burrowed(), world.actors[0].burrowed());
            assert_eq!(
                predicted.race_skill.fire_latched,
                world.actors[0].race_skill.fire_latched
            );
            if sequence % 3 == 0 {
                motion.observe(&world, 0, sequence);
            }
        }
    }
}

fn fixture() -> World {
    let mut world = World::room(MapId::default(), true);
    world.map.clear();
    world.phase = Phase::Playing;
    world.actors[0].pos = V3::default();
    world.actors[0].character = 3;
    world
}

fn forward() -> Input {
    Input {
        forward: 1.0,
        ..Default::default()
    }
}

#[test]
fn naia_fire_and_release_predict_without_waiting_for_a_snapshot() {
    use crate::naia_skill::{Phase as SkillPhase, State, SPEED_MULTIPLIER, START_SECONDS};
    let mut world = fixture();
    world.actors[0].character = 1;
    world.actors[0].combat.naia = State {
        phase: SkillPhase::Move,
        elapsed: START_SECONDS + 0.5,
        pulse: 0.0,
    };
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    motion.advance(
        &world,
        1,
        Input {
            fire: true,
            forward: 1.0,
            jump: true,
            ..Default::default()
        },
    );
    let actor = motion.actor(&world, 0, 0);
    assert_eq!(actor.pos, V3::default());
    assert_eq!(actor.combat.naia.phase, SkillPhase::Fire);
    motion.advance(&world, 2, forward());
    let actor = motion.actor(&world, 0, 0);
    assert_eq!(actor.combat.naia.phase, SkillPhase::Move);
    assert!((actor.pos.z - movement::WALK_SPEED * SPEED_MULTIPLIER * DT).abs() < 0.00001);
    assert_eq!(world.actors[0].pos, V3::default());
}

#[test]
fn crouch_predicts_immediately_and_reconciles_pending_release() {
    let mut world = fixture();
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    let input = Input {
        crouch: true,
        forward: 1.0,
        ..Default::default()
    };
    motion.advance(&world, 1, input);
    assert!(motion.actor(&world, 0, 0).crouching);
    assert!(!world.actors[0].crouching);
    assert_eq!(motion.actor(&world, 0, 0).eye().y, 1.15);
    movement::step_actor(&world.map, &mut world.actors[0], input);
    world.tick += 1;
    motion.advance(&world, 2, Input::default());
    motion.observe(&world, 0, 1);
    assert!(!motion.actor(&world, 0, 0).crouching);
    assert!(world.actors[0].crouching);
}

#[test]
fn local_move_is_immediate_and_ack_replays_only_unprocessed_inputs() {
    let mut world = fixture();
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    for seq in 1..=6 {
        motion.advance(&world, seq, forward());
    }
    let expected = movement::WALK_SPEED * DT * 6.0;
    assert!((motion.actor(&world, 0, 0).pos.z - expected).abs() < 0.0001);
    assert_eq!(
        world.actors[0].pos,
        V3::default(),
        "prediction cannot alter authority"
    );
    for _ in 0..3 {
        movement::step_actor(&world.map, &mut world.actors[0], forward());
        world.tick += 1;
    }
    motion.observe(&world, 0, 3);
    assert_eq!(motion.pending.len(), 3);
    assert!((motion.predicted.as_ref().unwrap().pos.z - expected).abs() < 0.0001);
    // A second snapshot acknowledging the same input must not apply it twice.
    motion.observe(&world, 0, 3);
    assert!((motion.predicted.as_ref().unwrap().pos.z - expected).abs() < 0.0001);
}

#[test]
fn jump_reconciliation_preserves_vertical_velocity_and_lands() {
    let mut world = fixture();
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    for seq in 1..=90 {
        let input = Input {
            jump: seq == 1,
            ..Default::default()
        };
        motion.advance(&world, seq, input);
        movement::step_actor(&world.map, &mut world.actors[0], input);
        world.tick += 1;
        if seq % 3 == 0 {
            motion.observe(&world, 0, seq);
        }
        assert!((motion.predicted.as_ref().unwrap().pos - world.actors[0].pos).length() < 0.0001);
        assert!((motion.predicted.as_ref().unwrap().vy - world.actors[0].vy).abs() < 0.0001);
    }
    assert_eq!(motion.actor(&world, 0, 0).pos.y, 0.0);
}

#[test]
fn prediction_obeys_walls_ghost_phasing_and_status_effects() {
    let mut world = fixture();
    world.map.push(crate::game::Block {
        center: V3::new(0.0, 1.5, 0.8),
        size: V3::new(5.0, 3.0, 0.2),
        color: [0; 3],
        phaseable: true,
    });
    for hero in [3, 14] {
        world.actors[0].character = hero;
        let mut motion = ClientMotion::default();
        motion.observe(&world, 0, 0);
        for seq in 1..=30 {
            motion.advance(
                &world,
                seq,
                Input {
                    phase_shift: true,
                    ..forward()
                },
            );
        }
        let z = motion.actor(&world, 0, 0).pos.z;
        if hero == 14 {
            assert!(z > 1.3);
        } else {
            assert!(z < 0.31);
        }
    }
    world.map.clear();
    world.actors[0].combat.stun = 0.5;
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    for seq in 1..=12 {
        motion.advance(&world, seq, forward());
    }
    assert_eq!(motion.actor(&world, 0, 0).pos.z, 0.0);
    world.actors[0].combat.stun = 0.0;
    world.actors[0].combat.slow = 1.0;
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    motion.advance(&world, 1, forward());
    assert!((motion.actor(&world, 0, 0).pos.z - movement::WALK_SPEED * 0.65 * DT).abs() < 0.0001);
}

#[test]
fn phase_prediction_matches_authority_through_expiry_and_pending_replay() {
    let mut world = fixture();
    world.actors[0].character = 14;
    world.map.push(crate::game::Block {
        center: V3::new(0.0, 2.0, 2.0),
        size: V3::new(8.0, 4.0, 2.0),
        color: [0; 3],
        phaseable: true,
    });
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    for sequence in 1..=220 {
        let input = Input {
            phase_shift: true,
            forward: if sequence <= 20 { 1.0 } else { 0.0 },
            ..Default::default()
        };
        motion.advance(&world, sequence, input);
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0] = input;
        world.tick(inputs);
        let predicted = motion.predicted.as_ref().unwrap();
        assert!(
            (predicted.pos - world.actors[0].pos).length() < 0.001,
            "position at tick {sequence}"
        );
        assert!(
            (predicted.phase_skill.remaining - world.actors[0].phase_skill.remaining).abs() < 0.001
        );
        assert!(
            (predicted.phase_skill.cooldown - world.actors[0].phase_skill.cooldown).abs() < 0.001
        );
        if sequence % 3 == 0 {
            motion.observe(&world, 0, sequence);
        }
    }
    assert!(!movement::blocked(
        &world.map,
        motion.actor(&world, 0, 0).pos
    ));
    assert!(world.actors[0].phase_skill.cooldown > 11.0);
}

#[test]
fn death_respawn_character_map_and_round_clear_old_inputs() {
    let mut world = fixture();
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    for seq in 1..=6 {
        motion.advance(&world, seq, forward());
    }
    world.actors[0].hp = 0;
    world.actors[0].deaths += 1;
    world.tick += 3;
    motion.observe(&world, 0, 0);
    assert!(motion.pending.is_empty());
    assert_eq!(motion.actor(&world, 0, 0).pos, world.actors[0].pos);
    world.actors[0].hp = 120;
    world.actors[0].pos = V3::new(20.0, 0.0, 20.0);
    world.tick += 3;
    motion.observe(&world, 0, 6);
    assert_eq!(motion.actor(&world, 0, 0).pos, world.actors[0].pos);
    for transition in 0..3 {
        motion.advance(&world, 20 + transition, forward());
        match transition {
            0 => world.actors[0].character = 14,
            1 => world.map_id = MapId::ElfCity,
            _ => world.round += 1,
        }
        world.tick += 3;
        motion.observe(&world, 0, 6);
        assert!(motion.pending.is_empty());
        assert_eq!(motion.actor(&world, 0, 0).pos, world.actors[0].pos);
    }
}

#[test]
fn stalled_link_bounds_prediction_and_recovers_without_old_inputs() {
    let mut world = fixture();
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    for seq in 1..=60 {
        motion.advance(&world, seq, forward());
    }
    let stopped = motion.actor(&world, 0, 0).pos;
    for seq in 61..=120 {
        motion.advance(&world, seq, forward());
    }
    assert_eq!(motion.actor(&world, 0, 0).pos, stopped);
    assert!(motion.pending.len() <= HISTORY);
    world.tick = 120;
    motion.observe(&world, 0, 100);
    assert!(motion.pending.is_empty());
    assert_eq!(motion.actor(&world, 0, 0).pos, world.actors[0].pos);
}

#[test]
fn remote_motion_is_continuous_between_twenty_hz_snapshots() {
    let mut world = fixture();
    world.actors[1] = world.actors[0].clone();
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    let mut previous = 0.0;
    for tick in 1..=120 {
        if tick % 3 == 0 {
            world.tick = tick;
            world.actors[1].pos.z = tick as f32 * DT * movement::WALK_SPEED;
            motion.observe(&world, 0, tick);
        }
        motion.advance(&world, tick, Input::default());
        let z = motion.actor(&world, 1, 0).pos.z;
        if tick > 20 {
            assert!(
                (0.8..1.25).contains(&((z - previous) / (movement::WALK_SPEED * DT))),
                "tick {tick}: step {}",
                z - previous
            );
        }
        previous = z;
    }
}

#[test]
fn remote_turn_crosses_angle_wrap_by_shortest_path_and_death_is_immediate() {
    let mut world = fixture();
    world.actors[1] = world.actors[0].clone();
    world.actors[1].yaw = std::f32::consts::TAU - 0.1;
    let mut motion = ClientMotion::default();
    motion.observe(&world, 0, 0);
    world.tick = 3;
    world.actors[1].yaw = 0.1;
    motion.observe(&world, 0, 3);
    motion.render_tick = 1.5;
    assert!(motion.actor(&world, 1, 0).yaw.sin().abs() < 0.001);
    world.actors[1].hp = 0;
    world.actors[1].pos.z = 1.0;
    world.tick = 6;
    motion.observe(&world, 0, 6);
    assert_eq!(motion.actor(&world, 1, 0).pos, world.actors[1].pos);
}

#[test]
fn delayed_jittery_lossy_snapshots_do_not_gate_local_movement() {
    let mut server = fixture();
    let mut client = server.clone();
    let mut motion = ClientMotion::default();
    motion.observe(&client, 0, 0);
    let mut deliveries = VecDeque::new();
    let mut previous = 0.0;
    let mut max_step = 0.0_f32;
    for tick in 1..=180 {
        motion.advance(&client, tick, forward());
        // 50 ms each way, 0-33 ms variable downstream delay, one in seven
        // snapshots lost. The host holds the latest input between packets.
        let ack = tick.saturating_sub(3);
        if ack > 0 {
            movement::step_actor(&server.map, &mut server.actors[0], forward());
        }
        server.tick = tick;
        if tick % 3 == 0 && tick % 21 != 0 {
            deliveries.push_back((tick + 3 + (tick / 3) % 3, ack, server.clone()));
        }
        while deliveries.front().is_some_and(|(at, _, _)| *at <= tick) {
            let (_, ack, snapshot) = deliveries.pop_front().unwrap();
            motion.observe(&snapshot, 0, ack);
            client = snapshot;
        }
        let z = motion.actor(&client, 0, 0).pos.z;
        if tick == 1 {
            assert!(z > movement::WALK_SPEED * DT * 0.9, "must move before the server's reply");
        }
        if tick > 12 {
            assert!(z > previous, "movement paused at tick {tick}");
            max_step = max_step.max(z - previous);
        }
        previous = z;
    }
    assert!(max_step < movement::WALK_SPEED * DT * 1.7, "large correction: {max_step}");
    assert!(motion.pending.len() < 20);
}
