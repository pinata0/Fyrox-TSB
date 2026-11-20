//! GPU checkpoints for status transitions, priority, and overlay cleanup.
use super::*;

const STAGES: [&str; 34] = [
    "clear",
    "naia-windup",
    "concentrating",
    "released",
    "stunned",
    "interrupted",
    "dead",
    "respawned",
    "selection",
    "settings",
    "scoreboard",
    "round-over",
    "polymorph",
    "frozen",
    "frostbite",
    "poison",
    "weaken",
    "sleep",
    "taunt",
    "blind",
    "time-stop",
    "stacked-debuffs",
    "slow-first-person",
    "rain-first-person",
    "time-slow-first-person",
    "slow-third-person",
    "slow-other-player",
    "slow-cleared",
    "slow-scoped",
    "slow-help",
    "slow-home",
    "slow-lobby",
    "slow-inactive-slot",
    "slow-critical-health",
];

pub(in crate::presentation) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let step = rt.review_capture as usize;
    if step >= STAGES.len() {
        println!("STATUS_EFFECTS_REVIEW_OK: debuffs, stacked labels, slow/rain/time effects, first/third person, scope, death/respawn and overlay cleanup");
        context.loop_controller.exit();
        return Ok(());
    }
    rt.home_open = step == 30;
    rt.lobby.open = step == 31;
    rt.selecting = step == 8;
    rt.settings_menu.open = step == 9;
    rt.character_help.open = step == 29;
    rt.grabbed = true;
    rt.chat.open = false;
    rt.keys = Keys::default();
    rt.keys.score = step == 10;
    rt.keys.zoom = step == 28;
    if rt.review_frames == 0 {
        rt.deathcam = DeathCamera::default();
        if let Session::Solo(world) = &mut rt.session {
            **world = World::default();
            world.phase = if step == 11 {
                Phase::Intermission
            } else {
                Phase::Playing
            };
            for (i, actor) in world.actors.iter_mut().enumerate() {
                actor.active = (i == 0 && step != 32) || (i == 6 && step == 26);
            }
            let actor = &mut world.actors[0];
            actor.human = true;
            actor.character = 1;
            actor.pos = V3::new(0.0, 0.0, -12.0);
            actor.yaw = 0.0;
            actor.pitch = 0.0;
            actor.ammo = 24;
            actor.hp = if step == 6 { 0 } else { MAX_HP };
            actor.respawn = if step == 6 { 5.0 } else { 0.0 };
            actor.combat.naia.phase = match step {
                1 => NaiaPhase::Start,
                2 | 5 => NaiaPhase::Fire,
                3 => NaiaPhase::Move,
                _ => NaiaPhase::Off,
            };
            actor.combat.naia.elapsed = if step == 1 { 0.7 } else { 2.0 };
            actor.combat.stun = if matches!(step, 4..=6 | 8..=11) {
                1.5
            } else {
                0.0
            };
            if matches!(step, 6 | 8..=11 | 22 | 25 | 28..=33) {
                actor.combat.slow = 2.0;
            }
            match step {
                12 => actor.combat.polymorph = 2.0,
                13 => actor.combat.frozen = 2.0,
                14 => actor.combat.frost = 2.0,
                15 => actor.combat.poison = 2.0,
                16 => actor.combat.skill_weaken = 2.0,
                17 => actor.combat.sleep = 2.0,
                18 => actor.combat.taunt = 2.0,
                19 => actor.combat.blind = 2.0,
                20 => actor.combat.temporal = 1.0,
                21 => {
                    actor.combat.stun = 2.0;
                    actor.combat.frost = 2.0;
                    actor.combat.poison = 2.0;
                    actor.combat.weaken = 2.0;
                    actor.combat.rain_slow = 2.0;
                }
                23 => actor.combat.rain_slow = 2.0,
                24 => actor.combat.temporal = 0.2,
                25 => actor.dancing = true,
                28 => actor.character = zoom::SHION,
                33 => actor.hp = 11,
                _ => {}
            }
            rt.yaw = actor.yaw;
            rt.pitch = actor.pitch;
            rt.view_pos = actor.eye();
            if step == 26 {
                let actor = &mut world.actors[6];
                actor.character = 1;
                actor.pos = V3::new(0.0, 0.0, -8.0);
                actor.combat.slow = 2.0;
            }
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)
}

pub(in crate::presentation) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if !context
        .graphics_context
        .as_initialized_ref()
        .renderer
        .scene_data_map
        .contains_key(&rt.scene)
    {
        return Ok(());
    }
    rt.review_frames += 1;
    // Fixtures are frozen: four rendered frames settle visibility, layout and fonts
    // without waiting through 24 expensive frames for every static checkpoint.
    if rt.review_frames < 4 {
        return Ok(());
    }
    let step = rt.review_capture as usize;
    let ui = context.user_interfaces.first();
    let label = &ui[rt.hud.status_effects.label];
    let expected = match step {
        2 => Some("정신집중"),
        4 | 5 | 21 => Some("기절"),
        12 => Some("변이 · 이동만 가능"),
        13 | 14 => Some("동상"),
        15 => Some("중독"),
        16 => Some("약화"),
        17 => Some("수면"),
        18 => Some("도발"),
        19 => Some("실명"),
        20 => Some("시간 정지"),
        _ => None,
    };
    assert_eq!(label.visibility(), expected.is_some(), "{}", STAGES[step]);
    let details = &ui[rt.hud.status_effects.details];
    assert_eq!(details.visibility(), step == 21, "{} details", STAGES[step]);
    if step == 21 {
        assert_eq!(details.cast::<Text>().unwrap().text(), "동상 · 중독 · 약화");
    }
    assert!(
        ui[rt.hud.status].cast::<Text>().unwrap().text().is_empty(),
        "No duplicate status or slow text"
    );
    let slow_screen = matches!(step, 20..=24 | 28 | 33);
    assert_eq!(
        ui[rt.hud.slow_screen.root].visibility(),
        slow_screen,
        "{} screen",
        STAGES[step]
    );
    let base = if step == 28 { 35.0 } else { 80.0 };
    if slow_screen {
        assert!(rt.gameplay_fov().to_degrees() < base);
    } else {
        assert!((rt.gameplay_fov().to_degrees() - base).abs() < 0.001);
    }
    let g = &context.scenes[rt.scene].graph;
    let feet = rt.slow_feet.as_ref().unwrap();
    assert_eq!(
        g[feet.nodes[0]].visibility(),
        step == 25,
        "{} local feet",
        STAGES[step]
    );
    assert_eq!(
        g[feet.nodes[6]].visibility(),
        step == 26,
        "{} remote feet",
        STAGES[step]
    );
    if let Some(expected) = expected {
        assert_eq!(label.cast::<Text>().unwrap().text(), expected);
        let bounds = label.screen_bounds();
        assert!((bounds.center().x - ui.screen_size().x * 0.5).abs() < 1.0);
        assert!(bounds.position.y > ui.screen_size().y * 0.3);
        assert!(bounds.position.y + bounds.size.y < ui.screen_size().y * 0.5);
    }
    let size = ui.screen_size();
    let tag = format!(
        "status-effects-{}x{}-{}-",
        size.x as u32, size.y as u32, STAGES[step]
    );
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
