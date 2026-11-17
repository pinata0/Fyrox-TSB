//! Real-shot GPU fixtures: surface alignment, debris, fade and cleanup.
use super::*;

const STAGES: [(&str, f32); 12] = [
    ("before", 0.0),
    ("burst", 0.12),
    ("dent", 1.5),
    ("fading", 3.5),
    ("expired", 4.2),
    ("floor", 0.15),
    ("slash-burst", 0.12),
    ("slash-dent", 1.5),
    ("slash-finisher", 1.5),
    ("slash-fading", 3.5),
    ("slash-expired", 4.2),
    ("slash-floor", 0.15),
];

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let Some(&(tag, age)) = STAGES.get(rt.review_capture as usize) else {
        println!("IMPACT_REVIEW_OK: real ray and melee contacts, bullet dents, directional cuts, finisher, fragments, fade, four-second cleanup and floor contacts");
        context.loop_controller.exit();
        return Ok(());
    };
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = -std::f32::consts::FRAC_PI_2;
    rt.pitch = if tag.ends_with("floor") { -0.8 } else { 0.0 };
    let melee = tag.starts_with("slash-");
    if let Session::Solo(w) = &mut rt.session {
        if rt.review_frames == 0 {
            w.phase = Phase::Playing;
            w.round += 1;
            w.tick = 100;
            w.impacts.clear();
            w.projectiles.clear();
            for (i, a) in w.actors.iter_mut().enumerate() {
                *a = Actor {
                    human: true,
                    active: i == 0,
                    ..Default::default()
                };
            }
            let a = &mut w.actors[0];
            a.character = if melee { 0 } else { 14 };
            if tag == "slash-finisher" {
                a.combat.vivi_combo = 2;
            }
            a.ammo = 30;
            a.hp = MAX_HP;
            a.pos = V3::new(-3.4, 0.0, -23.5);
            a.yaw = rt.yaw;
            a.pitch = rt.pitch;
            if tag != "before" {
                let mut input = [Input::default(); PLAYERS];
                input[0] = Input {
                    fire: true,
                    yaw: rt.yaw,
                    pitch: rt.pitch,
                    ..Default::default()
                };
                w.tick(input);
                assert_eq!(
                    w.impacts.len(),
                    1,
                    "review must use a real combat collision"
                );
                assert_eq!(
                    w.impacts[0].kind,
                    if melee {
                        trickcal_shooting_balddagu::impact::ImpactKind::Slash
                    } else {
                        trickcal_shooting_balddagu::impact::ImpactKind::Bullet
                    }
                );
                w.tick += (age / DT).round() as u64;
            }
            rt.view_pos = w.actors[0].eye();
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    g[rt.weapon].set_visibility(false);
    let ui = context.user_interfaces.first_mut();
    ui.send(ui.root(), WidgetMessage::Visibility(false));
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let tag = STAGES[rt.review_capture as usize].0;
    let g = &context.scenes[rt.scene].graph;
    let chunks = g
        .traverse_handle_iter(g.get_root())
        .filter(|&h| g[h].name() == "Temporary impact fragment")
        .count();
    assert_eq!(
        chunks,
        match tag {
            "burst" | "floor" => 7,
            "slash-burst" | "slash-floor" => 5,
            _ => 0,
        }
    );
    capture_preview(rt.scene, 1, &format!("impact-{tag}-"), context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
