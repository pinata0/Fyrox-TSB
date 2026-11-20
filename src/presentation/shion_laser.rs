//! Scoped Shion rays: a fixed, confirmed hit segment with a brief violet afterglow.
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

pub(super) const SECONDS: f32 = 0.18;

pub(super) struct Lasers {
    nodes: [Handle<Node>; PLAYERS],
    materials: [MaterialResource; PLAYERS],
    paths: [(Vector3<f32>, Vector3<f32>); PLAYERS],
    active: [bool; PLAYERS],
    pending: [bool; PLAYERS],
}

impl std::fmt::Debug for Lasers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShionLasers")
            .field("active", &self.active)
            .finish()
    }
}

impl Lasers {
    pub fn new(g: &mut Graph) -> Self {
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("shion_laser.shader")).expect("Shion laser shader"),
        );
        let vertices = [
            (-0.5, 0.0, 0.0, 0.0),
            (0.5, 0.0, 1.0, 0.0),
            (-0.5, 1.0, 0.0, 1.0),
            (0.5, 1.0, 1.0, 1.0),
        ]
        .map(|(x, y, u, v)| StaticVertex::from_pos_uv(Vector3::new(x, y, 0.0), Vector2::new(u, v)));
        let mesh = SurfaceResource::new_embedded(SurfaceData::new(
            VertexBuffer::new(vertices.len(), vertices.to_vec()).unwrap(),
            TriangleBuffer::new(vec![
                TriangleDefinition([0, 1, 2]),
                TriangleDefinition([1, 3, 2]),
            ]),
        ));
        let materials =
            std::array::from_fn(|_| MaterialResource::new(Material::from_shader(shader.clone())));
        let nodes = std::array::from_fn(|i| {
            MeshBuilder::new(
                BaseBuilder::new()
                    .with_name("Shion scoped laser")
                    .with_cast_shadows(false)
                    .with_visibility(false),
            )
            .with_render_path(RenderPath::Forward)
            .with_surfaces(vec![SurfaceBuilder::new(mesh.clone())
                .with_material(materials[i].clone())
                .build()])
            .build(g)
            .to_base()
        });
        Self {
            nodes,
            materials,
            paths: [(Vector3::zeros(), Vector3::zeros()); PLAYERS],
            active: [false; PLAYERS],
            pending: [false; PLAYERS],
        }
    }

    pub fn shot(&mut self, slot: usize, origin: V3, end: V3, active: bool) {
        self.paths[slot] = (vec(origin), vec(end));
        self.active[slot] = active;
        self.pending[slot] = active;
    }

    pub fn update(&self, g: &mut Graph, slot: usize, remaining: f32, visible: bool) -> bool {
        let show = self.active[slot]
            && visible
            && remaining > 0.0
            && (self.paths[slot].1 - self.paths[slot].0).norm() > 0.01;
        g[self.nodes[slot]].set_visible(show);
        self.materials[slot]
            .data_ref()
            .set_property("opacity", (remaining / SECONDS).clamp(0.0, 1.0).sqrt());
        self.active[slot]
    }

    pub fn pose(&mut self, g: &mut Graph, slot: usize, muzzle: Vector3<f32>, camera: Vector3<f32>) {
        let node = self.nodes[slot];
        if !g[node].visibility() {
            return;
        }
        if self.pending[slot] {
            let (origin, end) = self.paths[slot];
            // At point blank range the confirmed hit may be behind the barrel.
            // Keep the beam on the near side of the hit instead of shooting back.
            let ray = end - origin;
            self.paths[slot].0 = if (end - muzzle).dot(&ray) > 0.0 {
                muzzle
            } else {
                origin
            };
            self.pending[slot] = false;
        }
        let (start, end) = self.paths[slot];
        let delta = end - start;
        let Some(dir) = delta.try_normalize(0.0001) else {
            g[node].set_visible(false);
            return;
        };
        let view = camera - (start + end) * 0.5;
        let across = dir.cross(&view).try_normalize(0.0001).unwrap_or_else(|| {
            dir.cross(&Vector3::y())
                .try_normalize(0.0001)
                .unwrap_or(Vector3::x())
        });
        g[node]
            .local_transform_mut()
            .set_position(start)
            .set_rotation(UnitQuaternion::from_matrix(
                &fyrox::core::algebra::Matrix3::from_columns(&[across, dir, across.cross(&dir)]),
            ))
            .set_scale(Vector3::new(0.16, delta.norm(), 1.0));
        g.update_hierarchical_data_for_descendants(node);
    }

    pub fn verify(&self, g: &Graph, slot: usize, visible: bool) {
        let node = self.nodes[slot];
        assert_eq!(
            g[node].visibility(),
            visible,
            "scoped shot laser visibility"
        );
        if visible {
            let (start, end) = self.paths[slot];
            assert!(
                !self.pending[slot],
                "laser must capture the animated muzzle"
            );
            assert!((g[node].global_position() - start).norm() < 0.001);
            let tip = g[node]
                .global_transform()
                .transform_point(&Vector3::y().into())
                .coords;
            assert!(
                (tip - end).norm() < 0.001,
                "laser must stop at the confirmed hit"
            );
        }
    }
}
