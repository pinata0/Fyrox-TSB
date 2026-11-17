//! Client-only cheek interaction and sampled original standing-motion timing.
use serde::Deserialize;
use std::sync::OnceLock;

pub const MAX_RELEASE_SECONDS: f32 = 3.0;
pub const RELEASE_FADE_SECONDS: f32 = 0.35;

pub fn release_duration(id: u8) -> f32 {
    motions(id).release.duration.min(MAX_RELEASE_SECONDS)
}

#[derive(Debug, Deserialize)]
pub struct Clip {
    pub duration: f32,
    pub samples: Vec<[f32; 8]>,
    pub faces: Vec<[f32; 4]>,
    pub hands: Vec<[f32; 4]>,
}
impl Clip {
    pub fn hands(&self, time: f32, looping: bool) -> [f32; 4] {
        let t = if looping {
            time.rem_euclid(self.duration)
        } else {
            time.clamp(0.0, self.duration)
        };
        let frame = t / self.duration * (self.hands.len() - 1) as f32;
        let a = frame.floor() as usize;
        let b = (a + 1).min(self.hands.len() - 1);
        std::array::from_fn(|i| {
            self.hands[a][i] + (self.hands[b][i] - self.hands[a][i]) * frame.fract()
        })
    }
    pub fn face(&self, time: f32, looping: bool) -> [f32; 4] {
        let t = if looping {
            time.rem_euclid(self.duration)
        } else {
            time.clamp(0.0, self.duration)
        };
        self.faces[((t / self.duration * (self.faces.len() - 1) as f32) as usize)
            .min(self.faces.len() - 1)]
    }
    pub fn sample(&self, time: f32, looping: bool) -> [f32; 8] {
        let t = if looping {
            time.rem_euclid(self.duration)
        } else {
            time.clamp(0.0, self.duration)
        };
        let frame = t / self.duration * (self.samples.len() - 1) as f32;
        let a = frame.floor() as usize;
        let b = (a + 1).min(self.samples.len() - 1);
        std::array::from_fn(|i| {
            self.samples[a][i] + (self.samples[b][i] - self.samples[a][i]) * frame.fract()
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct Motions {
    pub slug: String,
    pub hold: Clip,
    pub release: Clip,
}
pub fn motions(id: u8) -> &'static Motions {
    static DATA: OnceLock<Vec<Motions>> = OnceLock::new();
    &DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/cheeks/motions.json"))
            .expect("validated cheek motion samples")
    })[id as usize]
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    Held,
    Released,
}

#[derive(Debug, Default)]
pub struct Interaction {
    pub phase: Phase,
    /// -1/+1 in model space, mirrored independently of camera screen coordinates.
    pub side: f32,
    pub age: f32,
    pub pull: [f32; 2],
    origin: [f32; 2],
    pixels_per_unit: f32,
    dragged: bool,
}
impl Interaction {
    pub fn begin(&mut self, side: f32, cursor: [f32; 2], pixels_per_unit: f32) {
        *self = Self {
            phase: Phase::Held,
            side,
            origin: cursor,
            pixels_per_unit: pixels_per_unit.max(1.0),
            ..Default::default()
        };
    }
    pub fn drag(&mut self, cursor: [f32; 2]) {
        if self.phase != Phase::Held || !cursor.iter().all(|v| v.is_finite()) {
            return;
        }
        let dx = cursor[0] - self.origin[0];
        let dy = cursor[1] - self.origin[1];
        self.dragged |= dx.hypot(dy) >= 4.0;
        // Keep the drag sensitivity, but cap extension at one quarter of the original.
        self.pull = [
            (0.5 * dx / self.pixels_per_unit).clamp(-0.12, 0.12),
            (-0.5 * dy / self.pixels_per_unit).clamp(-0.07, 0.07),
        ];
    }
    pub fn release(&mut self) {
        if self.phase == Phase::Held {
            if self.dragged {
                self.phase = Phase::Released;
                self.age = 0.0;
            } else {
                self.cancel();
            }
        }
    }
    pub fn cancel(&mut self) {
        *self = Self::default();
    }
    pub fn tick(&mut self, dt: f32, id: u8) {
        if self.phase == Phase::Idle {
            return;
        }
        if dt.is_finite() {
            self.age += dt.max(0.0);
        }
        if self.phase == Phase::Released && self.age >= release_duration(id) + RELEASE_FADE_SECONDS
        {
            self.cancel();
        }
    }
    pub fn blend(&self, id: u8) -> f32 {
        match self.phase {
            Phase::Idle => 0.0,
            Phase::Held => (self.age / 0.12).min(1.0),
            Phase::Released => {
                let remaining = ((release_duration(id) + RELEASE_FADE_SECONDS - self.age)
                    / RELEASE_FADE_SECONDS)
                    .clamp(0.0, 1.0);
                remaining * remaining * (3.0 - 2.0 * remaining)
            }
        }
    }
    pub fn stretch(&self) -> [f32; 2] {
        let spring = match self.phase {
            Phase::Idle => 0.0,
            Phase::Held => 1.0,
            Phase::Released => (-14.0 * self.age).exp() * (23.0 * self.age).cos(),
        };
        self.pull.map(|v| v * spring)
    }
    pub fn pose(&self, id: u8) -> [f32; 8] {
        match self.phase {
            Phase::Idle => [0.0; 8],
            Phase::Held => motions(id).hold.sample(self.age, true),
            Phase::Released => motions(id)
                .release
                .sample(self.age.min(release_duration(id)), false),
        }
    }
    pub fn face(&self, id: u8) -> [f32; 4] {
        match self.phase {
            Phase::Idle => [0.0; 4],
            Phase::Held => motions(id).hold.face(self.age, true),
            Phase::Released => motions(id)
                .release
                .face(self.age.min(release_duration(id)), false),
        }
    }
    pub fn hands(&self, id: u8) -> [f32; 4] {
        match self.phase {
            Phase::Idle => [0.0; 4],
            Phase::Held => motions(id).hold.hands(self.age, true),
            Phase::Released => motions(id)
                .release
                .hands(self.age.min(release_duration(id)), false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_clips_cover_fifteen_unique_roster_entries() {
        let mut unique = std::collections::HashSet::new();
        for id in 0..15 {
            let m = motions(id);
            assert!(unique.insert(&m.slug));
            for clip in [&m.hold, &m.release] {
                assert!(clip.duration > 0.2 && clip.duration < 7.0);
                assert!(clip.samples.len() >= 9);
                assert_eq!(clip.samples.len(), clip.faces.len());
                assert_eq!(clip.samples.len(), clip.hands.len());
                assert!(clip.samples.iter().flatten().all(|x| x.is_finite()));
                assert!(clip.hands.iter().flatten().all(|x| x.is_finite()));
                assert!(clip.faces.iter().all(|f| (0.0..=1.0).contains(&f[0])
                    && (0.0..=1.0).contains(&f[1])
                    && (-1.0..=1.0).contains(&f[2])
                    && (0.0..=3.0).contains(&f[3])));
                assert!(clip.samples.windows(2).any(|w| w[0] != w[1]));
                assert_eq!(
                    clip.sample(clip.duration, false),
                    *clip.samples.last().unwrap()
                );
                assert_eq!(clip.sample(clip.duration, true), clip.samples[0]);
            }
        }
    }
    #[test]
    fn drag_is_bounded_release_is_continuous_and_returns_to_idle() {
        for id in 0..15 {
            let mut s = Interaction::default();
            s.begin(-1.0, [300.0, 200.0], 400.0);
            s.drag([-10000.0, -10000.0]);
            assert_eq!(s.stretch(), [-0.12, 0.07]);
            s.release();
            assert_eq!(s.phase, Phase::Released);
            assert_eq!(s.stretch(), [-0.12, 0.07]);
            for _ in 0..80 {
                s.tick(0.1, id);
            }
            assert_eq!(s.phase, Phase::Idle);
            assert_eq!(s.stretch(), [0.0; 2]);
        }
    }
    #[test]
    fn click_cancel_and_regrab_do_not_leave_stale_drag() {
        let mut s = Interaction::default();
        s.begin(1.0, [0.0; 2], 300.0);
        s.release();
        assert_eq!(s.phase, Phase::Idle);
        s.begin(1.0, [0.0; 2], 300.0);
        s.drag([100.0, 0.0]);
        s.release();
        s.begin(-1.0, [100.0; 2], 300.0);
        assert_eq!(s.pull, [0.0; 2]);
        assert_eq!(s.age, 0.0);
        s.cancel();
        s.drag([500.0; 2]);
        s.release();
        assert_eq!(s.phase, Phase::Idle);
    }
    #[test]
    fn release_fades_to_idle_within_three_and_a_half_seconds_for_every_hero() {
        for id in 0..15 {
            let mut s = Interaction::default();
            s.begin(-1.0, [0.0; 2], 300.0);
            s.drag([100.0, -40.0]);
            s.tick(8.0, id);
            assert_eq!(s.phase, Phase::Held);
            s.release();
            s.tick(release_duration(id), id);
            assert_eq!(s.phase, Phase::Released);
            assert!((s.blend(id) - 1.0).abs() < 0.00001);
            let pose = s.pose(id);
            s.tick(RELEASE_FADE_SECONDS / 2.0, id);
            assert!((s.blend(id) - 0.5).abs() < 0.00001);
            assert_eq!(s.pose(id), pose);
            s.tick(RELEASE_FADE_SECONDS / 2.0 + 0.001, id);
            assert_eq!(s.phase, Phase::Idle);
            assert_eq!(s.blend(id), 0.0);
            assert_eq!(s.stretch(), [0.0; 2]);
            assert_eq!(s.pose(id), [0.0; 8]);
            assert_eq!(s.face(id), [0.0; 4]);
            assert_eq!(s.hands(id), [0.0; 4]);

            // A slow frame must not prolong the reaction beyond its timeout.
            s.begin(1.0, [0.0; 2], 300.0);
            s.drag([100.0, 0.0]);
            s.release();
            s.tick(3.5, id);
            assert_eq!(s.phase, Phase::Idle);
        }
    }
    #[test]
    fn resize_scale_gives_same_world_stretch() {
        let mut a = Interaction::default();
        let mut b = Interaction::default();
        a.begin(1.0, [0.0; 2], 400.0);
        b.begin(1.0, [0.0; 2], 200.0);
        a.drag([80.0, 40.0]);
        b.drag([40.0, 20.0]);
        assert_eq!(a.pull, b.pull);
        assert_eq!(a.stretch(), [0.1, -0.05]);
    }
}
