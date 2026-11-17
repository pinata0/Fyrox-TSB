//! Pose imported bones without losing the FBX pre/post rotations that define their local axes.
use super::*;

/// The shipped rigs use coordinate-side names: .L is anatomical right (-X),
/// .R is anatomical left (+X), with the character facing +Z in Fyrox.
/// Tripo's original RightHand fingers likewise belong to Hand.L.
pub(super) fn arm_bone_names(left: bool) -> [&'static str; 3] {
    if left {
        ["UpperArm.R", "Forearm.R", "Hand.R"]
    } else {
        ["UpperArm.L", "Forearm.L", "Hand.L"]
    }
}

pub(super) fn set_global_rotation(
    g: &mut Graph,
    joint: Handle<Node>,
    rotation: UnitQuaternion<f32>,
) {
    let parent = world_rotation(g, g[joint].parent());
    let transform = g[joint].local_transform();
    let local = transform.pre_rotation().inverse()
        * parent.inverse()
        * rotation
        * **transform.post_rotation();
    g[joint].local_transform_mut().set_rotation(local);
    g.update_hierarchical_data_for_descendants(joint);
}

/// The scale-free matrix is already orthonormal. Direct quaternion conversion
/// remains stable near 180 degrees, where iterative polar fitting can stall.
pub(super) fn world_rotation(g: &Graph, node: Handle<Node>) -> UnitQuaternion<f32> {
    UnitQuaternion::from_rotation_matrix(&fyrox::core::algebra::Rotation3::from_matrix_unchecked(
        g.global_transform_no_scale(node)
            .fixed_view::<3, 3>(0, 0)
            .into_owned(),
    ))
}

pub(super) fn aim_joint(
    g: &mut Graph,
    joint: Handle<Node>,
    end: Handle<Node>,
    target: Vector3<f32>,
) {
    let direction = g[end].global_position() - g[joint].global_position();
    if let Some(delta) = UnitQuaternion::rotation_between(&direction, &target) {
        set_global_rotation(g, joint, delta * world_rotation(g, joint));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_joint_preserves_world_orientation_and_connected_wrist() {
        let mut g = Graph::new();
        let parent: Handle<Node> = PivotBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_rotation(UnitQuaternion::from_euler_angles(0.3, -0.7, 0.2))
                    .build(),
            ),
        )
        .build(&mut g)
        .to_base();
        let joint: Handle<Node> = PivotBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_pre_rotation(UnitQuaternion::from_euler_angles(1.1, 0.6, -0.4))
                    .with_post_rotation(UnitQuaternion::from_euler_angles(-0.2, 0.5, 0.8))
                    .build(),
            ),
        )
        .build(&mut g)
        .to_base();
        let wrist: Handle<Node> = PivotBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 0.2, 0.0))
                    .build(),
            ),
        )
        .build(&mut g)
        .to_base();
        g.link_nodes(joint, parent);
        g.link_nodes(wrist, joint);
        g.update_hierarchical_data_for_descendants(parent);
        let target = UnitQuaternion::from_euler_angles(-0.5, 0.2, 1.4);
        set_global_rotation(&mut g, joint, target);
        assert!(g.global_rotation(joint).angle_to(&target) < 1e-5);
        let expected = target * Vector3::new(0.0, 0.2, 0.0);
        assert!((g[wrist].global_position() - g[joint].global_position() - expected).norm() < 1e-5);

        g[parent]
            .local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                std::f32::consts::PI,
            ));
        g.update_hierarchical_data_for_descendants(parent);
        set_global_rotation(&mut g, joint, target);
        assert!(world_rotation(&g, joint).angle_to(&target) < 1e-5);
        assert!((g[wrist].global_position() - g[joint].global_position() - expected).norm() < 1e-5);

        // Reposition the upper arm, then restore the forearm without twisting its sleeve.
        let saved = g.global_rotation(joint);
        g[parent]
            .local_transform_mut()
            .set_rotation(UnitQuaternion::from_euler_angles(-0.8, 1.0, -1.2));
        g.update_hierarchical_data_for_descendants(parent);
        set_global_rotation(&mut g, joint, saved);
        assert!(g.global_rotation(joint).angle_to(&saved) < 1e-5);
        assert!((g[wrist].global_position() - g[joint].global_position() - expected).norm() < 1e-5);
    }
}
