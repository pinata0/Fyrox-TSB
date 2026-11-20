//! Source-driven clover plus batched rainfall and the existing Eru frog mesh.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    core::math::TriangleDefinition,
    material::shader::{Shader, ShaderResource},
    resource::model::{ModelResource, ModelResourceExtension},
    scene::{
        animation::AnimationPlayer,
        mesh::{
            buffer::{TriangleBuffer, VertexBuffer},
            vertex::StaticVertex,
            Mesh, RenderPath,
        },
    },
};
use trickcal_shooting_balddagu::{motion::Pose, ui_skill as ui};

const STAFF_GLOW: &str = "Ui staff glow";
pub(super) const FLASH_SECONDS: f32 = 0.24;

/// The glow's origin doubles as the launch socket, so both follow the grip's
/// animation and the different world/viewmodel scales exactly.
pub(super) fn attach_staff_glow(g: &mut Graph, root: Handle<Node>, imported: bool) {
    let shader = ShaderResource::new_embedded(
        Shader::from_string(include_str!("ui_staff_glow.shader")).expect("Ui staff glow shader"),
    );
    let glow: Handle<Node> = MeshBuilder::new(
        BaseBuilder::new()
            .with_name(STAFF_GLOW)
            .with_cast_shadows(false)
            .with_visibility(false),
    )
    .with_render_path(RenderPath::Forward)
    .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(
        SurfaceData::make_sphere(24, 24, 1.0, &Matrix4::identity()),
    ))
    .with_material(MaterialResource::new(Material::from_shader(shader)))
    .build()])
    .build(g)
    .to_base();
    // Ui_Clover's authored hub is Z=0.365 m, converted from Blender Z-up to Y-up.
    let center = if imported { 0.365 } else { 0.06 };
    g[glow].local_transform_mut()
        .set_position(Vector3::new(0.0, center, 0.0))
        .set_scale(Vector3::repeat(0.23));
    g.link_nodes(glow, root);
}

pub(super) fn staff_glow(g: &Graph, root: Handle<Node>) -> Handle<Node> {
    g[root].children().iter().copied()
        .find(|&h| g[h].name() == STAFF_GLOW)
        .expect("Ui staff glow and launch socket")
}

pub(super) fn muzzle(g: &Graph, root: Handle<Node>) -> Vector3<f32> {
    g[staff_glow(g, root)].global_position()
}

pub(super) fn weapon(g: &mut Graph, root: Handle<Node>, pose: Pose) {
    let glow = staff_glow(g, root);
    let flash = pose.motion != trickcal_shooting_balddagu::motion::Motion::Death
        && (0.0..FLASH_SECONDS).contains(&pose.shot_age);
    g[glow].set_visible(flash);
    if flash {
        let phase = pose.shot_age / FLASH_SECONDS;
        g[glow].local_transform_mut().set_scale(Vector3::repeat(0.23 + 0.07 * phase));
        g[glow].cast::<Mesh>().unwrap().surfaces()[0].material().data_ref()
            .set_property("strength", (1.0 - phase).powi(2));
    }
    let node = g
        .traverse_handle_iter(root)
        .find(|&h| g[h].name() == "Ui spinning clover hub");
    if let Some(node) = node {
        let [spin, x, y] = ui::clover(pose.motion, pose.age);
        g[node]
            .local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -spin))
            .set_scale(Vector3::new(x, y, 1.0));
    }
}
fn rain_mesh(g: &mut Graph, mat: MaterialResource, ring: bool) -> Handle<Node> {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    if ring {
        for j in 0..128 {
            let base = vertices.len() as u32;
            for (angle, r) in [
                (j as f32, ui::RAIN_RADIUS - 0.055),
                (j as f32 + 1.0, ui::RAIN_RADIUS - 0.055),
                (j as f32 + 1.0, ui::RAIN_RADIUS + 0.055),
                (j as f32, ui::RAIN_RADIUS + 0.055),
            ] {
                let angle = angle * std::f32::consts::TAU / 128.0;
                vertices.push(StaticVertex::from_pos_uv(
                    Vector3::new(angle.cos() * r, 0.025, angle.sin() * r),
                    Vector2::new(0.0, 0.0),
                ));
            }
            triangles.extend([
                TriangleDefinition([base, base + 1, base + 2]),
                TriangleDefinition([base, base + 2, base + 3]),
            ]);
        }
    } else {
        for j in 0..240 {
            let angle = j as f32 * 2.399963;
            let r = ((j as f32 + 0.5) / 240.0).sqrt() * ui::RAIN_RADIUS;
            let center = Vector3::new(angle.cos() * r, 0.0, angle.sin() * r);
            for axis in [Vector3::x(), Vector3::z()] {
                let base = vertices.len() as u32;
                for (x, y) in [(-1.0, 0.0), (1.0, 0.0), (1.0, 1.0), (-1.0, 1.0)] {
                    vertices.push(StaticVertex::from_pos_uv(
                        center + axis * x * 0.012 + Vector3::y() * y * 0.30,
                        Vector2::new(j as f32, y),
                    ));
                }
                triangles.extend([
                    TriangleDefinition([base, base + 1, base + 2]),
                    TriangleDefinition([base, base + 2, base + 3]),
                ]);
            }
        }
    }
    MeshBuilder::new(
        BaseBuilder::new()
            .with_name(if ring {
                "Ui rain boundary"
            } else {
                "Ui falling rain"
            })
            .with_cast_shadows(false)
            .with_visibility(false),
    )
    .with_render_path(RenderPath::Forward)
    .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(
        SurfaceData::new(
            VertexBuffer::new(vertices.len(), vertices).unwrap(),
            TriangleBuffer::new(triangles),
        ),
    ))
    .with_material(mat)
    .build()])
    .build(g)
    .to_base()
}
#[derive(Debug)]
pub(super) struct Effects {
    rain: Vec<[Handle<Node>; 2]>,
    materials: Vec<MaterialResource>,
    pub frogs: [Handle<Node>; PLAYERS],
}
impl Effects {
    pub fn new(g: &mut Graph, model: &ModelResource) -> Self {
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("ui_rain.shader")).expect("Ui rain shader"),
        );
        let materials: Vec<_> = (0..MAX_PROJECTILES)
            .map(|_| MaterialResource::new(Material::from_shader(shader.clone())))
            .collect();
        let rain = materials
            .iter()
            .map(|m| {
                let mut ring = Material::from_shader(shader.clone());
                ring.set_property("ring", 1.0);
                [
                    rain_mesh(g, m.clone(), false),
                    rain_mesh(g, MaterialResource::new(ring), true),
                ]
            })
            .collect();
        let data = model.data_ref();
        let frogs = std::array::from_fn(|_| {
            let (root, _) = ModelResource::instantiate_from(
                model.clone(),
                &data,
                data.get_scene().graph.get_root(),
                g,
                &mut |_, _| {},
            );
            let nodes: Vec<_> = g.traverse_handle_iter(root).collect();
            for node in nodes {
                if g[node].cast::<Mesh>().is_some() {
                    let frog = g[node].name() == "Frog";
                    g[node].set_visible(frog);
                }
                if let Some(player) = g[node].cast_mut::<AnimationPlayer>() {
                    player.set_enabled(false);
                }
            }
            g[root].set_name("Ui polymorph frog");
            g[root].set_visible(false);
            g[root]
                .local_transform_mut()
                .set_scale(Vector3::repeat(0.0075));
            root
        });
        Self {
            rain,
            materials,
            frogs,
        }
    }
    pub fn update(
        &self,
        g: &mut Graph,
        w: &World,
        actors: &[Actor; PLAYERS],
        view: usize,
        first: bool,
        enabled: bool,
    ) {
        let fields: Vec<_> = w
            .projectiles
            .iter()
            .filter(|p| p.kind == ProjectileKind::RainField)
            .collect();
        for (i, nodes) in self.rain.iter().enumerate() {
            let field = fields.get(i).copied();
            for &node in nodes {
                g[node].set_visible(enabled && field.is_some());
            }
            if let Some(p) = field {
                self.materials[i].data_ref().set_property("age", p.age);
                for &node in nodes {
                    g[node].local_transform_mut().set_position(vec(p.pos));
                }
            }
        }
        for (i, a) in actors.iter().enumerate() {
            let show = enabled && a.alive() && a.combat.polymorph > 0.0 && !(first && i == view);
            let node = self.frogs[i];
            g[node].set_visible(show);
            if show {
                let bob = (w.tick as f32 * 0.24).sin().abs() * 0.045;
                g[node]
                    .local_transform_mut()
                    .set_position(vec(a.pos) + Vector3::y() * bob)
                    .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), a.yaw));
            }
        }
    }
}
