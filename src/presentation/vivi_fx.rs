//! The modeled mercury lance, a circular impact telegraph and a silver liquid pool.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    core::math::TriangleDefinition,
    material::shader::{Shader, ShaderResource},
    scene::mesh::{
        buffer::{TriangleBuffer, VertexBuffer},
        surface::Surface,
        vertex::StaticVertex,
        RenderPath,
    },
};
use trickcal_shooting_balddagu::{
    motion::{Motion, Pose},
    vivi_skill as v,
};
#[derive(serde::Deserialize)]
struct Part {
    color: [u8; 3],
    vertices: Vec<[f32; 6]>,
    triangles: Vec<[u32; 3]>,
}
fn mesh(g: &mut Graph, name: &str, surfaces: Vec<Surface>, forward: bool) -> Handle<Node> {
    MeshBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_visibility(false)
            .with_cast_shadows(!forward),
    )
    .with_surfaces(surfaces)
    .with_render_path(if forward {
        RenderPath::Forward
    } else {
        RenderPath::Deferred
    })
    .build(g)
    .to_base()
}
fn surface(
    vertices: Vec<StaticVertex>,
    triangles: Vec<TriangleDefinition>,
    mat: MaterialResource,
) -> Surface {
    let mut data = SurfaceData::new(
        VertexBuffer::new(vertices.len(), vertices).unwrap(),
        TriangleBuffer::new(triangles),
    );
    data.calculate_normals().unwrap();
    SurfaceBuilder::new(SurfaceResource::new_embedded(data))
        .with_material(mat)
        .build()
}
fn disk(mat: MaterialResource) -> Surface {
    let mut vertices = vec![StaticVertex::from_pos_uv(
        Vector3::new(0.0, 0.04, 0.0),
        Vector2::zeros(),
    )];
    let mut triangles = Vec::new();
    for i in 0..=128 {
        let a = i as f32 * std::f32::consts::TAU / 128.0;
        vertices.push(StaticVertex::from_pos_uv(
            Vector3::new(a.cos() * v::POOL_RADIUS, 0.04, a.sin() * v::POOL_RADIUS),
            Vector2::zeros(),
        ));
        if i > 0 {
            triangles.push(TriangleDefinition([0, i + 1, i]));
        }
    }
    surface(vertices, triangles, mat)
}
fn splash(mat: MaterialResource) -> Surface {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for j in 0..64 {
        let a = j as f32 * std::f32::consts::TAU / 64.0;
        let b = (j + 1) as f32 * std::f32::consts::TAU / 64.0;
        let height = 0.30 + ((j * 7) % 9) as f32 * 0.10;
        let base = vertices.len() as u32;
        for (angle, r, y) in [
            (a, 0.6, 0.05),
            (b, 0.6, 0.05),
            (b, 1.4, height),
            (a, 1.4, height),
        ] {
            vertices.push(StaticVertex::from_pos_uv(
                Vector3::new(angle.cos() * r, y, angle.sin() * r),
                Vector2::zeros(),
            ));
        }
        triangles.extend([
            TriangleDefinition([base, base + 1, base + 2]),
            TriangleDefinition([base, base + 2, base + 3]),
        ]);
    }
    surface(vertices, triangles, mat)
}
#[derive(Debug)]
pub(super) struct Effects {
    pub fields: Vec<[Handle<Node>; 3]>,
    materials: Vec<[MaterialResource; 2]>,
    held: [Handle<Node>; PLAYERS],
    view: Handle<Node>,
}
impl Effects {
    pub fn new(g: &mut Graph) -> Self {
        let parts: Vec<Part> =
            serde_json::from_str(include_str!("../../assets/effects/vivi-lance-mesh.json"))
                .expect("modeled Vivi lance");
        let surfaces: Vec<_> = parts
            .into_iter()
            .map(|p| {
                let mat = material(p.color);
                mat.data_ref().set_property("roughness", 0.24_f32);
                mat.data_ref().set_property("metallic", 0.65_f32);
                surface(
                    p.vertices
                        .into_iter()
                        .map(|v| {
                            StaticVertex::from_pos_uv(
                                Vector3::new(v[0], v[1], v[2]),
                                Vector2::zeros(),
                            )
                        })
                        .collect(),
                    p.triangles.into_iter().map(TriangleDefinition).collect(),
                    mat,
                )
            })
            .collect();
        let held =
            std::array::from_fn(|_| mesh(g, "Vivi summoned hand lance", surfaces.clone(), false));
        let view = mesh(g, "Vivi first-person lance", surfaces.clone(), false);
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("vivi_mercury.shader"))
                .expect("silver mercury shader"),
        );
        let mut fields = Vec::new();
        let mut materials = Vec::new();
        for _ in 0..MAX_PROJECTILES {
            let mut crown = Material::from_shader(shader.clone());
            crown.set_property("splash", 1.0_f32);
            let mats = [
                MaterialResource::new(Material::from_shader(shader.clone())),
                MaterialResource::new(crown),
            ];
            fields.push([
                mesh(g, "Vivi giant falling lance", surfaces.clone(), false),
                mesh(
                    g,
                    "Vivi silver mercury circle",
                    vec![disk(mats[0].clone())],
                    true,
                ),
                mesh(
                    g,
                    "Vivi mercury splash crown",
                    vec![splash(mats[1].clone())],
                    true,
                ),
            ]);
            materials.push(mats);
        }
        Self {
            fields,
            materials,
            held,
            view,
        }
    }
    pub fn fields(&self, g: &mut Graph, w: &World, show: bool) {
        for (i, nodes) in self.fields.iter().enumerate() {
            for &n in nodes {
                g[n].set_visible(false);
            }
            let Some(p) = w
                .projectiles
                .get(i)
                .filter(|p| p.kind == ProjectileKind::MercuryField && show)
            else {
                continue;
            };
            let t = p.age - v::FALL_SECONDS;
            let lance_visible = t < 1.10;
            g[nodes[0]].set_visible(lance_visible);
            g[nodes[0]]
                .local_transform_mut()
                .set_position(
                    vec(p.pos)
                        + Vector3::y()
                            * (v::lance_tip(p.age) + 1.75 * 3.0
                                - v::smooth((t - 0.30) / 0.80) * 8.5),
                )
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::z_axis(),
                    std::f32::consts::PI,
                ))
                .set_scale(Vector3::repeat(3.0));
            g[nodes[1]].set_visible(true);
            g[nodes[1]].local_transform_mut().set_position(vec(p.pos));
            g[nodes[2]].set_visible((0.0..0.65).contains(&t));
            g[nodes[2]]
                .local_transform_mut()
                .set_position(vec(p.pos))
                .set_scale(Vector3::new(
                    1.0 + t.max(0.0) * 3.0,
                    (1.0 - v::smooth(t / 0.65)).max(0.001),
                    1.0 + t.max(0.0) * 3.0,
                ));
            for mat in &self.materials[i] {
                mat.data_ref().set_property("age", p.age);
            }
        }
    }
    pub fn held(
        &self,
        g: &mut Graph,
        slot: usize,
        pose: Pose,
        origin: Vector3<f32>,
        rotation: UnitQuaternion<f32>,
        first: bool,
        show: bool,
    ) {
        let node = if first { self.view } else { self.held[slot] };
        let visible = show
            && pose.motion == Motion::ViviUltimate
            && (0.13..v::LANCE_RELEASE).contains(&pose.age);
        g[node].set_visible(visible);
        if visible {
            let upright = v::smooth((pose.age - 0.45) / 0.3);
            let tilt = (1.0 - upright) * 1.25;
            g[node]
                .local_transform_mut()
                .set_position(origin)
                .set_rotation(rotation * UnitQuaternion::from_euler_angles(0.12, 0.0, tilt))
                .set_scale(Vector3::repeat(if first { 0.48 } else { 0.72 }));
        }
    }
    pub fn hide_view(&self, g: &mut Graph) {
        g[self.view].set_visible(false);
    }
}
