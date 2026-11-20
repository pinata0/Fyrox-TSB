//! Vivi's two halves of Attack1_1, mercury guard and falling Quicksilver Lance.
use crate::{
    combat::{AttackClass, Projectile, ProjectileKind},
    game::{direction, team, Actor, Input, World, DT, MAX_HP, PLAYERS, V3},
    motion::Motion,
};

pub const LEFT_SECONDS: f32 = 24.0 / 30.0;
pub const RIGHT_SECONDS: f32 = 33.0 / 30.0;
pub const SKILL_SECONDS: f32 = 44.0 / 30.0;
pub const ULTIMATE_SECONDS: f32 = 4.0;
pub const SHIELD_RELEASE: f32 = 20.0 / 30.0;
pub const SHIELD_HEALTH: f32 = MAX_HP as f32 * 0.40;
pub const SHIELD_SECONDS: f32 = 6.0;
pub const LANCE_RELEASE: f32 = 35.0 / 30.0;
pub const FALL_SECONDS: f32 = 27.0 / 30.0;
pub const POOL_SECONDS: f32 = 5.0;
pub const FIELD_SECONDS: f32 = FALL_SECONDS + POOL_SECONDS;
pub const POOL_RADIUS: f32 = 4.0;
pub const IMPACT_DAMAGE: f32 = 100.0;
pub const LANCE_HEIGHT: f32 = 8.0;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Left,
    Right,
    Skill,
    Ultimate,
}
impl Phase {
    pub fn from_wire(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Off,
            1 => Self::Left,
            2 => Self::Right,
            3 => Self::Skill,
            4 => Self::Ultimate,
            _ => return None,
        })
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Left => LEFT_SECONDS,
            Self::Right => RIGHT_SECONDS,
            Self::Skill => SKILL_SECONDS,
            Self::Ultimate => ULTIMATE_SECONDS,
        }
    }
    pub fn basic(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
    pub fn motion(self) -> Option<Motion> {
        Some(match self {
            Self::Off => return None,
            Self::Left => Motion::ViviLeft,
            Self::Right => Motion::ViviRight,
            Self::Skill => Motion::ViviSkill,
            Self::Ultimate => Motion::ViviUltimate,
        })
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub elapsed: f32,
}
impl State {
    pub fn active(self) -> bool {
        self.phase != Phase::Off
    }
    pub fn casting(self) -> bool {
        matches!(self.phase, Phase::Skill | Phase::Ultimate)
    }
    pub fn pack(self) -> u32 {
        if self.active() {
            ((self.elapsed / DT).round() as u32)
                .min((self.phase.duration() / DT).round() as u32 - 1)
        } else {
            0
        }
    }
    pub fn unpack(phase: Phase, ticks: u32) -> Option<Self> {
        let elapsed = ticks as f32 * DT;
        ((phase == Phase::Off && ticks == 0) || (phase != Phase::Off && elapsed < phase.duration()))
            .then_some(Self { phase, elapsed })
    }
}
pub fn advance_actor(a: &mut Actor) {
    if a.character != 0 || !a.alive() || a.combat.action_interrupted() || a.burrowed() {
        a.combat.vivi = State::default();
    } else if a.combat.vivi.active() {
        a.combat.vivi.elapsed += DT
            * crate::renua_skill::time_factor(a)
            * if a.combat.vivi.phase.basic() {
                crate::attack::rate(a)
            } else {
                1.0
            };
        if a.combat.vivi.elapsed + 0.00001 >= a.combat.vivi.phase.duration() {
            a.combat.vivi = State::default();
        }
    }
}
pub fn smooth(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    v * v * (3.0 - 2.0 * v)
}
/// E's root lift belongs only to the world model; Q moves the actor and camera.
pub fn skill_lift(t: f32) -> f32 {
    0.42 * smooth((t - 0.52) / 0.18) * (1.0 - smooth((t - 1.03) / 0.43))
}
pub fn ultimate_lift(t: f32) -> f32 {
    1.85 * smooth((t - 0.10) / 0.72) * (1.0 - smooth((t - 1.66) / 0.67))
}
pub fn lance_tip(age: f32) -> f32 {
    LANCE_HEIGHT * (1.0 - (age / FALL_SECONDS).clamp(0.0, 1.0).powi(2))
}

fn clear(map: &[crate::game::Block], from: V3, to: V3) -> bool {
    let d = to - from;
    !map.iter().any(|b| {
        crate::game::ray_box(from, d.normalized(), b.min(), b.max())
            .is_some_and(|t| t < d.length() - 0.02)
    })
}
pub fn in_pool(map: &[crate::game::Block], p: &Projectile, a: &Actor) -> bool {
    a.alive()
        && !a.burrowed()
        && (a.pos - p.pos).flat().length() <= POOL_RADIUS
        && (-0.15..=2.5).contains(&(a.pos.y - p.pos.y))
        && clear(
            map,
            p.pos + V3::new(0.0, 0.7, 0.0),
            a.pos + V3::new(0.0, 0.7, 0.0),
        )
}
impl World {
    pub(crate) fn start_vivi(&mut self, source: usize, phase: Phase) {
        let a = &mut self.actors[source];
        a.combat.vivi = State {
            phase,
            elapsed: 0.0,
        };
        a.reload = 0.0;
        a.shield = 0.0;
        a.ability_flash = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        if phase.basic() {
            a.combat.vivi_combo = if phase == Phase::Left { 1 } else { 2 };
            a.combat.attacks = a.combat.attacks.wrapping_add(1);
            a.shot = a.shot.wrapping_add(1);
            a.cooldown = phase.duration() / crate::attack::rate(a);
        } else {
            a.cooldown = 0.0;
        }
        if phase == Phase::Ultimate {
            a.combat.ultimate_flash = ULTIMATE_SECONDS;
            a.vy = 0.0;
        }
    }
    pub(crate) fn advance_vivi(&mut self, source: usize, input: Input) {
        let before = self.actors[source].combat.vivi;
        advance_actor(&mut self.actors[source]);
        let a = &self.actors[source];
        let after = a.combat.vivi;
        if a.character != 0 || !a.alive() || a.combat.action_interrupted() || a.burrowed() {
            return;
        }
        if before.active() && !after.active() {
            if before.phase.basic()
                && input.fire
                && !input.ability
                && !input.ultimate
                && !input.phase_shift
                && !a.combat.disabled()
            {
                self.start_vivi(
                    source,
                    if before.phase == Phase::Left {
                        Phase::Right
                    } else {
                        Phase::Left
                    },
                );
            }
            return;
        }
        if before.phase != after.phase || !after.active() {
            return;
        }
        let crossed = |t| before.elapsed + 0.00001 < t && after.elapsed + 0.00001 >= t;
        if after.phase.basic()
            && crossed(if after.phase == Phase::Left {
                12.0 / 30.0
            } else {
                7.0 / 30.0
            })
        {
            let k = crate::kits::kit(0);
            self.melee(source, k.range, k.damage, 1, false);
        } else if after.phase == Phase::Skill && crossed(SHIELD_RELEASE) {
            // Base HP is intentional: Fashion's temporary maximum must not double this shield.
            self.add_guard(source, SHIELD_HEALTH, SHIELD_SECONDS);
            self.actors[source].combat.mercury = true;
        } else if after.phase == Phase::Ultimate && crossed(LANCE_RELEASE) {
            let a = &self.actors[source];
            let dir = direction(a.yaw, a.pitch);
            let distance = self
                .map
                .iter()
                .filter_map(|b| crate::game::ray_box(a.eye(), dir, b.min(), b.max()))
                .fold(12.0_f32, f32::min)
                .max(0.5);
            let mut center = a.eye() + dir * distance;
            // A forward circle on the nearest supporting floor, including raised platforms.
            center.y = crate::movement::support(&self.map, center, a.eye().y + 2.0);
            // Stop the chosen ground center on the near side of cover.
            let horizontal = (center - a.pos).flat();
            let distance = self
                .map
                .iter()
                .filter_map(|b| {
                    crate::game::ray_box(
                        a.pos + V3::new(0.0, 0.7, 0.0),
                        horizontal.normalized(),
                        b.min(),
                        b.max(),
                    )
                })
                .fold(horizontal.length(), f32::min);
            if distance < horizontal.length() {
                center = a.pos + horizontal.normalized() * (distance - 0.25).max(0.0);
                center.y = crate::movement::support(&self.map, center, a.pos.y + 1.0);
            }
            if let Some(i) = self.projectile(
                source,
                ProjectileKind::MercuryField,
                V3::default(),
                0.0,
                IMPACT_DAMAGE,
                AttackClass::Ultimate,
            ) {
                let p = &mut self.projectiles[i];
                p.pos = center;
                p.origin = center;
                p.life = FIELD_SECONDS;
            }
        }
    }
    pub(crate) fn advance_mercury(&mut self, p: &mut Projectile) {
        if p.life <= 0.00001 {
            return;
        }
        let impact = p.step == 0 && p.age + 0.00001 >= FALL_SECONDS;
        for i in 0..PLAYERS {
            if team(i) == team(p.owner)
                || !self.actors[i].alive()
                || self.actors[i].burrowed()
                || self.in_spawn(i)
            {
                continue;
            }
            let a = &self.actors[i];
            // Swept descending tip catches direct hits before ground impact. Its
            // hit mask also prevents the same target being damaged again by the circle.
            let tip = lance_tip(p.age);
            let previous = lance_tip((p.age - DT).max(0.0));
            let direct = p.step == 0
                && (a.pos - p.pos).flat().length() < 0.58
                && a.pos.y + a.hit_height() >= p.pos.y + tip
                && a.pos.y <= p.pos.y + previous
                && clear(&self.map, p.pos + V3::new(0.0, previous, 0.0), a.eye());
            let inside = in_pool(&self.map, p, a);
            if (direct || impact && inside) && p.hit_mask & (1 << i) == 0 {
                p.hit_mask |= 1 << i;
                self.damage_hit(p.owner, i, p.damage, AttackClass::Ultimate, 0);
            }
            if p.age + 0.00001 >= FALL_SECONDS && inside && self.apply_taunt(p.owner, i, DT * 2.0) {
                self.actors[i].combat.weaken = self.actors[i].combat.weaken.max(DT * 2.0);
            }
        }
        if impact {
            p.step = 1;
        }
    }
}

#[derive(serde::Deserialize)]
pub struct Sample {
    pub hands: [[f32; 3]; 2],
    pub details: Vec<f32>,
    pub blades: [[f32; 3]; 4],
}
#[derive(serde::Deserialize)]
struct Clip {
    samples: Vec<Sample>,
}
#[derive(serde::Deserialize)]
struct Data {
    clips: Vec<Clip>,
}
pub fn sample(motion: Motion, age: f32) -> (&'static Sample, &'static Sample, f32) {
    static DATA: std::sync::OnceLock<Data> = std::sync::OnceLock::new();
    let d = DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/characters/skill/vivi.json"))
            .expect("Vivi archived tracks")
    });
    let (clip, time) = match motion {
        Motion::ViviLeft => (1, age),
        Motion::ViviRight => (1, age + LEFT_SECONDS),
        Motion::ViviSkill => (2, age),
        Motion::ViviUltimate => (3, age),
        _ => (0, age.rem_euclid(40.0 / 30.0)),
    };
    let s = &d.clips[clip].samples;
    let f = (time * 30.0).clamp(0.0, (s.len() - 1) as f32);
    let i = f as usize;
    (&s[i], &s[(i + 1).min(s.len() - 1)], f.fract())
}
/// Explicit shoulder-crossing arc, with the same three timing landmarks in the bake.
pub fn raised_hand(age: f32) -> [f32; 3] {
    let knots = [
        (0.0, [-0.52, -0.38, 0.72]),
        (0.28, [0.40, -0.25, 0.72]),
        (0.55, [0.42, -0.22, 0.72]),
        (0.75, [-0.42, 0.33, 1.02]),
        (1.03, [-0.44, 0.32, 1.04]),
        (SKILL_SECONDS, [-0.52, -0.38, 0.72]),
    ];
    for pair in knots.windows(2) {
        if age <= pair[1].0 {
            let t = smooth((age - pair[0].0) / (pair[1].0 - pair[0].0));
            return std::array::from_fn(|j| pair[0].1[j] + (pair[1].1[j] - pair[0].1[j]) * t);
        }
    }
    knots[5].1
}
pub fn hand(motion: Motion, age: f32, left: bool) -> crate::attack::HandMotion {
    let side = if left { 1.0 } else { -1.0 };
    if motion == Motion::ViviSkill {
        return if left {
            Default::default()
        } else {
            let p = raised_hand(age);
            crate::attack::HandMotion {
                position: [p[0] + 0.52, p[1] + 0.38, p[2] - 0.72],
                angles: [
                    -0.20 * smooth(age / 0.4) * (1.0 - smooth((age - 1.0) / 0.46)),
                    0.0,
                    -0.20 * smooth(age / 0.4) * (1.0 - smooth((age - 1.0) / 0.46)),
                ],
            }
        };
    }
    if !matches!(
        motion,
        Motion::ViviLeft | Motion::ViviRight | Motion::ViviUltimate
    ) {
        return Default::default();
    }
    let (a, b, t) = sample(motion, age);
    let j = if left { 0 } else { 1 };
    let p: [f32; 3] = std::array::from_fn(|i| a.hands[j][i] + (b.hands[j][i] - a.hands[j][i]) * t);
    let duration = motion.seconds(0);
    let w = smooth(age / 0.10) * smooth((duration - age) / 0.15);
    let sweep = if motion == Motion::ViviLeft {
        1.0
    } else {
        -1.0
    };
    crate::attack::HandMotion {
        position: [
            (p[0] * 0.24
                + if motion != Motion::ViviUltimate {
                    sweep * (0.5 - smooth(age / (duration * 0.62))) * 0.38
                } else {
                    0.0
                })
            .clamp(-0.30, 0.36)
                * w,
            (p[1] * 0.36).clamp(-0.15, 0.55) * w,
            (-p[0] * 0.14).clamp(-0.10, 0.2) * w,
        ],
        angles: [
            (-p[2] * 0.3).clamp(-0.8, 0.8) * w,
            side * p[0].clamp(-0.4, 0.4) * w,
            side * p[2].clamp(-0.55, 0.55) * w,
        ],
    }
}

/// Four blade pivots in character-local metres; +X is the caster's left.
pub fn blade(index: usize, motion: Motion, age: f32, time: f32) -> ([f32; 3], [f32; 3]) {
    let side = if index < 2 { -1.0 } else { 1.0 };
    let outer = index % 2 == 1;
    let mut p = [
        side * if outer { 0.78 } else { 0.42 },
        if outer { 1.10 } else { 1.34 },
        -0.38 - if outer { 0.08 } else { 0.0 },
    ];
    let mut angles = [-0.12, 0.0, -side * if outer { 0.58 } else { 0.27 }];
    let bob = (time * std::f32::consts::TAU / (40.0 / 30.0) + index as f32 * 0.45).sin() * 0.025;
    p[1] += bob;
    if matches!(motion, Motion::ViviLeft | Motion::ViviRight) {
        let duration = motion.seconds(0);
        let phase = (age / duration).clamp(0.0, 1.0);
        let weight = smooth(phase / 0.18) * smooth((1.0 - phase) / 0.25);
        let reverse = if motion == Motion::ViviRight {
            -1.0
        } else {
            1.0
        };
        let progress = smooth((phase - 0.12 - index as f32 * 0.018) / 0.62);
        let angle = reverse * (1.20 - 2.40 * progress);
        let target = [
            angle.sin() * 1.8,
            1.18 + (index as f32 - 1.5) * 0.13,
            1.6 + angle.cos() * 0.6,
        ];
        for j in 0..3 {
            p[j] += (target[j] - p[j]) * weight;
        }
        angles[0] += (std::f32::consts::FRAC_PI_2 - angles[0]) * weight;
        angles[1] = angle * weight;
        angles[2] *= 1.0 - weight;
    } else if motion == Motion::ViviSkill {
        let w = smooth((age - 0.45) / 0.20) * (1.0 - smooth((age - 1.10) / 0.36));
        let down = if outer { -1.0 } else { 1.0 };
        let target = [side * 0.53, 1.05 + down * 0.32, -0.46];
        for j in 0..3 {
            p[j] += (target[j] - p[j]) * w;
        }
        angles[2] += (-side * if outer { 2.25 } else { 0.85 } - angles[2]) * w;
        p[1] += skill_lift(age);
    }
    (p, angles)
}
