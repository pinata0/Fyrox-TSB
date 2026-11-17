//! Authored skin-only controls, keyed before FBX UV seam duplication.
use super::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug)]
pub(super) struct Mask {
    pub centers: [Vector3<f32>; 2],
    pub hit_radius: f32,
    pub vertices: HashMap<[i32; 3], Control>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Control {
    pub weights: [f32; 2],
    pub gradients: [Vector3<f32>; 2],
    pub protected: u8,
}

#[derive(Deserialize)]
struct Source {
    version: u8,
    centers: [[f32; 3]; 2],
    hit_radius: f32,
    vertices: Vec<[f32; 12]>,
}

static MASKS: std::sync::OnceLock<Vec<Mask>> = std::sync::OnceLock::new();

pub(super) fn key(p: Vector3<f32>) -> [i32; 3] {
    [p.x, p.y, p.z].map(|v| (v * 10000.0).round() as i32)
}

pub(super) fn load() -> Result<(), String> {
    let mut masks = Vec::new();
    for slug in characters::SLUGS {
        let bytes = std::fs::read(characters::asset_path(&format!("{slug}-cheeks.json")))
            .map_err(|e| format!("{slug} cheek mask: {e}"))?;
        let source: Source = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if source.version != 1 || !(0.10..=0.18).contains(&source.hit_radius) {
            return Err(format!("{slug}: invalid cheek control version/radius"));
        }
        let mut vertices: HashMap<[i32; 3], Control> = HashMap::new();
        for row in source.vertices {
            if !row.iter().all(|v| v.is_finite())
                || !row[3..5].iter().all(|v| (0.0..=1.0).contains(v))
                || (row[11] != 0.0 && row[3..5] != [0.0, 0.0])
            {
                return Err(format!("{slug}: invalid or unpinned cheek control"));
            }
            let control = Control {
                weights: [row[3], row[4]],
                gradients: [
                    Vector3::new(row[5], row[6], row[7]),
                    Vector3::new(row[8], row[9], row[10]),
                ],
                protected: row[11] as u8,
            };
            vertices
                .entry(key(Vector3::new(row[0], row[1], row[2])))
                .and_modify(|existing| {
                    existing.protected |= control.protected;
                    for side in 0..2 {
                        if control.weights[side] < existing.weights[side] {
                            existing.weights[side] = control.weights[side];
                            existing.gradients[side] = control.gradients[side];
                        }
                    }
                    if existing.protected != 0 {
                        existing.weights = [0.0; 2];
                        existing.gradients = [Vector3::zeros(); 2];
                    }
                })
                .or_insert(control);
        }
        masks.push(Mask {
            centers: source.centers.map(Vector3::from),
            hit_radius: source.hit_radius,
            vertices,
        });
    }
    let _ = MASKS.set(masks);
    Ok(())
}

pub(super) fn get(id: u8) -> &'static Mask {
    &MASKS.get().expect("validated cheek skin masks")[id as usize]
}
