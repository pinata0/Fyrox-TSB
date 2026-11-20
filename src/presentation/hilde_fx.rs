//! Modeled medical case with hinged lid, rotary launcher, and healing field.
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
    hilde_skill as h,
    motion::{Motion, Pose},
};
pub(super) const SCALE: f32 = 1.25;
pub(super) const VIEW_SCALE: f32 = 0.65;
pub(super) const BLAST_PARTS: usize = 18;
const HEAL_PARTICLES: usize = 96;
// Center of the hollow barrel opening authored in model_hilde_props.py.
const LAUNCHER_MUZZLE: Vector3<f32> = Vector3::new(0.0, 0.18, 0.69);

/// World-space burst shared by local play, network contacts and death replays.
#[derive(Debug)]
pub(super) struct Blast {
    nodes: [Handle<Node>; BLAST_PARTS],
    material: MaterialResource,
}
impl Blast {
    pub fn new(g: &mut Graph) -> Self {
        static RESOURCES: std::sync::OnceLock<(SurfaceResource, ShaderResource)> =
            std::sync::OnceLock::new();
        let (geometry, shader) = RESOURCES.get_or_init(|| {
            (
                SurfaceResource::new_embedded(SurfaceData::make_sphere(
                    16, 16, 1.0, &Matrix4::identity(),
                )),
                ShaderResource::new_embedded(
                    Shader::from_string(include_str!("hilde_blast.shader"))
                        .expect("Hilde green explosion shader"),
                ),
            )
        });
        let material = MaterialResource::new(Material::from_shader(shader.clone()));
        let surface = SurfaceBuilder::new(geometry.clone())
            .with_material(material.clone())
            .build();
        let nodes = std::array::from_fn(|_| {
            MeshBuilder::new(
                BaseBuilder::new()
                    .with_name("Hilde green explosion")
                    .with_cast_shadows(false),
            )
            .with_render_path(RenderPath::Forward)
            .with_surfaces(vec![surface.clone()])
            .build(g)
            .to_base()
        });
        Self { nodes, material }
    }

    pub fn update(&self, g: &mut Graph, position: V3, radius: f32, age: f32) {
        let progress = (age / h::BLAST_SECONDS).clamp(0.0, 1.0);
        let spread = radius * (1.0 - (-age * 18.0).exp()).max(0.04);
        self.material.data_ref().set_property("age", age);
        self.material.data_ref().set_property("opacity", (1.0 - progress).powf(1.3));
        for (i, &node) in self.nodes.iter().enumerate() {
            let angle = i as f32 * 2.399963;
            let y = 1.0 - 2.0 * ((i as f32 * 0.618034).fract());
            let horizontal = (1.0 - y * y).sqrt();
            let outward = Vector3::new(angle.cos() * horizontal, y, angle.sin() * horizontal);
            let (offset, scale, rotation) = match i {
                0 => (Vector3::zeros(), Vector3::repeat(spread * 0.48), UnitQuaternion::identity()),
                1..=6 => (
                    outward * spread * 0.42,
                    Vector3::repeat(spread * (0.28 + 0.035 * (i % 3) as f32)),
                    UnitQuaternion::from_euler_angles(angle, age * 2.0, 0.0),
                ),
                7..=8 => (
                    Vector3::zeros(),
                    Vector3::new(spread, 0.025, spread),
                    UnitQuaternion::from_euler_angles(if i == 7 { 0.0 } else { 1.2 }, 0.0, 0.5),
                ),
                _ => (
                    outward * spread * (0.75 + progress * 0.25),
                    Vector3::new(0.025, 0.025, 0.18 * (1.0 - progress) + 0.035),
                    UnitQuaternion::face_towards(&outward, &Vector3::y()),
                ),
            };
            g[node].local_transform_mut()
                .set_position(vec(position) + offset)
                .set_scale(scale)
                .set_rotation(rotation);
        }
    }

    pub fn remove(self, g: &mut Graph) {
        for node in self.nodes {
            g.remove_node(node);
        }
    }
}

pub(super) fn active(p: Pose) -> bool {
    matches!(p.motion, Motion::HildeSkill | Motion::HildeUltimate)
}
pub(super) fn support(p: Pose) -> Vector3<f32> {
    if p.motion == Motion::HildeSkill {
        Vector3::new(0.16, -0.18, 0.10)
    } else {
        Vector3::new(0.0, -0.025, 0.28)
    }
}
pub(super) fn rotation(p: Pose) -> UnitQuaternion<f32> {
    let hand = h::hand_sample(p.motion, p.age, false);
    let recoil = if p.motion == Motion::HildeUltimate {
        (1.0 - p.shot_age / 0.15).max(0.0)
    } else {
        0.0
    };
    UnitQuaternion::from_euler_angles(
        (hand[2] * 0.10).clamp(-0.25, 0.25) - recoil * 0.08,
        0.0,
        0.0,
    )
}
pub(super) fn view_origin(p: Pose) -> Vector3<f32> {
    let hand = h::hand_sample(p.motion, p.age, false);
    let lower = if p.motion == Motion::HildeUltimate {
        ((0.55 - p.age) / 0.55).clamp(0.0, 1.0) + ((p.age - 5.5) / 0.66).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let case = p.motion == Motion::HildeSkill;
    Vector3::new(
        if case {-0.22} else {-0.35},
        (if case {0.015} else {-0.27}) + (hand[1] * 0.10).clamp(-0.06, 0.08) - lower * 0.32,
        0.85 + (-hand[0] * 0.09).clamp(-0.06, 0.08),
    )
}
pub(super) fn view_rotation(p: Pose) -> UnitQuaternion<f32> {
    rotation(p) * UnitQuaternion::from_axis_angle(&Vector3::y_axis(), if p.motion==Motion::HildeSkill {std::f32::consts::PI} else {0.0})
}
pub(super) fn view_support(p: Pose) -> Vector3<f32> {
    if p.motion==Motion::HildeSkill {Vector3::new(-0.18,-0.18,0.10)} else {support(p)}
}
#[derive(serde::Deserialize)]
struct Part {
    group: String,
    color: [u8; 3],
    vertices: Vec<[f32; 6]>,
    triangles: Vec<[u32; 3]>,
}
#[derive(Debug)]
pub(super) struct Props {
    pub case: Handle<Node>,
    lid: Handle<Node>,
    pub launcher: Handle<Node>,
    drum: Handle<Node>,
}
impl Props {
    pub fn muzzle(&self, g: &Graph) -> Vector3<f32> {
        g[self.launcher].global_transform()
            .transform_point(&LAUNCHER_MUZZLE.into()).coords
    }

    fn new(g: &mut Graph, surfaces: &[(String, Surface)]) -> Self {
        let mut mesh = |name: &str, group: &str| -> Handle<Node> {
            MeshBuilder::new(BaseBuilder::new().with_name(name).with_visibility(false))
                .with_surfaces(
                    surfaces
                        .iter()
                        .filter(|s| s.0 == group)
                        .map(|s| s.1.clone())
                        .collect(),
                )
                .build(g)
                .to_base()
        };
        let case = mesh("Hilde medical case", "case");
        let lid = mesh("Hilde hinged medical case lid", "lid");
        let launcher = mesh("Hilde medical grenade launcher", "launcher");
        let drum = mesh("Hilde rotary capsule drum", "drum");
        g.link_nodes(lid, case);
        g.link_nodes(drum, launcher);
        g[lid].set_visible(true);
        g[drum].set_visible(true);
        Self {
            case,
            lid,
            launcher,
            drum,
        }
    }
    pub fn update(
        &self,
        g: &mut Graph,
        p: Pose,
        position: Vector3<f32>,
        rotation: UnitQuaternion<f32>,
        scale: f32,
        show: bool,
    ) {
        g[self.case].set_visible(show && p.motion == Motion::HildeSkill);
        g[self.launcher].set_visible(show && p.motion == Motion::HildeUltimate);
        for node in [self.case, self.launcher] {
            g[node]
                .local_transform_mut()
                .set_position(position)
                .set_rotation(rotation)
                .set_scale(Vector3::repeat(scale));
        }
        let opening = ((p.age - 0.65) / 0.65).clamp(0.0, 1.0)
            * (1.0 - ((p.age - 3.0) / 0.30).clamp(0.0, 1.0));
        g[self.lid]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, -0.40, 0.10))
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::x_axis(),
                opening * 1.8,
            ));
        g[self.drum]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, 0.14, 0.19))
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::z_axis(),
                if (h::EQUIP_SECONDS..h::SOURCE_LOOP_END).contains(&p.age) {
                    (p.age - h::EQUIP_SECONDS) * 5.0 * std::f32::consts::TAU / 6.0
                } else {
                    0.0
                },
            ));
        g.update_hierarchical_data_for_descendants(self.case);
        g.update_hierarchical_data_for_descendants(self.launcher);
    }
}
#[derive(Debug)]
pub(super) struct Effects {
    pub world: [Props; PLAYERS],
    pub view: Props,
    pub fields: [Handle<Node>; PLAYERS],
    field_materials: [MaterialResource; PLAYERS],
}
impl Effects {
    pub fn new(g: &mut Graph) -> Self {
        let parts: Vec<Part> = serde_json::from_str(include_str!(
            "../../assets/effects/hilde-medical-props.json"
        ))
        .expect("medical prop meshes");
        let surfaces: Vec<_> = parts
            .into_iter()
            .map(|p| {
                let pivot = match p.group.as_str() {
                    "lid" => Vector3::new(0.0, -0.40, 0.10),
                    "drum" => Vector3::new(0.0, 0.14, 0.19),
                    _ => Vector3::zeros(),
                };
                let verts: Vec<_> = p
                    .vertices
                    .iter()
                    .map(|v| {
                        StaticVertex::from_pos_uv(
                            Vector3::new(v[0], v[1], v[2]) - pivot,
                            Vector2::zeros(),
                        )
                    })
                    .collect();
                let mut data = SurfaceData::new(
                    VertexBuffer::new(verts.len(), verts).unwrap(),
                    TriangleBuffer::new(p.triangles.into_iter().map(TriangleDefinition).collect()),
                );
                data.calculate_normals().unwrap();
                (
                    p.group,
                    SurfaceBuilder::new(SurfaceResource::new_embedded(data))
                        .with_material(material(p.color))
                        .build(),
                )
            })
            .collect();
        let world = std::array::from_fn(|_| Props::new(g, &surfaces));
        let view = Props::new(g, &surfaces);
        character_shadows::disable_casting(g, view.case);
        character_shadows::disable_casting(g, view.launcher);
        let mut verts = Vec::new();
        let mut triangles = Vec::new();
        // One shared, batched mesh per field. The shader raises and billboards
        // each cross; its encoded index keeps the particles evenly distributed.
        for i in 0..HEAL_PARTICLES {
            let angle = i as f32 * 2.399963;
            let radius = ((i as f32 + 0.5) / HEAL_PARTICLES as f32).sqrt()
                * (h::HEAL_RADIUS - 0.4);
            let base = verts.len() as u32;
            for (u, v) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
                verts.push(StaticVertex::from_pos_uv(
                    // Include the full animated height in the CPU culling bounds.
                    Vector3::new(angle.cos() * radius + (u - 0.5) * 0.7,
                        v * 2.3, angle.sin() * radius),
                    Vector2::new(i as f32 * 4.0 + u, v),
                ));
            }
            triangles.extend([
                TriangleDefinition([base, base + 1, base + 2]),
                TriangleDefinition([base, base + 2, base + 3]),
            ]);
        }
        let geometry = SurfaceResource::new_embedded(SurfaceData::new(
            VertexBuffer::new(verts.len(), verts).unwrap(),
            TriangleBuffer::new(triangles),
        ));
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("hilde_healing.shader"))
                .expect("Hilde rising healing crosses shader"),
        );
        let field_materials = std::array::from_fn(|_| {
            MaterialResource::new(Material::from_shader(shader.clone()))
        });
        let fields = std::array::from_fn(|i| {
            MeshBuilder::new(
                BaseBuilder::new()
                    .with_name("Hilde rising healing crosses")
                    .with_cast_shadows(false)
                    .with_visibility(false),
            )
            .with_render_path(RenderPath::Forward)
            .with_surfaces(vec![SurfaceBuilder::new(geometry.clone())
                .with_material(field_materials[i].clone()).build()])
            .build(g)
            .to_base()
        });
        Self { world, view, fields, field_materials }
    }
    pub fn field(&self, g: &mut Graph, i: usize, a: &Actor, show: bool) {
        // Use the skill clock, since the source animation ends before healing.
        let age = a.combat.hilde.elapsed - h::HEAL_RELEASE;
        let visible = show && a.alive() && a.combat.hilde.phase == h::Phase::Skill
            && (0.0..h::HEAL_SECONDS).contains(&age);
        g[self.fields[i]].set_visible(visible);
        if visible {
            g[self.fields[i]].local_transform_mut().set_position(vec(a.pos));
            let mut material = self.field_materials[i].data_ref();
            material.set_property("age", age);
            material.set_property("opacity", (age / 0.15).min(1.0)
                * ((h::HEAL_SECONDS - age) / 0.25).min(1.0));
            g.update_hierarchical_data_for_descendants(self.fields[i]);
        }
    }
}
