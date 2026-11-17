//! Portal markers only; destination cameras and scene clipping are disabled.
use super::*;
use fyrox::{
    core::math::TriangleDefinition,
    material::shader::{Shader, ShaderResource},
    scene::mesh::{
        buffer::{TriangleBuffer, VertexBuffer},
        vertex::StaticVertex,
        RenderPath,
    },
};
use trickcal_shooting_balddagu::portal::{HALF_HEIGHT, HALF_WIDTH};

pub(super) const COLORS: [[u8; 3]; 2] = [[255, 218, 64], [61, 159, 255]];
const APERTURES: usize = PLAYERS * 2;

#[derive(Debug)]
struct Aperture {
    surface: Handle<Node>,
    material: MaterialResource,
}

#[derive(Debug, Default)]
pub(super) struct Portals {
    apertures: Vec<Aperture>,
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
        let apertures = (0..APERTURES)
            .map(|i| {
                let mut material = Material::from_shader(shader.clone());
                material.set_property(
                    "color",
                    vec(V3::new(
                        COLORS[i % 2][0] as f32 / 255.0,
                        COLORS[i % 2][1] as f32 / 255.0,
                        COLORS[i % 2][2] as f32 / 255.0,
                    )),
                );
                let material = MaterialResource::new(material);
                let surface = MeshBuilder::new(
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
                Aperture { surface, material }
            })
            .collect();
        Self { apertures }
    }

    pub fn update(&mut self, g: &mut Graph, actors: &[Actor; PLAYERS], show: bool, time: f32) {
        for (i, aperture) in self.apertures.iter_mut().enumerate() {
            let actor = &actors[i / 2];
            let end = actor.portals.ends[i % 2];
            let active = show && actor.alive() && end.is_some();
            g[aperture.surface].set_visibility(active);
            if !active {
                continue;
            }
            let entry = end.unwrap();
            g[aperture.surface]
                .local_transform_mut()
                .set_position(vec(entry.center + entry.normal() * 0.035))
                .set_rotation(UnitQuaternion::face_towards(
                    &vec(entry.normal()),
                    &vec(entry.up()),
                ))
                .set_scale(Vector3::new(HALF_WIDTH, HALF_HEIGHT, 1.0));
            aperture.material.data_ref().set_property("time", time);
        }
    }
}
