//! Cake portions shared by first person, observers, dropped debris and replays.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    core::math::TriangleDefinition,
    scene::mesh::{
        buffer::{TriangleBuffer, VertexBuffer},
        vertex::StaticVertex,
    },
};
use std::collections::HashMap;

fn slice_layer(g: &mut Graph, index: usize, y: f32, height: f32, color: [u8; 3]) -> Handle<Node> {
    let z0 = -0.11 + index as f32 * 0.055;
    let z1 = z0 + 0.055;
    let width = |z: f32| 0.105 - (z + 0.11) * 0.42;
    let (w0, w1) = (width(z0), width(z1));
    let (lo, hi) = (y - height * 0.5, y + height * 0.5);
    let points = [
        [-w0, lo, z0],
        [w0, lo, z0],
        [w1, lo, z1],
        [-w1, lo, z1],
        [-w0, hi, z0],
        [w0, hi, z0],
        [w1, hi, z1],
        [-w1, hi, z1],
    ]
    .map(Vector3::from);
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for face in [
        [0, 1, 2, 3],
        [4, 7, 6, 5],
        [0, 4, 5, 1],
        [3, 2, 6, 7],
        [0, 3, 7, 4],
        [1, 5, 6, 2],
    ] {
        let normal = (points[face[1]] - points[face[0]])
            .cross(&(points[face[2]] - points[face[0]]))
            .normalize();
        let base = vertices.len() as u32;
        for i in face {
            vertices.push(StaticVertex::from_pos_uv_normal(
                points[i],
                Vector2::zeros(),
                normal,
            ));
        }
        triangles.extend([
            TriangleDefinition([base, base + 1, base + 2]),
            TriangleDefinition([base, base + 2, base + 3]),
        ]);
    }
    let data = SurfaceResource::new_embedded(SurfaceData::new(
        VertexBuffer::new(vertices.len(), vertices).unwrap(),
        TriangleBuffer::new(triangles),
    ));
    MeshBuilder::new(BaseBuilder::new().with_name("Cake slice layer"))
        .with_surfaces(vec![SurfaceBuilder::new(data)
            .with_material(material(color))
            .build()])
        .build(g)
        .to_base()
}

#[derive(Debug)]
pub(super) struct Cake {
    pub root: Handle<Node>,
    portions: [Handle<Node>; 4],
}
impl Cake {
    pub fn new(g: &mut Graph, shapes: &Shapes) -> Self {
        let root: Handle<Node> =
            PivotBuilder::new(BaseBuilder::new().with_name("Erpin strawberry cake"))
                .build(g)
                .to_base();
        let portions = std::array::from_fn(|i| {
            let part: Handle<Node> =
                PivotBuilder::new(BaseBuilder::new().with_name(format!("Cake portion {i}")))
                    .build(g)
                    .to_base();
            g.link_nodes(part, root);
            // Four tapered strips form a cake slice. Each bite removes the front strip.
            let z = (i as f32 - 1.5) * 0.055;
            for (y, height, color) in [
                (0.025, 0.05, [242, 200, 128]),
                (0.055, 0.018, [232, 110, 139]),
                (0.082, 0.035, [248, 221, 161]),
                (0.109, 0.019, [255, 246, 226]),
            ] {
                let h = slice_layer(g, i, y, height, color);
                g.link_nodes(h, part);
            }
            if i == 1 {
                let berry = shapes.solid(
                    g,
                    "Strawberry",
                    V3::new(0.0, 0.15, z),
                    V3::new(0.036, 0.043, 0.032),
                    [224, 55, 83],
                    1,
                );
                g.link_nodes(berry, part);
                let leaf = shapes.solid(
                    g,
                    "Strawberry leaf",
                    V3::new(0.0, 0.186, z),
                    V3::new(0.045, 0.008, 0.03),
                    [97, 160, 92],
                    0,
                );
                g.link_nodes(leaf, part);
            }
            part
        });
        g[root].set_visible(false);
        Self { root, portions }
    }
    pub fn show(&self, g: &mut Graph, remaining: u8) {
        g[self.root].set_visible(remaining > 0);
        for (i, &part) in self.portions.iter().enumerate() {
            g[part].set_visible(i < remaining as usize);
        }
    }
}
pub(super) struct View {
    pub slot: usize,
    pub first_person: bool,
    pub muzzle: Vector3<f32>,
    pub replay: bool,
    pub dt: f32,
}

fn ultimate_rings(g: &mut Graph) -> [Handle<Node>; PLAYERS] {
    const SEGMENTS: u32 = 96;
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for i in 0..SEGMENTS {
        let angle = i as f32 * std::f32::consts::TAU / SEGMENTS as f32;
        for radius in [0.94, 1.0] {
            vertices.push(StaticVertex::from_pos_uv_normal(
                Vector3::new(angle.cos() * radius, 0.0, angle.sin() * radius),
                Vector2::zeros(),
                Vector3::y(),
            ));
        }
        let n = i * 2;
        let next = ((i + 1) % SEGMENTS) * 2;
        triangles.extend([
            TriangleDefinition([n, next, n + 1]),
            TriangleDefinition([n + 1, next, next + 1]),
        ]);
    }
    let data = SurfaceResource::new_embedded(SurfaceData::new(
        VertexBuffer::new(vertices.len(), vertices).unwrap(),
        TriangleBuffer::new(triangles),
    ));
    let orange = material([255, 138, 36]);
    std::array::from_fn(|_| {
        MeshBuilder::new(
            BaseBuilder::new()
                .with_name("Erpin ultimate orange ring")
                .with_visibility(false)
                .with_cast_shadows(false),
        )
        .with_surfaces(vec![SurfaceBuilder::new(data.clone())
            .with_material(orange.clone())
            .build()])
        .build(g)
        .to_base()
    })
}

#[derive(Debug)]
pub(super) struct Effects {
    pub view: Cake,
    ultimate_rings: [Handle<Node>; PLAYERS],
    world: Vec<Cake>,
    dropped: HashMap<u32, Cake>,
    bolts: HashMap<u32, (Vector3<f32>, f32)>,
    sweat: Vec<Handle<Node>>,
    dizzy: Vec<[Handle<Node>; 3]>,
    key: Option<(u16, trickcal_shooting_balddagu::level::MapId, bool)>,
}
impl Effects {
    pub fn new(g: &mut Graph, shapes: &Shapes) -> Self {
        Self {
            view: Cake::new(g, shapes),
            ultimate_rings: ultimate_rings(g),
            world: (0..PLAYERS).map(|_| Cake::new(g, shapes)).collect(),
            dropped: HashMap::new(),
            bolts: HashMap::new(),
            sweat: (0..PLAYERS)
                .map(|_| {
                    let node = shapes.mesh(
                        g,
                        "Erpin flustered sweat",
                        V3::default(),
                        V3::new(0.045, 0.10, 0.035),
                        material([111, 219, 255]),
                        1,
                    );
                    g[node].set_cast_shadows(false);
                    g[node].set_visible(false);
                    node
                })
                .collect(),
            dizzy: (0..PLAYERS)
                .map(|_| {
                    std::array::from_fn(|_| {
                        let node = shapes.mesh(
                            g,
                            "Erpin dizzy star",
                            V3::default(),
                            V3::new(0.08, 0.08, 0.025),
                            material([255, 224, 44]),
                            0,
                        );
                        g[node].set_cast_shadows(false);
                        g[node].set_visible(false);
                        node
                    })
                })
                .collect(),
            key: None,
        }
    }
    pub fn update(
        &mut self,
        g: &mut Graph,
        w: &World,
        visuals: &[characters::Visual],
        projectile_nodes: &[Handle<Node>],
        view: View,
    ) {
        let View {
            slot: view_slot,
            first_person,
            muzzle: view_muzzle,
            replay,
            dt,
        } = view;
        let key = (w.round, w.map_id, replay);
        if self.key != Some(key) {
            for (_, cake) in self.dropped.drain() {
                g.remove_node(cake.root);
            }
            self.bolts.clear();
            self.key = Some(key);
        }
        for (i, (cake, visual)) in self.world.iter().zip(visuals).enumerate() {
            let pose = visual.pose();
            let actor = &w.actors[i];
            let ring = self.ultimate_rings[i];
            let show_ring = actor.active && actor.character == 2 && !actor.burrowed()
                && actor.combat.ultimate_flash > 0.0;
            g[ring].set_visible(show_ring);
            if show_ring {
                let radius = 3.0 + (w.tick as f32 * 0.12).sin().abs() * 4.0;
                g[ring]
                    .local_transform_mut()
                    .set_position(vec(actor.pos + V3::new(0.0, 0.04, 0.0)))
                    .set_scale(Vector3::new(radius, 1.0, radius));
            }
            let show = actor.character == 2 && actor.alive() && !(i == view_slot && first_person);
            let worry = trickcal_shooting_balddagu::erpin_skill::flustered(pose.motion, pose.age);
            let head = visual.head_position(g);
            let rotation = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), actor.yaw);
            g[self.sweat[i]].set_visible(show && worry > 0.2);
            g[self.sweat[i]]
                .local_transform_mut()
                .set_position(
                    head + rotation
                        * Vector3::new(0.32, 0.07 - 0.06 * (pose.age * 2.0).fract(), 0.23),
                )
                .set_rotation(rotation)
                .set_scale(Vector3::new(0.045, 0.10, 0.035) * worry);
            for (j, &node) in self.dizzy[i].iter().enumerate() {
                let angle = pose.age * 4.0 + j as f32 * std::f32::consts::TAU / 3.0;
                g[node].set_visible(
                    show && pose.motion == trickcal_shooting_balddagu::motion::Motion::ErpinRecover,
                );
                g[node]
                    .local_transform_mut()
                    .set_position(
                        head + Vector3::new(
                            angle.cos() * 0.40,
                            0.64 + 0.05 * (angle * 2.0).sin(),
                            angle.sin() * 0.40,
                        ),
                    )
                    .set_rotation(
                        rotation
                            * UnitQuaternion::from_euler_angles(
                                0.0,
                                0.0,
                                std::f32::consts::FRAC_PI_4,
                            ),
                    );
            }
            let eating = visual.pose().motion
                == trickcal_shooting_balddagu::motion::Motion::ErpinEat
                && w.actors[i].alive();
            let state = trickcal_shooting_balddagu::erpin::State {
                eating,
                elapsed: visual.pose().age,
                ..Default::default()
            };
            cake.show(
                g,
                if eating && !(i == view_slot && first_person) {
                    state.remaining()
                } else {
                    0
                },
            );
            if eating {
                let point = visual.palm_contact(g, true);
                g[cake.root]
                    .local_transform_mut()
                    .set_position(point)
                    .set_scale(Vector3::repeat(0.85))
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        w.actors[i].yaw,
                    ));
            }
        }
        let live: std::collections::HashSet<_> = w.projectiles.iter().map(|p| p.id).collect();
        self.dropped.retain(|id, cake| {
            if live.contains(id) {
                true
            } else {
                g.remove_node(cake.root);
                false
            }
        });
        self.bolts.retain(|id, _| live.contains(id));
        for (p, &node) in w.projectiles.iter().zip(projectile_nodes) {
            if let Some(remaining) = p.kind.cake_remaining() {
                let cake = self
                    .dropped
                    .entry(p.id)
                    .or_insert_with(|| Cake::new(g, &Shapes::new()));
                g[node].set_visible(false);
                cake.show(g, remaining);
                g[cake.root]
                    .local_transform_mut()
                    .set_position(vec(p.pos))
                    .set_rotation(UnitQuaternion::from_euler_angles(
                        0.0,
                        p.id as f32 * 0.7,
                        0.0,
                    ))
                    .set_scale(Vector3::repeat(0.85));
            } else if w.actors[p.owner].character == 2
                // Basic bolts use Models::update_muzzle_launches so their world
                // transforms (including the glow) are updated before rendering.
                && p.kind == ProjectileKind::MagicBomb
            {
                let entry = self.bolts.entry(p.id).or_insert_with(|| {
                    (
                        (if p.owner == view_slot && first_person {
                            view_muzzle
                        } else {
                            visuals[p.owner].staff_muzzle(g)
                        }) - vec(p.pos),
                        0.0,
                    )
                });
                let blend = (1.0 - entry.1 / 0.12).max(0.0);
                g[node]
                    .local_transform_mut()
                    .set_position(vec(p.pos) + entry.0 * blend);
                // Show the first frame at the gem before advancing toward the
                // authoritative trajectory; otherwise a slow frame skips it.
                entry.1 += dt;
            }
        }
    }
}
