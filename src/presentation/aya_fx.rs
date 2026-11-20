//! Authored butterfly meshes with independent wings, blooming flowers, snowfall and ice.
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
use std::f32::consts::TAU;
use trickcal_shooting_balddagu::{aya_skill as aya, combat::Projectile};
#[derive(serde::Deserialize)]
struct Part {
    asset: String,
    group: usize,
    style: usize,
    vertices: Vec<[f32; 5]>,
    triangles: Vec<[u32; 3]>,
}
fn surface(
    vertices: Vec<StaticVertex>,
    triangles: Vec<TriangleDefinition>,
    material: MaterialResource,
) -> Surface {
    SurfaceBuilder::new(SurfaceResource::new_embedded(SurfaceData::new(
        VertexBuffer::new(vertices.len(), vertices).unwrap(),
        TriangleBuffer::new(triangles),
    )))
    .with_material(material)
    .build()
}
fn model(
    parts: &[Part],
    asset: &str,
    group: usize,
    materials: &[MaterialResource; 8],
) -> Vec<Surface> {
    (0..8)
        .filter_map(|style| {
            let mut vertices = Vec::new();
            let mut triangles = Vec::new();
            for p in parts
                .iter()
                .filter(|p| p.asset == asset && p.group == group && p.style == style)
            {
                let base = vertices.len() as u32;
                vertices.extend(p.vertices.iter().map(|v| {
                    StaticVertex::from_pos_uv(
                        Vector3::new(v[0], v[1], v[2]),
                        Vector2::new(v[3], v[4]),
                    )
                }));
                triangles.extend(
                    p.triangles
                        .iter()
                        .map(|t| TriangleDefinition(t.map(|i| i + base))),
                );
            }
            (!vertices.is_empty()).then(|| surface(vertices, triangles, materials[style].clone()))
        })
        .collect()
}
fn node(g: &mut Graph, name: &str, surfaces: Vec<Surface>) -> Handle<Node> {
    MeshBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_cast_shadows(false)
            .with_visibility(false),
    )
    .with_render_path(RenderPath::Forward)
    .with_surfaces(surfaces)
    .build(g)
    .to_base()
}
fn snow(material: MaterialResource) -> Surface {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for j in 0..220 {
        let angle = j as f32 * 2.399963;
        let r = ((j as f32 + 0.5) / 220.0).sqrt() * aya::ULTIMATE_RADIUS;
        let center = Vector3::new(
            angle.cos() * r,
            (j as f32 * 0.618034).fract() * 6.0,
            angle.sin() * r,
        );
        let size = 0.025 + (j % 5) as f32 * 0.007;
        for axis in [Vector3::x(), Vector3::z()] {
            let base = vertices.len() as u32;
            for (k, p) in [
                -axis - Vector3::y(),
                axis - Vector3::y(),
                axis + Vector3::y(),
                -axis + Vector3::y(),
            ]
            .into_iter()
            .enumerate()
            {
                vertices.push(StaticVertex::from_pos_uv(
                    center + p * size,
                    Vector2::new(
                        j as f32 * 2.0 + if k == 1 || k == 2 { 0.95 } else { 0.05 },
                        if k >= 2 { 0.95 } else { 0.05 },
                    ),
                ));
            }
            triangles.extend([
                TriangleDefinition([base, base + 1, base + 2]),
                TriangleDefinition([base, base + 2, base + 3]),
            ]);
        }
    }
    surface(vertices, triangles, material)
}
const FLOWERS: usize = 36;
#[derive(Debug)]
pub(super) struct Effects {
    butterflies: Vec<[Handle<Node>; 3]>,
    snow: [Handle<Node>; PLAYERS],
    flowers: [[Handle<Node>; FLOWERS]; PLAYERS],
    ice: [Handle<Node>; PLAYERS],
    snow_materials: [MaterialResource; PLAYERS],
}
impl Effects {
    pub fn new(g: &mut Graph) -> Self {
        let parts: Vec<Part> =
            serde_json::from_str(include_str!("../../assets/effects/aya-effects-mesh.json"))
                .expect("Aya modeled effects");
        let shader = ShaderResource::new_embedded(
            Shader::from_string(include_str!("aya_effect.shader")).expect("Aya pearl shader"),
        );
        let material = |style: usize| {
            let mut m = Material::from_shader(shader.clone());
            m.set_property("style", style as f32);
            MaterialResource::new(m)
        };
        let materials = std::array::from_fn(material);
        let butterfly: [Vec<Surface>; 3] =
            std::array::from_fn(|j| model(&parts, "butterfly", j, &materials));
        let butterflies = (0..MAX_PROJECTILES + PLAYERS * aya::BUTTERFLIES)
            .map(|_| std::array::from_fn(|j| node(g, "Aya snow butterfly", butterfly[j].clone())))
            .collect();
        let flower = model(&parts, "flower", 0, &materials);
        let crystal = model(&parts, "ice", 0, &materials);
        let flowers = std::array::from_fn(|_| {
            std::array::from_fn(|_| node(g, "Aya blooming white snow flower", flower.clone()))
        });
        let ice = std::array::from_fn(|_| node(g, "Frostbite ice prison", crystal.clone()));
        let snow_materials = std::array::from_fn(|_| material(7));
        let snow = std::array::from_fn(|j| {
            node(
                g,
                "Aya wide snowfall",
                vec![snow(snow_materials[j].clone())],
            )
        });
        Self {
            butterflies,
            snow,
            flowers,
            ice,
            snow_materials,
        }
    }
    fn butterfly(
        &self,
        g: &mut Graph,
        index: usize,
        pos: Vector3<f32>,
        rotation: UnitQuaternion<f32>,
        age: f32,
        scale: f32,
    ) {
        for (j, n) in self.butterflies[index].iter().enumerate() {
            let flap = if j == 2 {
                0.0
            } else {
                (age * 22.0 + index as f32 * 1.3).sin() * 0.72 * if j == 0 { -1.0 } else { 1.0 }
            };
            g[*n].set_visible(true);
            g[*n]
                .local_transform_mut()
                .set_position(pos)
                .set_rotation(rotation * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), flap))
                .set_scale(Vector3::repeat(scale));
        }
    }
    pub fn update(
        &self,
        g: &mut Graph,
        world: &World,
        actors: &[Actor; PLAYERS],
        view: usize,
        first: bool,
        enabled: bool,
    ) {
        for nodes in &self.butterflies {
            for n in nodes {
                g[*n].set_visible(false);
            }
        }
        let time = world.tick as f32 * DT;
        for (j, p) in world.projectiles.iter().enumerate() {
            if enabled && p.kind == ProjectileKind::Butterfly {
                let d = if p.velocity.length() > 0.01 {
                    p.velocity
                } else {
                    direction(actors[p.owner].yaw, actors[p.owner].pitch)
                };
                let rot = super::projectiles::facing(d);
                self.butterfly(
                    g,
                    j,
                    vec(p.pos) + Vector3::y() * (time * 15.0 + p.id as f32).sin() * 0.045,
                    rot,
                    time + p.id as f32,
                    0.90,
                );
            }
        }
        for (i, a) in actors.iter().enumerate() {
            let show = enabled && a.alive();
            g[self.ice[i]].set_visible(show && a.combat.frozen > 0.0 && !(first && i == view));
            g[self.ice[i]]
                .local_transform_mut()
                .set_position(vec(a.pos))
                .set_scale(Vector3::new(
                    a.body_scale(),
                    a.hit_height() / 1.8 * a.body_scale(),
                    a.body_scale(),
                ));
            if show
                && a.combat.aya.phase == aya::Phase::Skill
                && (0.5..aya::BUTTERFLY_RELEASE).contains(&a.combat.aya.elapsed)
            {
                for j in 0..aya::BUTTERFLIES {
                    let phase =
                        a.combat.aya.elapsed * 3.0 + j as f32 * TAU / aya::BUTTERFLIES as f32;
                    let pos = vec(a.pos)
                        + Vector3::new(
                            phase.cos() * 0.68,
                            1.0 + phase.sin() * 0.22,
                            phase.sin() * 0.68,
                        );
                    self.butterfly(
                        g,
                        MAX_PROJECTILES + i * aya::BUTTERFLIES + j,
                        pos,
                        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -phase),
                        time + j as f32,
                        0.55,
                    );
                }
            }
            // One independently timed garden per caster; newer casts replace old display slots.
            let field = world
                .projectiles
                .iter()
                .rev()
                .find(|p| p.owner == i && p.kind == ProjectileKind::SnowGarden)
                .filter(|_| enabled);
            self.garden(g, world, i, field);
        }
    }
    fn garden(&self, g: &mut Graph, world: &World, i: usize, field: Option<&Projectile>) {
        g[self.snow[i]].set_visible(field.is_some());
        for n in self.flowers[i] {
            g[n].set_visible(false);
        }
        let Some(p) = field else {
            return;
        };
        let fade = ((aya::SNOW_SECONDS - p.age) / 0.7).clamp(0.0, 1.0);
        self.snow_materials[i]
            .data_ref()
            .set_property("time", p.age);
        self.snow_materials[i]
            .data_ref()
            .set_property("opacity", fade * (p.age / 0.3).min(1.0));
        g[self.snow[i]]
            .local_transform_mut()
            .set_position(vec(p.pos));
        for (j, n) in self.flowers[i].iter().enumerate() {
            let angle = j as f32 * 2.399963;
            let r = ((j as f32 + 0.5) / FLOWERS as f32).sqrt() * (aya::ULTIMATE_RADIUS - 0.7);
            let mut pos = p.pos + V3::new(angle.cos() * r, 0.0, angle.sin() * r);
            pos.y = trickcal_shooting_balddagu::movement::support(&world.map, pos, p.pos.y + 0.35)
                + 0.025;
            let growth = aya::bloom(p.age - r * 0.018) * fade;
            g[*n].set_visible(growth > 0.005);
            g[*n]
                .local_transform_mut()
                .set_position(vec(pos))
                .set_scale(Vector3::repeat((0.85 + (j % 4) as f32 * 0.14) * growth))
                .set_rotation(UnitQuaternion::from_euler_angles(
                    0.0,
                    angle,
                    0.07 * (j as f32).sin(),
                ));
        }
    }
}
