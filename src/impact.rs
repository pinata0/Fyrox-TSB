//! Bounded, authoritative surface contacts shared by rendering and replication.
use crate::game::{direction, ray_box, Block, World, DT, V3};

pub const MAX_IMPACTS: usize = 32;
pub const IMPACT_SECONDS: f32 = 4.0;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ImpactKind {
    #[default]
    Bullet,
    Slash,
}

impl ImpactKind {
    pub fn from_wire(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Bullet),
            1 => Some(Self::Slash),
            _ => None,
        }
    }
    pub fn size_range(self) -> std::ops::RangeInclusive<f32> {
        match self {
            Self::Bullet => 0.08..=0.32,
            Self::Slash => 0.25..=0.8,
        }
    }
}
pub const NORMALS: [V3; 6] = [
    V3::new(-1.0, 0.0, 0.0),
    V3::new(1.0, 0.0, 0.0),
    V3::new(0.0, -1.0, 0.0),
    V3::new(0.0, 1.0, 0.0),
    V3::new(0.0, 0.0, -1.0),
    V3::new(0.0, 0.0, 1.0),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Impact {
    pub id: u32,
    pub tick: u64,
    pub position: V3,
    pub face: u8,
    pub color: [u8; 3],
    pub radius: f32,
    pub kind: ImpactKind,
    /// Surface-plane angle, one turn in 256 steps. Shared by clients/replays.
    pub rotation: u8,
}

impl Impact {
    pub fn angle(self) -> f32 {
        self.rotation as f32 * (std::f32::consts::TAU / 256.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SurfaceHit {
    pub distance: f32,
    pub position: V3,
    pub face: u8,
    pub color: [u8; 3],
}

/// Uses the same padded slabs as projectile collision, but puts the mark on
/// the unpadded surface. Rays originating inside a solid do not make decals.
pub fn trace(map: &[Block], origin: V3, dir: V3, range: f32, radius: f32) -> Option<SurfaceHit> {
    let mut hit = None;
    let mut nearest = range;
    let pad = V3::new(radius, radius, radius);
    for b in map {
        let min = b.min() - pad;
        let max = b.max() + pad;
        let Some(t) = ray_box(origin, dir, min, max) else {
            continue;
        };
        if t > nearest {
            continue;
        }
        let p = origin + dir * t;
        let planes = [
            p.x - min.x,
            p.x - max.x,
            p.y - min.y,
            p.y - max.y,
            p.z - min.z,
            p.z - max.z,
        ];
        let face = (0..6)
            .filter(|&f| dir.dot(NORMALS[f]) < -0.00001)
            .min_by(|&a, &b| planes[a].abs().total_cmp(&planes[b].abs()))?;
        if planes[face].abs() > 0.002 {
            continue;
        }
        let surface = p - NORMALS[face] * radius;
        let lo = b.min();
        let hi = b.max();
        hit = Some(SurfaceHit {
            distance: t,
            position: V3::new(
                surface.x.clamp(lo.x, hi.x),
                surface.y.clamp(lo.y, hi.y),
                surface.z.clamp(lo.z, hi.z),
            ),
            face: face as u8,
            color: b.color,
        });
        nearest = t;
    }
    if dir.y < -0.00001 {
        // Match the simulation's fallback ground plane.
        let t = (0.05 - origin.y) / dir.y;
        if t >= 0.0 && t <= nearest {
            hit = Some(SurfaceHit {
                distance: t,
                position: origin + dir * t,
                face: 3,
                color: [157, 151, 131],
            });
        }
    }
    hit
}

/// Prefer the aimed surface, then check the sides of the close-range swing.
/// Horizontal sampling avoids scratching the floor on ordinary air swings.
pub fn melee_contact(
    map: &[Block],
    origin: V3,
    yaw: f32,
    pitch: f32,
    range: f32,
) -> Option<SurfaceHit> {
    for offset in [0.0, -0.24, 0.24, -0.48, 0.48] {
        let dir = direction(yaw + offset, pitch);
        let limit = map
            .iter()
            .filter_map(|b| ray_box(origin, dir, b.min(), b.max()))
            .fold(range, f32::min);
        if let Some(hit) = trace(map, origin, dir, limit, 0.0) {
            return Some(hit);
        }
    }
    None
}

fn cross(a: V3, b: V3) -> V3 {
    V3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

/// Must match the tangent frame in impact_surface.glsl.
pub fn surface_basis(face: u8) -> (V3, V3) {
    let normal = NORMALS[face as usize];
    let helper = if normal.y.abs() > 0.8 {
        V3::new(1.0, 0.0, 0.0)
    } else {
        V3::new(0.0, 1.0, 0.0)
    };
    let tangent = cross(normal, helper).normalized();
    (tangent, cross(normal, tangent))
}

pub fn slash_rotation(face: u8, yaw: f32, pitch: f32, tilt: f32) -> u8 {
    let right = V3::new(yaw.cos(), 0.0, -yaw.sin());
    let up = cross(direction(yaw, pitch), right);
    let stroke = right * tilt.cos() + up * tilt.sin();
    let (tangent, bitangent) = surface_basis(face);
    let angle = stroke.dot(bitangent).atan2(stroke.dot(tangent));
    ((angle.rem_euclid(std::f32::consts::TAU) * 256.0 / std::f32::consts::TAU).round() as u16 % 256)
        as u8
}

impl World {
    pub(crate) fn record_impact(&mut self, hit: SurfaceHit, radius: f32) {
        self.record_mark(hit, radius, ImpactKind::Bullet, 0);
    }
    pub(crate) fn record_slash(&mut self, hit: SurfaceHit, half_length: f32, rotation: u8) {
        self.record_mark(hit, half_length, ImpactKind::Slash, rotation);
    }
    fn record_mark(&mut self, hit: SurfaceHit, size: f32, kind: ImpactKind, rotation: u8) {
        self.next_impact = self.next_impact.wrapping_add(1);
        if self.impacts.len() >= MAX_IMPACTS {
            self.impacts.remove(0);
        }
        self.impacts.push(Impact {
            id: self.next_impact,
            tick: self.tick,
            position: hit.position,
            face: hit.face,
            color: hit.color,
            radius: size.clamp(*kind.size_range().start(), *kind.size_range().end()),
            kind,
            rotation,
        });
    }
    pub(crate) fn expire_impacts(&mut self) {
        self.impacts
            .retain(|hit| self.tick.saturating_sub(hit.tick) as f32 * DT < IMPACT_SECONDS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_faces_and_ground_have_outward_normals_and_contacts_on_the_surface() {
        let block = Block {
            center: V3::new(0.0, 5.0, 0.0),
            size: V3::new(2.0, 2.0, 2.0),
            color: [150; 3],
            phaseable: false,
        };
        for (face, normal) in NORMALS.into_iter().enumerate() {
            for radius in [0.0, 0.24] {
                let hit = trace(
                    &[block],
                    block.center + normal * 3.0,
                    normal * -1.0,
                    5.0,
                    radius,
                )
                .unwrap();
                assert_eq!(hit.face, face as u8);
                assert!((hit.position - (block.center + normal)).length() < 0.001);
                assert!((hit.distance - (2.0 - radius)).abs() < 0.001);
            }
        }
        assert!(trace(&[block], block.center, V3::new(1.0, 0.0, 0.0), 5.0, 0.0).is_none());
        let ground = trace(
            &[],
            V3::new(0.0, 2.0, 0.0),
            V3::new(0.0, -1.0, 0.0),
            3.0,
            0.0,
        )
        .unwrap();
        assert_eq!(ground.face, 3);
        assert!((ground.position.y - 0.05).abs() < 0.0001);
    }

    #[test]
    fn impacts_stay_bounded_and_expire_even_in_the_lobby() {
        let mut world = World::default();
        let hit = SurfaceHit {
            distance: 1.0,
            position: V3::default(),
            face: 3,
            color: [120; 3],
        };
        for i in 0..100 {
            if i % 2 == 0 {
                world.record_impact(hit, 0.12);
            } else {
                world.record_slash(hit, 0.48, 35);
            }
        }
        assert_eq!(world.impacts.len(), MAX_IMPACTS);
        assert_eq!(world.impacts.last().unwrap().id, 100);
        for _ in 0..241 {
            world.tick(Default::default());
        }
        assert!(world.impacts.is_empty());
    }

    #[test]
    fn melee_sweep_finds_off_center_objects_without_crossing_solids() {
        let origin = V3::new(0.0, 1.5, 0.0);
        let side = Block {
            center: V3::new(0.65, 1.5, 2.0),
            size: V3::new(0.3, 3.0, 0.3),
            color: [150; 3],
            phaseable: false,
        };
        assert!(trace(&[side], origin, direction(0.0, 0.0), 3.8, 0.0).is_none());
        assert!(melee_contact(&[side], origin, 0.0, 0.0, 3.8).is_some());
        let enclosing = Block {
            center: origin,
            size: V3::new(1.0, 3.0, 1.0),
            ..side
        };
        assert!(melee_contact(&[enclosing, side], origin, 0.0, 0.0, 3.8).is_none());
        assert!(melee_contact(&[], origin, 0.0, 0.0, 3.8).is_none());
    }

    #[test]
    fn slash_orientation_tracks_the_view_on_every_surface() {
        for (face, normal) in NORMALS.into_iter().enumerate() {
            let dir = normal * -1.0;
            let yaw = dir.x.atan2(dir.z);
            let pitch = dir.y.asin();
            let right = V3::new(yaw.cos(), 0.0, -yaw.sin());
            let up = cross(direction(yaw, pitch), right);
            for tilt in [-0.55_f32, 0.55] {
                let rotation = slash_rotation(face as u8, yaw, pitch, tilt);
                let angle = rotation as f32 * std::f32::consts::TAU / 256.0;
                let (tangent, bitangent) = surface_basis(face as u8);
                let actual = tangent * angle.cos() + bitangent * angle.sin();
                let expected = right * tilt.cos() + up * tilt.sin();
                assert!(
                    actual.dot(expected) > 0.999,
                    "face {face}: cut must follow the projected swing"
                );
            }
        }
    }
}
