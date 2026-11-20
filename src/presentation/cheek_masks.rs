//! Authored skin-only controls, keyed before FBX UV seam duplication.
use super::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug)]
pub(super) struct Mask {
    pub centers: [Vector3<f32>; 2],
    pub hit_radius: f32,
    pub tear_anchors: [Vector3<f32>; 2],
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

#[derive(Deserialize)]
struct RigSource {
    face: Landmarks,
}
#[derive(Deserialize)]
struct Landmarks {
    center_x: f32,
    eye_x: f32,
    eye_y: f32,
    front: f32,
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
        let rig: RigSource = serde_json::from_slice(
            &std::fs::read(characters::asset_path(&format!("{slug}-rig.json")))
                .map_err(|e| format!("{slug} tear landmarks: {e}"))?,
        )
        .map_err(|e| e.to_string())?;
        // Landmarks and the skin are authored in FBX source space, before the
        // rider/head animation. World-space face heights put Ui's tears on her hands.
        let tear_anchors = [-1.0, 1.0].map(|side| {
            let target = Vector3::new(
                rig.face.center_x + side * rig.face.eye_x,
                -rig.face.front,
                rig.face.eye_y - 0.035,
            );
            let nearest = source
                .vertices
                .iter()
                .min_by(|a, b| {
                    let score = |row: &[f32; 12]| {
                        let delta = Vector3::new(row[0], row[1], row[2]) - target;
                        delta.x * delta.x + delta.z * delta.z + delta.y * delta.y * 0.15
                    };
                    score(a).total_cmp(&score(b))
                })
                .expect("nonempty cheek skin");
            Vector3::new(target.x, nearest[1] - 0.006, target.z)
        });
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
            tear_anchors,
            vertices,
        });
    }
    let _ = MASKS.set(masks);
    Ok(())
}

pub(super) fn get(id: u8) -> &'static Mask {
    &MASKS.get().expect("validated cheek skin masks")[id as usize]
}
