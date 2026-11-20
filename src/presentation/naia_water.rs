//! Animated continuous liquid mesh, spiral foam, spray and splash crown.
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
use trickcal_shooting_balddagu::naia_skill;

#[derive(serde::Deserialize)]
struct Part {
    vertices: Vec<[f32; 5]>,
    triangles: Vec<[u32; 3]>,
    foam: bool,
}
#[derive(Debug)]
pub(super) struct Water {
    pub nodes: [Handle<Node>; PLAYERS],
    materials: [MaterialResource; 2],
}
impl Water {
    pub fn new(g: &mut Graph) -> Self {
        let parts: Vec<Part> =
            serde_json::from_str(include_str!("../../assets/effects/naia-water-mesh.json"))
                .expect("water mesh");
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("naia_water.shader")).expect("water shader"),
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
                    .with_name("Naia healing water stream")
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
    pub fn update(
        &self,
        g: &mut Graph,
        slot: usize,
        a: &Actor,
        map: &[trickcal_shooting_balddagu::game::Block],
        origin: Vector3<f32>,
        time: f32,
    ) {
        let node = self.nodes[slot];
        let enabled =
            a.character == 1 && a.alive() && a.combat.naia.phase == naia_skill::Phase::Fire;
        g[node].set_visible(enabled);
        if !enabled {
            return;
        }
        for m in &self.materials {
            m.data_ref().set_property("time", time);
        }
        let end = vec(naia_skill::stream_end(map, a));
        let delta = end - origin;
        if delta.norm() < 0.15 || delta.dot(&vec(direction(a.yaw, a.pitch))) <= 0.0 {
            g[node].set_visible(false);
            return;
        }
        let rotation =
            UnitQuaternion::rotation_between(&Vector3::z(), &delta.normalize()).unwrap_or_default();
        g[node]
            .local_transform_mut()
            .set_position(origin)
            .set_rotation(rotation)
            .set_scale(Vector3::new(1.0, 1.0, delta.norm()));
    }
}
