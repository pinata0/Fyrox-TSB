//! Deterministic voice HUD captures; never opens a microphone or plays audio.
use super::*;

const STAGES: [&str; 7] = [
    "muted",
    "enabled",
    "speaking",
    "muted-while-speaking",
    "unfocused",
    "full-team",
    "selection",
];

pub(in crate::presentation) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let step = rt.review_capture as usize;
    if step == STAGES.len() {
        println!("VOICE_REVIEW_OK: local first, portraits, names, mute/unmute, speaking, focus, full team, selection");
        context.loop_controller.exit();
        return Ok(());
    }
    if step == 0 && rt.review_frames == 0 {
        let mut server = Server::bind("127.0.0.1:0", true)
            .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        server.world = rt.session.world().clone();
        server.player_names[0] = "시심이".into();
        server.player_names[1] = "전기역자".into();
        server.player_names[2] = "종정이".into();
        server.player_names[3] = "아주긴플레이어이름테스트입니다".into();
        rt.session = Session::Host(Box::new(server));
        let (_, captured) = mpsc::sync_channel(1);
        let (incoming, _) = mpsc::sync_channel(1);
        let (_, status) = mpsc::sync_channel(1);
        rt.voice.audio = Some(VoiceAudio {
            generation: Arc::new(AtomicU64::new(0)),
            stop: Arc::new(AtomicBool::new(false)),
            captured,
            incoming,
            status,
            worker: None,
            input_ready: true,
            input_error: None,
            output_error: None,
        });
    }
    if rt.review_frames == 0 {
        rt.voice.mic.focused = true;
        let muted = matches!(step, 0 | 3 | 5 | 6);
        if rt.voice.mic.muted != muted {
            rt.voice.key(true, false, false);
        }
        rt.voice.mic.focused = step != 4;
    }
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = step == 6;
    rt.grabbed = true;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    if let Session::Host(server) = &mut rt.session {
        let world = &mut server.world;
        world.phase = Phase::Playing;
        for (slot, actor) in world.actors.iter_mut().enumerate() {
            actor.active = slot < if step >= 5 { TEAM_SIZE } else { 3 };
            actor.human = true;
            actor.pos = V3::new(slot as f32 * 3.0, 0.0, -12.0);
        }
        rt.view_pos = world.actors[0].eye();
    }
    // Deliberately retain activity through mute: the red X must win immediately.
    rt.voice.speaking_until = std::array::from_fn(|slot| {
        (slot == 1 || (slot == 0 && matches!(step, 2 | 3)))
            .then(|| Instant::now() + Duration::from_secs(1))
    });
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
    let hud = &rt.hud.voice;
    let local = &hud.rows[0];
    assert!(ui[hud.root].visibility());
    assert_eq!(ui[local.name].cast::<Text>().unwrap().text(), "시심이");
    assert_eq!(ui[local.muted].visibility(), rt.voice.mic.muted);
    assert_eq!(ui[local.waves].visibility(), step == 2);
    assert_eq!(
        ui[local.speaker].foreground(),
        Brush::Solid(rgb(if rt.voice.mic.muted {
            VOICE_RED
        } else if step == 4 {
            VOICE_DIM
        } else {
            VOICE_CYAN
        }))
    );
    for (index, row) in hud.rows.iter().enumerate() {
        assert_eq!(
            ui[row.root].visibility(),
            index < if step >= 5 { TEAM_SIZE } else { 3 }
        );
    }
    let size = ui.screen_size();
    if step != 6 {
        let scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
        assert!(
            (ui[hud.root].actual_local_position() - Vector2::new(24.0, 20.0) * scale).norm() < 0.1
        );
        assert!(!ui[rt.hud.brand].visibility());
    }
    capture_preview(
        rt.scene,
        1,
        &format!(
            "voice-{}x{}-{}-",
            size.x as u32, size.y as u32, STAGES[step]
        ),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
