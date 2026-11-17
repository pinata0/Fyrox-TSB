//! Restore sparse FBX offsets after Fyrox 1.0.1 deduplicates polygon vertices.
//! Its importer currently assigns repeated vertices to the next insertion index.
use super::characters;
use fyrox::{
    core::algebra::Vector3,
    resource::model::ModelResource,
    scene::mesh::{
        buffer::{VertexAttributeUsage, VertexReadTrait},
        surface::{BlendShapesContainer, InputBlendShapeData},
        Mesh,
    },
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Source {
    vertices: Vec<f32>,
    shapes: Vec<Shape>,
}

#[derive(Deserialize)]
struct Shape {
    name: String,
    indices: Vec<usize>,
    offsets: Vec<f32>,
    #[serde(default)]
    normal_indices: Vec<usize>,
    #[serde(default)]
    normal_targets: Vec<[f32; 4]>,
}

fn normal_delta(base: Vector3<f32>, target: [f32; 4]) -> Vector3<f32> {
    (Vector3::new(target[0], target[1], target[2]) - base) * target[3]
}

fn key(p: &[f32]) -> [i32; 3] {
    [p[0], p[1], p[2]].map(|v| (v * 10000.0).round() as i32)
}

pub(super) fn restore(model: &ModelResource, slug: &str) -> Result<(), String> {
    let bytes = std::fs::read(characters::asset_path(&format!("{slug}.morphs.json")))
        .map_err(|e| format!("{slug} morph source: {e}"))?;
    let source: Source = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let lookup: HashMap<_, _> = source
        .vertices
        .chunks_exact(3)
        .enumerate()
        .map(|(i, p)| (key(p), i))
        .collect();
    let model = model.data_ref();
    let mut restored = 0;
    for node in model.get_scene().graph.linear_iter() {
        let Some(mesh) = node.cast::<Mesh>() else {
            continue;
        };
        if mesh.blend_shapes().is_empty() {
            continue;
        }
        for surface in mesh.surfaces() {
            let mut data = surface.data_ref().data_ref();
            let mapping: Vec<_> = data
                .vertex_buffer
                .iter()
                .map(|v| {
                    let p = v
                        .read_3_f32(VertexAttributeUsage::Position)
                        .map_err(|e| e.to_string())?;
                    lookup
                        .get(&key(p.as_slice()))
                        .copied()
                        .ok_or_else(|| format!("{slug}: unmapped FBX vertex {p:?}"))
                })
                .collect::<Result<_, String>>()?;
            let mut inputs = Vec::new();
            for imported in mesh.blend_shapes().iter() {
                let shape = source
                    .shapes
                    .iter()
                    .find(|s| imported.name.ends_with(&s.name))
                    .ok_or_else(|| format!("{slug}: missing morph {}", imported.name))?;
                if shape.offsets.len() != shape.indices.len() * 3 {
                    return Err(format!("{slug}: malformed morph offsets"));
                }
                if shape.normal_indices.len() != shape.normal_targets.len()
                    || shape
                        .normal_indices
                        .iter()
                        .any(|&i| i >= source.vertices.len() / 3)
                    || shape.normal_targets.iter().any(|target| {
                        !target.iter().all(|v| v.is_finite()) || !(0.0..=1.0).contains(&target[3])
                    })
                {
                    return Err(format!("{slug}: malformed morph normal targets"));
                }
                let offsets: HashMap<_, _> =
                    shape
                        .indices
                        .iter()
                        .copied()
                        .zip(shape.offsets.chunks_exact(3).map(|d| {
                            fyrox::utils::vec3_f16_from_f32(Vector3::new(d[0], d[1], d[2]))
                        }))
                        .collect();
                let positions = mapping
                    .iter()
                    .enumerate()
                    .filter_map(|(i, source)| offsets.get(source).map(|delta| (i as u32, *delta)))
                    .collect();
                let targets: HashMap<_, _> = shape
                    .normal_indices
                    .iter()
                    .copied()
                    .zip(shape.normal_targets.iter().copied())
                    .collect();
                let mut normals = Default::default();
                if !targets.is_empty() {
                    let mut mapped = Vec::new();
                    for (i, (&source_index, vertex)) in
                        mapping.iter().zip(data.vertex_buffer.iter()).enumerate()
                    {
                        if let Some(&target) = targets.get(&source_index) {
                            let base = vertex
                                .read_3_f32(VertexAttributeUsage::Normal)
                                .map_err(|e| e.to_string())?;
                            mapped.push((
                                i as u32,
                                fyrox::utils::vec3_f16_from_f32(normal_delta(base, target)),
                            ));
                        }
                    }
                    normals = mapped.into_iter().collect();
                }
                inputs.push(InputBlendShapeData {
                    default_weight: 0.0,
                    name: imported.name.clone(),
                    positions,
                    normals,
                    tangents: Default::default(),
                });
            }
            data.blend_shapes_container = Some(BlendShapesContainer::from_lists(
                &data.vertex_buffer,
                &inputs,
            ));
            restored += inputs.len();
        }
    }
    if restored != source.shapes.len() {
        return Err(format!(
            "{slug}: restored {restored} of {} morph channels",
            source.shapes.len()
        ));
    }
    println!("MORPH_REMAP_OK: {slug}, {restored} channels, all imported vertices mapped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uv_duplicates_map_to_the_same_source_independent_of_insertion_order() {
        let original = [[0.25, 1.5, -0.2], [0.5, 1.6, -0.3]];
        let lookup: HashMap<_, _> = original
            .iter()
            .enumerate()
            .map(|(i, p)| (key(p), i))
            .collect();
        let imported = [original[1], original[0], original[1], original[0]];
        let mapping: Vec<_> = imported.iter().map(|p| lookup[&key(p)]).collect();
        assert_eq!(mapping, [1, 0, 1, 0]);
    }

    #[test]
    fn blink_replaces_each_seam_normal_and_preserves_neutral_at_zero() {
        let target = [0.0, -1.0, 0.0, 1.0];
        for base in [Vector3::new(0.6, -0.8, 0.0), Vector3::new(-0.8, -0.6, 0.0)] {
            let delta = normal_delta(base, target);
            assert!((base + delta - Vector3::new(0.0, -1.0, 0.0)).norm() < 1e-6);
            assert_eq!(base + delta * 0.0, base);
            let feathered = normal_delta(base, [0.0, -1.0, 0.0, 0.5]);
            assert!((feathered - delta * 0.5).norm() < 1e-6);
        }
    }
}
