//! Opt-in, inclusive CPU timings and renderer counters. Run with --perf-profile.
//! TSB_PERF_SECONDS optionally exits after this many seconds of rendered gameplay.
//! TSB_PERF_VARIANT=no-shadows, no-map, or no-ui isolates rendering costs.
//! TSB_PERF_NO_VSYNC=1 disables swap synchronization only with --perf-profile.
use crate::presentation::SceneState;
use super::*;
use std::{cell::RefCell, collections::BTreeMap, sync::OnceLock, time::Duration};

pub(super) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::args().any(|a| a == "--perf-profile"))
}

#[derive(Default)]
struct Sample {
    count: u64,
    total: f64,
    max: f64,
}
impl Sample {
    fn add(&mut self, value: f64) {
        self.count += 1;
        self.total += value;
        self.max = self.max.max(value);
    }
}
struct Profile {
    start: Instant,
    window: Instant,
    previous_frame: Option<Instant>,
    update_end: Option<Instant>,
    samples: BTreeMap<&'static str, Sample>,
    frames: u64,
    frame_times: Vec<f64>,
    inventoried: bool,
    seconds: Option<f64>,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            window: Instant::now(),
            previous_frame: None,
            update_end: None,
            samples: BTreeMap::new(),
            frames: 0,
            frame_times: Vec::new(),
            inventoried: false,
            seconds: std::env::var("TSB_PERF_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok()),
        }
    }
}
thread_local! { static PROFILE: RefCell<Profile> = RefCell::new(Profile::default()); }

pub(super) struct Span {
    name: &'static str,
    start: Option<Instant>,
}
pub(super) fn span(name: &'static str) -> Span {
    Span {
        name,
        start: enabled().then(Instant::now),
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            PROFILE.with(|p| {
                let mut p = p.borrow_mut();
                p.samples.entry(self.name).or_default().add(elapsed);
                if self.name == "plugin.update" {
                    p.update_end = Some(Instant::now());
                }
            });
        }
    }
}

pub(super) fn post_update(context: &PluginContext, scene: Handle<Scene>) {
    if !enabled() {
        return;
    }
    PROFILE.with(|p| {
        let mut p = p.borrow_mut();
        if let Some(end) = p.update_end.take() {
            p.samples
                .entry("engine.between_update_and_post_ms")
                .or_default()
                .add(end.elapsed().as_secs_f64() * 1000.0);
        }
        let g = &context.scenes[scene].performance_statistics.graph;
        for (name, value) in [
            ("engine.ui_ms", context.performance_statistics.ui_time),
            ("engine.hierarchy_ms", g.hierarchical_properties_time),
            ("engine.sync_ms", g.sync_time),
            ("engine.sound_ms", g.sound_update_time),
            ("engine.physics_ms", g.physics.total() + g.physics2d.total()),
        ] {
            p.samples
                .entry(name)
                .or_default()
                .add(value.as_secs_f64() * 1000.0);
        }
    });
}

pub(super) fn frame(rt: &Runtime, context: &mut PluginContext) {
    if !enabled() || rt.startup.active() {
        return;
    }
    PROFILE.with(|p| {
        let mut p = p.borrow_mut();
        let now = Instant::now();
        if !p.inventoried {
            let g = &context.scenes[rt.scene].graph;
            let mut groups: BTreeMap<String, usize> = BTreeMap::new();
            let mut meshes = 0;
            let mut surfaces = 0;
            let mut triangles = 0;
            let mut visible = 0;
            for node in g.linear_iter() {
                if let Some(mesh) = node.cast::<fyrox::scene::mesh::Mesh>() {
                    *groups.entry(node.name().to_string()).or_default() += 1;
                    meshes += 1;
                    visible += usize::from(node.global_visibility());
                    surfaces += mesh.surfaces().len();
                    for s in mesh.surfaces() { triangles += s.data_ref().data_ref().geometry_buffer.len(); }
                }
            }
            println!("PROFILE_SCENE nodes={} meshes={} visible_meshes={} surfaces={} instance_triangles={} actors={} active={}",
                g.node_count(), meshes, visible, surfaces, triangles, rt.visuals.len(),
                rt.session.world().actors.iter().filter(|a| a.active).count());
            for (i, v) in rt.visuals.iter().enumerate() {
                println!("PROFILE_ACTOR slot={i} character={} active={} nodes={}", v.id,
                    rt.session.world().actors[i].active, g.traverse_handle_iter(rt.actors[i]).count());
            }
            let scenery = g.linear_iter().find(|n| n.name().ends_with(" / Blender scenery"))
                .map(|n| n.handle());
            if let Some(root) = scenery {
                let mut map_meshes = 0;
                let mut map_surfaces = 0;
                let mut map_triangles = 0;
                for n in g.traverse_handle_iter(root) {
                    if let Some(mesh) = g[n].cast::<fyrox::scene::mesh::Mesh>() {
                        map_meshes += 1;
                        map_surfaces += mesh.surfaces().len();
                        for s in mesh.surfaces() { map_triangles += s.data_ref().data_ref().geometry_buffer.len(); }
                    }
                }
                println!("PROFILE_MAP meshes={map_meshes} surfaces={map_surfaces} triangles={map_triangles}");
            }
            let mut groups: Vec<_> = groups.into_iter().collect();
            groups.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            for (name, count) in groups.into_iter().take(15) {
                println!("PROFILE_MESH_GROUP count={count} name={name}");
            }
            let variant = std::env::var("TSB_PERF_VARIANT").unwrap_or_default();
            println!("PROFILE_VARIANT {variant:?}");
            if variant == "no-shadows" {
                let renderer = &mut context.graphics_context.as_initialized_mut().renderer;
                let mut quality = renderer.get_quality_settings();
                quality.csm_settings.enabled = false;
                if let Err(error) = renderer.set_quality_settings(&quality) {
                    eprintln!("PROFILE_VARIANT_FAILED {error}");
                }
            } else if variant == "no-map" {
                if let Some(root) = scenery { context.scenes[rt.scene].graph[root].set_visible(false); }
            } else if variant == "no-ui" {
                let ui = context.user_interfaces.first_mut();
                ui.send(ui.root(), WidgetMessage::Visibility(false));
            }
            p.start = now;
            p.window = now;
            p.samples.clear();
            p.inventoried = true;
        }
        if let Some(last) = p.previous_frame.replace(now) {
            let interval = now.duration_since(last).as_secs_f64() * 1000.0;
            p.samples.entry("frame.interval_ms").or_default().add(interval);
            p.frame_times.push(interval);
        }
        p.frames += 1;
        let stats = context.graphics_context.as_initialized_ref().renderer.get_statistics();
        for (name, value) in [
            ("renderer.pure_ms", stats.pure_frame_time as f64 * 1000.0),
            ("renderer.capped_ms", stats.capped_frame_time as f64 * 1000.0),
            ("renderer.draw_calls", stats.geometry.draw_calls as f64),
            ("renderer.triangles", stats.geometry.triangles_rendered as f64),
            ("renderer.texture_binds", stats.pipeline.texture_binding_changes as f64),
            ("renderer.shader_binds", stats.pipeline.program_binding_changes as f64),
            ("renderer.shadow_maps", stats.lighting.csm_rendered as f64),
        ] { p.samples.entry(name).or_default().add(value); }
        let exit = p.seconds.is_some_and(|limit| p.start.elapsed().as_secs_f64() >= limit);
        if p.window.elapsed() >= Duration::from_secs(2) || exit {
            let seconds = p.window.elapsed().as_secs_f64();
            println!("PROFILE_WINDOW elapsed={:.2} seconds={seconds:.3} frames={} fps={:.2} phase={:?}",
                p.start.elapsed().as_secs_f64(), p.frames, p.frames as f64 / seconds, rt.session.world().phase);
            for (name, s) in &p.samples {
                println!("PROFILE {name} calls={} per_frame={:.2} mean={:.4} max={:.4} total={:.3}",
                    s.count, s.count as f64 / p.frames as f64, s.total / s.count as f64, s.max, s.total);
            }
            if !p.frame_times.is_empty() {
                p.frame_times.sort_by(f64::total_cmp);
                let percentile = |q: f64| p.frame_times[((p.frame_times.len() - 1) as f64 * q).ceil() as usize];
                println!("PROFILE_FRAME_TIME p50={:.3} p95={:.3} p99={:.3} worst={:.3}",
                    percentile(0.50), percentile(0.95), percentile(0.99), percentile(1.0));
                p.frame_times.clear();
            }
            p.samples.clear(); p.frames = 0; p.window = Instant::now();
        }
        if exit { println!("PROFILE_DONE"); context.loop_controller.exit(); }
    });
}
