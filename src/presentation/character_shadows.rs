//! Hide the first-person world model from color/depth passes while retaining its real shadow.
use super::*;
use fyrox::{
    material::shader::{Shader, ShaderResource},
    renderer::is_shadow_pass,
    scene::mesh::Mesh,
};
use std::collections::HashMap;

fn shadow_shader(original: &ShaderResource) -> ShaderResource {
    // A fresh resource keeps the shared FBX shader and GPU cache intact.
    let mut shader = Shader {
        definition: original.data_ref().definition.clone(),
        ..Default::default()
    };
    shader
        .definition
        .name
        .push_str(" / First-person world shadow");
    shader.definition.disabled_passes.extend(
        shader
            .definition
            .passes
            .iter()
            .filter(|pass| !is_shadow_pass(&pass.name))
            .map(|pass| pass.name.clone()),
    );
    shader
        .definition
        .passes
        .retain(|pass| is_shadow_pass(&pass.name));
    ShaderResource::new_embedded(shader)
}

#[derive(Debug)]
struct MeshMaterials {
    node: Handle<Node>,
    surfaces: Vec<(MaterialResource, MaterialResource)>,
}

#[derive(Debug, Default)]
pub(super) struct ShadowOnly {
    enabled: bool,
    meshes: Vec<MeshMaterials>,
}

impl ShadowOnly {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, g: &mut Graph, roots: [Handle<Node>; 2], enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        if self.meshes.is_empty() {
            let mut materials = HashMap::new();
            let mut shaders = HashMap::new();
            for root in roots {
                for node in g.traverse_handle_iter(root) {
                    let Some(mesh) = g[node].cast::<Mesh>() else {
                        continue;
                    };
                    let surfaces = mesh
                        .surfaces()
                        .iter()
                        .map(|surface| {
                            let original = surface.material();
                            let shadow = materials.entry(original.key()).or_insert_with(|| {
                                let material = original.data_ref();
                                let shader = shaders
                                    .entry(material.shader().key())
                                    .or_insert_with(|| shadow_shader(material.shader()));
                                let mut shadow = Material::from_shader(shader.clone());
                                // Preserve alpha cutouts, textures and all original shadow parameters.
                                for (name, binding) in material.bindings() {
                                    shadow.bind(name.clone(), binding.clone());
                                }
                                MaterialResource::new(shadow)
                            });
                            (original.clone(), shadow.clone())
                        })
                        .collect();
                    self.meshes.push(MeshMaterials { node, surfaces });
                }
            }
        }
        for mesh in &self.meshes {
            for (surface, (original, shadow)) in g[mesh.node]
                .cast_mut::<Mesh>()
                .unwrap()
                .surfaces_mut()
                .iter_mut()
                .zip(&mesh.surfaces)
            {
                surface.set_material(if enabled { shadow } else { original }.clone());
            }
        }
        self.enabled = enabled;
    }
}

/// Camera-mounted props must not add a second, detached weapon shadow.
pub(super) fn disable_casting(g: &mut Graph, root: Handle<Node>) {
    let nodes: Vec<_> = g.traverse_handle_iter(root).collect();
    for node in nodes {
        g[node].set_cast_shadows(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_person_keeps_original_shadow_passes_and_restores_world_materials() {
        let mut g = Graph::new();
        let shapes = Shapes::new();
        let original = material([90, 120, 150]);
        let body = shapes.mesh(
            &mut g,
            "body",
            V3::default(),
            V3::new(1.0, 1.0, 1.0),
            original.clone(),
            0,
        );
        let weapon = shapes.mesh(
            &mut g,
            "weapon",
            V3::default(),
            V3::new(1.0, 1.0, 1.0),
            original.clone(),
            0,
        );
        let other = shapes.mesh(
            &mut g,
            "other player",
            V3::default(),
            V3::new(1.0, 1.0, 1.0),
            original.clone(),
            0,
        );
        let mut shadow_only = ShadowOnly::default();
        shadow_only.set_enabled(&mut g, [body, weapon], true);
        let replacement = g[body].cast::<Mesh>().unwrap().surfaces()[0]
            .material()
            .clone();
        assert_eq!(
            replacement.key(),
            g[weapon].cast::<Mesh>().unwrap().surfaces()[0]
                .material()
                .key()
        );
        assert_eq!(
            original.key(),
            g[other].cast::<Mesh>().unwrap().surfaces()[0]
                .material()
                .key()
        );
        let original_data = original.data_ref();
        let shadow_data = replacement.data_ref();
        let original_shader = original_data.shader().data_ref();
        let shadow_shader = shadow_data.shader().data_ref();
        assert!(original_shader
            .definition
            .passes
            .iter()
            .any(|p| p.name == "GBuffer"));
        assert_eq!(shadow_shader.definition.passes.len(), 3);
        for pass in original_shader
            .definition
            .passes
            .iter()
            .filter(|pass| !is_shadow_pass(&pass.name))
        {
            assert!(shadow_shader
                .definition
                .disabled_passes
                .contains(&pass.name));
        }
        for pass in &shadow_shader.definition.passes {
            assert!(is_shadow_pass(&pass.name));
            let original_pass = original_shader
                .definition
                .passes
                .iter()
                .find(|p| p.name == pass.name)
                .unwrap();
            assert_eq!(pass.vertex_shader, original_pass.vertex_shader);
            assert_eq!(pass.fragment_shader, original_pass.fragment_shader);
            assert_eq!(pass.draw_parameters, original_pass.draw_parameters);
        }
        drop(shadow_shader);
        drop(shadow_data);
        drop(original_shader);
        drop(original_data);
        for node in [body, weapon] {
            assert!(g[node].visibility() && g[node].cast_shadows());
        }
        shadow_only.set_enabled(&mut g, [body, weapon], false);
        for node in [body, weapon] {
            assert_eq!(
                original.key(),
                g[node].cast::<Mesh>().unwrap().surfaces()[0]
                    .material()
                    .key()
            );
        }
        shadow_only.set_enabled(&mut g, [body, weapon], true);
        assert_eq!(
            replacement.key(),
            g[body].cast::<Mesh>().unwrap().surfaces()[0]
                .material()
                .key()
        );
        disable_casting(&mut g, weapon);
        assert!(!g[weapon].cast_shadows());
        assert!(g[body].cast_shadows());
    }
}
