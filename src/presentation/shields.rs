//! Shared transparent, depth-tested grid spheres for skill and racial shields.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    material::shader::{Shader, ShaderResource},
    scene::mesh::RenderPath,
};

pub(super) fn create(g: &mut Graph) -> [Handle<Node>; PLAYERS] {
    let shader = Shader::from_string(include_str!("shields.shader"))
        .expect("embedded shield shader must parse");
    let shader = ShaderResource::new_embedded(shader);
    let surface =
        SurfaceResource::new_embedded(SurfaceData::make_sphere(48, 64, 1.0, &Matrix4::identity()));
    std::array::from_fn(|_| {
        MeshBuilder::new(
            BaseBuilder::new()
                .with_name("Transparent shield grid sphere")
                .with_visibility(false)
                .with_cast_shadows(false),
        )
        .with_render_path(RenderPath::Forward)
        .with_surfaces(vec![SurfaceBuilder::new(surface.clone())
            .with_material(MaterialResource::new(Material::from_shader(shader.clone())))
            .build()])
        .build(g)
        .to_base()
    })
}

pub(super) fn update(g: &mut Graph, node: Handle<Node>, actor: &Actor) {
    let mercury=actor.character==0 && actor.combat.mercury && actor.combat.guard>0.0;
    if let Some(mesh)=g[node].cast_mut::<fyrox::scene::mesh::Mesh>() {
        for surface in mesh.surfaces_mut() {surface.material().data_ref().set_property("mercury",if mercury{1.0_f32}else{0.0_f32});}
    }
    g[node].set_visible(
        actor.alive()
            && !actor.burrowed()
            && (actor.shield_health() > 0.0 || actor.combat.id.invulnerable > 0.0),
    );
    let scale = actor.body_scale();
    g[node]
        .local_transform_mut()
        .set_position(vec(actor.pos + V3::new(0.0, (1.05 + if actor.combat.vivi.phase==trickcal_shooting_balddagu::vivi_skill::Phase::Skill {trickcal_shooting_balddagu::vivi_skill::skill_lift(actor.combat.vivi.elapsed)}else{0.0}) * scale, 0.0)))
        .set_scale(Vector3::repeat(1.3 * scale));
}
