//! Sebastian's own mesh supplies the two view limbs and the nine miniature dolls.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    resource::model::ModelResource,
    scene::mesh::{
        buffer::{TriangleBuffer, VertexAttributeUsage, VertexBuffer, VertexReadTrait},
        surface::Surface,
        vertex::StaticVertex,
        Mesh,
    },
};
use trickcal_shooting_balddagu::{
    chloe_skill as c,
    motion::{Motion, Pose},
};

fn mesh(g: &mut Graph, name: &str, surfaces: Vec<Surface>) -> Handle<Node> {
    MeshBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_cast_shadows(false)
            .with_visibility(false),
    )
    .with_surfaces(surfaces)
    .build(g)
    .to_base()
}
/// Flatten the rest mesh once, retaining source UVs/materials. The 3D limbs have
/// independent transforms; no camera-attached copy of Chloe's body can leak in.
fn surfaces(model: &ModelResource, side: Option<bool>) -> Vec<Surface> {
    let data = model.data_ref();
    let graph = &data.get_scene().graph;
    let node = graph
        .linear_iter()
        .find(|n| n.name() == "Sebastian")
        .expect("Sebastian mesh");
    let source = node.cast::<Mesh>().unwrap();
    if let Some(left) = side {
        #[derive(serde::Deserialize)]
        struct Part {
            side: String,
            cap: bool,
            vertices: Vec<[f32; 5]>,
            triangles: Vec<[u32; 3]>,
        }
        // Load from the same asset set as Sebastian: a remesh changes both
        // the limb geometry and its UVs, including during staged art reviews.
        let source_json = std::fs::read_to_string(characters::asset_path("chloe-view-hands.json"))
            .expect("Sebastian view limb source");
        let parts: Vec<Part> = serde_json::from_str(&source_json)
        .expect("closed Sebastian view limbs");
        return parts
            .into_iter()
            .filter(|p| p.side == if left { "R" } else { "L" })
            .map(|p| {
                let vertices: Vec<_> = p
                    .vertices
                    .iter()
                    .map(|v| {
                        StaticVertex::from_pos_uv(
                            Vector3::new(v[0], v[1], v[2]),
                            Vector2::new(v[3], v[4]),
                        )
                    })
                    .collect();
                let mut data = SurfaceData::new(
                    VertexBuffer::new(vertices.len(), vertices).unwrap(),
                    TriangleBuffer::new(
                        p.triangles
                            .into_iter()
                            .map(fyrox::core::math::TriangleDefinition)
                            .collect(),
                    ),
                );
                data.calculate_normals().unwrap();
                data.calculate_tangents().unwrap();
                let mat = if p.cap {
                    material(if left {
                        [211, 124, 153]
                    } else {
                        [136, 197, 194]
                    })
                } else {
                    source.surfaces()[0].material().clone()
                };
                SurfaceBuilder::new(SurfaceResource::new_embedded(data))
                    .with_material(mat)
                    .build()
            })
            .collect();
    }
    source
        .surfaces()
        .iter()
        .map(|s| {
            let d = s.data_ref().data_ref();
            let points: Vec<_> = d
                .vertex_buffer
                .iter()
                .map(|v| {
                    node.global_transform()
                        .transform_point(
                            &v.read_3_f32(VertexAttributeUsage::Position).unwrap().into(),
                        )
                        .coords
                })
                .collect();
            let lo = points
                .iter()
                .fold(Vector3::repeat(f32::INFINITY), |a, p| a.inf(p));
            let hi = points
                .iter()
                .fold(Vector3::repeat(f32::NEG_INFINITY), |a, p| a.sup(p));
            let center = (lo + hi) * 0.5;
            let scale = 0.58 / (hi.y - lo.y).max(0.001);
            let vertices: Vec<_> = d
                .vertex_buffer
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let p = points[i];
                    let p = Vector3::new(
                        p.x - center.x,
                        p.y - lo.y - 0.25 * (hi.y - lo.y),
                        p.z - center.z,
                    ) * scale;
                    StaticVertex::from_pos_uv(
                        p,
                        v.read_2_f32(VertexAttributeUsage::TexCoord0)
                            .unwrap_or_default(),
                    )
                })
                .collect();
            let triangles = d.geometry_buffer.iter().copied().collect();
            let mut d = SurfaceData::new(
                VertexBuffer::new(vertices.len(), vertices).unwrap(),
                TriangleBuffer::new(triangles),
            );
            d.calculate_normals().unwrap();
            d.calculate_tangents().unwrap();
            SurfaceBuilder::new(SurfaceResource::new_embedded(d))
                .with_material(s.material().clone())
                .build()
        })
        .collect()
}
#[derive(Debug)]
pub(super) struct Effects {
    pub hands: [Handle<Node>; 2],
    dolls: Vec<Handle<Node>>,
    confetti: Vec<Vec<Handle<Node>>>,
    doll_surfaces: Vec<Surface>,
    paper_surfaces: [Surface; 5],
}
impl Effects {
    pub fn new(
        g: &mut Graph,
        model: &ModelResource,
        camera: Handle<Node>,
        shapes: &Shapes,
    ) -> Self {
        let hands = [false, true].map(|left| {
            let h = mesh(
                g,
                if left {
                    "Sebastian view left"
                } else {
                    "Sebastian view right"
                },
                surfaces(model, Some(left)),
            );
            g.link_nodes(h, camera);
            g[h].set_frustum_culling(false);
            h
        });
        let doll_surfaces = surfaces(model, None);
        let dolls = vec![Handle::NONE; MAX_PROJECTILES];
        let colors = [
            [255, 108, 159],
            [110, 230, 226],
            [255, 218, 85],
            [185, 123, 245],
            [245, 245, 255],
        ];
        let paper_surfaces = colors.map(|color| {
            SurfaceBuilder::new(shapes.cube.clone()).with_material(material(color)).build()
        });
        let confetti = (0..MAX_PROJECTILES).map(|_| Vec::new()).collect();
        Self {
            hands,
            dolls,
            confetti,
            doll_surfaces,
            paper_surfaces,
        }
    }
    pub fn update(
        &mut self,
        g: &mut Graph,
        w: &World,
        a: &Actor,
        pose: Pose,
        first: bool,
        enabled: bool,
    ) {
        for (i, h) in self.hands.iter().enumerate() {
            let show = first && enabled && a.character == 6;
            g[*h].set_visible(show);
            if !show { continue; }
            let arm = c::view_arm(pose.motion, pose.age, i == 1);
            let tip = Vector3::from(arm.tip);
            // The extracted limb runs from -Z at the shoulder to zero at the paw.
            // Aim its shoulder out through its own corner even at full extension.
            let direction = (tip - Vector3::from(arm.corner)).normalize();
            let rotation = UnitQuaternion::rotation_between(&Vector3::z(), &direction)
                .unwrap_or_default()
                * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), arm.roll);
            g[*h]
                .local_transform_mut()
                .set_position(tip)
                .set_rotation(rotation)
                // A fixed longer forearm keeps the original shoulder cut below
                // the frame throughout the wide swing; never stretch by phase.
                .set_scale(Vector3::new(arm.scale, arm.scale, arm.scale * 1.60));
        }
        for i in 0..MAX_PROJECTILES {
            let p = w.projectiles.get(i);
            let doll = p.filter(|p| enabled && p.kind == ProjectileKind::Doll);
            if doll.is_some() && self.dolls[i].is_none() {
                self.dolls[i] = mesh(g, "Petit Sebastian", self.doll_surfaces.clone());
            }
            if self.dolls[i].is_some() { g[self.dolls[i]].set_visible(doll.is_some()); }
            if let Some(p) = doll {
                let yaw = if p.velocity.flat().length() > 0.01 {
                    p.velocity.x.atan2(p.velocity.z)
                } else {
                    w.actors[p.owner].yaw
                };
                let t = w.tick as f32 * DT + i as f32 * 0.41;
                g[self.dolls[i]]
                    .local_transform_mut()
                    .set_position(vec(p.pos) + Vector3::y() * (t * 18.0).sin().abs() * 0.055)
                    .set_rotation(UnitQuaternion::from_euler_angles(
                        0.08,
                        yaw,
                        (t * 18.0).sin() * 0.10,
                    ));
            }
            let p = p.filter(|p| enabled && p.kind == ProjectileKind::Confetti);
            if p.is_some() && self.confetti[i].is_empty() {
                self.confetti[i] = (0..20).map(|j| {
                    mesh(g, "Confetti paper", vec![self.paper_surfaces[j % 5].clone()])
                }).collect();
            }
            for (j, &h) in self.confetti[i].iter().enumerate() {
                g[h].set_visible(p.is_some());
                if let Some(p) = p {
                    let t = p.age;
                    let angle = j as f32 * 2.399963;
                    let speed = 1.6 + (j % 4) as f32 * 0.38;
                    let radius = t * speed;
                    g[h].local_transform_mut()
                        .set_position(
                            vec(p.pos)
                                + Vector3::new(
                                    angle.cos() * radius,
                                    0.12 + (1.0 + (j % 3) as f32 * 0.6) * t - 2.8 * t * t,
                                    angle.sin() * radius,
                                ),
                        )
                        .set_rotation(UnitQuaternion::from_euler_angles(
                            t * 9.0 + j as f32,
                            t * 13.0,
                            angle,
                        ))
                        .set_scale(
                            Vector3::new(0.045, 0.09, 0.008) * (1.0 - c::smooth((t - 0.52) / 0.18)),
                        );
                }
            }
        }
    }
}
pub fn face(g: &mut Graph, root: Handle<Node>, p: Pose) {
    let scissor = p.motion == Motion::ChloeMount && c::spread(p.age) > 0.15
        || p.motion == Motion::ChloeRoar && p.age > 0.6 && p.age < 1.7;
    let angry = p.base().0 == Motion::ChloeRideMove;
    let closed = matches!(p.base().0, Motion::Walk | Motion::Run)
        || scissor
        || p.motion == Motion::ChloeRoar;
    let nodes: Vec<_> = g
        .traverse_handle_iter(root)
        .filter(|h| {
            matches!(
                g[*h].name(),
                "ChloeScissorEyes" | "ChloeMoveEyes" | "SebastianClosedEyes"
            )
        })
        .collect();
    for h in nodes {
        let show = match g[h].name() {
            "ChloeScissorEyes" => scissor,
            "ChloeMoveEyes" => angry,
            _ => closed,
        };
        g[h].set_visible(show);
    }
}
