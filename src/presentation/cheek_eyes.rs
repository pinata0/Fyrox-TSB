//! Selection-private illustrated eyes, blended on the original skinned surface.
use super::*;
use fyrox::{
    asset::manager::ResourceManager,
    core::futures::{executor::block_on, future::join_all},
    graphics::gpu_program::{ShaderProperty, ShaderPropertyKind},
    material::shader::{Shader, ShaderResource, ShaderResourceKind},
    scene::mesh::Mesh,
};
use trickcal_shooting_balddagu::cheeks::{eye_profile, EyeReaction};

static MANAGER: std::sync::OnceLock<ResourceManager> = std::sync::OnceLock::new();

pub(super) fn load(manager: &ResourceManager) {
    let _ = MANAGER.set(manager.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_restores_only_our_private_face_material() {
        let mut g = Graph::new();
        let original = MaterialResource::new(Material::standard());
        let source_shader = original.data_ref().shader().clone();
        let original_resources = source_shader.data_ref().definition.resources.len();
        let eye_shader = shader(&source_shader);
        assert_eq!(
            source_shader.data_ref().definition.resources.len(),
            original_resources
        );
        assert_eq!(
            eye_shader.data_ref().definition.resources.len(),
            original_resources + 2
        );
        let variants = std::array::from_fn(|_| {
            MaterialResource::new(Material::from_shader(eye_shader.clone()))
        });
        let surface = SurfaceBuilder::new(SurfaceResource::new_embedded(SurfaceData::make_cube(
            Matrix4::identity(),
        )))
        .with_material(variants[0].clone())
        .build();
        let node = MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![surface])
            .build(&mut g)
            .to_base();
        let eyes = Eyes {
            root: node,
            surfaces: vec![(node, 0, variants.clone(), original.clone())],
            applied: Default::default(),
        };
        let idle = trickcal_shooting_balddagu::cheeks::Interaction::default().eyes(0);
        eyes.apply(&mut g, idle);
        assert_eq!(
            g[node].cast::<Mesh>().unwrap().surfaces()[0].material(),
            &original
        );
        let blink = MaterialResource::new(Material::standard());
        g[node].cast_mut::<Mesh>().unwrap().surfaces_mut()[0].set_material(blink.clone());
        eyes.apply(&mut g, idle);
        assert_eq!(
            g[node].cast::<Mesh>().unwrap().surfaces()[0].material(),
            &blink
        );
        g[node].cast_mut::<Mesh>().unwrap().surfaces_mut()[0].set_material(variants[1].clone());
        eyes.apply(&mut g, idle);
        assert_eq!(
            g[node].cast::<Mesh>().unwrap().surfaces()[0].material(),
            &original
        );
    }
}

fn shader(original: &ShaderResource) -> ShaderResource {
    let mut shader = Shader {
        definition: original.data_ref().definition.clone(),
        ..Default::default()
    };
    shader.definition.name.push_str(" / Cheek illustrated eyes");
    // Clone the original sampler type and allocate unused texture bindings.
    let sampler = shader
        .definition
        .resources
        .iter()
        .find(|r| r.name.as_str() == "diffuseTexture")
        .unwrap()
        .clone();
    let binding = shader
        .definition
        .resources
        .iter()
        .map(|r| r.binding)
        .max()
        .unwrap()
        + 1;
    for (i, name) in ["cheekHoldTexture", "cheekReleaseTexture"]
        .into_iter()
        .enumerate()
    {
        let mut resource = sampler.clone();
        resource.name = name.into();
        resource.binding = binding + i;
        shader.definition.resources.push(resource);
    }
    let properties = &mut shader
        .definition
        .resources
        .iter_mut()
        .find(|r| r.name.as_str() == "properties")
        .unwrap()
        .kind;
    let ShaderResourceKind::PropertyGroup(properties) = properties else {
        panic!("face properties")
    };
    for name in ["cheekOpacity", "cheekReleaseMix"] {
        properties.push(ShaderProperty::new(
            name,
            ShaderPropertyKind::Float { value: 0.0 },
        ));
    }
    let pass = shader
        .definition
        .passes
        .iter_mut()
        .find(|p| p.name == "GBuffer")
        .unwrap();
    let needle = "texture(diffuseTexture, tc)";
    assert!(
        pass.fragment_shader.contains(needle),
        "pinned Fyrox face shader"
    );
    *pass.fragment_shader = pass.fragment_shader.replace(needle,
        "mix(texture(diffuseTexture, tc), mix(texture(cheekHoldTexture, tc), texture(cheekReleaseTexture, tc), properties.cheekReleaseMix), properties.cheekOpacity)");
    ShaderResource::new_embedded(shader)
}

#[derive(Debug)]
pub(super) struct Eyes {
    root: Handle<Node>,
    surfaces: Vec<(Handle<Node>, usize, [MaterialResource; 2], MaterialResource)>,
    applied: std::cell::Cell<Option<EyeReaction>>,
}

impl Eyes {
    pub fn new(g: &Graph, root: Handle<Node>, id: u8) -> Self {
        let profile = eye_profile(id);
        // Only the currently previewed character loads these large atlases.
        let manager = MANAGER.get().expect("loaded character resources");
        let mut paths = Vec::new();
        for (phase, released) in [("hold", false), ("release", true)] {
            for side in 0..=usize::from(profile.mirrored(released)) {
                paths.push(format!(
                    "cheeks/{}-{phase}-{}.png",
                    profile.slug,
                    if side == 0 { "L" } else { "R" }
                ));
            }
        }
        let textures: Vec<_> =
            block_on(join_all(paths.iter().map(|path| {
                manager.request::<Texture>(characters::asset_path(path))
            })))
            .into_iter()
            .zip(&paths)
            .map(|(r, p)| r.unwrap_or_else(|e| panic!("{p}: {e:?}")))
            .collect();
        let release = 1 + usize::from(profile.mirrored(false));
        let mut surfaces = Vec::new();
        for node in g.traverse_handle_iter(root) {
            if g[node].name() != format!("{}_SkinnedMesh", profile.slug) {
                continue;
            }
            let Some(mesh) = g[node].cast::<Mesh>() else {
                continue;
            };
            // Surface zero is the face atlas. Other materials (weapons, sleeves)
            // keep their own texture and material bindings.
            let original = mesh.surfaces()[0].material().data_ref();
            let shader = shader(original.shader());
            let variants = std::array::from_fn(|side| {
                let mut mat = Material::from_shader(shader.clone());
                for (name, binding) in original.bindings() {
                    mat.bind(name.clone(), binding.clone());
                }
                mat.bind(
                    "cheekHoldTexture",
                    textures[side * usize::from(profile.mirrored(false))].clone(),
                );
                mat.bind(
                    "cheekReleaseTexture",
                    textures[release + side * usize::from(profile.mirrored(true))].clone(),
                );
                MaterialResource::new(mat)
            });
            surfaces.push((node, 0, variants, mesh.surfaces()[0].material().clone()));
        }
        assert!(!surfaces.is_empty(), "{} cheek face surface", profile.slug);
        Self {
            root,
            surfaces,
            applied: Default::default(),
        }
    }

    pub fn apply(&self, g: &mut Graph, reaction: EyeReaction) {
        self.applied.set(Some(reaction));
        if reaction.opacity <= 0.0 {
            // A cancellation can arrive between pose evaluation and drawing.
            // Restore our own material immediately, while leaving normal blink
            // variants selected by the ordinary expression system untouched.
            for (node, index, variants, original) in &self.surfaces {
                let mesh = g[*node].cast_mut::<Mesh>().unwrap();
                if variants.contains(mesh.surfaces()[*index].material()) {
                    mesh.surfaces_mut()[*index].set_material(original.clone());
                }
            }
            return;
        }
        for (node, index, variants, _) in &self.surfaces {
            let material = &variants[usize::from(reaction.mirror)];
            {
                let mut mat = material.data_ref();
                mat.set_property("cheekOpacity", reaction.opacity);
                mat.set_property("cheekReleaseMix", reaction.release_mix);
            }
            g[*node].cast_mut::<Mesh>().unwrap().surfaces_mut()[*index]
                .set_material(material.clone());
        }
        let nodes: Vec<_> = g.traverse_handle_iter(self.root).collect();
        for node in nodes {
            let Some(mesh) = g[node].cast_mut::<Mesh>() else {
                continue;
            };
            for shape in mesh.blend_shapes_mut() {
                let side = usize::from(shape.name.ends_with("_R"));
                if shape.name.contains("eyeBlink_") {
                    shape.weight = 100.0 * reaction.closure[side];
                } else if shape.name.contains("browDown_") {
                    shape.weight = 100.0 * (-reaction.brow[side]).max(0.0);
                } else if shape.name.contains("browUp_") {
                    shape.weight = 100.0 * reaction.brow[side].max(0.0);
                } else if shape.name.contains("eyeSquint_") {
                    shape.weight = 0.0;
                }
            }
        }
    }

    pub fn review(&self, g: &Graph) {
        // Focus/cursor events can cancel input after post_update but before
        // capture. Inspect the pose actually applied to this draw, not the next
        // input state; that state is restored by the following post_update.
        let Some(reaction) = self.applied.get() else {
            return;
        };
        if reaction.opacity <= 0.0 {
            for (node, index, _, _) in &self.surfaces {
                let material = g[*node].cast::<Mesh>().unwrap().surfaces()[*index]
                    .material()
                    .data_ref();
                assert!(
                    !material
                        .shader()
                        .data_ref()
                        .definition
                        .name
                        .contains("Cheek illustrated eyes"),
                    "idle restores the ordinary face material"
                );
            }
            return;
        }
        for (node, index, variants, _) in &self.surfaces {
            let mesh = g[*node].cast::<Mesh>().unwrap();
            assert_eq!(
                mesh.surfaces()[*index].material(),
                &variants[usize::from(reaction.mirror)]
            );
            for shape in mesh
                .blend_shapes()
                .iter()
                .filter(|s| s.name.contains("eyeBlink_"))
            {
                let side = usize::from(shape.name.ends_with("_R"));
                assert!((shape.weight - 100.0 * reaction.closure[side]).abs() < 0.001);
            }
        }
    }
}
