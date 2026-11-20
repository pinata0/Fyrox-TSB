//! GPU checks of the real overlay at normal and small window sizes.
use super::*;
use trickcal_shooting_balddagu::match_stats::MatchStats;

const CASES: [&str; 8] = [
    "full",
    "sparse-dead",
    "results",
    "released",
    "selection",
    "lobby",
    "healer",
    "settings",
];

pub(in crate::presentation) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let step = rt.review_capture as usize;
    if step >= CASES.len() {
        println!("SCOREBOARD_REVIEW_OK: full roster, empty slots, death, results, release, selection, lobby, healer, settings, layout bounds");
        context.loop_controller.exit();
        return Ok(());
    }
    if rt.review_frames == 0 {
        rt.home_open = false;
        rt.lobby.open = false;
        rt.selecting = false;
        rt.settings_menu.open = false;
        rt.character_help.open = false;
        rt.grabbed = true;
        rt.keys = Keys::default();
        if let Session::Solo(w) = &mut rt.session {
            w.start();
            w.phase = Phase::Playing;
            w.remaining = 168.0;
            w.scores = [2, 1];
            for (slot, a) in w.actors.iter_mut().enumerate() {
                a.active = true;
                a.human = slot == 0;
                a.character = (slot % 15) as u8;
                a.hp = MAX_HP;
                a.shield = 0.0;
                a.combat.ultimate = [9.0, 1.0, 32.0, 94.0, 89.0, 100.0][slot % TEAM_SIZE];
                a.kills = (9 - slot.min(8)) as u16;
                a.deaths = 2;
                a.stats = MatchStats {
                    assists: (slot % 4) as u16,
                    solo_kills: 1,
                    damage: 1928.0 + slot as f32 * 128.0,
                    healing: if character(a.character).healer {
                        1531.0
                    } else {
                        0.0
                    },
                    mitigated: if slot % 3 == 0 { 1882.0 } else { 0.0 },
                };
            }
            w.actors[0].pos = V3::new(0.0, 0.0, -12.0);
            w.actors[0].yaw = 0.0;
            w.actors[0].pitch = 0.0;
            if step == 1 {
                for slot in [2, 4, 7, 10] {
                    w.clear_slot(slot);
                }
                w.actors[0].hp = 0;
                w.actors[0].respawn = 4.0;
                w.actors[0].killed_by = Some(6);
                w.actors[1].stats.damage = 1234567.0;
                w.actors[1].stats.healing = 999999.0;
            }
            if step == 2 {
                w.phase = Phase::Intermission;
                w.result = Some(0);
                w.phase_time = 3.0;
            }
            if step == 6 {
                w.actors[0].character = 13;
                w.actors[0].stats.healing = 12345.0;
            }
            rt.yaw = 0.0;
            rt.pitch = 0.0;
            rt.view_pos = w.actors[0].eye();
        }
        on_tab(rt, true);
        on_tab(rt, true); // Key repeat must continue to hold, never toggle.
        assert!(showing(rt));
        match step {
            3 => on_tab(rt, false),
            4 => rt.selecting = true,
            5 => rt.lobby.open = true,
            7 => rt.settings_menu.open = true,
            _ => {}
        }
        if matches!(step, 3..=5 | 7) {
            assert!(!showing(rt));
        }
        if step == 0 {
            assert!(
                rt.controls_active(),
                "Tab does not pause movement or the match"
            );
        }
    }
    // Window activation can emit a focus-loss event during initial asset loading.
    // Reassert the held key for this deterministic capture fixture.
    on_tab(rt, matches!(step, 0..=2 | 6));
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)
}

pub(in crate::presentation) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if !rt
        .hud
        .scoreboard
        .textures
        .iter()
        .all(|t| t.state().data_ref().is_some())
        || !context
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
    let ui = context.user_interfaces.first();
    let board = &rt.hud.scoreboard;
    let show = matches!(rt.review_capture, 0..=2 | 6);
    assert_eq!(ui[board.root].visibility(), show);
    let size = ui.screen_size();
    if show {
        for label in board.labels() {
            let bounds = ui[label].screen_bounds();
            assert!(bounds.position.x >= -1.0 && bounds.position.y >= -1.0);
            assert!(bounds.position.x + bounds.size.x <= size.x + 1.0);
            assert!(bounds.position.y + bounds.size.y <= size.y + 1.0);
            let formatted = ui[label].cast::<Text>().unwrap().formatted_text.borrow();
            for line in formatted.get_lines() {
                assert!(
                    line.y_offset + line.height <= ui[label].height() + 2.0,
                    "Vertical overflow: {}",
                    formatted.text()
                );
            }
        }
        assert_eq!(ui[board.personal[0]].cast::<Text>().unwrap().text(), "9");
        assert_eq!(
            ui[board.rows[0].values[0]].cast::<Text>().unwrap().text(),
            "9"
        );
    }
    let tag = format!(
        "scoreboard-{}x{}-{}-",
        size.x as u32, size.y as u32, CASES[rt.review_capture as usize]
    );
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
