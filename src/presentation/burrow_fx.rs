//! Ground-level soil ripples mark submerged spirits without exposing a body hitbox.
use crate::presentation::SceneState;
use super::*;
use trickcal_shooting_balddagu::race_skill::{BURROW_DURATION, EMERGE_HOLD};

pub(super) fn create(g: &mut Graph) -> [Handle<Node>; PLAYERS] {
    let soil = SurfaceResource::new_embedded(SurfaceData::make_torus(
        0.65,
        0.09,
        8,
        40,
        &Matrix4::identity(),
    ));
    let soil_material = material([173, 130, 77]);
    let shapes = Shapes::new();
    std::array::from_fn(|_| {
        let root = MeshBuilder::new(
            BaseBuilder::new()
                .with_name("Burrow soil ripple")
                .with_visibility(false)
                .with_cast_shadows(false),
        )
        .with_surfaces(vec![SurfaceBuilder::new(soil.clone())
            .with_material(soil_material.clone())
            .build()])
        .build(g)
        .to_base();
        for n in 0..7 {
            let angle = n as f32 * std::f32::consts::TAU / 7.0;
            let rock = shapes.solid(
                g,
                "Burrow loose soil",
                V3::new(angle.cos() * 0.83, 0.12, angle.sin() * 0.83),
                V3::new(0.14, 0.16, 0.12),
                [216, 180, 121],
                0,
            );
            g[rock].set_cast_shadows(false);
            g.link_nodes(rock, root);
        }
        root
    })
}

pub(super) fn update(g: &mut Graph, node: Handle<Node>, a: &Actor) {
    g[node].set_visible(a.alive() && a.burrowed());
    if !a.burrowed() {
        return;
    }
    let age = BURROW_DURATION - a.race_skill.remaining;
    let radius = 1.0 + (age * 12.0).sin() * 0.06 + a.race_skill.charge / EMERGE_HOLD * 0.4;
    g[node]
        .local_transform_mut()
        .set_position(vec(a.pos + V3::new(0.0, 0.05, 0.0)))
        .set_scale(Vector3::new(radius, 0.5, radius))
        .set_rotation(UnitQuaternion::from_axis_angle(
            &Vector3::y_axis(),
            age * 1.2,
        ));
}
