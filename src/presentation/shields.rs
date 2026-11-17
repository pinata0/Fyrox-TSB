//! Shared transparent, depth-tested grid spheres for skill and racial shields.
use super::*;
use fyrox::{
    material::shader::{Shader, ShaderResource},
    scene::mesh::RenderPath,
};

pub(super) fn create(g: &mut Graph) -> [Handle<Node>; PLAYERS] {
    let shader = Shader::from_string(include_str!("shields.shader"))
        .expect("embedded shield shader must parse");
    let material =
        MaterialResource::new(Material::from_shader(ShaderResource::new_embedded(shader)));
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
            .with_material(material.clone())
            .build()])
        .build(g)
        .to_base()
    })
}

pub(super) fn update(g: &mut Graph, node: Handle<Node>, actor: &Actor) {
    g[node].set_visibility(actor.alive() && actor.shield_health() > 0.0);
    let scale = actor.body_scale();
    g[node]
        .local_transform_mut()
        .set_position(vec(actor.pos + V3::new(0.0, 1.05 * scale, 0.0)))
        .set_scale(Vector3::repeat(1.3 * scale));
}
