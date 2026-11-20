//! Blur a live overview of the active objective behind the sharp hero preview.
use super::*;
use fyrox::{
    core::{math::Rect, sstorage::ImmutableString},
    graphics::{
        error::FrameworkError,
        framebuffer::{Attachment, GpuFrameBuffer},
        gpu_texture::PixelKind,
        server::GraphicsServer,
    },
    renderer::{
        cache::shader::{binding, property, PropertyGroup, RenderMaterial, RenderPassContainer},
        make_viewport_matrix, RenderPassStatistics, Renderer, SceneRenderPass,
        SceneRenderPassContext,
    },
    scene::collider::BitMask,
};
use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

const PREVIEW_LAYER: u32 = 1 << 31;

pub(super) fn create_camera(g: &mut Graph) -> Handle<Node> {
    let camera: Handle<Node> =
        CameraBuilder::new(BaseBuilder::new().with_name("Selection active objective overview"))
            .with_fov(62.0_f32.to_radians())
            .with_z_near(0.02)
            .with_z_far(180.0)
            .with_exposure(Exposure::Manual(1.0))
            .enabled(false)
            .build(g)
            .to_base();
    g[camera]
        .render_mask
        .set_value_and_mark_modified(BitMask(!PREVIEW_LAYER));
    camera
}

pub(super) fn update_camera(g: &mut Graph, camera: Handle<Node>, world: &World, show: bool) {
    g[camera]
        .cast_mut::<fyrox::scene::camera::Camera>()
        .unwrap()
        .set_enabled(show);
    if !show {
        return;
    }

    // Authored per objective to stay inside the halls and clear roofs and pillars.
    // Read the authoritative point every frame, independent of player/team/deathcam.
    let offset = match (world.map_id, world.point) {
        (MapId::FairyKingdomCastle, 0) => V3::new(-8.0, 12.0, -18.0),
        (MapId::FairyKingdomCastle, _) => V3::new(-15.0, 7.0, -1.0),
        (MapId::ElfCity, 0) => V3::new(-9.0, 8.0, -11.0),
        (MapId::ElfCity, _) => V3::new(-13.0, 9.0, -10.0),
    };
    let objective = vec(world.points()[world.point]);
    let position = objective + vec(offset);
    let mut target = objective + Vector3::new(0.0, 1.5, 0.0);
    let forward = target - position;
    let right = Vector3::new(-forward.z, 0.0, forward.x).normalize();
    // Leave the capture zone beside the hero, rather than hidden behind its bust.
    target += right * 5.0;
    let direction = target - position;
    let yaw = direction.x.atan2(direction.z);
    let pitch = direction.y.atan2(direction.x.hypot(direction.z));
    g[camera]
        .local_transform_mut()
        .set_position(position)
        .set_rotation(UnitQuaternion::from_euler_angles(-pitch, yaw, 0.0));
}

// --selection-review --selection-objective-review exercises A -> B and a live map switch.
pub(super) fn review_objectives(rt: &mut Runtime) {
    if rt.review_frames == 0 {
        let map = MapId::ALL[(rt.review_capture / 2) as usize];
        if rt.review_capture.is_multiple_of(2) {
            let mut world = World::default();
            world.select_map(map);
            world.start();
            world.phase = Phase::Playing;
            world.actors[0].human = true;
            rt.session = Session::Solo(Box::new(world));
        }
        if let Session::Solo(world) = &mut rt.session {
            world.point = (rt.review_capture % 2) as usize;
        }
    }
    rt.selected = 0;
}

pub(super) fn isolate_preview(g: &mut Graph, root: Handle<Node>) {
    let nodes: Vec<_> = g.traverse_handle_iter(root).collect();
    for node in nodes {
        g[node]
            .render_mask
            .set_value_and_mark_modified(BitMask(PREVIEW_LAYER));
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct View {
    scene: Handle<Scene>,
    map: Handle<Node>,
    preview: Handle<Node>,
    active: bool,
}

#[derive(Debug, Default)]
pub(super) struct Backdrop {
    view: Rc<Cell<View>>,
    registration: Weak<()>,
}

impl Backdrop {
    pub(super) fn install(
        &mut self,
        renderer: &mut Renderer,
        scene: Handle<Scene>,
        map: Handle<Node>,
        preview: Handle<Node>,
        active: bool,
    ) {
        self.view.set(View {
            scene,
            map,
            preview,
            active,
        });
        if self.registration.upgrade().is_some() {
            return;
        }
        let registration = Rc::new(());
        self.registration = Rc::downgrade(&registration);
        renderer.add_render_pass(Rc::new(RefCell::new(BackdropPass {
            view: self.view.clone(),
            _registration: registration,
            targets: None,
            composite: None,
            ready: false,
        })));
    }
}

struct Targets {
    size: Vector2<usize>,
    horizontal: GpuFrameBuffer,
    vertical: GpuFrameBuffer,
}

impl Targets {
    fn new(server: &dyn GraphicsServer, size: Vector2<usize>) -> Result<Self, FrameworkError> {
        let target = |name| {
            server.create_frame_buffer(
                None,
                vec![Attachment::color(server.create_2d_render_target(
                    name,
                    PixelKind::RGBA16F,
                    size.x,
                    size.y,
                )?)],
            )
        };
        Ok(Self {
            size,
            horizontal: target("SelectionMapHorizontal")?,
            vertical: target("SelectionMapVertical")?,
        })
    }
}

struct BackdropPass {
    view: Rc<Cell<View>>,
    _registration: Rc<()>,
    targets: Option<Targets>,
    composite: Option<RenderPassContainer>,
    ready: bool,
}

impl SceneRenderPass for BackdropPass {
    fn source_type_id(&self) -> TypeId {
        TypeId::of::<Game>()
    }

    fn on_hdr_render(
        &mut self,
        ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let view = self.view.get();
        let viewport = ctx.observer.viewport;
        if !view.active || ctx.scene_handle != view.scene {
            self.ready = false;
            return Ok(Default::default());
        }
        if viewport.w() <= 0 || viewport.h() <= 0 {
            return Ok(Default::default());
        }
        let resources = ctx.renderer_resources;
        let pass = ImmutableString::new("Primary");
        let mut stats = RenderPassStatistics::default();
        if ctx.observer.handle == view.map {
            self.ready = false;
            let small = Vector2::new(
                (viewport.w() as usize).div_ceil(4),
                (viewport.h() as usize).div_ceil(4),
            );
            if self
                .targets
                .as_ref()
                .is_none_or(|targets| targets.size != small)
            {
                self.targets = Some(Targets::new(ctx.server, small)?);
            }
            let targets = self.targets.as_ref().unwrap();
            let small_viewport = Rect::new(0, 0, small.x as i32, small.y as i32);
            let projection = make_viewport_matrix(small_viewport);
            let pixel_size = Vector2::new(1.0 / small.x as f32, 1.0 / small.y as f32);
            // Ping-pong targets: no CPU readback, and never sample the active attachment.
            for (input, output, horizontal) in [
                (
                    &ctx.framebuffer.color_attachments()[0].texture,
                    &targets.horizontal,
                    true,
                ),
                (
                    &targets.horizontal.color_attachments()[0].texture,
                    &targets.vertical,
                    false,
                ),
            ] {
                let properties = PropertyGroup::from([
                    property("worldViewProjection", &projection),
                    property("pixelSize", &pixel_size),
                    property("horizontal", &horizontal),
                ]);
                let material = RenderMaterial::from([
                    binding("image", (input, &resources.linear_clamp_sampler)),
                    binding("properties", &properties),
                ]);
                stats += resources.shaders.gaussian_blur.run_pass(
                    1,
                    &pass,
                    output,
                    &resources.quad,
                    small_viewport,
                    &material,
                    ctx.uniform_buffer_cache,
                    Default::default(),
                    None,
                )?;
            }
            self.ready = true;
        } else if ctx.observer.handle == view.preview && self.ready {
            // Consume only the map drawn earlier in this frame, including after a map switch.
            self.ready = false;
            if self.composite.is_none() {
                self.composite = Some(RenderPassContainer::from_str(
                    ctx.server,
                    include_str!("selection_backdrop.shader"),
                )?);
            }
            let targets = self.targets.as_ref().unwrap();
            let projection = make_viewport_matrix(viewport);
            let properties = PropertyGroup::from([property("worldViewProjection", &projection)]);
            let material = RenderMaterial::from([
                binding(
                    "image",
                    (
                        &targets.vertical.color_attachments()[0].texture,
                        &resources.linear_clamp_sampler,
                    ),
                ),
                binding(
                    "depthTexture",
                    (ctx.depth_texture, &resources.nearest_clamp_sampler),
                ),
                binding("properties", &properties),
            ]);
            // Compose before tone mapping and FXAA so the hero's edges stay clean.
            stats += self.composite.as_ref().unwrap().run_pass(
                1,
                &pass,
                ctx.framebuffer,
                &resources.quad,
                viewport,
                &material,
                ctx.uniform_buffer_cache,
                Default::default(),
                None,
            )?;
        }
        Ok(stats)
    }
}
