//! Separate padded quilt, hollow food tin and giant landing shock ring.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    core::math::TriangleDefinition,
    scene::mesh::{
        buffer::{TriangleBuffer, VertexBuffer},
        surface::Surface,
        vertex::StaticVertex,
    },
};
use trickcal_shooting_balddagu::{
    komi_skill as k,
    motion::{Motion, Pose},
};

#[derive(serde::Deserialize)]
struct Part {
    group: String,
    color: [u8; 3],
    vertices: Vec<[f32; 6]>,
    triangles: Vec<[u32; 3]>,
}
#[derive(Debug)]
pub(super) struct Props {
    pub blanket: Handle<Node>,
    pub can: Handle<Node>,
    lid: Handle<Node>,
}
impl Props {
    fn new(g: &mut Graph, surfaces: &[(String, Surface)]) -> Self {
        let mut mesh = |name: &str, group: &str| -> Handle<Node> {
            MeshBuilder::new(BaseBuilder::new().with_name(name).with_visibility(false))
                .with_surfaces(
                    surfaces
                        .iter()
                        .filter(|s| s.0 == group)
                        .map(|s| s.1.clone())
                        .collect(),
                )
                .build(g)
                .to_base()
        };
        let blanket = mesh("Komi separate padded quilt", "blanket");
        let can = mesh("Komi separate food tin", "can");
        let lid = mesh("Komi peeled metal lid", "lid");
        g.link_nodes(lid, can);
        g[lid].set_visible(true);
        Self { blanket, can, lid }
    }
    pub fn can(
        &self,
        g: &mut Graph,
        p: Pose,
        origin: Vector3<f32>,
        rotation: UnitQuaternion<f32>,
        scale: f32,
        show: bool,
    ) {
        g[self.can]
            .set_visible(show && p.motion == Motion::KomiFeast && (0.10..4.10).contains(&p.age));
        g[self.can]
            .local_transform_mut()
            .set_position(origin)
            .set_rotation(rotation)
            .set_scale(Vector3::repeat(scale));
        let opening = ((p.age - 0.70) / 0.27).clamp(0.0, 1.0);
        g[self.lid]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, 0.44, -0.20))
            .set_rotation(UnitQuaternion::from_euler_angles(-opening * 2.0, 0.0, 0.0));
        g.update_hierarchical_data_for_descendants(self.can);
    }
}
pub(super) fn tin_rotation(p: Pose) -> UnitQuaternion<f32> {
    let drink =
        ((p.age - 1.3) / 0.4).clamp(0.0, 1.0) * (1.0 - ((p.age - 2.4) / 0.6).clamp(0.0, 1.0));
    UnitQuaternion::from_euler_angles(-drink * 1.15, 0.0, 0.06 * (p.age * 8.0).sin())
}
pub(super) fn view_tin(p: Pose) -> Vector3<f32> {
    let lift =
        ((p.age - 0.1) / 0.8).clamp(0.0, 1.0) * (1.0 - ((p.age - 3.4) / 0.7).clamp(0.0, 1.0));
    Vector3::new(0.0, -0.60 + lift * 0.35, 0.72)
}
pub(super) fn grip(left: bool) -> Vector3<f32> {
    Vector3::new(if left { 0.22 } else { -0.22 }, 0.20, 0.0)
}
#[derive(Debug)]
pub(super) struct Effects {
    pub world: [Props; PLAYERS],
    pub view: Props,
    rings: [Handle<Node>; PLAYERS],
}
impl Effects {
    pub fn new(g: &mut Graph) -> Self {
        let parts: Vec<Part> =
            serde_json::from_str(include_str!("../../assets/effects/komi-props.json"))
                .expect("Komi prop meshes");
        let surfaces: Vec<_> = parts
            .into_iter()
            .map(|p| {
                let verts: Vec<_> = p
                    .vertices
                    .iter()
                    .map(|v| {
                        StaticVertex::from_pos_uv(Vector3::new(v[0], v[1], v[2]), Vector2::zeros())
                    })
                    .collect();
                let mut data = SurfaceData::new(
                    VertexBuffer::new(verts.len(), verts).unwrap(),
                    TriangleBuffer::new(p.triangles.into_iter().map(TriangleDefinition).collect()),
                );
                data.calculate_normals().unwrap();
                (
                    p.group,
                    SurfaceBuilder::new(SurfaceResource::new_embedded(data))
                        .with_material(material(p.color))
                        .build(),
                )
            })
            .collect();
        let world = std::array::from_fn(|_| Props::new(g, &surfaces));
        let view = Props::new(g, &surfaces);
        character_shadows::disable_casting(g, view.can);
        character_shadows::disable_casting(g, view.blanket);
        let mut verts = Vec::new();
        let mut tris = Vec::new();
        for i in 0..96 {
            let a = i as f32 * std::f32::consts::TAU / 96.0;
            for r in [0.96, 1.0] {
                verts.push(StaticVertex::from_pos_uv(
                    Vector3::new(a.cos() * r, 0.0, a.sin() * r),
                    Vector2::zeros(),
                ));
            }
            let b = i * 2;
            let n = (i + 1) % 96 * 2;
            tris.extend([
                TriangleDefinition([b, n, b + 1]),
                TriangleDefinition([b + 1, n, n + 1]),
                TriangleDefinition([b + 1, n, b]),
                TriangleDefinition([n + 1, n, b + 1]),
            ]);
        }
        let mut data = SurfaceData::new(
            VertexBuffer::new(verts.len(), verts).unwrap(),
            TriangleBuffer::new(tris),
        );
        data.calculate_normals().unwrap();
        let surface = SurfaceBuilder::new(SurfaceResource::new_embedded(data))
            .with_material(material([255, 188, 221]))
            .build();
        let rings = std::array::from_fn(|_| {
            MeshBuilder::new(
                BaseBuilder::new()
                    .with_name("Komi giant landing")
                    .with_visibility(false)
                    .with_cast_shadows(false),
            )
            .with_surfaces(vec![surface.clone()])
            .build(g)
            .to_base()
        });
        Self { world, view, rings }
    }
    pub fn actor(&self, g: &mut Graph, i: usize, a: &Actor, p: Pose, show: bool) {
        let rest = show && a.alive() && a.character == 12 && a.combat.komi.resting();
        let spread = (p.age / 0.7).clamp(0.0, 1.0) * (1.0 - ((p.age - 5.5) / 0.6).clamp(0.0, 1.0));
        g[self.world[i].blanket].set_visible(rest && spread > 0.01);
        g[self.world[i].blanket]
            .local_transform_mut()
            .set_position(vec(a.pos))
            .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), a.yaw))
            .set_scale(Vector3::new(
                1.0,
                0.92 + (0.08 * (p.age * 4.2).sin()),
                spread,
            ));
        let impact = a.combat.komi.impact;
        g[self.rings[i]].set_visible(show && a.alive() && impact > 0.0);
        let radius = (1.0 - impact / 0.35).clamp(0.0, 1.0) * k::LANDING_RADIUS;
        g[self.rings[i]]
            .local_transform_mut()
            .set_position(vec(a.pos) + Vector3::y() * 0.035)
            .set_scale(Vector3::repeat(radius));
    }
}
