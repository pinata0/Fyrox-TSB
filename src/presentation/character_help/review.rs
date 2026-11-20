//! Exercise the real overlay and capture every hero at either supported review size.
use super::*;

const STAGES: usize = ROSTER.len() + 3;

pub(in crate::presentation) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let step = rt.review_capture as usize;
    if step >= STAGES {
        println!("CHARACTER_HELP_REVIEW_OK: 15 heroes, live scene, input blocking, close, focus loss, round transition");
        context.loop_controller.exit();
        return Ok(());
    }
    if rt.review_frames == 0 {
        let window = &context.graphics_context.as_initialized_ref().window;
        let scene = rt.scene;
        rt.home_open = false;
        rt.lobby.open = false;
        rt.selecting = false;
        rt.character_help.open = false;
        rt.grabbed = true;
        if let Session::Solo(world) = &mut rt.session {
            world.phase = Phase::Playing;
            world.remaining = 120.0;
            for (i, actor) in world.actors.iter_mut().enumerate() {
                actor.active = i == 0 || i == TEAM_SIZE;
            }
            let actor = &mut world.actors[0];
            actor.character = step.min(14) as u8;
            actor.hp = MAX_HP;
            actor.respawn = 0.0;
            actor.pos = V3::new(0.0, 0.0, -12.0);
            actor.yaw = 0.0;
            rt.yaw = 0.0;
            rt.pitch = 0.0;
        }
        rt.keys.w = true;
        rt.keys.fire = true;
        rt.keys.zoom = true;
        let key = |rt: &mut Runtime, code, state, repeat| {
            on_key(rt, PhysicalKey::Code(code), state, repeat, context)
        };
        assert!(key(rt, KeyCode::F1, ElementState::Pressed, false));
        assert!(!rt.controls_active() && !rt.keys.w && !rt.keys.fire && !rt.keys.zoom);
        assert!(key(rt, KeyCode::F1, ElementState::Pressed, true));
        assert!(key(rt, KeyCode::F1, ElementState::Released, false));
        assert!(
            rt.character_help.open,
            "Holding or releasing F1 must not toggle twice"
        );
        assert_eq!(rt.scene, scene, "Help must keep the loaded scene");
        let time = rt.session.world().remaining;
        rt.session.update(Input::default()).unwrap();
        assert!(
            rt.session.world().remaining < time,
            "The live match must continue"
        );
        if step == 15 {
            assert!(key(rt, KeyCode::F1, ElementState::Pressed, false));
            assert!(rt.controls_active());
            assert!(key(rt, KeyCode::F1, ElementState::Pressed, true));
            assert!(!rt.character_help.open);
            assert!(key(rt, KeyCode::F1, ElementState::Pressed, false));
            assert!(key(rt, KeyCode::Escape, ElementState::Pressed, false));
            assert!(!rt.character_help.open && !rt.settings_menu.open);
            assert!(rt.controls_active() && !rt.keys.fire && !rt.keys.w);
        } else if step == 16 {
            let event = Event::WindowEvent {
                window_id: window.id(),
                event: WindowEvent::Focused(false),
            };
            assert!(!on_event(rt, &event, context));
            assert!(!rt.character_help.open && !rt.grabbed && !rt.controls_active());
        } else if step == 17 {
            if let Session::Solo(world) = &mut rt.session {
                world.phase = Phase::Intermission;
            }
            sync(rt, window);
            assert!(!rt.character_help.open);
        }
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
    let help = &rt.hud.character_help;
    let show = step < ROSTER.len();
    assert_eq!(ui[help.root].visibility(), show);
    assert_eq!(ui[help.shade].visibility(), show);
    let size = ui.screen_size();
    if show {
        assert_eq!(
            ui[help.name].cast::<Text>().unwrap().text(),
            ROSTER[step].korean
        );
        for label in help.labels() {
            let bounds = ui[label].screen_bounds();
            assert!(bounds.position.x >= 0.0 && bounds.position.y >= 0.0);
            assert!(bounds.position.x + bounds.size.x <= size.x + 1.0);
            assert!(bounds.position.y + bounds.size.y <= size.y + 1.0);
            let formatted = ui[label].cast::<Text>().unwrap().formatted_text.borrow();
            for line in formatted.get_lines() {
                assert!(
                    line.width <= ui[label].width() + 1.0,
                    "Text overflows horizontally: {}",
                    formatted.text()
                );
                assert!(
                    line.y_offset + line.height <= ui[label].height() + 1.0,
                    "Text overflows vertically: {}",
                    formatted.text()
                );
            }
        }
    }
    let tag = format!(
        "character-help-{}x{}-{step:02}-",
        size.x as u32, size.y as u32
    );
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
