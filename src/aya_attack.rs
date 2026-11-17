//! Aya's archived flower-fold, blade switches and separate normal/enhanced wrist paths.
use crate::attack::HandMotion;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Clip {
    duration: f32,
    samples: Vec<Sample>,
}
#[derive(Deserialize)]
struct Sample {
    hands: [[f32; 3]; 2],
    fold: f32,
    spread: f32,
    blade: u8,
    blade_alpha: f32,
}
fn clip(enhanced: bool) -> &'static Clip {
    static CLIPS: OnceLock<Vec<Clip>> = OnceLock::new();
    &CLIPS.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../assets/characters/attack/aya-transform.json"
        ))
        .expect("validated Aya source attacks")
    })[usize::from(enhanced)]
}
fn samples(phase: f32, enhanced: bool) -> Option<(&'static Sample, &'static Sample, f32)> {
    if !phase.is_finite() || phase <= 0.0 || phase >= 1.0 {
        return None;
    }
    let c = clip(enhanced);
    let at = phase * c.duration * 30.0;
    let a = (at.floor() as usize).min(c.samples.len() - 1);
    Some((
        &c.samples[a],
        &c.samples[(a + 1).min(c.samples.len() - 1)],
        at.fract(),
    ))
}
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Weapon {
    pub fold: f32,
    pub spread: f32,
    pub blade: u8,
    pub emergence: f32,
}
pub fn weapon(phase: f32, enhanced: bool) -> Weapon {
    let Some((a, b, t)) = samples(phase, enhanced) else {
        return Weapon::default();
    };
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    Weapon {
        fold: lerp(a.fold, b.fold).clamp(0.0, 1.0),
        spread: lerp(a.spread, b.spread).clamp(0.0, 1.0),
        // Attachment swaps are discrete at the source frame; morph curves interpolate.
        blade: a.blade,
        emergence: lerp(a.blade_alpha, b.blade_alpha).clamp(0.0, 1.0),
    }
}
pub fn hand(left: bool, phase: f32, enhanced: bool) -> HandMotion {
    let Some((a, b, t)) = samples(phase, enhanced) else {
        return HandMotion::default();
    };
    let smooth = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    let weight = smooth(phase / 0.08) * smooth((1.0 - phase) / 0.14);
    let side = if left { 1.0 } else { -1.0 };
    let i = usize::from(left);
    let p: [f32; 3] = std::array::from_fn(|j| a.hands[i][j] + (b.hands[i][j] - a.hands[i][j]) * t);
    HandMotion {
        position: [
            side * (p[0] * 0.10).clamp(-0.10, 0.10) * weight,
            (p[1] * 0.30).clamp(-0.18, 0.18) * weight,
            (p[0] * 0.34).clamp(-0.20, 0.26) * weight,
        ],
        angles: [
            (-p[2] * 0.36).clamp(-0.85, 0.85) * weight,
            side * (p[0] * 0.22).clamp(-0.24, 0.24) * weight,
            side * (p[2] * 0.18).clamp(-0.45, 0.45) * weight,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_switches_and_folds_are_distinct_and_recover_without_stale_blades() {
        assert_eq!(weapon(0.4, false).blade, 1);
        assert_eq!(weapon(0.4, true).blade, 2);
        assert_eq!(weapon(0.7, true).blade, 3);
        assert!(weapon(0.4, false).fold > 0.99);
        assert!(weapon(0.4, true).spread > 0.99);
        assert!(weapon(0.7, true).spread < 0.1);
        assert_ne!(hand(false, 0.4, false), hand(false, 0.4, true));
        // 43/30 seconds: the original enhanced attack changes to the hooked blade.
        let switch = 43.0 / 88.0;
        assert_eq!(weapon(switch - 0.001, true).blade, 2);
        assert_eq!(weapon(switch + 0.001, true).blade, 3);
        for enhanced in [false, true] {
            for phase in [0.0, 1.0, 1.5, f32::NAN] {
                assert_eq!(weapon(phase, enhanced), Weapon::default());
                assert_eq!(hand(false, phase, enhanced), HandMotion::default());
            }
            for i in 0..1000 {
                let w = weapon(i as f32 / 1000.0, enhanced);
                assert!([w.fold, w.spread, w.emergence]
                    .into_iter()
                    .all(|x| x.is_finite() && (0.0..=1.0).contains(&x)));
            }
        }
    }
}
