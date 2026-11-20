use trickcal_shooting_balddagu::{
    game::*,
    kits::kit,
    roster::{SelectionResult, POSITION_LIMIT},
};

#[test]
fn full_positions_reserve_entry_for_current_members_and_reopen_when_vacated() {
    for side in 0..2 {
        for position in 0..3_u8 {
            let mut w = World::default();
            let first = side * TEAM_SIZE;
            let outsider = (position + 1) % 3;
            let other = (position + 2) % 3;
            for (offset, id) in [
                outsider,
                position,
                position + 3,
                position + 6,
                other,
                other + 3,
            ]
            .into_iter()
            .enumerate()
            {
                w.actors[first + offset].character = id;
            }
            w.actors[first].human = true;
            assert_eq!(w.position_counts(side)[position as usize], POSITION_LIMIT);

            // Every hero in the full row is blocked, even a bot exchange.
            for id in (position..15).step_by(3) {
                assert_eq!(
                    w.selection_check(first, id),
                    Err(SelectionResult::PositionFull)
                );
                assert_eq!(w.select_character(first, id), SelectionResult::PositionFull);
                assert_eq!(w.actors[first].character, outsider);
            }

            // Current members retain their own hero and can change within the row.
            assert_eq!(
                w.select_character(first + 1, position),
                SelectionResult::Accepted
            );
            assert_eq!(
                w.select_character(first + 1, position + 9),
                SelectionResult::Accepted
            );
            assert_eq!(
                w.select_character(first + 1, position + 3),
                SelectionResult::Accepted
            );
            assert_eq!(w.position_counts(side)[position as usize], POSITION_LIMIT);
            w.actors[first + 2].human = true;
            assert_eq!(
                w.select_character(first + 1, position + 9),
                SelectionResult::Taken
            );

            // The opposite team's capacity is independent.
            assert!(w
                .selection_check((1 - side) * TEAM_SIZE, position + 12)
                .is_ok());
            w.clear_slot(first + 3);
            assert_eq!(
                w.position_counts(side)[position as usize],
                POSITION_LIMIT - 1
            );
            assert_eq!(
                w.select_character(first, position + 12),
                SelectionResult::Accepted
            );
            assert_eq!(w.position_counts(side)[position as usize], POSITION_LIMIT);
        }
    }
}

#[test]
fn live_selection_uses_current_friendly_spawn_for_both_sides_and_points() {
    for attacker in 0..2 {
        for point in 0..2 {
            for slot in [0, 6] {
                let mut w = World {
                    phase: Phase::Playing,
                    attacker,
                    point,
                    ..Default::default()
                };
                w.actors[slot].human = true;
                let center = w.spawn_center(team(slot));
                w.actors[slot].pos = center + V3::new(SPAWN_HALF_WIDTH, 2.25, SPAWN_HALF_DEPTH);
                assert!(w.in_spawn(slot));
                assert!(w.can_select(slot));
                assert_eq!(w.select_character(slot, 4), SelectionResult::Accepted);
                for offset in [
                    V3::new(6.51, 0.0, 0.0),
                    V3::new(0.0, 0.0, 3.01),
                    V3::new(0.0, 2.26, 0.0),
                    V3::new(0.0, -0.1, 0.0),
                ] {
                    w.actors[slot].pos = center + offset;
                    assert_eq!(w.select_character(slot, 7), SelectionResult::Locked);
                }
                w.actors[slot].pos = w.spawn_center(1 - team(slot));
                assert!(!w.can_select(slot));
                w.actors[slot].pos = center;
                w.point = 1 - point;
                // A/B share the rear defender room; only attackers relocate.
                assert_eq!(w.in_spawn(slot), team(slot) != attacker);
                w.actors[slot].hp = 0;
                assert!(w.can_select(slot));
            }
        }
    }
    assert!(!World::default().can_select(PLAYERS));
}

#[test]
fn swap_keeps_at_most_fifteen_percent_without_granting_charge_or_healing() {
    for charge in [0.0, 7.5, 15.0, 58.0, 100.0] {
        let mut w = World {
            phase: Phase::Playing,
            ..Default::default()
        };
        let a = &mut w.actors[0];
        a.human = true;
        a.hp = 42;
        a.ability_cooldown = 5.0;
        a.combat.ultimate = charge;
        a.ammo = 0;
        a.reload = 1.0;
        a.shield = 0.0;
        let pos = a.pos;
        assert_eq!(w.select_character(0, 13), SelectionResult::Accepted);
        let a = &w.actors[0];
        assert_eq!(a.combat.ultimate, charge.min(15.0));
        assert_eq!(a.hp, 42);
        assert_eq!(a.ability_cooldown, 5.0);
        assert_eq!(a.ammo, kit(13).magazine);
        assert_eq!(a.reload, 0.0);
        assert_eq!(a.pos, pos);
        assert_eq!(a.shield, 0.0);
        assert_eq!(w.select_character(0, 7), SelectionResult::Accepted);
        assert_eq!(w.actors[0].combat.ultimate, charge.min(15.0));
        assert_eq!(w.actors[0].ammo, 0);
    }
}

#[test]
fn rejected_or_same_hero_request_does_not_spend_charge_and_bot_swap_caps_each_actor() {
    let mut w = World {
        phase: Phase::Playing,
        ..Default::default()
    };
    w.actors[0].human = true;
    w.actors[0].combat.ultimate = 83.0;
    w.actors[0].ammo = 0;
    w.actors[0].reload = 1.0;
    let id = w.actors[0].character;
    assert_eq!(w.select_character(0, id), SelectionResult::Accepted);
    assert_eq!(w.actors[0].combat.ultimate, 83.0);
    assert_eq!(w.actors[0].ammo, 0);
    assert_eq!(w.actors[0].reload, 1.0);
    w.actors[0].pos = V3::default();
    assert_eq!(w.select_character(0, 4), SelectionResult::Locked);
    assert_eq!(w.actors[0].combat.ultimate, 83.0);
    assert_eq!(w.actors[0].ammo, 0);
    assert_eq!(w.actors[0].reload, 1.0);
    w.actors[0].pos = w.spawn_center(0);
    w.actors[1].combat.ultimate = 8.0;
    w.actors[1].ammo = 0;
    w.actors[1].reload = 0.5;
    let bot_id = w.actors[1].character;
    assert_eq!(w.select_character(0, bot_id), SelectionResult::Accepted);
    assert_eq!(w.actors[0].combat.ultimate, 15.0);
    assert_eq!(w.actors[1].combat.ultimate, 8.0);
    for slot in [0, 1] {
        let a = &w.actors[slot];
        assert_eq!(a.ammo, kit(a.character).magazine);
        assert_eq!(a.reload, 0.0);
    }
}
