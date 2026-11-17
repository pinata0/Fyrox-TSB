//! Compact team scores and clock, with capture progress inside A/B diamonds.
use super::*;
use fyrox::{
    core::algebra::Matrix3,
    gui::{
        canvas::CanvasBuilder,
        formatted_text::WrapMode,
        vector_image::{Primitive, VectorImage, VectorImageBuilder},
        HorizontalAlignment,
    },
};
use std::cell::Cell;

const WIDTH: f32 = 488.0;
const HEIGHT: f32 = 132.0;
const DIAMOND: f32 = 46.0;
const WHITE: [u8; 3] = [244, 249, 255];
const MUTED: [u8; 3] = [159, 174, 193];
const GOLD: [u8; 3] = [255, 190, 87];

#[derive(Debug, Default)]
pub(super) struct ObjectiveHud {
    root: Handle<UiNode>,
    scores: [Handle<UiNode>; 2],
    roles: [Handle<UiNode>; 2],
    names: [Handle<UiNode>; 2],
    clock: Handle<UiNode>,
    round: Handle<UiNode>,
    clock_label: Handle<UiNode>,
    fills: [Handle<UiNode>; 2],
    outlines: [Handle<UiNode>; 2],
    letters: [Handle<UiNode>; 2],
    last_progress: [Cell<Option<f32>>; 2],
}

fn polygon(points: &[Vector2<f32>]) -> Vec<Primitive> {
    (1..points.len().saturating_sub(1))
        .map(|i| Primitive::Triangle {
            points: [points[0], points[i], points[i + 1]],
        })
        .collect()
}

fn corners() -> [Vector2<f32>; 4] {
    let r = DIAMOND * 0.5;
    [(r, 0.0), (DIAMOND, r), (r, DIAMOND), (0.0, r)].map(|(x, y)| Vector2::new(x, y))
}

fn diamond_fill(progress: f32) -> Vec<Primitive> {
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return vec![];
    }
    if progress >= 1.0 {
        return polygon(&corners());
    }
    // Invert the triangular area, so 25% fills exactly a quarter of the diamond.
    let cut = if progress <= 0.5 {
        DIAMOND * (1.0 - (progress * 0.5).sqrt())
    } else {
        DIAMOND * ((1.0 - progress) * 0.5).sqrt()
    };
    let mut clipped = Vec::with_capacity(5);
    let points = corners();
    for i in 0..4 {
        let a = points[i];
        let b = points[(i + 1) % 4];
        if a.y >= cut {
            clipped.push(a);
        }
        if (a.y >= cut) != (b.y >= cut) {
            clipped.push(a + (b - a) * ((cut - a.y) / (b.y - a.y)));
        }
    }
    polygon(&clipped)
}

fn outline() -> Vec<Primitive> {
    let points = corners();
    (0..4)
        .map(|i| Primitive::Line {
            begin: points[i],
            end: points[(i + 1) % 4],
            thickness: 2.0,
        })
        .collect()
}

fn block(ctx: &mut BuildContext, color: Color) -> Handle<UiNode> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_background(Brush::Solid(color).into()),
    )
    .with_corner_radius(2.0.into())
    .build(ctx)
    .to_base()
}

fn centered(ctx: &mut BuildContext, size: f32, color: [u8; 3]) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(color)).into()),
    )
    .with_font_size(size.into())
    .with_horizontal_text_alignment(HorizontalAlignment::Center)
    .with_wrap(WrapMode::NoWrap)
    .with_shadow(true)
    .build(ctx)
    .to_base()
}

fn vector(ctx: &mut BuildContext, primitives: Vec<Primitive>, color: Color) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(color).into()),
    )
    .with_primitives(primitives)
    .build(ctx)
    .to_base()
}

fn foreground(ui: &UserInterface, node: Handle<UiNode>, color: [u8; 3]) {
    ui.send(
        node,
        WidgetMessage::Foreground(Brush::Solid(rgb(color)).into()),
    );
}

fn point_progress(world: &World, point: usize) -> f32 {
    if point < world.point {
        1.0
    } else if point == world.point {
        world.progress.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

impl ObjectiveHud {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        let backgrounds =
            std::array::from_fn::<_, 3, _>(|_| block(ctx, Color::from_rgba(20, 30, 44, 215)));
        let rules = [block(ctx, rgb(MINT)), block(ctx, rgb(PINK))];
        let names = std::array::from_fn(|_| centered(ctx, 19.0, WHITE));
        let scores = std::array::from_fn(|_| centered(ctx, 34.0, WHITE));
        let roles = std::array::from_fn(|_| centered(ctx, 12.0, MUTED));
        let clock = centered(ctx, 29.0, WHITE);
        let round = centered(ctx, 11.0, MUTED);
        let clock_label = centered(ctx, 11.0, MUTED);
        let tracks = std::array::from_fn::<_, 2, _>(|_| {
            vector(ctx, polygon(&corners()), Color::from_rgba(26, 38, 53, 230))
        });
        let fills = std::array::from_fn(|_| vector(ctx, vec![], rgb(MINT)));
        let outlines = std::array::from_fn(|_| vector(ctx, outline(), rgb(MUTED)));
        let letters = std::array::from_fn(|_| centered(ctx, 20.0, WHITE));
        let connector = block(ctx, Color::from_rgba(159, 174, 193, 140));
        let children: Vec<_> = backgrounds
            .into_iter()
            .chain(rules)
            .chain(names)
            .chain(scores)
            .chain(roles)
            .chain([clock, round, clock_label, connector])
            .chain(tracks)
            .chain(fills)
            .chain(outlines)
            .chain(letters)
            .collect();
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(WIDTH)
                .with_height(HEIGHT)
                .with_hit_test_visibility(false)
                .with_children(children),
        )
        .build(ctx)
        .to_base();
        let ui = ctx.inner();
        for (node, x, y, width, height) in [
            (backgrounds[0], 0.0, 0.0, 180.0, 70.0),
            (backgrounds[1], 184.0, 0.0, 120.0, 70.0),
            (backgrounds[2], 308.0, 0.0, 180.0, 70.0),
            (rules[0], 0.0, 0.0, 180.0, 3.0),
            (rules[1], 308.0, 0.0, 180.0, 3.0),
            (names[0], 10.0, 12.0, 100.0, 26.0),
            (names[1], 378.0, 12.0, 100.0, 26.0),
            (roles[0], 10.0, 41.0, 100.0, 20.0),
            (roles[1], 378.0, 41.0, 100.0, 20.0),
            (scores[0], 112.0, 15.0, 62.0, 49.0),
            (scores[1], 314.0, 15.0, 62.0, 49.0),
            (round, 184.0, 4.0, 120.0, 16.0),
            (clock, 184.0, 20.0, 120.0, 37.0),
            (clock_label, 184.0, 53.0, 120.0, 17.0),
            (connector, 223.0, 105.0, 42.0, 1.0),
        ] {
            place(ui, node, x, y, width, height);
        }
        text(ui, names[0], "BLOOM");
        text(ui, names[1], "COCOA");
        for i in 0..2 {
            let x = 177.0 + i as f32 * 88.0;
            for node in [tracks[i], fills[i], outlines[i]] {
                place(ui, node, x, 82.0, DIAMOND, DIAMOND);
            }
            place(ui, letters[i], x, 92.0, DIAMOND, 30.0);
            text(ui, letters[i], if i == 0 { "A" } else { "B" });
        }
        Self {
            root,
            scores,
            roles,
            names,
            clock,
            round,
            clock_label,
            fills,
            outlines,
            letters,
            last_progress: std::array::from_fn(|_| Cell::new(None)),
        }
    }

    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> {
        self.names
            .into_iter()
            .chain(self.scores)
            .chain(self.roles)
            .chain([self.clock, self.round, self.clock_label])
            .chain(self.letters)
    }

    pub fn render(&self, rt: &Runtime, ui: &mut UserInterface, show: bool) {
        visible(ui, self.root, show && !rt.home_open && !rt.lobby.open);
        if !show || rt.home_open || rt.lobby.open {
            return;
        }
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
        place(
            ui,
            self.root,
            (size.x - WIDTH * scale) * 0.5,
            18.0 * scale,
            WIDTH,
            HEIGHT,
        );
        ui.send(
            self.root,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        // Always use live match state, including while a recorded death replay is shown.
        let world = rt.session.world();
        let practice = world.team_count(0) == 0 || world.team_count(1) == 0;
        let seconds = world.remaining.max(0.0).ceil() as u32;
        text(
            ui,
            self.clock,
            if practice {
                "∞".into()
            } else {
                format!("{}:{:02}", seconds / 60, seconds % 60)
            },
        );
        text(
            ui,
            self.round,
            format!(
                "{} {}{}",
                if rt.korean_font { "라운드" } else { "ROUND" },
                world.round,
                if world.extra { "+" } else { "" }
            ),
        );
        text(
            ui,
            self.clock_label,
            match (practice, world.overtime, rt.korean_font) {
                (true, _, true) => "자유 연습",
                (true, _, false) => "PRACTICE",
                (_, true, true) => "추가시간",
                (_, true, false) => "OVERTIME",
                (_, _, true) => "남은 시간",
                _ => "TIME LEFT",
            },
        );
        foreground(
            ui,
            self.clock,
            if world.overtime && !practice {
                GOLD
            } else {
                WHITE
            },
        );
        foreground(
            ui,
            self.clock_label,
            if world.overtime && !practice {
                GOLD
            } else {
                MUTED
            },
        );
        for i in 0..2 {
            text(ui, self.scores[i], world.scores[i].to_string());
            text(
                ui,
                self.roles[i],
                match (i == world.attacker, rt.korean_font) {
                    (true, true) => "공격",
                    (false, true) => "수비",
                    (true, false) => "ATTACK",
                    (false, false) => "DEFEND",
                },
            );
            let progress = point_progress(world, i);
            if self.last_progress[i].get() != Some(progress) {
                let fill = ui[self.fills[i]]
                    .cast_mut::<VectorImage>()
                    .expect("Capture diamond");
                fill.primitives
                    .set_value_and_mark_modified(diamond_fill(progress));
                fill.invalidate_visual();
                self.last_progress[i].set(Some(progress));
            }
            let color = if world.attacker == 0 { MINT } else { PINK };
            foreground(ui, self.fills[i], color);
            foreground(
                ui,
                self.outlines[i],
                if i == world.point && world.contested {
                    GOLD
                } else if i == world.point {
                    WHITE
                } else if i < world.point {
                    color
                } else {
                    MUTED
                },
            );
            foreground(
                ui,
                self.letters[i],
                if i > world.point { MUTED } else { WHITE },
            );
        }
    }
}

const REVIEW: [&str; 9] = [
    "a-empty",
    "a-quarter",
    "a-half",
    "a-three-quarters",
    "b-half",
    "cocoa-b",
    "overtime",
    "practice",
    "selection-hidden",
];

pub(super) fn review_update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= REVIEW.len() {
        println!("OBJECTIVE_HUD_REVIEW_OK: A/B fill, both attackers, scores, clock, overtime, practice and selection cleanup");
        context.loop_controller.exit();
        return Ok(());
    }
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = rt.review_capture == 8;
    rt.grabbed = true;
    rt.settings_menu.open = false;
    rt.keys = Keys::default();
    if rt.review_frames == 0 {
        if let Session::Solo(world) = &mut rt.session {
            **world = World::default();
            world.actors[0].human = true;
            world.phase = Phase::Playing;
            world.round = 2;
            world.scores = [0, 2];
            world.remaining = 154.0;
            world.actors[0].pos = V3::new(0.0, 0.0, -12.0);
            world.actors[0].yaw = 0.0;
            world.actors[0].pitch = 0.0;
            match rt.review_capture {
                1..=3 => world.progress = rt.review_capture as f32 * 0.25,
                4..=6 => {
                    world.point = 1;
                    world.scores = [1, 2];
                    world.progress = 0.486;
                    if rt.review_capture == 5 {
                        world.attacker = 1;
                        world.scores = [2, 1];
                        world.progress = 0.354;
                        world.remaining = 68.0;
                    } else if rt.review_capture == 6 {
                        world.remaining = 0.0;
                        world.overtime = true;
                        world.progress = 2.0 / 3.0;
                        world.contested = true;
                    }
                }
                7 => {
                    for actor in &mut world.actors[TEAM_SIZE..] {
                        actor.active = false;
                    }
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
    let ui = context.user_interfaces.first();
    assert_eq!(
        ui[rt.hud.objective.root].visibility(),
        rt.review_capture != 8
    );
    if rt.review_capture != 8 {
        for i in 0..2 {
            assert_eq!(
                rt.hud.objective.last_progress[i].get(),
                Some(point_progress(rt.session.world(), i))
            );
        }
    }
    let size = ui.screen_size();
    let tag = format!(
        "objective-{}x{}-{}-",
        size.x as u32, size.y as u32, REVIEW[rt.review_capture as usize]
    );
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diamond_area_tracks_capture_percentage_and_stays_inside_outline() {
        for fraction in [0.0, 0.01, 0.25, 0.3333, 0.5, 0.6667, 0.75, 0.99, 1.0] {
            let mut area = 0.0;
            for primitive in diamond_fill(fraction) {
                let Primitive::Triangle { points: [a, b, c] } = primitive else {
                    panic!("fill must be triangles")
                };
                for p in [a, b, c] {
                    assert!(
                        (p.x - DIAMOND * 0.5).abs() + (p.y - DIAMOND * 0.5).abs()
                            <= DIAMOND * 0.5 + 0.001
                    );
                }
                area += ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() * 0.5;
            }
            assert!((area / (DIAMOND * DIAMOND * 0.5) - fraction).abs() < 0.0001);
        }
    }

    #[test]
    fn captured_point_stays_full_and_next_round_resets_both_diamonds() {
        let mut world = World {
            point: 0,
            progress: 0.75,
            ..World::default()
        };
        assert_eq!(
            [point_progress(&world, 0), point_progress(&world, 1)],
            [0.75, 0.0]
        );
        world.point = 1;
        world.progress = 0.25;
        assert_eq!(
            [point_progress(&world, 0), point_progress(&world, 1)],
            [1.0, 0.25]
        );
        world.point = 0;
        world.progress = 0.0;
        assert_eq!(
            [point_progress(&world, 0), point_progress(&world, 1)],
            [0.0, 0.0]
        );
    }
}
