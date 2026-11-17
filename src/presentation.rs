//! Fyrox scene and UI. Gameplay stays in the authoritative simulation.
mod account;
mod attack_review;
mod castle;
mod character_shadows;
mod characters;
mod chat;
mod cheek_masks;
mod cheek_review;
mod cheeks;
mod combat_hud;
mod combat_icons;
mod communication;
mod crosshair;
mod crouch_review;
mod dance_review;
mod deathcam_review;
mod erpin_fx;
mod erpin_review;
mod erpin_wings;
mod expressions;
mod first_person;
mod ghost_review;
mod ghost_walls;
mod greeting_review;
mod hand_grip;
mod home;
mod impact_review;
mod impacts;
mod lobby;
mod map_review;
mod morph_import;
mod move_review;
mod naia_skill_review;
mod naia_water;
mod nameplates;
mod objective_hud;
mod phase_hud;
mod player_info;
mod portal_review;
mod portals;
mod portraits;
mod rooms;
mod round_results;
mod scope;
mod scope_blur;
mod selection;
mod settings_menu;
mod shadow_review;
mod shield_review;
mod shields;
mod skeleton;
mod startup;
mod toon;
mod trait_markers;
mod ultimate_hud;
mod voice;
mod weapon_review;
mod weapons;
mod zoom;
use fyrox::{
    asset::ResourceData,
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector2, Vector3},
        color::Color,
        pool::Handle,
        reflect::prelude::*,
        visitor::prelude::*,
    },
    event::{DeviceEvent, ElementState, Event, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    graph::prelude::*,
    graphics::framebuffer::ReadTarget,
    gui::{
        border::BorderBuilder,
        brush::Brush,
        font::{Font, FontResource},
        text::{Text, TextBuilder, TextMessage},
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, UiNode, UserInterface,
    },
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    material::{Material, MaterialResource, MaterialResourceExtension},
    plugin::{error::GameResult, Plugin, PluginContext},
    renderer::ui_renderer::UiRenderInfo,
    resource::texture::{
        Texture, TextureKind, TexturePixelKind, TextureResource, TextureResourceExtension,
    },
    scene::{
        base::BaseBuilder,
        camera::{CameraBuilder, Exposure},
        graph::Graph,
        light::{directional::DirectionalLightBuilder, BaseLightBuilder},
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
            MeshBuilder,
        },
        node::Node,
        pivot::PivotBuilder,
        transform::TransformBuilder,
        Scene,
    },
    window::{CursorGrabMode, Window},
};
use std::time::Instant;
use trickcal_shooting_balddagu::{
    chat::{clean_text, ChatHistory, ChatInput, ChatMessage, ChatTranscript},
    combat::{ProjectileKind, MAX_PROJECTILES},
    communication::{wave_pose, Communication, CommunicationResult, CommunicationWheel},
    deathcam::{DeathCamera, Stage as DeathStage},
    emote::OrbitCamera,
    game::*,
    kits::{kit, AttackMode},
    level::MapId,
    net::{Client, Server},
    roster::{character, SelectionResult, ROSTER},
};

fn vec(v: V3) -> Vector3<f32> {
    Vector3::new(v.x, v.y, v.z)
}
fn rgb(v: [u8; 3]) -> Color {
    Color::opaque(v[0], v[1], v[2])
}
const MINT: [u8; 3] = [106, 226, 194];
const PINK: [u8; 3] = [245, 151, 180];
const INK: [u8; 3] = [31, 42, 54];
const CREAM: [u8; 3] = [253, 240, 210];

#[derive(Debug)]
pub enum Session {
    Solo(Box<World>),
    Host(Box<Server>),
    Join(Box<Client>),
}
impl Session {
    fn player_name(&self, slot: usize) -> String {
        match self {
            Self::Host(server) => server.player_names[slot].clone(),
            Self::Join(client) => client.player_names[slot].clone(),
            Self::Solo(_) => format!("PLAYER {}", slot + 1),
        }
    }
    fn world(&self) -> &World {
        match self {
            Self::Solo(w) => w,
            Self::Host(s) => &s.world,
            Self::Join(c) => &c.world,
        }
    }
    fn slot(&self) -> usize {
        match self {
            Self::Join(c) => c.slot.unwrap_or(0),
            _ => 0,
        }
    }
    fn start(&mut self) {
        if self.world().phase != Phase::Lobby {
            return;
        }
        match self {
            Self::Solo(w) => w.start(),
            Self::Host(s) => s.start(),
            Self::Join(_) => {}
        }
    }
    fn select(&mut self, id: u8) -> String {
        match self {
            Self::Solo(w) => w.select_character(0, id).message().into(),
            Self::Host(s) => s.world.select_character(0, id).message().into(),
            Self::Join(c) => {
                c.select_character(id);
                "Waiting for server...".into()
            }
        }
    }
    fn update(&mut self, input: Input) -> Result<(), String> {
        match self {
            Self::Solo(w) => {
                let mut inputs = [Input::default(); PLAYERS];
                inputs[0] = input;
                w.tick(inputs);
                Ok(())
            }
            Self::Host(s) => s.update(input).map_err(|e| e.to_string()),
            Self::Join(c) => {
                c.update(input).map_err(|e| e.to_string())?;
                c.error.clone().map_or(Ok(()), Err)
            }
        }
    }
    fn label(&self) -> String {
        match self {
            Self::Solo(w) => format!(
                "PRACTICE / {} BOTS",
                w.actors.iter().filter(|a| a.active && !a.human).count()
            ),
            Self::Host(s) if s.is_eos() => format!(
                "HOST / {} / {} HUMAN",
                s.network_label(),
                s.peer_count() + 1
            ),
            Self::Host(s) => format!(
                "HOST / UDP {} / {} HUMAN",
                s.addr().map_or(0, |a| a.port()),
                s.peer_count() + 1
            ),
            Self::Join(c) => {
                if let Some(err) = &c.error {
                    err.clone()
                } else if c.received {
                    format!(
                        "{} / P{} / RTT {} ms",
                        c.network_label(),
                        c.slot.unwrap_or(0) + 1,
                        c.rtt().map_or(0, |rtt| rtt.as_millis())
                    )
                } else {
                    "CONNECTING TO SERVER...".into()
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct Keys {
    w: bool,
    a: bool,
    s: bool,
    d: bool,
    jump: bool,
    phase_left: bool,
    phase_right: bool,
    crouch_left: bool,
    crouch_right: bool,
    reload: bool,
    fire: bool,
    alt_fire: bool,
    zoom: bool,
    score: bool,
    ability: bool,
    ultimate: bool,
    dance: bool,
}
#[derive(Debug, Default)]
struct Hud {
    scope: scope::ScopeUi,
    round_results: round_results::ResultUi,
    settings: settings_menu::SettingsUi,
    voice: voice::VoiceUi,
    brand: Handle<UiNode>,
    connection: Handle<UiNode>,
    objective: objective_hud::ObjectiveHud,
    player_info: player_info::PlayerInfoUi,
    nameplates: nameplates::Nameplates,
    combat: combat_hud::CombatHud,
    cross: crosshair::Crosshair,
    banner: Handle<UiNode>,
    panel: Handle<UiNode>,
    menu: Handle<UiNode>,
    scoreboard: Handle<UiNode>,
    status: Handle<UiNode>,
    ultimate: ultimate_hud::UltimateHud,
    selection: selection::SelectionUi,
    chat: chat::ChatUi,
    communication: communication::CommunicationUi,
    home: home::HomeUi,
    account: account::AccountUi,
    startup: startup::StartupUi,
    lobby: lobby::LobbyUi,
    rooms: rooms::BrowserUi,
}

#[derive(Debug)]
pub struct Runtime {
    settings_menu: settings_menu::Menu,
    settings_review: bool,
    last_perf_log: Instant,
    ghost_walls: ghost_walls::GhostWalls,
    portals: portals::Portals,
    portal_review: bool,
    portal_view_offset: Vector2<f32>,
    portal_transits: u16,
    impacts: impacts::Impacts,
    impact_review: bool,
    ghost_review: bool,
    shield_review: bool,
    shadow_review: bool,
    voice: voice::Voice,
    voice_review: bool,
    session: Session,
    scene: Handle<Scene>,
    rendered_map: MapId,
    toon: toon::ToonRenderer,
    camera: Handle<Node>,
    actors: [Handle<Node>; PLAYERS],
    weapon: Handle<Node>,
    wave_hand: Handle<Node>,
    first_person_arm: Option<first_person::FirstPersonArm>,
    first_person_support_arm: Option<first_person::FirstPersonArm>,
    weapons: [Handle<Node>; 15],
    models: Vec<fyrox::resource::model::ModelResource>,
    weapon_models: Vec<Option<fyrox::resource::model::ModelResource>>,
    visuals: Vec<characters::Visual>,
    orbit: OrbitCamera,
    deathcam: DeathCamera,
    deathcam_review: bool,
    round_results_review: bool,
    objective_hud_review: bool,
    map_review: bool,
    preview: Option<selection::Preview>,
    preview_serial: u64,
    pending_ready: Option<u8>,
    selection_review: bool,
    cheek_review: bool,
    communication_review: bool,
    hud_review: bool,
    nameplates_review: bool,
    weapon_review: bool,
    greeting_review: bool,
    dance_review: bool,
    attack_review: bool,
    naia_skill_review: bool,
    spawn_edges: [Handle<Node>; 4],
    smoke_dance_seen: bool,
    smoke_dance_cancelled: bool,
    art_review: bool,
    review_capture: u8,
    review_frames: u32,
    cheek_review_idle_frames: u8,
    projectiles: [Handle<Node>; MAX_PROJECTILES],
    projectile_materials: [MaterialResource; MAX_PROJECTILES],
    guards: [Handle<Node>; PLAYERS],
    team_marks: [Handle<Node>; PLAYERS],
    flash: Handle<Node>,
    point_materials: [MaterialResource; 2],
    tracers: [Handle<Node>; PLAYERS],
    naia_water: Option<naia_water::Water>,
    erpin_fx: Option<erpin_fx::Effects>,
    tracer_time: [f32; PLAYERS],
    last_shots: [u32; PLAYERS],
    hud: Hud,
    keys: Keys,
    modifiers: ModifiersState,
    chat: ChatInput,
    solo_chat: ChatHistory,
    chat_transcript: ChatTranscript,
    chat_revision: u64,
    chat_scroll: usize,
    chat_error: String,
    chat_restore_grab: bool,
    chat_ime_commit: bool,
    wheel: CommunicationWheel,
    communication_notice: String,
    communication_notice_time: f32,
    yaw: f32,
    pitch: f32,
    grabbed: bool,
    last_round: u16,
    last_dead: bool,
    view_pos: V3,
    started: Instant,
    smoke: Option<f32>,
    capture_stage: u8,
    smoke_ult_used: bool,
    auto_start: bool,
    error: Option<String>,
    last_phase: Phase,
    selecting: bool,
    selected: usize,
    selection_message: String,
    cursor: Vector2<f32>,
    korean_font: bool,
    home_open: bool,
    startup: startup::Startup,
    home_focus: usize,
    home_review: bool,
    lobby: lobby::Lobby,
    browser: rooms::Browser,
    account: account::Account,
    pulses: [Handle<Node>; PLAYERS],
}
impl Runtime {
    fn play_from_home(&mut self, window: &Window) {
        self.home_open = false;
        self.lobby.open = true;
        self.browser.open = true;
        if !self.lobby.solo {
            self.browser.refresh();
        }
        self.selecting = false;
        self.grabbed = false;
        self.keys = Keys::default();
        set_cursor(window, false);
    }
    fn begin_communication(&mut self, window: &Window) {
        self.wheel.begin();
        self.keys = Keys::default();
        window.set_cursor_visible(false);
    }
    fn end_communication(&mut self, window: &Window, execute: bool) {
        let selected = if execute {
            self.wheel.release()
        } else {
            self.wheel.cancel();
            None
        };
        self.keys = Keys::default();
        window.set_cursor_visible(!self.grabbed);
        if let Some(action) = selected {
            self.communicate(action);
        }
    }
    fn communicate(&mut self, action: Communication) {
        let result = match &mut self.session {
            Session::Solo(world) => match world.communicate(0, action) {
                Ok(text) => {
                    self.solo_chat.insert(ChatMessage {
                        id: self.solo_chat.revision + 1,
                        sender: 0,
                        character: world.actors[0].character,
                        channel: trickcal_shooting_balddagu::chat::ChatChannel::Team,
                        text,
                    });
                    CommunicationResult::Accepted
                }
                Err(result) => result,
            },
            Session::Host(server) => server.communicate(action),
            Session::Join(client) => client.communicate(action),
        };
        self.communication_feedback(result);
    }
    fn communication_feedback(&mut self, result: CommunicationResult) {
        if result != CommunicationResult::Accepted {
            self.communication_notice = result.message(self.korean_font).into();
            self.communication_notice_time = 2.5;
        }
    }
    fn chat_history(&self) -> &ChatHistory {
        &self.chat_transcript.history
    }
    fn sync_chat(&mut self) {
        let source = match &self.session {
            Session::Solo(_) => &self.solo_chat,
            Session::Host(server) => &server.chat,
            Session::Join(client) => &client.chat,
        };
        self.chat_transcript.sync(source);
    }
    fn open_chat(&mut self, window: &Window) {
        if let Some(preview) = &mut self.preview {
            preview.cheeks.cancel();
        }
        self.chat_restore_grab = self.grabbed;
        self.chat.open = true;
        self.chat.touch();
        self.chat_error.clear();
        self.chat_scroll = 0;
        self.chat_ime_commit = false;
        self.grabbed = false;
        self.keys = Keys::default();
        set_cursor(window, false);
        window.set_ime_allowed(true);
        let size = window.inner_size();
        let layout = chat::layout(Vector2::new(size.width as f32, size.height as f32));
        window.set_ime_cursor_area(
            fyrox::dpi::PhysicalPosition::new(layout.x + 12.0 * layout.scale, layout.input_y()),
            fyrox::dpi::PhysicalSize::new(layout.width, 24.0 * layout.scale),
        );
    }
    fn close_chat(&mut self, window: &Window, restore: bool) {
        self.chat.open = false;
        self.chat.preedit.clear();
        self.chat.touch();
        self.chat_ime_commit = false;
        self.chat_scroll = 0;
        self.keys = Keys::default();
        self.grabbed = restore
            && self.chat_restore_grab
            && !self.selecting
            && !self.lobby.open
            && !self.home_open;
        window.set_ime_allowed(false);
        set_cursor(window, self.grabbed);
    }
    fn send_chat(&mut self, window: &Window) {
        let text = clean_text(&self.chat.draft);
        if !text.is_empty() {
            if let Err(error) = self.deliver_chat(&text) {
                self.chat_error = error.into();
                return;
            }
        }
        self.chat.clear();
        self.close_chat(window, true);
    }
    fn deliver_chat(&mut self, text: &str) -> Result<(), &'static str> {
        if self.lobby.open && !self.lobby.room_active {
            return Err("먼저 방에 참여해 주세요 / Join a room first");
        }
        let channel = self.chat.channel;
        let result = match &mut self.session {
            Session::Solo(world) => {
                self.solo_chat.insert(ChatMessage {
                    id: self.solo_chat.revision + 1,
                    sender: 0,
                    character: world.actors[0].character,
                    channel,
                    text: text.into(),
                });
                Ok(())
            }
            Session::Host(server) => server.send_chat(channel, text),
            Session::Join(client) => client.send_chat(channel, text),
        };
        self.sync_chat();
        result
    }
    fn confirm_selection(&mut self) {
        let id = self.selected as u8;
        if let Err(reason) = self
            .session
            .world()
            .selection_check(self.session.slot(), id)
        {
            self.selection_message = reason.message().into();
            return;
        }
        if matches!(&self.session, Session::Join(c) if c.selection_pending()) {
            return;
        }
        self.selection_message = self.session.select(id);
        if self.selecting {
            self.pending_ready = Some(id);
        }
    }
    fn close_selection(&mut self, window: &Window) {
        if let Some(preview) = &mut self.preview {
            preview.cheeks.cancel();
        }
        self.selecting = false;
        self.home_open = false;
        self.grabbed = !self.chat.open;
        if self.chat.open {
            self.chat_restore_grab = true;
        }
        self.keys = Keys::default();
        self.pending_ready = None;
        self.lobby.return_from_selection = false;
        self.lobby.start_requested = false;
        set_cursor(window, self.grabbed);
    }
    fn cancel_selection(&mut self, window: &Window) {
        let return_to_lobby = self.lobby.return_from_selection;
        self.close_selection(window);
        if return_to_lobby || self.session.world().phase == Phase::Lobby {
            self.lobby.open = true;
            self.grabbed = false;
            set_cursor(window, false);
        }
    }
    pub fn new(session: Session, smoke: Option<f32>, auto_start: bool) -> Self {
        let lobby = lobby::Lobby::new(&session);
        let placeholder = MaterialResource::new(Material::standard());
        let review = std::env::args().any(|a| {
            matches!(
                a.as_str(),
                "--art-review"
                    | "--selection-review"
                    | "--cheek-review"
                    | "--communication-review"
                    | "--hud-review"
                    | "--weapon-review"
                    | "--greeting-review"
                    | "--dance-review"
                    | "--attack-review"
                    | "--naia-skill-review"
                    | "--portal-review"
                    | "--deathcam-review"
                    | "--map-review"
                    | "--ghost-review"
                    | "--shield-review"
                    | "--shadow-review"
                    | "--settings-review"
                    | "--round-results-review"
                    | "--objective-hud-review"
                    | "--nameplates-review"
                    | "--voice-review"
            )
        });
        Self {
            last_perf_log: Instant::now(),
            settings_menu: settings_menu::Menu::default(),
            settings_review: std::env::args().any(|a| a == "--settings-review"),
            session,
            ghost_walls: ghost_walls::GhostWalls::default(),
            portals: portals::Portals::default(),
            portal_review: std::env::args().any(|a| a == "--portal-review"),
            portal_view_offset: Vector2::zeros(),
            portal_transits: 0,
            impacts: impacts::Impacts::default(),
            impact_review: std::env::args().any(|a| a == "--impact-review"),
            ghost_review: std::env::args().any(|a| a == "--ghost-review"),
            shield_review: std::env::args().any(|a| a == "--shield-review"),
            shadow_review: std::env::args().any(|a| a == "--shadow-review"),
            voice: voice::Voice::default(),
            voice_review: std::env::args().any(|a| a == "--voice-review"),
            scene: Handle::NONE,
            rendered_map: MapId::default(),
            toon: toon::ToonRenderer::default(),
            camera: Handle::NONE,
            actors: [Handle::NONE; PLAYERS],
            weapon: Handle::NONE,
            wave_hand: Handle::NONE,
            first_person_arm: None,
            first_person_support_arm: None,
            weapons: [Handle::NONE; 15],
            models: vec![],
            weapon_models: vec![None; 15],
            visuals: vec![],
            orbit: OrbitCamera::default(),
            deathcam: DeathCamera::default(),
            deathcam_review: std::env::args().any(|a| a == "--deathcam-review"),
            round_results_review: std::env::args().any(|a| a == "--round-results-review"),
            objective_hud_review: std::env::args().any(|a| a == "--objective-hud-review"),
            map_review: std::env::args().any(|a| a == "--map-review"),
            preview: None,
            preview_serial: 0,
            pending_ready: None,
            selection_review: std::env::args().any(|a| a == "--selection-review"),
            cheek_review: std::env::args().any(|a| a == "--cheek-review"),
            communication_review: std::env::args().any(|a| a == "--communication-review"),
            hud_review: std::env::args().any(|a| a == "--hud-review" || a == "--phase-review"),
            nameplates_review: std::env::args().any(|a| a == "--nameplates-review"),
            weapon_review: std::env::args().any(|a| a == "--weapon-review"),
            greeting_review: std::env::args().any(|a| a == "--greeting-review"),
            dance_review: std::env::args().any(|a| a == "--dance-review"),
            attack_review: std::env::args().any(|a| a == "--attack-review"),
            naia_skill_review: std::env::args().any(|a| a == "--naia-skill-review"),
            spawn_edges: [Handle::NONE; 4],
            smoke_dance_seen: false,
            smoke_dance_cancelled: false,
            art_review: std::env::args().any(|a| a == "--art-review"),
            review_capture: if std::env::args().any(|a| a == "--phase-review") {
                37
            } else {
                0
            },
            review_frames: 0,
            cheek_review_idle_frames: 0,
            projectiles: [Handle::NONE; MAX_PROJECTILES],
            projectile_materials: std::array::from_fn(|_| material(CREAM)),
            guards: [Handle::NONE; PLAYERS],
            team_marks: [Handle::NONE; PLAYERS],
            flash: Handle::NONE,
            point_materials: [placeholder.clone(), placeholder],
            tracers: [Handle::NONE; PLAYERS],
            naia_water: None,
            erpin_fx: None,
            tracer_time: [0.0; PLAYERS],
            last_shots: [0; PLAYERS],
            hud: Hud::default(),
            keys: Keys::default(),
            modifiers: ModifiersState::default(),
            chat: ChatInput::default(),
            solo_chat: ChatHistory::default(),
            chat_transcript: ChatTranscript::default(),
            chat_revision: 0,
            chat_scroll: 0,
            chat_error: String::new(),
            chat_restore_grab: false,
            chat_ime_commit: false,
            wheel: CommunicationWheel::default(),
            communication_notice: String::new(),
            communication_notice_time: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            grabbed: false,
            last_round: 0,
            last_dead: false,
            view_pos: V3::new(0.0, EYE_HEIGHT, -13.0),
            started: Instant::now(),
            smoke,
            capture_stage: 0,
            smoke_ult_used: false,
            auto_start,
            error: None,
            last_phase: Phase::Lobby,
            selecting: review && !auto_start && smoke.is_none(),
            selected: 0,
            selection_message: String::new(),
            cursor: Vector2::zeros(),
            korean_font: false,
            home_open: !auto_start && smoke.is_none() && !review,
            startup: startup::Startup::new(
                !auto_start
                    && smoke.is_none()
                    && !review
                    && !std::env::args().any(|a| {
                        matches!(
                            a.as_str(),
                            "--home-review"
                                | "--lobby-review"
                                | "--account-review"
                                | "--online-session"
                        )
                    }),
                std::env::args().any(|a| a == "--startup-review"),
            ),
            home_focus: 0,
            home_review: std::env::args().any(|a| a == "--home-review"),
            lobby,
            browser: rooms::Browser::default(),
            account: account::Account::default(),
            pulses: [Handle::NONE; PLAYERS],
        }
    }
}

#[derive(Default, Debug, Visit, Reflect)]
#[reflect(non_cloneable)]
pub struct Game {
    #[visit(skip)]
    #[reflect(hidden)]
    runtime: Option<Runtime>,
    #[visit(skip)]
    #[reflect(hidden)]
    pending_runtime: Option<Runtime>,
}
impl Game {
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime: None,
            pending_runtime: Some(runtime),
        }
    }
}

struct Shapes {
    cube: SurfaceResource,
    sphere: SurfaceResource,
    cylinder: SurfaceResource,
}
impl Shapes {
    fn new() -> Self {
        Self {
            cube: SurfaceResource::new_embedded(SurfaceData::make_cube(Matrix4::identity())),
            sphere: SurfaceResource::new_embedded(SurfaceData::make_sphere(
                16,
                16,
                1.0,
                &Matrix4::identity(),
            )),
            cylinder: SurfaceResource::new_embedded(SurfaceData::make_cylinder(
                48,
                1.0,
                1.0,
                true,
                &Matrix4::identity(),
            )),
        }
    }
    fn mesh(
        &self,
        g: &mut Graph,
        name: &str,
        pos: V3,
        size: V3,
        mat: MaterialResource,
        shape: u8,
    ) -> Handle<Node> {
        let data = match shape {
            1 => &self.sphere,
            2 => &self.cylinder,
            _ => &self.cube,
        };
        MeshBuilder::new(
            BaseBuilder::new().with_name(name).with_local_transform(
                TransformBuilder::new()
                    .with_local_position(vec(pos))
                    .with_local_scale(vec(size))
                    .build(),
            ),
        )
        .with_surfaces(vec![SurfaceBuilder::new(data.clone())
            .with_material(mat)
            .build()])
        .build(g)
        .to_base()
    }
    fn solid(
        &self,
        g: &mut Graph,
        name: &str,
        pos: V3,
        size: V3,
        color: [u8; 3],
        shape: u8,
    ) -> Handle<Node> {
        self.mesh(g, name, pos, size, material(color), shape)
    }
}
fn material(color: [u8; 3]) -> MaterialResource {
    let mut m = Material::standard();
    m.set_property("diffuseColor", rgb(color));
    m.set_property("roughness", 0.85_f32);
    MaterialResource::new(m)
}

fn make_scene(rt: &mut Runtime) -> Scene {
    let mut scene = Scene::new();
    let night = rt.session.world().map_id == MapId::ElfCity;
    if night {
        scene.set_skybox(Some(fyrox::scene::skybox::SkyBox::from_single_color(rgb(
            [55, 68, 103],
        ))));
    }
    scene.rendering_options.get_value_mut_silent().clear_color =
        Some(rgb(if night { [13, 21, 40] } else { [183, 219, 223] }));
    scene
        .rendering_options
        .get_value_mut_silent()
        .ambient_lighting_color = rgb(if night {
        [190, 205, 230]
    } else {
        [205, 200, 185]
    });
    let g = &mut scene.graph;
    let shapes = Shapes::new();
    rt.guards = shields::create(g);
    rt.naia_water = Some(naia_water::Water::new(g));
    rt.erpin_fx = Some(erpin_fx::Effects::new(g, &shapes));
    for i in 0..4 {
        rt.spawn_edges[i] = shapes.solid(
            g,
            "Active friendly sanctuary boundary",
            V3::default(),
            V3::new(1.0, 0.03, 1.0),
            MINT,
            0,
        );
    }
    DirectionalLightBuilder::new(
        BaseLightBuilder::new(
            BaseBuilder::new()
                .with_name("Castle morning sun")
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_rotation(UnitQuaternion::from_euler_angles(-0.9, -0.5, 0.0))
                        .build(),
                ),
        )
        .with_color(rgb(if night {
            [125, 167, 255]
        } else {
            [255, 244, 222]
        }))
        .with_intensity(if night { 0.75 } else { 2.0 }),
    )
    .build(g);
    for (index, p) in rt.session.world().points().into_iter().enumerate() {
        let color = if index == 0 { MINT } else { PINK };
        let mat = material(color);
        rt.point_materials[index] = mat.clone();
        shapes.solid(
            g,
            "Capture pad",
            p + V3::new(0.0, 0.045, 0.0),
            V3::new(RADIUS, 0.08, RADIUS),
            [216, 220, 201],
            2,
        );
        for n in 0..36 {
            let a = n as f32 * std::f32::consts::TAU / 36.0;
            let marker = shapes.mesh(
                g,
                "Capture boundary",
                p + V3::new(a.sin() * RADIUS, 0.11, a.cos() * RADIUS),
                V3::new(0.4, 0.08, 0.12),
                mat.clone(),
                0,
            );
            g[marker]
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), a));
        }
        shapes.solid(
            g,
            "Objective beacon",
            p + V3::new(0.0, 4.3, 0.0),
            V3::new(0.43, 0.43, 0.43),
            color,
            1,
        );
        shapes.solid(
            g,
            "Beacon crown",
            p + V3::new(0.0, 4.9, 0.0),
            V3::new(0.15, 0.15, 0.15),
            CREAM,
            1,
        );
    }
    for i in 0..PLAYERS {
        let root = PivotBuilder::new(BaseBuilder::new().with_name(format!("Balddagu {}", i + 1)))
            .build(g)
            .to_base();
        rt.team_marks[i] = shapes.solid(
            g,
            "Team color marker",
            V3::default(),
            V3::new(0.58, 0.018, 0.58),
            if team(i) == 0 { MINT } else { PINK },
            2,
        );
        rt.actors[i] = root;
        rt.tracers[i] = shapes.solid(
            g,
            "Sugar tracer",
            V3::default(),
            V3::new(0.025, 0.025, 1.0),
            [255, 221, 134],
            0,
        );
        g[rt.tracers[i]].set_visibility(false);
        rt.pulses[i] = shapes.solid(
            g,
            "Ability pulse",
            V3::default(),
            V3::new(1.0, 0.025, 1.0),
            [164, 233, 244],
            2,
        );
        g[rt.pulses[i]].set_visibility(false);
    }
    // Offscreen observers precede the main observer: portals display this frame.
    rt.portals = portals::Portals::new(g);
    rt.camera = CameraBuilder::new(BaseBuilder::new().with_name("FPS camera"))
        .with_fov(zoom::NORMAL_FOV.to_radians())
        .with_z_near(0.02)
        .with_z_far(180.0)
        .with_exposure(Exposure::Manual(1.0))
        .build(g)
        .to_base();
    rt.weapon = PivotBuilder::new(BaseBuilder::new().with_name("First person weapon"))
        .build(g)
        .to_base();
    g.link_nodes(rt.weapon, rt.camera);
    rt.wave_hand = PivotBuilder::new(BaseBuilder::new().with_name("Character first-person arm"))
        .build(g)
        .to_base();
    g.link_nodes(rt.wave_hand, rt.camera);
    g[rt.wave_hand].set_visibility(false);
    for id in 0..15 {
        let h = weapons::build(g, &shapes, id, rt.weapon_models[id as usize].as_ref());
        g[h].local_transform_mut()
            .set_position(if id == 0 {
                // Empty hands are in front; levitating blades rest behind the shoulders.
                Vector3::zeros()
            } else {
                // Camera-local -X is screen right; +Z is distance from the eye.
                Vector3::new(-0.44, -0.28, 0.56)
            })
            .set_scale(Vector3::repeat(0.72));
        g.link_nodes(h, rt.weapon);
        rt.weapons[id as usize] = h;
    }
    for i in 0..MAX_PROJECTILES {
        rt.projectiles[i] = shapes.mesh(
            g,
            "Moving projectile",
            V3::default(),
            V3::new(0.1, 0.1, 0.1),
            rt.projectile_materials[i].clone(),
            1,
        );
        g[rt.projectiles[i]].set_visibility(false);
    }
    rt.flash = shapes.solid(
        g,
        "Muzzle flash",
        V3::new(-0.32, -0.28, 1.12),
        V3::new(0.12, 0.12, 0.16),
        [255, 235, 139],
        1,
    );
    g.link_nodes(rt.flash, rt.weapon);
    g[rt.flash].set_visibility(false);
    character_shadows::disable_casting(g, rt.weapon);
    scene
}

fn populate_scene(
    rt: &mut Runtime,
    manager: &fyrox::asset::manager::ResourceManager,
) -> Result<Scene, String> {
    rt.visuals.clear();
    rt.first_person_arm = None;
    rt.first_person_support_arm = None;
    rt.ghost_walls = ghost_walls::GhostWalls::default();
    rt.impacts = impacts::Impacts::default();
    let mut scene = make_scene(rt);
    if !rt.selection_review {
        let scenery = castle::attach(&mut scene, manager, rt.session.world().map_id)?;
        rt.ghost_walls.attach(&mut scene.graph, scenery);
    }
    let shapes = Shapes::new();
    for i in 0..PLAYERS {
        let id = rt.session.world().actors[i].character;
        rt.visuals.push(characters::Visual::spawn(
            &mut scene,
            rt.actors[i],
            &rt.models[id as usize],
            id,
            &shapes,
            rt.weapon_models[id as usize].as_ref(),
        ));
    }
    rt.preview = Some(selection::Preview::new(&mut scene, &rt.models));
    rt.rendered_map = rt.session.world().map_id;
    Ok(scene)
}

fn sync_map_scene(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.rendered_map != rt.session.world().map_id {
        let scene = populate_scene(rt, context.resource_manager)
            .map_err(fyrox::plugin::error::GameError::str)?;
        context.scenes.remove(rt.scene);
        rt.scene = context.scenes.add(scene);
        rt.orbit = OrbitCamera::default();
        rt.deathcam = DeathCamera::default();
        rt.last_round = 0;
        rt.last_shots = [0; PLAYERS];
        rt.tracer_time = [0.0; PLAYERS];
        println!("MAP_SWITCH_OK: {}", rt.rendered_map.slug());
    }
    Ok(())
}

fn label(ctx: &mut BuildContext, text: &str, size: f32, color: [u8; 3]) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(color)).into()),
    )
    .with_text(text)
    .with_font_size(size.into())
    .with_shadow(true)
    .build(ctx)
    .to_base()
}
fn panel(ctx: &mut BuildContext, color: Color) -> Handle<UiNode> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_background(Brush::Solid(color).into()),
    )
    .with_corner_radius(12.0.into())
    .build(ctx)
    .to_base()
}
fn make_hud(ui: &mut UserInterface) -> Hud {
    // World labels sit behind the scope mask and every gameplay/menu overlay.
    let nameplates = nameplates::Nameplates::new(ui);
    // Create the scope first so health, ammo, objectives and menus remain above it.
    let scope = scope::ScopeUi::new(ui);
    let ctx = &mut ui.build_ctx();
    let hud = Hud {
        nameplates,
        scope,
        round_results: round_results::ResultUi::default(),
        brand: label(ctx, "TRICKCAL\nSHOOTING BALDDAGU", 19.0, CREAM),
        connection: label(ctx, "", 14.0, CREAM),
        objective: objective_hud::ObjectiveHud::default(),
        combat: combat_hud::CombatHud::default(),
        cross: crosshair::Crosshair::new(ctx),
        banner: label(ctx, "", 24.0, CREAM),
        panel: panel(ctx, Color::from_rgba(24, 35, 46, 235)),
        menu: label(ctx, "", 22.0, CREAM),
        scoreboard: label(ctx, "", 17.0, CREAM),
        player_info: player_info::PlayerInfoUi::default(),
        status: label(ctx, "", 16.0, MINT),
        ultimate: ultimate_hud::UltimateHud::default(),
        selection: selection::SelectionUi::default(),
        chat: chat::ChatUi::default(),
        voice: voice::VoiceUi::default(),
        communication: communication::CommunicationUi::default(),
        home: home::HomeUi::default(),
        account: account::AccountUi::default(),
        startup: startup::StartupUi::default(),
        lobby: lobby::LobbyUi::default(),
        rooms: rooms::BrowserUi::default(),
        settings: settings_menu::SettingsUi::default(),
    };
    // Widgets are already attached to the UI root canvas. A nested canvas with
    // no explicit size measures to zero and clips the entire HUD.
    hud
}
fn place(ui: &UserInterface, h: Handle<UiNode>, x: f32, y: f32, w: f32, height: f32) {
    ui.send(h, WidgetMessage::DesiredPosition(Vector2::new(x, y)));
    ui.send(h, WidgetMessage::Width(w));
    ui.send(h, WidgetMessage::Height(height));
}
fn text(ui: &UserInterface, h: Handle<UiNode>, s: impl Into<String>) {
    ui.send(h, TextMessage::Text(s.into()));
}
fn text_size(ui: &UserInterface, h: Handle<UiNode>, size: f32) {
    ui.send(h, TextMessage::FontSize(size.into()));
    // Fyrox advances lines by the ascender only. Add room for ONE Mobile POP's
    // descenders and shadows, and scale the spacing with responsive text sizes.
    let label = ui[h].cast::<Text>().expect("Text label");
    let mut formatted = label.formatted_text.borrow_mut();
    let spacing = size * 0.4;
    if formatted.line_space() != spacing {
        formatted.set_line_space(spacing);
        label.invalidate_layout();
    }
}
fn visible(ui: &UserInterface, h: Handle<UiNode>, v: bool) {
    ui.send(h, WidgetMessage::Visibility(v));
}
fn set_cursor(window: &Window, grab: bool) {
    if grab {
        let _ = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
    } else {
        let _ = window.set_cursor_grab(CursorGrabMode::None);
    }
    window.set_cursor_visible(!grab);
}

impl Plugin for Game {
    fn post_update(&mut self, context: &mut PluginContext) -> GameResult {
        if let Some(rt) = &mut self.runtime {
            let g = &mut context.scenes[rt.scene].graph;
            if let Some(preview) = &mut rt.preview {
                preview.react(
                    g,
                    rt.camera,
                    context.user_interfaces.first().screen_size(),
                    context.dt,
                );
                if rt.selection_review && std::env::args().any(|a| a == "--blink-review") {
                    preview.review_blink(g, [0.0, 0.52, 1.0][(rt.review_capture % 3) as usize]);
                } else if rt.selection_review && std::env::args().any(|a| a == "--face-review") {
                    preview.review_face(g, rt.review_capture % 2 == 1);
                }
            }
            for (visual, actor) in rt
                .visuals
                .iter()
                .zip(&rt.deathcam.scene().unwrap_or(rt.session.world()).actors)
            {
                visual.layer_attack_arms(g);
                visual.crouch(g);
                if actor.wave_ticks > 0 {
                    visual.wave(g, actor);
                }
                visual.attach_weapons(g);
                visual.update_wings(g, context.dt);
            }
            if let Some(water) = &rt.naia_water {
                let world = rt.deathcam.scene().unwrap_or(rt.session.world());
                let view_slot = rt.deathcam.view.map_or(rt.session.slot(), |v| v.target);
                g.update_hierarchical_data_for_descendants(rt.camera);
                for i in 0..PLAYERS {
                    let actor = if rt.deathcam.scene().is_none() {
                        match &rt.session {
                            Session::Join(client) => {
                                client.motion.actor(world, i, rt.session.slot())
                            }
                            _ => world.actors[i].clone(),
                        }
                    } else {
                        world.actors[i].clone()
                    };
                    let first = i == view_slot && g[rt.weapon].visibility();
                    let origin = if first {
                        g[rt.weapons[1]]
                            .global_transform()
                            .transform_point(&Vector3::new(0.0, 0.10, 0.49).into())
                            .coords
                    } else {
                        rt.visuals[i].water_muzzle(g)
                    };
                    water.update(g, i, &actor, &world.map, origin, world.tick as f32 * DT);
                }
            }
            if let Some(arm) = &rt.first_person_arm {
                arm.update(
                    g,
                    rt.wave_hand,
                    rt.camera,
                    rt.weapons[arm.character as usize],
                    rt.deathcam.scene().unwrap_or(rt.session.world()).actors
                        [rt.deathcam.view.map_or(rt.session.slot(), |v| v.target)]
                    .wave_ticks,
                );
            }
            let world = rt.deathcam.scene().unwrap_or(rt.session.world());
            let view_slot = rt.deathcam.view.map_or(rt.session.slot(), |v| v.target);
            let pose = rt.visuals[view_slot].pose();
            let eating = pose.motion == trickcal_shooting_balddagu::motion::Motion::ErpinEat;
            if let Some(effects) = &mut rt.erpin_fx {
                effects.view.show(g, 0);
                if let Some(arm) = &rt.first_person_support_arm {
                    if arm.character == 2 && eating && g[rt.wave_hand].visibility() {
                        let point = arm.eat(g, rt.wave_hand, rt.camera, pose.age);
                        let state = trickcal_shooting_balddagu::erpin::State {
                            eating: true,
                            elapsed: pose.age,
                            ..Default::default()
                        };
                        effects.view.show(g, state.remaining());
                        let rotation = skeleton::world_rotation(g, rt.camera);
                        g[effects.view.root]
                            .local_transform_mut()
                            .set_position(point)
                            .set_scale(Vector3::repeat(0.48))
                            .set_rotation(rotation);
                        character_shadows::disable_casting(g, effects.view.root);
                    } else {
                        arm.update(
                            g,
                            rt.wave_hand,
                            rt.camera,
                            rt.weapons[arm.character as usize],
                            world.actors[view_slot].wave_ticks,
                        );
                    }
                }
                let muzzle = g[rt.weapons[2]]
                    .global_transform()
                    .transform_point(&Vector3::new(0.0, 0.72, 0.0).into())
                    .coords;
                effects.update(
                    g,
                    world,
                    &rt.visuals,
                    &rt.projectiles,
                    erpin_fx::View {
                        slot: view_slot,
                        first_person: g[rt.weapon].visibility(),
                        muzzle,
                        replay: rt.deathcam.is_replay(),
                        dt: context.dt,
                    },
                );
            }
        }
        Ok(())
    }
    fn before_rendering(&mut self, context: PluginContext) -> GameResult {
        let Some(rt) = self.runtime.as_mut() else {
            return Ok(());
        };
        rt.toon
            .install(
                &mut context.graphics_context.as_initialized_mut().renderer,
                rt.scene,
                rt.camera,
            )
            .map_err(|error| fyrox::plugin::error::GameError::str(error.to_string()))?;
        rt.hud
            .scope
            .blur
            .install(&mut context.graphics_context.as_initialized_mut().renderer);
        if rt.startup.active() {
            return startup::before_rendering(rt, context);
        }
        if rt.account.review {
            return account::review_capture(rt, context);
        }
        if rt.settings_review {
            return settings_menu::review_capture(rt, context);
        }
        if rt.round_results_review {
            return round_results::review_capture(rt, context);
        }
        if rt.objective_hud_review {
            return objective_hud::review_capture(rt, context);
        }
        if rt.nameplates_review {
            return nameplates::review::capture(rt, context);
        }
        if rt.voice_review {
            return voice::review::capture(rt, context);
        }
        if rt.last_perf_log.elapsed().as_secs() >= 5 {
            let stats = context
                .graphics_context
                .as_initialized_ref()
                .renderer
                .get_statistics();
            let age = match &rt.session {
                Session::Join(client) => client.snapshot_age().as_millis(),
                _ => 0,
            };
            println!(
                "PERF: fps={} render_ms={:.2} update_ms={:.2} snapshot_age_ms={} {}",
                stats.frames_per_second,
                stats.pure_frame_time * 1000.0,
                context.performance_statistics.plugins_time.as_secs_f64() * 1000.0,
                age,
                rt.session.label()
            );
            rt.last_perf_log = Instant::now();
        }
        if (rt.selection_review
            || rt.cheek_review
            || rt.art_review
            || rt.hud_review
            || rt.communication_review
            || rt.weapon_review
            || rt.greeting_review
            || rt.dance_review
            || rt.attack_review
            || rt.naia_skill_review
            || rt.portal_review
            || rt.impact_review
            || rt.shadow_review)
            && rt.review_frames > 0
        {
            // Imported models can stall a draw while fixed updates catch up. Wait for actual
            // rendered frames so the scene readback and the current character's UI agree.
            if !context
                .graphics_context
                .as_initialized_ref()
                .renderer
                .scene_data_map
                .contains_key(&rt.scene)
            {
                return Ok(());
            }
            rt.review_frames += 1;
        }
        if rt.ghost_review && rt.review_frames >= ghost_review::capture_frames(rt) {
            return ghost_review::capture(rt, context);
        }
        if rt.impact_review && rt.review_frames >= 8 {
            return impact_review::capture(rt, context);
        }
        if rt.shadow_review && rt.review_frames >= 24 {
            return shadow_review::capture(rt, context);
        }
        if rt.shield_review && rt.review_frames > 0 {
            // Count rendered frames, not fixed updates that may catch up before a draw.
            rt.review_frames += 1;
            if rt.review_frames >= 36 {
                return shield_review::capture(rt, context);
            }
        }
        if rt.map_review && rt.review_frames >= 36 {
            return map_review::capture(rt, context);
        }
        if rt.deathcam_review && rt.review_frames > 0 {
            rt.review_frames += 1;
        }
        if rt.deathcam_review && rt.review_frames >= 12 {
            return deathcam_review::capture(rt, context);
        }
        if (rt.home_review || rt.lobby.review)
            && rt.review_frames >= 36
            && rt.hud.home.ready()
            && rt.hud.lobby.ready()
            && context
                .graphics_context
                .as_initialized_ref()
                .renderer
                .scene_data_map
                .contains_key(&rt.scene)
        {
            let size = context.user_interfaces.first().screen_size();
            let tag = format!(
                "{}-{}x{}-step{}-",
                if rt.lobby.review { "lobby" } else { "home" },
                size.x as u32,
                size.y as u32,
                rt.review_capture
            );
            capture_preview_impl(rt.scene, 0, &tag, rt.home_open || rt.lobby.open, context)
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            rt.review_capture += 1;
            rt.review_frames = 0;
            return Ok(());
        }
        if rt.hud_review && rt.review_frames >= 24 {
            let prefix = if std::env::args().any(|a| a == "--hud-review-small") {
                "hud-small"
            } else {
                "hud"
            };
            let tag = if rt.review_capture < 15 {
                format!(
                    "{prefix}-{:02}-{}-",
                    rt.review_capture,
                    characters::SLUGS[rt.review_capture as usize]
                )
            } else {
                format!("{prefix}-state-{}-", rt.review_capture - 15)
            };
            capture_preview(rt.scene, 1, &tag, context)
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            rt.review_capture += 1;
            rt.review_frames = 0;
            return Ok(());
        }
        if rt.weapon_review && rt.review_frames >= 24 {
            return weapon_review::capture(rt, context);
        }
        if rt.greeting_review && rt.review_frames >= 24 {
            return greeting_review::capture(rt, context);
        }
        if rt.dance_review && rt.review_frames >= 12 {
            return dance_review::capture(rt, context);
        }
        if rt.attack_review && rt.review_frames >= 6 {
            return attack_review::capture(rt, context);
        }
        if rt.naia_skill_review && rt.review_frames >= 8 {
            return naia_skill_review::capture(rt, context);
        }
        if rt.portal_review && rt.review_frames >= 8 {
            return portal_review::capture(rt, context);
        }
        if rt.communication_review && rt.review_frames >= 36 {
            if rt.review_capture >= 8 {
                let g = &context.scenes[rt.scene].graph;
                let camera_position = **g[rt.camera].local_transform().position();
                if rt.orbit.active
                    || !g[rt.wave_hand].visibility()
                    || !rt.first_person_arm.as_ref().is_some_and(|arm| {
                        arm.character == rt.session.world().actors[rt.session.slot()].character
                            && arm.triangles > 0
                    })
                    || !rt.visuals[rt.session.slot()].is_shadow_only()
                    || (camera_position - vec(rt.view_pos)).norm() > 0.001
                {
                    return Err(fyrox::plugin::error::GameError::str(
                        "Hello must keep the first-person camera and show the waving hand",
                    ));
                }
            }
            let tag = match rt.review_capture {
                n @ 0..=7 => format!("communication-wheel-{n}-"),
                8 => "communication-first-person-".into(),
                n => format!(
                    "communication-first-person-{}-",
                    characters::SLUGS[(n - 9) as usize]
                ),
            };
            capture_preview(rt.scene, 1, &tag, context)
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            rt.review_capture += 1;
            rt.review_frames = 0;
            return Ok(());
        }
        if rt.cheek_review {
            return cheek_review::capture(rt, context);
        }
        if rt.selection_review && rt.review_frames >= 36 {
            let blink = std::env::args().any(|a| a == "--blink-review");
            let steps = if blink { 3 } else { 2 };
            let id = rt.review_capture / steps;
            let tag = format!(
                "{}-{:02}-{}-{}-",
                if blink {
                    "blink"
                } else if std::env::args().any(|a| a == "--face-review") {
                    "face"
                } else {
                    "selection"
                },
                id,
                characters::SLUGS[id as usize],
                rt.review_capture % steps
            );
            capture_preview(rt.scene, 0, &tag, context)
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            rt.review_capture += 1;
            rt.review_frames = 0;
            return Ok(());
        }
        if rt.art_review && rt.review_frames >= 25 {
            let id = rt.review_capture / 2;
            let side = rt.review_capture % 2;
            if !rt.orbit.active {
                return Err(fyrox::plugin::error::GameError::str(
                    "Art review camera inactive",
                ));
            }
            let tag = format!("gallery-{:02}-{}-", id, characters::SLUGS[id as usize]);
            capture_preview(rt.scene, 3 + side, &tag, context)
                .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
            rt.review_capture += 1;
            rt.review_frames = 0;
            return Ok(());
        }
        if rt.smoke.is_some()
            && rt.capture_stage < 5
            && rt.started.elapsed().as_secs_f32()
                > match rt.capture_stage {
                    0 => 1.0,
                    1 => 4.0,
                    2 => 6.0,
                    3 => 8.0,
                    _ => 10.0,
                }
        {
            if rt.capture_stage >= 3 && !rt.orbit.active {
                return Ok(());
            }
            // Shader warmup may exceed one second when two instances start at
            // once. Wait for the first completed scene frame before readback.
            if !context
                .graphics_context
                .as_initialized_ref()
                .renderer
                .scene_data_map
                .contains_key(&rt.scene)
            {
                return Ok(());
            }
            let tag = match rt.session {
                Session::Solo(_) => "",
                Session::Host(_) => "host-",
                Session::Join(_) => "client-",
            };
            match capture_preview(rt.scene, rt.capture_stage, tag, context) {
                Ok(()) => rt.capture_stage += 1,
                Err(error) => {
                    eprintln!("PREVIEW_ERROR: {error}");
                    rt.capture_stage = 255;
                }
            }
        }
        Ok(())
    }
    fn init(&mut self, _path: Option<&str>, context: PluginContext) -> GameResult {
        // Fyrox can dispatch window/update callbacks before plugin init while
        // the resource registry loads. Publish the runtime only after its
        // scene and UI exist, so those callbacks keep taking their None path.
        let mut runtime = self
            .pending_runtime
            .take()
            .expect("Game runtime is constructed by main");
        let rt = &mut runtime;
        rt.models = characters::load(context.resource_manager)
            .map_err(fyrox::plugin::error::GameError::str)?;
        rt.weapon_models = weapons::load(context.resource_manager)
            .map_err(fyrox::plugin::error::GameError::str)?;
        let scene = populate_scene(rt, context.resource_manager)
            .map_err(fyrox::plugin::error::GameError::str)?;
        rt.scene = context.scenes.add(scene);
        context
            .user_interfaces
            .add(UserInterface::new(Vector2::new(1280.0, 800.0)));
        rt.hud = make_hud(context.user_interfaces.first_mut());
        rt.hud.player_info = player_info::PlayerInfoUi::new(
            context.user_interfaces.first_mut(),
            context.resource_manager,
        );
        rt.hud.combat = combat_hud::CombatHud::new(context.user_interfaces.first_mut());
        rt.hud.objective = objective_hud::ObjectiveHud::new(context.user_interfaces.first_mut());
        rt.hud.ultimate = ultimate_hud::UltimateHud::new(context.user_interfaces.first_mut());
        rt.hud.selection = selection::SelectionUi::new(
            context.user_interfaces.first_mut(),
            context.resource_manager,
        );
        rt.hud.communication =
            communication::CommunicationUi::new(context.user_interfaces.first_mut());
        rt.hud.home = home::HomeUi::new(
            context.user_interfaces.first_mut(),
            context.resource_manager,
        );
        rt.hud.voice = voice::VoiceUi::new(
            context.user_interfaces.first_mut(),
            context.resource_manager,
        );
        rt.hud.lobby = lobby::LobbyUi::new(
            context.user_interfaces.first_mut(),
            context.resource_manager,
        );
        // Use the project font consistently across every UI, including home menus.
        rt.hud.rooms = rooms::BrowserUi::new(context.user_interfaces.first_mut());
        rt.hud.account = account::AccountUi::new(context.user_interfaces.first_mut());
        rt.hud.round_results = round_results::ResultUi::new(
            context.user_interfaces.first_mut(),
            context.resource_manager,
        );
        // Shared overlay must be drawn above the lobby and map-selection modal.
        rt.hud.chat = chat::ChatUi::new(context.user_interfaces.first_mut());
        rt.hud.settings = settings_menu::SettingsUi::new(context.user_interfaces.first_mut());
        // The startup gate owns input and must cover every other overlay.
        rt.hud.startup = startup::StartupUi::new(
            context.user_interfaces.first_mut(),
            context.resource_manager,
        );
        let font_path = "assets/fonts/ONE Mobile POP.ttf";
        let font_bytes = std::fs::read(font_path).map_err(|error| {
            fyrox::plugin::error::GameError::str(format!("Failed to read {font_path}: {error}"))
        })?;
        let font =
            Font::from_memory(font_bytes, 1024, Default::default(), vec![]).map_err(|error| {
                fyrox::plugin::error::GameError::str(format!("Failed to load {font_path}: {error}"))
            })?;
        let font = FontResource::new_embedded(font);
        let ui = context.user_interfaces.first_mut();
        ui.default_font = font.clone();
        for h in rt
            .hud
            .selection
            .labels()
            .into_iter()
            .chain([
                rt.hud.brand,
                rt.hud.connection,
                rt.hud.menu,
                rt.hud.banner,
                rt.hud.status,
                rt.hud.scoreboard,
            ])
            .chain(rt.hud.player_info.labels())
            .chain(rt.hud.nameplates.labels())
            .chain(rt.hud.combat.labels())
            .chain(rt.hud.objective.labels())
            .chain(rt.hud.ultimate.labels())
            .chain(rt.hud.chat.labels())
            .chain(rt.hud.communication.labels())
            .chain(rt.hud.home.labels())
            .chain(rt.hud.startup.labels())
            .chain(rt.hud.lobby.labels())
            .chain(rt.hud.rooms.labels())
            .chain(rt.hud.account.labels())
            .chain(rt.hud.voice.labels())
            .chain(rt.hud.settings.labels())
            .chain(rt.hud.round_results.labels())
        {
            ui.send(h, TextMessage::Font(font.clone()));
            let size = **ui[h]
                .cast::<Text>()
                .expect("Text label")
                .formatted_text
                .borrow()
                .font_size();
            text_size(ui, h, size);
        }
        ui.send(
            rt.hud.connection,
            TextMessage::HorizontalAlignment(fyrox::gui::HorizontalAlignment::Right),
        );
        rt.korean_font = true;
        if rt.auto_start && rt.smoke.is_none() {
            rt.session.start();
        }
        if rt.smoke.is_some() {
            rt.selected = if matches!(rt.session, Session::Join(_)) {
                11
            } else {
                4
            };
            rt.confirm_selection();
        }
        rt.started = Instant::now();
        if rt.communication_review {
            rt.selecting = false;
            rt.grabbed = true;
            for action in Communication::ALL
                .into_iter()
                .filter(|a| *a != Communication::VoiceLine)
            {
                if let Session::Solo(world) = &mut rt.session {
                    world.actors[0].communication_cooldown = 0.0;
                }
                rt.communicate(action);
            }
        }
        self.runtime = Some(runtime);
        Ok(())
    }
    fn on_os_event(&mut self, event: &Event<()>, mut context: PluginContext) -> GameResult {
        let Some(rt) = &mut self.runtime else {
            return Ok(());
        };
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::ModifiersChanged(modifiers) => rt.modifiers = modifiers.state(),
                WindowEvent::Focused(focused) => {
                    rt.voice.focus(*focused);
                    if *focused && !rt.home_review && !rt.lobby.review && !rt.account.review {
                        rt.browser.refresh_profile();
                    }
                    if !focused {
                        if let Some(preview) = &mut rt.preview {
                            preview.cheeks.cancel();
                        }
                        rt.modifiers = ModifiersState::default();
                    }
                }
                WindowEvent::Resized(size) => {
                    if let Some(preview) = &mut rt.preview {
                        preview.cheeks.cancel();
                    }
                    if startup::resize(
                        rt,
                        context.user_interfaces.first_mut(),
                        Vector2::new(size.width as f32, size.height as f32),
                    ) || home::resize(
                        rt,
                        context.user_interfaces.first_mut(),
                        Vector2::new(size.width as f32, size.height as f32),
                    ) {
                        context
                            .graphics_context
                            .as_initialized_ref()
                            .window
                            .request_redraw();
                    }
                }
                WindowEvent::CursorLeft { .. } => {
                    if let Some(preview) = &mut rt.preview {
                        preview.cheeks.cancel();
                    }
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.physical_key == PhysicalKey::Code(KeyCode::KeyM)
                        && !rt.chat.open
                        && !rt.settings_menu.open
                        && !rt.home_open
                        && !rt.startup.active()
                        && !rt.lobby.open =>
                {
                    rt.voice
                        .key(event.state == ElementState::Pressed, event.repeat, false);
                    return Ok(());
                }
                _ => {}
            }
        }
        if startup::on_event(rt, event, &context) {
            return Ok(());
        }
        if account::on_event(rt, event, &context) {
            return Ok(());
        }
        if settings_menu::on_event(rt, event, &mut context) {
            if let Some(preview) = &mut rt.preview {
                preview.cheeks.cancel();
            }
            return Ok(());
        }
        // Home consumes input before gameplay, selection, IME and chat dispatch.
        if rt.home_open {
            let mut action = None;
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    rt.cursor = Vector2::new(position.x as f32, position.y as f32);
                    if let Some(hover) =
                        home::hit_action(context.user_interfaces.first().screen_size(), rt.cursor)
                    {
                        rt.home_focus = hover.menu_index();
                    }
                }
                Event::WindowEvent {
                    event:
                        WindowEvent::MouseInput {
                            button: MouseButton::Left,
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    action =
                        home::hit_action(context.user_interfaces.first().screen_size(), rt.cursor);
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { event, .. },
                    ..
                } if event.state == ElementState::Pressed && !event.repeat => {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ArrowDown | KeyCode::ArrowUp | KeyCode::Tab) => {
                            let choices = [0, 1, 5, 6, 8];
                            let index = choices
                                .iter()
                                .position(|&i| i == rt.home_focus)
                                .unwrap_or(0);
                            let step = if event.physical_key == PhysicalKey::Code(KeyCode::ArrowUp)
                            {
                                choices.len() - 1
                            } else {
                                1
                            };
                            rt.home_focus = choices[(index + step) % choices.len()];
                        }
                        PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
                            action = Some(home::Action::from_index(rt.home_focus));
                        }
                        PhysicalKey::Code(KeyCode::F10) => action = Some(home::Action::Quit),
                        _ => {}
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Focused(false),
                    ..
                } => {
                    rt.keys = Keys::default();
                    set_cursor(&context.graphics_context.as_initialized_ref().window, false);
                }
                _ => {}
            }
            match action {
                Some(home::Action::Play) => {
                    rt.play_from_home(&context.graphics_context.as_initialized_ref().window)
                }
                Some(home::Action::Training) => {
                    rt.practice_room();
                    set_cursor(&context.graphics_context.as_initialized_ref().window, false);
                }
                Some(home::Action::Quit) => context.loop_controller.exit(),
                Some(home::Action::Social | home::Action::Profile) => {
                    account::open(rt, action == Some(home::Action::Profile));
                }
                None => {}
            }
            return Ok(());
        }
        if chat::on_event(rt, event, &context) {
            return Ok(());
        }
        if rt.lobby.open {
            if rt.browser.open {
                rooms::on_event(rt, event, &context);
            } else {
                lobby::on_event(rt, event, &context);
            }
            return Ok(());
        }
        if rt.session.world().in_round_transition() {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { event, .. },
                    ..
                } => match event.physical_key {
                    PhysicalKey::Code(KeyCode::Tab) => {
                        rt.keys.score = event.state == ElementState::Pressed;
                    }
                    PhysicalKey::Code(KeyCode::F10) if event.state == ElementState::Pressed => {
                        context.loop_controller.exit();
                    }
                    _ => {}
                },
                Event::WindowEvent {
                    event: WindowEvent::Focused(false),
                    ..
                } => rt.keys = Keys::default(),
                _ => {}
            }
            return Ok(());
        }
        match event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } if rt.wheel.open => {
                rt.wheel.drag(delta.0 as f32, delta.1 as f32);
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } if rt.grabbed
                && !rt.chat.open
                && rt.session.world().actors[rt.session.slot()].alive() =>
            {
                if rt.orbit.active {
                    rt.orbit.rotate(delta.0 as f32, delta.1 as f32);
                } else {
                    let sensitivity = rt.look_sensitivity();
                    rt.yaw -= delta.0 as f32 * sensitivity;
                    rt.pitch = (rt.pitch - delta.1 as f32 * sensitivity).clamp(-1.45, 1.45);
                }
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Focused(false) => {
                    rt.wheel.cancel();
                    if rt.chat.open {
                        rt.close_chat(&context.graphics_context.as_initialized_ref().window, false);
                    }
                    rt.grabbed = false;
                    rt.keys = Keys::default();
                    set_cursor(&context.graphics_context.as_initialized_ref().window, false);
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    let down = event.state == ElementState::Pressed;
                    if rt.wheel.open {
                        let window = &context.graphics_context.as_initialized_ref().window;
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::KeyC) if !down => {
                                rt.end_communication(window, true)
                            }
                            PhysicalKey::Code(KeyCode::Escape) if down => {
                                rt.end_communication(window, false)
                            }
                            PhysicalKey::Code(KeyCode::F10) if down => {
                                context.loop_controller.exit()
                            }
                            _ => {}
                        }
                        return Ok(());
                    }
                    if let PhysicalKey::Code(code) = event.physical_key {
                        match code {
                            KeyCode::KeyC if down && !event.repeat && !rt.selecting => {
                                rt.begin_communication(
                                    &context.graphics_context.as_initialized_ref().window,
                                );
                            }
                            KeyCode::KeyW => rt.keys.w = down,
                            KeyCode::KeyS => rt.keys.s = down,
                            KeyCode::KeyA => rt.keys.a = down,
                            KeyCode::KeyD => rt.keys.d = down,
                            KeyCode::Space => rt.keys.jump = down,
                            KeyCode::ShiftLeft => rt.keys.phase_left = down,
                            KeyCode::ShiftRight => rt.keys.phase_right = down,
                            KeyCode::ControlLeft => rt.keys.crouch_left = down,
                            KeyCode::ControlRight => rt.keys.crouch_right = down,
                            KeyCode::KeyR => rt.keys.reload = down,
                            KeyCode::KeyE => rt.keys.ability = down,
                            KeyCode::KeyQ => rt.keys.ultimate = down,
                            KeyCode::KeyG => rt.keys.dance = down,
                            KeyCode::Tab => rt.keys.score = down,
                            KeyCode::KeyH if down && !event.repeat => {
                                let window = &context.graphics_context.as_initialized_ref().window;
                                if rt.selecting {
                                    rt.cancel_selection(window);
                                } else if rt.session.world().can_select(rt.session.slot()) {
                                    rt.selecting = true;
                                    rt.grabbed = false;
                                    rt.keys = Keys::default();
                                    rt.selected = rt.session.world().actors[rt.session.slot()]
                                        .character
                                        as usize;
                                    rt.selection_message.clear();
                                    rt.preview_serial += 1;
                                    set_cursor(window, false);
                                } else {
                                    rt.selection_message = SelectionResult::Locked.message().into();
                                }
                            }
                            KeyCode::ArrowLeft
                            | KeyCode::ArrowRight
                            | KeyCode::ArrowUp
                            | KeyCode::ArrowDown
                                if down && rt.selecting =>
                            {
                                let offset = match code {
                                    KeyCode::ArrowLeft => -1,
                                    KeyCode::ArrowRight => 1,
                                    KeyCode::ArrowUp => -5,
                                    _ => 5,
                                };
                                rt.selected = selection::navigate(rt.selected, offset);
                                rt.preview_serial += 1;
                                if let Some(preview) = &mut rt.preview {
                                    preview.cheeks.cancel();
                                }
                                rt.selection_message.clear();
                            }
                            KeyCode::F5 if down && !event.repeat && rt.selecting => {
                                rt.confirm_selection();
                            }
                            KeyCode::F5 if down && !event.repeat => {
                                if matches!(
                                    rt.session.world().phase,
                                    Phase::Lobby | Phase::Finished
                                ) {
                                    rt.session.start();
                                }
                                rt.selecting = false;
                                rt.grabbed = true;
                                set_cursor(
                                    &context.graphics_context.as_initialized_ref().window,
                                    true,
                                );
                            }
                            KeyCode::Escape if down && !event.repeat && rt.selecting => {
                                rt.cancel_selection(
                                    &context.graphics_context.as_initialized_ref().window,
                                );
                            }
                            KeyCode::F10 if down => context.loop_controller.exit(),
                            _ => {}
                        }
                    }
                }
                WindowEvent::MouseInput {
                    button: MouseButton::Right,
                    state: ElementState::Pressed,
                    ..
                } if rt.wheel.open => {
                    rt.end_communication(
                        &context.graphics_context.as_initialized_ref().window,
                        false,
                    );
                }
                WindowEvent::MouseInput { .. } | WindowEvent::MouseWheel { .. }
                    if rt.wheel.open => {}
                WindowEvent::MouseInput {
                    button: MouseButton::Right,
                    state,
                    ..
                } => {
                    rt.keys.alt_fire = *state == ElementState::Pressed;
                    rt.set_zoom(*state == ElementState::Pressed);
                }
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => {
                    if rt.selecting && *state == ElementState::Released {
                        rt.keys.fire = false;
                        if let Some(preview) = &mut rt.preview {
                            preview.cheeks.interaction.release();
                        }
                    } else if rt.selecting && *state == ElementState::Pressed {
                        rt.keys.fire = false;
                        if selection::showing(rt)
                            && rt.preview.as_mut().is_some_and(|p| {
                                p.cheeks.id as usize == rt.selected && p.cheeks.press(rt.cursor)
                            })
                        {
                            return Ok(());
                        }
                        let size = context.user_interfaces.first().screen_size();
                        for id in 0..ROSTER.len() {
                            let (x, y, w, h) = selection::card_rect(size.x, size.y, id);
                            if rt.cursor.x >= x
                                && rt.cursor.x <= x + w
                                && rt.cursor.y >= y
                                && rt.cursor.y <= y + h
                            {
                                rt.selected = id;
                                rt.preview_serial += 1;
                                if let Some(preview) = &mut rt.preview {
                                    preview.cheeks.cancel();
                                }
                                rt.selection_message.clear();
                                break;
                            }
                        }
                        if selection::contains(rt.cursor, selection::confirm_rect(size.x, size.y)) {
                            rt.confirm_selection();
                        } else if selection::contains(
                            rt.cursor,
                            selection::back_rect(size.x, size.y),
                        ) {
                            rt.cancel_selection(
                                &context.graphics_context.as_initialized_ref().window,
                            );
                        }
                    } else if !rt.selecting && !rt.grabbed && *state == ElementState::Pressed {
                        rt.grabbed = true;
                        set_cursor(&context.graphics_context.as_initialized_ref().window, true);
                    } else {
                        rt.keys.fire = *state == ElementState::Pressed;
                    }
                }
                WindowEvent::MouseWheel { delta, .. } if rt.orbit.active && rt.grabbed => {
                    let steps = match delta {
                        MouseScrollDelta::LineDelta(_, y) => *y,
                        MouseScrollDelta::PixelDelta(p) => p.y as f32 / 80.0,
                    };
                    rt.orbit.zoom(steps);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    rt.cursor = Vector2::new(position.x as f32, position.y as f32);
                    if let Some(preview) = &mut rt.preview {
                        preview.cheeks.drag(rt.cursor);
                    }
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
    fn on_ui_message(
        &mut self,
        context: &mut PluginContext,
        message: &fyrox::gui::message::UiMessage,
        _ui_handle: Handle<UserInterface>,
    ) -> GameResult {
        if let Some(rt) = &mut self.runtime {
            account::on_ui_message(rt, context.user_interfaces.first_mut(), message);
        }
        Ok(())
    }
    fn update(&mut self, context: &mut PluginContext) -> GameResult {
        if context.loop_controller.exiting() {
            return Ok(());
        }
        let Some(rt) = &mut self.runtime else {
            return Ok(());
        };
        rt.chat.tick(context.dt);
        if !rt.account.review {
            rooms::update(rt);
            account::update(rt);
        }
        account::update_picker(rt, context.user_interfaces.first_mut());
        if startup::update(rt, context)? {
            return Ok(());
        }
        rooms::sync_ime(rt, &context.graphics_context.as_initialized_ref().window);
        account::sync_ime(rt, &context.graphics_context.as_initialized_ref().window);
        sync_map_scene(rt, context)?;
        if rt.settings_review {
            settings_menu::review_update(rt, context);
        }
        if rt.round_results_review {
            return round_results::review_update(rt, context);
        }
        if rt.objective_hud_review {
            return objective_hud::review_update(rt, context);
        }
        if rt.nameplates_review {
            return nameplates::review::update(rt, context);
        }
        if rt.voice_review {
            return voice::review::update(rt, context);
        }
        if rt.greeting_review {
            return greeting_review::update(rt, context);
        }
        if rt.dance_review {
            return dance_review::update(rt, context);
        }
        if rt.attack_review {
            return attack_review::update(rt, context);
        }
        if rt.naia_skill_review {
            return naia_skill_review::update(rt, context);
        }
        if rt.portal_review {
            return portal_review::update(rt, context);
        }
        if rt.impact_review {
            return impact_review::update(rt, context);
        }
        if rt.shadow_review {
            return shadow_review::update(rt, context);
        }
        if rt.cheek_review {
            return cheek_review::update(rt, context);
        }
        if rt.weapon_review {
            return weapon_review::update(rt, context);
        }
        if rt.shield_review {
            return shield_review::update(rt, context);
        }
        if rt.ghost_review {
            return ghost_review::update(rt, context);
        }
        if rt.map_review {
            return map_review::update(rt, context);
        }
        if rt.deathcam_review {
            return deathcam_review::update(rt, context);
        }
        if rt.account.review {
            account::review_update(rt, context);
        } else if rt.home_review || rt.lobby.review {
            lobby::review(rt, context);
        }
        if rt.hud_review {
            if rt.review_capture >= 47 {
                println!("HUD_REVIEW_OK: 15 portraits/skills/weapons; health, recovery, selection hidden; cooldown, reload, empty ammo, disabled, magic; ultimate 0/25/50/75/99/100%, ready, disabled, dead, activated and reset");
                context.loop_controller.exit();
                return Ok(());
            }
            rt.review_frames = rt.review_frames.max(1);
            // Keep captures deterministic if desktop input reaches the review window.
            rt.settings_menu.open = false;
            rt.chat.open = false;
            rt.keys.score = false;
            rt.wheel.cancel();
            rt.yaw = 0.0;
            rt.pitch = 0.0;
            rt.grabbed = true;
            rt.selecting = matches!(rt.review_capture, 19 | 46);
            rt.selected = 11;
            if let Session::Solo(world) = &mut rt.session {
                world.phase = Phase::Lobby;
                let actor = &mut world.actors[0];
                actor.character =
                    if rt.review_capture == 25 || (37..47).contains(&rt.review_capture) {
                        14
                    } else if rt.review_capture < 15 {
                        rt.review_capture
                    } else {
                        11
                    };
                actor.pos = V3::new(0.0, 0.0, -12.0);
                actor.hp = match rt.review_capture {
                    15 => 72,
                    16 => 24,
                    17 | 34 | 45 => 0,
                    _ => MAX_HP,
                };
                actor.combat.regen = if matches!(rt.review_capture, 15 | 17 | 18) {
                    4.0
                } else {
                    0.0
                };
                actor.ammo = match rt.review_capture {
                    15 | 20 | 21 | 26 => 7,
                    22 | 23 => 0,
                    25 => 3,
                    _ => kit(actor.character).magazine,
                };
                actor.ability_cooldown = match rt.review_capture {
                    20 => (7.4 - rt.review_frames as f32 * DT).max(0.0),
                    21 => 0.6,
                    _ => 0.0,
                };
                actor.reload = if rt.review_capture == 22 { 0.8 } else { 0.0 };
                actor.combat.stun = if matches!(rt.review_capture, 24 | 33 | 44) {
                    1.0
                } else {
                    0.0
                };
                actor.combat.magic_ammo = if rt.review_capture == 25 { 3 } else { 0 };
                actor.combat.ultimate = match rt.review_capture {
                    0..=14 => rt.review_capture as f32 * 100.0 / 14.0,
                    17 | 19 | 24 | 26 | 32..=34 => 100.0,
                    27 | 35 | 36 => 0.0,
                    28 => 25.0,
                    29 => 50.0,
                    30 => 75.0,
                    31 => 99.9,
                    _ => 64.0,
                };
                actor.combat.ultimate_flash = if rt.review_capture == 35 { 1.0 } else { 0.0 };
                actor.phase_skill = Default::default();
                actor.phase_skill.remaining = match rt.review_capture {
                    38 => 3.0,
                    39 => 1.5,
                    40 => 0.4,
                    _ => 0.0,
                };
                actor.phase_skill.cooldown = match rt.review_capture {
                    41 => 12.0,
                    42 => 6.0,
                    43 => 0.4,
                    _ => 0.0,
                };
            }
            render(rt, context)?;
            return Ok(());
        }
        rt.communication_notice_time = (rt.communication_notice_time - context.dt).max(0.0);
        if rt.communication_review {
            if rt.review_capture >= 24 {
                println!("COMMUNICATION_REVIEW_OK: 8 wheel directions, 15 first-person greetings, camera and hand visibility verified");
                context.loop_controller.exit();
                return Ok(());
            }
            rt.review_frames = rt.review_frames.max(1);
            if let Session::Solo(world) = &mut rt.session {
                world.phase = Phase::Lobby;
                for (i, actor) in world.actors.iter_mut().enumerate() {
                    actor.human = true;
                    actor.pos = V3::new(40.0 + i as f32 * 3.0, 0.0, 30.0);
                }
                let actor = &mut world.actors[0];
                actor.pos = V3::new(0.0, 0.0, -12.0);
                actor.yaw = 0.0;
                actor.character = rt.review_capture.saturating_sub(9);
                actor.dancing = false;
                actor.wave_ticks = if rt.review_capture >= 8 {
                    108 - rt.review_frames.min(100) as u8
                } else {
                    0
                };
            }
            if rt.review_capture < 8 {
                rt.wheel.begin();
                let angle = rt.review_capture as f32 * std::f32::consts::FRAC_PI_4;
                rt.wheel.drag(angle.sin() * 180.0, -angle.cos() * 180.0);
            } else {
                rt.wheel.cancel();
            }
            rt.chat.touch();
        }
        rt.sync_chat();
        let revision = rt.chat_history().revision;
        if revision != rt.chat_revision {
            rt.chat_revision = revision;
            rt.chat.touch();
            if !rt.chat.open {
                rt.chat_scroll = 0;
            }
        }
        if rt.selection_review {
            let steps = if std::env::args().any(|a| a == "--blink-review") {
                3
            } else {
                2
            };
            if rt.review_capture >= 15 * steps {
                if steps == 3 {
                    println!("BLINK_REVIEW_OK: 15 heroes, 45 open/half/closed GPU frames, coupled eyebrows");
                }
                println!("SELECTION_REVIEW_OK: 15 heroes, 30 selection motion/UI frames");
                if std::env::args().any(|a| a == "--face-review") {
                    println!("FACE_REVIEW_OK: 15 heroes, 345 imported morph controls, 30 neutral/closed-eye GPU frames");
                }
                context.loop_controller.exit();
                return Ok(());
            }
            rt.selecting = true;
            rt.selected = (rt.review_capture / steps) as usize;
            rt.review_frames = rt.review_frames.max(1);
        }
        if rt.selecting && !rt.selection_review && !rt.session.world().can_select(rt.session.slot())
        {
            rt.close_selection(&context.graphics_context.as_initialized_ref().window);
        }
        if rt.art_review {
            if rt.review_capture >= 30 {
                println!("ART_REVIEW_OK: all 15 FBX models, 30 front/back dance frames");
                context.loop_controller.exit();
                return Ok(());
            }
            if let Session::Solo(w) = &mut rt.session {
                // Keep the art fixture out of live bot combat while orbiting each model.
                w.phase = Phase::Lobby;
                w.actors[0].hp = MAX_HP;
                w.actors[0].respawn = 0.0;
                w.actors[0].wave_ticks = 0;
                w.actors[0].character = rt.review_capture / 2;
                w.actors[0].pos = V3::new(0.0, 0.0, -12.0);
                w.actors[0].yaw = 0.0;
                w.actors[0].dancing = true;
            }
            rt.selecting = false;
            rt.orbit.active = true;
            rt.orbit.distance = 3.1;
            rt.orbit.pitch = -0.12;
            rt.orbit.yaw = if rt.review_capture.is_multiple_of(2) {
                std::f32::consts::PI
            } else {
                0.0
            };
            rt.review_frames = rt.review_frames.max(1);
        }
        let controls_active = rt.controls_active();
        let mut input = Input {
            yaw: rt.yaw - rt.portal_view_offset.x,
            pitch: rt.pitch - rt.portal_view_offset.y,
            forward: if controls_active {
                (rt.keys.w as i8 - rt.keys.s as i8) as f32
            } else {
                0.0
            },
            strafe: if controls_active {
                (rt.keys.d as i8 - rt.keys.a as i8) as f32
            } else {
                0.0
            },
            fire: controls_active && rt.keys.fire,
            alt_fire: controls_active && rt.keys.alt_fire,
            jump: controls_active && rt.keys.jump,
            phase_shift: controls_active && (rt.keys.phase_left || rt.keys.phase_right),
            crouch: controls_active && (rt.keys.crouch_left || rt.keys.crouch_right),
            reload: controls_active && rt.keys.reload,
            ability: controls_active && rt.keys.ability,
            ultimate: controls_active && rt.keys.ultimate,
            dance: controls_active && rt.keys.dance,
        };
        if rt.smoke.is_some() {
            let time = rt.started.elapsed().as_secs_f32();
            rt.keys.score = rt.capture_stage == 2;
            if time > 2.0 && rt.session.world().phase == Phase::Lobby {
                rt.session.start();
                let world = match &mut rt.session {
                    Session::Solo(w) => Some(w.as_mut()),
                    Session::Host(s) => Some(&mut s.world),
                    Session::Join(_) => None,
                };
                if let Some(w) = world {
                    w.phase = Phase::Playing;
                    w.actors[0].pos = V3::new(0.0, 0.0, -12.0);
                    if w.actors[6].human {
                        w.actors[6].pos = V3::new(0.0, 0.0, 26.0);
                    }
                    // Test fixture only: exercise Q in a short real GPU/UDP run.
                    for a in &mut w.actors {
                        if a.human {
                            a.combat.ultimate = 100.0;
                            a.shield = 30.0;
                        }
                    }
                }
            }
            input.fire = (2.0..5.0).contains(&time);
            input.forward = if (2.0..4.0).contains(&time) { 0.2 } else { 0.0 };
            input.reload = (5.0..5.2).contains(&time);
            input.ability = (3.0..3.2).contains(&time);
            input.ultimate = (3.6..4.2).contains(&time);
            input.dance = (7.0..7.3).contains(&time);
            // Keep the visual inspection fixture alive after combat actions remove spawn protection.
            if (6.0..11.0).contains(&time) {
                let world = match &mut rt.session {
                    Session::Solo(w) => Some(w.as_mut()),
                    Session::Host(s) => Some(&mut s.world),
                    Session::Join(_) => None,
                };
                if let Some(w) = world {
                    for a in &mut w.actors {
                        if a.human {
                            a.shield = 2.0;
                        }
                    }
                }
            }
            if (8.3..10.5).contains(&time) && rt.orbit.active {
                rt.orbit.rotate(-12.0, 0.0);
                rt.orbit.zoom(-0.008);
            }
            if (11.0..11.5).contains(&time) {
                input.forward = 0.2;
            }
        }
        if let Err(e) = rt.session.update(input) {
            rt.error = Some(e);
        }
        if rt.session.world().phase == Phase::Intermission && rt.last_phase != Phase::Intermission {
            rt.keys = Keys::default();
            rt.wheel.cancel();
            rt.selecting = false;
            rt.pending_ready = None;
            rt.settings_menu.open = false;
        }
        if rt.sync_match_end() {
            set_cursor(&context.graphics_context.as_initialized_ref().window, false);
        }
        if rt.sync_match_start() {
            set_cursor(&context.graphics_context.as_initialized_ref().window, false);
        }
        rt.sync_chat();
        sync_map_scene(rt, context)?;
        rt.voice.update(
            &mut rt.session,
            rt.home_open || rt.lobby.open,
            rt.error.is_some(),
        );
        if let Session::Join(client) = &mut rt.session {
            if let Some(result) = client.communication_result.take() {
                rt.communication_feedback(result);
            }
        }
        if rt.smoke.is_some()
            && rt.session.world().actors[rt.session.slot()]
                .combat
                .ultimate_flash
                > 0.0
        {
            rt.smoke_ult_used = true;
        }
        if let Some(id) = rt.pending_ready {
            let accepted = match &rt.session {
                Session::Join(c) => {
                    !c.selection_pending()
                        && c.selection_result == Some(SelectionResult::Accepted)
                        && c.world.actors[rt.session.slot()].character == id
                }
                _ => rt.session.world().actors[rt.session.slot()].character == id,
            };
            if accepted {
                let window = &context.graphics_context.as_initialized_ref().window;
                if rt.lobby.return_from_selection && !rt.lobby.start_requested {
                    rt.cancel_selection(window);
                } else {
                    if matches!(rt.session.world().phase, Phase::Lobby | Phase::Finished) {
                        rt.session.start();
                    }
                    rt.close_selection(window);
                }
            } else if matches!(&rt.session,Session::Join(c) if !c.selection_pending() && c.selection_result.is_some_and(|r|r!=SelectionResult::Accepted))
            {
                if let Session::Join(c) = &rt.session {
                    rt.selection_message = c.selection_result.unwrap().message().into();
                }
                rt.pending_ready = None;
            }
        }
        render(rt, context)?;
        if rt.smoke.is_some() {
            rt.smoke_dance_seen |= rt.orbit.active;
            if rt.started.elapsed().as_secs_f32() > 11.5 && rt.smoke_dance_seen && !rt.orbit.active
            {
                rt.smoke_dance_cancelled = true;
            }
        }
        if rt
            .smoke
            .is_some_and(|limit| rt.started.elapsed().as_secs_f32() > limit)
        {
            let connected = !matches!(&rt.session, Session::Join(client) if !client.received || client.error.is_some());
            let selected =
                rt.session.world().actors[rt.session.slot()].character as usize == rt.selected;
            if rt.capture_stage != 5
                || !connected
                || !selected
                || !rt.smoke_ult_used
                || !rt.smoke_dance_seen
                || !rt.smoke_dance_cancelled
                || rt.error.is_some()
            {
                eprintln!(
                    "SMOKE_FAILED: render, selection, connection, Q, dance and camera restoration checks must pass"
                );
                std::process::exit(1);
            }
            println!(
                "SMOKE_OK: rendered {} ticks, phase {:?}, {}, character={}, team_bonus={}%, actors={}, ultimate=OK, models=15, dance=OK, orbit=OK, cancel=OK",
                rt.session.world().tick,
                rt.session.world().phase,
                rt.session.label(),
                character(rt.session.world().actors[rt.session.slot()].character).name,
                rt.session.world().damage_bonus(team(rt.session.slot())),
                PLAYERS
            );
            context.loop_controller.exit();
        }
        Ok(())
    }
}

fn render(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let show_selection = selection::showing(rt);
    if let Some(preview) = &mut rt.preview {
        preview.update(
            &mut context.scenes[rt.scene],
            &rt.models,
            rt.selected as u8,
            rt.preview_serial,
            show_selection,
            context.dt,
        );
        if rt.selection_review
            && !std::env::args()
                .any(|arg| matches!(arg.as_str(), "--face-review" | "--blink-review"))
        {
            // Slow GPU captures must sample the source motion, not a later idle frame.
            preview.review_spawn(
                &mut context.scenes[rt.scene].graph,
                [0.25, 0.65][(rt.review_capture % 2) as usize],
            );
        }
    }
    let live = rt.session.world();
    let slot = rt.session.slot();
    let me = &live.actors[slot];
    let previous_stage = rt.deathcam.stage;
    let death_view = rt.deathcam.update(live, slot, context.dt);
    let top_down = rt.deathcam.stage == DeathStage::TopDown;
    let replaying = rt.deathcam.is_replay();
    let scene_changed = previous_stage != rt.deathcam.stage;
    let w = rt.deathcam.scene().unwrap_or(live);
    let result_active = live.in_round_transition();
    let result_started = result_active && rt.last_phase != live.phase;
    let world_dt = context.dt * live.presentation_time_scale();
    let poses: [Actor; PLAYERS] = std::array::from_fn(|i| {
        if death_view.is_some() {
            w.actors[i].clone()
        } else {
            match &rt.session {
                Session::Join(client) => client.motion.actor(w, i, slot),
                _ => w.actors[i].clone(),
            }
        }
    });
    let view_slot = death_view.map_or(slot, |v| v.target);
    let viewed = &w.actors[view_slot];
    let view_alive = !top_down && viewed.alive();
    for i in 0..PLAYERS {
        let id = w.actors[i].character;
        if rt.visuals[i].id != id {
            let mut new = characters::Visual::spawn(
                &mut context.scenes[rt.scene],
                rt.actors[i],
                &rt.models[id as usize],
                id,
                &Shapes::new(),
                rt.weapon_models[id as usize].as_ref(),
            );
            new.inherit_timeline(&rt.visuals[i]);
            let old = std::mem::replace(&mut rt.visuals[i], new);
            old.remove(&mut context.scenes[rt.scene].graph);
        }
    }
    if rt.first_person_arm.as_ref().is_none_or(|arm| {
        arm.character != viewed.character || arm.source != rt.visuals[view_slot].root
    }) {
        if let Some(old) = rt.first_person_arm.take() {
            old.remove(&mut context.scenes[rt.scene].graph);
        }
        if let Some(old) = rt.first_person_support_arm.take() {
            old.remove(&mut context.scenes[rt.scene].graph);
        }
        rt.first_person_arm = Some(
            first_person::FirstPersonArm::spawn(
                &mut context.scenes[rt.scene],
                &rt.models[viewed.character as usize],
                &rt.visuals[view_slot],
                rt.wave_hand,
                false,
            )
            .map_err(fyrox::plugin::error::GameError::str)?,
        );
        if viewed.character == 2 || weapons::grips(viewed.character)[1].is_some() {
            rt.first_person_support_arm = Some(
                first_person::FirstPersonArm::spawn(
                    &mut context.scenes[rt.scene],
                    &rt.models[viewed.character as usize],
                    &rt.visuals[view_slot],
                    rt.wave_hand,
                    true,
                )
                .map_err(fyrox::plugin::error::GameError::str)?,
            );
        }
    }
    let g = &mut context.scenes[rt.scene].graph;
    rt.ghost_walls.update(w, world_dt);
    rt.impacts.update(
        g,
        w,
        if rt.impact_review { 0.0 } else { world_dt },
        &rt.ghost_walls,
    );
    let portal_state = poses[slot].portals;
    let offset = Vector2::new(portal_state.yaw_offset, portal_state.pitch_offset);
    let teleported = rt.portal_transits != portal_state.transits;
    rt.yaw += offset.x - rt.portal_view_offset.x;
    rt.pitch = (rt.pitch + offset.y - rt.portal_view_offset.y).clamp(-1.45, 1.45);
    rt.portal_view_offset = offset;
    rt.portal_transits = portal_state.transits;
    if rt.last_round != w.round
        || (rt.last_dead && me.alive())
        || (rt.last_phase == Phase::Lobby && w.phase != Phase::Lobby)
    {
        rt.yaw = me.yaw;
        rt.pitch = me.pitch;
        rt.view_pos = me.eye();
        rt.last_round = w.round;
        rt.keys.zoom = false;
    }
    // A held attack or movement key must not carry across death or respawn.
    if rt.last_dead == me.alive() {
        rt.keys = Keys::default();
    }
    rt.last_dead = !me.alive();
    rt.last_phase = w.phase;
    // Interruptions require a new right-button press, even if it was held throughout.
    rt.keys.zoom &= rt.can_zoom();
    let zoomed = rt.zoomed();
    rt.view_pos = if teleported || matches!(rt.session, Session::Join(_)) {
        poses[slot].eye()
    } else {
        rt.view_pos + (me.eye() - rt.view_pos) * 0.55
    };
    // Hello always uses the player's eye camera, including during a transition out of dance.
    let waving = view_alive && viewed.wave_ticks > 0;
    if let Some(view) = death_view {
        rt.orbit.active = false;
        g[rt.camera]
            .local_transform_mut()
            .set_position(vec(view.position))
            .set_rotation(UnitQuaternion::from_euler_angles(
                -view.pitch,
                view.yaw,
                0.0,
            ));
    } else if me.dancing && me.alive() && !waving {
        if !rt.orbit.active {
            rt.orbit.enter(me.yaw);
        }
        g[rt.camera]
            .local_transform_mut()
            .set_position(vec(rt.orbit.position(poses[slot].pos, &w.map)))
            .set_rotation(UnitQuaternion::from_euler_angles(
                -rt.orbit.pitch,
                rt.orbit.yaw,
                0.0,
            ));
    } else {
        rt.orbit.active = false;
        g[rt.camera]
            .local_transform_mut()
            .set_position(vec(rt.view_pos))
            .set_rotation(UnitQuaternion::from_euler_angles(-rt.pitch, rt.yaw, 0.0));
    }
    if show_selection {
        // Keep both faces above the lower roster cards for the companion pairs.
        let (height, distance) = match rt.selected {
            6 => (1.00, 5.00),
            10 => (0.78, 3.65),
            _ => (1.18, 2.65),
        };
        g[rt.camera]
            .local_transform_mut()
            // Center the bust between the left team summary and right skill panel.
            .set_position(Vector3::new(120.06, height, distance))
            .set_rotation(UnitQuaternion::from_euler_angles(
                0.0,
                std::f32::consts::PI,
                0.0,
            ));
    }
    g[rt.camera]
        .cast_mut::<fyrox::scene::camera::Camera>()
        .unwrap()
        .set_projection(fyrox::scene::camera::Projection::Perspective(
            fyrox::scene::camera::PerspectiveProjection {
                z_near: 0.02,
                fov: if show_selection {
                    48.0_f32.to_radians()
                } else if death_view.is_some() {
                    70.0_f32.to_radians()
                } else {
                    rt.gameplay_fov()
                },
                ..Default::default()
            },
        ));
    let spawn = w.spawn_center(team(slot));
    for (i, node) in rt.spawn_edges.iter().enumerate() {
        let (offset, scale) = if i < 2 {
            (
                V3::new(
                    if i == 0 {
                        -SPAWN_HALF_WIDTH
                    } else {
                        SPAWN_HALF_WIDTH
                    },
                    0.015,
                    0.0,
                ),
                V3::new(0.07, 0.03, SPAWN_HALF_DEPTH * 2.0),
            )
        } else {
            (
                V3::new(
                    0.0,
                    0.015,
                    if i == 2 {
                        -SPAWN_HALF_DEPTH
                    } else {
                        SPAWN_HALF_DEPTH
                    },
                ),
                V3::new(SPAWN_HALF_WIDTH * 2.0, 0.03, 0.07),
            )
        };
        g[*node]
            .local_transform_mut()
            .set_position(vec(spawn + offset))
            .set_scale(vec(scale));
    }
    for (i, a) in poses.iter().enumerate() {
        g[rt.pulses[i]]
            .set_visibility(a.active && (a.ability_flash > 0.0 || a.combat.ultimate_flash > 0.0));
        let radius = if a.combat.ultimate_flash > 0.0 {
            3.0 + (w.tick as f32 * 0.12).sin().abs() * 4.0
        } else {
            (1.0 - a.ability_flash / 0.4).max(0.05) * 6.0
        };
        g[rt.pulses[i]]
            .local_transform_mut()
            .set_position(vec(a.pos + V3::new(0.0, 0.04, 0.0)))
            .set_scale(Vector3::new(radius, 0.025, radius));
        g[rt.actors[i]].set_visibility(a.active && (!rt.art_review || i == slot));
        // Global visibility also gates shadow passes. Keep the animated body and
        // world weapon alive, hiding only their camera passes in first person.
        rt.visuals[i].set_shadow_only(g, i == view_slot && !rt.orbit.active && !top_down);
        let target = vec(a.pos);
        let previous = g[rt.actors[i]].local_transform().position();
        let position =
            if death_view.is_some() || scene_changed || (**previous - target).norm() > 4.0 {
                target
            } else {
                previous.lerp(&target, 0.5)
            };
        g[rt.actors[i]]
            .local_transform_mut()
            .set_position(position)
            .set_scale(Vector3::repeat(a.body_scale()))
            .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), a.yaw));
        if scene_changed {
            rt.visuals[i].reset_timeline(g);
            rt.last_shots[i] = a.shot;
            rt.tracer_time[i] = 0.0;
        }
        let previous_pose = rt.visuals[i].pose();
        rt.visuals[i].update(
            g,
            rt.actors[i],
            a,
            world_dt,
            w.tick,
            trickcal_shooting_balddagu::movement::grounded(&w.map, a.pos),
        );
        if result_active && !result_started && !scene_changed {
            // Preserve the finishing action instead of selecting idle as motion slows.
            let mut pose = previous_pose;
            if !a.alive() && pose.motion != trickcal_shooting_balddagu::motion::Motion::Death {
                pose.motion = trickcal_shooting_balddagu::motion::Motion::Death;
                pose.age = 0.0;
                pose.locomotion = None;
            }
            pose.age += world_dt;
            pose.shot_age += world_dt;
            if let Some(movement) = &mut pose.locomotion {
                movement.age =
                    (movement.age + world_dt).rem_euclid(movement.motion.seconds(a.character));
            }
            rt.visuals[i].seek(g, pose);
        }
        if replaying || top_down && i == slot {
            rt.visuals[i].seek(g, rt.deathcam.poses()[i]);
        }
        if rt.attack_review && i == slot {
            let pose = attack_review::pose(rt, a.character);
            rt.visuals[i].seek(g, pose);
        }
        if rt.weapon_review && i == slot {
            if let Some(pose) = weapon_review::attack_pose(rt, a.character) {
                rt.visuals[i].seek(g, pose);
            }
        }
        g[rt.team_marks[i]].set_visibility(
            (!rt.art_review || i == slot) && a.alive() && (i != view_slot || rt.orbit.active),
        );
        g[rt.team_marks[i]]
            .local_transform_mut()
            .set_position(vec(a.pos + V3::new(0.0, 0.025, 0.0)));
        shields::update(g, rt.guards[i], a);
        if !a.active {
            rt.last_shots[i] = a.shot;
            rt.tracer_time[i] = 0.0;
        }
        if a.active && (a.shot != rt.last_shots[i] || replaying) {
            rt.last_shots[i] = a.shot;
            rt.tracer_time[i] = if a.character == 0 {
                weapons::VIVI_SWING_SECONDS
            } else if kit(a.character).mode == AttackMode::Melee {
                0.24
            } else {
                0.09
            };
            let origin = a.eye() + V3::new(0.0, -0.15, 0.0);
            let diff = a.shot_end - origin;
            let rot = UnitQuaternion::rotation_between(&Vector3::z(), &vec(diff).normalize())
                .unwrap_or_default();
            g[rt.tracers[i]]
                .local_transform_mut()
                .set_position(vec(origin + diff * 0.5))
                .set_scale(Vector3::new(0.025, 0.025, diff.length()))
                .set_rotation(rot);
        }
        rt.tracer_time[i] = if replaying {
            (rt.tracer_time[i] - rt.deathcam.poses()[i].shot_age).max(0.0)
        } else {
            (rt.tracer_time[i] - world_dt).max(0.0)
        };
        g[rt.tracers[i]]
            .set_visibility(rt.tracer_time[i] > 0.0 && kit(a.character).mode == AttackMode::Ray);
    }
    for (i, node) in rt.projectiles.iter().enumerate() {
        let Some(p) = w.projectiles.get(i) else {
            g[*node].set_visibility(false);
            continue;
        };
        let (color, scale) = projectile_look(p.kind);
        rt.projectile_materials[i]
            .data_ref()
            .set_property("diffuseColor", rgb(color));
        let direction = if p.velocity.length() > 0.01 {
            p.velocity
        } else {
            direction(w.actors[p.owner].yaw, w.actors[p.owner].pitch)
        };
        let rotation = UnitQuaternion::rotation_between(&Vector3::z(), &vec(direction).normalize())
            .unwrap_or_default();
        g[*node].set_visibility(true);
        g[*node]
            .local_transform_mut()
            .set_position(vec(p.pos))
            .set_scale(vec(scale))
            .set_rotation(rotation);
    }
    let weapon_kit = kit(viewed.character);
    for id in 0..15 {
        g[rt.weapons[id]].set_visibility(id == viewed.character as usize);
    }
    g[rt.weapon].set_visibility(
        view_alive && !rt.orbit.active && !show_selection && viewed.wave_ticks == 0 && !zoomed,
    );
    let show_arms = g[rt.weapon].visibility() || (waving && !rt.orbit.active && !show_selection);
    g[rt.wave_hand].set_visibility(show_arms);
    g[rt.flash]
        .set_visibility(rt.tracer_time[view_slot] > 0.0 && weapon_kit.mode == AttackMode::Ray);
    let pose = rt.visuals[view_slot].pose();
    let attack_phase = if pose.motion.is_attack() {
        pose.age / trickcal_shooting_balddagu::attack::seconds(viewed.character)
    } else {
        0.0
    };
    let hand = if pose.motion.is_naia_skill() {
        trickcal_shooting_balddagu::naia_skill::hand(pose.motion, pose.age)
    } else if viewed.character == 4 {
        trickcal_shooting_balddagu::aya_attack::hand(
            false,
            attack_phase,
            pose.motion == trickcal_shooting_balddagu::motion::Motion::Attack2,
        )
    } else {
        trickcal_shooting_balddagu::attack::hand(viewed.character, false, attack_phase)
    };
    let moving = !result_active
        && death_view.is_none()
        && input_moving(&rt.keys)
        && rt.grabbed
        && viewed.combat.naia.phase != trickcal_shooting_balddagu::naia_skill::Phase::Fire
        && !viewed.combat.erpin.eating;
    let bob = if moving {
        (w.tick as f32 * 0.18).sin() * 0.012
    } else {
        0.0
    };
    // The near-camera arms follow the weapon grip after this transform in post_update.
    // Vivi's two blades have independent sampled paths, so do not move their shared root twice.
    let motion = if viewed.character == 0 {
        Default::default()
    } else {
        hand
    };
    // Turn held weapons slightly inward to show their side in first person.
    // Vivi's floating blades keep their independently authored paths.
    let grip_yaw = if viewed.character == 0 {
        0.0
    } else {
        10.0_f32.to_radians()
    };
    let rotation = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), grip_yaw)
        * UnitQuaternion::from_euler_angles(motion.angles[0], motion.angles[1], motion.angles[2]);
    // Rotate around the equipped grip, not the camera origin. Long weapons and
    // deep wind-ups otherwise pull the wrist behind the near plane.
    let equipped = g[rt.weapons[viewed.character as usize]].local_transform();
    let grip = weapons::grips(viewed.character)[0].unwrap();
    let pivot =
        **equipped.position() + **equipped.rotation() * grip.component_mul(&**equipped.scale());
    let flash_offset = match viewed.character {
        // Follow the corrected guns' forward muzzles, including viewmodel scale.
        13 => {
            **equipped.rotation()
                * Vector3::new(0.0, 0.15, 0.565).component_mul(&**equipped.scale())
        }
        14 => {
            **equipped.rotation()
                * Vector3::new(0.0, 0.06, 0.735).component_mul(&**equipped.scale())
        }
        _ => Vector3::new(0.06, 0.07, 0.32),
    };
    let flash_position = **equipped.position() + flash_offset;
    g[rt.flash]
        .local_transform_mut()
        .set_position(flash_position);
    g[rt.weapon]
        .local_transform_mut()
        .set_position(
            Vector3::from(motion.position) + pivot
                - rotation * pivot
                - Vector3::y()
                    * if pose.motion == trickcal_shooting_balddagu::motion::Motion::ErpinEat {
                        0.12
                    } else {
                        0.0
                    }
                + Vector3::new(0.0, bob - if viewed.reload > 0.0 { 0.18 } else { 0.0 }, 0.0),
        )
        .set_rotation(rotation);
    if viewed.character == 0 {
        weapons::update_vivi_viewmodel(
            g,
            rt.weapons[0],
            attack_phase,
            pose.motion == trickcal_shooting_balddagu::motion::Motion::Attack2,
        );
    }
    if viewed.character == 11 {
        weapons::update_slingshot(g, rt.weapons[11], attack_phase);
    }
    if viewed.character == 4 {
        weapons::update_aya(
            g,
            rt.weapons[4],
            attack_phase,
            pose.motion == trickcal_shooting_balddagu::motion::Motion::Attack2,
        );
    }
    for i in 0..2 {
        let color = if i < w.point {
            MINT
        } else if i > w.point {
            [108, 115, 127]
        } else if w.contested {
            [255, 181, 95]
        } else {
            if w.attacker == 0 {
                MINT
            } else {
                PINK
            }
        };
        rt.point_materials[i]
            .data_ref()
            .set_property("diffuseColor", rgb(color));
    }
    rt.portals.update(
        g,
        &poses,
        rt.camera,
        context.user_interfaces.first().screen_size(),
        !show_selection && !rt.home_open && !rt.lobby.open,
        w.tick as f32 * DT,
    );
    let ui = context.user_interfaces.first();
    let size = ui.screen_size();
    let width = size.x;
    let height = size.y;
    let center = width * 0.5;
    let hud = &rt.hud;
    hud.nameplates.render(rt, g, ui);
    place(ui, hud.brand, 24.0, 20.0, 255.0, 65.0);
    let connection_width =
        ((width - 488.0 * (width / 1280.0).min(height / 800.0).clamp(0.65, 1.25)) * 0.5 - 40.0)
            .clamp(180.0, 350.0);
    place(
        ui,
        hud.connection,
        width - connection_width - 24.0,
        24.0,
        connection_width,
        65.0,
    );
    text(
        ui,
        hud.connection,
        rt.error.clone().unwrap_or_else(|| {
            format!(
                "{}\n{} FPS",
                rt.session.label(),
                context
                    .graphics_context
                    .as_initialized_ref()
                    .renderer
                    .get_statistics()
                    .frames_per_second
            )
        }),
    );
    text(ui, hud.status, me.combat.status_text());
    place(
        ui,
        hud.banner,
        center - 260.0,
        if death_view.is_some() {
            155.0
        } else {
            height * 0.5 - 135.0
        },
        600.0,
        75.0,
    );
    let banner = if result_active {
        String::new()
    } else if top_down {
        if rt.korean_font {
            format!(
                "{} · 사망 모션\n부활까지 {:.0}초",
                character(w.actors[slot].character).korean,
                me.respawn.ceil().max(0.0)
            )
        } else {
            format!(
                "DEATH ANIMATION\nRespawning in {:.0}s",
                me.respawn.ceil().max(0.0)
            )
        }
    } else if let Some(view) = death_view {
        let c = character(viewed.character);
        let remaining = (rt.deathcam.replay_duration - rt.deathcam.replay_time).max(0.0);
        if rt.korean_font {
            format!(
                "데스캠 · PLAYER {} / {} · {}\n처치 {:.1}초 전 · 부활까지 {:.0}초",
                view.target + 1,
                c.korean,
                if view.frozen {
                    "재생 완료"
                } else {
                    "녹화 재생"
                },
                remaining,
                me.respawn.ceil().max(0.0)
            )
        } else {
            format!(
                "DEATH REPLAY · PLAYER {} / {}{}\n{:.1}s before elimination · Respawn {:.0}s",
                view.target + 1,
                c.name,
                if view.frozen { " · COMPLETE" } else { "" },
                remaining,
                me.respawn.ceil().max(0.0)
            )
        }
    } else if !me.alive() {
        format!("BALDDAGU DOWN\nRespawning in {:.0}s", me.respawn.ceil())
    } else if w.phase == Phase::Setup {
        format!(
            "{} / Starts in {:.0}",
            if team(slot) == w.attacker {
                "GET READY TO ATTACK"
            } else {
                "GET READY TO DEFEND"
            },
            w.phase_time.ceil()
        )
    } else if w.phase == Phase::Intermission {
        format!("SWITCHING SIDES / {:.0}", w.phase_time.ceil())
    } else if me.hurt > 0.0 {
        "TAKING FIRE".into()
    } else {
        String::new()
    };
    text(ui, hud.banner, banner);
    let show_menu = !rt.art_review
        && !result_active
        && !rt.settings_menu.open
        && !rt.hud_review
        && !rt.objective_hud_review
        && !rt.weapon_review
        && !rt.greeting_review
        && !rt.dance_review
        && !rt.attack_review
        && !rt.naia_skill_review
        && !rt.communication_review
        && !show_selection
        && !rt.chat.open
        && !rt.wheel.open
        && ((w.phase == Phase::Lobby && !rt.orbit.active)
            || w.phase == Phase::Finished
            || (!rt.grabbed && rt.smoke.is_none())
            || rt.error.is_some());
    visible(ui, hud.panel, show_menu || rt.keys.score);
    visible(ui, hud.menu, show_menu && !rt.keys.score);
    visible(ui, hud.scoreboard, rt.keys.score);
    place(
        ui,
        hud.panel,
        center - 340.0,
        height * 0.5 - 220.0,
        680.0,
        440.0,
    );
    place(
        ui,
        hud.menu,
        center - 300.0,
        height * 0.5 - 150.0,
        620.0,
        320.0,
    );
    if show_menu {
        let title = if w.phase == Phase::Finished {
            w.event.clone()
        } else {
            "TRICKCAL : SHOOTING BALDDAGU".into()
        };
        let prompt = if w.phase == Phase::Finished {
            if rt.korean_font {
                format!(
                    "{}초 후 게임 로비로 돌아갑니다.\nTab  경기 기록 / Enter  채팅",
                    w.phase_time.ceil() as u32
                )
            } else {
                format!(
                    "Returning to the game lobby in {}s.\nTab  scoreboard / Enter  chat",
                    w.phase_time.ceil() as u32
                )
            }
        } else if matches!(rt.session, Session::Join(_)) {
            "F5 / CLICK  to control your Balddagu\nHost starts the match. Enter to chat. F10 to quit.".into()
        } else if w.phase == Phase::Lobby {
            "F5  to start a new match\nEnter  to chat / F10  to quit".into()
        } else {
            "F5 / CLICK  to resume controls\nEnter to chat. The match continues while the cursor is free.".into()
        };
        text(ui,hud.menu,format!("{title}\n\nFAIRY KINGDOM CASTLE / COMPETITIVE 2CP / 6 vs 6\nCapture A, then B. Swap sides. Win the time bank.\n\n{prompt}\nH  Character selection / E  Skill / Q  Ultimate / G  Dance\n\n{}",rt.error.clone().unwrap_or_else(||rt.session.label())));
    }
    place(
        ui,
        hud.scoreboard,
        center - 290.0,
        height * 0.5 - 195.0,
        620.0,
        405.0,
    );
    if rt.keys.score {
        let mut board = "6 vs 6 / CHARACTER / POSITION / ELIMS : DEATHS\n".to_string();
        for (i, a) in w.actors.iter().enumerate() {
            if i % TEAM_SIZE == 0 {
                let counts = w.position_counts(team(i));
                board.push_str(&format!(
                    "\n{}  +{}% DMG  F{}/M{}/R{}\n",
                    team_name(team(i)),
                    w.damage_bonus(team(i)),
                    counts[0],
                    counts[1],
                    counts[2]
                ));
            }
            let c = character(a.character);
            if !a.active {
                board.push_str(&format!("  {} EMPTY\n", i % TEAM_SIZE + 1));
                continue;
            }
            board.push_str(&format!(
                "{} {} {}  {} / {}    {} : {}\n",
                if i == slot { ">" } else { " " },
                i % TEAM_SIZE + 1,
                if a.human { "PLAYER" } else { "BOT   " },
                if rt.korean_font { c.korean } else { c.name },
                c.position.name(),
                a.kills,
                a.deaths
            ));
        }
        text(ui, hud.scoreboard, board);
    }
    hud.cross.render(
        ui,
        me.active
            && me.alive()
            && !rt.home_open
            && !rt.lobby.open
            && !rt.startup.active()
            && !show_selection
            && !result_active
            && !rt.art_review
            && !rt.orbit.active
            && !rt.wheel.open
            && !rt.settings_menu.open
            && !rt.keys.score
            && !show_menu
            && !zoomed,
        me.hit > 0.0,
    );
    for node in [hud.brand, hud.connection, hud.banner, hud.status] {
        let show =
            !show_selection && (!result_active || node == hud.brand || node == hud.connection);
        // Team voice owns the top-left corner during gameplay.
        visible(
            ui,
            node,
            show && (node != hud.brand || result_active || rt.art_review),
        );
    }
    hud.player_info
        .render(rt, ui, !show_selection && !rt.art_review && !result_active);
    hud.selection.render(rt, ui, show_selection);
    hud.chat.render(rt, ui, rt.chat_history());
    hud.communication.render(rt, ui);
    hud.home.render(rt, ui);
    hud.voice.render(rt, ui, show_selection);
    hud.lobby.render(rt, ui);
    hud.rooms.render(rt, ui);
    hud.account.render(rt, ui);
    hud.round_results.render(rt, ui);
    hud.settings.render(rt, ui);
    hud.startup.render(rt, ui);
    hud.scope.render(
        rt,
        context.user_interfaces.first_mut(),
        zoomed && !show_selection && !rt.art_review && !result_active,
    );
    hud.combat.render(
        rt,
        context.user_interfaces.first_mut(),
        !show_selection && !rt.art_review && !result_active,
    );
    hud.ultimate.render(
        rt,
        context.user_interfaces.first_mut(),
        !show_selection && !rt.art_review && !result_active,
    );
    hud.objective.render(
        rt,
        context.user_interfaces.first_mut(),
        !show_selection && !rt.art_review && !result_active,
    );
    Ok(())
}

fn projectile_look(kind: ProjectileKind) -> ([u8; 3], V3) {
    use ProjectileKind::*;
    match kind {
        Water => ([109, 216, 255], V3::new(0.12, 0.12, 0.22)),
        Stone | Ricochet => ([165, 166, 155], V3::new(0.15, 0.12, 0.13)),
        Needle | Poison => ([197, 238, 113], V3::new(0.035, 0.035, 0.32)),
        Syringe => ([115, 239, 172], V3::new(0.05, 0.05, 0.22)),
        Butterfly => ([176, 225, 255], V3::new(0.46, 0.10, 0.22)),
        Spear => ([231, 237, 255], V3::new(0.10, 0.10, 1.0)),
        Wave => ([102, 209, 243], V3::new(1.7, 0.9, 0.30)),
        Doll => ([241, 180, 212], V3::new(0.28, 0.36, 0.25)),
        Boulder => ([182, 173, 151], V3::new(0.65, 0.65, 0.65)),
        DarkOrb => ([146, 99, 218], V3::new(0.30, 0.30, 0.30)),
        Flame => ([202, 143, 253], V3::new(0.50, 0.75, 0.35)),
        Echo => ([128, 238, 219], V3::new(0.15, 0.15, 0.38)),
        MagicBomb => ([253, 180, 222], V3::new(0.30, 0.30, 0.30)),
        Cake4 | Cake3 | Cake2 | Cake1 => ([255, 240, 211], V3::new(0.1, 0.1, 0.1)),
        Bolt => ([255, 224, 145], V3::new(0.12, 0.12, 0.18)),
    }
}
fn input_moving(keys: &Keys) -> bool {
    keys.w || keys.a || keys.s || keys.d
}

/// Read only this application's GPU output, including its actual HUD render.
/// This makes smoke-test visual QA independent of desktop focus or other apps.
fn capture_preview(
    scene: Handle<Scene>,
    stage: u8,
    tag: &str,
    context: PluginContext,
) -> Result<(), Box<dyn std::error::Error>> {
    capture_preview_impl(scene, stage, tag, false, context)
}

fn capture_preview_impl(
    scene: Handle<Scene>,
    stage: u8,
    tag: &str,
    opaque_ui: bool,
    mut context: PluginContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let (width, height, pixels) = capture_frame(scene, opaque_ui, &mut context)?;
    // OpenGL readback starts at the bottom, PNG starts at the top.
    let row = width as usize * 4;
    let flipped: Vec<u8> = pixels.chunks_exact(row).rev().flatten().copied().collect();
    std::fs::create_dir_all("artifacts")?;
    let path = format!(
        "artifacts/preview-{tag}{}.png",
        match stage {
            0 => "menu",
            1 => "game",
            2 => "teams",
            3 => "dance-front",
            _ => "dance-back",
        }
    );
    Texture::from_bytes(
        TextureKind::Rectangle { width, height },
        TexturePixelKind::RGBA8,
        flipped,
    )
    .ok_or("Invalid screenshot size")?
    .save(std::path::Path::new(&path))?;
    println!("PREVIEW_OK: {path}");
    Ok(())
}

fn capture_frame(
    scene: Handle<Scene>,
    opaque_ui: bool,
    context: &mut PluginContext,
) -> Result<(u32, u32, Vec<u8>), Box<dyn std::error::Error>> {
    let renderer = &mut context.graphics_context.as_initialized_mut().renderer;
    let data = renderer
        .scene_data_map
        .get(&scene)
        .ok_or("Scene has not rendered yet")?;
    let width = data.scene_data.gbuffer.width as u32;
    let height = data.scene_data.gbuffer.height as u32;
    let mut pixels = data
        .scene_data
        .ldr_scene_framebuffer
        .read_pixels(ReadTarget::Color(0))
        .ok_or("Scene readback failed")?;
    let target = TextureResource::new_render_target(width, height);
    renderer.render_ui(UiRenderInfo {
        ui: context.user_interfaces.first(),
        render_target: Some(target.clone()),
        clear_color: Color::TRANSPARENT,
        resource_manager: context.resource_manager,
    })?;
    let overlay = renderer
        .ui_frame_buffers
        .get(&target.key())
        .ok_or("HUD framebuffer missing")?
        .read_pixels(ReadTarget::Color(0))
        .ok_or("HUD readback failed")?;
    if pixels.len() != overlay.len() || pixels.len() != width as usize * height as usize * 4 {
        return Err("Unexpected framebuffer format".into());
    }
    for (background, foreground) in pixels.chunks_exact_mut(4).zip(overlay.chunks_exact(4)) {
        // The home already paints the entire UI target with an opaque image.
        // Layer blending can lower target alpha; do not composite the 3D scene twice.
        let alpha = if opaque_ui {
            1.0
        } else {
            foreground[3] as f32 / 255.0
        };
        for channel in 0..3 {
            background[channel] = (foreground[channel] as f32
                + background[channel] as f32 * (1.0 - alpha))
                .min(255.0) as u8;
        }
        background[3] = 255;
    }
    Ok((width, height, pixels))
}
