//! GPU regression fixtures for every equipped hand, combat motion and visibility transition.
use super::*;

const SWING_PHASES: [f32; 9] = [0.0, 0.08, 0.16, 0.29, 0.42, 0.52, 0.62, 0.80, 1.0];
const ERPIN_STEPS: [u8; 7] = [2, 17, 32, 46, 47, 48, 49];
const RENUA_STEPS: [u8; 5] = [7, 22, 37, 79, 94];
const HILDE_STEPS: [u8; 5] = [13, 28, 43, 85, 100];
const SHION_STEPS: [u8; 5] = [14, 45, 45, 14, 46];
const WORLD_STEPS: [u8; 30] = [
    72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95,
    96, 97, 98, 99, 100, 101,
];
const SUPPORT_STEPS: [u8; 16] = [
    4, 11, 12, 14, 19, 26, 27, 29, 76, 83, 84, 86, 91, 98, 99, 101,
];

fn selected_steps() -> Option<&'static [u8]> {
    if std::env::args().any(|arg| arg == "--world-weapons-only") {
        Some(&WORLD_STEPS)
    } else if std::env::args().any(|arg| arg == "--shion-only") {
        Some(&SHION_STEPS)
    } else if std::env::args().any(|arg| arg == "--hilde-only") {
        Some(&HILDE_STEPS)
    } else if std::env::args().any(|arg| arg == "--support-grips-only") {
        Some(&SUPPORT_STEPS)
    } else if std::env::args().any(|arg| arg == "--renua-only") {
        Some(&RENUA_STEPS)
    } else if std::env::args().any(|arg| arg == "--erpin-only") {
        Some(&ERPIN_STEPS)
    } else {
        None
    }
}

pub(super) fn attack_pose(
    rt: &Runtime,
    id: u8,
) -> Option<trickcal_shooting_balddagu::motion::Pose> {
    use trickcal_shooting_balddagu::motion::{Motion, Pose};
    let step =
        selected_steps().map_or(rt.review_capture, |steps| steps[rt.review_capture as usize]);
    // A roster switch starts Spawn, which intentionally hides the world prop.
    // Sample a stable idle clock as well as attacks so fixture timing cannot
    // mistake a long entrance animation for a missing/small weapon.
    if step == 70 || (72..87).contains(&step) {
        return Some(Pose {
            motion: Motion::Idle,
            age: 0.0,
            shot_age: 100.0,
            locomotion: None,
        });
    }
    let phase = match step {
        15..30 => 0.4,
        50..68 => SWING_PHASES[(step as usize - 50) % 9],
        71 | 87..102 => 0.5,
        _ => return None,
    };
    Some(Pose {
        motion: Motion::Attack,
        age: phase * Motion::Attack.seconds(id),
        shot_age: 0.0,
        locomotion: None,
    })
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let erpin_only = std::env::args().any(|arg| arg == "--erpin-only");
    if selected_steps().is_some_and(|steps| rt.review_capture as usize >= steps.len()) {
        if std::env::args().any(|arg| arg == "--world-weapons-only") {
            println!("WORLD_WEAPONS_OK: 15 world idle/attack poses; contact error < 1 mm");
        } else if std::env::args().any(|arg| arg == "--shion-only") {
            println!("SHION_SCOPE_REVIEW_OK: idle, scope, scoped hit, release, selection");
        } else if std::env::args().any(|arg| arg == "--hilde-only") {
            println!("HILDE_WEAPON_REVIEW_OK: first-person idle/attack/reload, world idle/attack; contact error < 1 mm");
        } else if erpin_only {
            println!("ERPIN_WEAPON_REVIEW_OK: idle, attack, reload, selection, dance, greeting, return; contact error < 1 mm");
        } else if std::env::args().any(|arg| arg == "--support-grips-only") {
            println!(
                "SUPPORT_GRIPS_OK: first-person and world idle/attack, articulated palm contacts"
            );
        } else {
            println!("RENUA_WEAPON_REVIEW_OK: first-person idle/attack/reload; world idle/attack without duplicate hand circle");
        }
        context.loop_controller.exit();
        return Ok(());
    }
    let vivi_only = std::env::args().any(|arg| arg == "--vivi-only");
    if rt.review_capture == 0 && vivi_only {
        rt.review_capture = 50;
    }
    if rt.review_capture >= if vivi_only { 72 } else { 102 } {
        if !vivi_only {
            println!("WEAPON_ROSTER_OK: 15 idle, 15 attack, 15 reload, zoom, selection, dance, greeting, return");
        }
        println!("WEAPON_REVIEW_OK: Vivi left/right screen sides, 18 swing phases, greeting/return, world idle/attack; contact error < 1 mm");
        if !vivi_only {
            println!("RIGHT_HAND_ROSTER_OK: 15 anatomical right arms, 15 world idle/attack grips; contact error < 1 mm");
        }
        context.loop_controller.exit();
        return Ok(());
    }
    let step = if let Some(steps) = selected_steps() {
        steps[rt.review_capture as usize]
    } else {
        rt.review_capture
    };
    let id = if erpin_only {
        2
    } else if step < 45 {
        step % 15
    } else if step >= 72 {
        (step - 72) % 15
    } else if step >= 50 {
        0
    } else if step == 45 {
        14
    } else {
        11
    };
    rt.review_frames = rt.review_frames.max(1);
    rt.home_open = false;
    rt.lobby.open = false;
    rt.grabbed = true;
    rt.selecting = step == 46;
    rt.selected = id as usize;
    rt.keys.zoom = step == 45;
    rt.keys.w = step < 45 && step / 15 == 1;
    rt.yaw = if step < 45 {
        (id as f32 - 7.0) * 0.08
    } else {
        0.0
    };
    rt.pitch = if step < 45 {
        (id as f32 - 7.0) * 0.06
    } else {
        0.0
    };
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Playing;
        w.tick += 1;
        let actor = &mut w.actors[0];
        actor.character = id;
        actor.pos = V3::new(0.0, 0.0, -12.0);
        actor.hp = MAX_HP;
        actor.yaw = rt.yaw;
        actor.pitch = rt.pitch;
        actor.dancing = step == 47;
        actor.wave_ticks = if step == 48 || step == 68 { 90 } else { 0 };
        actor.reload = if (30..45).contains(&step) && kit(id).mode != AttackMode::Melee {
            0.8
        } else {
            0.0
        };
        actor.ammo = kit(id).magazine;
        actor.hit = if std::env::args().any(|arg| arg == "--shion-only") && rt.review_capture == 2 {
            0.2
        } else {
            0.0
        };
        if step >= 70 {
            actor.shot = if step == 71 || step >= 87 { 3 } else { 2 };
        }
        // Hold a visible point in the attack animation while still exercising the normal renderer.
        rt.tracer_time[0] = if (50..68).contains(&step) {
            actor.shot = 1 + (step - 50) as u32 / 9;
            rt.last_shots[0] = actor.shot;
            weapons::VIVI_SWING_SECONDS * (1.0 - SWING_PHASES[(step as usize - 50) % 9])
                + context.dt
        } else if (15..30).contains(&step) {
            if kit(id).mode == AttackMode::Melee {
                0.12
            } else {
                0.075
            }
        } else {
            0.0
        };
    }
    render(rt, context)?;
    if step >= 70 {
        let g = &mut context.scenes[rt.scene].graph;
        rt.visuals[0].set_shadow_only(g, false);
        {
            let player = g[rt.visuals[0].player]
                .cast_mut::<fyrox::scene::animation::AnimationPlayer>()
                .unwrap();
            for animation in player.animations_mut().iter_mut() {
                let attack = step == 71 || step >= 87;
                animation.set_time_slice(if attack {
                    156.0 / 30.0..180.0 / 30.0
                } else {
                    0.0..2.0
                });
                animation.set_time_position(if attack { 168.0 / 30.0 } else { 0.0 });
                animation.set_speed(0.0).set_enabled(true);
            }
        }
        for (index, actor) in rt.actors.iter().enumerate() {
            g[*actor].set_visibility(index == 0);
        }
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let side_view = std::env::args().any(|arg| arg == "--weapon-side-view");
        let angle: f32 = if side_view { 1.15 } else { 0.0 };
        g[rt.camera]
            .local_transform_mut()
            .set_position(Vector3::new(
                -2.9 * angle.sin(),
                1.2,
                -12.0 + 2.9 * angle.cos(),
            ))
            .set_rotation(UnitQuaternion::from_euler_angles(
                0.0,
                std::f32::consts::PI - angle,
                0.0,
            ));
    }
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let g = &context.scenes[rt.scene].graph;
    let step = if let Some(steps) = selected_steps() {
        steps[rt.review_capture as usize]
    } else {
        rt.review_capture
    };
    let id = rt.session.world().actors[0].character;
    let held = step < 45 || step == 49 || (50..68).contains(&step) || step == 69;
    assert_eq!(g[rt.weapon].visibility(), held);
    assert_eq!(
        g[rt.wave_hand].visibility(),
        held || step == 48 || step == 68
    );
    assert_eq!(rt.orbit.active, step == 47);
    let ui = context.user_interfaces.first();
    assert_eq!(ui[rt.hud.scope.root].visibility(), step == 45);
    assert_eq!(rt.hud.scope.blur.active(), step == 45);
    if step == 45 {
        assert!(
            !ui[rt.hud.cross.root].visibility(),
            "Scope replaces the hip-fire crosshair"
        );
    }
    if step >= 70 {
        let weapon = g
            .traverse_handle_iter(rt.actors[0])
            .find(|h| g[*h].name() == kit(id).weapon)
            .unwrap();
        assert_eq!(g[weapon].visibility(), !matches!(id, 3 | 6));
        for left in [false, true] {
            if id == 0 {
                let blade = weapons::vivi_blade_node(g, weapon, left);
                assert!(
                    (g[blade].global_position() - rt.visuals[0].palm_contact(g, left)).norm()
                        > 0.12,
                    "Vivi's sword must float independently of the hand"
                );
                rt.visuals[0].verify_hand_grip(g, left);
                continue;
            }
            if left && (weapons::grips(id)[1].is_none() || matches!(id, 3 | 6)) {
                continue;
            }
            let name = skeleton::arm_bone_names(left)[2];
            let hand = g
                .traverse_handle_iter(rt.visuals[0].root)
                .find(|h| g[*h].name() == name)
                .unwrap();
            let blade = weapons::grip_node(g, weapon, id, left);
            let grip = weapons::grips(id)[usize::from(left)].unwrap();
            let contact = g[blade]
                .global_transform()
                .transform_point(&grip.into())
                .coords;
            assert!(
                (contact - rt.visuals[0].palm_contact(g, left)).norm() < 0.001,
                "{} world grip detached (left={left}, error={})",
                characters::SLUGS[id as usize],
                (contact - rt.visuals[0].palm_contact(g, left)).norm()
            );
            if !matches!(id, 3 | 6) {
                rt.visuals[0].verify_hand_grip(g, left);
            }
            // A supporting arm can cross the midline to reach a shared prop.
            // Bone identity, handedness and actual palm contact are checked above.
            if step < 87 && step != 71 && weapons::grips(id)[1].is_none() {
                let position = g[rt.actors[0]]
                    .global_transform()
                    .try_inverse()
                    .unwrap()
                    .transform_point(&g[hand].global_position().into());
                assert!(
                    if left {
                        position.x > 0.0
                    } else {
                        position.x < 0.0
                    },
                    "{} wrong anatomical hand",
                    characters::SLUGS[id as usize]
                );
            }
        }
    }
    if held {
        let right = rt
            .first_person_arm
            .as_ref()
            .expect("right hand must be loaded");
        assert_eq!(right.character, id);
        assert_eq!(right.source, rt.visuals[0].root);
        assert_eq!(
            rt.first_person_support_arm.is_some(),
            weapons::grips(id)[1].is_some()
        );
        for arm in std::iter::once(right).chain(rt.first_person_support_arm.iter()) {
            arm.verify_anatomical_hand(g, rt.camera, true);
            let error = arm.grip_error(g, rt.wave_hand, rt.weapons[id as usize]);
            assert!(
                error.is_finite() && error < 0.001,
                "{} hand detached: {error}",
                characters::SLUGS[id as usize]
            );
        }
        if id == 0 {
            let camera_inverse = g[rt.camera].global_transform().try_inverse().unwrap();
            for left in [false, true] {
                let blade = weapons::grip_node(g, rt.weapons[0], id, left);
                let position = camera_inverse.transform_point(&g[blade].global_position().into());
                assert!(
                    position.z > 0.02,
                    "blade must remain in front of the camera"
                );
                assert!(
                    if left {
                        position.x > 0.0
                    } else {
                        position.x < 0.0
                    },
                    "Vivi's hand is on the wrong screen side: left={left}, {position:?}"
                );
            }
        }
    }
    if id == 2 && step == 48 {
        rt.first_person_arm
            .as_ref()
            .unwrap()
            .verify_anatomical_hand(g, rt.camera, false);
    }
    let state = match step {
        0..15 => "idle",
        15..30 => "attack",
        30..45 => "reload",
        45 => "zoom",
        46 => "selection",
        47 => "dance",
        48 => "hello",
        50..59 => "right-slash",
        59..68 => "left-slash",
        68 => "hello",
        70 => "world-idle",
        71 => "world-attack",
        72..87 => "world-idle",
        87..102 => "world-attack",
        _ => "return",
    };
    let mut tag = format!(
        "weapon-{step:02}-{}-{state}-",
        characters::SLUGS[id as usize]
    );
    if std::env::args().any(|arg| arg == "--shion-only") {
        tag = format!("shion-scope-{}-{tag}", rt.review_capture);
    }
    if step >= 70 && std::env::args().any(|arg| arg == "--weapon-side-view") {
        tag = format!("side-{tag}");
    }
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
