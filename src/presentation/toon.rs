//! Matte, banded deferred lighting and a single depth/normal ink pass.
//! GBuffer materials, alpha cutouts, animation, shadow passes and effects stay intact.
use super::*;
use fyrox::{
    core::sstorage::ImmutableString,
    graphics::error::FrameworkError,
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
}

#[derive(Debug, Default)]
pub(super) struct ToonRenderer {
    view: Rc<Cell<View>>,
    registration: Weak<()>,
}

impl ToonRenderer {
    pub(super) fn install(
        &mut self,
        renderer: &mut Renderer,
        scene: Handle<Scene>,
        camera: Handle<Node>,
    ) -> Result<(), FrameworkError> {
        self.view.set(View { scene, camera });
        // A diagnostic comparison switch; normal play always uses the toon renderer.
        if self.registration.upgrade().is_some()
            || std::env::args().any(|arg| arg == "--render-standard")
        {
            return Ok(());
        }
        let server = renderer.graphics_server();
        let lighting = |source: &str| {
            RenderPassContainer::from_str(
                server,
                &source.replace("// TSB_TOON_LIGHTING", include_str!("toon/lighting.glsl")),
            )
        };
        // Compile everything before changing the renderer. Keep Fyrox 1.0.1's exact
        // resource layouts, shadow cascades, point attenuation and spot cookies.
        let directional = lighting(include_str!("toon/deferred_directional_light.shader"))?;
        let point = lighting(include_str!("toon/deferred_point_light.shader"))?;
        let spot = lighting(include_str!("toon/deferred_spot_light.shader"))?;
        let ambient = lighting(include_str!("toon/ambient_light.shader"))?;
        let outline = RenderPassContainer::from_str(server, include_str!("toon/outline.shader"))?;
        let mut quality = renderer.get_quality_settings();
        quality.use_ssao = false;
        quality.fxaa = true;
        quality.hdr_settings.bloom_settings.use_bloom = false;
        renderer.set_quality_settings(&quality)?;
        let shaders = &mut renderer.renderer_resources.shaders;
        shaders.directional_light = directional;
        shaders.point_light = point;
        shaders.spot_light = spot;
        shaders.ambient_light = ambient;
        // A weak token automatically expires when a graphics context is recreated.
        let registration = Rc::new(());
        self.registration = Rc::downgrade(&registration);
        renderer.add_render_pass(Rc::new(RefCell::new(InkPass {
            view: self.view.clone(),
            _registration: registration,
            shader: outline,
        })));
        println!("TOON_RENDERER_OK: ambient + 3 banded lights + depth/normal ink, Fyrox 1.0.1");
        Ok(())
    }
}

struct InkPass {
    view: Rc<Cell<View>>,
    _registration: Rc<()>,
    shader: RenderPassContainer,
}

impl SceneRenderPass for InkPass {
    fn source_type_id(&self) -> TypeId {
        TypeId::of::<Game>()
    }

    fn on_hdr_render(
        &mut self,
        ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let view = self.view.get();
        let viewport = ctx.observer.viewport;
        if view.scene != ctx.scene_handle
            || view.camera != ctx.observer.handle
            || viewport.w() <= 0
            || viewport.h() <= 0
        {
            return Ok(Default::default());
        }
        let resources = ctx.renderer_resources;
        let projection = make_viewport_matrix(viewport);
        let Some(inverse_projection) = ctx.observer.position.projection_matrix.try_inverse() else {
            return Ok(Default::default());
        };
        let screen_size = Vector2::new(viewport.w() as f32, viewport.h() as f32);
        let width = (screen_size.y / 800.0 * 1.35).clamp(1.0, 2.6);
        let properties = PropertyGroup::from([
            property("worldViewProjection", &projection),
            property("inverseProjection", &inverse_projection),
            property("screenSize", &screen_size),
            property("lineWidth", &width),
        ]);
        // Blend ink directly; never sample the framebuffer being written, and never
        // process UI/text. Running before tone mapping lets FXAA smooth the ink,
        // and the later scope pass blurs these same world outlines.
        let material = RenderMaterial::from([
            binding(
                "depthTexture",
                (ctx.depth_texture, &resources.nearest_clamp_sampler),
            ),
            binding(
                "normalTexture",
                (ctx.normal_texture, &resources.nearest_clamp_sampler),
            ),
            binding("properties", &properties),
        ]);
        let mut stats = RenderPassStatistics::default();
        stats += self.shader.run_pass(
            1,
            &ImmutableString::new("Primary"),
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
