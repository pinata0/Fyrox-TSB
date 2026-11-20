//! Modeled scoped dart rifle and four-barrel belt-fed rotary gun.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    core::math::TriangleDefinition,
    scene::mesh::{
        buffer::{TriangleBuffer, VertexBuffer},
        surface::Surface,
        vertex::StaticVertex,
    },
};
use trickcal_shooting_balddagu::{
    mayo_skill as m,
    motion::{Motion, Pose},
};
pub(super) const SCALE: f32 = 0.90;
pub(super) const VIEW_SCALE: f32 = 0.65;
pub(super) fn active(p: Pose) -> bool {
    matches!(p.motion, Motion::MayoSkill | Motion::MayoUltimate)
}
fn phase(p: Pose) -> m::Phase {
    if p.motion == Motion::MayoUltimate {
        m::Phase::Ultimate
    } else if p.motion == Motion::MayoSkill {
        m::Phase::Skill
    } else {
        m::Phase::Off
    }
}
pub(super) fn shot_age(p: Pose) -> f32 {
    phase(p)
        .shots()
        .iter()
        .filter(|&&t| t <= p.age + 0.00001)
        .last()
        .map_or(100.0, |&t| (p.age - t).max(0.0))
}
pub(super) fn support(p: Pose) -> Vector3<f32> {
    Vector3::new(
        0.0,
        -0.015,
        if p.motion == Motion::MayoSkill {
            0.25
        } else {
            0.24
        },
    )
}
pub(super) fn rotation(p: Pose) -> UnitQuaternion<f32> {
    let h = m::hand_sample(p.motion, p.age, false);
    let raised = if p.motion == Motion::MayoUltimate {
        ((p.age - 0.1) / 0.25).clamp(0.0, 1.0) * (1.0 - ((p.age - 0.75) / 0.60).clamp(0.0, 1.0))
    } else {
        0.0
    };
    let lower = ((0.22 - p.age) / 0.22).clamp(0.0, 1.0)
        + ((p.age - phase(p).duration() + 0.28) / 0.28).clamp(0.0, 1.0);
    let recoil = (1.0 - shot_age(p) / 0.10).clamp(0.0, 1.0);
    UnitQuaternion::from_euler_angles(
        -raised * 0.85 + lower * 0.55 + (h[2] * 0.035).clamp(-0.08, 0.08) - recoil * 0.07,
        0.0,
        0.0,
    )
}
pub(super) fn view_rotation(p: Pose) -> UnitQuaternion<f32> {
    rotation(p)
}
pub(super) fn view_origin(p: Pose) -> Vector3<f32> {
    let h = m::hand_sample(p.motion, p.age, false);
    let lower = ((0.25 - p.age) / 0.25).clamp(0.0, 1.0)
        + ((p.age - phase(p).duration() + 0.3) / 0.3).clamp(0.0, 1.0);
    Vector3::new(
        -0.29,
        -0.23 - lower * 0.28 + (h[1] * 0.04).clamp(-0.04, 0.05),
        0.68 - (1.0 - shot_age(p) / 0.10).clamp(0.0, 1.0) * 0.025,
    )
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
    pub skill: Handle<Node>,
    pub ultimate: Handle<Node>,
    rotor: Handle<Node>,
    skill_muzzle: Handle<Node>,
    ultimate_muzzle: Handle<Node>,
    belt: [Handle<Node>; 8],
    flash: Handle<Node>,
}
impl Props {
    fn new(g: &mut Graph, surfaces: &[(String, Surface)]) -> Self {
        let mut mesh = |group: &str| -> Handle<Node> {
            MeshBuilder::new(
                BaseBuilder::new()
                    .with_name(format!("Mayo {group}"))
                    .with_visibility(false),
            )
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
        let skill = mesh("skill");
        let ultimate = mesh("ultimate");
        let rotor = mesh("rotor");
        let belt = std::array::from_fn(|i| mesh(&format!("belt{i}")));
        let flash = mesh("flash");
        for n in std::iter::once(rotor).chain(belt) {
            g.link_nodes(n, ultimate);
            g[n].set_visible(true);
        }
        let mut socket = |name: &str, parent, position| {
            let node: Handle<Node> = PivotBuilder::new(BaseBuilder::new().with_name(name))
                .build(g).to_base();
            g[node].local_transform_mut().set_position(position);
            g.link_nodes(node, parent);
            node
        };
        // Bore openings from model_mayo_guns.py; the rotary socket follows one
        // actual barrel, including its rotation around the hub at Y=0.17.
        let skill_muzzle = socket("Mayo rifle muzzle", skill, Vector3::new(0.0, 0.16, 0.8325));
        let barrel_offset = 0.09 * std::f32::consts::FRAC_1_SQRT_2;
        let ultimate_muzzle = socket("Mayo rotary muzzle", rotor,
            Vector3::new(barrel_offset, barrel_offset, 0.77));
        Self {
            skill,
            ultimate,
            rotor,
            skill_muzzle,
            ultimate_muzzle,
            belt,
            flash,
        }
    }
    pub fn muzzle(&self, g: &Graph, p: Pose) -> Vector3<f32> {
        g[if p.motion == Motion::MayoSkill { self.skill_muzzle } else { self.ultimate_muzzle }]
            .global_position()
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
        g[self.skill].set_visible(show && p.motion == Motion::MayoSkill);
        g[self.ultimate].set_visible(show && p.motion == Motion::MayoUltimate);
        for n in [self.skill, self.ultimate] {
            g[n].local_transform_mut()
                .set_position(position)
                .set_rotation(rotation)
                .set_scale(Vector3::repeat(scale));
        }
        let spinning = ((p.age - m::ULTIMATE_SHOTS[0]) / (4.0 / 30.0)).clamp(0.0, 8.0);
        g[self.rotor]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, 0.17, 0.0))
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::z_axis(),
                spinning * std::f32::consts::FRAC_PI_2,
            ));
        let fired = m::ULTIMATE_SHOTS.iter().filter(|&&t| t <= p.age).count();
        for (i, &n) in self.belt.iter().enumerate() {
            g[n].set_visible(i >= fired);
            g[n].local_transform_mut().set_position(Vector3::new(
                (p.age * 17.0).sin() * 0.006,
                0.0,
                0.0,
            ));
        }
        let pulse = (1.0 - shot_age(p) / 0.07).clamp(0.0, 1.0);
        g.update_hierarchical_data_for_descendants(self.skill);
        g.update_hierarchical_data_for_descendants(self.ultimate);
        let muzzle = self.muzzle(g, p);
        g[self.flash].set_visible(show && active(p) && pulse > 0.0);
        g[self.flash]
            .local_transform_mut()
            .set_position(muzzle)
            .set_rotation(rotation)
            .set_scale(Vector3::new(0.065, 0.065, 0.17) * scale * pulse);
    }
}
#[derive(Debug)]
pub(super) struct Effects {
    pub world: [Props; PLAYERS],
    pub view: Props,
}
impl Effects {
    pub fn new(g: &mut Graph) -> Self {
        let parts: Vec<Part> =
            serde_json::from_str(include_str!("../../assets/effects/mayo-guns.json"))
                .expect("Mayo modeled guns");
        let mut surfaces: Vec<_> = parts
            .into_iter()
            .map(|p| {
                let pivot = if p.group == "rotor" {
                    Vector3::new(0.0, 0.17, 0.0)
                } else {
                    Vector3::zeros()
                };
                let vertices: Vec<_> = p
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
                    VertexBuffer::new(vertices.len(), vertices).unwrap(),
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
        let vertices = vec![
            Vector3::z(),
            -Vector3::z(),
            Vector3::x(),
            Vector3::y(),
            -Vector3::x(),
            -Vector3::y(),
        ]
        .into_iter()
        .map(|p| StaticVertex::from_pos_uv(p, Vector2::zeros()))
        .collect::<Vec<_>>();
        let triangles = (0..4)
            .flat_map(|i| {
                [
                    TriangleDefinition([0, 2 + i, 2 + (i + 1) % 4]),
                    TriangleDefinition([1, 2 + (i + 1) % 4, 2 + i]),
                ]
            })
            .collect();
        let mut data = SurfaceData::new(
            VertexBuffer::new(vertices.len(), vertices).unwrap(),
            TriangleBuffer::new(triangles),
        );
        data.calculate_normals().unwrap();
        surfaces.push((
            "flash".into(),
            SurfaceBuilder::new(SurfaceResource::new_embedded(data))
                .with_material(material([203, 248, 110]))
                .build(),
        ));
        let world = std::array::from_fn(|_| Props::new(g, &surfaces));
        let view = Props::new(g, &surfaces);
        for n in [view.skill, view.ultimate, view.flash] {
            character_shadows::disable_casting(g, n);
        }
        Self { world, view }
    }
}
