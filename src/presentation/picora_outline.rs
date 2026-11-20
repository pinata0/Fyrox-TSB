//! An occluded skinned silhouette follows the actual character, including the skirt.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    material::shader::{Shader, ShaderResource},
    scene::mesh::{Mesh, RenderPath},
};
#[derive(Debug, Default)]
pub(super) struct Outline {
    nodes: Vec<(Handle<Node>, Handle<Node>)>,
    materials: Vec<MaterialResource>,
}
impl Outline {
    pub fn update(&mut self, g: &mut Graph, root: Handle<Node>, active: bool, time: f32) {
        if active && self.nodes.is_empty() {
            static SHADER: std::sync::OnceLock<ShaderResource> = std::sync::OnceLock::new();
            let shader = SHADER.get_or_init(|| {
                ShaderResource::new_embedded(
                    Shader::from_string(include_str!("picora_outline.shader"))
                        .expect("Picora rainbow outline shader"),
                )
            });
            let materials: [MaterialResource; 2] = std::array::from_fn(|j| {
                let mut m = Material::from_shader(shader.clone());
                m.set_property("width", if j == 0 { 0.009 } else { 0.004 });
                m.set_property("opacity", if j == 0 { 0.30 } else { 0.85 });
                MaterialResource::new(m)
            });
            self.materials = materials.to_vec();
            let meshes: Vec<_> = g
                .traverse_handle_iter(root)
                .filter(|n| {
                    g[*n].cast::<Mesh>().is_some()
                        && !g[*n].name().contains("ViewSleeve")
                        && !g[*n].name().contains("outline")
                })
                .collect();
            for source in meshes {
                let original = g[source].cast::<Mesh>().unwrap().surfaces().to_vec();
                for material in &materials {
                    let mut surfaces = original.clone();
                    for surface in &mut surfaces {
                        surface.set_material(material.clone());
                    }
                    let node = MeshBuilder::new(
                        BaseBuilder::new()
                            .with_name("Picora rainbow violet glow outline")
                            .with_cast_shadows(false),
                    )
                    .with_render_path(RenderPath::Forward)
                    .with_surfaces(surfaces)
                    .build(g)
                    .to_base();
                    g.link_nodes(node, source);
                    self.nodes.push((source, node));
                }
            }
        }
        for material in &self.materials {
            material.data_ref().set_property("time", time);
        }
        for (source, node) in &self.nodes {
            let show = active && g[*source].visibility();
            g[*node].set_visible(show);
        }
    }
}
