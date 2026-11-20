//! Deterministic in-engine captures for the two viewpoints and each release seam.
use super::*;
use trickcal_shooting_balddagu::{
    butter_skill::{self as b, Phase as Action, State},
    motion::{Motion, Pose},
};
const SAMPLES: &[(bool, Action, f32)] = &[
    (false, Action::Off, 0.0),
    (false, Action::Draw, 0.30),
    (false, Action::Draw, b::DRAW_SECONDS),
    (false, Action::Release, 0.08),
    (false, Action::Skill, 0.70),
    (false, Action::Skill, 1.14),
    (false, Action::Ultimate, 0.5),
    (false, Action::Ultimate, 1.15),
    (false, Action::Ultimate, 1.70),
    (false, Action::Ultimate, 2.3),
    (false, Action::Ultimate, 2.65),
    (false, Action::Ultimate, 3.65),
    (true, Action::Off, 0.0),
    (true, Action::Draw, b::DRAW_SECONDS),
    (true, Action::Release, 0.08),
    (true, Action::Skill, 0.70),
    (true, Action::Ultimate, 0.5),
    (true, Action::Ultimate, 1.15),
    (true, Action::Ultimate, 1.70),
    (true, Action::Ultimate, 2.3),
    (true, Action::Ultimate, 2.65),
    (true, Action::Ultimate, 3.65),
    (false, Action::Off, 4.1),
    (true, Action::Off, 4.1),
];
fn projectiles(seed: &World, index: usize) -> Vec<trickcal_shooting_balddagu::combat::Projectile> {
    use std::sync::OnceLock;
    static SHOTS: OnceLock<Vec<Vec<trickcal_shooting_balddagu::combat::Projectile>>> =
        OnceLock::new();
    SHOTS.get_or_init(|| {
        SAMPLES
            .iter()
            .map(|&(_, phase, elapsed)| {
                if phase != Action::Ultimate && elapsed < 4.0 {
                    return Vec::new();
                }
                let mut world = seed.clone();
                world.phase = Phase::Playing;
                world.actors[0].combat.butter = State {
                    phase: Action::Ultimate,
                    elapsed: 0.0,
                    fire_down: false,
                };
                for a in &mut world.actors {
                    a.human = true;
                }
                for _ in 0..(elapsed / DT).round() as usize {
                    world.tick([Input::default(); PLAYERS]);
                }
                world.projectiles
            })
            .collect()
    })[index]
        .clone()
}
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("BUTTER_SKILL_REVIEW_OK: {} source-timed captures, two-hand contacts and actual long-range projectile flight",SAMPLES.len());
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
        fire_down: phase == Action::Draw,
    };
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        w.tick = (elapsed * 60.0) as u64;
        w.map.clear();
        w.projectiles.clear();
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0;
            a.shield = 0.0;
        }
        let a = &mut w.actors[0];
        a.character = 11;
        a.pos = V3::new(
            0.0,
            if state.hovering() {
                b::lift(elapsed)
            } else {
                0.0
            },
            -12.0,
        );
        a.hp = MAX_HP;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.ammo = 12;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        a.combat.butter = state;
        rt.view_pos = a.eye();
        w.projectiles = projectiles(w, rt.review_capture as usize);
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
        let position = Vector3::new(3.6, 2.6, -7.0);
        let target = Vector3::new(0.0, 1.0, -11.5);
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
    let (first, phase, elapsed) = SAMPLES[rt.review_capture as usize];
    if !first && phase == Action::Ultimate && (0.95..2.5).contains(&elapsed) {
        let errors = rt.visuals[0].butter_draw_errors(&context.scenes[rt.scene].graph);
        println!("BUTTER_WORLD_GRIP: {elapsed:.2}s {errors:?}");
        assert!(
            errors.iter().all(|e| *e < 0.02),
            "giant fork must stay in both palms: {errors:?}"
        );
    }
    if first {
        let g = &context.scenes[rt.scene].graph;
        assert_eq!(
            g[rt.weapons[11]].visibility(),
            b::weapon_visible(phase.motion().unwrap_or(Motion::Idle), elapsed)
        );
        if phase == Action::Ultimate && (1.27..3.05).contains(&elapsed) {
            assert!(g[rt.weapons[11]].local_transform().scale().x > 3.0);
        }
        if phase == Action::Ultimate && (0.95..2.5).contains(&elapsed) {
            for (left, arm) in [
                (false, &rt.first_person_arm),
                (true, &rt.first_person_support_arm),
            ] {
                let error = (arm.as_ref().unwrap().palm(g)
                    - g[butter_fx::draw_node(g, rt.weapons[11], left)].global_position())
                .norm();
                println!("BUTTER_VIEW_GRIP: {elapsed:.2}s left={left} error={error}");
                assert!(error < 0.002, "first-person palm must touch the pouch");
            }
        }
    }
    capture_preview(
        rt.scene,
        1,
        &format!(
            "butter-skills-{:02}-{}-{:?}-",
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
