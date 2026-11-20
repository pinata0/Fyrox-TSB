//! --attack-review --move-review: all 15 heroes, Walk/Run and layered attacks.
use super::*;
use trickcal_shooting_balddagu::motion::{LocomotionPose, Motion, Pose};

pub(super) fn enabled() -> bool {
    std::env::args().any(|arg| arg == "--move-review")
}

fn ui_only() -> bool {
    std::env::args().any(|arg| arg == "--ui-only")
}

fn first_person(step: usize) -> bool {
    ui_only() && step >= 16
}

pub(super) fn pose(rt: &Runtime, id: u8) -> Pose {
    let step = rt.review_capture as usize;
    let mode = step % 16 / 4;
    let phase = [0.12, 0.35, 0.60, 0.85][step % 4];
    let movement = if mode.is_multiple_of(2) {
        Motion::Walk
    } else {
        Motion::Run
    };
    let age = phase * movement.seconds(id);
    if mode < 2 {
        Pose {
            motion: movement,
            age,
            shot_age: 100.0,
            locomotion: Some(LocomotionPose {
                motion: movement,
                age,
            }),
        }
    } else {
        let motion = if mode == 3 && matches!(id, 0 | 4) {
            Motion::Attack2
        } else {
            Motion::Attack
        };
        Pose {
            motion,
            age: phase * motion.seconds(id),
            shot_age: phase * motion.seconds(id),
            locomotion: Some(LocomotionPose {
                motion: movement,
                age,
            }),
        }
    }
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let step = rt.review_capture as usize;
    if step >= if ui_only() { 32 } else { 240 } {
        println!("MOVE_REVIEW_OK: non-arm bones preserve Move during attacks");
        if ui_only() {
            println!("UI_MOVE_REVIEW_OK: 16 world + 16 first-person poses; frog tracks, stable camera, hand/weapon sway and grip verified");
        }
        context.loop_controller.exit();
        return Ok(());
    }
    let id = if ui_only() { 10 } else { (step / 16) as u8 };
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.chat.open = false;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    rt.keys.w = first_person(step);
    rt.keys.zoom = false;
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        w.tick = 120 + if ui_only() { (step % 4) as u64 * 5 } else { 0 };
        for (i, actor) in w.actors.iter_mut().enumerate() {
            actor.active = i == 0;
        }
        let actor = &mut w.actors[0];
        actor.character = id;
        actor.pos = V3::new(0.0, 0.0, -12.0);
        actor.hp = MAX_HP;
        actor.shield = 0.0;
        actor.dancing = false;
        actor.crouching = false;
        actor.wave_ticks = 0;
        actor.yaw = 0.0;
        actor.pitch = 0.0;
        actor.reload = 0.0;
        actor.hurt = 0.0;
        actor.ability_flash = 0.0;
        actor.combat = Default::default();
        rt.view_pos = actor.eye();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    if first_person(step) {
        return Ok(());
    }
    let g = &mut context.scenes[rt.scene].graph;
    rt.visuals[0].set_shadow_only(g, false);
    g[rt.weapon].set_visibility(false);
    g[rt.wave_hand].set_visibility(false);
    g[rt.camera]
        .local_transform_mut()
        .set_position(Vector3::new(1.65, 1.25, -8.7))
        .set_rotation(UnitQuaternion::from_euler_angles(
            0.0,
            std::f32::consts::PI + 0.46,
            0.0,
        ));
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, mut context: PluginContext) -> GameResult {
    use fyrox::generic_animation::value::{TrackValue, ValueBinding};
    use fyrox::scene::animation::AnimationPlayer;
    let id = rt.session.world().actors[0].character;
    let g = &context.scenes[rt.scene].graph;
    let visual = &rt.visuals[0];
    let mut checked = 0;
    for anim in g[visual.player]
        .cast::<AnimationPlayer>()
        .unwrap()
        .animations()
        .iter()
    {
        for (handle, local) in anim.pose().poses() {
            let name = g[*handle].name();
            if !matches!(name, "Hips" | "Torso" | "Head" | "Mount")
                && !name.starts_with("Thigh.")
                && !name.starts_with("Shin.")
                && !name.starts_with("Foot.")
                && !(ui_only() && name.starts_with("Frog"))
            {
                continue;
            }
            let actual = g[*handle].local_transform();
            for value in &local.values.values {
                let error = match (&value.binding, &value.value) {
                    (ValueBinding::Position, TrackValue::Vector3(expected)) => {
                        (**actual.position() - expected).norm()
                    }
                    (ValueBinding::Scale, TrackValue::Vector3(expected)) => {
                        (**actual.scale() - expected).norm()
                    }
                    (ValueBinding::Rotation, TrackValue::UnitQuaternion(expected)) => {
                        actual.rotation().angle_to(expected)
                    }
                    _ => continue,
                };
                assert!(error < 0.0001, "Move overwritten {id}/{name}: {error}");
                checked += 1;
            }
        }
    }
    assert!(checked >= 27, "missing Move body tracks for {id}");
    if first_person(rt.review_capture as usize) {
        let actor = &rt.session.world().actors[0];
        let camera_error = (g[rt.camera].global_position() - vec(actor.eye())).norm();
        assert!(
            camera_error < 0.0001,
            "Ui hop leaked into camera: {camera_error}"
        );
        assert!(rt.visuals[0].is_shadow_only());
        assert!(g[rt.weapon].visibility() && g[rt.wave_hand].visibility());
        let arm = rt.first_person_arm.as_ref().expect("Ui first-person hand");
        assert!(arm.visible(g));
        assert!(arm.grip_error(g, rt.wave_hand, rt.weapons[10]) < 0.001);
        // In non-attacking poses, only the original walking sway moves the grip vertically.
        if rt.review_capture % 16 < 8 {
            let bob = (rt.session.world().tick as f32 * 0.18).sin() * 0.012;
            assert!(bob.abs() > 0.001, "review must sample nonzero sway");
            assert!((g[rt.weapon].local_transform().position().y - bob).abs() < 0.0001);
            assert!((g[rt.wave_hand].local_transform().position().y - bob).abs() < 0.0001);
        }
        println!("UI_MOVE_FIRST_PERSON_OK step={} camera_error={camera_error:.7} weapon_y={:.6} hand_y={:.6}",
            rt.review_capture, g[rt.weapon].local_transform().position().y,
            g[rt.wave_hand].local_transform().position().y);
    }
    let mode = ["walk", "run", "walk-attack", "run-attack"][rt.review_capture as usize % 16 / 4];
    // A full roster produces 240 screenshots. JPEG keeps review artifacts small.
    let (width, height, pixels) = capture_frame(rt.scene, false, &mut context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    let rgba = image::RgbaImage::from_raw(width, height, pixels).unwrap();
    let capture = image::DynamicImage::ImageRgba8(image::imageops::flip_vertical(&rgba)).to_rgb8();
    let path = format!(
        "artifacts/preview-move-{id:02}-{}-{mode}-phase{}-{}.jpg",
        characters::SLUGS[id as usize],
        rt.review_capture % 4,
        if first_person(rt.review_capture as usize) {
            "fps"
        } else {
            "dance-front"
        },
    );
    capture
        .save(&path)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    println!("PREVIEW_OK: {path}");
    println!(
        "MOVE_POSE_OK {id} {mode} {} {checked}",
        rt.review_capture % 4
    );
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
