//! Temporary surface dents and deterministic, outward-flying chunks.
use super::*;
use fyrox::core::algebra::Vector4;
use trickcal_shooting_balddagu::impact::{
    surface_basis, Impact, ImpactKind, IMPACT_SECONDS, MAX_IMPACTS, NORMALS,
};

const DEBRIS_SECONDS: f32 = 0.95;
const CHUNKS: usize = 7;
const SLASH_CHUNKS: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_do_not_duplicate_chunks_and_expiry_or_rewind_removes_nodes() {
        for kind in [ImpactKind::Bullet, ImpactKind::Slash] {
            check_lifetime(kind);
        }
    }

    fn check_lifetime(kind: ImpactKind) {
        let mut graph = Graph::new();
        let surfaces = ghost_walls::GhostWalls::default();
        let mut effects = Impacts::default();
        let mut world = World {
            tick: 100,
            ..Default::default()
        };
        world.impacts.push(Impact {
            id: 1,
            tick: 100,
            position: V3::new(0.0, 1.0, 0.0),
            face: 4,
            color: [150; 3],
            radius: if kind == ImpactKind::Slash {
                0.48
            } else {
                0.13
            },
            kind,
            rotation: 23,
        });
        let baseline = graph.traverse_handle_iter(graph.get_root()).count();
        for _ in 0..5 {
            effects.update(&mut graph, &world, DT, &surfaces);
        }
        assert_eq!(
            graph.traverse_handle_iter(graph.get_root()).count(),
            baseline
                + if kind == ImpactKind::Slash {
                    SLASH_CHUNKS
                } else {
                    CHUNKS
                }
        );
        world.tick = 161;
        effects.update(&mut graph, &world, DT, &surfaces);
        assert_eq!(
            graph.traverse_handle_iter(graph.get_root()).count(),
            baseline
        );
        assert_eq!(effects.effects.len(), 1, "mark outlasts its fragments");
        world.tick = 341;
        effects.update(&mut graph, &world, DT, &surfaces);
        assert!(effects.effects.is_empty());
        effects.update(&mut graph, &world, DT, &surfaces);
        assert!(
            effects.effects.is_empty(),
            "a repeated stale packet cannot revive a mark"
        );
        world.tick = 100;
        effects.update(&mut graph, &world, DT, &surfaces);
        assert_eq!(
            effects.effects.len(),
            1,
            "replay rewind restores its recorded contact"
        );
        world.round += 1;
        world.impacts.clear();
        effects.update(&mut graph, &world, DT, &surfaces);
        assert!(effects.effects.is_empty());
        assert_eq!(
            graph.traverse_handle_iter(graph.get_root()).count(),
            baseline
        );
    }
}

#[derive(Debug)]
struct Chunk {
    node: Handle<Node>,
    offset: Vector3<f32>,
    velocity: Vector3<f32>,
    spin: Vector3<f32>,
    scale: Vector3<f32>,
}

#[derive(Debug)]
struct Effect {
    hit: Impact,
    chunks: Vec<Chunk>,
}

#[derive(Debug, Default)]
pub(super) struct Impacts {
    effects: Vec<Effect>,
    geometry: Option<SurfaceResource>,
    clock: f64,
    tick: u64,
    round: u16,
    map: Option<MapId>,
}

impl Impacts {
    pub(super) fn update(
        &mut self,
        g: &mut Graph,
        w: &World,
        dt: f32,
        surfaces: &ghost_walls::GhostWalls,
    ) {
        if self.round != w.round || self.map != Some(w.map_id) || w.tick < self.tick {
            for effect in self.effects.drain(..) {
                for chunk in effect.chunks {
                    g.remove_node(chunk.node);
                }
            }
            self.clock = w.tick as f64;
        } else {
            self.clock = (self.clock + (dt / DT) as f64).max(w.tick as f64);
        }
        self.round = w.round;
        self.map = Some(w.map_id);
        self.tick = w.tick;
        // A snapshot repeats contacts to survive packet loss. Reuse each id,
        // and retain the server timestamp so a late join never restarts a hit.
        for hit in &w.impacts {
            let age = ((self.clock - hit.tick as f64) * DT as f64).max(0.0) as f32;
            if age >= IMPACT_SECONDS || self.effects.iter().any(|e| e.hit.id == hit.id) {
                continue;
            }
            if self.effects.len() == MAX_IMPACTS {
                for chunk in self.effects.remove(0).chunks {
                    g.remove_node(chunk.node);
                }
            }
            let chunks = if age < DEBRIS_SECONDS {
                self.spawn_chunks(g, *hit)
            } else {
                vec![]
            };
            self.effects.push(Effect { hit: *hit, chunks });
        }
        let mut centers = Vec::with_capacity(MAX_IMPACTS);
        let mut normals = Vec::with_capacity(MAX_IMPACTS);
        let mut shapes = Vec::with_capacity(MAX_IMPACTS);
        self.effects.retain_mut(|effect| {
            let age = ((self.clock - effect.hit.tick as f64) * DT as f64).max(0.0) as f32;
            let hit = effect.hit;
            if age >= DEBRIS_SECONDS {
                for chunk in effect.chunks.drain(..) {
                    g.remove_node(chunk.node);
                }
            }
            if age >= IMPACT_SECONDS {
                return false;
            }
            let normal = vec(NORMALS[hit.face as usize]);
            for chunk in &effect.chunks {
                let mut offset = chunk.velocity * age - Vector3::y() * (4.9 * age * age);
                if normal.y > 0.5 && offset.y < 0.0 {
                    offset.y *= -0.22;
                }
                let mut position = vec(hit.position) + chunk.offset + normal * 0.025 + offset;
                position.y = position.y.max(0.055);
                let fade = ((DEBRIS_SECONDS - age) / 0.28).clamp(0.0, 1.0);
                g[chunk.node]
                    .local_transform_mut()
                    .set_position(position)
                    .set_scale(chunk.scale * fade)
                    .set_rotation(UnitQuaternion::from_scaled_axis(chunk.spin * age));
            }
            let p = hit.position;
            centers.push(Vector4::new(p.x, p.y, p.z, hit.radius));
            normals.push(Vector4::new(
                normal.x,
                normal.y,
                normal.z,
                ((IMPACT_SECONDS - age) / 1.25).clamp(0.0, 1.0),
            ));
            shapes.push(Vector4::new(
                hit.kind as u8 as f32,
                hit.angle().cos(),
                hit.angle().sin(),
                0.055,
            ));
            true
        });
        surfaces.set_impacts(centers, normals, shapes);
    }

    fn spawn_chunks(&mut self, g: &mut Graph, hit: Impact) -> Vec<Chunk> {
        let geometry = self.geometry.get_or_insert_with(|| {
            SurfaceResource::new_embedded(SurfaceData::make_cube(Matrix4::identity()))
        });
        let normal = vec(NORMALS[hit.face as usize]);
        let (tangent, bitangent) = surface_basis(hit.face);
        let (tangent, bitangent) = (vec(tangent), vec(bitangent));
        let stroke = tangent * hit.angle().cos() + bitangent * hit.angle().sin();
        let slash = hit.kind == ImpactKind::Slash;
        let count = if slash { SLASH_CHUNKS } else { CHUNKS };
        let mat = material(hit.color.map(|v| (v as f32 * 0.83).round() as u8));
        (0..count)
            .map(|i| {
                let seed = hit
                    .id
                    .wrapping_mul(1664525)
                    .wrapping_add((i as u32).wrapping_mul(1013904223));
                let noise = (seed % 997) as f32 / 997.0;
                let angle = i as f32 * 2.399_963 + noise;
                let radial = tangent * angle.cos() + bitangent * angle.sin();
                let size = hit.radius * (if slash { 0.23 } else { 1.0 }) * (0.13 + noise * 0.18);
                let node =
                    MeshBuilder::new(BaseBuilder::new().with_name("Temporary impact fragment"))
                        .with_surfaces(vec![SurfaceBuilder::new(geometry.clone())
                            .with_material(mat.clone())
                            .build()])
                        .build(g)
                        .to_base();
                character_shadows::disable_casting(g, node);
                Chunk {
                    node,
                    offset: if slash {
                        stroke * hit.radius * (i as f32 / (count - 1) as f32 - 0.5) * 1.6
                    } else {
                        Vector3::zeros()
                    },
                    velocity: if slash {
                        normal * (0.7 + noise) + stroke * (0.7 + noise * 0.5) + Vector3::y() * 0.5
                    } else {
                        normal * (1.1 + noise * 1.6)
                            + radial * (0.6 + noise * 1.2)
                            + Vector3::y() * 0.7
                    },
                    spin: Vector3::new(7.0 + noise * 9.0, angle * 2.0, 11.0 - noise * 6.0),
                    scale: Vector3::new(size, size * 0.65, size * 1.5),
                }
            })
            .collect()
    }
}
