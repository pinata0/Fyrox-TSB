use trickcal_shooting_balddagu::{
    combat::ProjectileKind,
    game::*,
    kits::kit,
    motion::{Motion, MotionClock},
    picora_skill::{self as p},
};
fn world() -> World {
    let mut w = World::default();
    w.start();
    w.phase = Phase::Playing;
    w.map.clear();
    for (i, a) in w.actors.iter_mut().enumerate() {
        a.active = true;
        a.human = true;
        a.character = 5;
        a.hp = MAX_HP;
        a.shield = 0.0;
        a.barrier = 0.0;
        a.combat = Default::default();
        a.ammo = 18;
        a.pos = V3::new(60.0 + i as f32, 0.0, -60.0);
    }
    w.actors[0].pos = V3::default();
    w.actors[1].pos = V3::new(0.0, 0.0, 5.0);
    w
}
fn input(w: &mut World, i: usize, x: Input) {
    let mut inputs = [Input::default(); PLAYERS];
    inputs[i] = x;
    w.tick(inputs);
}
fn advance(w: &mut World, n: usize) {
    for _ in 0..n {
        w.tick([Input::default(); PLAYERS]);
    }
}
fn cast(w: &mut World, ult: bool) {
    w.actors[0].combat.ultimate = 100.0;
    input(
        w,
        0,
        Input {
            ability: !ult,
            ultimate: ult,
            ..Default::default()
        },
    );
}
#[test]
fn aimed_sticker_releases_once_heals_only_its_target_and_expires() {
    let mut w = world();
    w.actors[1].hp = 40;
    w.actors[2].hp = 10;
    w.actors[2].pos = V3::new(3.0, 0.0, 5.0);
    cast(&mut w, false);
    assert_eq!(w.actors[0].combat.picora.target, Some(1));
    advance(&mut w, 49);
    assert_eq!(w.actors[1].hp, 40);
    assert_eq!(w.actors[1].combat.sticker, 0.0);
    advance(&mut w, 1);
    assert_eq!(w.actors[1].hp, 84);
    assert!(w.actors[1].combat.sticker > 3.98);
    assert_eq!(w.actors[2].hp, 10);
    assert_eq!(w.actors[0].stats.healing, 44.0);
    advance(&mut w, 239);
    assert!(w.actors[1].combat.sticker > 0.0);
    advance(&mut w, 2);
    assert_eq!(w.actors[1].combat.sticker, 0.0);
    assert_eq!(w.actors[1].hp, 84);
}
#[test]
fn empty_enemy_behind_and_occluded_aim_do_not_consume_e_or_q() {
    for case in 0..5 {
        let mut w = world();
        match case {
            0 => w.actors[1].pos.x = 3.0,
            1 => w.actors[1].pos.z = -5.0,
            2 => {
                w.actors[1].hp = 0;
                w.actors[1].respawn = 10.0;
            }
            3 => w.map.push(Block {
                center: V3::new(0.0, 1.0, 2.5),
                size: V3::new(3.0, 4.0, 0.5),
                color: [0; 3],
                phaseable: true,
            }),
            _ => {
                w.actors[1].pos.x = 3.0;
                w.actors[6].pos = V3::new(0.0, 0.0, 4.0);
            }
        }
        cast(&mut w, false);
        assert_eq!(w.actors[0].ability_cooldown, 0.0, "case {case}");
        cast(&mut w, true);
        assert_eq!(w.actors[0].combat.ultimate, 100.0);
        assert!(!w.actors[0].combat.picora.active());
    }
}
#[test]
fn target_is_locked_and_cover_death_or_interrupt_cancel_the_effect() {
    for case in 0..4 {
        let mut w = world();
        w.actors[1].hp = 50;
        cast(&mut w, false);
        match case {
            0 => w.actors[0].combat.stun = 2.0,
            1 => {
                w.actors[1].hp = 0;
                w.actors[1].respawn = 10.0;
            }
            2 => w.map.push(Block {
                center: V3::new(0.0, 1.0, 2.5),
                size: V3::new(3.0, 4.0, 0.5),
                color: [0; 3],
                phaseable: true,
            }),
            _ => w.actors[1].pos.z = 25.0,
        }
        advance(&mut w, 55);
        assert_eq!(w.actors[1].combat.sticker, 0.0);
        assert_eq!(w.actors[0].stats.healing, 0.0);
    }
    let mut w = world();
    w.actors[1].hp = 50;
    cast(&mut w, false);
    input(
        &mut w,
        0,
        Input {
            yaw: 2.0,
            ..Default::default()
        },
    );
    advance(&mut w, 50);
    assert!(
        w.actors[1].combat.sticker > 0.0,
        "turning after selection must not retarget"
    );
}
#[test]
fn ultimate_doubles_capacity_and_current_ratio_then_restores_without_taunt_or_guard() {
    let mut w = world();
    w.actors[1].hp = 60;
    cast(&mut w, true);
    advance(&mut w, 25);
    assert_eq!(w.actors[1].max_hp(), 120);
    advance(&mut w, 1);
    assert_eq!(w.actors[1].max_hp(), 240);
    assert_eq!(w.actors[1].hp, 120);
    assert_eq!(w.actors[1].combat.guard, 0.0);
    assert_eq!(w.actors[6].combat.taunt, 0.0);
    // A second cast refreshes duration; it cannot multiply capacity or heal again.
    advance(&mut w, 100);
    w.actors[0].combat.picora = Default::default();
    cast(&mut w, true);
    advance(&mut w, 26);
    assert_eq!(w.actors[1].max_hp(), 240);
    assert_eq!(w.actors[1].hp, 120);
    w.actors[1].hp = 191;
    advance(&mut w, 481);
    assert_eq!(w.actors[1].max_hp(), 120);
    assert_eq!(w.actors[1].hp, 96);
    assert_eq!(
        w.actors[0].stats.healing, 0.0,
        "capacity changes are not healing statistics"
    );
}
#[test]
fn healing_can_fill_the_expanded_bar_and_sticker_reduces_magic_and_physical_hits() {
    let mut w = world();
    w.actors[1].combat.fashion = 8.0;
    w.actors[1].hp = 200;
    cast(&mut w, false);
    advance(&mut w, 50);
    assert_eq!(w.actors[1].hp, 240);
    assert_eq!(w.actors[0].stats.healing, 40.0);
    for id in [3, 14] {
        let mut plain = world();
        plain.actors[6].character = id;
        plain.actors[6].ammo = kit(id).magazine;
        plain.actors[6].pos = V3::new(0.0, 0.0, 8.0);
        let mut protected = plain.clone();
        protected.actors[1].combat.sticker = 4.0;
        let fire = Input {
            fire: true,
            yaw: std::f32::consts::PI,
            ..Default::default()
        };
        input(&mut plain, 6, fire);
        input(&mut protected, 6, fire);
        assert!(
            plain.actors[1].hp < protected.actors[1].hp,
            "damage reduction for character {id}"
        );
    }
}
#[test]
fn basic_bolt_waits_for_attack_event_and_never_ricochets() {
    let mut w = world();
    w.actors[0].combat.attacks = 3;
    input(
        &mut w,
        0,
        Input {
            fire: true,
            ..Default::default()
        },
    );
    assert!(w.projectiles.is_empty());
    advance(&mut w, 10);
    assert!(w.projectiles.is_empty());
    advance(&mut w, 1);
    assert_eq!(w.projectiles.len(), 1);
    assert_eq!(w.projectiles[0].kind, ProjectileKind::Bolt);
    assert!(w.projectiles[0].velocity.z > 23.0);
    assert_eq!(w.projectiles[0].bounces, 0);
    assert_eq!(w.actors[0].ammo, 17);
}
#[test]
fn cast_motion_uses_source_time_and_first_person_ultimate_has_no_motion() {
    let mut w = world();
    cast(&mut w, true);
    advance(&mut w, 30);
    let pose = MotionClock::default().observe(&w.actors[0], 0.0, 1, true);
    assert_eq!(pose.motion, Motion::PicoraUltimate);
    assert!((pose.age - 0.5).abs() < 0.001);
    assert_eq!(Motion::PicoraSkill.seconds(5), p::SKILL_SECONDS);
    assert_eq!(Motion::PicoraUltimate.seconds(5), 2.0);
    for i in 0..61 {
        let h = p::hand(Motion::PicoraUltimate, i as f32 / 30.0, true);
        assert_eq!(h.position, [0.0; 3]);
        assert_eq!(h.angles, [0.0; 3]);
    }
    assert_eq!(w.actors[0].pos.y, 0.0);
    assert_eq!(w.actors[0].eye().y, EYE_HEIGHT);
}

#[test]
fn swapping_a_buffed_recipient_restores_capacity_and_clears_sticker() {
    let mut w = world();
    w.phase = Phase::Lobby;
    for a in &mut w.actors {
        a.human = false;
    }
    w.actors[1].hp = 190;
    w.actors[1].combat.fashion = 6.0;
    w.actors[1].combat.sticker = 3.0;
    assert_eq!(
        w.select_character(1, 1),
        trickcal_shooting_balddagu::roster::SelectionResult::Accepted
    );
    assert_eq!(w.actors[1].max_hp(), MAX_HP);
    assert_eq!(w.actors[1].hp, 95);
    assert_eq!(w.actors[1].combat.sticker, 0.0);
}
