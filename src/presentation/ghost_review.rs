//! Deterministic GPU captures of wall phasing and ripples on curved statue scenery.
use super::*;

const STAGES: [(&str, usize, bool); 8] = [
    ("before", 0, false),
    ("entry", 10, false),
    ("inside", 14, false),
    ("exit", 22, true),
    ("faded", 45, false),
    ("statue-before", 0, false),
    ("statue-ripple", 8, false),
    ("statue-faded", 45, false),
];

pub(super) fn capture_frames(rt: &Runtime) -> u32 {
    if STAGES
        .get(rt.review_capture as usize)
        .is_some_and(|s| s.0.ends_with("faded"))
    {
        90
    } else {
        36
    }
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let Some(&(_, steps, reverse)) = STAGES.get(rt.review_capture as usize) else {
        println!("GHOST_REVIEW_OK: wall entry/exit/fade and statue surface ripple/fade");
        context.loop_controller.exit();
        return Ok(());
    };
    rt.home_open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = if reverse {
        std::f32::consts::FRAC_PI_2
    } else {
        -std::f32::consts::FRAC_PI_2
    };
    rt.pitch = -0.1;
    let statue = rt.review_capture >= 5;
    if statue {
        rt.yaw = -1.1;
        rt.pitch = -0.04;
    }
    if let Session::Solo(w) = &mut rt.session {
        if rt.review_frames == 0 {
            w.phase = Phase::Playing;
            for (i, a) in w.actors.iter_mut().enumerate() {
                a.human = true;
                a.pos = V3::new(0.0, 0.0, -13.0 - i as f32 * 0.1);
                a.shield = 0.0;
                a.ghost_phase = 0.0;
                a.phase_skill = Default::default();
            }
            w.actors[6].character = 14;
            w.actors[6].active = true;
            w.actors[6].hp = MAX_HP;
            w.actors[6].pos = if statue {
                V3::new(26.1, 0.0, 26.8)
            } else {
                V3::new(-4.7, 0.0, -23.5)
            };
            let mut inputs = [Input::default(); PLAYERS];
            inputs[6] = Input {
                phase_shift: true,
                forward: 1.0,
                yaw: -std::f32::consts::FRAC_PI_2,
                ..Default::default()
            };
            for _ in 0..steps {
                w.tick(inputs);
            }
            if steps == 0 {
                assert_eq!(w.actors[6].ghost_phase, 0.0);
            }
            if (10..=22).contains(&steps) || (statue && steps == 8) {
                assert!(w.actors[6].ghost_phase > 0.0);
            }
            if steps >= 22 && !statue {
                assert!(w.actors[6].pos.x < -6.7);
            }
            if statue && steps == 45 {
                w.actors[6].ghost_phase = 0.0;
            }
        }
        w.actors[0].pos = if statue {
            V3::new(28.5, 0.0, 25.1)
        } else {
            V3::new(if reverse { -8.9 } else { -3.5 }, 0.0, -23.5)
        };
        w.actors[0].yaw = rt.yaw;
        w.actors[0].pitch = rt.pitch;
        rt.view_pos = w.actors[0].eye();
    }
    rt.review_frames += 1;
    render(rt, context)?;
    if statue {
        // Keep the same unobstructed view for before/ripple/fade comparison.
        let g = &mut context.scenes[rt.scene].graph;
        for node in rt.actors.iter().chain(rt.team_marks.iter()) {
            g[*node].set_visibility(false);
        }
        g[rt.weapon].set_visibility(false);
        let ui = context.user_interfaces.first_mut();
        ui.send(ui.root(), WidgetMessage::Visibility(false));
    }
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if !context
        .graphics_context
        .as_initialized_ref()
        .renderer
        .scene_data_map
        .contains_key(&rt.scene)
    {
        return Ok(());
    }
    let tag = format!("ghost-{}-", STAGES[rt.review_capture as usize].0);
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
