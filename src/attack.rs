//! Source-sampled normal attacks. 2D forward/up wrist paths are adapted to the FPS camera.
use crate::{game::Actor, kits::kit};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Clip {
    samples: Vec<Sample>,
}
#[derive(Deserialize)]
struct Sample {
    hands: [[f32; 3]; 2],
    #[serde(default)]
    blades: [[f32; 3]; 2],
}

fn vivi_clip(finisher: bool) -> &'static Clip {
    static CLIPS: OnceLock<Vec<Clip>> = OnceLock::new();
    &CLIPS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/attack/vivi-combo.json"))
            .expect("validated Vivi attack and floating blade samples")
    })[usize::from(finisher)]
}

fn sample_path(samples: &[Sample], left: bool, phase: f32, blade: bool) -> [f32; 3] {
    if !phase.is_finite() || phase <= 0.0 || phase >= 1.0 {
        return [0.0; 3];
    }
    let at = phase * (samples.len() - 1) as f32;
    let a = at as usize;
    let b = (a + 1).min(samples.len() - 1);
    let t = at - a as f32;
    let smooth = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    let weight = smooth(phase / 0.08) * smooth((1.0 - phase) / 0.14);
    let path = |i: usize| if blade { samples[i].blades } else { samples[i].hands }[usize::from(left)];
    std::array::from_fn(|j| (path(a)[j] + (path(b)[j] - path(a)[j]) * t) * weight)
}

/// Empty hand gestures are sampled independently of the levitating weapon bones.
pub fn vivi_hand(left: bool, phase: f32, finisher: bool) -> HandMotion {
    let path = sample_path(&vivi_clip(finisher).samples, left, phase, false);
    let side = if left { 1.0 } else { -1.0 };
    HandMotion {
        position: [
            side * (path[0] * 0.18).clamp(-0.16, 0.16),
            (path[1] * 0.38).clamp(-0.18, 0.25),
            (path[0] * 0.28).clamp(-0.12, 0.28),
        ],
        angles: [
            (-path[2] * 0.4).clamp(-0.9, 0.9),
            side * (path[0] * 0.3).clamp(-0.35, 0.35),
            side * (path[2] * 0.3).clamp(-0.7, 0.7),
        ],
    }
}

/// Source-plane forward/up displacement and rotation of a floating dagger, relative to idle.
pub fn vivi_blade(left: bool, phase: f32, finisher: bool) -> [f32; 3] {
    sample_path(&vivi_clip(finisher).samples, left, phase, true)
}

/// Finish the recovery before the next shot, keeping the current FPS balance.
pub fn seconds(id: u8) -> f32 {
    kit(id).interval * 0.94
}

pub fn rate(actor: &Actor) -> f32 {
    1.0 / ((if actor.combat.haste > 0.0 { 0.75 } else { 1.0 })
        * (if actor.combat.giant > 0.0 { 0.62 } else { 1.0 })
        * (if actor.combat.slow > 0.0 { 1.25 } else { 1.0 }))
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct HandMotion {
    pub position: [f32; 3],
    pub angles: [f32; 3],
}

/// The weapon-side source chain maps to shipped Hand.L. The second path is the left hand.
/// Amplitude is limited for near-camera readability; timings and reversals are source-derived.
pub fn hand(id: u8, left: bool, phase: f32) -> HandMotion {
    if !phase.is_finite() || phase <= 0.0 || phase >= 1.0 || id >= 15 {
        return HandMotion::default();
    }
    static CLIPS: OnceLock<Vec<Clip>> = OnceLock::new();
    let clips = CLIPS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/attack/motions.json"))
            .expect("validated original attack samples")
    });
    let samples = &clips[id as usize].samples;
    let at = phase * (samples.len() - 1) as f32;
    let a = at as usize;
    let b = (a + 1).min(samples.len() - 1);
    let t = at - a as f32;
    let smooth = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    let weight = smooth(phase / 0.08) * smooth((1.0 - phase) / 0.14);
    let i = usize::from(left);
    let mut path: [f32; 3] = std::array::from_fn(|j| {
        samples[a].hands[i][j] + (samples[b].hands[i][j] - samples[a].hands[i][j]) * t
    });
    if id == 1 {
        // Naia faces negative source X. The sprite elbow turns through 360 degrees;
        // its periodic wrist accent must not clamp the 3D dolphin upside down.
        path[0] = -path[0];
        path[2] = path[2].sin();
    }
    let side = if left { 1.0 } else { -1.0 };
    HandMotion {
        position: [
            side * (path[0] * 0.10).clamp(-0.10, 0.10) * weight,
            (path[1] * 0.30).clamp(-0.18, 0.18) * weight,
            (path[0] * 0.34).clamp(-0.20, 0.26) * weight,
        ],
        angles: [
            (-path[2] * 0.36).clamp(-0.85, 0.85) * weight,
            side * (path[0] * 0.22).clamp(-0.24, 0.24) * weight,
            side * (path[2] * 0.18).clamp(-0.45, 0.45) * weight,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vivi_has_distinct_gestures_and_independent_blades_for_both_attacks() {
        let normal = vivi_clip(false);
        let finisher = vivi_clip(true);
        assert_eq!(normal.samples.len(), 58);
        assert_eq!(finisher.samples.len(), 52);
        for enhanced in [false, true] {
            for left in [false, true] {
                assert_eq!(vivi_hand(left, 0.0, enhanced), HandMotion::default());
                assert_eq!(vivi_hand(left, 1.0, enhanced), HandMotion::default());
                assert_eq!(vivi_blade(left, 1.0, enhanced), [0.0; 3]);
                assert_ne!(
                    vivi_blade(left, 0.4, enhanced),
                    vivi_hand(left, 0.4, enhanced).position
                );
                for i in 0..=240 {
                    let phase = i as f32 / 240.0;
                    let hand = vivi_hand(left, phase, enhanced);
                    assert!(hand
                        .position
                        .into_iter()
                        .chain(hand.angles)
                        .all(f32::is_finite));
                    assert!(vivi_blade(left, phase, enhanced)
                        .into_iter()
                        .all(f32::is_finite));
                }
            }
        }
        assert_ne!(vivi_hand(false, 0.4, false), vivi_hand(false, 0.4, true));
        assert_ne!(vivi_blade(false, 0.4, false), vivi_blade(false, 0.4, true));
    }

    #[test]
    fn all_source_clips_are_distinct_complete_and_return_to_the_grip() {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../assets/characters/attack/motions.json")).unwrap();
        let clips = data.as_array().unwrap();
        assert_eq!(clips.len(), 15);
        let mut signatures = std::collections::HashSet::new();
        for (id, clip) in clips.iter().enumerate() {
            assert_eq!(clip["clip"], "Attack1_1");
            assert_eq!(
                clip["samples"].as_array().unwrap().len(),
                (clip["duration"].as_f64().unwrap() * 30.0).round() as usize + 1
            );
            signatures.insert(clip["samples"].to_string());
            assert!(seconds(id as u8) < kit(id as u8).interval);
            for left in [false, true] {
                assert_eq!(hand(id as u8, left, 0.0), HandMotion::default());
                assert_eq!(hand(id as u8, left, 1.0), HandMotion::default());
                let mut prior = hand(id as u8, left, 0.0);
                let mut movement = 0.0_f32;
                for i in 1..=240 {
                    let next = hand(id as u8, left, i as f32 / 240.0);
                    for j in 0..3 {
                        assert!(next.position[j].is_finite() && next.angles[j].is_finite());
                        assert!((next.position[j] - prior.position[j]).abs() < 0.10);
                        assert!(
                            (next.angles[j] - prior.angles[j]).abs() < 0.35,
                            "wrist discontinuity: {id}/{left}/{i}"
                        );
                        movement += next.position[j].abs() + next.angles[j].abs();
                    }
                    prior = next;
                }
                assert!(movement > 0.05, "missing hand motion {id}/{left}");
            }
        }
        assert_eq!(signatures.len(), 15);
    }
}
