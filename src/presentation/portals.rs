//! Live, perspective-correct portal windows using offscreen cameras in the same scene.
use super::*;
use fyrox::scene::collider::BitMask;
use fyrox::{
    core::{algebra::Vector4, math::TriangleDefinition},
    graphics::gpu_program::{ShaderProperty, ShaderPropertyKind},
    material::{
        shader::{Shader, ShaderResource, ShaderResourceKind},
        MaterialResourceBinding,
    },
    scene::{
        camera::{Camera, PerspectiveProjection, Projection},
        mesh::{
            buffer::{TriangleBuffer, VertexBuffer},
            vertex::StaticVertex,
            Mesh, RenderPath,
        },
    },
};
use std::collections::HashMap;
use trickcal_shooting_balddagu::portal::{HALF_HEIGHT, HALF_WIDTH};

pub(super) const COLORS: [[u8; 3]; 2] = [[255, 218, 64], [61, 159, 255]];
const VIEWS: usize = PLAYERS * 2;

#[derive(Debug)]
struct Window {
    camera: Handle<Node>,
    surface: Handle<Node>,
    material: MaterialResource,
    target: TextureResource,
}

#[derive(Debug, Default)]
pub(super) struct Portals {
    windows: Vec<Window>,
    size: (u32, u32),
    materials: HashMap<u64, (MaterialResource, MaterialResource)>,
    shaders: HashMap<u64, ShaderResource>,
}

impl Portals {
    pub fn new(g: &mut Graph) -> Self {
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("portal.shader")).expect("portal shader"),
        );
        let vertices = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .map(|(x, y)| StaticVertex::from_pos_uv(Vector3::new(x, y, 0.0), Vector2::new(x, y)));
        let data = SurfaceResource::new_embedded(SurfaceData::new(
            VertexBuffer::new(4, vertices.to_vec()).unwrap(),
            TriangleBuffer::new(vec![
                TriangleDefinition([0, 1, 2]),
                TriangleDefinition([0, 2, 3]),
            ]),
        ));
        let windows = (0..VIEWS)
            .map(|i| {
                let target = TextureResource::new_render_target(640, 400);
                let mut material = Material::from_shader(shader.clone());
                material.set_property(
                    "color",
                    vec(V3::new(
                        COLORS[i % 2][0] as f32 / 255.0,
                        COLORS[i % 2][1] as f32 / 255.0,
                        COLORS[i % 2][2] as f32 / 255.0,
                    )),
                );
                material.bind(
                    "portalTexture",
                    MaterialResourceBinding::from(None::<TextureResource>),
                );
                let material = MaterialResource::new(material);
                let camera: Handle<Node> =
                    CameraBuilder::new(BaseBuilder::new().with_name(format!("Portal view {i}")))
                        .enabled(false)
                        .with_z_near(0.01)
                        .with_z_far(180.0)
                        .with_exposure(Exposure::Manual(1.0))
                        .with_render_target(Some(target.clone()))
                        .build(g)
                        .to_base();
                let surface: Handle<Node> = MeshBuilder::new(
                    BaseBuilder::new()
                        .with_name(format!("Portal aperture {i}"))
                        .with_visibility(false)
                        .with_cast_shadows(false),
                )
                .with_render_path(RenderPath::Forward)
                .with_surfaces(vec![SurfaceBuilder::new(data.clone())
                    .with_material(material.clone())
                    .build()])
                .build(g)
                .to_base();
                g[camera]
                    .render_mask
                    .set_value_and_mark_modified(BitMask(2));
                g[surface]
                    .render_mask
                    .set_value_and_mark_modified(BitMask(1));
                Window {
                    camera,
                    surface,
                    material,
                    target,
                }
            })
            .collect();
        Self {
            windows,
            size: (640, 400),
            ..Default::default()
        }
    }

    pub fn update(
        &mut self,
        g: &mut Graph,
        actors: &[Actor; PLAYERS],
        main: Handle<Node>,
        size: Vector2<f32>,
        show: bool,
        time: f32,
    ) {
        let camera = g[main].cast::<Camera>().unwrap();
        let position = **camera.local_transform().position();
        let rotation = **camera.local_transform().rotation();
        let eye = V3::new(position.x, position.y, position.z);
        let projection = camera.projection_value();
        let fov = match projection {
            Projection::Perspective(p) => p.fov,
            _ => 80.0_f32.to_radians(),
        };
        let target_size = (
            640,
            ((640.0 * size.y / size.x.max(1.0)).round() as u32).max(1),
        );
        let mut clip_cameras = vec![Vector4::zeros(); VIEWS];
        let mut clip_planes = vec![Vector4::zeros(); VIEWS];
        let forward = rotation * Vector3::z();
        // Camera-mounted hands and weapons are only visible to the main eye.
        let first_person: Vec<_> = g
            .traverse_handle_iter(main)
            .filter(|h| *h != main)
            .collect();
        for h in first_person {
            g[h].render_mask.set_value_and_mark_modified(BitMask(1));
        }
        for (i, window) in self.windows.iter_mut().enumerate() {
            if self.size != target_size {
                window.target = TextureResource::new_render_target(target_size.0, target_size.1);
                g[window.camera]
                    .cast_mut::<Camera>()
                    .unwrap()
                    .set_render_target(Some(window.target.clone()));
                window.material.data_ref().bind(
                    "portalTexture",
                    MaterialResourceBinding::from(window.target.clone()),
                );
            }
            let actor = &actors[i / 2];
            let end = actor.portals.ends[i % 2];
            let other = actor.portals.ends[1 - i % 2];
            let active = show && actor.alive() && end.is_some();
            g[window.surface].set_visibility(active);
            g[window.camera]
                .cast_mut::<Camera>()
                .unwrap()
                .set_enabled(false);
            window.material.data_ref().set_property("linked", 0.0_f32);
            window.material.data_ref().bind(
                "portalTexture",
                MaterialResourceBinding::from(None::<TextureResource>),
            );
            if !active {
                continue;
            }
            let entry = end.unwrap();
            g[window.surface]
                .local_transform_mut()
                .set_position(vec(entry.center + entry.normal() * 0.035))
                .set_rotation(UnitQuaternion::face_towards(
                    &vec(entry.normal()),
                    &vec(entry.up()),
                ))
                .set_scale(Vector3::new(HALF_WIDTH, HALF_HEIGHT, 1.0));
            window.material.data_ref().set_property("time", time);
            let Some(exit) = other else {
                continue;
            };
            let delta = entry.center - eye;
            if (eye - entry.center).dot(entry.normal()) <= 0.015
                || delta.length() > 85.0
                || vec(delta).dot(&forward) < -2.0
            {
                continue;
            }
            let view = entry.point_to(exit, eye);
            let transform = |v: Vector3<f32>| vec(entry.vector_to(exit, V3::new(v.x, v.y, v.z)));
            let look = transform(rotation * Vector3::z());
            let up = transform(rotation * Vector3::y());
            g[window.camera]
                .local_transform_mut()
                .set_position(vec(view))
                .set_rotation(UnitQuaternion::face_towards(&look, &up));
            let camera = g[window.camera].cast_mut::<Camera>().unwrap();
            camera.set_projection(Projection::Perspective(PerspectiveProjection {
                fov,
                z_near: 0.01,
                z_far: 180.0,
            }));
            camera.set_enabled(true);
            clip_cameras[i] = Vector4::new(view.x, view.y, view.z, 1.0);
            let n = exit.normal();
            clip_planes[i] = Vector4::new(n.x, n.y, n.z, -n.dot(exit.center) - 0.045);
            window.material.data_ref().set_property("linked", 1.0_f32);
            window.material.data_ref().bind(
                "portalTexture",
                MaterialResourceBinding::from(window.target.clone()),
            );
        }
        self.size = target_size;
        // Scene-local copies retain textures, animation, toon lighting and shadows.
        // World-space clipping removes geometry between the virtual eye and exit.
        if clip_cameras.iter().any(|v| v.w > 0.0) {
            self.attach(g);
        }
        for (source, material) in self.materials.values() {
            let mut m = material.data_ref();
            // Keep live impact decals, ghost ripples and objective colors
            // from the owning systems when using a clipped shader variant.
            for (name, binding) in source.data_ref().bindings() {
                m.bind(name.clone(), binding.clone());
            }
            m.set_property("portalClipCameras", clip_cameras.clone());
            m.set_property("portalClipPlanes", clip_planes.clone());
        }
    }

    fn attach(&mut self, g: &mut Graph) {
        let handles: Vec<_> = g
            .pair_iter()
            .filter_map(|(h, n)| n.cast::<Mesh>().map(|_| h))
            .collect();
        for h in handles {
            for surface in g[h].cast_mut::<Mesh>().unwrap().surfaces_mut() {
                let original = surface.material();
                if original
                    .data_ref()
                    .shader()
                    .data_ref()
                    .definition
                    .name
                    .contains(" / Portal clipping")
                {
                    continue;
                }
                if let Some((_, replacement)) = self.materials.get(&original.key()) {
                    surface.set_material(replacement.clone());
                    continue;
                }
                let source = original.data_ref();
                let shader = source.shader();
                if !shader.data_ref().definition.passes.iter().any(|p| {
                    matches!(p.name.as_str(), "GBuffer" | "Forward")
                        && p.fragment_shader.contains("in vec3 position;")
                }) {
                    continue;
                }
                let shader = self
                    .shaders
                    .entry(shader.key())
                    .or_insert_with(|| clipped_shader(shader));
                let mut m = Material::from_shader(shader.clone());
                for (name, binding) in source.bindings() {
                    m.bind(name.clone(), binding.clone());
                }
                let material = MaterialResource::new(m);
                self.materials
                    .insert(original.key(), (original.clone(), material.clone()));
                drop(source);
                surface.set_material(material);
            }
        }
    }
}

fn clipped_shader(source: &ShaderResource) -> ShaderResource {
    let mut shader = Shader {
        definition: source.data_ref().definition.clone(),
        ..Default::default()
    };
    shader.definition.name.push_str(" / Portal clipping");
    let properties = shader
        .definition
        .resources
        .iter_mut()
        .find(|r| r.name.as_str() == "properties")
        .expect("material properties");
    if let ShaderResourceKind::PropertyGroup(properties) = &mut properties.kind {
        for name in ["portalClipCameras", "portalClipPlanes"] {
            properties.push(ShaderProperty::new(
                name,
                ShaderPropertyKind::Vector4Array {
                    value: vec![Vector4::zeros(); VIEWS],
                    max_len: VIEWS,
                },
            ));
        }
    }
    for pass in &mut shader.definition.passes {
        if !matches!(pass.name.as_str(), "GBuffer" | "Forward")
            || !pass.fragment_shader.contains("in vec3 position;")
        {
            continue;
        }
        let main = pass
            .fragment_shader
            .find("void main()")
            .expect("fragment main");
        let start = main + pass.fragment_shader[main..].find('{').unwrap() + 1;
        pass.fragment_shader.insert_str(
            start,
            r#"
            if (fyrox_cameraData.zNear < 0.015) {
                for (int i=0; i<24; ++i) {
                    vec4 camera=properties.portalClipCameras[i];
                    if (camera.w>0.5 && distance(camera.xyz,fyrox_cameraData.position)<0.002) {
                        if (dot(vec4(position,1.0),properties.portalClipPlanes[i])<0.0) discard;
                        break;
                    }
                }
            }
        "#,
        );
    }
    ShaderResource::new_embedded(shader)
}
