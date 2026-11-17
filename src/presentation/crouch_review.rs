//! Run with --solo --auto-start --shadow-review --crouch-review.
use super::*;
use fyrox::scene::animation::AnimationPlayer;

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let step = rt.review_capture as usize;
    if step == 32 {
        println!("CROUCH_REVIEW_OK: 15 standing/crouched rigs and both first-person heights");
        context.loop_controller.exit();
        return Ok(());
    }
    let id = if step < 30 { (step / 2) as u8 } else { 2 };
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Playing;
        for (i, actor) in w.actors.iter_mut().enumerate() {
            actor.active = i == 0;
        }
        let a = &mut w.actors[0];
        a.character = id;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.hp = MAX_HP;
        a.crouching = step % 2 == 1;
        a.dancing = false;
        a.wave_ticks = 0;
        rt.view_pos = a.eye();
    }
    // Readback happens before rendering: allow one completed frame of this fixed pose.
    rt.review_frames = rt.review_frames.max(22);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    if step < 30 {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let eye = Vector3::new(2.4, 1.45, -8.6);
        let look = Vector3::new(0.0, 0.85, -12.0) - eye;
        g[rt.camera]
            .local_transform_mut()
            .set_position(eye)
            .set_rotation(UnitQuaternion::from_euler_angles(
                -(look.y / look.norm()).asin(),
                look.x.atan2(look.z),
                0.0,
            ));
    }
    for animation in g[rt.visuals[0].player]
        .cast_mut::<AnimationPlayer>()
        .unwrap()
        .animations_mut()
        .iter_mut()
    {
        animation.set_time_position(0.0).set_speed(0.0);
    }
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let step = rt.review_capture;
    let g = &context.scenes[rt.scene].graph;
    let root = rt.visuals[0].root;
    for name in [
        "Hips", "Thigh.L", "Shin.L", "Foot.L", "Thigh.R", "Shin.R", "Foot.R",
    ] {
        let joint = g
            .traverse_handle_iter(root)
            .find(|h| g[*h].name() == name)
            .expect("crouch bone");
        assert!(g[joint].global_position().iter().all(|v| v.is_finite()));
    }
    if step < 30 {
        let positions: Vec<[f32; 3]> = ["Head", "Foot.L", "Foot.R"]
            .iter()
            .map(|name| {
                let joint = g
                    .traverse_handle_iter(root)
                    .find(|h| g[*h].name() == *name)
                    .unwrap();
                g[joint].global_position().into()
            })
            .collect();
        let baseline = format!("artifacts/crouch-rig-{}.json", step / 2);
        if step.is_multiple_of(2) {
            std::fs::write(baseline, serde_json::to_vec(&positions).unwrap()).unwrap();
        } else {
            let standing: Vec<[f32; 3]> =
                serde_json::from_slice(&std::fs::read(baseline).unwrap()).unwrap();
            assert!(
                (standing[0][1] - positions[0][1] - CROUCH_DROP).abs() < 0.002,
                "head height: hero {}",
                step / 2
            );
            for foot in 1..3 {
                assert!(
                    (Vector3::from(standing[foot]) - Vector3::from(positions[foot])).norm() < 0.002,
                    "planted foot: hero {}",
                    step / 2
                );
            }
        }
    }
    if step >= 30 {
        assert!(
            (g[rt.camera].global_position().y - if step == 30 { 1.5 } else { 1.15 }).abs() < 0.001
        );
    }
    capture_preview(rt.scene, 1, &format!("crouch-{step:02}-"), context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
