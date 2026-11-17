//! Bounded, low-latency team voice. Wire audio is 20 ms of mono 16 kHz PCM16.
//! Kept independent of audio devices so dedicated servers need no audio backend.
use crate::game::PLAYERS;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub const SAMPLE_RATE: u32 = 16_000;
pub const FRAME_SAMPLES: usize = 320;
pub const QUEUED_FRAMES: usize = 6;
pub type Samples = [i16; FRAME_SAMPLES];
const MAX_AGE: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, PartialEq)]
pub struct VoiceFrame {
    pub sender: usize,
    pub sequence: u64,
    pub samples: Samples,
}

#[derive(Debug)]
pub struct VoiceInbox {
    queues: [VecDeque<(Instant, VoiceFrame)>; PLAYERS],
    sequences: [u64; PLAYERS],
}
impl Default for VoiceInbox {
    fn default() -> Self {
        Self {
            queues: std::array::from_fn(|_| VecDeque::new()),
            sequences: [0; PLAYERS],
        }
    }
}
impl VoiceInbox {
    pub fn push(&mut self, frame: VoiceFrame) {
        if frame.sender >= PLAYERS || frame.sequence <= self.sequences[frame.sender] {
            return;
        }
        self.sequences[frame.sender] = frame.sequence;
        let queue = &mut self.queues[frame.sender];
        if queue.len() == QUEUED_FRAMES {
            queue.pop_front();
        }
        queue.push_back((Instant::now(), frame));
    }
    pub fn drain(&mut self) -> Vec<VoiceFrame> {
        let now = Instant::now();
        self.queues
            .iter_mut()
            .flat_map(|queue| queue.drain(..))
            .filter(|(time, _)| now.duration_since(*time) <= MAX_AGE)
            .map(|(_, frame)| frame)
            .collect()
    }
}

/// A small burst allowance absorbs tick jitter without permitting unbounded relay traffic.
#[derive(Debug)]
pub struct VoiceRate {
    sequence: u64,
    credit: f32,
    updated: Instant,
}
impl Default for VoiceRate {
    fn default() -> Self {
        Self {
            sequence: 0,
            credit: 10.0,
            updated: Instant::now(),
        }
    }
}
impl VoiceRate {
    pub fn accept(&mut self, sequence: u64, now: Instant) -> bool {
        if sequence <= self.sequence {
            return false;
        }
        self.sequence = sequence;
        self.credit =
            (self.credit + now.duration_since(self.updated).as_secs_f32() * 50.0).min(10.0);
        self.updated = now;
        if self.credit < 1.0 {
            return false;
        }
        self.credit -= 1.0;
        true
    }
}

#[derive(Debug)]
pub struct MicState {
    pub muted: bool,
    pub focused: bool,
}
impl Default for MicState {
    fn default() -> Self {
        Self {
            muted: true,
            focused: true,
        }
    }
}
impl MicState {
    pub fn toggle_key(&mut self, pressed: bool, repeat: bool, typing: bool) -> bool {
        if !pressed || repeat || typing || !self.focused {
            return false;
        }
        self.muted = !self.muted;
        true
    }
    pub fn transmitting(&self, connected: bool, in_home: bool) -> bool {
        !self.muted && self.focused && connected && !in_home
    }
}

/// Downmix/resample device input, preserving fractional phase between callbacks.
/// An area average also low-pass filters high-rate input before downsampling.
pub struct Capture {
    rate: f64,
    remaining: f64,
    sum: f64,
    frame: Samples,
    cursor: usize,
}
impl Capture {
    pub fn new(input_rate: u32) -> Self {
        let rate = input_rate.max(1) as f64 / SAMPLE_RATE as f64;
        Self {
            rate,
            remaining: rate,
            sum: 0.0,
            frame: [0; FRAME_SAMPLES],
            cursor: 0,
        }
    }
    pub fn push(&mut self, sample: f32, mut emit: impl FnMut(Samples)) {
        let sample = if sample.is_finite() {
            sample.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let mut available: f64 = 1.0;
        while available > 1e-9 {
            let weight = available.min(self.remaining);
            self.sum += sample as f64 * weight;
            self.remaining -= weight;
            available -= weight;
            if self.remaining < 1e-9 {
                self.frame[self.cursor] = (self.sum / self.rate * i16::MAX as f64).round() as i16;
                self.sum = 0.0;
                self.remaining = self.rate;
                self.cursor += 1;
                if self.cursor == FRAME_SAMPLES {
                    emit(self.frame);
                    self.cursor = 0;
                }
            }
        }
    }
}

#[derive(Default)]
struct Speaker {
    frames: VecDeque<VoiceFrame>,
    sample: usize,
    playing: bool,
    last: u64,
    received: Option<Instant>,
}
/// Two-frame jitter buffer per speaker; missing audio becomes silence, never a replay.
pub struct VoiceMixer {
    speakers: [Speaker; PLAYERS],
}
impl Default for VoiceMixer {
    fn default() -> Self {
        Self {
            speakers: std::array::from_fn(|_| Speaker::default()),
        }
    }
}
impl VoiceMixer {
    pub fn push(&mut self, frame: VoiceFrame, now: Instant) {
        let Some(speaker) = self.speakers.get_mut(frame.sender) else {
            return;
        };
        if frame.sequence <= speaker.last {
            return;
        }
        if speaker
            .received
            .is_some_and(|time| now.duration_since(time) > MAX_AGE)
        {
            speaker.frames.clear();
            speaker.sample = 0;
            speaker.playing = false;
        }
        speaker.last = frame.sequence;
        speaker.received = Some(now);
        if speaker.frames.len() == QUEUED_FRAMES {
            speaker.frames.pop_front();
            speaker.sample = 0;
        }
        speaker.frames.push_back(frame);
    }
    pub fn next_sample(&mut self, now: Instant) -> f32 {
        let mut sum = 0.0;
        for speaker in &mut self.speakers {
            if speaker
                .received
                .is_some_and(|time| now.duration_since(time) > MAX_AGE)
            {
                speaker.frames.clear();
                speaker.sample = 0;
                speaker.playing = false;
            }
            if !speaker.playing {
                // Start short final utterances after 40 ms even if only one frame arrived.
                speaker.playing = speaker.frames.len() >= 2
                    || (!speaker.frames.is_empty()
                        && speaker
                            .received
                            .is_some_and(|t| now.duration_since(t) >= Duration::from_millis(40)));
            }
            if !speaker.playing {
                continue;
            }
            if let Some(frame) = speaker.frames.front() {
                sum += frame.samples[speaker.sample] as f32 / 32768.0;
                speaker.sample += 1;
                if speaker.sample == FRAME_SAMPLES {
                    speaker.frames.pop_front();
                    speaker.sample = 0;
                    speaker.playing = !speaker.frames.is_empty();
                }
            }
        }
        // Leave headroom for simultaneous speakers; hard limit the mixed output.
        (sum * 0.7).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mic_toggle_ignores_repeat_release_typing_and_gates_background() {
        let mut mic = MicState::default();
        assert!(!mic.transmitting(true, false));
        for (down, repeat, typing) in [
            (false, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            assert!(!mic.toggle_key(down, repeat, typing));
            assert!(mic.muted);
        }
        assert!(mic.toggle_key(true, false, false));
        assert!(mic.transmitting(true, false));
        assert!(!mic.transmitting(false, false));
        assert!(!mic.transmitting(true, true));
        mic.focused = false;
        assert!(!mic.transmitting(true, false));
        mic.focused = true;
        assert!(mic.transmitting(true, false));
        mic.toggle_key(true, false, false);
        assert!(!mic.transmitting(true, false));
    }
    #[test]
    fn capture_resamples_fractional_rates_without_drift() {
        for rate in [8_000, 16_000, 44_100, 48_000, 96_000] {
            let mut capture = Capture::new(rate);
            let mut frames = Vec::new();
            for _ in 0..rate {
                capture.push(0.25, |frame| frames.push(frame));
            }
            assert_eq!(frames.len(), 50, "{rate}");
            assert!(frames.iter().flatten().all(|sample| *sample == 8192));
        }
    }
    #[test]
    fn inbox_and_mixer_bound_backlog_reject_replays_and_expire() {
        let mut inbox = VoiceInbox::default();
        for sequence in 1..=30 {
            inbox.push(VoiceFrame {
                sender: 0,
                sequence,
                samples: [1000; FRAME_SAMPLES],
            });
        }
        inbox.push(VoiceFrame {
            sender: 0,
            sequence: 30,
            samples: [0; FRAME_SAMPLES],
        });
        let frames = inbox.drain();
        assert_eq!(frames.len(), QUEUED_FRAMES);
        assert_eq!(frames[0].sequence, 25);
        assert!(inbox.drain().is_empty());
        let now = Instant::now();
        let mut mixer = VoiceMixer::default();
        mixer.push(frames[0].clone(), now);
        assert_eq!(mixer.next_sample(now), 0.0);
        mixer.push(frames[1].clone(), now);
        assert!(mixer.next_sample(now) > 0.0);
        assert_eq!(
            mixer.next_sample(now + MAX_AGE + Duration::from_millis(1)),
            0.0
        );
        mixer.push(frames[1].clone(), now);
        assert_eq!(mixer.next_sample(now), 0.0);
    }
    #[test]
    fn rate_limit_allows_realtime_frames_and_caps_floods() {
        let mut rate = VoiceRate::default();
        let now = Instant::now();
        for sequence in 1..=10 {
            assert!(rate.accept(sequence, now));
        }
        assert!(!rate.accept(11, now));
        assert!(!rate.accept(10, now));
        assert!(rate.accept(12, now + Duration::from_millis(21)));
    }
}
