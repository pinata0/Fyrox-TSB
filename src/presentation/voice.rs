//! Device streams live on a worker: device startup cannot stall gameplay.
pub(super) mod review;
use super::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SizedSample,
};
use fyrox::{
    asset::manager::ResourceManager,
    core::algebra::Matrix3,
    gui::{
        canvas::CanvasBuilder,
        formatted_text::WrapMode,
        image::{ImageBuilder, ImageMessage},
        vector_image::{Primitive, VectorImageBuilder},
    },
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    thread,
    time::Duration,
};
use trickcal_shooting_balddagu::voice::{
    Capture, MicState, Samples, VoiceFrame, VoiceMixer, SAMPLE_RATE,
};

struct Captured {
    generation: u64,
    time: Instant,
    samples: Samples,
}
enum Status {
    Input(u64, Result<(), String>),
    Output(Result<(), String>),
}

pub(super) struct VoiceAudio {
    generation: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    captured: Receiver<Captured>,
    incoming: SyncSender<(Instant, VoiceFrame)>,
    status: Receiver<Status>,
    worker: Option<thread::JoinHandle<()>>,
    pub input_ready: bool,
    pub input_error: Option<String>,
    pub output_error: Option<String>,
}
impl std::fmt::Debug for VoiceAudio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceAudio").finish_non_exhaustive()
    }
}
impl Drop for VoiceAudio {
    fn drop(&mut self) {
        self.generation.fetch_and(!1, Ordering::SeqCst);
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl VoiceAudio {
    fn new() -> Result<Self, String> {
        let generation = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (capture_tx, captured) = mpsc::sync_channel(4);
        let (incoming, receive) = mpsc::sync_channel(36);
        let (status_tx, status) = mpsc::sync_channel(16);
        let worker_generation = generation.clone();
        let worker_stop = stop.clone();
        let worker = thread::Builder::new()
            .name("team-voice".into())
            .spawn(move || {
                let host = cpal::default_host();
                let output = output_stream(&host, receive, status_tx.clone());
                let _ = status_tx.try_send(Status::Output(
                    output.as_ref().map(|_| ()).map_err(Clone::clone),
                ));
                let _output = output.ok(); // Listening is independent of the local microphone.
                let mut input = None;
                let mut previous = 0;
                while !worker_stop.load(Ordering::SeqCst) {
                    let current = worker_generation.load(Ordering::SeqCst);
                    if current != previous {
                        input.take();
                        if current & 1 != 0 {
                            let result = input_stream(
                                &host,
                                current,
                                worker_generation.clone(),
                                capture_tx.clone(),
                                status_tx.clone(),
                            );
                            let _ = status_tx.try_send(Status::Input(
                                current,
                                result.as_ref().map(|_| ()).map_err(Clone::clone),
                            ));
                            input = result.ok();
                        }
                        previous = current;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            generation,
            stop,
            captured,
            incoming,
            status,
            worker: Some(worker),
            input_ready: false,
            input_error: None,
            output_error: None,
        })
    }
    fn set_enabled(&mut self, enabled: bool) {
        let previous = self.generation.load(Ordering::SeqCst);
        if (previous & 1 != 0) != enabled {
            self.generation
                .store(((previous + 2) & !1) | u64::from(enabled), Ordering::SeqCst);
            self.input_ready = false;
            if enabled {
                self.input_error = None;
            }
            // A mute/unmute must never transmit queued audio from before the toggle.
            while self.captured.try_recv().is_ok() {}
        }
    }
    fn poll(&mut self) -> Vec<Samples> {
        let current = self.generation.load(Ordering::SeqCst);
        for status in self.status.try_iter() {
            match status {
                Status::Input(generation, result) if generation == current => {
                    self.input_ready = result.is_ok();
                    self.input_error = result.err();
                    if let Some(error) = &self.input_error {
                        eprintln!("VOICE_INPUT: {error}");
                    }
                }
                Status::Output(result) => {
                    self.output_error = result.err();
                    if let Some(error) = &self.output_error {
                        eprintln!("VOICE_OUTPUT: {error}");
                    }
                }
                _ => {}
            }
        }
        self.captured
            .try_iter()
            .filter(|frame| {
                current & 1 != 0
                    && self.input_ready
                    && frame.generation == current
                    && frame.time.elapsed() < Duration::from_millis(100)
            })
            .map(|frame| frame.samples)
            .collect()
    }
}

fn input_stream(
    host: &cpal::Host,
    generation: u64,
    gate: Arc<AtomicU64>,
    tx: SyncSender<Captured>,
    status: SyncSender<Status>,
) -> Result<cpal::Stream, String> {
    let device = host.default_input_device().ok_or("No default microphone")?;
    let config = device.default_input_config().map_err(|e| e.to_string())?;
    macro_rules! build {
        ($t:ty) => {
            build_input::<$t>(&device, &config.into(), generation, gate, tx, status)
        };
    }
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build!(f32),
        cpal::SampleFormat::F64 => build!(f64),
        cpal::SampleFormat::I16 => build!(i16),
        cpal::SampleFormat::U16 => build!(u16),
        cpal::SampleFormat::I32 => build!(i32),
        cpal::SampleFormat::U32 => build!(u32),
        cpal::SampleFormat::I8 => build!(i8),
        cpal::SampleFormat::U8 => build!(u8),
        cpal::SampleFormat::I64 => build!(i64),
        cpal::SampleFormat::U64 => build!(u64),
        format => return Err(format!("Unsupported microphone format: {format}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}
fn build_input<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    generation: u64,
    gate: Arc<AtomicU64>,
    tx: SyncSender<Captured>,
    status: SyncSender<Status>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let mut capture = Capture::new(config.sample_rate.0);
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            if gate.load(Ordering::SeqCst) != generation {
                return;
            }
            for frame in data.chunks_exact(channels) {
                let mono =
                    frame.iter().map(|&s| f32::from_sample(s)).sum::<f32>() / channels as f32;
                capture.push(mono, |samples| {
                    let _ = tx.try_send(Captured {
                        generation,
                        time: Instant::now(),
                        samples,
                    });
                });
            }
        },
        move |error| {
            let _ = status.try_send(Status::Input(generation, Err(error.to_string())));
        },
        None,
    )
}

fn output_stream(
    host: &cpal::Host,
    receive: Receiver<(Instant, VoiceFrame)>,
    status: SyncSender<Status>,
) -> Result<cpal::Stream, String> {
    let device = host.default_output_device().ok_or("No default speakers")?;
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    macro_rules! build {
        ($t:ty) => {
            build_output::<$t>(&device, &config.into(), receive, status)
        };
    }
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build!(f32),
        cpal::SampleFormat::F64 => build!(f64),
        cpal::SampleFormat::I16 => build!(i16),
        cpal::SampleFormat::U16 => build!(u16),
        cpal::SampleFormat::I32 => build!(i32),
        cpal::SampleFormat::U32 => build!(u32),
        cpal::SampleFormat::I8 => build!(i8),
        cpal::SampleFormat::U8 => build!(u8),
        cpal::SampleFormat::I64 => build!(i64),
        cpal::SampleFormat::U64 => build!(u64),
        format => return Err(format!("Unsupported speaker format: {format}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}
fn build_output<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    receive: Receiver<(Instant, VoiceFrame)>,
    status: SyncSender<Status>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let step = SAMPLE_RATE as f64 / config.sample_rate.0 as f64;
    let mut phase = 0.0;
    let mut left = 0.0;
    let mut right = 0.0;
    let mut mixer = VoiceMixer::default();
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            let now = Instant::now();
            for (time, frame) in receive.try_iter().take(36) {
                if now.duration_since(time) < Duration::from_millis(150) {
                    mixer.push(frame, now);
                }
            }
            for frame in data.chunks_mut(channels) {
                while phase >= 1.0 {
                    left = right;
                    right = mixer.next_sample(now);
                    phase -= 1.0;
                }
                let value = left + (right - left) * phase as f32;
                frame.fill(T::from_sample(value));
                phase += step;
            }
        },
        move |error| {
            let _ = status.try_send(Status::Output(Err(error.to_string())));
        },
        None,
    )
}

#[derive(Debug, Default)]
pub(super) struct Voice {
    pub mic: MicState,
    audio: Option<VoiceAudio>,
    startup_error: Option<String>,
    speaking_until: [Option<Instant>; PLAYERS],
}
impl Voice {
    pub fn key(&mut self, down: bool, repeat: bool, typing: bool) {
        if self.mic.toggle_key(down, repeat, typing) && self.mic.muted {
            if let Some(audio) = &mut self.audio {
                audio.set_enabled(false);
            }
        }
    }
    pub fn focus(&mut self, focused: bool) {
        self.mic.focused = focused;
        if !focused {
            if let Some(audio) = &mut self.audio {
                audio.set_enabled(false);
            }
        }
    }
    fn connected(session: &Session) -> bool {
        match session {
            Session::Solo(_) => false,
            Session::Host(_) => true,
            Session::Join(client) => client.voice_connected(),
        }
    }
    pub fn update(&mut self, session: &mut Session, in_home: bool, game_error: bool) {
        let connected = Self::connected(session) && !game_error;
        let receive = match session {
            Session::Solo(_) => vec![],
            Session::Host(s) => s.voice.drain(),
            Session::Join(c) => c.voice.drain(),
        };
        if connected && !in_home && self.audio.is_none() && self.startup_error.is_none() {
            match VoiceAudio::new() {
                Ok(audio) => self.audio = Some(audio),
                Err(error) => self.startup_error = Some(error),
            }
        }
        let Some(audio) = &mut self.audio else {
            return;
        };
        audio.set_enabled(self.mic.transmitting(connected, in_home));
        let frames = audio.poll();
        if audio.input_error.is_some() && !self.mic.muted {
            self.mic.muted = true;
            audio.set_enabled(false);
        }
        if self.mic.transmitting(connected, in_home) {
            for samples in frames {
                if audible(&samples) {
                    self.speaking_until[session.slot()] =
                        Some(Instant::now() + Duration::from_millis(180));
                }
                match session {
                    Session::Host(s) => s.send_voice(samples),
                    Session::Join(c) => {
                        let _ = c.send_voice(samples);
                    }
                    Session::Solo(_) => {}
                }
            }
        }
        if connected && !in_home {
            for frame in receive {
                if audible(&frame.samples) {
                    self.speaking_until[frame.sender] =
                        Some(Instant::now() + Duration::from_millis(180));
                }
                let _ = audio.incoming.try_send((Instant::now(), frame));
            }
        }
    }
    fn label(&self, session: &Session, in_home: bool) -> (String, [u8; 3]) {
        if matches!(session, Session::Solo(_)) {
            return ("팀 보이스 · 연습 모드".into(), CREAM);
        }
        if in_home {
            return ("팀 보이스 · 플레이에서 연결".into(), CREAM);
        }
        if !Self::connected(session) {
            return ("팀 보이스 · 연결 대기".into(), CREAM);
        }
        if self.startup_error.is_some() {
            return ("팀 보이스 · 오디오 시작 실패".into(), PINK);
        }
        let mut state = if self.audio.as_ref().is_some_and(|a| a.input_error.is_some()) {
            "마이크 확인 필요 · M 재시도"
        } else if self.mic.muted {
            "마이크 꺼짐 · M 켜기"
        } else if !self.mic.focused {
            "마이크 일시 중지"
        } else if self.audio.as_ref().is_some_and(|a| a.input_ready) {
            "마이크 켜짐 · M 끄기"
        } else {
            "마이크 연결 중 · M 취소"
        }
        .to_owned();
        if self
            .audio
            .as_ref()
            .is_some_and(|a| a.output_error.is_some())
        {
            state.push_str("\n출력 장치 확인 · 게임 재실행");
        }
        (
            format!("팀 보이스 | {state}"),
            if self.mic.muted { PINK } else { MINT },
        )
    }
}
fn audible(samples: &Samples) -> bool {
    samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / samples.len() as f64
        > 350.0_f64.powi(2)
}

const VOICE_CYAN: [u8; 3] = [55, 220, 225];
const VOICE_RED: [u8; 3] = [255, 73, 86];
const VOICE_DIM: [u8; 3] = [119, 154, 163];
const ROW_HEIGHT: f32 = 32.0;
const WIDTH: f32 = 280.0;
const HEIGHT: f32 = TEAM_SIZE as f32 * ROW_HEIGHT + 36.0;

// The local entry never depends on audio activity or its position in the team.
fn roster_slots(world: &World, local: usize) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(local).chain((0..PLAYERS).filter(move |&slot| {
        slot != local
            && team(slot) == team(local)
            && world.actors[slot].active
            && world.actors[slot].human
    }))
}

#[derive(Debug, Default)]
struct VoiceRow {
    root: Handle<UiNode>,
    frame: Handle<UiNode>,
    portrait: Handle<UiNode>,
    speaker: Handle<UiNode>,
    waves: Handle<UiNode>,
    muted: Handle<UiNode>,
    name: Handle<UiNode>,
}

#[derive(Debug, Default)]
pub(super) struct VoiceUi {
    root: Handle<UiNode>,
    status: Handle<UiNode>,
    rows: [VoiceRow; TEAM_SIZE],
    textures: Vec<TextureResource>,
}

fn vector(ctx: &mut BuildContext, primitives: Vec<Primitive>) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(VOICE_CYAN)).into()),
    )
    .with_primitives(primitives)
    .build(ctx)
    .to_base()
}

fn line(a: (f32, f32), b: (f32, f32)) -> Primitive {
    Primitive::Line {
        begin: Vector2::new(a.0, a.1),
        end: Vector2::new(b.0, b.1),
        thickness: 2.0,
    }
}

impl VoiceUi {
    pub fn new(ui: &mut UserInterface, manager: &ResourceManager) -> Self {
        let textures = portraits::load(manager);
        let ctx = &mut ui.build_ctx();
        let rows: [VoiceRow; TEAM_SIZE] = std::array::from_fn(|index| {
            let frame = BorderBuilder::new(
                WidgetBuilder::new()
                    .with_hit_test_visibility(false)
                    .with_background(Brush::Solid(Color::from_rgba(12, 28, 36, 220)).into())
                    .with_foreground(Brush::Solid(rgb(VOICE_CYAN)).into()),
            )
            .with_stroke_thickness(fyrox::gui::Thickness::uniform(1.0).into())
            .build(ctx)
            .to_base();
            let portrait = ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
                .with_texture(textures[0].clone())
                .with_sync_with_texture_size(false)
                .build(ctx)
                .to_base();
            // A solid speaker silhouette, independent of font/emoji support.
            let points = [
                (1.0, 7.0),
                (6.0, 7.0),
                (12.0, 2.0),
                (12.0, 22.0),
                (6.0, 17.0),
                (1.0, 17.0),
            ]
            .map(|(x, y)| Vector2::new(x, y));
            let speaker = vector(
                ctx,
                (1..points.len() - 1)
                    .map(|i| Primitive::Triangle {
                        points: [points[0], points[i], points[i + 1]],
                    })
                    .collect(),
            );
            let waves = vector(
                ctx,
                vec![
                    line((15.0, 8.0), (17.0, 12.0)),
                    line((17.0, 12.0), (15.0, 16.0)),
                    line((19.0, 5.0), (22.0, 12.0)),
                    line((22.0, 12.0), (19.0, 19.0)),
                ],
            );
            let muted = vector(
                ctx,
                vec![
                    line((15.0, 8.0), (23.0, 16.0)),
                    line((23.0, 8.0), (15.0, 16.0)),
                ],
            );
            let name = TextBuilder::new(
                WidgetBuilder::new()
                    .with_hit_test_visibility(false)
                    .with_foreground(Brush::Solid(rgb(VOICE_CYAN)).into()),
            )
            .with_font_size(15.0.into())
            .with_wrap(WrapMode::NoWrap)
            .with_trim_text(true)
            .with_shadow(true)
            .build(ctx)
            .to_base();
            let root = CanvasBuilder::new(
                WidgetBuilder::new()
                    .with_width(WIDTH)
                    .with_height(ROW_HEIGHT)
                    .with_hit_test_visibility(false)
                    .with_children([frame, portrait, speaker, waves, muted, name]),
            )
            .build(ctx)
            .to_base();
            for (node, x, y, w, h) in [
                (root, 0.0, index as f32 * ROW_HEIGHT, WIDTH, ROW_HEIGHT),
                (frame, 0.0, 0.0, 28.0, 28.0),
                (portrait, 1.0, 1.0, 26.0, 26.0),
                (speaker, 34.0, 2.0, 24.0, 24.0),
                (waves, 34.0, 2.0, 24.0, 24.0),
                (muted, 34.0, 2.0, 24.0, 24.0),
                (name, 65.0, 4.0, WIDTH - 65.0, 24.0),
            ] {
                place(ctx.inner(), node, x, y, w, h);
            }
            VoiceRow {
                root,
                frame,
                portrait,
                speaker,
                waves,
                muted,
                name,
            }
        });
        let status = label(ctx, "", 11.0, CREAM);
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(WIDTH)
                .with_height(HEIGHT)
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_children(rows.iter().map(|row| row.root).chain([status])),
        )
        .build(ctx)
        .to_base();
        Self {
            root,
            status,
            rows,
            textures,
        }
    }

    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> + '_ {
        self.rows.iter().map(|row| row.name).chain([self.status])
    }

    pub fn render(&self, rt: &Runtime, ui: &UserInterface, selecting: bool) {
        let show = !rt.home_open
            && !rt.lobby.open
            && !rt.startup.active()
            && !rt.art_review
            && !rt.session.world().in_round_transition();
        visible(ui, self.root, show);
        if !show {
            return;
        }
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0);
        let (x, y, scale) = if selecting {
            // Below team synergies, left of the team cards. Keep the same
            // readable row size, including when all six humans are present.
            (
                (size.x - 1280.0 * scale) * 0.5 + 44.0 * scale,
                (size.y - 800.0 * scale) * 0.5 + 324.0 * scale,
                scale,
            )
        } else {
            let scale = scale.clamp(0.65, 1.25);
            (24.0 * scale, 20.0 * scale, scale)
        };
        place(ui, self.root, x, y, WIDTH, HEIGHT);
        ui.send(
            self.root,
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        let local = rt.session.slot();
        let slots: Vec<_> = roster_slots(rt.session.world(), local).collect();
        let now = Instant::now();
        for (index, row) in self.rows.iter().enumerate() {
            visible(ui, row.root, index < slots.len());
            let Some(&slot) = slots.get(index) else {
                continue;
            };
            let muted = slot == local && rt.voice.mic.muted;
            let available = slot != local
                || rt
                    .voice
                    .mic
                    .transmitting(Voice::connected(&rt.session), rt.home_open);
            let speaking = !muted
                && available
                && rt.voice.speaking_until[slot].is_some_and(|until| until > now);
            let color = if muted {
                VOICE_RED
            } else if available {
                VOICE_CYAN
            } else {
                VOICE_DIM
            };
            for node in [row.speaker, row.waves, row.muted] {
                ui.send(
                    node,
                    WidgetMessage::Foreground(Brush::Solid(rgb(color)).into()),
                );
            }
            visible(ui, row.muted, muted);
            visible(ui, row.waves, speaking);
            for node in [row.frame, row.name] {
                ui.send(
                    node,
                    WidgetMessage::Foreground(
                        Brush::Solid(rgb(if speaking {
                            [173, 255, 255]
                        } else {
                            VOICE_CYAN
                        }))
                        .into(),
                    ),
                );
            }
            let id = rt.session.world().actors[slot].character as usize;
            let texture = &self.textures[id];
            ui.send(row.portrait, ImageMessage::Texture(Some(texture.clone())));
            ui.send(
                row.portrait,
                ImageMessage::UvRect(portraits::uv(id, texture, 26.0, 26.0)),
            );
            text(ui, row.name, rt.session.player_name(slot));
        }
        place(
            ui,
            self.status,
            0.0,
            slots.len() as f32 * ROW_HEIGHT + 2.0,
            WIDTH,
            34.0,
        );
        let (label, color) = rt.voice.label(&rt.session, rt.home_open);
        text(ui, self.status, label);
        ui.send(
            self.status,
            WidgetMessage::Foreground(Brush::Solid(rgb(color)).into()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_team_slot_is_first_in_its_own_voice_roster() {
        let mut world = World::default();
        for actor in &mut world.actors {
            actor.human = true;
        }
        for local in 0..PLAYERS {
            let slots: Vec<_> = roster_slots(&world, local).collect();
            assert_eq!(slots[0], local);
            assert_eq!(slots.len(), TEAM_SIZE);
            assert!(slots.iter().all(|&slot| team(slot) == team(local)));
            assert_eq!(slots.iter().filter(|&&slot| slot == local).count(), 1);
        }
    }

    #[test]
    fn voice_roster_excludes_bots_empty_slots_and_enemies_but_keeps_dead_teammates() {
        let mut world = World::default();
        for actor in &mut world.actors {
            actor.human = true;
        }
        world.actors[1].active = false;
        world.actors[2].human = false;
        world.actors[3].hp = 0;
        assert_eq!(roster_slots(&world, 4).collect::<Vec<_>>(), [4, 0, 3, 5]);
        for actor in &mut world.actors {
            actor.active = false;
        }
        assert_eq!(roster_slots(&world, 4).collect::<Vec<_>>(), [4]);
    }

    #[test]
    fn mute_discards_backlog_and_rejects_previous_capture_generation() {
        let (tx, captured) = mpsc::sync_channel(4);
        let (incoming, _receive) = mpsc::sync_channel(4);
        let (_status, status) = mpsc::sync_channel(4);
        let mut audio = VoiceAudio {
            generation: Arc::new(AtomicU64::new(0)),
            stop: Arc::new(AtomicBool::new(false)),
            captured,
            incoming,
            status,
            worker: None,
            input_ready: false,
            input_error: None,
            output_error: None,
        };
        audio.set_enabled(true);
        let old = audio.generation.load(Ordering::SeqCst);
        audio.input_ready = true;
        tx.send(Captured {
            generation: old,
            time: Instant::now(),
            samples: [1000; 320],
        })
        .unwrap();
        audio.set_enabled(false);
        assert!(audio.poll().is_empty());
        audio.set_enabled(true);
        audio.input_ready = true;
        let current = audio.generation.load(Ordering::SeqCst);
        tx.send(Captured {
            generation: old,
            time: Instant::now(),
            samples: [1000; 320],
        })
        .unwrap();
        tx.send(Captured {
            generation: current,
            time: Instant::now() - Duration::from_secs(1),
            samples: [2000; 320],
        })
        .unwrap();
        tx.send(Captured {
            generation: current,
            time: Instant::now(),
            samples: [3000; 320],
        })
        .unwrap();
        assert_eq!(audio.poll(), vec![[3000; 320]]);
    }
    #[test]
    #[ignore = "Requires default microphone and speakers; captures briefly without saving audio"]
    fn default_audio_devices_capture_and_mute() {
        let mut audio = VoiceAudio::new().unwrap();
        assert_eq!(audio.generation.load(Ordering::SeqCst), 0);
        for _ in 0..2 {
            audio.set_enabled(true);
            let start = Instant::now();
            let mut count = 0;
            while start.elapsed() < Duration::from_secs(2) && count < 3 {
                count += audio.poll().len();
                assert!(audio.input_error.is_none(), "{:?}", audio.input_error);
                assert!(audio.output_error.is_none(), "{:?}", audio.output_error);
                thread::sleep(Duration::from_millis(10));
            }
            assert!(count >= 3, "Microphone produced no frames");
            audio.set_enabled(false);
            thread::sleep(Duration::from_millis(80));
            assert!(audio.poll().is_empty());
        }
    }
}
