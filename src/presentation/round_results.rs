//! Trickcal-inspired result art, timed from the server's unscaled phase clock.
use super::*;
use fyrox::{
    asset::manager::ResourceManager,
    core::algebra::Matrix3,
    gui::{canvas::CanvasBuilder, image::ImageBuilder, HorizontalAlignment},
};
use trickcal_shooting_balddagu::round_transition::{REVEAL_SECONDS, SLOW_MOTION_SECONDS};

#[derive(Debug, Default)]
pub(super) struct ResultUi {
    root: Handle<UiNode>,
    dim: Handle<UiNode>,
    card: Handle<UiNode>,
    victory: Handle<UiNode>,
    defeat: Handle<UiNode>,
    draw: Handle<UiNode>,
    heading: Handle<UiNode>,
    score: Handle<UiNode>,
    next: Handle<UiNode>,
    textures: Vec<TextureResource>,
}

impl ResultUi {
    pub fn new(ui: &mut UserInterface, manager: &ResourceManager) -> Self {
        let textures: Vec<TextureResource> = ["victory", "defeat"]
            .into_iter()
            .map(|name| manager.request(format!("assets/ui/round-results/{name}.png")))
            .collect();
        let ctx = &mut ui.build_ctx();
        let dim = panel(ctx, Color::from_rgba(12, 17, 28, 125));
        let [victory, defeat] = std::array::from_fn(|i| {
            ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
                .with_texture(textures[i].clone())
                .build(ctx)
                .to_base()
        });
        let draw = label(ctx, "DRAW", 96.0, [226, 224, 238]);
        let heading = label(ctx, "", 34.0, CREAM);
        let score = label(ctx, "", 21.0, CREAM);
        let next = label(ctx, "", 18.0, [208, 217, 226]);
        let card = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(760.0)
                .with_height(560.0)
                .with_hit_test_visibility(false)
                .with_children(vec![victory, defeat, draw, heading, score, next]),
        )
        .build(ctx)
        .to_base();
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_children(vec![dim, card]),
        )
        .build(ctx)
        .to_base();
        for node in [victory, defeat] {
            place(ui, node, 50.0, 0.0, 660.0, 440.0);
        }
        for (node, y, height) in [
            (draw, 190.0, 140.0),
            (heading, 425.0, 52.0),
            (score, 484.0, 32.0),
            (next, 528.0, 30.0),
        ] {
            place(ui, node, 0.0, y, 760.0, height);
            ui.send(
                node,
                TextMessage::HorizontalAlignment(HorizontalAlignment::Center),
            );
        }
        Self {
            root,
            dim,
            card,
            victory,
            defeat,
            draw,
            heading,
            score,
            next,
            textures,
        }
    }

    pub fn labels(&self) -> [Handle<UiNode>; 4] {
        [self.draw, self.heading, self.score, self.next]
    }

    pub fn render(&self, rt: &Runtime, ui: &UserInterface) {
        let world = rt.session.world();
        let elapsed = world.result_elapsed();
        let show = world.in_round_transition()
            && !rt.home_open
            && !rt.lobby.open
            && !rt.settings_menu.open
            && !rt.keys.score;
        visible(ui, self.root, show);
        if !show {
            return;
        }
        let size = ui.screen_size();
        place(ui, self.root, 0.0, 0.0, size.x, size.y);
        place(ui, self.dim, 0.0, 0.0, size.x, size.y);
        ui.send(
            self.dim,
            WidgetMessage::Opacity(Some((elapsed / SLOW_MOTION_SECONDS).clamp(0.0, 1.0))),
        );
        let revealed = elapsed >= SLOW_MOTION_SECONDS;
        visible(ui, self.card, revealed);
        if !revealed {
            return;
        }
        let t = ((elapsed - SLOW_MOTION_SECONDS) / REVEAL_SECONDS).clamp(0.0, 1.0);
        let pop = 1.0 + 2.70158 * (t - 1.0).powi(3) + 1.70158 * (t - 1.0).powi(2);
        let fit = (size.x / 1000.0).min(size.y / 740.0).min(1.15);
        let scale = fit * (0.78 + 0.22 * pop);
        place(
            ui,
            self.card,
            (size.x - 760.0 * scale) * 0.5,
            (size.y - 560.0 * scale) * 0.5 - 20.0 * fit,
            760.0,
            560.0,
        );
        ui.send(
            self.card,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        ui.send(
            self.card,
            WidgetMessage::Opacity(Some(
                (t * 3.0).min(1.0) * (world.phase_time / 0.35).clamp(0.0, 1.0),
            )),
        );
        let result = world.result.unwrap_or(2);
        let won = result == team(rt.session.slot());
        visible(ui, self.victory, result < 2 && won);
        visible(ui, self.defeat, result < 2 && !won);
        visible(ui, self.draw, result == 2);
        let final_result = world.phase == Phase::Finished;
        let title = if rt.korean_font {
            match (final_result, result == 2, won) {
                (_, true, _) => "무승부",
                (true, false, true) => "최종 승리",
                (true, false, false) => "최종 패배",
                (false, false, true) => "라운드 승리",
                (false, false, false) => "라운드 패배",
            }
        } else if result == 2 {
            "DRAW"
        } else if final_result {
            if won {
                "MATCH VICTORY"
            } else {
                "MATCH DEFEAT"
            }
        } else if won {
            "ROUND VICTORY"
        } else {
            "ROUND DEFEAT"
        };
        text(ui, self.heading, title);
        ui.send(
            self.heading,
            WidgetMessage::Foreground(
                Brush::Solid(rgb(if result == 2 {
                    [226, 224, 238]
                } else if won {
                    [204, 255, 146]
                } else {
                    [222, 205, 237]
                }))
                .into(),
            ),
        );
        let round = if final_result {
            world.round
        } else {
            world.round.saturating_sub(1)
        };
        text(
            ui,
            self.score,
            format!(
                "ROUND {round:02}    ·    BLOOM  {} : {}  COCOA",
                world.scores[0], world.scores[1]
            ),
        );
        text(
            ui,
            self.next,
            if rt.korean_font {
                if final_result {
                    format!(
                        "{}초 후 게임 로비로 돌아갑니다",
                        world.phase_time.ceil() as u32
                    )
                } else {
                    format!(
                        "{}초 후 공수 교대 · 다음 라운드 준비",
                        world.phase_time.ceil() as u32
                    )
                }
            } else if final_result {
                format!("Returning to lobby in {}s", world.phase_time.ceil() as u32)
            } else {
                format!("Switching sides in {}s", world.phase_time.ceil() as u32)
            },
        );
    }

    fn ready(&self) -> bool {
        self.textures
            .iter()
            .all(|texture| texture.state().data_ref().is_some())
    }
}

// Deterministic GPU fixtures exercise both perspectives, result timings and cleanup.
const REVIEW: [&str; 7] = [
    "slow-motion",
    "victory",
    "defeat",
    "final-victory",
    "final-defeat",
    "draw",
    "next-round",
];

pub(super) fn review_update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= REVIEW.len() {
        println!(
            "ROUND_RESULTS_REVIEW_OK: delayed reveal, both teams, final results, draw, cleanup"
        );
        context.loop_controller.exit();
        return Ok(());
    }
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.settings_menu.open = false;
    rt.keys = Keys::default();
    if let Session::Solo(world) = &mut rt.session {
        if rt.review_frames == 0 {
            world.start();
            world.phase = Phase::Playing;
            world.actors[0].pos = V3::new(0.0, 0.0, -12.0);
            world.actors[0].yaw = 0.0;
            world.actors[0].pitch = 0.0;
            world.scores = [2, 1];
            world.finish_round(true);
            world.phase_time = 4.0;
            match rt.review_capture {
                0 => world.phase_time = 5.6,
                2 => world.result = Some(1),
                3..=5 => {
                    world.phase = Phase::Finished;
                    world.result = Some((rt.review_capture - 3) as usize);
                    world.phase_time = 3.0;
                }
                6 => {
                    world.phase_time = DT;
                    world.tick([Input::default(); PLAYERS]);
                    assert_eq!(world.phase, Phase::Setup);
                    assert_eq!(world.result, None);
                }
                _ => {}
            }
            rt.yaw = world.actors[0].yaw;
            rt.pitch = world.actors[0].pitch;
            rt.view_pos = world.actors[0].eye();
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)
}

pub(super) fn review_capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if !rt.hud.round_results.ready()
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
    let result = &rt.hud.round_results;
    assert_eq!(ui[result.root].visibility(), rt.review_capture != 6);
    if rt.review_capture == 0 {
        assert!(!ui[result.card].visibility());
    }
    if (1..=5).contains(&rt.review_capture) {
        assert!(ui[result.card].visibility());
        assert_eq!(
            ui[result.victory].visibility(),
            matches!(rt.review_capture, 1 | 3)
        );
        assert_eq!(
            ui[result.defeat].visibility(),
            matches!(rt.review_capture, 2 | 4)
        );
        assert_eq!(ui[result.draw].visibility(), rt.review_capture == 5);
    }
    let size = ui.screen_size();
    let tag = format!(
        "round-result-{}x{}-{}-",
        size.x as u32, size.y as u32, REVIEW[rt.review_capture as usize]
    );
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
