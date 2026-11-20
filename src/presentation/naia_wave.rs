//! Independent broad curling ultimate wave built from the generated reference.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    core::math::TriangleDefinition,
    material::shader::{Shader, ShaderResource},
    scene::mesh::{
        buffer::{TriangleBuffer, VertexBuffer},
        vertex::StaticVertex,
        RenderPath,
    },
};

#[derive(serde::Deserialize)]
struct Part {
    vertices: Vec<[f32; 5]>,
    triangles: Vec<[u32; 3]>,
    foam: bool,
}
#[derive(Debug)]
pub(super) struct Wave {
    pub nodes: [Handle<Node>; PLAYERS],
    materials: [MaterialResource; 2],
}
impl Wave {
    pub fn new(g: &mut Graph) -> Self {
        let parts: Vec<Part> = serde_json::from_str(include_str!(
            "../../assets/effects/naia-ultimate-wave-mesh.json"
        ))
        .expect("water mesh");
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("naia_wave.shader")).expect("water shader"),
        );
        let materials = std::array::from_fn(|i| {
            let mut m = Material::from_shader(shader.clone());
            m.set_property("foam", i as f32);
            MaterialResource::new(m)
        });
        // Merge the modeled parts into two surfaces: liquid and foam, shared by every actor.
        let surfaces: Vec<_> = (0..2)
            .map(|kind| {
                let mut vertices = Vec::new();
                let mut triangles = Vec::new();
                for p in parts.iter().filter(|p| usize::from(p.foam) == kind) {
                    let base = vertices.len() as u32;
                    vertices.extend(p.vertices.iter().map(|v| {
                        StaticVertex::from_pos_uv(
                            Vector3::new(v[0], v[1], v[2]),
                            Vector2::new(v[3], v[4]),
                        )
                    }));
                    triangles.extend(
                        p.triangles
                            .iter()
                            .map(|t| TriangleDefinition(t.map(|i| i + base))),
                    );
                }
                let data = SurfaceData::new(
                    VertexBuffer::new(vertices.len(), vertices).expect("water vertices"),
                    TriangleBuffer::new(triangles),
                );
                SurfaceBuilder::new(SurfaceResource::new_embedded(data))
                    .with_material(materials[kind].clone())
                    .build()
            })
            .collect();
        let nodes = std::array::from_fn(|_| {
            MeshBuilder::new(
                BaseBuilder::new()
                    .with_name("Naia ultimate tidal wave")
                    .with_visibility(false)
                    .with_cast_shadows(false),
            )
            .with_render_path(RenderPath::Forward)
            .with_surfaces(surfaces.clone())
            .build(g)
            .to_base()
        });
        Self { nodes, materials }
    }
    pub fn update(&self, g: &mut Graph, world: &World) {
        for m in &self.materials {
            m.data_ref().set_property("time", world.tick as f32 * DT);
        }
        let mut waves = world
            .projectiles
            .iter()
            .filter(|p| p.kind == ProjectileKind::Wave);
        for node in self.nodes {
            let Some(p) = waves.next() else {
                g[node].set_visible(false);
                continue;
            };
            g[node].set_visible(true);
            let yaw = p.velocity.x.atan2(p.velocity.z);
            g[node]
                .local_transform_mut()
                .set_position(vec(p.pos))
                .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw))
                .set_scale(Vector3::repeat(1.0));
        }
    }
}
