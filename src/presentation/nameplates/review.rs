//! Deterministic GPU checks for nameplate placement, health and visibility.
use super::*;

const STAGES: [&str; 12] = [
    "full",
    "damaged",
    "healed",
    "crouched",
    "giant",
    "dead",
    "occluded",
    "behind",
    "selection",
    "third-person",
    "first-person",
    "inactive",
];

pub(in crate::presentation) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let step = rt.review_capture as usize;
    if step == STAGES.len() {
        println!("NAMEPLATES_REVIEW_OK: names, team colors, HP, healing, crouch, giant, death, occlusion, behind camera, selection, third/first person, empty slots");
        context.loop_controller.exit();
        return Ok(());
    }
    let wall = Block {
        center: V3::new(0.0, 31.5, -7.0),
        size: V3::new(2.0, 5.0, 0.6),
        color: [96, 111, 128],
        phaseable: false,
    };
    let g = &mut context.scenes[rt.scene].graph;
    if step == 0 && rt.review_frames == 0 {
        let shapes = Shapes::new();
        shapes.solid(
            g,
            "Nameplate review floor",
            V3::new(0.0, 29.9, -6.0),
            V3::new(35.0, 0.2, 35.0),
            [185, 193, 205],
            0,
        );
        shapes.solid(
            g,
            "Nameplate review wall",
            wall.center,
            wall.size,
            wall.color,
            0,
        );
    }
    for node in g.linear_iter_mut() {
        if node.name() == "Nameplate review wall" {
            node.set_visibility(step == 6);
        }
    }
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = step == 8;
    rt.grabbed = true;
    rt.yaw = 0.0;
    rt.pitch = 0.04;
    if let Session::Solo(world) = &mut rt.session {
        world.phase = Phase::Playing;
        world.map = world.map_id.arena();
        if step == 6 {
            world.map.push(wall);
        }
        for (i, actor) in world.actors.iter_mut().enumerate() {
            *actor = Actor::default();
            actor.active = matches!(i, 0 | 1 | 6 | 7);
            actor.human = i != 7;
            actor.character = if i == 7 { 2 } else { 0 };
            actor.pos = V3::new(
                match i {
                    1 => -3.0,
                    7 => 3.0,
                    _ => 0.0,
                },
                30.0,
                if i == 0 { -12.0 } else { -4.0 },
            );
            actor.yaw = if i == 0 { rt.yaw } else { std::f32::consts::PI };
        }
        world.actors[0].dancing = step == 9;
        world.actors[6].hp = match step {
            1 => 30,
            2 => 90,
            5 => 0,
            _ => 60,
        };
        world.actors[6].crouching = step == 3;
        world.actors[6].combat.giant = if step == 4 { 5.0 } else { 0.0 };
        world.actors[6].active = step != 11;
        world.actors[7].hp = 24;
        if step == 7 {
            world.actors[6].pos.z = -20.0;
        }
        rt.view_pos = world.actors[0].eye();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)
}

pub(in crate::presentation) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if !context
        .graphics_context
        .as_initialized_ref()
        .renderer
        .scene_data_map
        .contains_key(&rt.scene)
    {
        return Ok(());
    }
    rt.review_frames += 1;
    if rt.review_frames < 24 {
        return Ok(());
    }
    let step = rt.review_capture as usize;
    let ui = context.user_interfaces.first();
    let plates = &rt.hud.nameplates.plates;
    assert_eq!(
        ui[plates[0].root].visibility(),
        step == 9,
        "first/third person self label"
    );
    if step != 9 {
        assert_eq!(
            ui[plates[1].root].visibility(),
            step != 8,
            "ally visibility"
        );
        assert_eq!(
            ui[plates[6].root].visibility(),
            !matches!(step, 5..=8 | 11),
            "enemy visibility at {}",
            STAGES[step]
        );
        assert_eq!(ui[plates[7].root].visibility(), step != 8, "bot visibility");
    }
    for i in [2, 3, 4, 5, 8, 9, 10, 11] {
        assert!(!ui[plates[i].root].visibility(), "empty slot {i}");
    }
    if ui[plates[6].root].visibility() {
        let hp = rt.session.world().actors[6].hp;
        assert!((ui[plates[6].fill].width() - BAR_WIDTH * hp as f32 / MAX_HP as f32).abs() < 0.01);
    }
    if step == 0 {
        assert_eq!(
            ui[plates[1].name].cast::<Text>().unwrap().text(),
            "PLAYER 2"
        );
        assert_eq!(
            ui[plates[7].name].cast::<Text>().unwrap().text(),
            "BOT · 에르핀"
        );
    }
    let size = ui.screen_size();
    let tag = format!(
        "nameplates-{}x{}-{}-",
        size.x as u32, size.y as u32, STAGES[step]
    );
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
