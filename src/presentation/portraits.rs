//! Shared 2D illustration assets and face framing for player and selection cards.
use super::{characters, TextureKind, TextureResource};
use fyrox::{asset::manager::ResourceManager, core::math::Rect};

// Face center (u, v) and crop width in the original masked illustration.
// Hats, mounts and long hair must not determine the zoom.
const FRAMING: [(f32, f32, f32); 15] = [
    (0.500, 0.360, 0.500), // Vivi
    (0.500, 0.330, 0.670), // Naia
    (0.490, 0.300, 0.700), // Erpin
    (0.550, 0.390, 0.340), // Id: face inside the crescent mount.
    (0.500, 0.395, 0.340), // Aya: face below the parasol.
    (0.500, 0.350, 0.760), // Picora
    (0.450, 0.470, 0.500), // Chloe: her face, not the puppet's.
    (0.485, 0.385, 0.640), // Renua
    (0.470, 0.345, 0.700), // Mayo
    (0.550, 0.250, 0.460), // Bella
    (0.540, 0.320, 0.510), // Ui: exclude the frog mount.
    (0.490, 0.320, 0.760), // Butter
    (0.430, 0.310, 0.680), // Komi
    (0.475, 0.345, 0.730), // Hilde
    (0.410, 0.350, 0.615), // Shion
];

pub(super) fn load(manager: &ResourceManager) -> Vec<TextureResource> {
    characters::SLUGS
        .iter()
        .map(|slug| manager.request(format!("assets/character-inputs/inputs-masked/{slug}.png")))
        .collect()
}

pub(super) fn uv(id: usize, texture: &TextureResource, width: f32, height: f32) -> Rect<f32> {
    let (center_x, center_y, crop_width) = FRAMING[id];
    let aspect = texture
        .state()
        .data_ref()
        .map_or(1.0, |data| match data.kind() {
            TextureKind::Rectangle { width, height } => width as f32 / height as f32,
            _ => 1.0,
        });
    // Match each widget's aspect ratio without stretching the illustration.
    let crop_height = crop_width * aspect * height / width;
    let fit = crop_height.max(1.0);
    let crop_width = crop_width / fit;
    let crop_height = crop_height / fit;
    Rect::new(
        (center_x - crop_width * 0.5).clamp(0.0, 1.0 - crop_width),
        (center_y - crop_height * 0.5).clamp(0.0, 1.0 - crop_height),
        crop_width,
        crop_height,
    )
}
