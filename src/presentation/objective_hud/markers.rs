//! World-anchored A/B diamonds. The active objective is drawn through geometry.
use super::*;
use fyrox::scene::camera::Camera;

const ANCHOR_HEIGHT: f32 = 3.2;
const MAX_SIZE: f32 = 64.0;
const MIN_SIZE: f32 = 28.0;
const FAR_DISTANCE: f32 = 60.0;

#[derive(Debug, Default)]
struct Marker {
    root: Handle<UiNode>,
    outline: Handle<UiNode>,
    letter: Handle<UiNode>,
}

#[derive(Debug, Default)]
pub(in crate::presentation) struct ObjectiveMarkers {
    markers: [Marker; 2],
}

fn marker_size(distance: f32) -> f32 {
    // Ground distance matches the capture circle: every position on the point
    // gets the same maximum size, regardless of character height or crouching.
    let distance = distance.clamp(RADIUS, FAR_DISTANCE);
    let proximity =
        (distance.recip() - FAR_DISTANCE.recip()) / (RADIUS.recip() - FAR_DISTANCE.recip());
    MIN_SIZE + (MAX_SIZE - MIN_SIZE) * proximity
}

fn marker_screen(
    camera: &Camera,
    size: Vector2<f32>,
    world: &World,
    point: usize,
) -> Option<Vector2<f32>> {
    let anchor = vec(world.points()[point]) + Vector3::y() * ANCHOR_HEIGHT;
    let screen = camera.project(anchor, size).filter(|p| {
        p.x.is_finite()
            && p.y.is_finite()
            && p.x >= 0.0
            && p.x <= size.x
            && p.y >= 0.0
            && p.y <= size.y
    })?;
    // The active marker is a HUD overlay with no depth/line-of-sight test.
    // Locked and captured points remain visible only when their anchor is clear.
    if point != world.point {
        let position = camera.global_position();
        let origin = V3::new(position.x, position.y, position.z);
        let delta = V3::new(anchor.x, anchor.y, anchor.z) - origin;
        let distance = delta.length();
        if world.map.iter().any(|wall| {
            ray_box(origin, delta.normalized(), wall.min(), wall.max())
                .is_some_and(|hit| hit < distance - 0.05)
        }) {
            return None;
        }
    }
    Some(screen)
}

impl ObjectiveMarkers {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        Self {
            markers: std::array::from_fn(|i| {
                let background =
                    vector(ctx, polygon(&corners()), Color::from_rgba(20, 30, 44, 220));
                let outline = vector(ctx, outline(), rgb(WHITE));
                let letter = centered(ctx, 23.0, WHITE);
                let root = CanvasBuilder::new(
                    WidgetBuilder::new()
                        .with_width(DIAMOND)
                        .with_height(DIAMOND)
                        .with_visibility(false)
                        .with_hit_test_visibility(false)
                        .with_children([background, outline, letter]),
                )
                .build(ctx)
                .to_base();
                for node in [background, outline] {
                    place(ctx.inner(), node, 0.0, 0.0, DIAMOND, DIAMOND);
                }
                place(ctx.inner(), letter, 0.0, 8.0, DIAMOND, 34.0);
                text(ctx.inner(), letter, if i == 0 { "A" } else { "B" });
                Marker {
                    root,
                    outline,
                    letter,
                }
            }),
        }
    }

    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> + '_ {
        self.markers.iter().map(|marker| marker.letter)
    }

    pub fn render(&self, rt: &Runtime, g: &mut Graph, ui: &UserInterface) {
        let live = rt.session.world();
        let show = matches!(live.phase, Phase::Setup | Phase::Playing)
            && !rt.home_open
            && !rt.lobby.open
            && !rt.startup.active()
            && !selection::showing(rt)
            && !rt.settings_menu.open
            && !rt.character_help.open
            && !rt.keys.score
            && !rt.art_review;
        if !show {
            for marker in &self.markers {
                visible(ui, marker.root, false);
            }
            return;
        }
        let size = ui.screen_size();
        // Use this frame's camera, including zoom, third person and death replay.
        g.update_hierarchical_data_for_descendants(rt.camera);
        g[rt.camera]
            .cast_mut::<Camera>()
            .unwrap()
            .calculate_matrices(size);
        let camera = g[rt.camera].cast::<Camera>().unwrap();
        let position = camera.global_position();
        let origin = V3::new(position.x, position.y, position.z);
        let world = rt.deathcam.scene().unwrap_or(live);
        let hud_scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
        for (i, marker) in self.markers.iter().enumerate() {
            let Some(screen) = marker_screen(camera, size, world, i) else {
                visible(ui, marker.root, false);
                continue;
            };
            let distance = (origin - world.points()[i]).flat().length();
            let scale = hud_scale * marker_size(distance) / DIAMOND;
            visible(ui, marker.root, true);
            place(
                ui,
                marker.root,
                screen.x - DIAMOND * scale * 0.5,
                screen.y - DIAMOND * scale * 0.5,
                DIAMOND,
                DIAMOND,
            );
            ui.send(
                marker.root,
                WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
            );
            let active = i == world.point;
            let color = if !active {
                MUTED
            } else if world.contested {
                GOLD
            } else if world.attacker == 0 {
                MINT
            } else {
                PINK
            };
            foreground(ui, marker.outline, color);
            foreground(ui, marker.letter, if active { WHITE } else { MUTED });
        }
    }
}

// Extend --objective-hud-review with deterministic distance and occlusion shots.
pub(super) fn prepare_review(rt: &mut Runtime, context: &mut PluginContext) {
    let step = rt.review_capture;
    if step < 9 {
        return;
    }
    if let Session::Solo(world) = &mut rt.session {
        world.point = usize::from(step == 15);
        world.attacker = usize::from(step == 15);
        let point = world.points()[world.point];
        let distance = match step {
            9 => RADIUS,
            10 => 1.0,
            11 => 20.0,
            12 => FAR_DISTANCE,
            13 => FAR_DISTANCE + 30.0,
            _ => 12.0,
        };
        let actor = &mut world.actors[0];
        actor.pos = point + V3::new(0.0, 0.0, -distance);
        actor.yaw = if step == 16 {
            std::f32::consts::PI
        } else {
            0.0
        };
        // Keep the letter above the crosshair so screenshots can verify it.
        actor.pitch = (ANCHOR_HEIGHT - actor.eye().y + point.y).atan2(distance) - 0.12;
        rt.yaw = actor.yaw;
        rt.pitch = actor.pitch;
        rt.view_pos = actor.eye();
        for actor in &mut world.actors[1..] {
            actor.active = false;
        }
        let g = &mut context.scenes[rt.scene].graph;
        for node in g.linear_iter_mut() {
            if node.name() == "Objective marker review wall" {
                node.set_visibility(matches!(step, 14 | 15));
                node.local_transform_mut()
                    .set_position(vec(point + V3::new(0.0, 3.0, -6.0)));
            }
        }
        if rt.review_frames == 0 && matches!(step, 14 | 15) {
            let wall = Block {
                center: point + V3::new(0.0, 3.0, -6.0),
                size: V3::new(12.0, 8.0, 1.0),
                color: [96, 111, 128],
                phaseable: false,
            };
            if step == 14 {
                Shapes::new().solid(
                    g,
                    "Objective marker review wall",
                    wall.center,
                    wall.size,
                    wall.color,
                    0,
                );
            }
            world.map.push(wall);
        }
    }
}

pub(super) fn assert_review(rt: &Runtime, ui: &UserInterface) {
    let markers = &rt.hud.objective_markers.markers;
    if rt.review_capture == 8 {
        assert!(markers.iter().all(|marker| !ui[marker.root].visibility()));
    } else if rt.review_capture >= 9 {
        let world = rt.session.world();
        let marker = &markers[world.point];
        assert_eq!(ui[marker.root].visibility(), rt.review_capture != 16);
        assert_eq!(
            ui[marker.letter].cast::<Text>().unwrap().text(),
            if world.point == 0 { "A" } else { "B" }
        );
        let expected = match rt.review_capture {
            9 | 10 => Some(MAX_SIZE),
            12 | 13 => Some(MIN_SIZE),
            _ => None,
        };
        if let Some(expected) = expected {
            let size = ui.screen_size();
            let hud_scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
            let actual = ui[marker.root].render_transform()[(0, 0)] * DIAMOND / hud_scale;
            assert!(
                (actual - expected).abs() < 0.01,
                "marker size at {}",
                rt.review_capture
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_is_capped_on_capture_circle_and_beyond_far_distance() {
        for distance in [0.0, 0.1, RADIUS * 0.5, RADIUS] {
            assert_eq!(marker_size(distance), MAX_SIZE);
        }
        for distance in [FAR_DISTANCE, FAR_DISTANCE * 2.0, 10_000.0] {
            assert_eq!(marker_size(distance), MIN_SIZE);
        }
        let mut previous = MAX_SIZE;
        for step in 1..=100 {
            let distance = RADIUS + (FAR_DISTANCE - RADIUS) * step as f32 / 100.0;
            let size = marker_size(distance);
            assert!((MIN_SIZE..=MAX_SIZE).contains(&size));
            assert!(
                size < previous,
                "size must shrink continuously before the far limit"
            );
            previous = size;
        }
    }

    #[test]
    fn only_active_point_ignores_walls_and_projection_rejects_behind_camera() {
        let size = Vector2::new(1280.0, 800.0);
        for map_id in MapId::ALL {
            let mut world = World {
                map_id,
                ..World::default()
            };
            let mut graph = Graph::new();
            let handle: Handle<Node> = CameraBuilder::new(BaseBuilder::new())
                .build(&mut graph)
                .to_base();
            for point in 0..2 {
                let target = world.points()[point];
                let origin = target + V3::new(0.0, EYE_HEIGHT, -12.0);
                graph[handle]
                    .local_transform_mut()
                    .set_position(vec(origin));
                graph.update_hierarchical_data_for_descendants(handle);
                let camera = graph[handle].cast_mut::<Camera>().unwrap();
                camera.calculate_matrices(size);
                // A synthetic wall fully covers the marker and the capture point.
                world.map = vec![Block {
                    center: target + V3::new(0.0, 3.0, -6.0),
                    size: V3::new(8.0, 10.0, 1.0),
                    color: [80; 3],
                    phaseable: false,
                }];
                world.point = point;
                let projected = marker_screen(camera, size, &world, point).unwrap();
                assert!((projected.x - size.x * 0.5).abs() < 0.01);
                world.point = 1 - point;
                assert!(marker_screen(camera, size, &world, point).is_none());
                world.map.clear();
                assert!(marker_screen(camera, size, &world, point).is_some());
                world.point = point;
                graph[handle]
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        std::f32::consts::PI,
                    ));
                graph.update_hierarchical_data_for_descendants(handle);
                let camera = graph[handle].cast_mut::<Camera>().unwrap();
                camera.calculate_matrices(size);
                assert!(marker_screen(camera, size, &world, point).is_none());
                graph[handle]
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::identity());
            }
        }
    }
}
