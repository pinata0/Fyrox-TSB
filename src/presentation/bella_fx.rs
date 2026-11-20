//! Open-cage lantern, palm fire, one inbound soul per confirmed target and a white finale.
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
    bella_skill as b,
    motion::{Motion, Pose},
};

#[derive(serde::Deserialize)]
struct Part {
    group: String,
    color: [u8; 3],
    vertices: Vec<[f32; 6]>,
    triangles: Vec<[u32; 3]>,
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
fn mesh(g: &mut Graph, name: &str, surfaces: Vec<Surface>, forward: bool) -> Handle<Node> {
    MeshBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_cast_shadows(false)
            .with_visibility(false),
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
fn energy(shader: &ShaderResource, mode: f32) -> MaterialResource {
    let mut m = Material::from_shader(shader.clone());
    m.set_property("mode", mode);
    MaterialResource::new(m)
}
fn soul(mat: MaterialResource) -> Surface {
    let mut v = Vec::new();
    let mut t = Vec::new();
    for j in 0..=16 {
        let h = j as f32 / 16.0;
        let r = (h * std::f32::consts::PI).sin().powf(0.7) * 0.062;
        for i in 0..16 {
            let a = i as f32 * std::f32::consts::TAU / 16.0;
            v.push(StaticVertex::from_pos_uv(
                Vector3::new(
                    a.cos() * r + 0.025 * (h * 7.0).sin() * h,
                    a.sin().abs() * 0.005 + h * 0.32,
                    a.sin() * r,
                ),
                Vector2::new(i as f32 / 16.0, h),
            ));
            if j < 16 {
                let n = j * 16 + i;
                let k = j * 16 + (i + 1) % 16;
                t.extend([
                    TriangleDefinition([n, k, n + 16]),
                    TriangleDefinition([k, k + 16, n + 16]),
                ]);
            }
        }
    }
    surface(v, t, mat)
}
fn field(mat: MaterialResource) -> Surface {
    let v = [(-1.5, 0.0), (1.5, 0.0), (1.5, 10.0), (-1.5, 10.0)]
        .into_iter()
        .map(|(x, z)| StaticVertex::from_pos_uv(Vector3::new(x, 0.035, z), Vector2::zeros()))
        .collect();
    surface(
        v,
        vec![TriangleDefinition([0, 2, 1]), TriangleDefinition([0, 3, 2])],
        mat,
    )
}
fn tongues(mat: MaterialResource) -> Surface {
    let mut v = Vec::new();
    let mut tri = Vec::new();
    for z in 0..20 {
        for x in 0..6 {
            for plane in 0..2 {
                let p = Vector3::new(-1.28 + x as f32 * 0.51, 0.04, 0.25 + z as f32 * 0.50);
                let right = if plane == 0 {
                    Vector3::x()
                } else {
                    Vector3::z()
                };
                let n = v.len() as u32;
                let height = 0.38 + ((x * 7 + z * 11) % 9) as f32 * 0.055;
                for (dx, y, u) in [
                    (-0.17, 0.0, 0.0),
                    (0.17, 0.0, 1.0),
                    (0.17, height, 1.0),
                    (-0.17, height, 0.0),
                ] {
                    v.push(StaticVertex::from_pos_uv(
                        p + right * dx + Vector3::y() * y,
                        Vector2::new(u, if y == 0.0 { 0.0 } else { 1.0 }),
                    ));
                }
                tri.extend([
                    TriangleDefinition([n, n + 1, n + 2]),
                    TriangleDefinition([n, n + 2, n + 3]),
                ]);
            }
        }
    }
    surface(v, tri, mat)
}
fn ring(mat: MaterialResource) -> Surface {
    let mut v = Vec::new();
    let mut t = Vec::new();
    for i in 0..128 {
        let a = i as f32 * std::f32::consts::TAU / 128.0;
        for (r, u) in [(0.88, 0.0), (0.94, 0.5), (1.0, 1.0)] {
            v.push(StaticVertex::from_pos_uv(
                Vector3::new(a.cos() * r, 0.0, a.sin() * r),
                Vector2::new(0.0, u),
            ));
        }
        for j in 0..2 {
            let n = i * 3 + j;
            let k = ((i + 1) % 128) * 3 + j;
            t.extend([
                TriangleDefinition([n, k, n + 1]),
                TriangleDefinition([n + 1, k, k + 1]),
            ]);
        }
    }
    surface(v, t, mat)
}
#[derive(Debug)]
pub(super) struct Effects {
    pub lanterns: [Handle<Node>; PLAYERS],
    pub view_lantern: Handle<Node>,
    pub palm_fire: Handle<Node>,
    cores: [Handle<Node>; PLAYERS],
    view_core: Handle<Node>,
    streams: [[Handle<Node>; PLAYERS]; PLAYERS],
    soul_material: MaterialResource,
    fields: Vec<[Handle<Node>; 2]>,
    field_materials: Vec<[MaterialResource; 2]>,
    waves: Vec<Handle<Node>>,
    wave_materials: Vec<MaterialResource>,
}
impl Effects {
    pub fn new(g: &mut Graph) -> Self {
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("bella_energy.shader")).expect("Bella energy shader"),
        );
        let soul_material = energy(&shader, 0.0);
        let flame = soul(soul_material.clone());
        let data: Vec<Part> =
            serde_json::from_str(include_str!("../../assets/effects/bella-lantern-mesh.json"))
                .expect("Bella modeled lantern");
        let surfaces: Vec<_> = data
            .into_iter()
            .filter(|p| p.group != "flame" && p.group != "core")
            .map(|p| {
                let v = p
                    .vertices
                    .into_iter()
                    .map(|p| {
                        StaticVertex::from_pos_uv(Vector3::new(p[0], p[1], p[2]), Vector2::zeros())
                    })
                    .collect();
                surface(
                    v,
                    p.triangles.into_iter().map(TriangleDefinition).collect(),
                    material(p.color),
                )
            })
            .collect();
        let lanterns =
            std::array::from_fn(|_| mesh(g, "Bella ultimate lantern", surfaces.clone(), false));
        let view_lantern = mesh(g, "Bella first-person lantern", surfaces, false);
        let palm_fire = mesh(g, "Bella spinning palm flame", vec![flame.clone()], true);
        let cores = std::array::from_fn(|_| {
            mesh(g, "Bella lantern living flame", vec![flame.clone()], true)
        });
        let view_core = mesh(
            g,
            "Bella first-person lantern core",
            vec![flame.clone()],
            true,
        );
        let streams = std::array::from_fn(|_| {
            std::array::from_fn(|_| mesh(g, "Bella captured enemy soul", vec![flame.clone()], true))
        });
        let mut fields = Vec::new();
        let mut field_materials = Vec::new();
        let mut waves = Vec::new();
        let mut wave_materials = Vec::new();
        for _ in 0..MAX_PROJECTILES {
            let mats = [energy(&shader, 1.0), energy(&shader, 3.0)];
            fields.push([
                mesh(
                    g,
                    "Bella violet fire rectangle",
                    vec![field(mats[0].clone())],
                    true,
                ),
                mesh(
                    g,
                    "Bella rising violet fire",
                    vec![tongues(mats[1].clone())],
                    true,
                ),
            ]);
            field_materials.push(mats);
            let m = energy(&shader, 2.0);
            waves.push(mesh(
                g,
                "Bella white healing ring",
                vec![ring(m.clone())],
                true,
            ));
            wave_materials.push(m);
        }
        Self {
            lanterns,
            view_lantern,
            palm_fire,
            cores,
            view_core,
            streams,
            soul_material,
            fields,
            field_materials,
            waves,
            wave_materials,
        }
    }
    pub fn fields(&self, g: &mut Graph, w: &World, enabled: bool) {
        self.soul_material
            .data_ref()
            .set_property("age", w.tick as f32 * DT);
        for i in 0..MAX_PROJECTILES {
            let p = w.projectiles.get(i);
            let fire = enabled && p.is_some_and(|p| p.kind == ProjectileKind::BellaFire);
            let heal = enabled && p.is_some_and(|p| p.kind == ProjectileKind::BellaHeal);
            for &n in &self.fields[i] {
                g[n].set_visible(fire);
            }
            g[self.waves[i]].set_visible(heal);
            if let Some(p) = p {
                if fire {
                    for j in 0..2 {
                        self.field_materials[i][j]
                            .data_ref()
                            .set_property("age", w.tick as f32 * DT);
                        g[self.fields[i][j]]
                            .local_transform_mut()
                            .set_position(vec(p.pos))
                            .set_rotation(UnitQuaternion::from_axis_angle(
                                &Vector3::y_axis(),
                                p.velocity.x.atan2(p.velocity.z),
                            ));
                    }
                }
                if heal {
                    self.wave_materials[i].data_ref().set_property("age", p.age);
                    g[self.waves[i]]
                        .local_transform_mut()
                        .set_position(vec(p.pos) + Vector3::y() * 0.14)
                        .set_scale(Vector3::repeat(
                            (p.age / b::WAVE_SECONDS).clamp(0.01, 1.0) * b::HEAL_RADIUS,
                        ));
                }
            }
        }
    }
    pub fn begin(&self, g: &mut Graph) {
        for n in [self.view_lantern, self.view_core, self.palm_fire] {
            g[n].set_visible(false);
        }
    }
    pub fn actor(
        &self,
        g: &mut Graph,
        w: &World,
        i: usize,
        a: &Actor,
        p: Pose,
        palm: Vector3<f32>,
        camera: Handle<Node>,
        first: bool,
        show: bool,
    ) {
        let show =
            show && a.alive() && a.character == 9 && a.combat.polymorph <= 0.0 && !a.burrowed();
        let ultimate = show
            && p.motion == Motion::BellaUltimate
            && (0.25..b::ULTIMATE_SECONDS).contains(&p.age);
        let node = if first {
            self.view_lantern
        } else {
            self.lanterns[i]
        };
        let core = if first { self.view_core } else { self.cores[i] };
        g[self.lanterns[i]].set_visible(ultimate && !first);
        g[self.cores[i]].set_visible(ultimate && !first);
        if first {
            g[self.view_lantern].set_visible(ultimate);
            g[self.view_core].set_visible(ultimate);
        }
        let rotation = if first {
            skeleton::world_rotation(g, camera)
        } else {
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), a.yaw)
        };
        let swing = b::lantern(p.motion, p.age)[2].sin() * 0.13;
        let rotation = rotation
            * UnitQuaternion::from_euler_angles(0.0, 0.0, swing + if first { 0.32 } else { 0.0 });
        let scale = if first { 0.80 } else { 1.25 * a.body_scale() };
        let center = palm + rotation * Vector3::new(0.0, -0.34 * scale, 0.0);
        if ultimate {
            g[node]
                .local_transform_mut()
                .set_position(palm)
                .set_rotation(rotation)
                .set_scale(Vector3::repeat(scale));
            g[core]
                .local_transform_mut()
                .set_position(palm + rotation * Vector3::new(0.0, -0.405 * scale, 0.0))
                .set_rotation(
                    rotation * UnitQuaternion::from_axis_angle(&Vector3::y_axis(), p.age * 3.0),
                )
                .set_scale(Vector3::repeat(scale * 0.52));
        }
        let idle_fire = show && first && !ultimate && a.wave_ticks == 0;
        if first {
            g[self.palm_fire].set_visible(idle_fire);
        }
        if idle_fire {
            let t = w.tick as f32 * DT;
            g[self.palm_fire]
                .local_transform_mut()
                .set_position(palm + Vector3::y() * (0.10 + 0.025 * (t * 3.0).sin()))
                .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), t * 3.2))
                .set_scale(Vector3::repeat(0.70));
        }
        let mask = a.combat.bella.targets;
        let count = mask.count_ones().max(1);
        let mut ordinal = 0;
        for target in 0..PLAYERS {
            let selected = mask & (1 << target) != 0;
            let start = b::TAUNT_RELEASE + ordinal as f32 * 0.10;
            let duration = (3.5 - start).max(0.7);
            let t = (p.age - start) / duration;
            let visible = ultimate && selected && (0.0..1.0).contains(&t);
            let n = self.streams[i][target];
            g[n].set_visible(visible);
            if visible {
                let from = vec(w.actors[target].eye());
                let d = center - from;
                let u = t * t * (3.0 - 2.0 * t);
                let side = Vector3::new(d.z, 0.0, -d.x)
                    .try_normalize(0.001)
                    .unwrap_or(Vector3::x());
                let arc = (t * std::f32::consts::PI).sin();
                let pos = from
                    + d * u
                    + side * arc * (0.30 + ordinal as f32 / count as f32 * 0.35)
                    + Vector3::y() * arc * 0.6;
                g[n].local_transform_mut()
                    .set_position(pos)
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        p.age * 6.0 + target as f32,
                    ))
                    .set_scale(Vector3::repeat(0.85 * (1.0 - t * 0.45)));
            }
            if selected {
                ordinal += 1;
            }
        }
    }
}
