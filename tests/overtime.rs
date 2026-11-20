use trickcal_shooting_balddagu::{game::*, level::MapId};

fn fixture(map: MapId, attacker: usize, point: usize) -> World {
    let mut world = World::for_map(map);
    world.start();
    world.phase = Phase::Playing;
    world.attacker = attacker;
    world.point = point;
    world.remaining = DT * 0.5;
    for actor in &mut world.actors {
        actor.human = true;
        actor.pos = V3::new(12.0, 0.0, -12.0);
    }
    world
}

#[test]
fn contested_overtime_has_no_time_limit_on_either_map_team_or_point() {
    for map in MapId::ALL {
        for attacker in 0..2 {
            for point in 0..2 {
                let mut world = fixture(map, attacker, point);
                let slot = attacker * TEAM_SIZE;
                let defender = (1 - attacker) * TEAM_SIZE;
                let objective = world.points()[point];
                world.actors[slot].pos = objective;
                world.actors[slot + 1].pos = objective;
                world.actors[defender].pos = objective;
                world.progress = 0.5;
                for _ in 0..1800 {
                    world.update_objective();
                    assert_eq!(world.phase, Phase::Playing);
                    assert!(world.overtime);
                    assert!(world.contested);
                    assert_eq!(world.result, None);
                }
                assert_eq!(world.remaining, 0.0);
                assert_eq!(world.progress, 0.5);

                world.actors[slot].pos = objective + V3::new(RADIUS + 0.1, 0.0, 0.0);
                world.update_objective();
                assert_eq!(world.phase, Phase::Playing);
                assert!(world.overtime);
                world.actors[slot + 1].hp = 0;
                world.update_objective();
                assert_eq!(world.phase, Phase::Intermission);
                assert_eq!(world.result, Some(1 - attacker));
                assert!(!world.overtime);
            }
        }
    }
}

#[test]
fn expiry_requires_a_living_attacker_on_the_active_point() {
    for case in 0..5 {
        let mut world = fixture(MapId::ElfCity, 0, 0);
        let point = world.points()[0];
        // Defenders alone must never trigger overtime.
        world.actors[TEAM_SIZE].pos = point;
        world.actors[0].pos = point;
        match case {
            0 => world.actors[0].hp = 0,
            1 => world.actors[0].active = false,
            2 => world.actors[0].pos.y = 3.6,
            3 => world.actors[0].pos = world.points()[1],
            _ => world.actors[0].pos.x += RADIUS + 0.1,
        }
        world.update_objective();
        assert_eq!(world.phase, Phase::Intermission);
        assert_eq!(world.result, Some(1));
        assert!(!world.overtime);
    }
}

#[test]
fn uncontested_overtime_continues_until_b_is_fully_captured() {
    for attacker in 0..2 {
        let mut world = fixture(MapId::FairyKingdomCastle, attacker, 1);
        world.scores[attacker] = 1;
        world.actors[attacker * TEAM_SIZE].pos = world.points()[1];
        for _ in 0..(CAPTURE_SECONDS / DT) as usize - 1 {
            world.update_objective();
            assert_eq!(world.phase, Phase::Playing);
            assert!(world.overtime);
        }
        for _ in 0..3 {
            world.update_objective();
        }
        assert_eq!(world.phase, Phase::Intermission);
        assert_eq!(world.result, Some(attacker));
        assert_eq!(world.scores[attacker], 2);
        assert_eq!(world.rounds[attacker].points, 2);
        assert_eq!(world.rounds[attacker].bank, 0.0);
        assert!(!world.overtime);
    }
}

#[test]
fn capturing_a_in_overtime_restores_the_regular_time_bonus() {
    let mut world = fixture(MapId::FairyKingdomCastle, 0, 0);
    world.actors[0].pos = world.points()[0];
    world.update_objective();
    assert!(world.overtime);
    world.progress = 0.999;
    world.update_objective();
    assert_eq!(world.point, 1);
    assert_eq!(world.scores, [1, 0]);
    assert_eq!(world.remaining, 180.0);
    assert_eq!(world.phase, Phase::Playing);
    assert!(!world.overtime);
    assert!(!world.contested);
}

#[test]
fn time_bank_overtime_checks_b_presence_immediately_after_capturing_a() {
    for b_occupied in [false, true] {
        let mut world = fixture(MapId::ElfCity, 0, 0);
        world.extra = true;
        world.actors[0].pos = world.points()[0];
        world.actors[TEAM_SIZE].pos = world.points()[1];
        if b_occupied {
            world.actors[1].pos = world.points()[1];
        }
        world.update_objective();
        assert!(world.overtime);
        world.progress = 0.999;
        world.update_objective();
        assert_eq!(world.point, 1);
        assert_eq!(world.scores, [1, 0]);
        assert_eq!(world.remaining, 0.0);
        if b_occupied {
            assert_eq!(world.phase, Phase::Playing);
            assert!(world.overtime);
            assert!(world.contested);
        } else {
            assert_eq!(world.phase, Phase::Intermission);
            assert_eq!(world.rounds[0].points, 1);
            assert!(!world.overtime);
        }
    }
}
