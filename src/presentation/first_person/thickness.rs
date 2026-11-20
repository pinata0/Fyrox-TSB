//! Thicken only the isolated view arm; keep its bone lengths and palm contact intact.
use super::*;
use fyrox::scene::mesh::buffer::VertexWriteTrait;

pub(super) fn widen(
    data: &mut SurfaceData,
    bones: &[Handle<Node>],
    graph: &Graph,
    joints: [Handle<Node>; 3],
    factor: f32,
) {
    let positions = joints.map(|joint| {
        graph[joint]
            .inv_bind_pose_transform()
            .try_inverse()
            .expect("arm bind pose must be invertible")
            .transform_point(&Vector3::zeros().into())
            .coords
    });
    let frames = [0, 1].map(|i| {
        let axis = (positions[i + 1] - positions[i]).normalize();
        (
            joints[i],
            positions[i],
            Matrix3::identity() - axis * axis.transpose(),
        )
    });
    let mut buffer = data.vertex_buffer.modify();
    for mut vertex in buffer.iter_mut() {
        let indices = vertex.read_4_u8(VertexAttributeUsage::BoneIndices).unwrap();
        let weights = vertex.read_4_f32(VertexAttributeUsage::BoneWeight).unwrap();
        let original = vertex.read_3_f32(VertexAttributeUsage::Position).unwrap();
        let mut displacement = Vector3::zeros();
        let mut deformation = Matrix3::identity();
        for (joint, origin, radial) in &frames {
            let weight: f32 = indices
                .iter()
                .zip(weights.iter())
                .filter(|(index, _)| bones.get(**index as usize) == Some(joint))
                .map(|(_, weight)| *weight)
                .sum();
            let expansion = radial * ((factor - 1.0) * weight);
            displacement += expansion * (original - origin);
            deformation += expansion;
        }
        // Hand/finger-only vertices have no arm weight and keep their exact shape.
        // The existing wrist weights blend the wider forearm into the palm.
        vertex
            .write_3_f32(VertexAttributeUsage::Position, original + displacement)
            .unwrap();
        if let Ok(normal) = vertex.read_3_f32(VertexAttributeUsage::Normal) {
            let corrected = deformation.try_inverse().unwrap().transpose() * normal;
            vertex
                .write_3_f32(
                    VertexAttributeUsage::Normal,
                    corrected.try_normalize(1e-6).unwrap_or(normal),
                )
                .unwrap();
        }
        if let Ok(tangent) = vertex.read_4_f32(VertexAttributeUsage::Tangent) {
            let corrected = deformation * tangent.xyz();
            vertex
                .write_4_f32(
                    VertexAttributeUsage::Tangent,
                    corrected
                        .try_normalize(1e-6)
                        .unwrap_or(tangent.xyz())
                        .push(tangent.w),
                )
                .unwrap();
        }
    }
}
