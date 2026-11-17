//! Screen-space name and health bars anchored to the rendered character heads.
pub(super) mod review;
use super::*;
use fyrox::{
    core::algebra::Matrix3,
    gui::{canvas::CanvasBuilder, formatted_text::WrapMode, HorizontalAlignment},
    scene::camera::Camera,
};

const WIDTH: f32 = 180.0;
const HEIGHT: f32 = 39.0;
const BAR_WIDTH: f32 = 156.0;
const BAR_HEIGHT: f32 = 7.0;

#[derive(Debug, Default)]
struct Plate {
    root: Handle<UiNode>,
    name: Handle<UiNode>,
    fill: Handle<UiNode>,
}

#[derive(Debug, Default)]
pub(super) struct Nameplates {
    plates: [Plate; PLAYERS],
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

impl Nameplates {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        Self {
            plates: std::array::from_fn(|_| {
                let name = TextBuilder::new(
                    WidgetBuilder::new()
                        .with_hit_test_visibility(false)
                        .with_foreground(Brush::Solid(Color::WHITE).into()),
                )
                .with_font_size(17.0.into())
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .with_wrap(WrapMode::NoWrap)
                .with_trim_text(true)
                .with_shadow(true)
                .build(ctx)
                .to_base();
                let frame = block(ctx, Color::from_rgba(9, 16, 25, 245));
                let track = block(ctx, Color::from_rgba(74, 87, 102, 235));
                let fill = block(ctx, rgb(MINT));
                let root = CanvasBuilder::new(
                    WidgetBuilder::new()
                        .with_width(WIDTH)
                        .with_height(HEIGHT)
                        .with_visibility(false)
                        .with_hit_test_visibility(false)
                        .with_children([name, frame, track, fill]),
                )
                .build(ctx)
                .to_base();
                for (node, x, y, width, height) in [
                    (name, 5.0, 0.0, WIDTH - 10.0, 26.0),
                    (frame, 10.0, 28.0, BAR_WIDTH + 4.0, BAR_HEIGHT + 4.0),
                    (track, 12.0, 30.0, BAR_WIDTH, BAR_HEIGHT),
                    (fill, 12.0, 30.0, BAR_WIDTH, BAR_HEIGHT),
                ] {
                    place(ctx.inner(), node, x, y, width, height);
                }
                Plate { root, name, fill }
            }),
        }
    }

    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> + '_ {
        self.plates.iter().map(|plate| plate.name)
    }

    pub fn render(&self, rt: &Runtime, g: &mut Graph, ui: &UserInterface) {
        let live = rt.session.world();
        let show = matches!(live.phase, Phase::Setup | Phase::Playing)
            && !rt.home_open
            && !rt.lobby.open
            && !rt.startup.active()
            && !selection::showing(rt)
            && !rt.settings_menu.open
            && !rt.keys.score
            && !rt.art_review;
        if !show {
            for plate in &self.plates {
                visible(ui, plate.root, false);
            }
            return;
        }

        let size = ui.screen_size();
        // render() has just moved the camera and smoothed the actor roots. Refresh
        // both before projection instead of using last frame's camera matrices.
        g.update_hierarchical_data_for_descendants(rt.camera);
        g[rt.camera]
            .cast_mut::<Camera>()
            .unwrap()
            .calculate_matrices(size);
        let world = rt.deathcam.scene().unwrap_or(live);
        let view_slot = rt
            .deathcam
            .view
            .map_or(rt.session.slot(), |view| view.target);
        let hud_scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
        for (i, plate) in self.plates.iter().enumerate() {
            let actor = &world.actors[i];
            let visual = &rt.visuals[i];
            if !actor.alive() || visual.is_shadow_only() || !g[rt.actors[i]].visibility() {
                visible(ui, plate.root, false);
                continue;
            }
            g.update_hierarchical_data_for_descendants(rt.actors[i]);
            let head = visual.head_position(g);
            let anchor = head + Vector3::y() * (0.95 * actor.body_scale());
            let camera = g[rt.camera].cast::<Camera>().unwrap();
            let camera_pos = camera.global_position();
            // Test visibility at the head, not the floating label, so a tall wall
            // cannot reveal a hidden player just because their label clears it.
            let origin = V3::new(camera_pos.x, camera_pos.y, camera_pos.z);
            let target = V3::new(head.x, head.y, head.z);
            let delta = target - origin;
            let distance = delta.length();
            let screen = camera.project(anchor, size).filter(|p| {
                p.x.is_finite()
                    && p.y.is_finite()
                    && p.x >= 0.0
                    && p.x <= size.x
                    && p.y >= 0.0
                    && p.y <= size.y
            });
            let Some(screen) = screen.filter(|_| {
                distance > 0.1
                    && !world.map.iter().any(|wall| {
                        ray_box(origin, delta * (1.0 / distance), wall.min(), wall.max())
                            .is_some_and(|hit| hit < distance - 0.05)
                    })
            }) else {
                visible(ui, plate.root, false);
                continue;
            };
            let scale = hud_scale * (12.0 / distance).clamp(0.75, 1.0);
            visible(ui, plate.root, true);
            place(
                ui,
                plate.root,
                screen.x - WIDTH * scale * 0.5,
                screen.y - HEIGHT * scale,
                WIDTH,
                HEIGHT,
            );
            ui.send(
                plate.root,
                WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
            );
            let name = if actor.human {
                rt.session.player_name(i)
            } else {
                let hero = character(actor.character);
                format!(
                    "BOT · {}",
                    if rt.korean_font {
                        hero.korean
                    } else {
                        hero.name
                    }
                )
            };
            text(ui, plate.name, name);
            let color = if team(i) == team(view_slot) {
                MINT
            } else {
                PINK
            };
            ui.send(
                plate.name,
                WidgetMessage::Foreground(Brush::Solid(rgb(color)).into()),
            );
            ui.send(
                plate.fill,
                WidgetMessage::Background(Brush::Solid(rgb(color)).into()),
            );
            place(
                ui,
                plate.fill,
                12.0,
                30.0,
                BAR_WIDTH * actor.hp.min(MAX_HP) as f32 / MAX_HP as f32,
                BAR_HEIGHT,
            );
        }
    }
}
