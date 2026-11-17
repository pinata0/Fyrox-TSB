//! Ghost ripples shade the existing scenery, never the collision proxy geometry.
use super::*;
use fyrox::{
    core::algebra::Vector4,
    graphics::gpu_program::{ShaderProperty, ShaderPropertyKind},
    material::shader::{Shader, ShaderResource, ShaderResourceKind},
    scene::mesh::Mesh,
};
use std::collections::HashMap;
use trickcal_shooting_balddagu::{ghost::wall_contacts, movement::BODY_HEIGHT};

const LIFE: f32 = 0.85;

#[derive(Debug, Default, Clone, Copy)]
struct Ripple {
    center: V3,
    life: f32,
    age: f32,
}

#[derive(Debug, Default)]
pub(super) struct GhostWalls {
    materials: Vec<MaterialResource>,
    ripples: [Ripple; PLAYERS],
    round: u16,
    tick: u64,
}

fn ripple_shader(original: &ShaderResource) -> ShaderResource {
    // Retain the original textures, lighting and shadow passes. A fresh Shader
    // also gives this variant its own GPU cache entry.
    let mut shader = Shader {
        definition: original.data_ref().definition.clone(),
        ..Default::default()
    };
    shader.definition.name.push_str(" / Ghost surface ripples");
    let properties = shader
        .definition
        .resources
        .iter_mut()
        .find(|r| r.name.as_str() == "properties")
        .expect("scenery shader properties");
    let ShaderResourceKind::PropertyGroup(properties) = &mut properties.kind else {
        panic!("scenery properties must be a property group");
    };
    properties.push(ShaderProperty::new(
        "ghostCount",
        ShaderPropertyKind::Int { value: 0 },
    ));
    for name in ["ghostCenters", "ghostTimes"] {
        properties.push(ShaderProperty::new(
            name,
            ShaderPropertyKind::Vector4Array {
                value: vec![Vector4::zeros(); PLAYERS],
                max_len: PLAYERS,
            },
        ));
    }
    properties.push(ShaderProperty::new(
        "impactCount",
        ShaderPropertyKind::Int { value: 0 },
    ));
    for name in ["impactCenters", "impactNormals", "impactShapes"] {
        properties.push(ShaderProperty::new(
            name,
            ShaderPropertyKind::Vector4Array {
                value: vec![Vector4::zeros(); trickcal_shooting_balddagu::impact::MAX_IMPACTS],
                max_len: trickcal_shooting_balddagu::impact::MAX_IMPACTS,
            },
        ));
    }
    let pass = shader
        .definition
        .passes
        .iter_mut()
        .find(|p| p.name == "GBuffer")
        .expect("scenery GBuffer pass");
    // Fyrox is pinned in Cargo.toml. Its standard fragment shader ends in main;
    // extend its shaded outputs without changing positions, depth or silhouettes.
    let end = pass.fragment_shader.rfind('}').expect("fragment main");
    pass.fragment_shader
        .insert_str(end, include_str!("ghost_surface.glsl"));
    let end = pass.fragment_shader.rfind('}').expect("fragment main");
    pass.fragment_shader
        .insert_str(end, include_str!("impact_surface.glsl"));
    ShaderResource::new_embedded(shader)
}

impl GhostWalls {
    pub(super) fn set_impacts(
        &self,
        centers: Vec<Vector4<f32>>,
        normals: Vec<Vector4<f32>>,
        shapes: Vec<Vector4<f32>>,
    ) {
        for material in &self.materials {
            let mut material = material.data_ref();
            material.set_property("impactCount", centers.len() as i32);
            material.set_property("impactCenters", centers.clone());
            material.set_property("impactNormals", normals.clone());
            material.set_property("impactShapes", shapes.clone());
        }
    }
    pub(super) fn attach(&mut self, g: &mut Graph, root: Handle<Node>) {
        let handles: Vec<_> = g.traverse_handle_iter(root).collect();
        let mut materials = HashMap::new();
        let mut shaders = HashMap::new();
        for handle in handles {
            let Some(mesh) = g[handle].cast_mut::<Mesh>() else {
                continue;
            };
            for surface in mesh.surfaces_mut() {
                let original = surface.material();
                let replacement = materials.entry(original.key()).or_insert_with(|| {
                    let original = original.data_ref();
                    let shader = shaders
                        .entry(original.shader().key())
                        .or_insert_with(|| ripple_shader(original.shader()));
                    let mut material = Material::from_shader(shader.clone());
                    for (name, binding) in original.bindings() {
                        material.bind(name.clone(), binding.clone());
                    }
                    material.set_property("ghostCount", 0_i32);
                    MaterialResource::new(material)
                });
                surface.set_material(replacement.clone());
            }
        }
        // Only scene-local material copies receive the effect. Cached FBX assets
        // and any other scenes using their materials remain untouched.
        self.materials = materials.into_values().collect();
    }

    pub(super) fn update(&mut self, w: &World, dt: f32) {
        if self.round != w.round || w.tick < self.tick {
            self.ripples = [Ripple::default(); PLAYERS];
        }
        self.round = w.round;
        self.tick = w.tick;
        let mut centers = Vec::with_capacity(PLAYERS);
        let mut times = Vec::with_capacity(PLAYERS);
        for (ripple, actor) in self.ripples.iter_mut().zip(&w.actors) {
            ripple.life = (ripple.life - dt).max(0.0);
            ripple.age += dt;
            if !actor.alive()
                || character(actor.character).race
                    != trickcal_shooting_balddagu::roster::Race::Ghost
            {
                *ripple = Ripple::default();
                continue;
            }
            if actor.ghost_phase > 0.0 && !wall_contacts(&w.map, actor.pos).is_empty() {
                if ripple.life <= 0.0 {
                    ripple.age = 0.0;
                }
                // The body locates the effect; actual rasterized model fragments
                // determine its surface, even where an AABB encloses empty space.
                ripple.center = actor.pos + V3::new(0.0, BODY_HEIGHT * 0.55, 0.0);
                ripple.life = LIFE;
            }
            if ripple.life > 0.0 {
                let p = ripple.center;
                centers.push(Vector4::new(p.x, p.y, p.z, (ripple.life / 0.45).min(1.0)));
                times.push(Vector4::new(ripple.age, 0.0, 0.0, 0.0));
            }
        }
        for material in &self.materials {
            let mut material = material.data_ref();
            material.set_property("ghostCount", centers.len() as i32);
            material.set_property("ghostCenters", centers.clone());
            material.set_property("ghostTimes", times.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene() -> (Graph, Handle<Node>, MaterialResource, SurfaceResource) {
        let mut graph = Graph::new();
        let original = material([90, 120, 150]);
        let geometry = SurfaceResource::new_embedded(SurfaceData::make_sphere(
            16,
            24,
            1.0,
            &Matrix4::identity(),
        ));
        let root = MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(geometry.clone())
                .with_material(original.clone())
                .build()])
            .build(&mut graph)
            .to_base();
        (graph, root, original, geometry)
    }

    fn contact_world() -> World {
        let mut world = World {
            map: vec![Block {
                center: V3::new(0.0, 1.5, 0.0),
                size: V3::new(1.0, 3.0, 1.0),
                color: [150; 3],
                phaseable: true,
            }],
            ..Default::default()
        };
        world.actors[0].active = true;
        world.actors[0].hp = MAX_HP;
        world.actors[0].character = 14;
        world.actors[0].pos = V3::new(0.6, 0.0, 0.0);
        world.actors[0].ghost_phase = 1.0;
        world
    }

    #[test]
    fn curved_scenery_keeps_geometry_and_original_material_bindings() {
        let (mut graph, root, original, geometry) = scene();
        let count = graph.traverse_handle_iter(graph.get_root()).count();
        let mut effect = GhostWalls::default();
        effect.attach(&mut graph, root);
        effect.update(&contact_world(), DT);
        assert_eq!(graph.traverse_handle_iter(graph.get_root()).count(), count);
        let mesh = graph[root].cast::<Mesh>().unwrap();
        assert_eq!(mesh.surfaces().len(), 1);
        assert_eq!(mesh.surfaces()[0].data().key(), geometry.key());
        let replacement = mesh.surfaces()[0].material();
        assert_ne!(replacement.key(), original.key());
        let original = original.data_ref();
        let replacement = replacement.data_ref();
        assert_eq!(
            original
                .property_group_ref("properties")
                .unwrap()
                .property_ref("diffuseColor")
                .unwrap()
                .as_color(),
            replacement
                .property_group_ref("properties")
                .unwrap()
                .property_ref("diffuseColor")
                .unwrap()
                .as_color()
        );
        assert!(original
            .property_group_ref("properties")
            .unwrap()
            .property_ref("ghostCount")
            .is_none());
        let base = original.shader().data_ref();
        let variant = replacement.shader().data_ref();
        for (before, after) in base
            .definition
            .passes
            .iter()
            .zip(&variant.definition.passes)
        {
            assert_eq!(before.vertex_shader, after.vertex_shader);
            assert_eq!(before.draw_parameters, after.draw_parameters);
            if before.name != "GBuffer" {
                assert_eq!(before.fragment_shader, after.fragment_shader);
            }
        }
    }

    #[test]
    fn ripple_fades_and_clears_on_death_round_change_and_rewind() {
        let (mut graph, root, _, _) = scene();
        let mut effect = GhostWalls::default();
        effect.attach(&mut graph, root);
        let mut world = contact_world();
        effect.update(&world, DT);
        assert!(effect.ripples[0].life > 0.0);
        world.actors[0].ghost_phase = 0.0;
        effect.update(&world, LIFE / 2.0);
        assert!(effect.ripples[0].life > 0.0);
        effect.update(&world, LIFE);
        assert_eq!(effect.ripples[0].life, 0.0);
        for reset in 0..4 {
            world.actors[0].hp = MAX_HP;
            world.actors[0].character = 14;
            world.actors[0].ghost_phase = 1.0;
            world.tick = 100;
            effect.update(&world, DT);
            world.actors[0].ghost_phase = 0.0;
            match reset {
                0 => world.actors[0].hp = 0,
                1 => world.round += 1,
                2 => world.tick = 0,
                _ => world.actors[0].character = 0,
            }
            effect.update(&world, DT);
            assert_eq!(effect.ripples[0].life, 0.0);
            assert_eq!(
                effect.materials[0]
                    .data_ref()
                    .property_group_ref("properties")
                    .unwrap()
                    .property_ref("ghostCount")
                    .unwrap()
                    .as_int(),
                Some(0)
            );
        }
    }
}
