//! GPU fixtures: real E activation, damage, expiry and shared shield rendering.
use super::*;

const STAGES: [&str; 7] = [
    "none", "active", "damaged", "expired", "spirit", "giant", "dead",
];

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= STAGES.len() {
        println!("SHIELD_REVIEW_OK: E, damage, expiry, spirit, giant, death; grid spheres and yellow shield HUD");
        context.loop_controller.exit();
        return Ok(());
    }
    rt.home_open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = 0.0;
    rt.pitch = -0.04;
    if let Session::Solo(w) = &mut rt.session {
        if rt.review_frames == 0 {
            w.phase = Phase::Playing;
            for (i, a) in w.actors.iter_mut().enumerate() {
                *a = Actor {
                    human: true,
                    pos: V3::new(25.0 + i as f32 * 2.0, 0.0, 40.0),
                    ..Default::default()
                };
            }
            w.actors[0].pos = V3::new(0.0, 0.0, -12.0);
            w.actors[1].pos = V3::new(1.6, 0.0, -6.5);
            w.actors[1].yaw = std::f32::consts::PI;
            if rt.review_capture > 0 {
                let mut inputs = [Input::default(); PLAYERS];
                inputs[0].ability = true;
                inputs[1].ability = true;
                inputs[1].yaw = std::f32::consts::PI;
                w.tick(inputs);
                assert_eq!(w.actors[0].combat.guard, 45.0);
                assert_eq!(w.actors[1].combat.guard, 45.0);
            }
            match rt.review_capture {
                2 => {
                    w.actors[6].character = 14;
                    w.actors[6].ammo = 6;
                    w.actors[6].pos = V3::new(0.0, 0.0, -8.0);
                    let mut inputs = [Input::default(); PLAYERS];
                    inputs[6] = Input {
                        fire: true,
                        yaw: std::f32::consts::PI,
                        ..Default::default()
                    };
                    w.tick(inputs);
                    assert!(w.actors[0].combat.guard > 0.0 && w.actors[0].combat.guard < 45.0);
                    assert_eq!(w.actors[0].hp, MAX_HP);
                    w.actors[0].hp = 72;
                    w.actors[6].pos.x = 30.0;
                }
                3 => {
                    for _ in 0..365 {
                        w.tick([Input::default(); PLAYERS]);
                    }
                    assert_eq!(w.actors[0].shield_health(), 0.0);
                    assert_eq!(w.actors[1].shield_health(), 0.0);
                }
                4 => {
                    for i in [0, 1] {
                        w.actors[i].character = 1;
                        w.actors[i].combat.guard = 0.0;
                        w.actors[i].barrier = 9.6;
                    }
                }
                5 => {
                    for i in [0, 1] {
                        w.actors[i].combat.giant = 8.0;
                        w.actors[i].combat.guard = 120.0;
                        w.actors[i].barrier = 9.6;
                    }
                }
                6 => {
                    w.actors[0].hp = 0;
                    w.actors[1].hp = 0;
                }
                _ => {}
            }
            rt.view_pos = w.actors[0].eye();
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)
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
    let world = rt.session.world();
    let g = &context.scenes[rt.scene].graph;
    for i in [0, 1] {
        let a = &world.actors[i];
        assert_eq!(
            g[rt.guards[i]].visibility(),
            a.alive() && a.shield_health() > 0.0
        );
        assert!((g[rt.guards[i]].local_transform().scale().x - 1.3 * a.body_scale()).abs() < 0.001);
    }
    let size = context.user_interfaces.first().screen_size();
    let tag = format!(
        "shield-{}x{}-{}-",
        size.x as u32, size.y as u32, STAGES[rt.review_capture as usize]
    );
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
