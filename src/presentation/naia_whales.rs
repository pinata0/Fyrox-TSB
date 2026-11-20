//! The gun's own mesh becomes a whale; the ultimate school uses that same morph.
use crate::presentation::SceneState;
use super::*;
use fyrox::resource::model::{ModelResource, ModelResourceExtension};
use trickcal_shooting_balddagu::{
    motion::{Motion, Pose},
    naia_skill,
};

pub const HANDS: [&str; 2] = ["Naia cradle.R", "Naia cradle.L"];
pub const CRADLE_HALF_WIDTH: f32 = 0.18;
pub const CRADLE_DEPTH: f32 = 0.04;
// Center of the dolphin gun's front opening, in imported weapon space.
pub const MUZZLE: Vector3<f32> = Vector3::new(-0.001, 0.123, 0.489);
pub const E_VIEW_SCALE: f32 = 1.65;

pub fn view_scale(pose: Pose) -> f32 {
    if pose.motion.is_naia_skill() && pose.motion != Motion::NaiaUltimate {
        E_VIEW_SCALE
    } else {
        1.0
    }
}

/// Palm sockets use -X (right) / +X (left) along the fingers and +Z through the palm.
/// Both sets of fingers point forward/up, with the palms facing each other.
pub fn cradle_rotation(left: bool) -> UnitQuaternion<f32> {
    let x = Vector3::new(0.0, 0.35, 1.0).normalize() * if left { 1.0 } else { -1.0 };
    let z = Vector3::new(if left { -1.0 } else { 1.0 }, 0.0, 0.0);
    let y = z.cross(&x);
    UnitQuaternion::from_matrix(&fyrox::core::algebra::Matrix3::from_columns(&[x, y, z]))
}

pub fn morph(g: &mut Graph, root: Handle<Node>, amount: f32) {
    let nodes: Vec<_> = g.traverse_handle_iter(root).collect();
    for n in nodes {
        if let Some(mesh) = g[n].cast_mut::<fyrox::scene::mesh::Mesh>() {
            for shape in mesh.blend_shapes_mut() {
                if shape.name.ends_with("swimmingWhale") {
                    shape.weight = amount * 100.0;
                }
            }
        }
    }
}

pub fn sockets(g: &mut Graph, root: Handle<Node>) {
    for name in HANDS {
        let h: Handle<Node> = PivotBuilder::new(BaseBuilder::new().with_name(name))
            .build(g)
            .to_base();
        g.link_nodes(h, root);
    }
}

pub fn update_weapon(g: &mut Graph, root: Handle<Node>, pose: Pose, first: bool) {
    let amount = naia_skill::whale_form(pose.motion, pose.age);
    morph(g, root, amount);
    let body = g[root]
        .children()
        .iter()
        .copied()
        .find(|h| g[*h].name() == "Naia transforming gun")
        .unwrap();
    g[body]
        .local_transform_mut()
        .set_rotation(UnitQuaternion::identity());
    for side in 0..2 {
        let node = weapons::grip_node(g, root, 1, side == 1);
        // Q's empty-hand gesture still starts from its original held pose.
        let ultimate = pose.motion == Motion::NaiaUltimate;
        let width = if ultimate { 0.22 } else { CRADLE_HALF_WIDTH };
        g[node].local_transform_mut().set_position(Vector3::new(
            (if side == 0 { -width } else { width }) * amount,
            (if ultimate { 0.02 } else { 0.10 }) * amount,
            if ultimate { 0.0 } else { CRADLE_DEPTH * amount },
        ));
    }
    if first {
        let scale = view_scale(pose);
        g[root]
            .local_transform_mut()
            .set_position(Vector3::new(
                -0.44 * (1.0 - amount),
                -0.28 - amount * 0.10 - if scale > 1.0 { 0.28 } else { 0.0 },
                0.56 + amount * 0.29,
            ))
            .set_scale(Vector3::repeat((0.72 + amount * 0.25) * scale));
    } else {
        g[root]
            .local_transform_mut()
            .set_scale(Vector3::repeat(0.7 + amount * 0.6));
    }
}

#[derive(Debug)]
pub struct School {
    pub root: Handle<Node>,
    pub whales: [Handle<Node>; 6],
}
impl School {
    pub fn new(g: &mut Graph, parent: Handle<Node>, model: &ModelResource) -> Self {
        let root = PivotBuilder::new(
            BaseBuilder::new()
                .with_name("Naia summoned whale school")
                .with_visibility(false),
        )
        .build(g)
        .to_base();
        g.link_nodes(root, parent);
        let data = model.data_ref();
        let whales = std::array::from_fn(|_| {
            let (node, _) = ModelResource::instantiate_from(
                model.clone(),
                &data,
                data.get_scene().graph.get_root(),
                g,
                &mut |_, _| {},
            );
            g.link_nodes(node, root);
            morph(g, node, 1.0);
            node
        });
        Self { root, whales }
    }
    pub fn update(&self, g: &mut Graph, pose: Pose, alive: bool) {
        let time = pose.age;
        let visible = alive && pose.motion == Motion::NaiaUltimate && (0.45..3.85).contains(&time);
        g[self.root].set_visible(visible);
        if !visible {
            return;
        }
        let smooth = |v: f32| {
            let v = v.clamp(0.0, 1.0);
            v * v * (3.0 - 2.0 * v)
        };
        for (i, node) in self.whales.iter().enumerate() {
            let side = if i < 3 { -1.0 } else { 1.0 };
            let rank = i % 3;
            let path = naia_skill::whale_path(time, rank);
            let envelope =
                smooth((time - 0.45 - rank as f32 * 0.08) / 0.25) * smooth((3.85 - time) / 0.35);
            let lift = path[1].clamp(-0.4, 1.4) * 0.65;
            g[*node]
                .local_transform_mut()
                .set_position(Vector3::new(
                    side * (1.15 + rank as f32 * 0.80),
                    0.95 + lift,
                    1.35 + rank as f32 * 0.28,
                ))
                .set_rotation(UnitQuaternion::from_euler_angles(
                    -path[2].sin() * 0.75,
                    side * 0.28,
                    side * path[2].cos() * 0.22,
                ))
                .set_scale(Vector3::repeat(0.014 * envelope.max(0.001)));
        }
    }
}
