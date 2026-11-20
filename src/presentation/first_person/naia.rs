//! Short, closed cuffs for Naia's first-person E. The full-body FBX stays untouched.
use super::*;
use fyrox::{
    core::{algebra::Vector4, math::TriangleDefinition},
    scene::camera::Camera,
    scene::mesh::{buffer::VertexBuffer, vertex::AnimatedVertex},
};

#[derive(Debug)]
pub(super) struct Cuff {
    sleeve: Handle<Node>,
    short: Handle<Node>,
    forearm: Handle<Node>,
    back_rim: Vec<Vector3<f32>>,
}

impl Cuff {
    pub fn show(&self, g: &mut Graph, short: bool) {
        g[self.sleeve].set_visibility(!short);
        g[self.short].set_visibility(short);
    }

    pub fn verify(&self, g: &Graph, camera: Handle<Node>, short: bool) {
        assert_eq!(g[self.sleeve].visibility(), !short);
        assert_eq!(g[self.short].visibility(), short);
        if short {
            let skin =
                g[self.forearm].global_transform() * g[self.forearm].inv_bind_pose_transform();
            let projection = g[camera].cast::<Camera>().unwrap().view_projection_matrix();
            let rim: Vec<_> = self
                .back_rim
                .iter()
                .map(|p| projection * skin * p.push(1.0))
                .collect();
            // The entire capped end must be beyond one clip plane, not merely its center.
            let outside = (0..6).any(|plane| {
                rim.iter().all(|p| match plane {
                    0 => p.x < -p.w,
                    1 => p.x > p.w,
                    2 => p.y < -p.w,
                    3 => p.y > p.w,
                    4 => p.z < -p.w,
                    _ => p.z > p.w,
                })
            });
            assert!(
                outside,
                "Naia sleeve cut remains in view: {:?}",
                rim.iter()
                    .map(|p| [p.x / p.w, p.y / p.w])
                    .collect::<Vec<_>>()
            );
        }
    }
}

pub(super) fn cuffs(
    g: &mut Graph,
    model: Handle<Node>,
    forearm: Handle<Node>,
    hand: Handle<Node>,
    fingers: &[Handle<Node>],
) -> Vec<Cuff> {
    // Surface vertices and inverse bind poses share the FBX's bind-space units.
    let bind_position = |node: Handle<Node>| {
        g[node]
            .inv_bind_pose_transform()
            .try_inverse()
            .expect("Naia bind pose")
            .transform_point(&Vector3::zeros().into())
            .coords
    };
    let wrist = bind_position(hand);
    let middle = fingers
        .iter()
        .find(|h| g[**h].name().starts_with("Middle1."))
        .unwrap();
    let index = fingers
        .iter()
        .find(|h| g[**h].name().starts_with("Index1."))
        .unwrap();
    let pinky = fingers
        .iter()
        .find(|h| g[**h].name().starts_with("Pinky1."))
        .unwrap();
    let direction = bind_position(*middle) - wrist;
    let size = direction.norm();
    let forward = direction.normalize();
    let across = bind_position(*index) - bind_position(*pinky);
    let across = (across - forward * across.dot(&forward)).normalize();
    let normal = forward.cross(&across).normalize();
    let sleeves: Vec<_> = g
        .traverse_handle_iter(model)
        .filter(|h| {
            g[*h].name().contains("_RefinedViewSleeve.")
                && g[*h]
                    .cast::<Mesh>()
                    .is_some_and(|m| !m.surfaces().is_empty())
        })
        .collect();
    assert_eq!(sleeves.len(), 1, "one Naia sleeve per anatomical hand");
    sleeves
        .into_iter()
        .map(|sleeve| {
            let materials: Vec<_> = g[sleeve]
                .cast::<Mesh>()
                .unwrap()
                .surfaces()
                .iter()
                .map(|s| {
                    // Use an interior cloth/trim swatch from the original atlas.
                    // A new cylindrical UV unwrap would cross unrelated hand islands.
                    let data = s.data_ref().data_ref();
                    let triangle = data
                        .geometry_buffer
                        .iter()
                        .max_by(|a, b| {
                            let area = |triangle: &TriangleDefinition| {
                                let p = triangle.0.map(|i| {
                                    data.vertex_buffer
                                        .get(i as usize)
                                        .unwrap()
                                        .read_3_f32(VertexAttributeUsage::Position)
                                        .unwrap()
                                });
                                (p[1] - p[0]).cross(&(p[2] - p[0])).norm_squared()
                            };
                            area(a).total_cmp(&area(b))
                        })
                        .unwrap();
                    let uv = triangle
                        .0
                        .iter()
                        .map(|i| {
                            data.vertex_buffer
                                .get(*i as usize)
                                .unwrap()
                                .read_2_f32(VertexAttributeUsage::TexCoord0)
                                .unwrap()
                        })
                        .sum::<Vector2<f32>>()
                        / 3.0;
                    (s.material().clone(), uv)
                })
                .collect();
            assert_eq!(materials.len(), 2, "Naia cloth and cuff trim");
            // A broad, gently tapered sleeve ends just behind the wrist. Both ends
            // are capped, so the floating viewmodel never exposes an open arm cut.
            let rings = [
                (-1.25, 0.68),
                (-1.15, 0.72),
                (-0.70, 0.59),
                (-0.54, 0.57),
                (-0.43, 0.55),
                (-0.36, 0.50),
            ];
            const SIDES: usize = 24;
            let mut vertices = Vec::new();
            let mut triangles = [Vec::new(), Vec::new()];
            for (i, (along, radius)) in rings.iter().enumerate() {
                for j in 0..SIDES {
                    let angle = j as f32 * std::f32::consts::TAU / SIDES as f32;
                    let radial = across * angle.cos() + normal * angle.sin();
                    vertices.push(AnimatedVertex {
                        position: wrist + (forward * *along + radial * *radius * ARM_THICKNESS) * size,
                        normal: radial,
                        tex_coord: Vector2::new(j as f32 / SIDES as f32, i as f32 / 5.0),
                        tangent: Vector4::new(forward.x, forward.y, forward.z, 1.0),
                        bone_weights: [1.0, 0.0, 0.0, 0.0],
                        bone_indices: [0; 4],
                    });
                    if i > 0 {
                        let a = ((i - 1) * SIDES + j) as u32;
                        let b = ((i - 1) * SIDES + (j + 1) % SIDES) as u32;
                        let c = (i * SIDES + j) as u32;
                        let d = (i * SIDES + (j + 1) % SIDES) as u32;
                        let material = usize::from(i >= 4);
                        triangles[material]
                            .extend([TriangleDefinition([a, b, c]), TriangleDefinition([b, d, c])]);
                    }
                }
            }
            for (material, ring, sign) in [(0, 0, -1.0), (1, rings.len() - 1, 1.0)] {
                let start = vertices.len() as u32;
                for j in 0..SIDES {
                    let mut vertex = vertices[ring * SIDES + j];
                    vertex.normal = forward * sign;
                    vertices.push(vertex);
                }
                for j in 1..SIDES - 1 {
                    let mut indices = [start, start + j as u32, start + j as u32 + 1];
                    if sign < 0.0 {
                        indices.swap(1, 2);
                    }
                    triangles[material].push(TriangleDefinition(indices));
                }
            }
            let back_rim = vertices[..SIDES].iter().map(|v| v.position).collect();
            let surfaces = triangles
                .into_iter()
                .zip(materials)
                .map(|(triangles, (material, uv))| {
                    let vertices: Vec<_> = vertices
                        .iter()
                        .map(|v| AnimatedVertex {
                            tex_coord: uv,
                            ..*v
                        })
                        .collect();
                    let data = SurfaceData::new(
                        VertexBuffer::new(vertices.len(), vertices).unwrap(),
                        TriangleBuffer::new(triangles),
                    );
                    SurfaceBuilder::new(SurfaceResource::new_embedded(data))
                        .with_material(material)
                        .with_bones(vec![forearm, hand])
                        .build()
                })
                .collect();
            let transform = g[sleeve].local_transform().clone();
            let short: Handle<Node> = MeshBuilder::new(
                BaseBuilder::new()
                    .with_name("Naia E short cuff")
                    .with_visibility(false)
                    // The elbow can be behind the camera while the cuff is in view.
                    // Imported bind-space bounds do not describe this camera-mounted skin.
                    .with_frustum_culling(false)
                    .with_local_transform(transform),
            )
            .with_surfaces(surfaces)
            .build(g)
            .to_base();
            g[short].set_cast_shadows(false);
            g.link_nodes(short, g[sleeve].parent());
            Cuff {
                sleeve,
                short,
                forearm,
                back_rim,
            }
        })
        .collect()
}
