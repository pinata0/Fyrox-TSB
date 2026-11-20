//! Independently rotating open clock structures, head halo, palm beams and hollow hexagons.
use crate::presentation::SceneState;
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
use std::f32::consts::TAU;
use trickcal_shooting_balddagu::{
    motion::{Motion, Pose},
    renua_skill as renua,
};

fn energy(blue: bool) -> MaterialResource {
    static SHADER: std::sync::OnceLock<ShaderResource> = std::sync::OnceLock::new();
    let shader = SHADER.get_or_init(|| {
        ShaderResource::new_embedded(
            Shader::from_string(include_str!("renua_energy.shader")).expect("Renua energy"),
        )
    });
    let mut m = Material::from_shader(shader.clone());
    m.set_property("blue", if blue { 1.0 } else { 0.0 });
    MaterialResource::new(m)
}
fn geometry(paths: Vec<Vec<Vector3<f32>>>, width: f32) -> SurfaceResource {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for path in paths {
        for points in path.windows(2) {
            let d = (points[1] - points[0]).normalize();
            let side = Vector3::new(-d.y, d.x, 0.0) * width * 0.5;
            let b = vertices.len() as u32;
            for (p, uv) in [
                (points[0] - side, Vector2::new(0.0, 0.0)),
                (points[0] + side, Vector2::new(1.0, 0.0)),
                (points[1] - side, Vector2::new(0.0, 1.0)),
                (points[1] + side, Vector2::new(1.0, 1.0)),
            ] {
                vertices.push(StaticVertex::from_pos_uv(p, uv));
            }
            triangles.extend([
                TriangleDefinition([b, b + 1, b + 2]),
                TriangleDefinition([b + 1, b + 3, b + 2]),
            ]);
        }
    }
    SurfaceResource::new_embedded(SurfaceData::new(
        VertexBuffer::new(vertices.len(), vertices).unwrap(),
        TriangleBuffer::new(triangles),
    ))
}
fn circle(radius: f32, n: usize) -> Vec<Vector3<f32>> {
    (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * TAU;
            Vector3::new(a.cos() * radius, a.sin() * radius, 0.0)
        })
        .collect()
}
fn clock(square: bool) -> SurfaceResource {
    let mut paths = vec![circle(1.0, 96), circle(0.91, 96)];
    for i in 0..4 {
        let a = i as f32 * TAU / 4.0;
        let rot = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), a);
        paths.push(
            [
                [0.0, 1.07, 0.0],
                [-0.085, 1.23, 0.0],
                [0.0, 1.51, 0.0],
                [0.085, 1.23, 0.0],
                [0.0, 1.07, 0.0],
            ]
            .map(|p| rot * Vector3::from(p))
            .to_vec(),
        );
    }
    if !square {
        for i in 0..32 {
            let a = i as f32 * TAU / 32.0;
            let d = Vector3::new(a.cos(), a.sin(), 0.0);
            paths.push(vec![d * 0.78, d * if i % 4 == 0 { 0.88 } else { 0.83 }]);
        }
    }
    geometry(paths, 0.065)
}
fn node(g: &mut Graph, name: &str, data: SurfaceResource, mat: MaterialResource) -> Handle<Node> {
    MeshBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_cast_shadows(false)
            .with_visibility(false),
    )
    .with_render_path(RenderPath::Forward)
    .with_surfaces(vec![SurfaceBuilder::new(data).with_material(mat).build()])
    .build(g)
    .to_base()
}
fn field_mesh() -> SurfaceResource {
    let mut paths = Vec::new();
    let r = 0.47;
    for row in 0..14 {
        for column in 0..10 {
            let x = (column as f32 - 4.5) * 0.78 + if row % 2 == 0 { 0.0 } else { 0.39 };
            let y = 0.5 + row as f32 * 0.83;
            if x.abs() + r > renua::FIELD_HALF_WIDTH || y + r > renua::FIELD_DEPTH {
                continue;
            }
            paths.push(
                circle(r, 6)
                    .into_iter()
                    .map(|p| p + Vector3::new(x, y, 0.0))
                    .collect(),
            );
        }
    }
    geometry(paths, 0.080)
}
#[derive(Debug)]
pub(super) struct Effects {
    pub halos: [[Handle<Node>; 3]; PLAYERS],
    pub constructs: [[Handle<Node>; 13]; PLAYERS],
    materials: [[MaterialResource; 13]; PLAYERS],
    pub beams: [Handle<Node>; PLAYERS],
    pub fields: [Handle<Node>; MAX_PROJECTILES],
}
impl Effects {
    pub fn new(g: &mut Graph) -> Self {
        let outer = clock(false);
        let plain = clock(true);
        let orbit = geometry(vec![circle(0.64, 64)], 0.055);
        let square = geometry(vec![circle(0.55, 4)], 0.060);
        let halo = clock(true);
        let gold = energy(false);
        let materials = std::array::from_fn(|_| std::array::from_fn(|_| energy(false)));
        let constructs = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                node(
                    g,
                    &format!("Renua summoned clock {i}/{j}"),
                    match j {
                        5 | 6 => square.clone(),
                        8 | 10 | 11 | 12 => orbit.clone(),
                        4 => plain.clone(),
                        _ => outer.clone(),
                    },
                    materials[i][j].clone(),
                )
            })
        });
        let halos = std::array::from_fn(|_| {
            std::array::from_fn(|j| {
                node(
                    g,
                    "Renua rotating head halo",
                    if j == 0 { halo.clone() } else { orbit.clone() },
                    gold.clone(),
                )
            })
        });
        let beam = geometry(vec![vec![Vector3::zeros(), Vector3::y()]], 2.0);
        let beams =
            std::array::from_fn(|_| node(g, "Renua palm laser", beam.clone(), energy(true)));
        let field = field_mesh();
        let fields = std::array::from_fn(|_| {
            node(
                g,
                "Renua hollow yellow hexagon field",
                field.clone(),
                gold.clone(),
            )
        });
        Self {
            halos,
            constructs,
            materials,
            beams,
            fields,
        }
    }
    pub fn actor(
        &self,
        g: &mut Graph,
        index: usize,
        actor: &Actor,
        pose: Pose,
        head: Vector3<f32>,
        palm: Vector3<f32>,
        camera: Handle<Node>,
        first: bool,
        enabled: bool,
    ) {
        let show = enabled && actor.character == 7 && actor.alive();
        if !show {
            for node in self.halos[index].iter().chain(self.constructs[index].iter()) {
                g[*node].set_visible(false);
            }
            g[self.beams[index]].set_visible(false);
            return;
        }
        let facing = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), actor.yaw);
        let (a, b, t) = renua::sample(pose.motion, pose.age);
        let mix = |x: f32, y: f32| x + (y - x) * t;
        for (j, n) in self.halos[index].iter().enumerate() {
            g[*n].set_visible(show && !first);
            let angle = mix(a.details[j], b.details[j]);
            g[*n]
                .local_transform_mut()
                .set_position(head + Vector3::y() * 0.55 - facing * Vector3::z() * 0.08)
                .set_scale(Vector3::new(0.42, 0.30, 0.42) * actor.body_scale())
                .set_rotation(
                    facing
                        * UnitQuaternion::from_euler_angles(
                            0.48 + j as f32 * 0.25,
                            angle,
                            if j == 1 { angle * 0.5 } else { 0.0 },
                        ),
                );
        }
        for j in 0..13 {
            let n = self.constructs[index][j];
            let p: [f32; 6] = std::array::from_fn(|k| mix(a.constructs[j][k], b.constructs[j][k]));
            g[n].set_visible(show && p[5] > 0.01);
            self.materials[index][j]
                .data_ref()
                .set_property("opacity", p[5]);
            // Source forward is -X. Split the two E/Q banks across the body's sides.
            let paired = matches!(pose.motion, Motion::RenuaSkill | Motion::RenuaUltimate);
            let pos = if paired {
                Vector3::new(p[0] * 0.80, 1.0 + p[1] * 0.70, 0.42)
            } else {
                Vector3::new(-0.28, 1.02 + p[1] * 0.55, -p[0] * 0.75)
            };
            let (origin, rotation, scale) = if first {
                let v = if paired {
                    Vector3::new(p[0] * 0.37, -0.24 + p[1] * 0.23, 0.90)
                } else {
                    Vector3::new(-0.30, -0.18 + p[1] * 0.20, 0.85 - p[0] * 0.20)
                };
                (
                    g[camera]
                        .global_transform()
                        .transform_point(&v.into())
                        .coords,
                    super::skeleton::world_rotation(g, camera),
                    0.18,
                )
            } else {
                (
                    vec(actor.pos) + facing * (pos * actor.body_scale()),
                    facing,
                    0.32 * actor.body_scale(),
                )
            };
            g[n].local_transform_mut()
                .set_position(origin)
                .set_scale(Vector3::new(
                    p[3].max(0.001) * scale,
                    p[4].max(0.001) * scale,
                    scale,
                ))
                .set_rotation(rotation * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), p[2]));
        }
        let beam = self.beams[index];
        let laser = matches!(pose.motion, Motion::RenuaEnhanced | Motion::RenuaSkill);
        g[beam].set_visible(show && laser && pose.shot_age < renua::BEAM_SECONDS);
        if show && laser && pose.shot_age < renua::BEAM_SECONDS {
            let delta = vec(actor.shot_end) - palm;
            let dir = delta.try_normalize(0.0001).unwrap_or(Vector3::y());
            let view = (g[camera].global_position() - (palm + vec(actor.shot_end)) * 0.5)
                .try_normalize(0.0001)
                .unwrap_or(Vector3::z());
            let across = dir
                .cross(&view)
                .try_normalize(0.0001)
                .unwrap_or(Vector3::x());
            let normal = across.cross(&dir);
            g[beam]
                .local_transform_mut()
                .set_position(palm)
                .set_rotation(UnitQuaternion::from_matrix(
                    &fyrox::core::algebra::Matrix3::from_columns(&[across, dir, normal]),
                ))
                .set_scale(Vector3::new(0.055, delta.norm(), 1.0));
        }
    }
    pub fn fields(&self, g: &mut Graph, world: &World, enabled: bool) {
        for (j, node) in self.fields.iter().enumerate() {
            let field = world
                .projectiles
                .get(j)
                .filter(|p| p.kind == ProjectileKind::TimeField);
            g[*node].set_visible(enabled && field.is_some());
            if let Some(p) = field {
                let yaw = p.velocity.x.atan2(p.velocity.z);
                g[*node]
                    .local_transform_mut()
                    .set_position(vec(p.pos) + Vector3::y() * 0.035)
                    .set_rotation(
                        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw)
                            * UnitQuaternion::from_axis_angle(
                                &Vector3::x_axis(),
                                std::f32::consts::FRAC_PI_2,
                            ),
                    );
            }
        }
    }
}
