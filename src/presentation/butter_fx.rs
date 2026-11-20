//! Shared contact geometry for Butter's ordinary and detached giant slingshots.
use crate::presentation::SceneState;
use super::*;
use trickcal_shooting_balddagu::{butter_skill as b, motion::Pose};

pub(super) fn pouch(giant: bool, pull: f32) -> Vector3<f32> {
    Vector3::new(0.0, if giant { 0.265 } else { 0.18 }, -0.32 * pull)
}

pub(super) fn sockets(g: &mut Graph, shapes: &Shapes, root: Handle<Node>) {
    for name in ["Butter right draw", "Butter left draw"] {
        let n: Handle<Node> = PivotBuilder::new(BaseBuilder::new().with_name(name))
            .build(g)
            .to_base();
        g.link_nodes(n, root);
    }
    for (name, color, size, shape) in [("Butter yellow ball", [255, 220, 32], 0.075, 1)] {
        let n = shapes.solid(
            g,
            name,
            V3::default(),
            V3::new(size, size, size),
            color,
            shape,
        );
        g.link_nodes(n, root);
        g[n].set_visible(false);
    }
    let rock: Handle<Node> = MeshBuilder::new(
        BaseBuilder::new()
            .with_name("Butter loaded boulder")
            .with_visibility(false),
    )
    .with_surfaces(super::projectiles::boulder_surfaces())
    .build(g)
    .to_base();
    g[rock]
        .local_transform_mut()
        .set_scale(Vector3::repeat(0.20));
    g.link_nodes(rock, root);
}
pub(super) fn draw_node(g: &Graph, weapon: Handle<Node>, left: bool) -> Handle<Node> {
    let name = if left {
        "Butter left draw"
    } else {
        "Butter right draw"
    };
    g[weapon]
        .children()
        .iter()
        .copied()
        .find(|&n| g[n].name() == name)
        .unwrap()
}
pub(super) fn weapon(g: &mut Graph, weapon: Handle<Node>, pose: Pose, first: bool) {
    let t = pose.age;
    let giant = b::giant(pose.motion, t);
    let pull = b::pull(pose.motion, t);
    if first {
        let scale = 0.90 + 2.30 * b::growth(pose.motion, t);
        g[weapon]
            .local_transform_mut()
            .set_scale(Vector3::repeat(scale))
            .set_position(if giant {
                Vector3::new(0.0, -0.65 - 0.085 * scale, 1.90)
            } else {
                Vector3::new(-0.35, -0.30, 0.64)
            });
        g[weapon].set_visible(b::weapon_visible(pose.motion, t));
    }
    let pouch = pouch(giant, pull);
    let support = weapons::grip_node(g, weapon, 11, true);
    g[support].local_transform_mut().set_position(pouch);
    for left in [false, true] {
        let n = draw_node(g, weapon, left);
        g[n].local_transform_mut()
            .set_position(pouch + Vector3::new(if left { 0.065 } else { -0.065 }, -0.025, -0.025));
    }
    let nodes: Vec<_> = g.traverse_handle_iter(weapon).collect();
    for n in nodes {
        let name = g[n].name().to_owned();
        if name == "Butter yellow ball" || name == "Butter loaded boulder" {
            g[n].set_visible(
                b::loaded(pose.motion, t) && (name == "Butter loaded boulder") == giant,
            );
            g[n].local_transform_mut()
                .set_position(pouch + Vector3::new(0.0, 0.0, if giant { 0.065 } else { 0.035 }));
        }
        if let Some(mesh) = g[n].cast_mut::<fyrox::scene::mesh::Mesh>() {
            for shape in mesh.blend_shapes_mut() {
                if shape.name.ends_with("bandPull") {
                    shape.weight = if giant { 0.0 } else { pull * 100.0 };
                } else if shape.name.ends_with("giantBandLift") {
                    shape.weight = if giant { 100.0 } else { 0.0 };
                } else if shape.name.ends_with("giantBandPull") {
                    shape.weight = if giant { pull * 100.0 } else { 0.0 };
                }
            }
        }
    }
}
pub(super) fn is_giant(pose: Pose) -> bool {
    b::giant(pose.motion, pose.age)
}
