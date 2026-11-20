//! Bake static scenery by material and spatial cell, retaining culling between rooms.
use super::*;
use fyrox::scene::mesh::{Mesh, RenderPath};
use std::collections::BTreeMap;

const CELL: f32 = 28.0;
type Key = (i32, i32, u64, u64, bool);

struct Batch {
    data: SurfaceData,
    material: MaterialResource,
}

fn append(groups: &mut BTreeMap<Key, Batch>, key: Key, data: SurfaceData, material: MaterialResource) -> Result<(), String> {
    if let Some(batch) = groups.get_mut(&key) {
        let offset = batch.data.vertex_buffer.vertex_count();
        {
            let mut vertices = batch.data.vertex_buffer.modify();
            for vertex in data.vertex_buffer.raw_data().chunks_exact(data.vertex_buffer.vertex_size() as usize) {
                vertices.push_vertex_raw(vertex).map_err(|e| format!("Map batch vertex: {e:?}"))?;
            }
        }
        batch.data.geometry_buffer.modify().push_triangles_iter_with_offset(offset, data.geometry_buffer.iter().copied());
    } else {
        groups.insert(key, Batch { data, material });
    }
    Ok(())
}

pub(super) fn merge(g: &mut Graph, root: Handle<Node>) -> Result<(), String> {
    g.update_hierarchical_data_for_descendants(root);
    let Some(inverse) = g[root].global_transform().try_inverse() else { return Ok(()); };
    let mut groups: BTreeMap<Key, Batch> = BTreeMap::new();
    let mut shadows = BTreeMap::new();
    let standard = Material::standard();
    let standard_shader = standard.shader().key();
    let shadow_material = MaterialResource::new(Material::from_shader(
        super::character_shadows::shadow_shader(standard.shader())));
    let mut shadow_surfaces = 0;
    let mut sources = Vec::new();
    let mut surfaces = 0;
    let mut triangles = 0;
    for h in g.traverse_handle_iter(root) {
        let Some(mesh) = g[h].cast::<Mesh>() else { continue; };
        if !mesh.global_visibility() || mesh.render_path() != RenderPath::Deferred
            || !mesh.blend_shapes().is_empty() || *mesh.render_mask != *g[root].render_mask
            || mesh.surfaces().iter().any(|s| !s.bones().is_empty() || s.data_ref().data_ref().blend_shapes_container.is_some()) {
            continue;
        }
        let center = mesh.world_bounding_box().center();
        let cell = ((center.x / CELL).floor() as i32, (center.z / CELL).floor() as i32);
        let transform = inverse * mesh.global_transform();
        // Mirrored nodes need different face winding. Keep them on their original path.
        if transform.fixed_view::<3, 3>(0, 0).determinant() <= 0.0 { continue; }
        for surface in mesh.surfaces() {
            let source = surface.data_ref().data_ref();
            let mut data = SurfaceData::new(source.vertex_buffer.clone(), source.geometry_buffer.clone());
            data.transform_geometry(&transform).map_err(|e| format!("Map batch transform: {e:?}"))?;
            // Standard shadow passes use only diffuse texture alpha. Opaque
            // untextured scenery can share one shadow draw regardless of color.
            // Textured/custom materials keep their exact original shadow path.
            let mat = surface.material().data_ref();
            let shared_shadow = mesh.cast_shadows() && mat.shader().key() == standard_shader
                && mat.texture("diffuseTexture").is_none();
            drop(mat);
            let key = (cell.0, cell.1, surface.material().key(), data.vertex_buffer.layout_hash(), mesh.cast_shadows() && !shared_shadow);
            triangles += data.geometry_buffer.len();
            surfaces += 1;
            if shared_shadow {
                let shadow_data = SurfaceData::new(data.vertex_buffer.clone(), data.geometry_buffer.clone());
                append(&mut shadows, (cell.0, cell.1, 0, key.3, true), shadow_data, shadow_material.clone())?;
                shadow_surfaces += 1;
            }
            append(&mut groups, key, data, surface.material().clone())?;
        }
        sources.push(h);
    }
    if groups.len() >= surfaces && shadows.len() >= shadow_surfaces { return Ok(()); }
    for h in sources {
        // Map collision uses the independent shared layout. No gameplay nodes are removed.
        if g[h].children().is_empty() {
            g.remove_node(h);
        } else {
            g[h].cast_mut::<Mesh>().unwrap().set_surfaces(Vec::new());
        }
    }
    let count = groups.len();
    let shadow_count = shadows.len();
    let batches = groups.into_iter().map(|(k, b)| (k, b, "Static scenery"))
        .chain(shadows.into_iter().map(|(k, b)| (k, b, "Static shadow")));
    for ((x, z, _, _, shadow), batch, name) in batches {
        let node: Handle<Node> = MeshBuilder::new(BaseBuilder::new()
            .with_name(format!("{name} {x},{z}"))
            .with_cast_shadows(shadow))
            .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(batch.data))
                .with_material(batch.material).build()])
            .build(g).to_base();
        g.link_nodes(node, root);
    }
    println!("MAP_BATCH_OK: {surfaces} surfaces -> {count} color + {shadow_count} shared shadow batches, {triangles} color triangles preserved");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fyrox::scene::mesh::buffer::{VertexAttributeUsage, VertexReadTrait};

    #[test]
    fn merge_preserves_world_vertices_triangles_and_shared_material() {
        let mut g = Graph::new();
        let root = PivotBuilder::new(BaseBuilder::new().with_local_transform(
            TransformBuilder::new().with_local_scale(Vector3::repeat(0.01)).build()
        )).build(&mut g).to_base();
        let shapes = Shapes::new();
        let mat = material(MINT);
        for x in [100.0, 200.0] {
            let h = shapes.mesh(&mut g, "source", V3::new(x, 100.0, 100.0), V3::new(50.0, 100.0, 150.0), mat.clone(), 0);
            g.link_nodes(h, root);
        }
        g.update_hierarchical_data_for_descendants(root);
        fn vertices(g: &Graph, root: Handle<Node>) -> Vec<[i32; 3]> {
            let mut points = Vec::new();
            for h in g.traverse_handle_iter(root) {
                if g[h].name().starts_with("Static shadow ") { continue; }
                if let Some(mesh) = g[h].cast::<Mesh>() {
                    for s in mesh.surfaces() {
                        for v in s.data_ref().data_ref().vertex_buffer.iter() {
                            let p = mesh.global_transform().transform_point(&v.read_3_f32(VertexAttributeUsage::Position).unwrap().into());
                            points.push([p.x, p.y, p.z].map(|n| (n * 10000.0).round() as i32));
                        }
                    }
                }
            }
            points.sort(); points
        }
        let before = vertices(&g, root);
        merge(&mut g, root).unwrap();
        g.update_hierarchical_data_for_descendants(root);
        assert_eq!(vertices(&g, root), before);
        let meshes: Vec<_> = g.traverse_handle_iter(root).filter(|h| !g[*h].name().starts_with("Static shadow ")).filter_map(|h| g[h].cast::<Mesh>()).collect();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].surfaces()[0].material().key(), mat.key());
        assert_eq!(meshes[0].surfaces()[0].data_ref().data_ref().geometry_buffer.len(), 24);
        assert_eq!(shapes.cube.data_ref().geometry_buffer.len(), 12, "cached source stays unchanged");
        let shadow = g.traverse_handle_iter(root).find(|h| g[*h].name().starts_with("Static shadow ")).unwrap();
        let shadow = g[shadow].cast::<Mesh>().unwrap();
        assert!(shadow.cast_shadows());
        assert!(!meshes[0].cast_shadows());
        assert_eq!(shadow.surfaces()[0].data_ref().data_ref().geometry_buffer.len(), 24);
        assert!(!shadow.surfaces()[0].material().data_ref().shader().data_ref().definition.passes.iter().any(|p| p.name == "GBuffer"));
    }
}
