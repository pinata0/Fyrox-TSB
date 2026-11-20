use super::*;
use trickcal_shooting_balddagu::{
    chloe_skill::{self as c, Phase as Action, State},
    motion::{LocomotionPose, Motion, Pose},
};
const SAMPLES: &[(bool, Motion, f32, bool)] = &[
    (false, Motion::Idle, 0.5, false),
    (false, Motion::Walk, 0.24, false),
    (false, Motion::ChloeAttack, 0.12, true),
    (false, Motion::ChloeAttack, 0.27, true),
    (false, Motion::ChloeMount, 0.6, false),
    (false, Motion::ChloeMount, 1.43, false),
    (false, Motion::ChloeRideIdle, 0.5, false),
    (false, Motion::ChloeRideMove, 0.25, false),
    (false, Motion::ChloeRideAttack, 0.27, true),
    (false, Motion::ChloeRoar, 0.8, false),
    (false, Motion::ChloeDismount, 0.7, false),
    (false, Motion::ChloeDismount, 1.9, false),
    (true, Motion::Idle, 0.5, false),
    (true, Motion::ChloeAttack, 0.27, false),
    (true, Motion::ChloeMount, 1.43, false),
    (true, Motion::ChloeRideIdle, 0.5, false),
    (true, Motion::ChloeRideMove, 0.25, false),
    (true, Motion::ChloeRoar, 0.8, false),
    (true, Motion::ChloeDismount, 1.9, false),
    (false, Motion::Idle, 0.7, false),
    (true, Motion::ChloeRideIdle, 0.4, false),
    (true, Motion::ChloeAttack, 0.0, false),
    (true, Motion::ChloeAttack, 0.12, false),
    (true, Motion::ChloeAttack, 0.18, false),
    (true, Motion::ChloeAttack, c::ATTACK_RELEASE, false),
    (true, Motion::ChloeAttack, 0.50, false),
    (true, Motion::ChloeAttack, 0.79, false),
    (true, Motion::ChloeRideAttack, 0.0, false),
    (true, Motion::ChloeRideAttack, 0.12, false),
    (true, Motion::ChloeRideAttack, 0.18, true),
    (true, Motion::ChloeRideAttack, c::ATTACK_RELEASE, true),
    (true, Motion::ChloeRideAttack, 0.50, true),
    (true, Motion::ChloeRideAttack, 0.79, false),
    (true, Motion::ChloeMount, 0.34, false),
    (true, Motion::ChloeMount, 0.36, false),
    (true, Motion::ChloeMount, 1.99, false),
    (true, Motion::ChloeDismount, 0.0, false),
    (true, Motion::ChloeDismount, 1.24, false),
    (true, Motion::ChloeDismount, 1.26, false),
];
pub fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("CHLOE_SKILL_REVIEW_OK: {} source, form, corner swing, rider-hand, face, doll, confetti and duration-bar captures", SAMPLES.len());
        context.loop_controller.exit();
        return Ok(());
    }
    let (first, motion, age, moving) = SAMPLES[rt.review_capture as usize];
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.chat.open = false;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    rt.keys = Keys::default();
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        w.tick = (age * 60.0) as u64;
        w.map.clear();
        w.projectiles.clear();
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0;
            a.character = 6;
            a.shield = 0.0;
            a.combat = Default::default();
            a.hp = MAX_HP;
            a.pos = V3::default();
            a.yaw = 0.0;
            a.pitch = 0.0;
            a.reload = 0.0;
            a.hurt = 0.0;
            a.wave_ticks = 0;
            a.dancing = false;
            a.crouching = false;
            a.ability_flash = 0.0;
        }
        let phase = match motion {
            Motion::ChloeAttack => Action::Attack,
            Motion::ChloeMount => Action::Mount,
            Motion::ChloeRoar => Action::Roar,
            Motion::ChloeDismount => Action::Dismount,
            Motion::ChloeRideAttack => Action::RideAttack,
            Motion::ChloeRideIdle | Motion::ChloeRideMove => Action::Ride,
            _ => Action::Off,
        };
        let riding = matches!(phase, Action::Ride | Action::RideAttack | Action::Roar);
        w.actors[0].combat.chloe = State {
            phase,
            elapsed: if phase == Action::Ride { 0.0 } else { age },
            remaining: if riding {
                if rt.review_capture == 20 {
                    1.2
                } else {
                    8.0
                }
            } else {
                0.0
            },
            hits: if riding { 5 } else { 0 },
        };
        rt.view_pos = w.actors[0].eye();
        if rt.review_capture == 19 {
            for i in 0..9 {
                let mut p = trickcal_shooting_balddagu::combat::Projectile::visual(
                    i + 1,
                    0,
                    ProjectileKind::Doll,
                    V3::new(
                        (i % 3) as f32 * 0.65 - 1.0,
                        0.35,
                        (i / 3) as f32 * 0.6 + 0.3,
                    ),
                );
                p.velocity = V3::new(0.0, 0.0, 7.0);
                w.projectiles.push(p);
            }
            let mut p = trickcal_shooting_balddagu::combat::Projectile::visual(
                20,
                0,
                ProjectileKind::Confetti,
                V3::new(1.0, 0.7, 1.5),
            );
            p.age = 0.3;
            p.life = 0.4;
            w.projectiles.push(p);
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    rt.visuals[0].seek(
        g,
        Pose {
            motion,
            age,
            shot_age: age,
            locomotion: if moving {
                Some(LocomotionPose {
                    motion: if motion == Motion::ChloeRideAttack {
                        Motion::ChloeRideMove
                    } else {
                        Motion::Walk
                    },
                    age: 0.24,
                })
            } else {
                None
            },
        },
    );
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let position = Vector3::new(-1.9, 2.1, 3.0);
        let target = Vector3::new(0.0, 1.5, 0.0);
        g[rt.camera]
            .local_transform_mut()
            .set_position(position)
            .set_rotation(UnitQuaternion::face_towards(
                &(target - position),
                &Vector3::y(),
            ));
    }
    Ok(())
}
pub fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let (first, motion, age, _) = SAMPLES[rt.review_capture as usize];
    let g = &context.scenes[rt.scene].graph;
    assert_eq!(rt.visuals[0].pose().motion, motion);
    for side in ["L", "R"] {
        let nodes = ["UpperArm", "Forearm", "Hand"].map(|part| {
            g.traverse_handle_iter(rt.visuals[0].root)
                .find(|h| g[*h].name() == format!("{part}.{side}"))
                .unwrap()
        });
        let lengths = [
            (g[nodes[1]].global_position() - g[nodes[0]].global_position()).norm(),
            (g[nodes[2]].global_position() - g[nodes[1]].global_position()).norm(),
        ];
        let expected = if !super::chloe_arms::carrying(rt.visuals[0].pose()) {
            super::chloe_arms::NORMAL_LENGTHS[usize::from(side == "R")]
        } else if side == "L" {
            [0.520284, 0.411568]
        } else {
            [0.501531, 0.396734]
        };
        for (actual, target) in lengths.into_iter().zip(expected) {
            assert!(
                (actual - target).abs() < 0.003,
                "Chloe arm stretched in {motion:?}: {actual} vs {target}"
            );
        }
    }

    if first {
        let human = c::view_hands(motion, age);
        for arm in rt
            .first_person_arm
            .iter()
            .chain(rt.first_person_support_arm.iter())
        {
            assert_eq!(arm.visible(g), human, "wrong Chloe hand form");
            if human {
                arm.verify_chloe_view(g, rt.camera, rt.visuals[0].pose());
            }
        }
        for h in rt.chloe_fx.as_ref().unwrap().hands {
            assert!(
                g[h].visibility(),
                "Sebastian arms must remain visible with rider hands"
            );
            let scale = g[h].local_transform().scale().x;
            assert!((scale - (1.0 + 0.38 * c::view_ride(motion, age))).abs() < 0.001);
            let shoulder = g[h]
                .global_transform()
                .transform_point(&Vector3::new(0.0, 0.0, -0.88).into());
            let clip = g[rt.camera]
                .cast::<fyrox::scene::camera::Camera>()
                .unwrap()
                .view_projection_matrix()
                * shoulder.to_homogeneous();
            assert!(
                clip.w <= 0.02 || clip.x.abs() > clip.w || clip.y.abs() > clip.w,
                "Sebastian shoulder cut entered the screen in {motion:?}: {clip:?}"
            );
        }
    }
    capture_preview(
        rt.scene,
        6,
        &format!(
            "chloe-skills-{:02}-{}-{motion:?}-",
            rt.review_capture,
            if first { "fp" } else { "world" }
        ),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
