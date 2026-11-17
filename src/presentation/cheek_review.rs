//! GPU review uses the same hit testing, dragging, release and cancellation as input.
use super::*;
use trickcal_shooting_balddagu::cheeks::{motions, release_duration, Phase as CheekPhase};

pub fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let focused = std::env::args().any(|arg| arg == "--cheek-focus");
    let heroes: Vec<usize> = if focused {
        vec![6, 10, 13]
    } else {
        (0..15).collect()
    };
    if rt.review_capture as usize >= heroes.len() * 7 {
        println!(
            "CHEEK_REVIEW_OK: {} heroes, both cheeks, wide rounded skin patches, fixed head/eyes/mouth/accessories, held/release/automatic idle return, {} GPU frames", heroes.len(), heroes.len()*7
        );
        context.loop_controller.exit();
        return Ok(());
    }
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = true;
    rt.chat.open = false;
    rt.keys = Keys::default();
    rt.selected = heroes[(rt.review_capture / 7) as usize];
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let p = rt.preview.as_mut().unwrap();
    p.review_idle(&mut context.scenes[rt.scene].graph);
    let step = rt.review_capture % 7;
    if rt.review_frames > 2 && (step != 6 || rt.review_frames == 3) {
        let c = &mut p.cheeks;
        c.cancel();
        if (1..=6).contains(&step) {
            let side = if step == 5 { 1 } else { 0 };
            let origin = c.centers[side];
            assert!(
                c.press(origin),
                "{} cheek hit area",
                characters::SLUGS[rt.selected]
            );
            let direction = (origin.x - c.centers[1 - side].x).signum();
            // Include a saturated diagonal pull on each side as well as a normal drag.
            let drag = if step == 1 {
                Vector2::new(direction * c.radius * 2.5, -c.radius * 0.35)
            } else {
                Vector2::new(direction * c.radius * 10.0, -c.radius * 10.0)
            };
            c.drag(origin + drag);
            assert!(c.held());
            assert!(c.interaction.pull[0].abs() <= 0.12);
            assert!(c.interaction.pull[1].abs() <= 0.07);
            if matches!(step, 3 | 4 | 6) {
                c.interaction.release();
                assert_eq!(c.interaction.phase, CheekPhase::Released);
                c.interaction.age = match step {
                    3 => 0.12,
                    4 => release_duration(rt.selected as u8) * 0.5,
                    // Let actual preview updates expire the reaction, without cancelling it.
                    _ => 0.0,
                };
            } else {
                c.interaction.age =
                    motions(rt.selected as u8).hold.duration * if step == 2 { 0.75 } else { 0.25 };
            }
        }
    }
    Ok(())
}
pub fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if rt.review_frames < 12 {
        return Ok(());
    }
    let p = rt.preview.as_ref().unwrap();
    p.cheeks.review_protected();
    let step = rt.review_capture % 7;
    if step == 6 {
        assert!(
            rt.review_frames < 600,
            "cheek reaction did not return to idle"
        );
        if p.cheeks.interaction.phase != CheekPhase::Idle || p.cheeks.changed_vertices != 0 {
            rt.cheek_review_idle_frames = 0;
            return Ok(());
        }
        // before_rendering reads the previous GPU frame. Give the restored pose
        // two draws to reach the framebuffer before inspecting or saving it.
        rt.cheek_review_idle_frames += 1;
        if rt.cheek_review_idle_frames < 3 {
            return Ok(());
        }
    }
    if matches!(step, 1 | 2 | 5) {
        assert!(
            p.cheeks.changed_vertices > 12,
            "{}: cheek has only {} moving vertices",
            characters::SLUGS[rt.selected],
            p.cheeks.changed_vertices
        );
    }
    if matches!(step, 0 | 6) {
        assert_eq!(p.cheeks.changed_vertices, 0);
        assert_eq!(p.cheeks.interaction.phase, CheekPhase::Idle);
    }
    let tag = format!(
        "cheeks-{:02}-{}-{}-",
        rt.selected,
        characters::SLUGS[rt.selected],
        step
    );
    capture_preview(rt.scene, 0, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    println!(
        "CHEEK_FRAME_OK: {} phase={} vertices={} centers={:?}",
        characters::SLUGS[rt.selected],
        step,
        p.cheeks.changed_vertices,
        p.cheeks.centers
    );
    rt.review_capture += 1;
    rt.review_frames = 0;
    rt.cheek_review_idle_frames = 0;
    Ok(())
}
