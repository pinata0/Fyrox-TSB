//! Sleeve stickers and spells emitted from the animated staff head in both views.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    core::math::TriangleDefinition,
    material::shader::{Shader, ShaderResource},
    scene::mesh::{
        buffer::{TriangleBuffer, VertexAttributeUsage, VertexBuffer, VertexReadTrait},
        surface::Surface,
        vertex::StaticVertex,
        Mesh, RenderPath,
    },
};
use trickcal_shooting_balddagu::{motion::Pose, picora_skill as p};

/// Cache one actual sleeve vertex and its skin weights, then follow that surface.
#[derive(Debug)]
struct Anchor {
    root: Handle<Node>,
    position: Vector3<f32>,
    normal: Vector3<f32>,
    bones: Vec<(Handle<Node>, f32)>,
}
impl Anchor {
    fn world(&self, g: &Graph) -> (Vector3<f32>, Vector3<f32>) {
        let mut p = Vector3::zeros();
        let mut n = Vector3::zeros();
        for &(bone, weight) in &self.bones {
            let m = g[bone].global_transform() * g[bone].inv_bind_pose_transform();
            p += m.transform_point(&self.position.into()).coords * weight;
            n += m.transform_vector(&self.normal) * weight;
        }
        (p, n.try_normalize(0.00001).unwrap_or(Vector3::z()))
    }
    fn find(
        g: &Graph,
        root: Handle<Node>,
        forearm: Handle<Node>,
        target: Vector3<f32>,
        outward: Vector3<f32>,
        first: bool,
    ) -> Option<Self> {
        let mut best = None;
        let mut score = f32::MAX;
        let view = first.then(|| {
            g[g[g[root].parent()].parent()]
                .global_transform()
                .try_inverse()
                .unwrap_or_default()
        });
        for node in g.traverse_handle_iter(root) {
            if !g[node].visibility() || g[node].name().contains("outline") {
                continue;
            }
            let Some(mesh) = g[node].cast::<Mesh>() else {
                continue;
            };
            for surface in mesh.surfaces() {
                let data = surface.data_ref().data_ref();
                let drawn: std::collections::BTreeSet<_> =
                    data.geometry_buffer.iter().flat_map(|t| t.0).collect();
                for index in drawn {
                    let vertex = data.vertex_buffer.get(index as usize).unwrap();
                    let (Ok(indices), Ok(weights), Ok(position), Ok(normal)) = (
                        vertex.read_4_u8(VertexAttributeUsage::BoneIndices),
                        vertex.read_4_f32(VertexAttributeUsage::BoneWeight),
                        vertex.read_3_f32(VertexAttributeUsage::Position),
                        vertex.read_3_f32(VertexAttributeUsage::Normal),
                    ) else {
                        continue;
                    };
                    let bones: Vec<_> = indices
                        .iter()
                        .zip(weights.iter())
                        .filter_map(|(&i, &w)| {
                            surface
                                .bones()
                                .get(i as usize)
                                .filter(|_| w > 0.001)
                                .map(|&h| (h, w))
                        })
                        .collect();
                    if !bones.iter().any(|&(h, w)| {
                        w > 0.05
                            && (h == forearm
                                || first && matches!(g[h].name(), "UpperArm.L" | "Hand.L"))
                    }) {
                        continue;
                    }
                    let anchor = Self {
                        root,
                        position,
                        normal,
                        bones,
                    };
                    let (p, n) = anchor.world(g);
                    if !first && n.dot(&outward) < 0.2 {
                        continue;
                    }
                    if let Some(m) = view {
                        let v = m.transform_point(&p.into()).coords;
                        let normal = m.transform_vector(&n);
                        if normal.dot(&-v.normalize()) < 0.15 {
                            continue;
                        }
                    }
                    let outside = view.is_some_and(|m| {
                        let v = m.transform_point(&p.into());
                        v.z <= 0.0 || v.x.abs() > v.z * 1.0 || v.y.abs() > v.z * 0.7
                    });
                    let d = (p - target).norm_squared() + if outside { 100.0 } else { 0.0 };
                    if d < score {
                        score = d;
                        best = Some(anchor);
                    }
                }
            }
        }
        best
    }
}

fn polygon(points: Vec<Vector3<f32>>, color: [u8; 3]) -> Surface {
    let mut vertices = vec![StaticVertex::from_pos_uv(
        Vector3::new(0.0, 0.0, points[0].z),
        Vector2::zeros(),
    )];
    vertices.extend(
        points
            .iter()
            .map(|v| StaticVertex::from_pos_uv(*v, Vector2::zeros())),
    );
    let triangles = (0..points.len() as u32)
        .map(|i| TriangleDefinition([0, i + 1, (i + 1) % points.len() as u32 + 1]))
        .collect();
    let mut data = SurfaceData::new(
        VertexBuffer::new(vertices.len(), vertices).unwrap(),
        TriangleBuffer::new(triangles),
    );
    data.calculate_normals().unwrap();
    SurfaceBuilder::new(SurfaceResource::new_embedded(data))
        .with_material(material(color))
        .build()
}
fn sticker() -> Vec<Surface> {
    let mut parts: Vec<_> = [(1.0, 0.0, [255, 255, 255]), (0.83, 0.002, [171, 115, 246])]
        .into_iter()
        .map(|(size, z, color)| {
            polygon(
                (0..10)
                    .map(|i| {
                        let a = std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                        let r = if i % 2 == 0 { 0.10 } else { 0.052 } * size;
                        Vector3::new(a.cos() * r, a.sin() * r, z)
                    })
                    .collect(),
                color,
            )
        })
        .collect();
    // A yellow heart in the center makes it read as a sticker rather than a floating buff orb.
    parts.push(polygon(
        (0..32)
            .map(|i| {
                let t = i as f32 * std::f32::consts::TAU / 32.0;
                Vector3::new(
                    0.0021 * 16.0 * t.sin().powi(3),
                    0.0021
                        * (13.0 * t.cos()
                            - 5.0 * (2.0 * t).cos()
                            - 2.0 * (3.0 * t).cos()
                            - (4.0 * t).cos()),
                    0.004,
                )
            })
            .rev()
            .collect(),
        [255, 226, 120],
    ));
    parts
}
fn mesh(g: &mut Graph, name: &str, surfaces: Vec<Surface>, forward: bool) -> Handle<Node> {
    MeshBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_visibility(false)
            .with_cast_shadows(false),
    )
    .with_surfaces(surfaces)
    .with_render_path(if forward {
        RenderPath::Forward
    } else {
        RenderPath::Deferred
    })
    .build(g)
    .to_base()
}
#[derive(Debug)]
pub(super) struct Effects {
    pub stickers: [Handle<Node>; PLAYERS],
    pub view_sticker: Handle<Node>,
    pub flashes: [Handle<Node>; PLAYERS],
    pub trails: [Handle<Node>; PLAYERS],
    anchors: [std::cell::RefCell<Option<Anchor>>; PLAYERS + 1],
}
impl Effects {
    pub fn new(g: &mut Graph) -> Self {
        let surfaces = sticker();
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("renua_orb.shader")).expect("Picora staff glow"),
        );
        let glow = SurfaceBuilder::new(SurfaceResource::new_embedded(SurfaceData::make_sphere(
            20,
            20,
            1.0,
            &Matrix4::identity(),
        )))
        .with_material(MaterialResource::new(Material::from_shader(shader)))
        .build();
        Self {
            stickers: std::array::from_fn(|_| {
                mesh(g, "Picora sleeve sticker", surfaces.clone(), false)
            }),
            view_sticker: mesh(g, "Picora first-person sleeve sticker", surfaces, false),
            flashes: std::array::from_fn(|_| {
                mesh(g, "Picora staff flash", vec![glow.clone()], true)
            }),
            trails: std::array::from_fn(|_| {
                mesh(g, "Picora staff spell trail", vec![glow.clone()], true)
            }),
            anchors: std::array::from_fn(|_| Default::default()),
        }
    }
    pub fn begin(&self, g: &mut Graph) {
        g[self.view_sticker].set_visible(false);
    }
    pub fn anchored(&self, slot: usize, first: bool) -> bool {
        self.anchors[if first { PLAYERS } else { slot }]
            .borrow()
            .is_some()
    }
    pub fn sleeve(
        &self,
        g: &mut Graph,
        slot: usize,
        first: bool,
        root: Handle<Node>,
        forearm: Handle<Node>,
        elbow: Vector3<f32>,
        wrist: Vector3<f32>,
        outward: Vector3<f32>,
        scale: f32,
        show: bool,
    ) {
        let node = if first {
            self.view_sticker
        } else {
            self.stickers[slot]
        };
        g[node].set_visible(show);
        if !show {
            return;
        }
        let along = (wrist - elbow).try_normalize(0.001).unwrap_or(Vector3::y());
        let normal = (outward - along * outward.dot(&along))
            .try_normalize(0.001)
            .unwrap_or(Vector3::z());
        let target = elbow.lerp(&wrist, if first { 0.96 } else { 0.68 }) + normal * (0.11 * scale);
        let mut anchor = self.anchors[if first { PLAYERS } else { slot }].borrow_mut();
        if anchor.as_ref().is_none_or(|a| a.root != root) {
            *anchor = Anchor::find(g, root, forearm, target, normal, first);
        }
        let (position, normal) = anchor.as_ref().map_or((target, normal), |a| a.world(g));
        let position = position + normal * (0.005 * scale);
        g[node]
            .local_transform_mut()
            .set_position(position)
            .set_rotation(UnitQuaternion::face_towards(&normal, &-along))
            .set_scale(Vector3::repeat(scale));
        g.update_hierarchical_data_for_descendants(node);
    }
    pub fn cast(&self, g: &mut Graph, slot: usize, pose: Pose, weapon: Handle<Node>, target: Option<Vector3<f32>>, show: bool) {
        use trickcal_shooting_balddagu::motion::Motion;
        let release = match pose.motion {
            Motion::Attack => trickcal_shooting_balddagu::attack::seconds(5) * 18.0 / 47.0,
            Motion::PicoraSkill => p::STICKER_RELEASE,
            Motion::PicoraUltimate => p::ULTIMATE_RELEASE,
            _ => f32::INFINITY,
        };
        let t = pose.age - release;
        let duration = if pose.motion == Motion::Attack { 0.12 } else { 0.34 };
        let show = show && (0.0..duration).contains(&t);
        let node = self.flashes[slot];
        let trail = self.trails[slot];
        g[node].set_visible(show);
        g[trail].set_visible(false);
        if show {
            let position = weapons::picora_muzzle(g, weapon);
            let size = 0.07 + 0.13 * (t / duration * std::f32::consts::PI).sin();
            g[node]
                .local_transform_mut()
                .set_position(position)
                .set_scale(Vector3::repeat(size));
            g.update_hierarchical_data_for_descendants(node);
            // E/Q resolve instantly on the server. A brief ray connects the
            // casting staff to the affected ally without delaying the buff.
            if let Some(target) = target.filter(|_| pose.motion != Motion::Attack && t < 0.24) {
                let delta = target - position;
                if let Some(direction) = delta.try_normalize(0.001) {
                    let radius = if pose.motion == Motion::PicoraUltimate { 0.06 } else { 0.04 }
                        * (1.0 - t / 0.24);
                    g[trail].set_visible(true);
                    g[trail].local_transform_mut()
                        .set_position((position + target) * 0.5)
                        .set_rotation(UnitQuaternion::rotation_between(&Vector3::y(), &direction).unwrap_or_default())
                        .set_scale(Vector3::new(radius, delta.norm() * 0.5, radius));
                    g.update_hierarchical_data_for_descendants(trail);
                }
            }
        }
    }
}
