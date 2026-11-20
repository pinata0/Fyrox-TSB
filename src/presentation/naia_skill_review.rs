//! Real GPU checkpoints for E transitions, Q and both camera perspectives.
use super::*;
use trickcal_shooting_balddagu::naia_skill::{
    Phase as SkillPhase, State, ACTIVE_SECONDS, START_SECONDS, TOTAL_SECONDS,
};
const SAMPLES: [(SkillPhase, f32); 16] = [
    (SkillPhase::Start, 0.0),
    (SkillPhase::Start, 0.2),
    // Highest source wrist positions are the worst cases for exposed sleeve ends.
    (SkillPhase::Start, 0.4),
    (SkillPhase::Start, 0.7),
    (SkillPhase::Move, START_SECONDS + 0.4),
    (SkillPhase::Fire, START_SECONDS + 0.6),
    (SkillPhase::Fire, START_SECONDS + 0.78),
    (SkillPhase::End, START_SECONDS + ACTIVE_SECONDS + 2.0 / 30.0),
    (SkillPhase::End, START_SECONDS + ACTIVE_SECONDS + 0.45),
    (SkillPhase::End, TOTAL_SECONDS - 0.02),
    (SkillPhase::Off, 0.0),
    (SkillPhase::Ultimate, 0.75),
    (SkillPhase::Ultimate, 1.25),
    (SkillPhase::Ultimate, 1.8),
    (SkillPhase::Ultimate, 2.6),
    (SkillPhase::Ultimate, 3.4),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture == 0 && std::env::args().any(|arg| arg == "--naia-first-person-only") {
        rt.review_capture = SAMPLES.len() as u8;
    }
    if rt.review_capture as usize >= SAMPLES.len() * 2 {
        println!("NAIA_SKILL_REVIEW_OK: enlarged E dolphin/hands/sleeves, sleeve cuts outside the camera, restored idle/Q, palm contacts");
        context.loop_controller.exit();
        return Ok(());
    }
    let (phase, elapsed) = SAMPLES[rt.review_capture as usize % SAMPLES.len()];
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
        w.tick = (elapsed * 60.0) as u64;
        w.map.clear();
        w.projectiles.clear();
        // A short wall-limited jet keeps the full silhouette inside the world camera.
        w.map.push(trickcal_shooting_balddagu::game::Block {
            center: V3::new(0.0, 2.0, -6.0),
            size: V3::new(5.0, 4.0, 0.2),
            color: [0; 3],
            phaseable: false,
        });
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0;
        }
        let a = &mut w.actors[0];
        a.character = 1;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.ammo = 24;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        a.combat.naia = State {
            phase,
            elapsed,
            pulse: 0.0,
        };
    }
    if let Session::Solo(w) = &mut rt.session {
        if phase == SkillPhase::Ultimate && elapsed >= 1.5 {
            let mut p = trickcal_shooting_balddagu::combat::Projectile::visual(
                1,
                0,
                ProjectileKind::Wave,
                V3::new(0.0, 0.0, -16.5 + (elapsed - 1.5) * 10.0),
            );
            p.velocity = V3::new(0.0, 0.0, 10.0);
            p.age = elapsed - 1.5;
            w.projectiles.push(p);
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    if (rt.review_capture as usize) < SAMPLES.len() {
        let g = &mut context.scenes[rt.scene].graph;
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let position = if phase == SkillPhase::Ultimate {
            Vector3::new(2.0, 3.8, -4.7)
        } else if phase == SkillPhase::Fire {
            Vector3::new(3.7, 2.5, -8.5)
        } else {
            Vector3::new(2.3, 1.7, -8.9)
        };
        let target = if phase == SkillPhase::Fire {
            Vector3::new(0.0, 1.15, -9.4)
        } else {
            Vector3::new(0.0, 1.0, -12.0)
        };
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
pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let g = &context.scenes[rt.scene].graph;
    let (phase, _) = SAMPLES[rt.review_capture as usize % SAMPLES.len()];
    let pose = rt.visuals[0].pose();
    if let Some(motion) = phase.motion() {
        assert_eq!(pose.motion, motion);
    } else {
        assert!(!pose.motion.is_naia_skill());
    }
    assert_eq!(
        g[rt.naia_water.as_ref().unwrap().nodes[0]].visibility(),
        phase == SkillPhase::Fire
    );
    if rt.review_capture as usize >= SAMPLES.len() {
        for arm in rt
            .first_person_arm
            .iter()
            .chain(rt.first_person_support_arm.iter())
        {
            arm.verify_naia_view(g, rt.camera, pose);
            if phase != SkillPhase::Ultimate {
                assert!(
                    arm.grip_error(g, rt.wave_hand, rt.weapons[1]) < 0.001,
                    "Naia E grip detached"
                );
            }
        }
        let amount = trickcal_shooting_balddagu::naia_skill::whale_form(pose.motion, pose.age);
        assert!(
            (g[rt.weapons[1]].local_transform().scale().x
                - (0.72 + amount * 0.25) * naia_whales::view_scale(pose))
            .abs()
                < 1e-6,
            "dolphin and hands must share the E view enlargement"
        );
        if amount == 1.0 && phase != SkillPhase::Ultimate {
            let direction = skeleton::world_rotation(g, rt.weapons[1]) * Vector3::z();
            let aim = skeleton::world_rotation(g, rt.camera) * Vector3::z();
            assert!(direction.dot(&aim) > 0.9999, "E muzzle must point forward");
            let body = g[rt.weapons[1]]
                .children()
                .iter()
                .find(|h| g[**h].name() == "Naia transforming gun")
                .unwrap();
            assert!(
                g[*body].local_transform().rotation().angle() < 1e-6,
                "dolphin must not turn sideways during E"
            );
        }
        if phase == SkillPhase::Fire {
            let muzzle = g[rt.weapons[1]]
                .global_transform()
                .transform_point(&naia_whales::MUZZLE.into())
                .coords;
            assert!(
                (g[rt.naia_water.as_ref().unwrap().nodes[0]].global_position() - muzzle).norm()
                    < 0.001,
                "healing water must leave the forward muzzle"
            );
        }
    }
    if phase == SkillPhase::Ultimate {
        assert!(
            !g[rt.weapons[1]].visibility(),
            "ultimate must leave empty hands"
        );
        rt.visuals[0].verify_naia_ultimate(g);
    }
    capture_preview(
        rt.scene,
        1,
        &format!("naia-skill-{:02}-{:?}-", rt.review_capture, phase),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
