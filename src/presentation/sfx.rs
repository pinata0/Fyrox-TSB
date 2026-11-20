//! Embedded CC0 clips, decoded once; bounded one-shot sources use Fyrox's audio device.
use super::*;
use fyrox::scene::sound::{
    listener::ListenerBuilder, DataSource, Sound, SoundBuffer, SoundBufferResource, SoundBuilder,
    Status,
};
use std::io::Cursor;
use trickcal_shooting_balddagu::sfx::{Cue, Event as SoundEvent, Tracker};

macro_rules! clips {
    ($($name:literal),* $(,)?) => { &[$(($name, include_bytes!(concat!("../../assets/audio/sfx/", $name, ".ogg")) as &[u8])),*, ("sebastian-roar", include_bytes!("../../assets/audio/sfx/sebastian-roar.wav") as &[u8])] };
}
const CLIPS: &[(&str, &[u8])] = clips![
    "click",
    "confirm",
    "back",
    "pluck",
    "heal",
    "ready",
    "capture",
    "win",
    "lose",
    "death",
    "respawn",
    "hit",
    "hurt",
    "wall",
    "blunt",
    "shield",
    "step0",
    "step1",
    "land",
    "slash",
    "dart",
    "reload",
    "reload_done",
    "dash",
    "jump",
    "water",
    "magic",
    "laser",
    "rifle",
    "skill",
    "ultimate",
    "portal",
];
const MAX_VOICES: usize = 24;
const WORLD_VOICES: usize = 18; // Reserve room for local hit/reload/UI feedback.

#[derive(Debug)]
struct Voice {
    node: Handle<Node>,
    gain: f32,
    spatial: bool,
}
#[derive(Debug)]
pub(super) struct Audio {
    buffers: Vec<Option<SoundBufferResource>>,
    tracker: Tracker,
    voices: Vec<Voice>,
    focused: bool,
    ui: Option<(u8, usize, u8, f32)>,
    review: Option<Review>,
}
impl Default for Audio {
    fn default() -> Self {
        let buffers = CLIPS
            .iter()
            .map(|(name, data)| match decode(data) {
                Ok(buffer) => Some(buffer),
                Err(error) => {
                    eprintln!("SFX_LOAD_ERROR: {name}: {error}");
                    None
                }
            })
            .collect::<Vec<_>>();
        println!(
            "SFX_READY: {}/{} embedded CC0 clips",
            buffers.iter().flatten().count(),
            CLIPS.len()
        );
        Self {
            buffers,
            tracker: Tracker::default(),
            voices: vec![],
            focused: true,
            ui: None,
            review: std::env::args()
                .any(|a| a == "--sfx-review")
                .then(Review::default),
        }
    }
}

fn decode(data: &[u8]) -> Result<SoundBufferResource, String> {
    let decoded = SoundBuffer::raw_generic(DataSource::Memory(Cursor::new(data.to_vec())))
        .map_err(|e| format!("{e:?}"))?;
    let rate = decoded.sample_rate();
    // Mono gives consistent positional sound even for stereo source assets. Cap tails,
    // normalize each clip and fade its edges so repeated cues don't click or pile up.
    let mut samples: Vec<f32> = decoded
        .samples()
        .chunks_exact(decoded.channel_count())
        .take(rate * 2)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect();
    let peak = samples.iter().fold(0.0_f32, |p, v| p.max(v.abs()));
    if rate == 0 || samples.is_empty() || peak < 0.0001 || samples.iter().any(|v| !v.is_finite()) {
        return Err("Empty or invalid clip".into());
    }
    let length = samples.len();
    let fade = (rate / 200).max(1) as f32;
    for (i, value) in samples.iter_mut().enumerate() {
        *value *=
            (0.8 / peak) * (i as f32 / fade).min(1.0) * ((length - 1 - i) as f32 / fade).min(1.0);
    }
    SoundBuffer::raw_generic(DataSource::Raw {
        sample_rate: rate,
        channel_count: 1,
        samples,
    })
    .map(SoundBufferResource::new_embedded)
    .map_err(|e| format!("{e:?}"))
}

/// Character families share a palette, with different pitch/weight per weapon.
fn clip(cue: Cue) -> (&'static str, f32, f64) {
    use Cue::*;
    match cue {
        SebastianRoar => ("sebastian-roar", 0.55, 1.10),
        Fire(id) => match id {
            0 => ("slash", 0.32, 0.9),
            1 => ("water", 0.28, 1.35),
            2 | 5 => ("magic", 0.27, if id == 2 { 1.1 } else { 1.4 }),
            3 => ("laser", 0.26, 1.25),
            4 => ("slash", 0.26, 1.2),
            6 | 12 => ("blunt", 0.24, if id == 6 { 1.1 } else { 0.9 }),
            7 => ("laser", 0.22, 1.5),
            8 | 13 => ("dart", 0.30, 1.4),
            9 => ("magic", 0.25, 0.7),
            10 => ("heal", 0.23, 1.3),
            11 => ("pluck", 0.32, 0.7),
            _ => ("rifle", 0.38, 0.65),
        },
        Skill(1) => ("water", 0.34, 0.9),
        Skill(5 | 10 | 13) => ("heal", 0.30, 1.0),
        Skill(8 | 11 | 14) => ("reload_done", 0.30, 0.8),
        Skill(id) => ("skill", 0.28, 0.9 + (id % 4) as f64 * 0.08),
        Ultimate(1) => ("water", 0.45, 0.65),
        Ultimate(5 | 10 | 13) => ("heal", 0.42, 0.65),
        Ultimate(_) => ("ultimate", 0.40, 1.0),
        Step(0) => ("step0", 0.19, 1.0),
        Step(_) => ("step1", 0.19, 1.0),
        Jump => ("jump", 0.19, 1.2),
        Land => ("land", 0.27, 0.85),
        Dash => ("dash", 0.32, 0.8),
        Portal => ("portal", 0.24, 1.2),
        Hit => ("hit", 0.24, 1.7),
        Hurt => ("hurt", 0.32, 0.85),
        ShieldBreak => ("shield", 0.30, 1.1),
        Wall => ("wall", 0.18, 1.2),
        SlashImpact => ("blunt", 0.23, 1.1),
        Reload => ("reload", 0.28, 1.0),
        ReloadDone => ("reload_done", 0.25, 1.0),
        Death => ("death", 0.28, 0.8),
        Respawn => ("respawn", 0.28, 1.0),
        Ready => ("ready", 0.28, 1.0),
        RoundStart => ("confirm", 0.35, 0.9),
        Capture => ("capture", 0.32, 1.0),
        Win => ("win", 0.38, 1.0),
        Lose => ("lose", 0.30, 0.85),
        Click => ("click", 0.23, 1.0),
        Confirm => ("confirm", 0.28, 1.0),
        Back => ("back", 0.23, 1.0),
    }
}

impl Audio {
    pub(super) fn attach(&mut self, g: &mut Graph, camera: Handle<Node>) {
        self.voices.clear(); // Handles belong to the old scene after a map swap.
        self.tracker = Tracker::default();
        let listener: Handle<Node> =
            ListenerBuilder::new(BaseBuilder::new().with_name("SFX listener"))
                .build(g)
                .to_base();
        g.link_nodes(listener, camera);
    }
    pub(super) fn focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn stop(&mut self, g: &mut Graph) {
        for voice in self.voices.drain(..) {
            if g.is_valid_handle(voice.node) {
                g.remove_node(voice.node);
            }
        }
    }

    fn play(&mut self, g: &mut Graph, event: SoundEvent, volume: f32, listener: V3) {
        if volume <= 0.0 || !self.focused {
            return;
        }
        let spatial = event.position.is_some();
        if event
            .position
            .is_some_and(|p| (p - listener).length() > 48.0)
        {
            return;
        }
        if spatial && self.voices.iter().filter(|v| v.spatial).count() >= WORLD_VOICES {
            return;
        }
        if self.voices.len() >= MAX_VOICES {
            if spatial {
                return;
            }
            let index = self.voices.iter().position(|v| v.spatial).unwrap_or(0);
            let oldest = self.voices.remove(index);
            if g.is_valid_handle(oldest.node) {
                g.remove_node(oldest.node);
            }
        }
        let (name, gain, pitch) = clip(event.cue);
        let index = CLIPS
            .iter()
            .position(|(n, _)| *n == name)
            .expect("known sound clip");
        let Some(buffer) = self.buffers[index].as_ref() else {
            return;
        };
        let variation = if matches!(
            event.cue,
            Cue::Fire(_) | Cue::Step(_) | Cue::Wall | Cue::SlashImpact
        ) {
            0.97 + (event.variation % 7) as f64 * 0.01
        } else {
            1.0
        };
        let gain = gain * event.gain;
        let node = SoundBuilder::new(
            BaseBuilder::new()
                .with_name(format!("SFX {name}"))
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(vec(event.position.unwrap_or(listener)))
                        .build(),
                ),
        )
        .with_buffer(Some(buffer.clone()))
        .with_status(Status::Playing)
        .with_play_once(true)
        .with_gain(gain * volume)
        .with_pitch(pitch * variation)
        .with_spatial_blend_factor(if spatial { 1.0 } else { 0.0 })
        .with_radius(3.0)
        .with_max_distance(48.0)
        .with_rolloff_factor(1.5)
        .build(g)
        .to_base();
        self.voices.push(Voice {
            node,
            gain,
            spatial,
        });
    }
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) {
    let w = rt.session.world();
    let slot = rt.session.slot();
    let layer = u8::from(rt.home_open)
        | u8::from(rt.lobby.open) << 1
        | u8::from(selection::showing(rt)) << 2
        | u8::from(rt.settings_menu.open) << 3;
    let volume = rt.settings_menu.settings.sfx_volume;
    let ui = (layer, rt.selected, w.actors[slot].character, volume);
    let sound = &mut rt.sfx;
    if sound.review.is_some() {
        review(
            sound,
            &mut context.scenes[rt.scene].graph,
            context.dt,
            &mut context.loop_controller,
        );
        return;
    }
    let mut events = sound.tracker.update(
        w,
        slot,
        sound.focused && layer == 0 && volume > 0.0 && rt.error.is_none(),
    );
    let previous_ui = sound.ui.replace(ui);
    if let Some(previous) = previous_ui {
        if sound.focused {
            if previous.2 != ui.2 {
                events.push(SoundEvent::local(Cue::Confirm));
            } else if previous.0 != layer {
                events.push(SoundEvent::local(if layer == 0 {
                    Cue::Back
                } else {
                    Cue::Click
                }));
            } else if (layer != 0 && previous.1 != ui.1) || previous.3 != volume {
                events.push(SoundEvent::local(Cue::Click));
            }
        }
    }
    let g = &mut context.scenes[rt.scene].graph;
    sound.voices.retain(|v| g.is_valid_handle(v.node));
    if !sound.focused || volume <= 0.0 {
        sound.stop(g);
        return;
    }
    // Silence lingering combat immediately when entering menus or a death replay.
    if layer != 0 || rt.deathcam.is_replay() {
        if previous_ui.is_some_and(|previous| previous.0 == 0) {
            sound.stop(g);
        }
        events.retain(|e| matches!(e.cue, Cue::Click | Cue::Confirm | Cue::Back));
    }
    for voice in &sound.voices {
        if let Some(node) = g
            .try_get_mut(voice.node)
            .ok()
            .and_then(|n| n.cast_mut::<Sound>())
        {
            node.set_gain(voice.gain * volume);
        }
    }
    let listener = w.actors[slot].eye();
    for event in events {
        sound.play(g, event, volume, listener);
    }
}

#[derive(Debug, Default)]
struct Review {
    time: f32,
    next: usize,
    advanced: usize,
    expired: usize,
}

// Real device/scene smoke test. Uses the normal player and verifies its playback clock,
// source expiry and silence gate; no microphone capture and no user settings are changed.
fn review(
    audio: &mut Audio,
    g: &mut Graph,
    dt: f32,
    control: &mut fyrox::engine::ApplicationLoopController<'_>,
) {
    let mut state = audio.review.take().unwrap();
    audio.focused = true;
    state.time += dt;
    for voice in &audio.voices {
        if let Ok(node) = g.try_get(voice.node) {
            if node
                .cast::<Sound>()
                .is_some_and(|s| s.playback_time() > 0.005)
            {
                state.advanced += 1;
            }
        } else {
            state.expired += 1;
        }
    }
    audio.voices.retain(|v| g.is_valid_handle(v.node));
    let cues: Vec<Cue> = (0..15)
        .map(Cue::Fire)
        .chain([
            Cue::Hit,
            Cue::Hurt,
            Cue::Reload,
            Cue::ReloadDone,
            Cue::Skill(2),
            Cue::Ultimate(2),
            Cue::Step(0),
            Cue::Step(1),
            Cue::Jump,
            Cue::Land,
            Cue::Dash,
            Cue::Portal,
            Cue::ShieldBreak,
            Cue::Death,
            Cue::Respawn,
            Cue::Ready,
            Cue::RoundStart,
            Cue::Capture,
            Cue::Win,
            Cue::Lose,
            Cue::Click,
            Cue::Confirm,
            Cue::Back,
        ])
        .collect();
    if state.next < cues.len() && state.time > 0.5 + state.next as f32 * 0.22 {
        let event = SoundEvent::local(cues[state.next]);
        audio.play(g, event, 0.65, V3::default());
        state.next += 1;
    }
    if state.time > 12.0 {
        assert!(
            state.advanced > 0,
            "SFX audio device did not advance playback"
        );
        assert!(
            state.expired > 0,
            "Finished SFX sources were not cleaned up"
        );
        assert_eq!(state.next, cues.len());
        audio.stop(g);
        audio.play(g, SoundEvent::local(Cue::Fire(14)), 0.0, V3::default());
        assert!(audio.voices.is_empty());
        println!(
            "SFX_REVIEW_OK: {} cues, {} playback observations, {} expired sources; mute verified",
            state.next, state.advanced, state.expired
        );
        control.exit();
    }
    audio.review = Some(state);
}

#[cfg(test)]
mod tests;
