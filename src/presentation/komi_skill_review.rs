//! Reproducible full-body and first-person review of both Komi actions.
use super::*;
use trickcal_shooting_balddagu::{
    komi_skill::{Phase as Action, State},
    motion::{Motion, Pose},
};
const SAMPLES: &[(bool, Action, f32)] = &[
    (false, Action::Rest, 0.4),
    (false, Action::Rest, 1.4),
    (false, Action::Rest, 3.7),
    (false, Action::Feast, 0.5),
    (false, Action::Feast, 1.8),
    (false, Action::Feast, 3.8),
    (false, Action::Giant, 2.0),
    (true, Action::Rest, 1.4),
    (true, Action::Feast, 1.8),
    (true, Action::Giant, 2.0),
    (true, Action::Off, 0.0),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!(
            "KOMI_SKILL_REVIEW_OK: blanket, food tin, growth, empty hands and restored pillow"
        );
        context.loop_controller.exit();
        return Ok(());
    }
    let (first, phase, elapsed) = SAMPLES[rt.review_capture as usize];
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.chat.open = false;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    rt.keys = Keys::default();
    let state = State {
        phase,
        elapsed,
        impact: if phase == Action::Giant { 0.2 } else { 0.0 },
        ..Default::default()
    };
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        w.tick = (elapsed * 60.0) as u64;
        w.map.clear();
        w.projectiles.clear();
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0 || i == 6;
            a.shield = 0.0;
        }
        w.actors[6].pos = V3::new(2.2, 0.0, -11.0);
        w.actors[6].character = 12;
        w.actors[6].combat = Default::default();
        let a = &mut w.actors[0];
        a.character = 12;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        a.combat.komi = state;
        rt.view_pos = a.eye();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    rt.visuals[0].seek(
        g,
        Pose {
            motion: phase.motion().unwrap_or(Motion::Idle),
            age: state.motion_age(),
            shot_age: 100.0,
            locomotion: None,
        },
    );
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let position = Vector3::new(3.3, 2.6, -7.6);
        let target = Vector3::new(0.0, if phase == Action::Giant { 1.2 } else { 0.8 }, -11.9);
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
    let (first, phase, _) = SAMPLES[rt.review_capture as usize];
    let g = &context.scenes[rt.scene].graph;
    let fx = rt.komi_fx.as_ref().unwrap();
    assert_eq!(g[fx.world[0].blanket].visibility(), phase == Action::Rest);
    let props = if first { &fx.view } else { &fx.world[0] };
    assert_eq!(g[props.can].visibility(), phase == Action::Feast);
    if first {
        assert_eq!(g[rt.weapons[12]].visibility(), phase == Action::Off);
        assert!(rt.first_person_arm.as_ref().unwrap().visible(g));
        assert_eq!(rt.first_person_support_arm.as_ref().unwrap().visible(g), phase != Action::Off);
    }
    capture_preview(
        rt.scene,
        1,
        &format!(
            "komi-skills-{:02}-{}-{:?}-",
            rt.review_capture,
            if first { "fp" } else { "world" },
            phase
        ),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
