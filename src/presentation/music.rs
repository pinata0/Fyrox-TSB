//! Screen music is streamed from embedded Vorbis files, independently of combat SFX.
use super::*;
use fyrox::scene::sound::{
    DataSource, Sound, SoundBuffer, SoundBufferResource, SoundBuilder, Status,
};
use std::io::Cursor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Track {
    Main,
    Lobby,
    Selection,
    Battle,
}
impl Track {
    fn screen(home: bool, lobby: bool, selecting: bool) -> Self {
        if home {
            Self::Main
        } else if lobby {
            Self::Lobby
        } else if selecting {
            Self::Selection
        } else {
            Self::Battle
        }
    }
    fn asset(self) -> (&'static str, &'static [u8]) {
        match self {
            Self::Main => (
                "Trickal Main Title",
                include_bytes!("../../assets/audio/music/main_title.ogg"),
            ),
            Self::Lobby => (
                "Day Life",
                include_bytes!("../../assets/audio/music/day_life.ogg"),
            ),
            Self::Selection => (
                "Lounge",
                include_bytes!("../../assets/audio/music/lounge.ogg"),
            ),
            Self::Battle => (
                "Adventure Begins",
                include_bytes!("../../assets/audio/music/adventure_begins.ogg"),
            ),
        }
    }
}

#[derive(Debug)]
pub(super) struct Music {
    current: Option<Track>,
    node: Handle<Node>,
    fade: f32,
    resume: f32,
    focused: bool,
    failed: Option<Track>,
    review: Option<Review>,
}
impl Default for Music {
    fn default() -> Self {
        Self {
            current: None,
            node: Handle::NONE,
            fade: 0.0,
            resume: 0.0,
            focused: true,
            failed: None,
            review: std::env::args()
                .any(|a| a == "--music-review")
                .then(Review::default),
        }
    }
}
impl Music {
    pub(super) fn focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Keep the playhead when a map replaces its scene graph.
    pub(super) fn detach(&mut self, g: &mut Graph) {
        if let Ok(node) = g.try_get(self.node) {
            if let Some(sound) = node.cast::<Sound>() {
                self.resume = sound.playback_time();
            }
            g.remove_node(self.node);
        }
        self.node = Handle::NONE;
        self.fade = 0.0;
    }

    fn tick(&mut self, g: &mut Graph, wanted: Track, volume: f32, dt: f32) {
        let audible = self.focused && volume > 0.0;
        let dt = dt.clamp(0.0, 0.1);
        if self.current != Some(wanted) {
            // Fade out completely before replacing the source; never overlap songs.
            self.fade = if audible {
                (self.fade - dt / 0.25).max(0.0)
            } else {
                0.0
            };
            if self.fade == 0.0 {
                if g.is_valid_handle(self.node) {
                    g.remove_node(self.node);
                }
                self.node = Handle::NONE;
                self.current = Some(wanted);
                self.resume = 0.0;
                self.failed = None;
            }
        } else if audible {
            self.fade = (self.fade + dt / 0.4).min(1.0);
        }
        if !g.is_valid_handle(self.node) && self.failed != self.current {
            if let Some(track) = self.current {
                let (name, bytes) = track.asset();
                // Only a small decoded chunk is resident, not four full PCM songs.
                match SoundBuffer::raw_streaming(DataSource::Memory(Cursor::new(bytes.to_vec()))) {
                    Ok(buffer) => {
                        let node: Handle<Node> =
                            SoundBuilder::new(BaseBuilder::new().with_name(format!("BGM: {name}")))
                                .with_buffer(Some(SoundBufferResource::new_embedded(buffer)))
                                .with_status(Status::Paused)
                                .with_looping(true)
                                .with_play_once(false)
                                .with_spatial_blend_factor(0.0)
                                .with_gain(0.0)
                                .build(g)
                                .to_base();
                        self.node = node;
                        if self.resume > 0.0 {
                            g[node]
                                .cast_mut::<Sound>()
                                .unwrap()
                                .set_playback_time(self.resume);
                        }
                        println!("BGM_TRACK: {name}");
                    }
                    Err(error) => {
                        eprintln!("BGM_LOAD_ERROR: {name}: {error:?}");
                        self.failed = Some(track);
                    }
                }
            }
        }
        if let Ok(node) = g.try_get_mut(self.node) {
            if let Some(sound) = node.cast_mut::<Sound>() {
                sound.set_gain(if audible {
                    volume.clamp(0.0, 1.0) * self.fade
                } else {
                    0.0
                });
                sound.set_status(if audible {
                    Status::Playing
                } else {
                    Status::Paused
                });
            }
        }
    }
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) {
    if rt.music.review.is_some() {
        review(rt, context);
        return;
    }
    // Use the underlying selection state: Tab, settings, chat and help overlays do
    // not change songs or restart the current song.
    let track = Track::screen(rt.home_open, rt.lobby.open, rt.selecting);
    rt.music.tick(
        &mut context.scenes[rt.scene].graph,
        track,
        rt.settings_menu.settings.music_volume,
        context.dt,
    );
}

#[derive(Debug, Default)]
struct Review {
    time: f32,
    heard: [bool; 4],
    checked_mute: bool,
    checked_focus: bool,
    loop_seek: bool,
    looped: bool,
}

fn review(rt: &mut Runtime, context: &mut PluginContext) {
    let mut state = rt.music.review.take().unwrap();
    state.time += context.dt;
    let stage = (state.time / 3.0) as usize;
    rt.home_open = stage == 0;
    rt.lobby.open = stage == 1;
    rt.selecting = stage == 2;
    let track = Track::screen(rt.home_open, rt.lobby.open, rt.selecting);
    rt.music.focus(stage != 5);
    let volume = if stage == 4 { 0.0 } else { 0.35 };
    let g = &mut context.scenes[rt.scene].graph;
    rt.music.tick(g, track, volume, context.dt);
    let sound = g[rt.music.node].cast_mut::<Sound>().expect("BGM source");
    if stage < 4 && rt.music.current == Some(track) && sound.playback_time() > 0.1 {
        state.heard[stage] = true;
        assert!(sound.is_looping());
    }
    if stage == 4 {
        assert_eq!(sound.gain(), 0.0);
        assert_eq!(sound.status(), Status::Paused);
        state.checked_mute = true;
    }
    if stage == 5 {
        assert_eq!(sound.status(), Status::Paused);
        state.checked_focus = true;
    }
    if state.time > 19.0 && !state.loop_seek {
        let duration = sound.buffer().unwrap().data_ref().duration().as_secs_f32();
        assert!(duration > 120.0);
        sound.set_playback_time(duration - 0.25);
        state.loop_seek = true;
    }
    if state.time > 20.0 && sound.playback_time() < 5.0 {
        state.looped = true;
    }
    if state.time > 22.0 {
        assert!(state.heard.iter().all(|&heard| heard));
        assert!(state.checked_mute && state.checked_focus);
        assert!(state.looped, "The streaming track must rewind at its end");
        assert_eq!(sound.status(), Status::Playing);
        println!("MUSIC_REVIEW_OK: all four screen tracks advanced on the audio device; end-of-track loop, mute, focus pause and resume verified");
        context.loop_controller.exit();
    }
    rt.music.review = Some(state);
}

#[cfg(test)]
mod tests;
