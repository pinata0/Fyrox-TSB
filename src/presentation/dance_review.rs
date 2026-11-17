//! Capture source-period dance poses through the normal live animation clock.
use super::*;
use trickcal_shooting_balddagu::motion::{dance_ticks, Motion};

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let step = rt.review_capture as usize;
    if step >= 60 {
        println!("DANCE_REVIEW_OK: 15 heroes, four source-phase poses, live FBX playback");
        context.loop_controller.exit();
        return Ok(());
    }
    let id = (step / 4) as u8;
    let phase = (step % 4) as u64;
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.chat.open = false;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        // Avoid sampling identical beats at quarter periods of a repeating track.
        w.tick = dance_ticks(id) * phase / 8;
        for (index, a) in w.actors.iter_mut().enumerate() {
            a.active = index == 0;
        }
        let a = &mut w.actors[0];
        a.character = id;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.dancing = true;
        a.wave_ticks = 0;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat.ultimate_flash = 0.0;
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    rt.visuals[0].set_shadow_only(g, false);
    for (i, a) in rt.actors.iter().enumerate() {
        g[*a].set_visibility(i == 0);
    }
    g[rt.weapon].set_visibility(false);
    g[rt.wave_hand].set_visibility(false);
    g[rt.camera]
        .local_transform_mut()
        .set_position(Vector3::new(0.0, 1.2, -8.5))
        .set_rotation(UnitQuaternion::from_euler_angles(
            0.0,
            std::f32::consts::PI,
            0.0,
        ));
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let id = rt.session.world().actors[0].character;
    let phase = rt.review_capture % 4;
    let duration = Motion::Dance.seconds(id);
    assert!(duration > 0.0);
    assert!(context.scenes[rt.scene].graph[rt.actors[0]].visibility());
    capture_preview(
        rt.scene,
        3,
        &format!(
            "dance-{:02}-{}-phase{phase}-",
            id,
            characters::SLUGS[id as usize]
        ),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
