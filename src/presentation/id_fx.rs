//! Depth-tested pod orbits, lantern discharge, eclipse and Nata's luminous ray.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    core::math::TriangleDefinition,
    material::shader::{Shader, ShaderResource},
    scene::{
        light::point::PointLightBuilder,
        mesh::{
            buffer::{TriangleBuffer, VertexBuffer},
            vertex::StaticVertex,
            RenderPath,
        },
    },
};
use std::f32::consts::TAU;
use trickcal_shooting_balddagu::{
    id_skill as id,
    motion::{Motion, Pose},
};

fn energy(white: bool) -> MaterialResource {
    static SHADER: std::sync::OnceLock<ShaderResource> = std::sync::OnceLock::new();
    let shader = SHADER.get_or_init(|| {
        ShaderResource::new_embedded(
            Shader::from_string(include_str!("id_energy.shader")).expect("Id energy shader"),
        )
    });
    let mut m = Material::from_shader(shader.clone());
    m.set_property("white", if white { 1.0 } else { 0.0 });
    MaterialResource::new(m)
}
fn ribbon(ring: bool) -> SurfaceResource {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    let count = if ring { 128 } else { 1 };
    for i in 0..=count {
        let t = i as f32 / count as f32;
        for side in 0..2 {
            let p = if ring {
                let a = t * TAU;
                let r = 1.0 + (side as f32 - 0.5) * 0.07;
                Vector3::new(a.cos() * r, 0.0, a.sin() * r)
            } else {
                Vector3::new(side as f32 * 2.0 - 1.0, 0.0, t)
            };
            vertices.push(StaticVertex::from_pos_uv(p, Vector2::new(side as f32, t)));
        }
        if i < count {
            let b = (i * 2) as u32;
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
#[derive(Debug)]
pub(super) struct Pod {
    pub root: Handle<Node>,
    pub cover: Handle<Node>,
    pub orbits: [Handle<Node>; 3],
    lamp_rings: [Handle<Node>; 2],
    lights: [Handle<Node>; 2],
    lamps: [Handle<Node>; 2],
    center: Handle<Node>,
    pub nata: Handle<Node>,
    white: MaterialResource,
}
impl Pod {
    pub fn new(g: &mut Graph, model: Handle<Node>) -> Self {
        let find = |name: &str| {
            g.traverse_handle_iter(model)
                .find(|h| g[*h].name() == name)
                .expect(name)
        };
        let center = find("Hips");
        let nata = find("NataRoot");
        let lamps = [find("IdPod.Lantern.L"), find("IdPod.Lantern.R")];
        let root: Handle<Node> =
            PivotBuilder::new(BaseBuilder::new().with_name("Id pod orbital effects"))
                .build(g)
                .to_base();
        g.link_nodes(root, model);
        g[root]
            .local_transform_mut()
            .set_scale(Vector3::repeat(100.0));
        let yellow = energy(false);
        let white = energy(true);
        let ring = ribbon(true);
        let orbits = std::array::from_fn(|i| {
            let n = node(g, &format!("Id orbit {i}"), ring.clone(), yellow.clone());
            g.link_nodes(n, root);
            n
        });
        let lamp_rings = std::array::from_fn(|i| {
            let n = node(
                g,
                &format!("Id lantern corona {i}"),
                ring.clone(),
                white.clone(),
            );
            g.link_nodes(n, root);
            n
        });
        let lights = std::array::from_fn(|_| {
            let n = PointLightBuilder::new(
                BaseLightBuilder::new(
                    BaseBuilder::new()
                        .with_name("Id lantern radiance")
                        .with_visibility(false)
                        .with_cast_shadows(false),
                )
                .with_color(Color::WHITE)
                .with_intensity(2.5),
            )
            .with_radius(2.8)
            .build(g)
            .to_base();
            g.link_nodes(n, root);
            n
        });
        let cover = MeshBuilder::new(
            BaseBuilder::new()
                .with_name("Id black circular canopy")
                .with_cast_shadows(false)
                .with_visibility(false),
        )
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(
            // Fyrox 1.0.1 couples the polar/azimuth increments: use equal counts.
            SurfaceData::make_sphere(48, 48, 1.0, &Matrix4::identity()),
        ))
        .with_material(material([1, 1, 2]))
        .build()])
        .build(g)
        .to_base();
        g.link_nodes(cover, root);
        Self {
            root,
            cover,
            orbits,
            lamp_rings,
            lights,
            lamps,
            center,
            nata,
            white,
        }
    }
    pub fn update(&self, g: &mut Graph, pose: Pose, visible: bool, orbit_age: f32) {
        g[self.root].set_visible(visible && pose.motion != Motion::Death);
        g.update_hierarchical_data_for_descendants(self.root);
        let inverse = g[self.root]
            .global_transform()
            .try_inverse()
            .unwrap_or_default();
        let center = inverse
            .transform_point(&(g[self.center].global_position() + Vector3::y() * 0.32).into())
            .coords;
        let q = pose.motion == Motion::IdUltimate;
        let e = pose.motion == Motion::IdSkill;
        let t = pose.age;
        let open = if q {
            id::smooth((t - id::ULTIMATE_RELEASE) / 0.22) * (1.0 - id::smooth((t - 4.1) / 0.4))
        } else {
            0.0
        };
        for (i, n) in self.orbits.iter().enumerate() {
            let size = if i == 0 {
                1.10 + if e {
                    0.25 * (t * 0.8).sin().max(0.0)
                } else {
                    0.0
                }
            } else {
                1.72 * open
            };
            g[*n].set_visible(i == 0 || open > 0.001);
            g[*n]
                .local_transform_mut()
                .set_position(center)
                .set_scale(Vector3::new(size, 1.0, size * 0.80))
                .set_rotation(UnitQuaternion::from_euler_angles(
                    if i == 0 {
                        0.14
                    } else if i == 1 {
                        0.72
                    } else {
                        -0.72
                    },
                    orbit_age * 0.19 + i as f32 * 1.8,
                    if i == 0 { 0.08 } else { 0.30 },
                ));
        }
        let cover = if q {
            id::smooth((t - 0.10) / 0.18) * (1.0 - id::smooth((t - 1.98) / 0.16))
        } else {
            0.0
        };
        g[self.cover].set_visible(cover > 0.001);
        g[self.cover]
            .local_transform_mut()
            .set_position(center + Vector3::new(0.0, 0.02, 0.30))
            .set_scale(Vector3::new(0.96, 1.13, 1.30) * cover.max(0.001));
        let pulse = if pose.motion == Motion::IdPulse {
            (1.0 - t / id::PULSE_SECONDS).max(0.0)
        } else {
            0.0
        };
        self.white.data_ref().set_property("opacity", pulse * 0.9);
        for i in 0..2 {
            let pos = inverse
                .transform_point(&g[self.lamps[i]].global_position().into())
                .coords
                + Vector3::z() * 0.07;
            g[self.lamp_rings[i]].set_visible(pulse > 0.0);
            g[self.lamp_rings[i]]
                .local_transform_mut()
                .set_position(pos)
                .set_scale(Vector3::repeat(0.15 + 0.18 * (1.0 - pulse)))
                .set_rotation(UnitQuaternion::from_euler_angles(
                    std::f32::consts::FRAC_PI_2,
                    0.0,
                    0.0,
                ));
            g[self.lights[i]].set_visible(visible && pulse > 0.0);
            g[self.lights[i]].local_transform_mut().set_position(pos);
        }
    }
    pub fn muzzle(&self, g: &Graph) -> Vector3<f32> {
        // Nata's imported head origin sits at the lower edge of its face.
        g[self.nata].global_position() + Vector3::y() * 0.21
    }
}
#[derive(Debug)]
pub(super) struct Effects {
    beams: [Handle<Node>; PLAYERS],
    rings: [[Handle<Node>; 2]; PLAYERS],
    pulse_materials: [MaterialResource; PLAYERS],
}
impl Effects {
    pub fn new(g: &mut Graph) -> Self {
        let beam = ribbon(false);
        let ring = ribbon(true);
        let yellow = energy(false);
        let beams =
            std::array::from_fn(|_| node(g, "Nata luminous laser", beam.clone(), yellow.clone()));
        let pulse_materials = std::array::from_fn(|_| energy(true));
        let rings = std::array::from_fn(|i| {
            std::array::from_fn(|_| {
                node(
                    g,
                    "Id pale expanding pulse",
                    ring.clone(),
                    pulse_materials[i].clone(),
                )
            })
        });
        Self {
            beams,
            rings,
            pulse_materials,
        }
    }
    pub fn update(
        &self,
        g: &mut Graph,
        index: usize,
        actor: &Actor,
        pose: Pose,
        muzzle: Vector3<f32>,
        camera: Handle<Node>,
        first: bool,
        enabled: bool,
    ) {
        let show = enabled && actor.character == 3 && actor.alive();
        let beam = self.beams[index];
        let age = pose.shot_age;
        g[beam].set_visible(show && age < 0.15);
        if show && age < 0.15 {
            let start = if first {
                g[camera]
                    .global_transform()
                    .transform_point(&Vector3::new(-1.35, -0.08, 0.35).into())
                    .coords
            } else {
                muzzle
            };
            let end = vec(actor.shot_end);
            let delta = end - start;
            let direction = delta.try_normalize(0.0001).unwrap_or(Vector3::z());
            let view = (g[camera].global_position() - (start + end) * 0.5)
                .try_normalize(0.0001)
                .unwrap_or(Vector3::y());
            let across = view
                .cross(&direction)
                .try_normalize(0.0001)
                .unwrap_or(Vector3::x());
            let up = direction.cross(&across);
            let rotation =
                UnitQuaternion::from_matrix(&fyrox::core::algebra::Matrix3::from_columns(&[
                    across, up, direction,
                ]));
            g[beam]
                .local_transform_mut()
                .set_position(start)
                .set_rotation(rotation)
                .set_scale(Vector3::new(0.18, 1.0, delta.norm()));
        }
        let age = id::PULSE_COOLDOWN - actor.combat.id.pulse_cooldown;
        let active = show && actor.combat.id.pulse_cooldown > 0.0 && age < id::PULSE_SECONDS;
        self.pulse_materials[index].data_ref().set_property(
            "opacity",
            (1.0 - age / id::PULSE_SECONDS).clamp(0.0, 1.0) * 0.55,
        );
        for (j, n) in self.rings[index].iter().enumerate() {
            let t = (age - j as f32 * 0.09).max(0.0) / id::PULSE_SECONDS;
            let radius = 0.25 + id::PULSE_RADIUS * t;
            g[*n].set_visible(active && age >= j as f32 * 0.09);
            g[*n]
                .local_transform_mut()
                .set_position(vec(actor.pos) + Vector3::y() * (0.35 + j as f32 * 0.38))
                .set_scale(Vector3::new(radius, 1.0, radius));
        }
    }
}
