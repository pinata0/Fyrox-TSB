//! Authored maps shared by Blender, rendering, server collision and navigation.
use crate::game::{Block, V3};
use serde::Deserialize;
use std::sync::OnceLock;

pub const SLUG: &str = "fairy-kingdom-castle";
pub const NAME: &str = "FAIRY KINGDOM CASTLE";
pub const UPPER_FLOOR: f32 = 3.6;
pub const MIN_X: f32 = -22.0;
pub const MAX_X: f32 = 68.0;
pub const MIN_Z: f32 = -54.0;
pub const MAX_Z: f32 = 38.0;

/// Stable wire IDs; never use display text to identify a multiplayer map.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MapId {
    #[default]
    FairyKingdomCastle = 0,
    ElfCity = 1,
}
impl MapId {
    pub const ALL: [Self; 2] = [Self::FairyKingdomCastle, Self::ElfCity];
    pub fn from_wire(id: u8) -> Option<Self> {
        Self::ALL.get(id as usize).copied()
    }
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.slug() == slug)
    }
    pub fn slug(self) -> &'static str {
        match self {
            Self::FairyKingdomCastle => SLUG,
            Self::ElfCity => "elf-city",
        }
    }
    pub fn display_name(self) -> &'static str {
        match self {
            Self::FairyKingdomCastle => "요정왕국 성",
            Self::ElfCity => "엘프 도시",
        }
    }
    pub fn detail(self) -> &'static str {
        match self {
            Self::FairyKingdomCastle => "A 왕실 정원 / B 왕관 광장",
            Self::ElfCity => "A 환승 정비고 / B 시청 광장",
        }
    }
    pub fn points(self) -> [V3; 2] {
        self.layout().points.map(point)
    }
    pub fn arena(self) -> Vec<Block> {
        self.layout()
            .blocks
            .iter()
            .map(|b| Block {
                center: point(b.center),
                size: point(b.size),
                color: b.color,
                phaseable: b.phaseable,
            })
            .collect()
    }
    pub fn spawn_center(self, attacking: bool, point_b: bool) -> V3 {
        let s = &self.layout().spawns;
        point(match (attacking, point_b) {
            (true, false) => s.attack_a,
            (true, true) => s.attack_b,
            (false, false) => s.defense_a,
            (false, true) => s.defense_b,
        })
    }
    pub fn environment_lights(self) -> &'static [EnvironmentLight] {
        &self.layout().lights
    }
    fn layout(self) -> &'static Layout {
        static CASTLE: OnceLock<Layout> = OnceLock::new();
        static CITY: OnceLock<Layout> = OnceLock::new();
        let (cache, json) = match self {
            Self::FairyKingdomCastle => (
                &CASTLE,
                include_str!("../assets/maps/fairy-kingdom-castle/layout.json"),
            ),
            Self::ElfCity => (&CITY, include_str!("../assets/maps/elf-city/layout.json")),
        };
        cache.get_or_init(|| serde_json::from_str(json).expect("validated map layout"))
    }
}

#[derive(Deserialize)]
struct Solid {
    center: [f32; 3],
    size: [f32; 3],
    color: [u8; 3],
    phaseable: bool,
}
#[derive(Deserialize)]
struct Spawns {
    attack_a: [f32; 3],
    attack_b: [f32; 3],
    defense_a: [f32; 3],
    defense_b: [f32; 3],
}
#[derive(Deserialize)]
pub struct EnvironmentLight {
    pub position: [f32; 3],
    pub color: [u8; 3],
    pub radius: f32,
    pub intensity: f32,
}
#[derive(Deserialize)]
struct Layout {
    blocks: Vec<Solid>,
    spawns: Spawns,
    points: [[f32; 3]; 2],
    #[serde(default)]
    lights: Vec<EnvironmentLight>,
}
fn point(v: [f32; 3]) -> V3 {
    V3::new(v[0], v[1], v[2])
}
pub fn spawn_center(attacking: bool, point_b: bool) -> V3 {
    MapId::default().spawn_center(attacking, point_b)
}
pub fn arena() -> Vec<Block> {
    MapId::default().arena()
}
