//! Naia's original Attack1_1 attachment switches and facial scale curves.
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Part {
    attachment: String,
    scale: f32,
}

#[derive(Deserialize)]
struct Sample {
    eyes: [Part; 2],
    mouth: Part,
}

#[derive(Deserialize)]
struct Detail {
    samples: Vec<Sample>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Face {
    /// Original, blink, wink, wink + smile, scissors + smile, scissors.
    pub texture: usize,
    /// Model coordinate sides: L (-X), R (+X).
    pub blink: [f32; 2],
    pub smile: f32,
    pub jaw: f32,
}

/// Phase uses the same attack clock as the skeletal clip, including haste and replay.
pub fn face(phase: f32) -> Face {
    if !phase.is_finite() || phase <= 0.0 || phase >= 1.0 {
        return Face::default();
    }
    static DETAIL: OnceLock<Detail> = OnceLock::new();
    let samples = &DETAIL
        .get_or_init(|| {
            serde_json::from_str(include_str!("../assets/characters/attack/naia-detail.json"))
                .expect("validated Naia facial source")
        })
        .samples;
    let at = phase * (samples.len() - 1) as f32;
    let a = at.floor() as usize;
    let b = (a + 1).min(samples.len() - 1);
    let t = at - a as f32;
    let sample = &samples[a];
    let happy = sample.mouth.attachment == "F_Mouth_Happy2";
    let scissors = sample.eyes[0].attachment == "Eye_Scissors_L";
    let wink = sample.eyes[1].attachment == "Eye_Happy2_R";
    let blink = [1, 0].map(|i| {
        if sample.eyes[i].attachment.contains("Scissors")
            || sample.eyes[i].attachment.contains("Happy")
        {
            1.0
        } else {
            let scale =
                sample.eyes[i].scale + (samples[b].eyes[i].scale - sample.eyes[i].scale) * t;
            (1.0 - scale).clamp(0.0, 1.0)
        }
    });
    let texture = match (scissors, wink, happy) {
        (true, _, true) => 4,
        (true, _, false) => 5,
        (_, true, true) => 3,
        (_, true, false) => 2,
        _ if blink.iter().all(|b| *b > 0.55) => 1,
        _ => 0,
    };
    Face {
        texture,
        blink,
        smile: if happy { 0.6 } else { 0.0 },
        jaw: if happy {
            ((sample.mouth.scale + (samples[b].mouth.scale - sample.mouth.scale) * t) * 0.22)
                .clamp(0.0, 0.4)
        } else {
            0.0
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_original_wink_scissors_smile_and_recovery_order() {
        assert_eq!(face(0.18).texture, 0);
        let wink = face(0.30);
        assert_eq!(wink.texture, 2);
        assert_eq!(wink.blink[0], 1.0);
        assert!(wink.blink[1] < 0.1);
        assert_eq!(face(0.49).texture, 3);
        assert_eq!(face(0.54).texture, 4);
        assert!(face(0.64).jaw > 0.1);
        assert_eq!(face(0.77).texture, 5);
        assert_eq!(face(0.78).texture, 1);
        assert_eq!(face(0.90).texture, 0);
        assert_eq!(face(1.0), Face::default());
    }

    #[test]
    fn seeking_and_repeated_attacks_never_leave_a_stale_expression() {
        for phase in [f32::NAN, f32::INFINITY, -1.0, 0.0, 1.0, 2.0] {
            assert_eq!(face(phase), Face::default());
        }
        for i in 0..=1000 {
            let f = face(i as f32 / 1000.0);
            assert!(f.texture < 6);
            assert!(f
                .blink
                .into_iter()
                .chain([f.smile, f.jaw])
                .all(|x| x.is_finite() && (0.0..=1.0).contains(&x)));
        }
        assert_eq!(face(0.3), face(0.3));
    }
}
