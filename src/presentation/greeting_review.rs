//! Exercise both sides of the wave and the equipped-hand transitions on every original sleeve.
use super::*;

const TICKS: [u8; 6] = [0, 100, 80, 69, 8, 0];
const WORLD_HEROES: [u8; 4] = [1, 2, 7, 11];

fn review_step(capture: u8) -> usize {
    if std::env::args().any(|arg| arg == "--first-person-framing-only") {
        capture as usize / 3 * 6 + [0, 2, 5][capture as usize % 3]
    } else if std::env::args().any(|arg| arg == "--erpin-only") {
        12 + capture as usize
    } else if std::env::args().any(|arg| arg == "--mayo-only") {
        48 + capture as usize
    } else {
        capture as usize
    }
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if std::env::args().any(|arg| arg == "--mayo-only") && rt.review_capture >= 6 {
        println!("MAYO_GREETING_REVIEW_OK: grip/raise/left/right/lower/return, 109-tick continuity and sleeve framing");
        context.loop_controller.exit();
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--first-person-framing-only") && rt.review_capture >= 45 {
        println!("GREETING_FRAMING_REVIEW_OK: 15 heroes, 45 captures, 109 ticks per hero, visible fingers/palm and smooth grip return");
        context.loop_controller.exit();
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--erpin-only") && rt.review_capture >= 6 {
        println!(
            "ERPIN_GREETING_REVIEW_OK: anatomical right arm, grip/raise/left/right/lower/return"
        );
        context.loop_controller.exit();
        return Ok(());
    }
    let step = review_step(rt.review_capture);
    if step >= 90 + WORLD_HEROES.len() {
        println!("GREETING_REVIEW_OK: 15 heroes, grip/raise/left/right/lower/return, 4 third-person sleeves");
        context.loop_controller.exit();
        return Ok(());
    }
    let world_view = step >= 90;
    let id = if world_view {
        WORLD_HEROES[step - 90]
    } else {
        (step / 6) as u8
    };
    let ticks = if world_view { 80 } else { TICKS[step % 6] };
    rt.review_frames = rt.review_frames.max(1);
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        let actor = &mut w.actors[0];
        actor.character = id;
        actor.pos = V3::new(0.0, 0.0, -12.0);
        actor.hp = MAX_HP;
        actor.dancing = false;
        actor.wave_ticks = ticks;
        actor.reload = 0.0;
        actor.yaw = 0.0;
        actor.pitch = 0.0;
        actor.ammo = kit(id).magazine;
    }
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    for (index, actor) in rt.actors.iter().enumerate() {
        g[*actor].set_visibility(index == 0);
    }
    if world_view {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        g[rt.camera]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, 1.2, -10.55))
            .set_rotation(UnitQuaternion::from_euler_angles(
                0.0,
                std::f32::consts::PI,
                0.0,
            ));
    }
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let step = review_step(rt.review_capture);
    let id = rt.session.world().actors[0].character;
    let world_view = step >= 90;
    if !world_view && step.is_multiple_of(6) {
        rt.first_person_arm
            .as_ref()
            .unwrap()
            .verify_greeting_continuity(
                &mut context.scenes[rt.scene].graph,
                rt.wave_hand,
                rt.camera,
                rt.weapons[id as usize],
            );
    }
    let g = &context.scenes[rt.scene].graph;
    let state = if world_view {
        "world"
    } else {
        ["grip", "raise", "left", "right", "lower", "return"][step % 6]
    };
    assert!(!rt.orbit.active);
    if !world_view {
        let held = TICKS[step % 6] == 0;
        assert!(g[rt.wave_hand].visibility());
        assert_eq!(g[rt.weapon].visibility(), held);
        assert!(g[rt.actors[0]].visibility());
        assert!(rt.visuals[0].is_shadow_only());
        assert!((g[rt.camera].global_position() - vec(rt.view_pos)).norm() < 0.001);
        let arm = rt.first_person_arm.as_ref().unwrap();
        assert_eq!(arm.character, id);
        let free = matches!(id, 3 | 7 | 9);
        arm.verify_anatomical_hand(g, rt.camera, held && !free);
        if held && !free {
            assert!(arm.grip_error(g, rt.wave_hand, rt.weapons[id as usize]) < 0.001);
        }
        if wave_pose(TICKS[step % 6]).0 == 1.0 {
            arm.verify_greeting_in_view(g, rt.camera);
            arm.verify_sleeve_corner(g, rt.camera);
        }
    }
    let tag = format!(
        "greeting-{step:02}-{}-{state}-",
        characters::SLUGS[id as usize]
    );
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
