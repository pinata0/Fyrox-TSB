//! Real GPU checkpoints for all four E clips and both camera perspectives.
use super::*;
use trickcal_shooting_balddagu::naia_skill::{
    Phase as SkillPhase, State, ACTIVE_SECONDS, START_SECONDS,
};
const SAMPLES: [(SkillPhase, f32); 5] = [
    (SkillPhase::Start, 0.7),
    (SkillPhase::Move, START_SECONDS + 0.4),
    (SkillPhase::Fire, START_SECONDS + 0.6),
    (SkillPhase::Fire, START_SECONDS + 0.78),
    (SkillPhase::End, START_SECONDS + ACTIVE_SECONDS + 0.45),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture >= 10 {
        println!("NAIA_SKILL_REVIEW_OK: four source clips, animated water, five world and five first-person frames");
        context.loop_controller.exit();
        return Ok(());
    }
    let (phase, elapsed) = SAMPLES[rt.review_capture as usize % 5];
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
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    if rt.review_capture < 5 {
        let g = &mut context.scenes[rt.scene].graph;
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let position = if phase == SkillPhase::Fire {
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
    let (phase, _) = SAMPLES[rt.review_capture as usize % 5];
    assert_eq!(rt.visuals[0].pose().motion, phase.motion().unwrap());
    assert_eq!(
        g[rt.naia_water.as_ref().unwrap().nodes[0]].visibility(),
        phase == SkillPhase::Fire
    );
    if rt.review_capture >= 5 {
        for arm in rt
            .first_person_arm
            .iter()
            .chain(rt.first_person_support_arm.iter())
        {
            assert!(
                arm.grip_error(g, rt.wave_hand, rt.weapons[1]) < 0.001,
                "Naia E grip detached"
            );
        }
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
