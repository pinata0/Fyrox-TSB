//! Original Korean character recordings, separate from team chat, music and SFX.
use super::*;
use fyrox::scene::sound::{
    DataSource, Sound, SoundBuffer, SoundBufferResource, SoundBuilder, Status,
};
use std::{collections::VecDeque, io::Cursor};
use trickcal_shooting_balddagu::{
    cheeks::Phase as CheekPhase,
    sfx::{Cue as SoundCue, Tracker},
};

mod bank;

// Keep every character line about 6 dB quieter, including saved volume settings.
const BASE_GAIN: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cue {
    Select,
    Spawn,
    Start,
    Win,
    Lose,
    Death,
    Cheek,
    Line,
    Skill,
    Ultimate,
}
impl Cue {
    const COUNT: usize = 10;
    fn priority(self) -> u8 {
        match self {
            Self::Win | Self::Lose => 7,
            Self::Death => 6,
            Self::Ultimate => 5,
            Self::Skill => 4,
            Self::Start | Self::Spawn => 3,
            Self::Select | Self::Cheek => 2,
            Self::Line => 1,
        }
    }
    fn from_sound(cue: SoundCue) -> Option<Self> {
        Some(match cue {
            SoundCue::Respawn => Self::Spawn,
            SoundCue::RoundStart => Self::Start,
            SoundCue::Win => Self::Win,
            SoundCue::Lose => Self::Lose,
            SoundCue::Death => Self::Death,
            SoundCue::Skill(_) => Self::Skill,
            SoundCue::Ultimate(_) => Self::Ultimate,
            _ => return None,
        })
    }
}
struct Clip {
    hero: u8,
    cue: Cue,
    name: &'static str,
    speaker: &'static str,
    text: &'static str,
    bytes: &'static [u8],
}

fn new_lines<'a>(
    history: &'a ChatHistory,
    seen: &mut Option<u64>,
    audible: bool,
) -> Vec<&'a ChatMessage> {
    let latest = history.messages.back().map_or(0, |m| m.id);
    let previous = seen.replace(latest);
    if !audible {
        return vec![];
    }
    let Some(previous) = previous else {
        return vec![];
    };
    history
        .messages
        .iter()
        .filter(|m| m.id > previous && m.voice_line)
        .collect()
}

#[derive(Debug)]
struct Playing {
    node: Handle<Node>,
    speaker: usize,
    cue: Cue,
    spatial: bool,
}

#[derive(Debug)]
pub(super) struct Audio {
    cache: VecDeque<(usize, SoundBufferResource)>,
    failed: Vec<usize>,
    playing: Vec<Playing>,
    tracker: Tracker,
    screen: Option<(u8, u8)>,
    cheek: bool,
    chat_id: Option<u64>,
    variants: [usize; 15 * Cue::COUNT],
    next_line: [f32; PLAYERS],
    next_cheek: f32,
    time: f32,
    focused: bool,
    review: Option<Review>,
}
impl Default for Audio {
    fn default() -> Self {
        Self {
            cache: VecDeque::new(),
            failed: vec![],
            playing: vec![],
            tracker: Tracker::default(),
            screen: None,
            cheek: false,
            chat_id: None,
            variants: [0; 15 * Cue::COUNT],
            next_line: [0.0; PLAYERS],
            next_cheek: 0.0,
            time: 0.0,
            focused: true,
            review: std::env::args()
                .any(|a| a == "--character-voice-review")
                .then(Review::default),
        }
    }
}

fn decode(bytes: &[u8]) -> Result<SoundBufferResource, String> {
    let decoded = SoundBuffer::raw_generic(DataSource::Memory(Cursor::new(bytes.to_vec())))
        .map_err(|e| format!("{e:?}"))?;
    let rate = decoded.sample_rate();
    if rate == 0 || decoded.channel_count() == 0 {
        return Err("Invalid voice format".into());
    }
    let mut samples: Vec<f32> = decoded
        .samples()
        .chunks_exact(decoded.channel_count())
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect();
    let peak = samples.iter().fold(0.0_f32, |peak, v| peak.max(v.abs()));
    if samples.is_empty() || peak < 0.0001 || samples.iter().any(|s| !s.is_finite()) {
        return Err("Empty or invalid recording".into());
    }
    // Preserve the entire performance, original rate and pitch (SFX's 2s cap is unsuitable).
    for sample in &mut samples {
        *sample *= 0.8 / peak;
    }
    SoundBuffer::raw_generic(DataSource::Raw {
        sample_rate: rate,
        channel_count: 1,
        samples,
    })
    .map(SoundBufferResource::new_embedded)
    .map_err(|e| format!("{e:?}"))
}

impl Audio {
    pub(super) fn attach(&mut self) {
        // The previous graph was replaced: never remove an old handle from a new graph.
        self.playing.clear();
        self.tracker = Tracker::default();
        self.screen = None;
        self.cheek = false;
        self.chat_id = None;
        self.next_line = [0.0; PLAYERS];
    }
    pub(super) fn focus(&mut self, focused: bool) {
        self.focused = focused;
    }
    fn stop(&mut self, g: &mut Graph) {
        for playing in self.playing.drain(..) {
            if g.is_valid_handle(playing.node) {
                g.remove_node(playing.node);
            }
        }
    }
    fn buffer(&mut self, index: usize) -> Option<SoundBufferResource> {
        if let Some(position) = self.cache.iter().position(|(i, _)| *i == index) {
            let entry = self.cache.remove(position).unwrap();
            let buffer = entry.1.clone();
            self.cache.push_back(entry);
            return Some(buffer);
        }
        if self.failed.contains(&index) {
            return None;
        }
        match decode(bank::CLIPS[index].bytes) {
            Ok(buffer) => {
                if self.cache.len() >= 32 {
                    self.cache.pop_front();
                }
                self.cache.push_back((index, buffer.clone()));
                Some(buffer)
            }
            Err(error) => {
                eprintln!(
                    "CHARACTER_VOICE_ERROR: {}: {error}",
                    bank::CLIPS[index].name
                );
                self.failed.push(index);
                None
            }
        }
    }
    fn maintain(&mut self, g: &mut Graph, volume: f32) {
        self.playing.retain(|p| g.is_valid_handle(p.node));
        if !self.focused || volume <= 0.0 {
            self.stop(g);
            return;
        }
        for p in &self.playing {
            if let Some(sound) = g[p.node].cast_mut::<Sound>() {
                sound.set_gain(BASE_GAIN * volume);
            }
        }
    }
    fn play(
        &mut self,
        g: &mut Graph,
        hero: u8,
        cue: Cue,
        speaker: usize,
        position: Option<V3>,
        listener: V3,
        volume: f32,
    ) {
        if !self.focused
            || volume <= 0.0
            || hero as usize >= ROSTER.len()
            || speaker >= PLAYERS
            || position.is_some_and(|p| (p - listener).length() > 32.0)
        {
            return;
        }
        self.playing.retain(|p| g.is_valid_handle(p.node));
        if cue == Cue::Line && self.time < self.next_line[speaker] {
            return;
        }
        if cue == Cue::Cheek && self.time < self.next_cheek {
            return;
        }
        // One utterance per speaker. Local outcomes can interrupt chatter; chatter cannot
        // interrupt a death or result line. Limit nearby teammates to two extra voices.
        if let Some(old) = self.playing.iter().find(|p| p.speaker == speaker) {
            if old.cue.priority() > cue.priority() {
                return;
            }
        }
        let spatial = position.is_some();
        if spatial
            && self
                .playing
                .iter()
                .filter(|p| p.spatial && p.speaker != speaker)
                .count()
                >= 2
        {
            return;
        }
        let choices: Vec<usize> = bank::CLIPS
            .iter()
            .enumerate()
            .filter(|(_, clip)| clip.hero == hero && clip.cue == cue)
            .map(|(i, _)| i)
            .collect();
        if choices.is_empty() {
            return;
        }
        let serial = hero as usize * Cue::COUNT + cue as usize;
        let index = choices[self.variants[serial] % choices.len()];
        let Some(buffer) = self.buffer(index) else {
            return;
        };
        self.variants[serial] = self.variants[serial].wrapping_add(1);
        if let Some(i) = self.playing.iter().position(|p| p.speaker == speaker) {
            let old = self.playing.remove(i);
            g.remove_node(old.node);
        }
        let node = SoundBuilder::new(
            BaseBuilder::new()
                .with_name(format!("Character voice: {}", bank::CLIPS[index].name))
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(vec(position.unwrap_or(listener)))
                        .build(),
                ),
        )
        .with_buffer(Some(buffer))
        .with_status(Status::Playing)
        .with_play_once(true)
        .with_gain(BASE_GAIN * volume)
        .with_pitch(1.0)
        .with_spatial_blend_factor(if spatial { 1.0 } else { 0.0 })
        .with_radius(4.0)
        .with_max_distance(32.0)
        .with_rolloff_factor(1.5)
        .build(g)
        .to_base();
        self.playing.push(Playing {
            node,
            speaker,
            cue,
            spatial,
        });
        if cue == Cue::Line {
            self.next_line[speaker] = self.time + 4.0;
        }
        if cue == Cue::Cheek {
            self.next_cheek = self.time + 1.5;
        }
        let clip = &bank::CLIPS[index];
        println!(
            "CHARACTER_VOICE_PLAY: {} | t={:.2}s slot={} speaker={} cue={cue:?} text={:?}",
            clip.name, self.time, speaker, clip.speaker, clip.text
        );
    }
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) {
    if rt.character_voice.review.is_some() {
        review(
            &mut rt.character_voice,
            &mut context.scenes[rt.scene].graph,
            context.dt,
            &mut context.loop_controller,
        );
        return;
    }
    let w = rt.session.world();
    let slot = rt.session.slot();
    let hero = w.actors[slot].character;
    let listener = w.actors[slot].eye();
    let layer = if rt.home_open {
        0
    } else if rt.lobby.open {
        1
    } else if rt.selecting {
        2
    } else {
        3
    };
    let selection = (layer, if layer == 2 { rt.selected as u8 } else { hero });
    let cheek = layer == 2
        && rt.preview.as_ref().is_some_and(|p| {
            p.cheeks.id == rt.selected as u8 && p.cheeks.interaction.phase == CheekPhase::Held
        });
    let paused = rt.settings_menu.open || rt.error.is_some();
    let volume = rt.settings_menu.settings.character_voice_volume;
    let history = &rt.chat_transcript.history;
    let audio = &mut rt.character_voice;
    audio.time += context.dt.clamp(0.0, 0.1);
    let audible = audio.focused && volume > 0.0 && !paused;
    let events = audio.tracker.update(w, slot, audible && layer == 3);
    let old = audio.screen.replace(selection);
    let cheek_start = cheek && !audio.cheek;
    audio.cheek = cheek;
    let messages = new_lines(
        history,
        &mut audio.chat_id,
        audible && layer == 3 && !rt.deathcam.is_replay(),
    );
    let g = &mut context.scenes[rt.scene].graph;
    if old.is_some_and(|previous| previous != selection) || paused {
        audio.stop(g);
    }
    audio.maintain(g, volume);
    if !audible {
        return;
    }
    if layer == 2 {
        if old != Some(selection) {
            audio.play(g, selection.1, Cue::Select, slot, None, listener, volume);
        }
        if cheek_start {
            audio.play(g, selection.1, Cue::Cheek, slot, None, listener, volume);
        }
    } else if layer == 3 {
        let mut cues: Vec<_> = events
            .iter()
            .filter(|event| event.position.is_none())
            .filter_map(|event| Cue::from_sound(event.cue))
            .filter(|cue| !rt.deathcam.is_replay() || !matches!(cue, Cue::Skill | Cue::Ultimate))
            .collect();
        if old != Some(selection)
            && w.actors[slot].alive()
            && matches!(w.phase, Phase::Setup | Phase::Playing)
        {
            cues.push(Cue::Spawn);
        }
        if let Some(cue) = cues.into_iter().max_by_key(|cue| cue.priority()) {
            audio.play(g, hero, cue, slot, None, listener, volume);
        }
        if !rt.deathcam.is_replay() {
            for event in events.iter().filter(|e| e.position.is_some()) {
                let (Some(speaker), Some(cue)) = (event.speaker, Cue::from_sound(event.cue)) else {
                    continue;
                };
                if matches!(cue, Cue::Skill | Cue::Ultimate) {
                    if let Some(actor) = w.actors.get(speaker).filter(|a| a.active && a.alive()) {
                        audio.play(g, actor.character, cue, speaker, event.position, listener, volume);
                    }
                }
            }
        }
        {
            // Typed text is never interpreted as a cue. Only the server's marker plays;
            // monotonically consumed IDs avoid retries/history/mute bursts.
            for message in messages {
                let Some(actor) = w.actors.get(message.sender) else {
                    continue;
                };
                if actor.active
                    && actor.alive()
                    && actor.character == message.character
                    && team(message.sender) == team(slot)
                {
                    audio.play(
                        g,
                        message.character,
                        Cue::Line,
                        message.sender,
                        (message.sender != slot).then_some(actor.eye()),
                        listener,
                        volume,
                    );
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct Review {
    time: f32,
    next: usize,
    advanced: bool,
}
fn review(
    audio: &mut Audio,
    g: &mut Graph,
    dt: f32,
    control: &mut fyrox::engine::ApplicationLoopController<'_>,
) {
    let mut state = audio.review.take().unwrap();
    state.time += dt;
    audio.time += dt;
    audio.focused = true;
    audio.maintain(g, 0.6);
    if let Some(p) = audio.playing.first() {
        if g[p.node]
            .cast::<Sound>()
            .is_some_and(|s| s.playback_time() > 0.05)
        {
            state.advanced = true;
        }
    }
    if state.next < 45 && state.time >= 0.5 + state.next as f32 * 1.0 {
        audio.stop(g);
        audio.play(
            g,
            (state.next / 3) as u8,
            [Cue::Select, Cue::Skill, Cue::Ultimate][state.next % 3],
            0,
            None,
            V3::default(),
            0.6,
        );
        assert_eq!(audio.playing.len(), 1);
        state.next += 1;
    }
    if state.time > 47.0 {
        assert!(state.advanced, "Voice device playback did not advance");
        audio.maintain(g, 0.0);
        assert!(audio.playing.is_empty());
        audio.focus(false);
        audio.play(g, 0, Cue::Line, 0, None, V3::default(), 0.6);
        assert!(audio.playing.is_empty());
        println!("CHARACTER_VOICE_REVIEW_OK: 15 heroes, selection/E/Q device playback, mute and focus gates");
        control.exit();
    }
    audio.review = Some(state);
}

#[cfg(test)]
mod tests;
