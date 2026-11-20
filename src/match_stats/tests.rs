use super::*;
use crate::{combat::AttackClass, game::*};

fn arena() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.character = 11;
        a.human = true;
        a.shield = 0.0;
        a.pos = V3::new(i as f32 * 2.0, 0.0, 0.0);
    }
    w
}

#[test]
fn effective_damage_assists_and_solo_kills_do_not_count_overkill_or_immunity() {
    let mut w = arena();
    w.actors[6].shield = 2.0;
    assert!(!w.damage_hit(0, 6, 20.0, AttackClass::Basic, 11));
    assert_eq!(w.actors[0].stats, MatchStats::default());
    w.actors[6].shield = 0.0;
    w.damage_hit(0, 6, 20.0, AttackClass::Basic, 11);
    let first = MAX_HP - w.actors[6].hp;
    w.damage_hit(1, 6, 9999.0, AttackClass::Basic, 11);
    assert_eq!(w.actors[0].stats.damage, first as f32);
    assert_eq!(w.actors[1].stats.damage, (MAX_HP - first) as f32);
    assert_eq!(w.actors[0].stats.assists, 1);
    assert_eq!(w.actors[1].kills, 1);
    assert_eq!(w.actors[1].stats.solo_kills, 0);
    assert_eq!(w.actors[6].deaths, 1);
    w.damage_hit(1, 6, 9999.0, AttackClass::Basic, 11);
    assert_eq!(w.actors[0].stats.assists, 1);

    w.damage_hit(0, 7, 20.0, AttackClass::Basic, 11);
    w.tick += (11.0 / DT) as u64;
    w.damage_hit(1, 7, 9999.0, AttackClass::Basic, 11);
    assert_eq!(
        w.actors[0].stats.assists, 1,
        "old damage cannot earn an assist"
    );
    assert_eq!(w.actors[1].stats.solo_kills, 1);
}

#[test]
fn scoreboard_healing_and_shields_count_actual_effect_not_requested_amount() {
    let mut w = arena();
    w.actors[1].hp = MAX_HP - 7;
    w.heal_from(0, 1, 1000.0);
    w.heal_from(0, 1, 1000.0);
    assert_eq!(w.actors[0].stats.healing, 7.0);
    w.actors[1].hp = 0;
    w.heal_from(0, 1, 1000.0);
    assert_eq!(w.actors[0].stats.healing, 7.0);
    w.actors[6].barrier = 100.0;
    w.damage_hit(0, 6, 40.0, AttackClass::Basic, 11);
    assert_eq!(w.actors[6].hp, MAX_HP);
    assert!(w.actors[6].stats.mitigated > 0.0);
    assert_eq!(w.actors[0].stats.damage, 100.0 - w.actors[6].barrier);
    // A regeneration effect credits its original caster, including when they died.
    w.actors[2].hp = MAX_HP - 30;
    w.actors[2].combat.regen = 2.0;
    w.actors[2].combat.regen_rate = 20.0;
    w.actors[2].combat.regen_source = Some(0);
    w.advance_combat();
    assert_eq!(w.actors[0].stats.healing, 12.0);
}

#[test]
fn scoreboard_totals_survive_respawn_and_rounds_but_reset_for_new_match() {
    let mut w = arena();
    w.damage_hit(0, 6, 1000.0, AttackClass::Basic, 11);
    let stats = w.actors[0].stats;
    w.actors[6].respawn = DT;
    w.tick([Input::default(); PLAYERS]);
    assert!(w.actors[6].alive());
    assert!(w.actors[6].contributors.iter().all(Option::is_none));
    assert_eq!(w.actors[0].stats, stats);
    w.finish_round(true);
    w.phase_time = DT;
    w.tick([Input::default(); PLAYERS]);
    assert_eq!(w.actors[0].stats, stats);
    assert_eq!(w.actors[0].kills, 1);
    w.start();
    assert_eq!(w.actors[0].stats, MatchStats::default());
    assert_eq!(w.actors[0].kills, 0);
}
