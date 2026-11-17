//! Cake portions shared by first person, observers, dropped debris and replays.
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
        g[root].set_visibility(false);
        Self { root, portions }
    }
    pub fn show(&self, g: &mut Graph, remaining: u8) {
        g[self.root].set_visibility(remaining > 0);
        for (i, &part) in self.portions.iter().enumerate() {
            g[part].set_visibility(i < remaining as usize);
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

#[derive(Debug)]
pub(super) struct Effects {
    pub view: Cake,
    world: Vec<Cake>,
    dropped: HashMap<u32, Cake>,
    bolts: HashMap<u32, (Vector3<f32>, f32)>,
    key: Option<(u16, trickcal_shooting_balddagu::level::MapId, bool)>,
}
impl Effects {
    pub fn new(g: &mut Graph, shapes: &Shapes) -> Self {
        Self {
            view: Cake::new(g, shapes),
            world: (0..PLAYERS).map(|_| Cake::new(g, shapes)).collect(),
            dropped: HashMap::new(),
            bolts: HashMap::new(),
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
                g[node].set_visibility(false);
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
            } else if w.actors[p.owner].character == 2 && p.kind == ProjectileKind::Bolt {
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
                entry.1 += dt;
                let blend = (1.0 - entry.1 / 0.12).max(0.0);
                g[node]
                    .local_transform_mut()
                    .set_position(vec(p.pos) + entry.0 * blend);
            }
        }
    }
}
