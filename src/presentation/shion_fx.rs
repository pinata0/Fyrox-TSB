//! Source-clocked purple charge, explosive lobes, shock rings and outward sparks.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    material::shader::{Shader, ShaderResource},
    scene::{light::point::PointLightBuilder, mesh::RenderPath},
};
use std::f32::consts::TAU;
use trickcal_shooting_balddagu::shion_skill::{self as shion, Phase as Action};

const PARTS: usize = 22;
#[derive(Debug)]
pub(super) struct Effects {
    pub nodes: [[Handle<Node>; PARTS]; PLAYERS],
    materials: [MaterialResource; PLAYERS],
    lights: [Handle<Node>; PLAYERS],
}
impl Effects {
    pub fn new(g: &mut Graph) -> Self {
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("shion_energy.shader")).expect("Shion energy shader"),
        );
        let materials =
            std::array::from_fn(|_| MaterialResource::new(Material::from_shader(shader.clone())));
        let sphere = SurfaceResource::new_embedded(SurfaceData::make_sphere(
            24,
            32,
            1.0,
            &Matrix4::identity(),
        ));
        let nodes = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                MeshBuilder::new(
                    BaseBuilder::new()
                        .with_name(format!("Shion purple blast {i}/{j}"))
                        .with_visibility(false)
                        .with_cast_shadows(false),
                )
                .with_render_path(RenderPath::Forward)
                .with_surfaces(vec![SurfaceBuilder::new(sphere.clone())
                    .with_material(materials[i].clone())
                    .build()])
                .build(g)
                .to_base()
            })
        });
        let lights = std::array::from_fn(|_| {
            PointLightBuilder::new(
                BaseLightBuilder::new(
                    BaseBuilder::new()
                        .with_name("Shion purple flash")
                        .with_visibility(false)
                        .with_cast_shadows(false),
                )
                .with_color(Color::from_rgba(182, 46, 255, 255))
                .with_intensity(3.5),
            )
            .with_radius(10.0)
            .build(g)
            .to_base()
        });
        Self {
            nodes,
            materials,
            lights,
        }
    }
    pub fn update(&self, g: &mut Graph, index: usize, a: &Actor, enabled: bool) {
        let state = a.combat.shion;
        let ultimate = enabled && a.character == 14 && a.alive() && state.phase == Action::Ultimate;
        let age = state.elapsed - shion::ULTIMATE_RELEASE;
        let blast = ultimate && (0.0..shion::BLAST_SECONDS).contains(&age);
        let charge = ultimate && age < 0.0;
        self.materials[index]
            .data_ref()
            .set_property("age", state.elapsed);
        self.materials[index].data_ref().set_property(
            "opacity",
            if blast {
                (1.0 - age / shion::BLAST_SECONDS).powf(1.2)
            } else {
                0.7
            },
        );
        let forward = vec(direction(a.yaw, 0.0));
        let origin = if blast {
            vec(a.shot_end)
        } else {
            vec(a.eye()) + forward * 1.65
        };
        for (j, node) in self.nodes[index].iter().enumerate() {
            g[*node].set_visible(blast || charge && j == 0);
            if !blast && !charge {
                continue;
            }
            let progress = (age / shion::BLAST_SECONDS).clamp(0.0, 1.0);
            let radius = shion::BLAST_RADIUS * (1.0 - (-age.max(0.0) * 22.0).exp()).max(0.05);
            let angle = j as f32 * 2.399963;
            let outward =
                Vector3::new(angle.cos(), (j as f32 * 1.7).sin() * 0.65, angle.sin()).normalize();
            let (offset, scale, rotation) = if charge {
                (
                    Vector3::zeros(),
                    Vector3::repeat(
                        0.12 + 0.40 * (state.elapsed / shion::ULTIMATE_RELEASE).powi(2),
                    ),
                    UnitQuaternion::identity(),
                )
            } else if j == 0 {
                (
                    Vector3::zeros(),
                    Vector3::repeat(radius * 0.8),
                    UnitQuaternion::identity(),
                )
            } else if j <= 6 {
                (
                    outward * radius * 0.5,
                    Vector3::repeat(radius * (0.38 + 0.05 * (j % 3) as f32)),
                    UnitQuaternion::from_euler_angles(angle, age * 2.0, 0.0),
                )
            } else if j <= 8 {
                (
                    Vector3::zeros(),
                    Vector3::new(radius * 1.12, 0.035, radius * 1.12),
                    UnitQuaternion::from_euler_angles(
                        if j == 7 { 0.0 } else { TAU * 0.19 },
                        0.0,
                        if j == 7 { 0.0 } else { 0.8 },
                    ),
                )
            } else {
                (
                    outward * radius * (0.65 + progress * 0.40),
                    Vector3::new(0.035, 0.035, 0.35 * (1.0 - progress) + 0.05),
                    UnitQuaternion::face_towards(&outward, &Vector3::y()),
                )
            };
            g[*node]
                .local_transform_mut()
                .set_position(origin + offset)
                .set_scale(scale)
                .set_rotation(rotation);
        }
        let light = self.lights[index];
        g[light].set_visible(blast && age < 0.28);
        g[light].local_transform_mut().set_position(origin);
    }
}
