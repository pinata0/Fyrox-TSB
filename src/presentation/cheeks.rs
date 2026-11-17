//! Preview-private soft cheek meshes and retargeted standing reactions.
use super::*;
use fyrox::scene::{
    camera::Camera,
    mesh::{
        buffer::{VertexAttributeUsage as Usage, VertexReadTrait, VertexWriteTrait},
        Mesh,
    },
};
use trickcal_shooting_balddagu::cheeks::{Interaction, Phase};

#[derive(Debug)]
struct Vertex {
    index: usize,
    original: Vector3<f32>,
    normal: Vector3<f32>,
    control: super::cheek_masks::Control,
}
#[derive(Debug)]
struct FaceSurface {
    resource: SurfaceResource,
    vertices: Vec<Vertex>,
    to_vertex: Matrix4<f32>,
}
#[derive(Debug)]
pub(super) struct Cheeks {
    pub interaction: Interaction,
    pub id: u8,
    bones: [Handle<Node>; 6],
    head_to_model: Matrix4<f32>,
    surfaces: Vec<FaceSurface>,
    pub centers: [Vector2<f32>; 2],
    pub radius: f32,
    screen_sign: f32,
    pub changed_vertices: usize,
    dirty: bool,
    face: FaceLayout,
    apex: [Vector3<f32>; 2],
    hit_radius: f32,
    hands: [Handle<Node>; 2],
    arm_sides: [f32; 2],
    tears: [Handle<Node>; 2],
}
#[derive(Debug, Clone, Copy)]
struct FaceLayout {
    cheek: Vector3<f32>,
    eye_y: f32,
    eye_x: f32,
}
impl FaceLayout {
    fn for_id(id: u8) -> Self {
        static LAYOUTS: std::sync::LazyLock<[[f32; 6]; 15]> = std::sync::LazyLock::new(|| {
            serde_json::from_str(include_str!("../../assets/characters/meshy-faces.json"))
                .expect("validated Meshy face landmarks")
        });
        let [x, y, z, eye_y, eye_x, _] = LAYOUTS[id as usize];
        Self {
            cheek: Vector3::new(x, y, z),
            eye_y,
            eye_x,
        }
    }
}
impl Cheeks {
    pub fn new(g: &mut Graph, visual: &characters::Visual, stage: Handle<Node>) -> Self {
        g.update_hierarchical_data_for_descendants(stage);
        let names = [
            "Head",
            "Torso",
            "UpperArm.L",
            "Forearm.L",
            "UpperArm.R",
            "Forearm.R",
        ];
        let bones = names.map(|name| {
            g.traverse_handle_iter(visual.root)
                .find(|h| g[*h].name() == name)
                .expect("character reaction bone")
        });
        let head = bones[0];
        let hands = ["Hand.L", "Hand.R"].map(|name| {
            g.traverse_handle_iter(visual.root)
                .find(|h| g[*h].name() == name)
                .unwrap()
        });
        let arm_sides = [bones[2], bones[4]]
            .map(|h| (g[h].global_position().x - g[head].global_position().x).signum());
        let to_model =
            g[stage].global_transform().try_inverse().unwrap() * g[head].global_transform();
        let inv_bind = g[head].inv_bind_pose_transform();
        let bind = to_model * inv_bind;
        let to_vertex = bind.try_inverse().unwrap();
        let nodes: Vec<_> = g.traverse_handle_iter(visual.root).collect();
        let face = FaceLayout::for_id(visual.id);
        let mask = super::cheek_masks::get(visual.id);
        let apex = mask.centers.map(|p| bind.transform_point(&p.into()).coords);
        let mut surfaces = Vec::new();
        let mut total = 0;
        for h in nodes {
            if !g[h].name().ends_with("_SkinnedMesh") {
                continue;
            }
            let Some(mesh) = g[h].cast::<Mesh>() else {
                continue;
            };
            let mut replacements = Vec::new();
            for (surface_index, surface) in mesh.surfaces().iter().enumerate() {
                if surface_index != 0 {
                    continue;
                }
                let data = surface.data_ref().data_ref();
                let mut vertices = Vec::new();
                for (index, v) in data.vertex_buffer.iter().enumerate() {
                    let (Ok(p), Ok(normal)) =
                        (v.read_3_f32(Usage::Position), v.read_3_f32(Usage::Normal))
                    else {
                        continue;
                    };
                    let control = *mask
                        .vertices
                        .get(&super::cheek_masks::key(p))
                        .expect("every FBX vertex must have an authored cheek control");
                    vertices.push(Vertex {
                        index,
                        original: p,
                        normal,
                        control,
                    });
                }
                if vertices.is_empty() {
                    continue;
                }
                total += vertices.len();
                // Never mutate the model resource shared by players and first-person arms.
                let resource = SurfaceResource::new_embedded(data.clone());
                replacements.push((
                    surface_index,
                    SurfaceBuilder::new(resource.clone())
                        .with_material(surface.material().clone())
                        .with_bones(surface.bones().to_vec())
                        .build(),
                ));
                surfaces.push(FaceSurface {
                    resource,
                    vertices,
                    to_vertex,
                });
            }
            for (index, surface) in replacements {
                g[h].cast_mut::<Mesh>().unwrap().surfaces_mut()[index] = surface;
            }
        }
        println!(
            "CHEEK_MESH_OK: {} {total} private face vertices",
            characters::SLUGS[visual.id as usize]
        );
        let shapes = Shapes::new();
        let head_scale = to_model.transform_vector(&Vector3::x()).norm();
        let tears = std::array::from_fn(|i| {
            let p = to_model
                .try_inverse()
                .unwrap()
                .transform_point(
                    &Vector3::new(
                        if i == 0 { -face.eye_x } else { face.eye_x },
                        face.eye_y - 0.07,
                        face.cheek.z + 0.085,
                    )
                    .into(),
                )
                .coords;
            let tear = shapes.solid(
                g,
                "Cheek reaction tear",
                V3::new(p.x, p.y, p.z),
                V3::new(0.018 / head_scale, 0.028 / head_scale, 0.01 / head_scale),
                [202, 241, 255],
                1,
            );
            g.link_nodes(tear, head);
            g[tear].set_visibility(false);
            g[tear].set_cast_shadows(false);
            tear
        });
        Self {
            interaction: Interaction::default(),
            id: visual.id,
            bones,
            head_to_model: to_model.try_inverse().unwrap(),
            surfaces,
            centers: [Vector2::zeros(); 2],
            radius: 0.0,
            screen_sign: 1.0,
            changed_vertices: 0,
            dirty: false,
            face,
            apex,
            hit_radius: mask.hit_radius,
            hands,
            arm_sides,
            tears,
        }
    }
    pub fn press(&mut self, cursor: Vector2<f32>) -> bool {
        if self.radius <= 0.0 {
            return false;
        }
        let hit = (0..2)
            .filter(|i| (self.centers[*i] - cursor).norm() <= self.radius)
            .min_by(|a, b| {
                (self.centers[*a] - cursor)
                    .norm_squared()
                    .total_cmp(&(self.centers[*b] - cursor).norm_squared())
            });
        if let Some(i) = hit {
            self.interaction.begin(
                if i == 0 { -1.0 } else { 1.0 },
                [cursor.x, cursor.y],
                self.radius / self.hit_radius,
            );
            true
        } else {
            false
        }
    }
    pub fn drag(&mut self, cursor: Vector2<f32>) {
        self.interaction.drag([cursor.x, cursor.y]);
    }
    pub fn cancel(&mut self) {
        self.interaction.cancel();
    }
    pub fn apply(&mut self, g: &mut Graph, camera: Handle<Node>, size: Vector2<f32>) {
        let state = &self.interaction;
        let blend = state.blend(self.id);
        let head_before = g[self.bones[0]].global_transform();
        let torso_before = g[self.bones[1]].global_transform();
        if blend > 0.0 {
            let pose = state.pose(self.id);
            // Arms may react, but the cheek gesture never rotates the head or torso.
            let base = self.bones.map(|h| super::skeleton::world_rotation(g, h));
            let mirror = if state.side > 0.0 { -1.0 } else { 1.0 };
            for i in [2, 3, 4, 5] {
                let source = if mirror < 0.0 && i >= 2 {
                    if i < 4 {
                        i + 2
                    } else {
                        i - 2
                    }
                } else {
                    i
                };
                let limit = 1.65;
                let angle = (pose[source] * mirror).clamp(-limit, limit) * blend;
                super::skeleton::set_global_rotation(
                    g,
                    self.bones[i],
                    UnitQuaternion::from_axis_angle(&Vector3::z_axis(), angle) * base[i],
                );
            }
            let targets = state.hands(self.id);
            let head_matrix = g[self.bones[0]].global_transform() * self.head_to_model;
            for arm in 0..2 {
                let source = if self.arm_sides[arm] * mirror > 0.0 {
                    0
                } else {
                    1
                };
                let offset = Vector3::new(
                    targets[source * 2] * mirror * 2.0 * self.face.cheek.x,
                    targets[source * 2 + 1] * 2.0 * self.face.cheek.x,
                    0.0,
                );
                let model_target = Vector3::new(
                    offset.x,
                    self.face.cheek.y + offset.y,
                    self.face.cheek.z + 0.10,
                );
                let target = head_matrix.transform_point(&model_target.into()).coords;
                let hand = self.hands[arm];
                let target = g[hand].global_position().lerp(&target, blend);
                pose_arm(
                    g,
                    self.bones[2 + arm * 2],
                    self.bones[3 + arm * 2],
                    hand,
                    target,
                    self.arm_sides[arm],
                );
            }
        }
        let pull = state.stretch();
        debug_assert!((g[self.bones[0]].global_transform() - head_before).norm() < 1.0e-5);
        debug_assert!((g[self.bones[1]].global_transform() - torso_before).norm() < 1.0e-5);
        let face = state.face(self.id);
        for (i, tear) in self.tears.iter().enumerate() {
            let source = if state.side > 0.0 { i } else { 1 - i };
            g[*tear].set_visibility(blend > 0.05 && (face[3] as u8 & (1 << source)) != 0);
        }
        // Lift the pinched skin slightly off the face so its rounded pad stays
        // in front of adjacent hair instead of shearing sideways through it.
        let stretch = Vector3::new(
            pull[0] * self.screen_sign,
            pull[1],
            pull[0].hypot(pull[1]) * 0.55,
        );
        if blend > 0.0 || self.dirty {
            self.changed_vertices = 0;
            for surface in &self.surfaces {
                let mut data = surface.resource.data_ref();
                let mut buffer = data.vertex_buffer.modify();
                for v in &surface.vertices {
                    if v.control.weights == [0.0, 0.0] {
                        continue;
                    }
                    let side = usize::from(state.side > 0.0);
                    let weight = v.control.weights[side];
                    let delta = stretch * weight;
                    let p = v.original + surface.to_vertex.transform_vector(&delta);
                    if delta.norm() > 0.001 {
                        self.changed_vertices += 1;
                    }
                    let mut vertex = buffer.get_mut(v.index).unwrap();
                    vertex.write_3_f32(Usage::Position, p).unwrap();
                    // The baked gradient follows the actual skin surface and all feature pins.
                    let grad_vertex = v.control.gradients[side];
                    let disp = surface.to_vertex.transform_vector(&stretch);
                    let denom = 1.0 + grad_vertex.dot(&disp);
                    let n = if weight > 0.0 && denom.abs() > 0.15 {
                        (v.normal - grad_vertex * (disp.dot(&v.normal) / denom)).normalize()
                    } else {
                        v.normal
                    };
                    vertex.write_3_f32(Usage::Normal, n).unwrap();
                }
            }
            self.dirty = blend > 0.0;
        }
        let head = g[self.bones[0]].global_transform();
        let world = |p: Vector3<f32>| {
            head.transform_point(&self.head_to_model.transform_point(&p.into()))
                .coords
        };
        let cam = g[camera].cast::<Camera>().unwrap();
        let left = cam.project(world(self.apex[0]), size);
        let right = cam.project(world(self.apex[1]), size);
        if let (Some(l), Some(r)) = (left, right) {
            self.centers = [l, r];
            self.radius =
                ((r - l).norm() / (self.apex[1] - self.apex[0]).norm() * self.hit_radius).max(10.0);
            self.screen_sign = (r.x - l.x).signum();
        } else {
            self.radius = 0.0;
        }
    }
    pub fn held(&self) -> bool {
        self.interaction.phase == Phase::Held
    }

    pub fn review_protected(&self) {
        let mut count = 0;
        for surface in &self.surfaces {
            let data = surface.resource.data_ref();
            for v in surface
                .vertices
                .iter()
                .filter(|v| v.control.protected != 0 || v.control.weights == [0.0, 0.0])
            {
                let vertex = data.vertex_buffer.get(v.index).unwrap();
                assert_eq!(vertex.read_3_f32(Usage::Position).unwrap(), v.original);
                assert!((vertex.read_3_f32(Usage::Normal).unwrap() - v.normal).norm() < 0.00001);
                count += 1;
            }
        }
        assert!(
            count > 12,
            "{}: eyes, mouth, glasses and head must stay pinned",
            self.id
        );
    }
}

/// Two-bone IK preserves sleeve lengths while fitting the original 2D hand target.
fn pose_arm(
    g: &mut Graph,
    shoulder: Handle<Node>,
    elbow: Handle<Node>,
    hand: Handle<Node>,
    target: Vector3<f32>,
    side: f32,
) {
    let start = g[shoulder].global_position();
    let a = (g[elbow].global_position() - start).norm();
    let b = (g[hand].global_position() - g[elbow].global_position()).norm();
    let delta = target - start;
    if a < 0.001 || b < 0.001 || delta.norm() < 0.001 {
        return;
    }
    let distance = delta.norm().clamp((a - b).abs() + 0.001, a + b - 0.001);
    let direction = delta.normalize();
    let along = (a * a - b * b + distance * distance) / (2.0 * distance);
    let height = (a * a - along * along).max(0.0).sqrt();
    let pole = Vector3::new(side * 0.45, -0.5, 0.5);
    let pole = (pole - direction * pole.dot(&direction))
        .try_normalize(0.001)
        .unwrap_or(Vector3::z());
    let desired_elbow = start + direction * along + pole * height;
    super::skeleton::aim_joint(g, shoulder, elbow, desired_elbow - start);
    super::skeleton::aim_joint(
        g,
        elbow,
        hand,
        start + direction * distance - g[elbow].global_position(),
    );
}
