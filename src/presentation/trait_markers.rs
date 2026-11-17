//! Original Trickcal trait markers. Sources and alpha checks live beside the PNGs.
use super::*;
use fyrox::asset::manager::ResourceManager;
use trickcal_shooting_balddagu::roster::{Personality, Position, Race};

#[derive(Debug, Default)]
pub struct TraitMarkers {
    personalities: Vec<TextureResource>,
    races: Vec<TextureResource>,
    positions: Vec<TextureResource>,
}

impl TraitMarkers {
    pub fn new(manager: &ResourceManager) -> Self {
        let load = |category: &str, names: &[&str]| {
            names
                .iter()
                .map(|name| manager.request(format!("assets/ui/markers/{category}-{name}.png")))
                .collect()
        };
        // Stable roster enum order; Sprite=Fairy and Elemental=Spirit.
        Self {
            personalities: load("personality", &["pure", "cool", "mad", "lively", "gloomy"]),
            races: load(
                "race",
                &[
                    "fairy", "beast", "elf", "spirit", "ghost", "dragon", "witch",
                ],
            ),
            positions: load("position", &["front", "middle", "rear"]),
        }
    }

    pub fn personality(&self, value: Personality) -> TextureResource {
        self.personalities[value as usize].clone()
    }

    pub fn race(&self, value: Race) -> TextureResource {
        self.races[value as usize].clone()
    }

    pub fn position(&self, value: Position) -> TextureResource {
        self.positions[value as usize].clone()
    }
}
