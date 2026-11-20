//! Blue ankle haze for observers, blue screen edges and a small FOV change in first person.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    core::math::TriangleDefinition,
    gui::{brush::GradientPoint, Thickness},
    material::shader::{Shader, ShaderResource},
    scene::mesh::{
        buffer::{TriangleBuffer, VertexBuffer},
        vertex::StaticVertex,
        RenderPath,
    },
};
use std::f32::consts::TAU;

/// Visual strength, independent of haste and voluntary movement restrictions.
fn strength(actor: &Actor) -> f32 {
    if !actor.alive() || trickcal_shooting_balddagu::erpin_skill::unstoppable(actor) {
        return 0.0;
    }
    let c = &actor.combat;
    let movement = if c.slow > 0.0 {
        0.35
    } else if c.rain_slow > 0.0 {
        0.15
    } else {
        0.0
    };
    ((1.0 - (1.0 - movement) * (1.0 - c.temporal.clamp(0.0, 1.0))) / 0.35).clamp(0.0, 1.0)
}

pub(super) fn view_strength(rt: &Runtime) -> f32 {
    let world = rt.session.world();
    let actor = &world.actors[rt.session.slot()];
    if !matches!(world.phase, Phase::Setup | Phase::Playing)
        || !actor.alive()
        || (actor.dancing && actor.wave_ticks == 0)
        || selection::showing(rt)
        || rt.art_review
        || rt.home_open
        || rt.lobby.open
        || rt.startup.active()
        || rt.settings_menu.open
        || rt.character_help.open
        || rt.keys.score
        || rt.wheel.open
        || !rt.grabbed
        || rt.error.is_some()
    {
        return 0.0;
    }
    match &rt.session {
        Session::Join(client) => strength(&client.motion.actor(
            world,
            rt.session.slot(),
            rt.session.slot(),
        )),
        _ => strength(actor),
    }
}

#[derive(Debug, Default)]
pub(super) struct Screen {
    pub(super) root: Handle<UiNode>,
}
impl Screen {
    pub fn new(ui: &mut UserInterface) -> Self {
        let root = BorderBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_background(
                    Brush::RadialGradient {
                        center: Vector2::new(0.5, 0.5),
                        stops: [
                            (0.0, [28, 111, 235, 0]),
                            (0.29, [28, 111, 235, 0]),
                            (0.40, [28, 111, 235, 18]),
                            (0.50, [24, 105, 230, 85]),
                            (0.63, [15, 64, 178, 155]),
                            (1.0, [10, 40, 130, 175]),
                        ]
                        .into_iter()
                        .map(|(stop, [r, g, b, a])| GradientPoint {
                            stop,
                            color: Color::from_rgba(r, g, b, a),
                        })
                        .collect(),
                    }
                    .into(),
                ),
        )
        .with_stroke_thickness(Thickness::uniform(0.0).into())
        .build(&mut ui.build_ctx())
        .to_base();
        Self { root }
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface, show: bool) {
        let strength = view_strength(rt);
        let active = show && strength > 0.0;
        visible(ui, self.root, active);
        if active {
            ui.send(
                self.root,
                WidgetMessage::Opacity(Some(0.55 + strength * 0.45)),
            );
            let size = ui.screen_size();
            place(ui, self.root, 0.0, 0.0, size.x, size.y);
        }
    }
}

#[derive(Debug)]
pub(super) struct Feet {
    pub(super) nodes: [Handle<Node>; PLAYERS],
    materials: [MaterialResource; PLAYERS],
}
impl Feet {
    pub fn new(g: &mut Graph) -> Self {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        // A soft disc and a short translucent skirt tint the ground and lower legs.
        for j in 0..48 {
            let a = j as f32 * TAU / 48.0;
            let b = (j + 1) as f32 * TAU / 48.0;
            let base = vertices.len() as u32;
            let point =
                |angle: f32, y: f32| Vector3::new(angle.cos() * 0.68, y, angle.sin() * 0.68);
            for p in [
                // Above the team marker's top face (0.034 m), so it cannot hide blue.
                Vector3::new(0.0, 0.05, 0.0),
                point(a, 0.05),
                point(b, 0.05),
                point(a, 0.065),
                point(b, 0.065),
                point(b, 0.46),
                point(a, 0.46),
            ] {
                vertices.push(StaticVertex::from_pos_uv(p, Vector2::zeros()));
            }
            triangles.extend([
                TriangleDefinition([base, base + 1, base + 2]),
                TriangleDefinition([base + 3, base + 4, base + 5]),
                TriangleDefinition([base + 3, base + 5, base + 6]),
            ]);
        }
        let surface = SurfaceResource::new_embedded(SurfaceData::new(
            VertexBuffer::new(vertices.len(), vertices).unwrap(),
            TriangleBuffer::new(triangles),
        ));
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("slow_foot.shader")).expect("Slow foot shader"),
        );
        let materials =
            std::array::from_fn(|_| MaterialResource::new(Material::from_shader(shader.clone())));
        let nodes = std::array::from_fn(|i| {
            MeshBuilder::new(
                BaseBuilder::new()
                    .with_name("Slow blue ankle haze")
                    .with_visibility(false)
                    .with_cast_shadows(false),
            )
            .with_render_path(RenderPath::Forward)
            .with_surfaces(vec![SurfaceBuilder::new(surface.clone())
                .with_material(materials[i].clone())
                .build()])
            .build(g)
            .to_base()
        });
        Self { nodes, materials }
    }
    pub fn update(
        &self,
        g: &mut Graph,
        actors: &[Actor; PLAYERS],
        view: usize,
        first: bool,
        show: bool,
        time: f32,
    ) {
        for (i, actor) in actors.iter().enumerate() {
            let strength = strength(actor);
            let active = show && strength > 0.0 && !actor.burrowed() && !(first && i == view);
            g[self.nodes[i]].set_visible(active);
            if active {
                self.materials[i]
                    .data_ref()
                    .set_property("strength", strength);
                self.materials[i].data_ref().set_property("time", time);
                g[self.nodes[i]]
                    .local_transform_mut()
                    .set_position(vec(actor.pos))
                    .set_scale(Vector3::repeat(actor.body_scale()));
            }
        }
    }
}
