//! Live, GPU-only peripheral blur. Runs before the scope reticle and combat HUD.
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
};
use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

#[derive(Clone, Copy, Debug, Default)]
struct View {
    scene: Handle<Scene>,
    camera: Handle<Node>,
    active: bool,
}

#[derive(Debug, Default)]
pub(super) struct ScopeEffect {
    view: Rc<Cell<View>>,
    registration: RefCell<Weak<()>>,
}

impl ScopeEffect {
    pub(super) fn set_view(&self, scene: Handle<Scene>, camera: Handle<Node>, active: bool) {
        self.view.set(View {
            scene,
            camera,
            active,
        });
    }

    pub(super) fn active(&self) -> bool {
        self.view.get().active
    }

    pub(super) fn install(&self, renderer: &mut Renderer) {
        // A recreated graphics context drops its old pass and registration token.
        if self.registration.borrow().upgrade().is_some() {
            return;
        }
        let registration = Rc::new(());
        *self.registration.borrow_mut() = Rc::downgrade(&registration);
        renderer.add_render_pass(Rc::new(RefCell::new(ScopeBlur {
            view: self.view.clone(),
            _registration: registration,
            targets: None,
            composite: None,
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
                    PixelKind::RGBA8,
                    size.x,
                    size.y,
                )?)],
            )
        };
        Ok(Self {
            size,
            horizontal: target("ShionScopeHorizontal")?,
            vertical: target("ShionScopeVertical")?,
        })
    }
}

struct ScopeBlur {
    view: Rc<Cell<View>>,
    _registration: Rc<()>,
    targets: Option<Targets>,
    composite: Option<RenderPassContainer>,
}

impl SceneRenderPass for ScopeBlur {
    fn source_type_id(&self) -> TypeId {
        TypeId::of::<Game>()
    }

    fn on_ldr_render(
        &mut self,
        ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let view = self.view.get();
        let viewport = ctx.observer.viewport;
        if !view.active
            || ctx.scene_handle != view.scene
            || ctx.observer.handle != view.camera
            || viewport.w() <= 0
            || viewport.h() <= 0
        {
            return Ok(Default::default());
        }

        let size = Vector2::new(viewport.w() as f32, viewport.h() as f32);
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
        if self.composite.is_none() {
            self.composite = Some(RenderPassContainer::from_str(
                ctx.server,
                include_str!("scope_blur.shader"),
            )?);
        }
        let targets = self.targets.as_ref().unwrap();
        let resources = ctx.renderer_resources;
        let pass = ImmutableString::new("Primary");
        let small_viewport = Rect::new(0, 0, small.x as i32, small.y as i32);
        let small_projection = make_viewport_matrix(small_viewport);
        let pixel_size = Vector2::new(1.0 / small.x as f32, 1.0 / small.y as f32);
        let mut stats = RenderPassStatistics::default();

        // Separate quarter-resolution targets avoid sampling the attachment being written.
        // Linear sampling also downsamples the live scene during the horizontal pass.
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
                property("worldViewProjection", &small_projection),
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

        let (radius, scale) = scope::lens_geometry(size);
        let projection = make_viewport_matrix(viewport);
        let feather = 2.0 * scale;
        let properties = PropertyGroup::from([
            property("worldViewProjection", &projection),
            property("screenSize", &size),
            property("lensRadius", &radius),
            property("feather", &feather),
        ]);
        let material = RenderMaterial::from([
            binding(
                "image",
                (
                    &targets.vertical.color_attachments()[0].texture,
                    &resources.linear_clamp_sampler,
                ),
            ),
            binding("properties", &properties),
        ]);
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
        Ok(stats)
    }
}
