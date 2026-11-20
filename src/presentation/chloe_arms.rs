//! Chloe's extended carry sleeves are used only while supporting Sebastian.
use super::*;
use fyrox::core::algebra::Matrix3;
use fyrox::scene::mesh::{
    buffer::{VertexAttributeUsage as Usage, VertexReadTrait, VertexWriteTrait},
    surface::Surface,
    Mesh,
};
use trickcal_shooting_balddagu::motion::{Motion, Pose};

// Measured from the preserved Meshy source, using the importer's 1.85 m scale.
pub(super) const NORMAL_LENGTHS: [[f32; 2]; 2] =
    [[0.15261574, 0.16932373], [0.15261567, 0.1638375]];

pub(super) fn carrying(pose: Pose) -> bool {
    match pose.base() {
        (
            Motion::ChloeRideIdle
            | Motion::ChloeRideMove
            | Motion::ChloeRideAttack
            | Motion::ChloeRoar,
            _,
        ) => false,
        // Match the exact carry() branches in bake_chloe_skills.py.
        (Motion::ChloeMount, age) => smooth((age - 0.25) / 0.8) < 0.10,
        (Motion::ChloeDismount, age) => 1.0 - smooth((age - 0.30) / 0.95) < 0.10,
        _ => true,
    }
}
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Debug)]
struct Sleeve {
    node: Handle<Node>,
    index: usize,
    side: usize,
    variants: [Surface; 2],
}

#[derive(Debug)]
pub(super) struct Rig {
    joints: [[Handle<Node>; 3]; 2],
    sleeves: Vec<Sleeve>,
    short: std::cell::Cell<[bool; 2]>,
}

impl Rig {
    pub fn new(g: &Graph, root: Handle<Node>) -> Self {
        let joints = ["L", "R"].map(|side| {
            ["UpperArm", "Forearm", "Hand"].map(|part| {
                g.traverse_handle_iter(root)
                    .find(|h| g[*h].name() == format!("{part}.{side}"))
                    .unwrap()
            })
        });
        let mut sleeves = Vec::new();
        for (side, label) in ["L", "R"].into_iter().enumerate() {
            let points = joints[side].map(|h| {
                g[h].inv_bind_pose_transform()
                    .try_inverse()
                    .unwrap()
                    .transform_point(&Vector3::zeros().into())
                    .coords
            });
            let frames = [0, 1].map(|i| {
                let offset = points[i + 1] - points[i];
                (
                    joints[side][i],
                    points[i],
                    offset.normalize(),
                    NORMAL_LENGTHS[side][i] / offset.norm(),
                )
            });
            for node in g
                .traverse_handle_iter(root)
                .filter(|h| g[*h].name() == format!("chloe_RefinedSleeve.{label}"))
            {
                let mesh = g[node].cast::<Mesh>().unwrap();
                for (index, surface) in mesh.surfaces().iter().enumerate() {
                    let mut data = surface.data_ref().data_ref().clone();
                    {
                        let mut buffer = data.vertex_buffer.modify();
                        for mut vertex in buffer.iter_mut() {
                            let indices = vertex.read_4_u8(Usage::BoneIndices).unwrap();
                            let weights = vertex.read_4_f32(Usage::BoneWeight).unwrap();
                            let p = vertex.read_3_f32(Usage::Position).unwrap();
                            let mut offset = Vector3::zeros();
                            let mut deformation = Matrix3::identity();
                            for (joint, origin, axis, ratio) in frames {
                                let weight: f32 = indices
                                    .iter()
                                    .zip(weights.iter())
                                    .filter(|(i, _)| surface.bones()[**i as usize] == joint)
                                    .map(|(_, w)| *w)
                                    .sum();
                                let change = axis * axis.transpose() * ((ratio - 1.0) * weight);
                                offset += change * (p - origin);
                                deformation += change;
                            }
                            vertex.write_3_f32(Usage::Position, p + offset).unwrap();
                            let n = vertex.read_3_f32(Usage::Normal).unwrap();
                            vertex
                                .write_3_f32(
                                    Usage::Normal,
                                    (deformation.try_inverse().unwrap().transpose() * n)
                                        .normalize(),
                                )
                                .unwrap();
                            if let Ok(t) = vertex.read_4_f32(Usage::Tangent) {
                                vertex
                                    .write_4_f32(
                                        Usage::Tangent,
                                        (deformation * t.xyz()).normalize().push(t.w),
                                    )
                                    .unwrap();
                            }
                        }
                    }
                    let short = SurfaceBuilder::new(SurfaceResource::new_embedded(data))
                        .with_material(surface.material().clone())
                        .with_bones(surface.bones().to_vec())
                        .build();
                    sleeves.push(Sleeve {
                        node,
                        index,
                        side,
                        variants: [surface.clone(), short],
                    });
                }
            }
        }
        Self {
            joints,
            sleeves,
            short: std::cell::Cell::new([false; 2]),
        }
    }

    pub fn apply(&self, g: &mut Graph, short: [bool; 2]) {
        for sleeve in &self.sleeves {
            let desired = &sleeve.variants[usize::from(short[sleeve.side])];
            let mesh = g[sleeve.node].cast_mut::<Mesh>().unwrap();
            if mesh.surfaces()[sleeve.index].data_ref() != desired.data_ref() {
                mesh.surfaces_mut()[sleeve.index] = desired.clone();
            }
        }
        for (side, joints) in self.joints.iter().enumerate().filter(|(i, _)| short[*i]) {
            let animated_wrist = g[joints[2]].global_position();
            let palm_rotation = super::skeleton::world_rotation(g, joints[2]);
            // Remove carry scale before any gesture/IK. Rotations and shoulder
            // placement come from the current animation; hands keep their size.
            for joint in joints {
                g[*joint]
                    .local_transform_mut()
                    .set_scale(Vector3::repeat(1.0));
            }
            g.update_hierarchical_data_for_descendants(joints[0]);
            let points = joints.map(|h| g[h].global_position());
            let bind = g[joints[0]].global_transform() * g[joints[0]].inv_bind_pose_transform();
            let scale = bind.transform_vector(&Vector3::x()).norm();
            let directions = [0, 1].map(|i| (points[i + 1] - points[i]).normalize());
            let elbow = points[0] + directions[0] * NORMAL_LENGTHS[side][0] * scale;
            let wrist = elbow + directions[1] * NORMAL_LENGTHS[side][1] * scale;
            for (joint, point) in [(joints[1], elbow), (joints[2], wrist)] {
                let local = g[g[joint].parent()]
                    .global_transform()
                    .try_inverse()
                    .unwrap()
                    .transform_point(&point.into())
                    .coords;
                g[joint].local_transform_mut().set_position(local);
                g.update_hierarchical_data_for_descendants(joint);
            }
            // Re-solve the authored action with the normal reach. Merely
            // shortening an already bent carry arm leaves ear-covering hands
            // at chest height instead of performing the intended gesture.
            let delta = animated_wrist - points[0];
            let [a, b] = NORMAL_LENGTHS[side].map(|length| length * scale);
            if let Some(direction) = delta.try_normalize(1.0e-6) {
                let target = points[0]
                    + direction * delta.norm().clamp((a - b).abs() + 0.0001, a + b - 0.0001);
                super::hand_grip::solve_arm(
                    g,
                    joints[0],
                    joints[1],
                    joints[2],
                    target,
                    points[1] - points[0],
                );
                super::skeleton::set_global_rotation(g, joints[2], palm_rotation);
            }
        }
        self.short.set(short);
    }

    pub fn review(&self, g: &Graph) {
        for (side, joints) in self
            .joints
            .iter()
            .enumerate()
            .filter(|(i, _)| self.short.get()[*i])
        {
            let bind = g[joints[0]].global_transform() * g[joints[0]].inv_bind_pose_transform();
            let scale = bind.transform_vector(&Vector3::x()).norm();
            for i in 0..2 {
                let actual =
                    (g[joints[i + 1]].global_position() - g[joints[i]].global_position()).norm();
                assert!(
                    (actual - NORMAL_LENGTHS[side][i] * scale).abs() < 0.001,
                    "Chloe normal arm length {actual}"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restoring_normal_length_keeps_the_action_target_and_palm_size() {
        let mut graph = Graph::new();
        let upper: Handle<Node> = PivotBuilder::new(BaseBuilder::new())
            .build(&mut graph)
            .to_base();
        let fore: Handle<Node> = PivotBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 0.5, 0.0))
                    .build(),
            ),
        )
        .build(&mut graph)
        .to_base();
        let hand: Handle<Node> = PivotBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.2, -0.3, 0.0))
                    .build(),
            ),
        )
        .build(&mut graph)
        .to_base();
        graph.link_nodes(fore, upper);
        graph.link_nodes(hand, fore);
        graph.update_hierarchical_data_for_descendants(upper);
        let target = graph[hand].global_position();
        let rig = Rig {
            joints: [[upper, fore, hand]; 2],
            sleeves: vec![],
            short: std::cell::Cell::new([false; 2]),
        };
        for _ in 0..3 {
            rig.apply(&mut graph, [true, false]);
            rig.review(&graph);
            assert!((graph[hand].global_position() - target).norm() < 0.0001);
            assert_eq!(
                **graph[hand].local_transform().scale(),
                Vector3::repeat(1.0)
            );
        }
    }
    #[test]
    fn carry_length_ends_as_soon_as_support_ends() {
        let pose = |motion, age| Pose {
            motion,
            age,
            ..Default::default()
        };
        assert!(carrying(pose(Motion::Idle, 0.0)));
        assert!(carrying(pose(Motion::ChloeMount, 0.40)));
        assert!(!carrying(pose(Motion::ChloeMount, 0.41)));
        assert!(!carrying(pose(Motion::ChloeDismount, 1.06)));
        assert!(carrying(pose(Motion::ChloeDismount, 1.07)));
        for motion in [
            Motion::ChloeRideIdle,
            Motion::ChloeRideMove,
            Motion::ChloeRideAttack,
            Motion::ChloeRoar,
        ] {
            assert!(!carrying(pose(motion, 0.0)));
        }
    }
}
