//! GPU regression for the first-person shadow and full-body visibility transitions.
use super::*;
use fyrox::{
    renderer::is_shadow_pass,
    scene::{animation::AnimationPlayer, mesh::Mesh},
};

const STAGES: usize = 23;

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if std::env::args().any(|arg| arg == "--crouch-review") {
        return super::crouch_review::update(rt, context);
    }
    let step = rt.review_capture as usize;
    if step == STAGES {
        println!("SHADOW_REVIEW_OK: baseline, 15 heroes, greeting, jump, third-person, return, death and respawn");
        context.loop_controller.exit();
        return Ok(());
    }
    if step == 0 && rt.review_frames == 0 {
        Shapes::new().solid(
            &mut context.scenes[rt.scene].graph,
            "Shadow review floor",
            V3::new(0.0, 29.9, -12.0),
            V3::new(30.0, 0.2, 30.0),
            [210, 210, 210],
            0,
        );
    }
    let id = if (2..17).contains(&step) {
        (step - 2) as u8
    } else {
        2
    };
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = -0.5;
    rt.pitch = -0.85;
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Playing;
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0;
        }
        let a = &mut w.actors[0];
        // Test removal of the expired body independently of the death-camera sequence.
        a.active = step != 21;
        a.character = id;
        a.pos = V3::new(0.0, if step == 18 { 30.7 } else { 30.0 }, -12.0);
        a.hp = if step == 21 { 0 } else { MAX_HP };
        a.respawn = if step == 21 { 4.0 } else { 0.0 };
        a.killed_by = None;
        a.dancing = step == 19;
        a.wave_ticks = if step == 17 { 80 } else { 0 };
        a.yaw = rt.yaw;
        a.pitch = rt.pitch;
        a.reload = 0.0;
        a.ammo = kit(id).magazine;
        rt.view_pos = a.eye();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    // Fixed poses make the baseline/visible-shadow pair directly comparable.
    for animation in g[rt.visuals[0].player]
        .cast_mut::<AnimationPlayer>()
        .unwrap()
        .animations_mut()
        .iter_mut()
    {
        animation.set_time_position(if step == 18 {
            4.3
        } else if step == 19 {
            10.0
        } else {
            0.0
        });
        animation.set_speed(0.0);
    }
    if step == 0 {
        g[rt.actors[0]].set_visibility(false);
    }
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if std::env::args().any(|arg| arg == "--crouch-review") {
        return super::crouch_review::capture(rt, context);
    }
    let step = rt.review_capture as usize;
    let g = &context.scenes[rt.scene].graph;
    assert_eq!(g[rt.actors[0]].visibility(), step != 0 && step != 21);
    assert_eq!(rt.visuals[0].is_shadow_only(), step != 19);
    let mut shadow_surfaces = 0;
    // Summoned skill effects (for example Naia's whale school) are actor
    // children too, but must retain their color passes in first person.
    for node in [rt.visuals[0].root, rt.visuals[0].held_weapon()]
        .into_iter().flat_map(|root| g.traverse_handle_iter(root)) {
        if let Some(mesh) = g[node].cast::<Mesh>() {
            // Forward-only light ribbons and outlines deliberately have no
            // shadow pass. Check the body/weapon surfaces that cast shadows.
            if !mesh.cast_shadows() {
                continue;
            }
            for surface in mesh.surfaces() {
                let material = surface.material().data_ref();
                let shader = material.shader().data_ref();
                assert_eq!(
                    shader.definition.passes.iter().any(|p| p.name == "GBuffer"),
                    step == 19,
                    "body/weapon color pass at step {step}: {}", g[node].name()
                );
                assert!(shader
                    .definition
                    .passes
                    .iter()
                    .any(|p| is_shadow_pass(&p.name)));
                if step != 0 && step != 21 && mesh.global_visibility() && mesh.cast_shadows() {
                    shadow_surfaces += 1;
                }
            }
        }
    }
    if step != 0 && step != 21 {
        assert!(shadow_surfaces > 0);
    }
    for root in [rt.weapon, rt.wave_hand] {
        for node in g.traverse_handle_iter(root) {
            if g[node].cast::<Mesh>().is_some() {
                assert!(!g[node].cast_shadows());
            }
        }
    }
    let label = match step {
        0 => "baseline",
        1 => "first-person",
        2..=16 => characters::SLUGS[step - 2],
        17 => "greeting",
        18 => "jump",
        19 => "third-person",
        20 => "return",
        21 => "dead",
        _ => "respawn",
    };
    capture_preview(rt.scene, 1, &format!("shadow-{step:02}-{label}-"), context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
