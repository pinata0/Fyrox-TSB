//! Deterministic GPU inspection of Shion's four clips, scope-ready rifle and blast.
use super::*;
use trickcal_shooting_balddagu::{
    motion::{Motion, Pose},
    shion_skill::{self as s, Phase as Action, State},
};
const SAMPLES: &[(bool, Action, f32)] = &[
    (false, Action::Load, 0.8),
    (false, Action::Load, 2.05),
    (false, Action::Ready, 0.0),
    (false, Action::Fire, 0.12),
    (false, Action::End, 0.7),
    (false, Action::Ultimate, 2.60),
    (false, Action::Ultimate, 2.72),
    (false, Action::Ultimate, 2.90),
    (false, Action::Ultimate, 3.35),
    (true, Action::Load, 1.2),
    (true, Action::Ready, 0.0),
    (true, Action::Fire, 0.12),
    (true, Action::End, 0.7),
    (true, Action::Ultimate, 2.90),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("SHION_SKILL_REVIEW_OK: 14 world/first-person frames; load, anchored stance, loop, recovery, frame-80 purple blast");
        context.loop_controller.exit();
        return Ok(());
    }
    let (first, phase, age) = SAMPLES[rt.review_capture as usize];
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
            a.active = i == 0 || i == 6;
            a.shield = 0.0;
        }
        w.actors[6].pos = V3::new(0.0, 0.0, -6.0);
        w.actors[6].hp = MAX_HP;
        w.actors[6].character = 2;
        let a = &mut w.actors[0];
        a.character = 14;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.ammo = 6;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.crouching = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        a.combat.shion = State {
            phase,
            elapsed: age,
        };
        a.combat.magic_ammo = if matches!(phase, Action::Ready | Action::Fire) {
            4
        } else {
            0
        };
        a.shot_end = a.eye() + V3::new(0.0, 0.0, s::BLAST_DISTANCE - 0.05);
        rt.view_pos = a.eye();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    let pose = Pose {
        motion: phase.motion().unwrap_or(Motion::Idle),
        age,
        shot_age: if phase == Action::Fire { 0.04 } else { 100.0 },
        locomotion: None,
    };
    rt.visuals[0].seek(g, pose);
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let (position, target) = if phase == Action::Ultimate {
            (Vector3::new(9.0, 5.0, -13.5), Vector3::new(0.0, 1.2, -8.5))
        } else {
            (Vector3::new(3.3, 2.1, -8.0), Vector3::new(0.0, 1.0, -11.6))
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
    let (first, phase, age) = SAMPLES[rt.review_capture as usize];
    let g = &context.scenes[rt.scene].graph;
    let fx = rt.shion_fx.as_ref().unwrap();
    let blast = phase == Action::Ultimate
        && (s::ULTIMATE_RELEASE..s::ULTIMATE_RELEASE + s::BLAST_SECONDS).contains(&age);
    assert_eq!(
        g[fx.nodes[0][1]].visibility(),
        blast,
        "blast visibility must follow frame 80"
    );
    assert_eq!(rt.visuals[0].pose().motion, phase.motion().unwrap());
    capture_preview(
        rt.scene,
        14,
        &format!(
            "shion-skills-{:02}-{}-{:?}-",
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
