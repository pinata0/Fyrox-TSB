//! Relax only compressing skin weights; preserve pins and UV seam continuity.
use super::*;
use std::collections::HashMap;

const MAX_COMPRESSION: f32 = 0.75;

#[derive(Debug)]
struct Triangle {
    indices: [usize; 3],
    basis: [Vector3<f32>; 2],
    area: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturated_drag_preserves_winding_pins_and_seams_in_every_direction() {
        let positions = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.03, 0.0, 0.0),
            Vector3::new(0.0, 0.03, 0.0),
            Vector3::new(0.03, 0.03, 0.0),
            Vector3::new(0.0, 0.0, 0.0),
        ];
        let vertices: Vec<_> = positions
            .iter()
            .enumerate()
            .map(|(index, &original)| Vertex {
                index,
                original,
                normal: Vector3::z(),
                control: super::super::super::cheek_masks::Control {
                    weights: if index == 1 || index == 3 {
                        [0.0; 2]
                    } else {
                        [1.0; 2]
                    },
                    gradients: [Vector3::zeros(); 2],
                    protected: 0,
                },
            })
            .collect();
        let triangles = [[0, 1, 2], [4, 3, 2]];
        let mut skin = Skin::new(&vertices, triangles.into_iter());
        for side in 0..2 {
            for dx in [-0.12, 0.0, 0.12] {
                for dy in [-0.07, 0.0, 0.07] {
                    let displacement = Vector3::new(dx, dy, f32::hypot(dx, dy) * 0.55);
                    skin.update(side, displacement);
                    skin.review();
                    let deformed: Vec<_> = positions
                        .iter()
                        .enumerate()
                        .map(|(i, p)| p + displacement * skin.weights[skin.groups[i]])
                        .collect();
                    assert_eq!(deformed[0], deformed[4]);
                    assert_eq!(deformed[1], positions[1]);
                    assert_eq!(deformed[3], positions[3]);
                    for [a, b, c] in triangles.map(|t| t.map(|i| i as usize)) {
                        let original =
                            (positions[b] - positions[a]).cross(&(positions[c] - positions[a]));
                        let current =
                            (deformed[b] - deformed[a]).cross(&(deformed[c] - deformed[a]));
                        assert!(current.dot(&original) / original.norm_squared() >= 0.249);
                    }
                }
            }
        }
        skin.update(0, Vector3::zeros());
        assert_eq!(
            skin.weights[skin.groups[0]], 1.0,
            "a new drag starts from the authored skin"
        );
    }
}

impl Triangle {
    fn gradient(&self, weights: &[f32]) -> Vector3<f32> {
        let [a, b, c] = self.indices.map(|i| weights[i]);
        self.basis[0] * (b - a) + self.basis[1] * (c - a)
    }
}

#[derive(Debug)]
pub(super) struct Skin {
    pub groups: Vec<usize>,
    original: Vec<[f32; 2]>,
    triangles: Vec<Triangle>,
    pub weights: Vec<f32>,
    pub gradients: Vec<Vector3<f32>>,
    previous: Option<(usize, Vector3<f32>)>,
}

impl Skin {
    pub fn new(vertices: &[Vertex], triangles: impl Iterator<Item = [u32; 3]>) -> Self {
        let mut lookup = HashMap::new();
        let mut original = Vec::new();
        let groups: Vec<_> = vertices
            .iter()
            .map(|v| {
                *lookup
                    .entry(super::super::cheek_masks::key(v.original))
                    .or_insert_with(|| {
                        original.push(v.control.weights);
                        original.len() - 1
                    })
            })
            .collect();
        let triangles = triangles
            .filter_map(|indices| {
                let [a, b, c] = indices.map(|i| i as usize);
                if [a, b, c]
                    .iter()
                    .all(|&i| vertices[i].control.weights == [0.0; 2])
                {
                    return None;
                }
                let e1 = vertices[b].original - vertices[a].original;
                let e2 = vertices[c].original - vertices[a].original;
                let normal = e1.cross(&e2);
                let area_squared = normal.norm_squared();
                if area_squared <= 1.0e-24 {
                    return None;
                }
                Some(Triangle {
                    indices: [a, b, c].map(|i| groups[i]),
                    basis: [
                        e2.cross(&normal) / area_squared,
                        normal.cross(&e1) / area_squared,
                    ],
                    area: area_squared.sqrt(),
                })
            })
            .collect();
        Self {
            weights: vec![0.0; original.len()],
            gradients: vec![Vector3::zeros(); original.len()],
            groups,
            original,
            triangles,
            previous: None,
        }
    }

    pub fn update(&mut self, side: usize, displacement: Vector3<f32>) {
        if self.previous == Some((side, displacement)) {
            return;
        }
        self.previous = Some((side, displacement));
        for (weight, original) in self.weights.iter_mut().zip(&self.original) {
            *weight = original[side];
        }
        // For p' = p + d*w, a triangle's signed area ratio is exactly
        // 1 + dot(grad(w), d). Only reduce weights that would fold the face.
        // Taking the minimum as the origin leaves every protected vertex at 0.
        for _ in 0..64 {
            let mut worst = 0.0_f32;
            for triangle in &self.triangles {
                let compression = -triangle.gradient(&self.weights).dot(&displacement);
                worst = worst.max(compression);
                if compression > MAX_COMPRESSION {
                    let minimum = triangle
                        .indices
                        .iter()
                        .map(|&i| self.weights[i])
                        .fold(f32::INFINITY, f32::min);
                    let scale = MAX_COMPRESSION / compression;
                    for i in triangle.indices {
                        self.weights[i] = minimum + (self.weights[i] - minimum) * scale;
                    }
                }
            }
            if worst <= MAX_COMPRESSION + 1.0e-5 {
                break;
            }
        }
        // A bounded solve must also be safe for nearly degenerate imported
        // triangles. Finish with one uniform correction if it has not converged.
        let worst = self
            .triangles
            .iter()
            .map(|t| -t.gradient(&self.weights).dot(&displacement))
            .fold(MAX_COMPRESSION, f32::max);
        let scale = MAX_COMPRESSION / worst;
        self.weights.iter_mut().for_each(|w| *w *= scale);

        self.gradients.fill(Vector3::zeros());
        let mut areas = vec![0.0; self.weights.len()];
        for triangle in &self.triangles {
            let gradient = triangle.gradient(&self.weights) * triangle.area;
            for i in triangle.indices {
                self.gradients[i] += gradient;
                areas[i] += triangle.area;
            }
        }
        for (gradient, area) in self.gradients.iter_mut().zip(areas) {
            if area > 0.0 {
                *gradient /= area;
            }
        }
    }

    pub fn review(&self) {
        let Some((_, displacement)) = self.previous else {
            return;
        };
        for triangle in &self.triangles {
            assert!(
                1.0 + triangle.gradient(&self.weights).dot(&displacement) >= 0.249,
                "cheek triangle folded during drag"
            );
        }
        for (weight, original) in self.weights.iter().zip(&self.original) {
            assert!(weight.is_finite());
            if original == &[0.0; 2] {
                assert_eq!(*weight, 0.0);
            }
        }
    }
}
