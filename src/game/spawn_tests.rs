use super::*;

fn duel(id: u8) -> World {
    let mut w = World {
        phase: Phase::Playing,
        ..Default::default()
    };
    w.map.clear();
    for a in &mut w.actors {
        a.hp = 0;
        a.human = true;
    }
    w.actors[0].hp = MAX_HP;
    w.actors[0].character = id;
    w.actors[0].human = false;
    w.actors[0].pos = w.spawn_center(1) + V3::new(0.0, 0.0, -6.0);
    w.actors[6].hp = MAX_HP;
    w.actors[6].shield = 0.0;
    w
}

#[test]
fn ai_drops_shooting_and_cached_melee_chase_immediately_on_spawn_entry() {
    for id in [0, 4, 12, 14] {
        let mut w = duel(id);
        w.actors[6].pos = w.spawn_center(1) + V3::new(0.0, 0.0, -3.1);
        w.bot_input(0);
        assert_eq!(w.actors[0].bot_target, Some(6));
        w.actors[0].think = 10.0;
        w.actors[0].combat.taunt = 3.0;
        w.actors[0].combat.taunt_source = 6;
        w.actors[0].combat.ultimate = 100.0;
        w.actors[6].pos.z += 0.2;
        let input = w.bot_input(0);
        assert_eq!(w.actors[0].bot_target, None);
        assert!(!input.fire && !input.ultimate);
        assert!((w.actors[0].waypoint - w.actors[6].pos).length() > 5.0);
        assert!(
            w.actors[0].think < 1.0,
            "must invalidate the old chase this frame"
        );
        w.actors[6].pos.z -= 0.2;
        w.bot_input(0);
        assert_eq!(w.actors[0].bot_target, Some(6));
    }
}

#[test]
fn ai_retargets_an_exposed_enemy_instead_of_a_protected_taunter() {
    let mut w = duel(4);
    w.actors[6].pos = w.spawn_center(1);
    w.actors[7].hp = MAX_HP;
    w.actors[7].pos = w.actors[0].pos + V3::new(1.0, 0.0, 1.0);
    w.actors[0].combat.taunt = 2.0;
    w.actors[0].combat.taunt_source = 6;
    w.bot_input(0);
    assert_eq!(w.actors[0].bot_target, Some(7));
}
