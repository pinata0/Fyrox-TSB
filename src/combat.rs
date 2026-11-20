//! Server-authoritative character weapons, skills, projectiles and ultimates.
use crate::{
    game::*,
    kits::kit,
    roster::{character, Personality, Race},
};

pub const MAX_PROJECTILES: usize = 96;
pub const ULT_PER_SECOND: f32 = 1.25;
pub const ULT_PER_DAMAGE: f32 = 0.12;
pub const HILDE_SYRINGE_HEAL: f32 = 8.0;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackClass {
    Basic,
    Skill,
    Ultimate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    Needles,
    Drain,
}
#[derive(Debug, Clone, Copy)]
pub struct Cast {
    pub kind: CastKind,
    pub left: u8,
    pub step: u8,
    pub timer: f32,
    pub interval: f32,
    pub dir: V3,
    pub origin: V3,
    pub mask: u16,
    pub target: Option<usize>,
    pub switched: bool,
    pub class: AttackClass,
}
impl Cast {
    pub fn blocking(self) -> bool {
        matches!(
            self.kind,
            CastKind::Needles
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct DamageOverTime {
    pub tick: f32,
    pub owner: usize,
    pub character: u8,
    pub class: Option<AttackClass>,
}

#[derive(Debug, Clone, Default)]
pub struct CombatState {
    pub chloe: crate::chloe_skill::State,
    pub vivi: crate::vivi_skill::State,
    pub erpin: crate::erpin::State,
    pub erpin_skill: crate::erpin_skill::State,
    pub naia: crate::naia_skill::State,
    pub id: crate::id_skill::State,
    pub renua: crate::renua_skill::State,
    pub hilde: crate::hilde_skill::State,
    pub shion: crate::shion_skill::State,
    pub aya: crate::aya_skill::State,
    pub ui: crate::ui_skill::State,
    pub picora: crate::picora_skill::State,
    pub sticker: f32,
    pub mayo: crate::mayo_skill::State,
    pub bella: crate::bella_skill::State,
    pub komi: crate::komi_skill::State,
    pub butter: crate::butter_skill::State,
    /// Frog form blocks actions but leaves walking and looking available.
    pub polymorph: f32,
    /// Independent 15% rain slow; other slows retain their own strength.
    pub rain_slow: f32,
    /// Ice prison: arrests motion, view and every gameplay input.
    pub frozen: f32,
    /// Continuous exposure to a temporal field: zero is free, one is frozen.
    pub temporal: f32,
    pub ultimate: f32,
    pub q_down: bool,
    pub ultimate_flash: f32,
    pub guard: f32,
    pub guard_time: f32,
    pub mercury: bool,
    pub stun: f32,
    pub slow: f32,
    pub poison: f32,
    pub frost: f32,
    pub dots: [DamageOverTime; 2], // Poison and frost keep independent attribution.
    pub weaken: f32,
    pub skill_weaken: f32,
    pub physical_weaken: f32,
    pub vulnerable: f32,
    pub haste: f32,
    pub speed_up: f32,
    pub damage_up: f32,
    pub skill_up: f32,
    pub resist: f32,
    pub physical_resist: f32,
    pub sleep: f32,
    pub giant: f32,
    pub empowered: f32,
    pub empower_stacks: u8,
    pub extended: bool,
    pub regen: f32,
    pub regen_rate: f32,
    pub regen_source: Option<usize>,
    pub regen_tick: f32,
    pub healing_up: f32,
    pub ward: f32,
    pub taunt: f32,
    pub taunt_source: usize,
    pub blind: f32,
    pub fashion: f32,
    pub aura_tick: f32,
    pub revive_armed: bool,
    pub revive_used: bool,
    pub attacks: u32,
    /// Last Vivi slash direction: 1 left-to-right, 2 right-to-left.
    pub vivi_combo: u8,
    /// Last Aya basic attack stage, retained for observers and replay.
    pub aya_combo: u8,
    pub incoming: u8,
    pub retaliate: bool,
    pub magic_ammo: u8,
    pub cast: Option<Cast>,
    pub dash: f32,
    pub dash_dir: V3,
    pub dash_mask: u16,
}
impl CombatState {
    /// Time fields pause a gesture; hard crowd control cancels it.
    pub fn action_interrupted(&self) -> bool {
        self.stun > 0.0 || self.sleep > 0.0 || self.frozen > 0.0 || self.polymorph > 0.0
    }
    /// Taunt locks skills without disabling walking, aiming, reload or basic attacks.
    pub fn skills_locked(&self) -> bool { self.disabled() || self.taunt > 0.0 }
    pub fn disabled(&self) -> bool {
        self.movement_disabled() || self.polymorph > 0.0
    }
    pub fn movement_disabled(&self) -> bool {
        self.stun > 0.0 || self.sleep > 0.0 || self.view_locked()
    }
    pub fn view_locked(&self) -> bool { self.frozen > 0.0 || self.temporal >= 1.0 }
    pub fn channeling(&self) -> bool {
        self.chloe.phase.basic() || self.channeling_blocks_surge()
    }
    /// Fairy/witch movement can overlap Chloe's normal and mounted swings.
    /// Every other action keeps its usual channeling restriction.
    pub fn channeling_blocks_surge(&self) -> bool {
        self.chloe.locked() || self.vivi.active()
            || self.erpin.eating
            || self.erpin_skill.active()
            || self.naia.active()
            || self.id.active()
            || self.renua.active()
            || self.hilde.active()
            || self.shion.active()
            || self.aya.active()
            || self.ui.active()
            || self.picora.active()
            || self.mayo.active()
            || self.bella.active()
            || self.komi.locked()
            || self.butter.active()
            || self.cast.is_some_and(Cast::blocking)
            || self.dash > 0.0
    }
    pub fn flags(&self) -> u16 {
        let values = [
            self.stun,
            self.slow,
            self.poison,
            self.frost,
            self.haste,
            self.empowered,
            self.giant,
            self.sleep,
            self.weaken.max(self.skill_weaken).max(self.physical_weaken),
            self.taunt,
            self.ward,
            self.resist.max(self.physical_resist).max(self.sticker),
            self.damage_up.max(self.skill_up),
            self.blind,
            self.ultimate_flash,
            self.speed_up,
        ];
        values
            .into_iter()
            .enumerate()
            .fold(0, |bits, (i, v)| bits | ((v > 0.0) as u16) << i)
    }
    pub fn read_flags(&mut self, flags: u16) {
        let timers = [
            &mut self.stun,
            &mut self.slow,
            &mut self.poison,
            &mut self.frost,
            &mut self.haste,
            &mut self.empowered,
            &mut self.giant,
            &mut self.sleep,
            &mut self.weaken,
            &mut self.taunt,
            &mut self.ward,
            &mut self.resist,
            &mut self.damage_up,
            &mut self.blind,
            &mut self.ultimate_flash,
            &mut self.speed_up,
        ];
        for (i, t) in timers.into_iter().enumerate() {
            *t = if flags & (1 << i) != 0 { 1.0 } else { 0.0 };
        }
    }
    pub fn status_text(&self) -> &'static str {
        if self.temporal >= 1.0 {
            "TIME STOP"
        } else if self.temporal > 0.0 {
            "TIME SLOW"
        } else if self.frozen > 0.0 {
            "FROSTBITE / FROZEN"
        } else if self.weaken > 0.0 {
            "WEAKENED / DAMAGE -20%"
        } else if self.polymorph > 0.0 {
            "FROG / MOVEMENT ONLY"
        } else if self.stun > 0.0 {
            "STUN"
        } else if self.sleep > 0.0 {
            "SLEEPING / DEBUFF IMMUNE"
        } else if self.giant > 0.0 {
            "GIANT FORM"
        } else if self.empowered > 0.0 {
            "DOLL WILL"
        } else if self.poison > 0.0 {
            "POISONED"
        } else if self.frost > 0.0 {
            "FROSTBITE"
        } else if self.rain_slow > 0.0 {
            "RAIN / SPEED -15%"
        } else if self.slow > 0.0 {
            "SLOWED"
        } else if self.taunt > 0.0 {
            "TAUNTED / SKILLS LOCKED"
        } else if self.ward > 0.0 {
            "HP FLOOR 10%"
        } else if self.haste > 0.0 {
            "ATTACK HASTE"
        } else if self.blind > 0.0 {
            "BLINDED / AIM DISRUPTED"
        } else {
            ""
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum ProjectileKind {
    #[default]
    Bolt,
    Water,
    Stone,
    Needle,
    Syringe,
    Butterfly,
    Spear,
    Wave,
    Doll,
    Boulder,
    DarkOrb,
    Ricochet,
    Flame,
    Echo,
    Poison,
    MagicBomb,
    Cake4,
    Cake3,
    Cake2,
    Cake1,
    RenuaOrb,
    TimeField,
    SnowGarden,
    MedicalGrenade,
    RainField,
    BoulderBlast,
    BellaFire,
    BellaHeal,
    ErpinBlast,
    ErpinCrash,
    MercuryField,
    Confetti,
}
impl ProjectileKind {
    pub fn cake(remaining: u8) -> Self {
        match remaining {
            1 => Self::Cake1,
            2 => Self::Cake2,
            3 => Self::Cake3,
            _ => Self::Cake4,
        }
    }
    pub fn cake_remaining(self) -> Option<u8> {
        match self {
            Self::Cake4 => Some(4),
            Self::Cake3 => Some(3),
            Self::Cake2 => Some(2),
            Self::Cake1 => Some(1),
            _ => None,
        }
    }
    pub fn from_wire(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Bolt,
            1 => Self::Water,
            2 => Self::Stone,
            3 => Self::Needle,
            4 => Self::Syringe,
            5 => Self::Butterfly,
            6 => Self::Spear,
            7 => Self::Wave,
            8 => Self::Doll,
            9 => Self::Boulder,
            10 => Self::DarkOrb,
            11 => Self::Ricochet,
            12 => Self::Flame,
            13 => Self::Echo,
            14 => Self::Poison,
            15 => Self::MagicBomb,
            16 => Self::Cake4,
            17 => Self::Cake3,
            18 => Self::Cake2,
            19 => Self::Cake1,
            20 => Self::RenuaOrb,
            21 => Self::TimeField,
            22 => Self::SnowGarden,
            23 => Self::MedicalGrenade,
            24 => Self::RainField,
            25 => Self::BoulderBlast,
            26 => Self::BellaFire,
            27 => Self::BellaHeal,
            28 => Self::ErpinBlast,
            29 => Self::ErpinCrash,
            30 => Self::MercuryField,
            31 => Self::Confetti,
            _ => return None,
        })
    }
    pub fn radius(self) -> f32 {
        match self {
            Self::Boulder => crate::butter_skill::BOULDER_RADIUS,
            Self::Wave => 1.7,
            Self::Flame => 0.6,
            Self::Doll => 0.3,
            Self::DarkOrb | Self::MagicBomb => 0.3,
            Self::Butterfly => 0.35,
            _ => 0.12,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Projectile {
    pub id: u32,
    pub owner: usize,
    pub character: u8,
    pub kind: ProjectileKind,
    pub pos: V3,
    pub velocity: V3,
    pub origin: V3,
    pub life: f32,
    pub age: f32,
    pub damage: f32,
    pub class: AttackClass,
    pub hit_mask: u16,
    pub special_mask: u16,
    pub returning: bool,
    pub bounces: u8,
    pub step: u8,
}
impl Projectile {
    pub fn visual(id: u32, owner: usize, kind: ProjectileKind, pos: V3) -> Self {
        Self {
            id,
            owner,
            character: 0,
            kind,
            pos,
            velocity: V3::default(),
            origin: pos,
            life: 0.0,
            age: 0.0,
            damage: 0.0,
            class: AttackClass::Basic,
            hit_mask: 0,
            special_mask: 0,
            returning: false,
            bounces: 0,
            step: 0,
        }
    }
}

impl World {
    pub fn clear_combat_on_spawn(&mut self, i: usize) {
        let old = &self.actors[i].combat;
        self.actors[i].combat = CombatState {
            ultimate: old.ultimate,
            q_down: old.q_down,
            revive_used: old.revive_used,
            ..Default::default()
        };
    }
    pub(crate) fn visible_between(&self, from: V3, to: V3) -> bool {
        let d = to - from;
        let len = d.length();
        len < 0.001
            || !self.map.iter().any(|b| {
                ray_box(from, d.normalized(), b.min(), b.max()).is_some_and(|t| t < len - 0.02)
            })
    }
    pub(crate) fn targets(
        &self,
        source: usize,
        origin: V3,
        radius: f32,
        allies: bool,
    ) -> Vec<usize> {
        self.actors
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                a.alive()
                    && !a.burrowed()
                    && (team(*i) == team(source)) == allies
                    && (allies || !self.in_spawn(*i))
                    && (a.eye() - origin).length() <= radius
                    && self.visible_between(origin, a.eye())
            })
            .map(|(i, _)| i)
            .collect()
    }
    fn wounded(&self, source: usize, range: f32) -> Vec<usize> {
        let mut targets = self.targets(source, self.actors[source].eye(), range, true);
        targets.sort_by_key(|&i| (self.actors[i].hp, i));
        targets
    }
    pub(crate) fn add_guard(&mut self, i: usize, amount: f32, time: f32) {
        let c = &mut self.actors[i].combat;
        c.guard = c.guard.max(amount).min(120.0);
        c.guard_time = c.guard_time.max(time);
    }
    pub(crate) fn cleanse(&mut self, i: usize, stun: bool) {
        let c = &mut self.actors[i].combat;
        c.poison = 0.0;
        c.frost = 0.0;
        c.frozen = 0.0;
        c.slow = 0.0;
        c.rain_slow = 0.0;
        c.taunt = 0.0;
        c.polymorph = 0.0;
        c.weaken = 0.0;
        c.skill_weaken = 0.0;
        c.physical_weaken = 0.0;
        c.vulnerable = 0.0;
        c.blind = 0.0;
        if stun {
            c.stun = 0.0;
        }
    }
    pub(crate) fn heal(&mut self, i: usize, amount: f32) {
        let a = &mut self.actors[i];
        if !a.alive() {
            return;
        }
        let factor = if character(a.character).race == Race::Fairy {
            1.1
        } else {
            1.0
        } * if a.combat.healing_up > 0.0 { 1.2 } else { 1.0 };
        a.hp =
            a.hp.saturating_add((amount * factor).round() as u16)
                .min(a.max_hp());
    }
    pub(crate) fn push(&mut self, target: usize, dir: V3, distance: f32) {
        if crate::erpin_skill::unstoppable(&self.actors[target]) { return; }
        if self.actors[target].shield > 0.0
            || !self.actors[target].alive()
            || self.actors[target].burrowed()
            || self.actors[target].combat.komi.resting()
            || self.actors[target].combat.butter.hovering()
            || self.actors[target].combat.frozen > 0.0
            || self.in_spawn(target)
        {
            return;
        }
        let factor = if character(self.actors[target].character).race == Race::Dragon {
            0.7
        } else {
            1.0
        };
        let step = dir.flat().normalized() * (distance * factor / 40.0);
        for _ in 0..40 {
            let next = self.actors[target].pos + step;
            if crate::movement::blocked_with_height(
                &self.map,
                next,
                self.actors[target].collision_height(),
            ) {
                break;
            }
            self.actors[target].pos = next;
        }
    }
    fn taunt(&mut self, source: usize, target: usize, seconds: f32) {
        self.apply_taunt(source, target, seconds);
    }
    pub(crate) fn apply_taunt(&mut self, source: usize, target: usize, seconds: f32) -> bool {
        if self.actors[target].alive() && self.actors[target].shield <= 0.0
            && !self.actors[target].burrowed()
            && self.actors[target].combat.sleep <= 0.0 && !self.actors[target].combat.komi.resting()
            && !self.in_spawn(target)
        {
            self.actors[target].combat.taunt = self.actors[target].combat.taunt.max(seconds);
            self.actors[target].combat.taunt_source = source;
            return true;
        }
        false
    }
    fn dot(&mut self, source: usize, target: usize, frost: bool, seconds: f32, class: AttackClass) {
        if self.in_spawn(target) || self.actors[target].burrowed() {
            return;
        }
        let id = self.actors[source].character;
        let c = &mut self.actors[target].combat;
        if c.sleep > 0.0 || c.komi.resting() {
            return;
        }
        let active = if frost { c.frost > 0.0 } else { c.poison > 0.0 };
        if frost {
            c.frost = c.frost.max(seconds);
        } else {
            c.poison = c.poison.max(seconds);
        }
        let dot = &mut c.dots[frost as usize];
        dot.owner = source;
        dot.character = id;
        dot.class = Some(class);
        if !active {
            dot.tick = 1.0;
        }
    }
    pub(crate) fn damage_hit(
        &mut self,
        source: usize,
        target: usize,
        base: f32,
        class: AttackClass,
        source_character: u8,
    ) -> bool {
        if source >= PLAYERS
            || target >= PLAYERS
            || team(source) == team(target)
            || !self.actors[target].alive()
            || self.actors[target].shield > 0.0
            || self.actors[target].combat.id.invulnerable > 0.0
            || self.actors[target].combat.komi.resting()
            || self.actors[target].burrowed()
            || self.in_spawn(target)
        {
            return false;
        }
        let s = &self.actors[source].combat;
        let t = &self.actors[target].combat;
        let physical = matches!(source_character, 7 | 8 | 11 | 12 | 14);
        let mut factor = self.damage_multiplier(team(source));
        if s.damage_up > 0.0 {
            factor *= 1.15;
        }
        if s.giant > 0.0 {
            factor *= 1.15;
        }
        if source_character == 14 && class == AttackClass::Basic {
            factor *= 1.0 + s.magic_ammo as f32 * 0.05;
        }
        if class == AttackClass::Basic && s.empowered > 0.0 {
            factor *= 1.0 + s.empower_stacks as f32 * 0.02;
        }
        if s.weaken > 0.0 || s.poison > 0.0 {
            factor *= crate::mayo_skill::POISON_DAMAGE_FACTOR;
        }
        if physical && s.physical_weaken > 0.0 {
            factor *= 0.8;
        }
        if class != AttackClass::Basic {
            if s.skill_weaken > 0.0 {
                factor *= 0.75;
            }
            if s.skill_up > 0.0 {
                factor *= 1.25;
            }
        }
        if source_character == 3 && class == AttackClass::Ultimate {
            // Fixed-health damage ignores attack buffs; poison and general weaken still apply.
            factor = if s.weaken > 0.0 || s.poison > 0.0 { crate::mayo_skill::POISON_DAMAGE_FACTOR } else { 1.0 };
        }
        if t.id.phase == crate::id_skill::Phase::Ultimate {
            factor *= 0.7;
        }
        if t.resist > 0.0 || t.sticker > 0.0 {
            factor *= crate::picora_skill::DAMAGE_FACTOR;
        }
        if physical && t.physical_resist > 0.0 {
            factor *= 0.8;
        }
        if t.vulnerable > 0.0 {
            factor *= 1.15;
        }
        if t.frost > 0.0 && character(source_character).personality == Personality::Cool {
            factor *= 1.15;
        }
        let execution = source_character == 14 && class == AttackClass::Ultimate;
        let mut damage = if execution {
            self.actors[target].hp as f32 + self.actors[target].shield_health()
        } else {
            (base * factor).round().max(0.0)
        };
        let a = &mut self.actors[target];
        let before_hp = a.hp;
        let before_barrier = a.shield_health();
        let incoming = damage;
        // Apply once per hit after damage modifiers, before either shield absorbs it.
        // A shield present at impact also halves that hit's overflow into HP.
        if !execution && before_barrier > 0.0 && damage > 10.0 {
            damage *= 0.5;
        }
        let absorbed = a.combat.guard.min(damage);
        a.combat.guard -= absorbed;
        damage -= absorbed;
        let absorbed = a.barrier.min(damage);
        a.barrier -= absorbed;
        damage -= absorbed;
        let mercury_break = a.combat.mercury && a.combat.guard <= 0.0;
        if mercury_break {
            a.combat.mercury = false;
        }
        if !execution && a.combat.ward > 0.0 {
            damage = damage.min(a.hp.saturating_sub(a.max_hp().div_ceil(10)) as f32);
        }
        // Shield absorption, shield reduction and a ward save protect this actor.
        a.stats.mitigated += (incoming - damage).max(0.0);
        a.hp = a.hp.saturating_sub(damage.ceil() as u16);
        let actual = (before_hp - a.hp) as f32 + (before_barrier - a.shield_health()).max(0.0);
        a.hurt = 0.35;
        a.dancing = false;
        a.wave_ticks = 0;
        if a.character == 9 && actual > 0.0 {
            a.combat.incoming = a.combat.incoming.saturating_add(1);
            let threshold = 4;
            if a.combat.incoming >= threshold {
                a.combat.incoming = 0;
                a.combat.retaliate = true;
            }
        }
        let revive =
            !execution && !a.alive() && a.character == 9 && a.combat.revive_armed && !a.combat.revive_used;
        if revive {
            a.hp = 60;
            a.combat.revive_used = true;
            a.combat.revive_armed = false;
            a.combat.stun = 0.4;
        }
        let killed = !a.alive();
        if killed {
            a.deaths = a.deaths.saturating_add(1);
            a.respawn = 7.0;
            a.killed_by = Some(source);
            a.ghost_phase = 0.0;
            a.phase_skill = Default::default();
            a.race_skill = Default::default();
            a.combat.cast = None;
            a.combat.id = Default::default();
            a.combat.shion = Default::default();
            a.combat.aya = Default::default();
            a.combat.frozen = 0.0;
            a.combat.picora = Default::default();
            a.combat.sticker = 0.0;
            a.combat.fashion = 0.0;
            a.combat.dash = 0.0;
        }
        self.record_damage(source, target, actual, killed);
        if revive && self.actors[source].combat.sleep <= 0.0 {
            self.actors[source].combat.stun = 0.6;
        }
        if killed {
            self.actors[source].kills = self.actors[source].kills.saturating_add(1);
            self.event = format!(
                "{} > {}",
                character(source_character).name,
                character(self.actors[target].character).name
            );
        }
        self.actors[source].hit = 0.18;
        if class != AttackClass::Ultimate
            && self.actors[source].alive()
            && self.actors[source].character == source_character
        {
            self.actors[source].combat.ultimate =
                (self.actors[source].combat.ultimate + actual * ULT_PER_DAMAGE).min(100.0);
        }
        if mercury_break && self.actors[target].alive() {
            self.mercury_burst(target);
        }
        true
    }
    pub(crate) fn burst(
        &mut self,
        source: usize,
        origin: V3,
        radius: f32,
        damage: f32,
        class: AttackClass,
        push: f32,
    ) -> Vec<usize> {
        let id = self.actors[source].character;
        let targets = self.targets(source, origin, radius, false);
        let mut hit = vec![];
        for i in targets {
            if self.damage_hit(source, i, damage, class, id) {
                self.push(i, self.actors[i].eye() - origin, push);
                hit.push(i);
            }
        }
        self.actors[source].ability_flash = 0.4;
        hit
    }
    fn mercury_burst(&mut self, source: usize) {
        for i in self.burst(
            source,
            self.actors[source].eye(),
            5.0,
            22.0,
            AttackClass::Skill,
            0.0,
        ) {
            if self.actors[i].combat.sleep <= 0.0 && !self.actors[i].combat.komi.resting() {
                self.actors[i].combat.vulnerable = 4.0;
            }
        }
    }
    pub(crate) fn melee(&mut self, source: usize, range: f32, damage: f32, hits: u8, enhanced: bool) {
        let a = &self.actors[source];
        let origin = a.eye();
        let dir = direction(a.yaw, a.pitch);
        let id = a.character;
        let targets = self.targets(source, origin, range, false);
        let mut connected = false;
        for i in targets {
            let diff = self.actors[i].eye() - origin;
            if diff.normalized().dot(dir) < 0.5 {
                continue;
            }
            let mut hit_target = false;
            for _ in 0..hits {
                hit_target |=
                    self.damage_hit(source, i, damage / hits as f32, AttackClass::Basic, id);
            }
            connected |= hit_target;
            if hit_target
                && enhanced
                && self.actors[i].alive()
                && self.actors[i].combat.sleep <= 0.0 && !self.actors[i].combat.komi.resting()
            {
                match id {
                    4 => self.actors[i].combat.slow = 2.0,
                    6 => {
                        self.push(i, dir, 1.3);
                        self.taunt(source, i, 2.0);
                    }
                    12 => self.actors[i].combat.stun = 0.35,
                    _ => {}
                }
            }
        }
        if id == 0 && connected {
            self.heal_from(source, source, 5.0);
        }
        self.actors[source].shot_end = origin + dir * range;
        let a = &self.actors[source];
        if let Some(hit) = crate::impact::melee_contact(&self.map, origin, a.yaw, a.pitch, range) {
            let tilt = if (id == 0 && a.combat.vivi_combo == 2) || (id != 0 && a.combat.attacks.is_multiple_of(2)) {
                -0.55
            } else {
                0.55
            };
            let rotation = crate::impact::slash_rotation(hit.face, a.yaw, a.pitch, tilt);
            let half_length = 0.48 * a.body_scale();
            self.record_slash(hit, half_length, rotation);
        }
    }
    pub(crate) fn ray(
        &mut self,
        source: usize,
        range: f32,
        damage: f32,
        hits: u8,
        class: AttackClass,
        pierce: bool,
    ) -> Option<usize> {
        let a = &self.actors[source];
        let origin = a.eye();
        let mut dir = direction(a.yaw, a.pitch);
        let id = a.character;
        if a.combat.blind > 0.0 {
            dir = direction(a.yaw + (self.tick as f32 * 1.7).sin() * 0.12, a.pitch);
        }
        let mut distance = range;
        for b in &self.map {
            if let Some(t) = ray_box(origin, dir, b.min(), b.max()) {
                distance = distance.min(t);
            }
        }
        let surface = crate::impact::trace(&self.map, origin, dir, range, 0.0)
            .filter(|hit| hit.distance <= distance + 0.002);
        if let Some(hit) = surface {
            distance = distance.min(hit.distance);
        }
        let mut targets: Vec<_> = self
            .actors
            .iter()
            .enumerate()
            .filter(|(i, a)| *i != source && a.alive() && !a.burrowed())
            .filter_map(|(i, a)| {
                crate::chloe_skill::ray_hit(a, origin, dir, 0.0)
                .filter(|t| *t < distance)
                .map(|t| (i, t))
            })
            .collect();
        targets.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut first = None;
        for (i, t) in targets {
            if first.is_none() {
                first = Some(i);
                distance = t;
            }
            for _ in 0..hits {
                let multiplier=if class==AttackClass::Basic && crate::chloe_skill::headshot(&self.actors[i],origin,dir,t,0.0){1.5}else{1.0};
                self.damage_hit(source, i, damage * multiplier / hits as f32, class, id);
            }
            if !pierce {
                break;
            }
        }
        self.actors[source].shot_end = origin + dir * distance;
        if first.is_none() || pierce {
            if let Some(hit) = surface {
                self.record_impact(hit, 0.13);
            }
        }
        first
    }
    pub(crate) fn projectile(
        &mut self,
        source: usize,
        kind: ProjectileKind,
        dir: V3,
        speed: f32,
        damage: f32,
        class: AttackClass,
    ) -> Option<usize> {
        if self.projectiles.len() >= MAX_PROJECTILES {
            return None;
        }
        self.next_projectile = self.next_projectile.wrapping_add(1);
        let origin = self.actors[source].eye();
        self.projectiles.push(Projectile {
            id: self.next_projectile,
            owner: source,
            character: self.actors[source].character,
            kind,
            pos: origin,
            origin,
            velocity: dir.normalized() * speed,
            life: if class == AttackClass::Basic {
                kit(self.actors[source].character).range / speed.max(0.01)
            } else {
                2.7
            },
            age: 0.0,
            damage,
            class,
            hit_mask: 0,
            special_mask: 0,
            returning: false,
            bounces: if kind == ProjectileKind::Ricochet {
                5
            } else {
                0
            },
            step: 0,
        });
        self.actors[source].shot_end = origin + dir * 3.0;
        Some(self.projectiles.len() - 1)
    }
    pub(crate) fn primary_attack(&mut self, source: usize) {
        let a = &self.actors[source];
        let k = kit(a.character);
        if a.cooldown > 0.0
            || a.reload > 0.0
            || a.combat.disabled()
            || a.combat.channeling()
            || a.burrowed()
            || a.combat.komi.active()
            || a.race_skill.fire_latched
        {
            return;
        }
        if k.magazine > 0 && a.ammo == 0 {
            self.reload(source);
            return;
        }
        if a.character == 6 { self.start_chloe(source, false); return; }
        if a.character == 7 {
            self.start_renua(source, crate::renua_skill::Phase::Basic);
            return;
        }
        if a.character == 0 {
            self.start_vivi(source, crate::vivi_skill::Phase::Left);
            return;
        }
        let id = a.character;
        let dir = direction(a.yaw, a.pitch);
        let a = &mut self.actors[source];
        a.combat.attacks = a.combat.attacks.wrapping_add(1);
        let n = a.combat.attacks;
        let boosted = a.combat.empowered > 0.0;
        if k.magazine > 0 {
            a.ammo -= 1;
        }
        a.shield = 0.0;
        a.shot = a.shot.wrapping_add(1);
        a.cooldown = k.interval
            * if a.combat.haste > 0.0 { 0.75 } else { 1.0 }
            * if a.combat.giant > 0.0 { 0.62 } else { 1.0 }
            * if a.combat.slow > 0.0 { 1.25 } else { 1.0 };
        if boosted {
            a.combat.empower_stacks = (a.combat.empower_stacks + 1).min(9);
            if a.combat.empower_stacks == 9 && !a.combat.extended {
                a.combat.empowered += 4.0;
                a.combat.extended = true;
            }
        }
        match id {
            0 => unreachable!("Vivi attacks use the source action clock"),
            1 => {
                let count = if n.is_multiple_of(4) { 3 } else { 1 };
                for _ in 0..count {
                    self.projectile(
                        source,
                        ProjectileKind::Water,
                        dir,
                        32.0,
                        if count == 3 { 7.0 } else { k.damage },
                        AttackClass::Basic,
                    );
                }
            }
            2 => {
                self.actors[source].combat.erpin.shot_delay =
                    crate::erpin::shot_delay() / crate::attack::rate(&self.actors[source]);
            }
            3 => {
                self.ray(source, k.range, k.damage, 1, AttackClass::Basic, false);
            }
            4 => {
                self.actors[source].combat.aya_combo = if n.is_multiple_of(3) {
                    3
                } else {
                    (n % 3) as u8
                };
                self.melee(
                    source,
                    k.range,
                    if n.is_multiple_of(3) { 36.0 } else { k.damage },
                    2,
                    n.is_multiple_of(3),
                );
            }
            5 => {
                self.actors[source].combat.picora.shot_delay = crate::picora_skill::shot_delay(&self.actors[source]);
            }
            6 => unreachable!("Chloe uses delayed source action"),
            8 => {
                self.projectile(
                    source,
                    if n.is_multiple_of(4) {
                        ProjectileKind::Poison
                    } else {
                        ProjectileKind::Needle
                    },
                    dir,
                    48.0,
                    k.damage,
                    AttackClass::Basic,
                );
            }
            9 => self.melee(source, k.range, k.damage, 2, false),
            10 => {
                if n.is_multiple_of(4) {
                    self.ray(source, 8.0, 22.0, 1, AttackClass::Basic, false);
                    for i in self.targets(source, self.actors[source].eye(), 7.0, true) {
                        self.actors[i].ability_cooldown =
                            (self.actors[i].ability_cooldown - 1.0).max(0.0);
                        self.cleanse(i, false);
                    }
                } else {
                    self.projectile(
                        source,
                        ProjectileKind::Bolt,
                        dir,
                        26.0,
                        k.damage,
                        AttackClass::Basic,
                    );
                }
            }
            11 => {
                self.projectile(
                    source,
                    ProjectileKind::Stone,
                    dir,
                    26.0,
                    if n.is_multiple_of(4) { 28.0 } else { k.damage },
                    AttackClass::Basic,
                );
            }
            12 => self.melee(
                source,
                k.range * self.actors[source].body_scale(),
                k.damage,
                1,
                n.is_multiple_of(4),
            ),
            13 => {
                let target = if n.is_multiple_of(3) {
                    self.wounded(source, 14.0)
                        .into_iter()
                        .find(|&i| self.actors[i].hp < self.actors[i].max_hp())
                } else {
                    None
                };
                if let Some(i) = target {
                    self.heal_from(source, i, 24.0);
                    self.actors[source].shot_end = self.actors[i].eye();
                    self.actors[source].ability_flash = 0.4;
                } else {
                    self.projectile(
                        source,
                        ProjectileKind::Syringe,
                        dir,
                        45.0,
                        k.damage,
                        AttackClass::Basic,
                    );
                }
            }
            14 => {
                let hit = self.ray(source, k.range, k.damage, 1, AttackClass::Basic, false);
                if n.is_multiple_of(4) {
                    self.actors[source].combat.magic_ammo =
                        (self.actors[source].combat.magic_ammo + 1).min(5);
                    if let Some(i) = hit {
                        for j in self.burst(
                            source,
                            self.actors[i].eye(),
                            2.5,
                            15.0,
                            AttackClass::Basic,
                            0.0,
                        ) {
                            if self.actors[j].combat.sleep <= 0.0 && !self.actors[j].combat.komi.resting() {
                                self.actors[j].combat.blind = 1.0;
                            }
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }
    pub(crate) fn use_skill(&mut self, source: usize) {
        let a = &self.actors[source];
        if a.ability_cooldown > 0.0 || a.combat.skills_locked() || a.combat.channeling() || a.combat.komi.active() || a.burrowed()
            || a.character == 12 && !crate::movement::grounded(&self.map, a.pos)
            || a.character == 6 && a.combat.chloe.active()
            || a.character == 5 && (a.combat.picora.shot_delay > 0.0 || self.picora_target(source).is_none())
        {
            return;
        }
        let id = a.character;
        self.actors[source].ability_cooldown = kit(id).cooldown
            * if character(id).race == Race::Witch {
                0.95
            } else {
                1.0
            };
        self.actors[source].ability_flash = 0.4;
        self.actors[source].shield = 0.0;
        match id {
            0 => self.start_vivi(source, crate::vivi_skill::Phase::Skill),
            1 => {
                self.actors[source].combat.naia.start();
                self.actors[source].reload = 0.0;
            }
            2 => self.start_erpin_skill(source, crate::erpin_skill::Phase::Skill),
            3 => self.protect_id_team(source),
            4 => self.start_aya(source, crate::aya_skill::Phase::Skill),
            5 => self.start_picora(source, crate::picora_skill::Phase::Skill),
            6 => self.start_chloe(source, true),
            7 => self.start_renua(source, crate::renua_skill::Phase::Skill),
            8 => self.start_mayo(source, crate::mayo_skill::Phase::Skill),
            9 => self.start_bella(source, crate::bella_skill::Phase::Skill),
            10 => self.start_ui(source, crate::ui_skill::Phase::Skill),
            11 => self.start_butter(source, crate::butter_skill::Phase::Skill),
            12 => self.start_komi(source, crate::komi_skill::Phase::Rest),
            13 => {
                self.start_hilde(source, crate::hilde_skill::Phase::Skill);
            }
            14 => {
                self.start_shion(source, crate::shion_skill::Phase::Load);
            }
            _ => unreachable!(),
        }
    }
    pub(crate) fn use_ultimate(&mut self, source: usize) -> bool {
        let a = &self.actors[source];
        if !a.alive()
            || a.combat.ultimate < 100.0
            || a.combat.skills_locked()
            || a.burrowed()
            || a.combat.chloe.busy()
            || a.combat.id.phase == crate::id_skill::Phase::Ultimate
            || a.combat.renua.phase == crate::renua_skill::Phase::Ultimate
            || a.combat.hilde.active()
            || a.combat.komi.active()
            || a.combat.ui.active()
            || a.combat.vivi.active()
            || a.combat.picora.active()
            || a.character == 5 && (a.combat.picora.shot_delay > 0.0 || self.picora_target(source).is_none())
            || a.combat.mayo.active()
            || a.combat.bella.active()
            || a.combat.shion.active()
            || a.combat.aya.phase == crate::aya_skill::Phase::Ultimate
        {
            return false;
        }
        let id = a.character;
        self.actors[source].combat.ultimate = 0.0;
        self.actors[source].combat.ultimate_flash = 1.2;
        self.actors[source].combat.cast = None;
        self.actors[source].combat.naia = Default::default();
        self.actors[source].ability_flash = 0.4;
        self.actors[source].shield = 0.0;
        self.event = format!("{}: {}", character(id).name, kit(id).ultimate);
        match id {
            0 => self.start_vivi(source, crate::vivi_skill::Phase::Ultimate),
            1 => {
                self.actors[source].combat.naia.phase = crate::naia_skill::Phase::Ultimate;
                self.actors[source].combat.ultimate_flash = crate::naia_skill::ULTIMATE_SECONDS;
                self.actors[source].reload = 0.0;
            }
            2 => self.start_erpin_skill(source, crate::erpin_skill::Phase::Start),
            3 => {
                self.actors[source]
                    .combat
                    .id
                    .start(crate::id_skill::Phase::Ultimate);
                self.actors[source].combat.ultimate_flash = crate::id_skill::ULTIMATE_SECONDS;
                self.actors[source].reload = 0.0;
            }
            4 => self.start_aya(source, crate::aya_skill::Phase::Ultimate),
            5 => self.start_picora(source, crate::picora_skill::Phase::Ultimate),
            6 => self.summon_sebastians(source),
            7 => {
                self.start_renua(source, crate::renua_skill::Phase::Ultimate);
                self.actors[source].combat.ultimate_flash = crate::renua_skill::ULTIMATE_SECONDS;
            }
            8 => self.start_mayo(source, crate::mayo_skill::Phase::Ultimate),
            9 => self.start_bella(source, crate::bella_skill::Phase::Ultimate),
            10 => self.start_ui(source, crate::ui_skill::Phase::Ultimate),
            11 => self.start_butter(source, crate::butter_skill::Phase::Ultimate),
            12 => self.start_komi(source, crate::komi_skill::Phase::Feast),
            13 => {
                self.start_hilde(source, crate::hilde_skill::Phase::Equip);
                self.actors[source].combat.ultimate_flash = crate::hilde_skill::EQUIP_SECONDS + crate::hilde_skill::LAUNCHER_SECONDS;
            }
            14 => {
                self.start_shion(source, crate::shion_skill::Phase::Ultimate);
                self.actors[source].combat.ultimate_flash = crate::shion_skill::ULTIMATE_SECONDS;
            }
            _ => unreachable!(),
        }
        true
    }
    pub(crate) fn advance_combat(&mut self) {
        // Protection expires before any actor's damage/DoT is processed this tick.
        for a in &mut self.actors {
            a.combat.rain_slow = 0.0;
            a.combat.id.tick_timers();
            a.combat.ward = (a.combat.ward - DT).max(0.0);
            if a.combat.ward < DT * 0.01 {
                a.combat.ward = 0.0;
            }
        }
        for i in 0..PLAYERS {
            if !self.actors[i].alive() {
                continue;
            }
            crate::picora_skill::advance_buffs(&mut self.actors[i]);
            let c = &mut self.actors[i].combat;
            c.ultimate = (c.ultimate + ULT_PER_SECOND * DT).min(100.0);
            let guard_expired = c.guard_time > 0.0 && c.guard_time <= DT;

            for t in [
                &mut c.ultimate_flash,
                &mut c.guard_time,
                &mut c.stun,
                &mut c.polymorph,
                &mut c.slow,
                &mut c.poison,
                &mut c.frost,
                &mut c.frozen,
                &mut c.weaken,
                &mut c.skill_weaken,
                &mut c.physical_weaken,
                &mut c.vulnerable,
                &mut c.haste,
                &mut c.speed_up,
                &mut c.damage_up,
                &mut c.skill_up,
                &mut c.resist,
                &mut c.physical_resist,
                &mut c.sleep,
                &mut c.giant,
                &mut c.empowered,
                &mut c.regen,
                &mut c.healing_up,
                &mut c.taunt,
                &mut c.blind,
            ] {
                *t = (*t - DT).max(0.0);
            }
            if guard_expired {
                c.guard = 0.0;
            }
            let mercury = guard_expired && c.mercury;
            if mercury {
                c.mercury = false;
            }
            c.regen_tick -= DT;
            c.aura_tick -= DT;
            let heal = if c.regen > 0.0 && c.regen_tick <= 0.0 {
                c.regen_tick = 0.25;
                c.regen_rate * 0.25
            } else {
                0.0
            };
            let dots: [Option<(usize, u8, AttackClass)>; 2] = std::array::from_fn(|d| {
                let active = if d == 0 { c.poison } else { c.frost };
                let dot = &mut c.dots[d];
                dot.tick -= DT;
                if d == 1 && active > 0.0 && dot.tick <= 0.0 {
                    dot.tick = 1.0;
                    dot.class.map(|class| (dot.owner, dot.character, class))
                } else {
                    None
                }
            });
            let regen_source = c.regen_source;
            let retaliate = c.retaliate;
            c.retaliate = false;
            if heal > 0.0 {
                if let Some(source) = regen_source {
                    self.heal_from(source, i, heal);
                } else {
                    self.heal(i, heal);
                }
            }
            for (source, id, class) in dots.into_iter().flatten() {
                self.damage_hit(source, i, 3.0, class, id);
            }
            if !self.actors[i].alive() {
                continue;
            }
            if mercury {
                self.mercury_burst(i);
            }
            if retaliate {
                let id = self.actors[i].character;
                self.heal_from(i, i, if id == 3 { 22.0 } else { 18.0 });
                for target in
                    self.burst(i, self.actors[i].eye(), 4.5, 20.0, AttackClass::Skill, 0.0)
                {
                    if self.actors[target].combat.sleep <= 0.0 && !self.actors[target].combat.komi.resting() {
                        self.actors[target].combat.weaken = 3.0;
                    }
                }
                if id == 9 && self.actors[i].combat.damage_up > 0.0 {
                    self.add_guard(i, 25.0, 5.0);
                }
            }
            let time_factor = crate::renua_skill::time_factor(&self.actors[i]);
            if time_factor == 0.0 && !self.actors[i].combat.action_interrupted() {
                continue;
            }
            self.advance_dash(i);
            if let Some(mut cast) = self.actors[i].combat.cast.take() {
                if self.actors[i].combat.disabled() {
                    continue;
                }
                cast.timer -= DT * time_factor;
                if cast.timer <= 0.0 {
                    self.cast_pulse(i, &mut cast);
                    cast.left = cast.left.saturating_sub(1);
                    cast.step += 1;
                    cast.timer += cast.interval;
                }
                if cast.left > 0 && self.actors[i].alive() {
                    self.actors[i].combat.cast = Some(cast);
                }
            }
        }
        self.advance_projectiles();
    }
    fn advance_dash(&mut self, i: usize) {
        if self.actors[i].combat.dash <= 0.0 {
            return;
        }
        if self.actors[i].combat.disabled() {
            self.actors[i].combat.dash = 0.0;
            return;
        }
        let action_dt = DT * crate::renua_skill::time_factor(&self.actors[i]);
        let step = self.actors[i].combat.dash_dir * (20.0 * action_dt / 5.0);
        for _ in 0..5 {
            let next = self.actors[i].pos + step;
            if crate::movement::blocked_with_height(
                &self.map,
                next,
                self.actors[i].collision_height(),
            ) {
                self.actors[i].combat.dash = 0.0;
                break;
            }
            self.actors[i].pos = next;
        }
        let origin = self.actors[i].eye();
        let id = self.actors[i].character;
        for target in self.targets(i, origin, 2.0, false) {
            if self.actors[i].combat.dash_mask & (1 << target) == 0 {
                self.actors[i].combat.dash_mask |= 1 << target;
                if self.damage_hit(i, target, 45.0, AttackClass::Ultimate, id) {
                    self.push(target, self.actors[i].combat.dash_dir, 2.0);
                }
            }
        }
        self.actors[i].combat.dash = (self.actors[i].combat.dash - action_dt).max(0.0);
    }
    fn cast_pulse(&mut self, i: usize, cast: &mut Cast) {
        let origin = self.actors[i].eye();
        match cast.kind {
            CastKind::Needles => {
                let offset = if cast.class == AttackClass::Ultimate {
                    (cast.step as f32 - 3.5) * 0.035
                } else {
                    0.0
                };
                let yaw = cast.dir.x.atan2(cast.dir.z) + offset;
                let pitch = cast.dir.y.asin();
                self.projectile(
                    i,
                    ProjectileKind::Poison,
                    direction(yaw, pitch),
                    44.0,
                    14.0,
                    cast.class,
                );
            }
            CastKind::Drain => {
                for t in self.burst(i, origin, 10.0, 12.0, cast.class, 0.0) {
                    self.taunt(i, t, 3.0);
                }
                for t in self.targets(i, origin, 10.0, true) {
                    if t != i {
                        self.heal_from(i, t, 10.0);
                    }
                }
                if cast.left == 1 {
                    self.heal_from(i, i, (self.actors[i].max_hp().saturating_sub(self.actors[i].hp)) as f32 * 0.4);
                }
            }

        }
        self.actors[i].ability_flash = 0.4;
    }
    pub(crate) fn projectile_hit(&mut self, p: &mut Projectile, target: usize) {
        let source = p.owner;
        let id = p.character;
        if p.kind == ProjectileKind::Syringe && team(source) == team(target) {
            self.heal_from(source, target, HILDE_SYRINGE_HEAL);
            return;
        }
        if p.kind == ProjectileKind::Wave && team(source) == team(target) {
            self.add_guard(
                target,
                crate::naia_skill::WAVE_GUARD,
                crate::naia_skill::WAVE_GUARD_SECONDS,
            );
            return;
        }
        let critical=p.class==AttackClass::Basic && crate::chloe_skill::headshot(&self.actors[target],p.pos,p.velocity.normalized(),p.velocity.length()*DT,p.kind.radius());
        if !self.damage_hit(source, target, p.damage*if critical{1.5}else{1.0}, p.class, id) {
            return;
        }
        let immune = self.actors[target].combat.sleep > 0.0 || self.actors[target].combat.komi.resting();
        match p.kind {
            ProjectileKind::Poison => self.dot(source, target, false, crate::mayo_skill::POISON_SECONDS, p.class),
            ProjectileKind::Butterfly => {
                if !immune {
                    self.actors[target].combat.weaken = crate::aya_skill::WEAKEN_SECONDS;
                }
                // hit_mask deduplicates each flight leg. Return hits are real hits too.
                if self.actors[source].alive() && self.actors[source].character == 4 {
                    self.actors[source].ability_cooldown =
                        (self.actors[source].ability_cooldown - crate::aya_skill::REFUND_SECONDS).max(0.0);
                }
            }
            ProjectileKind::Wave => self.push(target, p.velocity, crate::naia_skill::WAVE_PUSH),
            ProjectileKind::Flame => self.taunt(source, target, 3.0),
            ProjectileKind::Echo => {
                let c = &mut self.actors[target].combat;
                let buff = c.haste > 0.0 || c.damage_up > 0.0 || c.resist > 0.0 || c.skill_up > 0.0;
                let guard = c.guard > 0.0 || self.actors[target].barrier > 0.0;
                if matches!(p.step, 0 | 2 | 4) && buff {
                    let c = &mut self.actors[target].combat;
                    c.haste = 0.0;
                    c.damage_up = 0.0;
                    c.resist = 0.0;
                    c.skill_up = 0.0;
                    self.actors[source].combat.haste = 3.0;
                }
                if matches!(p.step, 1 | 3 | 4) && guard {
                    self.actors[target].combat.guard = 0.0;
                    self.actors[target].barrier = 0.0;
                    self.heal_from(source, source, 12.0);
                }
            }
            _ => {}
        }
    }
    pub(crate) fn explode_projectile(&mut self, p: &Projectile) {
        if p.kind == ProjectileKind::MagicBomb { self.erpin_blast(p.owner, p.pos, false); }
        if p.kind == ProjectileKind::MedicalGrenade {
            self.explode_hilde_grenade(p);
            return;
        }
        let (radius, push) = match p.kind {
            ProjectileKind::Spear => (4.0, 0.0),
            ProjectileKind::Doll => (2.0, 1.2),
            ProjectileKind::Boulder => (crate::butter_skill::BLAST_RADIUS, 3.0),
            ProjectileKind::DarkOrb => (4.0, 0.0),
            ProjectileKind::MagicBomb => (2.0, 0.0),
            _ => (0.0, 0.0),
        };
        if radius == 0.0 {
            return;
        }
        // Keep the original cast's character and attack class even after death.
        for target in self.targets(p.owner, p.pos, radius, false) {
            if self.damage_hit(p.owner, target, p.damage, p.class, p.character) {
                self.push(target, self.actors[target].eye() - p.pos, push);
                if p.kind == ProjectileKind::Boulder && self.actors[target].alive()
                    && self.actors[target].combat.sleep <= 0.0 && !self.actors[target].combat.komi.resting() {
                    self.actors[target].combat.stun = self.actors[target].combat.stun.max(crate::butter_skill::STUN_SECONDS);
                }
                if p.kind == ProjectileKind::Spear {
                    self.taunt(p.owner, target, 4.0);
                    if self.actors[target].combat.sleep <= 0.0 && !self.actors[target].combat.komi.resting() {
                        self.actors[target].combat.weaken = 4.0;
                    }
                }
            }
        }
        if p.kind == ProjectileKind::Doll { self.chloe_confetti(p); return; }
        if p.kind == ProjectileKind::Boulder {
            let mut effect = p.clone();
            effect.kind = ProjectileKind::BoulderBlast;
            effect.age = 0.0;
            effect.life = crate::butter_skill::BLAST_SECONDS;
            effect.velocity = V3::default();
            effect.damage = 0.0;
            self.projectiles.push(effect);
        } else if p.kind != ProjectileKind::MagicBomb { self.actors[p.owner].ability_flash = 0.4; }
    }
    fn advance_projectiles(&mut self) {
        let active = std::mem::take(&mut self.projectiles);
        for mut p in active {
            p.age += DT;
            p.life -= DT;
            if p.kind == ProjectileKind::Confetti {
                if p.life > 0.0 { self.projectiles.push(p); }
                continue;
            }
            if p.kind == ProjectileKind::Doll && p.life <= 0.0 { continue; }
            if p.kind == ProjectileKind::MercuryField {
                self.advance_mercury(&mut p);
                if p.life > 0.00001 { self.projectiles.push(p); }
                continue;
            }
            if matches!(p.kind, ProjectileKind::ErpinBlast | ProjectileKind::ErpinCrash) {
                if p.life > 0.0 { self.projectiles.push(p); }
                continue;
            }
            if p.kind == ProjectileKind::BellaFire {
                self.advance_bella_fire(&mut p);
                if p.life > 0.00001 { self.projectiles.push(p); }
                continue;
            }
            if p.kind == ProjectileKind::BellaHeal {
                if p.life > 0.00001 { self.projectiles.push(p); }
                continue;
            }
            if p.kind == ProjectileKind::BoulderBlast {
                if p.life > 0.0 { self.projectiles.push(p); }
                continue;
            }
            if p.kind == ProjectileKind::Ricochet && p.character == 11 {
                self.advance_butter_ball(&mut p);
                if p.life > 0.0 && p.pos.length() <= 180.0 { self.projectiles.push(p); }
                continue;
            }
            if p.kind == ProjectileKind::RainField {
                self.advance_ui_rain(&mut p);
                if p.life > 0.00001 { self.projectiles.push(p); }
                continue;
            }
            if p.kind.cake_remaining().is_some() {
                crate::erpin::step_cake(&mut p, &self.map);
                if p.life > 0.0 {
                    self.projectiles.push(p);
                }
                continue;
            }
            if p.kind == ProjectileKind::Wave {
                self.advance_naia_wave(&mut p);
                if p.life > 0.0 {
                    self.projectiles.push(p);
                }
                continue;
            }
            if p.kind == ProjectileKind::SnowGarden {
                if p.life > 0.0 && (p.step != 0 ||
                    (self.actors[p.owner].alive() && !self.actors[p.owner].combat.action_interrupted()
                        && self.actors[p.owner].combat.aya.phase == crate::aya_skill::Phase::Ultimate)) {
                    self.projectiles.push(p);
                }
                continue;
            }
            if p.kind == ProjectileKind::TimeField {
                if p.life > 0.0
                    && self.actors[p.owner].alive()
                    && self.actors[p.owner].character == 7
                {
                    self.projectiles.push(p);
                }
                continue;
            }
            if p.kind == ProjectileKind::Butterfly && p.age >= 1.25 && !p.returning {
                p.returning = true;
                p.hit_mask = 0;
                p.velocity = (p.origin - p.pos).normalized() * 14.0;
            }
            if p.kind == ProjectileKind::Doll {
                if let Some(t) = self
                    .targets(p.owner, p.pos, 16.0, false)
                    .into_iter()
                    .min_by(|&a, &b| {
                        (self.actors[a].eye() - p.pos)
                            .length()
                            .total_cmp(&(self.actors[b].eye() - p.pos).length())
                    })
                {
                    p.velocity = (p.velocity * 0.85
                        + (self.actors[t].pos + V3::new(0.0, 0.45, 0.0) - p.pos).normalized() * 1.5)
                        .normalized()
                        * 10.0;
                }
            }
            if p.kind == ProjectileKind::Boulder {
                p.velocity.y -= crate::butter_skill::BOULDER_GRAVITY * DT;
            } else if p.kind == ProjectileKind::Stone {
                p.velocity.y -= 5.0 * DT;
            }
            if p.kind == ProjectileKind::MedicalGrenade {
                p.velocity.y -= crate::hilde_skill::GRENADE_GRAVITY * DT;
            }
            let delta = p.velocity * DT;
            let distance = delta.length();
            let dir = delta.normalized();
            let radius = p.kind.radius();
            let mut wall = distance + 1.0;
            for b in &self.map {
                let pad = V3::new(radius, radius, radius);
                if let Some(t) = ray_box(p.pos, dir, b.min() - pad, b.max() + pad) {
                    if t <= distance {
                        wall = wall.min(t);
                    }
                }
            }
            if p.pos.y + delta.y < 0.05 && delta.y < 0.0 {
                wall = wall.min(((0.05 - p.pos.y) / dir.y).max(0.0));
            }
            let mut collisions: Vec<_> = self
                .actors
                .iter()
                .enumerate()
                .filter(|(i, a)| {
                    *i != p.owner
                        && a.alive()
                        && !a.burrowed()
                        && p.hit_mask & (1 << i) == 0
                        && (team(*i) != team(p.owner)
                            || matches!(p.kind, ProjectileKind::Wave | ProjectileKind::Syringe | ProjectileKind::MedicalGrenade))
                })
                .filter_map(|(i, a)| {
                    crate::chloe_skill::ray_hit(a, p.pos, dir, radius)
                    .filter(|t| *t <= distance && *t < wall)
                    .map(|t| (i, t))
                })
                .collect();
            collisions.sort_by(|a, b| a.1.total_cmp(&b.1));
            let piercing = matches!(
                p.kind,
                ProjectileKind::Wave | ProjectileKind::Butterfly | ProjectileKind::Flame
            );
            let explosive = matches!(
                p.kind,
                ProjectileKind::Spear
                    | ProjectileKind::Doll
                    | ProjectileKind::Boulder
                    | ProjectileKind::DarkOrb
                    | ProjectileKind::MagicBomb
                    | ProjectileKind::MedicalGrenade
            );
            let mut ended = false;
            let mut ricochet = false;
            for (target, t) in collisions {
                p.hit_mask |= 1 << target;
                if explosive {
                    p.pos = p.pos + dir * (t - 0.02).max(0.0);
                    self.explode_projectile(&p);
                    ended = true;
                    break;
                }
                self.projectile_hit(&mut p, target);
                if p.kind == ProjectileKind::Ricochet && p.bounces > 0 {
                    let center = self.actors[target].eye();
                    let next = self
                        .targets(p.owner, center, 8.0, false)
                        .into_iter()
                        .filter(|i| p.hit_mask & (1 << i) == 0)
                        .min_by(|&a, &b| {
                            (self.actors[a].eye() - center)
                                .length()
                                .total_cmp(&(self.actors[b].eye() - center).length())
                        });
                    if let Some(next) = next {
                        p.pos = center;
                        p.velocity = (self.actors[next].eye() - center).normalized() * 30.0;
                        p.bounces -= 1;
                        p.life = 1.0;
                        ricochet = true;
                        break;
                    }
                }
                if !piercing {
                    ended = true;
                    break;
                }
            }
            if ended {
                continue;
            }
            if !ricochet {
                if wall <= distance {
                    if let Some(hit) =
                        crate::impact::trace(&self.map, p.pos, dir, wall + 0.002, radius)
                    {
                        self.record_projectile_impact(hit, (radius * 0.65).max(0.11), p.kind, dir);
                    }
                    p.pos = p.pos + dir * (wall - 0.03).max(0.0);
                    if explosive {
                        if p.kind != ProjectileKind::Doll { self.explode_projectile(&p); }
                    }
                    continue;
                }
                p.pos = p.pos + delta;
            }
            if p.life <= 0.0
                || p.pos.length() > 180.0
                || (p.returning && (p.pos - p.origin).length() < 0.5)
            {
                if explosive && p.kind != ProjectileKind::Doll {
                    self.explode_projectile(&p);
                }
                continue;
            }
            self.projectiles.push(p);
        }
    }
}
