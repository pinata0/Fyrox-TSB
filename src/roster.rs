//! The approved MVP roster. IDs are stable on the wire; variants are not included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Personality {
    Pure,
    Cool,
    Mad,
    Lively,
    Gloomy,
}
impl Personality {
    pub fn name(self) -> &'static str {
        ["PURE", "COOL", "MAD", "LIVELY", "GLOOMY"][self as usize]
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Position {
    Front,
    Middle,
    Rear,
}
impl Position {
    pub fn name(self) -> &'static str {
        ["FRONT", "MIDDLE", "REAR"][self as usize]
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Race {
    Fairy,
    Beast,
    Elf,
    Spirit,
    Ghost,
    Dragon,
    Witch,
}
impl Race {
    pub fn name(self) -> &'static str {
        [
            "FAIRY", "BEAST", "ELF", "SPIRIT", "GHOST", "DRAGON", "WITCH",
        ][self as usize]
    }
    pub fn passive(self) -> &'static str {
        match self {
            Self::Fairy => "Healing received +10%",
            Self::Beast => "No race passive",
            Self::Elf => "Reload time -10%",
            Self::Spirit => "After 4s safe: restore 8% HP barrier",
            Self::Ghost => "Shift: wall phase for 3s, then 12s cooldown",
            Self::Dragon => "Knockback distance -30%",
            Self::Witch => "Ability cooldown -5%",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Character {
    pub name: &'static str,
    pub korean: &'static str,
    pub personality: Personality,
    pub position: Position,
    pub race: Race,
    pub healer: bool,
    pub color: [u8; 3],
}
pub const ROSTER: [Character; 15] = {
    use Personality::*;
    use Position::*;
    use Race::*;
    [
        Character {
            name: "Vivi",
            korean: "비비",
            personality: Pure,
            position: Front,
            race: Dragon,
            healer: false,
            color: [163, 204, 174],
        },
        Character {
            name: "Naia",
            korean: "나이아",
            personality: Pure,
            position: Middle,
            race: Spirit,
            healer: true,
            color: [106, 184, 236],
        },
        Character {
            name: "Erpin",
            korean: "에르핀",
            personality: Pure,
            position: Rear,
            race: Fairy,
            healer: false,
            color: [240, 202, 96],
        },
        Character {
            name: "Id",
            korean: "이드",
            personality: Cool,
            position: Front,
            race: Elf,
            healer: false,
            color: [202, 220, 238],
        },
        Character {
            name: "Aya",
            korean: "아야",
            personality: Cool,
            position: Middle,
            race: Witch,
            healer: false,
            color: [91, 138, 200],
        },
        Character {
            name: "Picora",
            korean: "피코라",
            personality: Cool,
            position: Rear,
            race: Witch,
            healer: true,
            color: [182, 160, 230],
        },
        Character {
            name: "Chloe",
            korean: "클로에",
            personality: Mad,
            position: Front,
            race: Fairy,
            healer: false,
            color: [128, 96, 158],
        },
        Character {
            name: "Renua",
            korean: "리뉴아",
            personality: Mad,
            position: Middle,
            race: Elf,
            healer: false,
            color: [238, 148, 103],
        },
        Character {
            name: "Mayo",
            korean: "마요",
            personality: Mad,
            position: Rear,
            race: Fairy,
            healer: false,
            color: [211, 213, 184],
        },
        Character {
            name: "Bella",
            korean: "벨라",
            personality: Lively,
            position: Front,
            race: Ghost,
            healer: false,
            color: [219, 103, 117],
        },
        Character {
            name: "Ui",
            korean: "우이",
            personality: Lively,
            position: Middle,
            race: Spirit,
            healer: true,
            color: [242, 170, 195],
        },
        Character {
            name: "Butter",
            korean: "버터",
            personality: Lively,
            position: Rear,
            race: Beast,
            healer: false,
            color: [245, 198, 83],
        },
        Character {
            name: "Komi",
            korean: "코미",
            personality: Gloomy,
            position: Front,
            race: Beast,
            healer: false,
            color: [184, 162, 217],
        },
        Character {
            name: "Hilde",
            korean: "힐데",
            personality: Gloomy,
            position: Middle,
            race: Elf,
            healer: true,
            color: [192, 220, 215],
        },
        Character {
            name: "Shion",
            korean: "시온 더 다크불릿",
            personality: Gloomy,
            position: Rear,
            race: Ghost,
            healer: false,
            color: [143, 117, 201],
        },
    ]
};
pub const DEFAULT_TEAM: [u8; 6] = [0, 12, 1, 10, 11, 14];
pub fn character(id: u8) -> &'static Character {
    &ROSTER[id as usize]
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SelectionResult {
    #[default]
    Accepted,
    Invalid,
    Locked,
    Taken,
    PositionFull,
}
impl SelectionResult {
    pub fn message(self) -> &'static str {
        match self {
            Self::Accepted => "Character confirmed",
            Self::Invalid => "Invalid character or player",
            Self::Locked => "Return to your active spawn to change hero (or wait for respawn)",
            Self::Taken => "A teammate already selected this character",
            Self::PositionFull => "This position already has 3 teammates",
        }
    }
    pub fn from_wire(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Accepted,
            1 => Self::Invalid,
            2 => Self::Locked,
            3 => Self::Taken,
            4 => Self::PositionFull,
            _ => return None,
        })
    }
}
