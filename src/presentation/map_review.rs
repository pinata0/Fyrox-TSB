//! Reproducible in-engine views of the castle, circulation and sanctuary UI.
use super::*;

const VIEWS: [(V3, f32, f32, &str); 19] = [
    (V3::new(-36.0, 59.0, -62.0), 0.73, -0.65, "overview"),
    (V3::new(2.0, 0.0, -17.0), 0.10, 0.13, "gate-a"),
    (V3::new(-12.0, 0.0, -6.5), 0.0, 0.08, "stairs"),
    (
        V3::new(-12.0, 0.0, 12.0),
        std::f32::consts::FRAC_PI_2,
        0.0,
        "ground-floor",
    ),
    (
        V3::new(-11.4, 3.6, 8.4),
        std::f32::consts::FRAC_PI_2,
        -0.18,
        "window",
    ),
    (V3::new(4.0, 0.0, 23.0), 1.8, 0.05, "turn-to-b"),
    (V3::new(33.0, 0.0, 32.0), 1.8, 0.0, "north-flank"),
    (
        V3::new(40.0, 0.0, 15.0),
        std::f32::consts::FRAC_PI_2,
        0.16,
        "b-hall",
    ),
    (
        V3::new(29.5, 0.0, 15.0),
        std::f32::consts::FRAC_PI_2,
        0.12,
        "point-b",
    ),
    (V3::new(5.0, 0.0, -45.0), 0.0, 0.0, "spawn"),
    (
        V3::new(5.0, 0.0, -41.6),
        std::f32::consts::PI,
        0.02,
        "bedroom",
    ),
    (V3::new(-11.8, 0.0, -18.0), -0.03, 0.06, "study"),
    (V3::new(1.0, 0.0, -38.0), 0.05, 0.16, "street"),
    (V3::new(1.0, 0.0, -24.0), 1.25, 0.08, "cafe"),
    (V3::new(5.0, 0.0, -40.0), 1.14, 0.18, "square"),
    (V3::new(11.6, 0.0, 10.8), 0.64, 0.31, "atrium"),
    (V3::new(26.5, 0.0, -1.5), 1.54, 0.09, "corridor"),
    (V3::new(26.0, 0.0, 30.0), 1.75, 0.12, "guardian-hall"),
    (
        V3::new(43.0, 0.0, 15.0),
        std::f32::consts::FRAC_PI_2,
        0.16,
        "throne",
    ),
];

const CITY_VIEWS: [(V3, f32, f32, &str); 11] = [
    (V3::new(-18.0, 50.0, -52.0), 0.55, -0.65, "overview"),
    (V3::new(-2.0, 0.0, -39.0), 0.10, 0.14, "arrival"),
    (V3::new(0.0, 3.6, -26.0), 0.18, 0.08, "overpass"),
    (V3::new(12.0, 0.0, -14.0), -0.25, 0.15, "transit"),
    (V3::new(-6.5, 3.6, 1.0), 1.8, -0.18, "catwalk"),
    (V3::new(-15.0, 0.0, -35.0), 0.0, 0.08, "pc-cafe"),
    (V3::new(25.5, 0.0, 12.5), 0.0, 0.02, "studio"),
    (V3::new(40.0, 0.0, 32.0), 2.3, 0.22, "city-hall"),
    (
        V3::new(43.0, 0.0, 18.0),
        std::f32::consts::FRAC_PI_2,
        0.45,
        "elena-banner",
    ),
    (V3::new(54.0, 0.0, 18.0), -1.8, 0.05, "civic-walkway"),
    (V3::new(-4.0, 0.0, -47.0), 0.0, 0.0, "spawn"),
];

fn views(rt: &Runtime) -> &'static [(V3, f32, f32, &'static str)] {
    if rt.session.world().map_id == MapId::ElfCity {
        &CITY_VIEWS
    } else {
        &VIEWS
    }
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let Some(&(position, yaw, pitch, tag)) = views(rt).get(rt.review_capture as usize) else {
        println!(
            "MAP_REVIEW_OK: {} / {} authored views and sanctuary HUD",
            rt.session.world().map_id.slug(),
            views(rt).len()
        );
        context.loop_controller.exit();
        return Ok(());
    };
    rt.home_open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.review_frames += 1;
    rt.yaw = yaw;
    rt.pitch = pitch;
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Playing;
        w.tick += 1;
        for a in &mut w.actors {
            a.human = true;
        }
        w.actors[0].pos = position;
        w.actors[0].yaw = yaw;
        w.actors[0].pitch = pitch;
        w.actors[0].hp = MAX_HP;
        w.actors[0].dancing = false;
    }
    render(rt, context)?;
    let show_hud = tag == "spawn";
    let scene = &mut context.scenes[rt.scene];
    scene.graph[rt.weapon].set_visibility(show_hud);
    scene.graph[rt.wave_hand].set_visibility(false);
    // Art review views omit actors and the floating objective marker; gameplay
    // and the first ten topology/HUD views retain their normal presentation.
    if rt.session.world().map_id == MapId::FairyKingdomCastle && rt.review_capture >= 10 {
        for node in rt
            .actors
            .iter()
            .chain(rt.team_marks.iter())
            .chain(rt.guards.iter())
        {
            scene.graph[*node].set_visibility(false);
        }
        let beacons: Vec<_> = scene
            .graph
            .traverse_handle_iter(scene.graph.get_root())
            .filter(|h| matches!(scene.graph[*h].name(), "Objective beacon" | "Beacon crown"))
            .collect();
        for node in beacons {
            scene.graph[node].set_visibility(false);
        }
    }
    let ui = context.user_interfaces.first_mut();
    ui.send(ui.root(), WidgetMessage::Visibility(show_hud));
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
    let tag = if rt.session.world().map_id == MapId::ElfCity {
        format!("elf-city-{}-", views(rt)[rt.review_capture as usize].3)
    } else {
        format!("map-{}-", views(rt)[rt.review_capture as usize].3)
    };
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
