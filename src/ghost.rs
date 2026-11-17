//! Wall-face contacts shared by local and networked ghost visuals.
use crate::{
    game::{Block, V3},
    movement::BODY_HEIGHT,
};

#[derive(Debug, Clone, Copy)]
pub struct WallContact {
    pub block: usize,
    pub face: u8,
    pub center: V3,
    pub normal: V3,
}

/// Only vertical, phaseable faces touching the body. Both sides of a thin wall can ripple.
pub fn wall_contacts(map: &[Block], feet: V3) -> Vec<WallContact> {
    let mut contacts = Vec::new();
    for (block, b) in map.iter().enumerate().filter(|(_, b)| b.phaseable) {
        let lo = b.min();
        let hi = b.max();
        if feet.y >= hi.y || feet.y + BODY_HEIGHT <= lo.y {
            continue;
        }
        let y = (feet.y + BODY_HEIGHT * 0.55).clamp(lo.y, hi.y);
        for (face, plane, axis, tangent, low, high, normal) in [
            (0, lo.x, feet.x, feet.z, lo.z, hi.z, V3::new(-1.0, 0.0, 0.0)),
            (1, hi.x, feet.x, feet.z, lo.z, hi.z, V3::new(1.0, 0.0, 0.0)),
            (2, lo.z, feet.z, feet.x, lo.x, hi.x, V3::new(0.0, 0.0, -1.0)),
            (3, hi.z, feet.z, feet.x, lo.x, hi.x, V3::new(0.0, 0.0, 1.0)),
        ] {
            // Includes one snapshot interval of travel to avoid dropping thin-wall exits.
            if (axis - plane).abs() <= 0.65 && tangent >= low - 0.4 && tangent <= high + 0.4 {
                contacts.push(WallContact {
                    block,
                    face,
                    center: if face < 2 {
                        V3::new(plane, y, tangent.clamp(low, high))
                    } else {
                        V3::new(tangent.clamp(low, high), y, plane)
                    },
                    normal,
                });
            }
        }
    }
    contacts
}
