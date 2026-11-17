//! Deterministic GPU review of both colors, parallax and the split cooldown tile.
use super::*;
use trickcal_shooting_balddagu::portal::Portal;

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture >= 6 {
        println!("PORTAL_REVIEW_OK: yellow/blue live views, parallax, foreground occlusion, unpaired entrance, authored castle wall, split cooldowns");
        context.loop_controller.exit();
        return Ok(());
    }
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.keys = Keys::default();
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Playing;
        w.tick = 120;
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0;
        }
        let a = &mut w.actors[0];
        a.character = 3;
        a.hp = MAX_HP;
        a.ammo = kit(3).magazine;
        a.combat = Default::default();
        a.pos = match rt.review_capture {
            1 => V3::new(108.7, 0.0, -29.0),
            2 => V3::new(130.0, 0.0, -30.0),
            _ => V3::new(110.0, 0.0, -30.0),
        };
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.portals.ends = [
            Some(Portal {
                center: V3::new(110.0, 1.5, -24.15),
                face: 5,
            }),
            Some(Portal {
                center: V3::new(130.0, 1.5, -24.15),
                face: 5,
            }),
        ];
        a.portals.cooldown = match rt.review_capture {
            0 => [5.0, 2.3],
            1 => [0.0, 0.7],
            _ => [1.8, 0.0],
        };
        if rt.review_capture == 4 {
            a.portals.ends[1] = None;
        }
        if rt.review_capture == 5 {
            a.pos = V3::new(0.0, 0.0, -35.0);
            a.yaw = std::f32::consts::PI;
            rt.yaw = a.yaw;
            a.portals.ends = [
                trickcal_shooting_balddagu::portal::placement(
                    &w.map,
                    a.eye(),
                    direction(a.yaw, 0.0),
                ),
                trickcal_shooting_balddagu::portal::placement(
                    &w.map,
                    V3::new(9.8, 1.5, -35.0),
                    direction(a.yaw, 0.0),
                ),
            ];
            assert!(a.portals.ends.iter().all(Option::is_some));
            a.portals.cooldown = [0.0, 0.0];
        }
        rt.view_pos = a.eye();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    if !g.linear_iter().any(|n| n.name() == "Portal review wall") {
        let shapes = Shapes::new();
        shapes.solid(
            g,
            "Portal occlusion check",
            V3::new(110.0, 1.5, -27.0),
            V3::new(0.35, 3.0, 0.3),
            [120, 80, 130],
            0,
        );
        for x in [110.0, 130.0] {
            shapes.solid(
                g,
                "Portal review wall",
                V3::new(x, 2.5, -24.0),
                V3::new(8.0, 5.0, 0.3),
                [130, 145, 155],
                0,
            );
            shapes.solid(
                g,
                "Portal review floor",
                V3::new(x, -0.1, -30.0),
                V3::new(12.0, 0.2, 16.0),
                [92, 112, 124],
                0,
            );
            for i in 0..4 {
                shapes.solid(
                    g,
                    "Portal destination columns",
                    V3::new(x - 3.0 + i as f32 * 2.0, 1.3, -34.0),
                    V3::new(0.5, 2.6, 0.5),
                    if x > 120.0 {
                        [56, 180, 223]
                    } else {
                        [240, 183, 70]
                    },
                    0,
                );
            }
            shapes.solid(
                g,
                "Portal destination pedestal",
                V3::new(x, 0.8, -32.0),
                V3::new(1.3, 1.6, 1.0),
                [230, 106, 123],
                0,
            );
        }
    }
    let occluder = g
        .pair_iter()
        .find(|(_, n)| n.name() == "Portal occlusion check")
        .map(|(h, _)| h);
    if let Some(h) = occluder {
        g[h].set_visibility(rt.review_capture == 3);
    }
    Ok(())
}
pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let g = &context.scenes[rt.scene].graph;
    let surface = g
        .linear_iter()
        .find(|n| n.name() == "Portal aperture 0")
        .unwrap();
    let normal =
        surface.global_transform() * fyrox::core::algebra::Vector4::new(0.0, 0.0, 1.0, 0.0);
    let expected = rt.session.world().actors[0].portals.ends[0]
        .unwrap()
        .normal();
    assert!(
        (normal.xyz().normalize() - vec(expected)).norm() < 0.00001,
        "Portal must lie flush with the surface"
    );
    capture_preview(
        rt.scene,
        1,
        &format!("portal-{:02}-", rt.review_capture),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
