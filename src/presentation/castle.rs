//! Optimized Blender map scenery. Collision comes from the shared headless layout.
use fyrox::{
    asset::manager::ResourceManager,
    core::{algebra::Vector3, futures::executor::block_on},
    graph::prelude::*,
    resource::{
        model::{Model, ModelResourceExtension},
        texture::Texture,
    },
    scene::{
        base::BaseBuilder,
        light::{point::PointLightBuilder, BaseLightBuilder},
        mesh::Mesh,
        transform::TransformBuilder,
        Scene,
    },
};
use trickcal_shooting_balddagu::level::MapId;

pub fn attach(
    scene: &mut Scene,
    manager: &ResourceManager,
    map: MapId,
) -> Result<fyrox::core::pool::Handle<fyrox::scene::node::Node>, String> {
    let path = format!("assets/maps/{0}/{0}.fbx", map.slug());
    let model = block_on(manager.request::<Model>(&path))
        .map_err(|e| format!("Map scenery {path}: {e:?}"))?;
    let root = model.instantiate(scene);
    scene.graph[root]
        .local_transform_mut()
        .set_scale(Vector3::repeat(0.01));
    scene.graph[root].set_name(format!("{} / Blender scenery", map.slug()));
    let count = scene
        .graph
        .traverse_handle_iter(root)
        .filter(|h| scene.graph[*h].cast::<Mesh>().is_some())
        .count();
    if count == 0 {
        return Err(format!("{} FBX contains no meshes", map.slug()));
    }
    if map == MapId::FairyKingdomCastle {
        let mask =
            manager.request::<Texture>("assets/maps/fairy-kingdom-castle/window-emission.png");
        for handle in scene.graph.traverse_handle_iter(root) {
            if !scene.graph[handle].name().contains("Daylit opal windows") {
                continue;
            }
            if let Some(mesh) = scene.graph[handle].cast::<Mesh>() {
                for surface in mesh.surfaces() {
                    let mut material = surface.material().data_ref();
                    material.bind("emissionTexture", mask.clone());
                    material.set_property("emissionStrength", Vector3::new(0.28, 0.33, 0.3));
                }
            }
        }
    }
    if map == MapId::ElfCity {
        let emission = manager.request::<Texture>("assets/maps/elf-city/emission.png");
        // Explicit relative binding also works in an extracted portable build.
        let elena = block_on(manager.request::<Texture>("assets/maps/elf-city/elena-banner.png"))
            .map_err(|e| format!("City hall Elena banner: {e:?}"))?;
        let handles: Vec<_> = scene.graph.traverse_handle_iter(root).collect();
        for handle in handles {
            let name = scene.graph[handle].name();
            if name.contains("ElenaBannerArtwork") {
                if let Some(mesh) = scene.graph[handle].cast::<Mesh>() {
                    for surface in mesh.surfaces() {
                        let mut material = surface.material().data_ref();
                        material.bind("diffuseTexture", elena.clone());
                        material.bind("emissionTexture", elena.clone());
                        material.set_property("emissionStrength", Vector3::repeat(0.22));
                    }
                }
                continue;
            }
            let color = if name.contains("GlowCyan") {
                Some(Vector3::new(0.15, 1.3, 1.15))
            } else if name.contains("GlowGold") {
                Some(Vector3::new(1.4, 0.8, 0.22))
            } else if name.contains("GlowPink") {
                Some(Vector3::new(1.15, 0.2, 0.6))
            } else if name.contains("GlowWhite") {
                Some(Vector3::new(0.7, 1.0, 1.25))
            } else if name.contains("GlowRed") {
                Some(Vector3::new(1.4, 0.08, 0.18))
            } else {
                None
            };
            if let (Some(color), Some(mesh)) = (color, scene.graph[handle].cast::<Mesh>()) {
                for surface in mesh.surfaces() {
                    let mut material = surface.material().data_ref();
                    material.bind("emissionTexture", emission.clone());
                    material.set_property("emissionStrength", color);
                }
            }
        }
        // Local fills keep the six dark districts readable in gameplay.
        for (p, color, intensity, radius) in [
            ([-2., 5., -34.], [108, 173, 255], 4., 14.),
            ([-15., 4.5, -28.], [80, 255, 220], 4., 12.),
            ([6., 8., -5.], [96, 213, 255], 7., 20.),
            ([49., 6., 20.], [255, 202, 120], 6., 19.),
            ([25., 4.5, 17.], [190, 130, 255], 3., 10.),
            ([35., 6., 8.], [80, 220, 220], 4., 13.),
            ([-4., 4., -47.], [184, 225, 246], 3., 10.),
            ([58., 4., 31.], [184, 225, 246], 3., 10.),
            ([4., 4., 20.], [184, 225, 246], 3., 10.),
            ([36., 4., -4.], [184, 225, 246], 3., 10.),
        ] {
            PointLightBuilder::new(
                BaseLightBuilder::new(
                    BaseBuilder::new()
                        .with_name("Elf city district lighting")
                        .with_cast_shadows(false)
                        .with_local_transform(
                            TransformBuilder::new()
                                .with_local_position(Vector3::from(p))
                                .build(),
                        ),
                )
                .with_color(super::rgb(color))
                .with_scatter_enabled(false)
                .with_intensity(intensity),
            )
            .with_radius(radius)
            .build(&mut scene.graph);
        }
    }
    for light in map.environment_lights() {
        PointLightBuilder::new(
            BaseLightBuilder::new(
                BaseBuilder::new()
                    .with_name("Concept room window fill")
                    .with_cast_shadows(false)
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(Vector3::from(light.position))
                            .build(),
                    ),
            )
            .with_color(super::rgb(light.color))
            .with_intensity(light.intensity),
        )
        .with_radius(light.radius)
        .build(&mut scene.graph);
    }
    super::scenery_batch::merge(&mut scene.graph, root)?;
    println!(
        "MAP_SCENERY_OK: {} / {count} mesh chunks / {} room lights",
        map.slug(),
        map.environment_lights().len()
    );
    Ok(root)
}
