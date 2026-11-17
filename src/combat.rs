//! Server-authoritative character weapons, skills, projectiles and ultimates.
use crate::{
    game::*,
    kits::kit,
    roster::{character, Personality, Race},
};

pub const MAX_PROJECTILES: usize = 96;
pub const ULT_PER_SECOND: f32 = 1.25;
pub const ULT_PER_DAMAGE: f32 = 0.12;

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
    Echo,
    Needles,
    Marksman,
    Universe,
    Rain,
    Drain,
    TimeBreak,
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
            CastKind::Echo | CastKind::Needles | CastKind::Marksman
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
    pub erpin: crate::erpin::State,
    pub naia: crate::naia_skill::State,
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
    pub regen_tick: f32,
    pub healing_up: f32,
    pub ward: f32,
    pub ward_saves: u8,
    pub taunt: f32,
    pub taunt_source: usize,
    pub blind: f32,
    pub fashion: f32,
    pub aura_tick: f32,
    pub revive_armed: bool,
    pub revive_used: bool,
    pub attacks: u32,
    /// Last Vivi basic attack in the repeating 1, 2, 3 sequence; zero after spawning.
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
    pub fn disabled(&self) -> bool {
        self.stun > 0.0 || self.sleep > 0.0
    }
    pub fn channeling(&self) -> bool {
        self.erpin.eating
            || self.naia.active()
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
            self.resist.max(self.physical_resist),
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
        if self.stun > 0.0 {
            "STUN / MORPH"
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
        } else if self.slow > 0.0 {
            "SLOWED"
        } else if self.taunt > 0.0 {
            "TAUNTED: OTHER TARGETS -25%"
        } else if self.ward > 0.0 {
            "LETHAL-HIT WARD"
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
            _ => return None,
        })
    }
    pub fn radius(self) -> f32 {
        match self {
            Self::Boulder => 0.65,
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
    fn targets(&self, source: usize, origin: V3, radius: f32, allies: bool) -> Vec<usize> {
        self.actors
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                a.alive()
                    && (team(*i) == team(source)) == allies
                    && (allies || !self.in_spawn(*i))
                    && (a.eye() - origin).length() <= radius
                    && self.visible_between(origin, a.eye())
            })
            .map(|(i, _)| i)
            .collect()
    }
    fn aimed_target(&self, source: usize, allies: bool, range: f32) -> Option<usize> {
        let a = &self.actors[source];
        let dir = direction(a.yaw, a.pitch);
        self.targets(source, a.eye(), range, allies)
            .into_iter()
            .filter(|&i| i != source)
            .filter_map(|i| {
                let d = self.actors[i].eye() - a.eye();
                let dot = dir.dot(d.normalized());
                (dot > 0.87).then_some((i, dot, d.length()))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.2.total_cmp(&a.2)))
            .map(|v| v.0)
    }
    fn wounded(&self, source: usize, range: f32) -> Vec<usize> {
        let mut targets = self.targets(source, self.actors[source].eye(), range, true);
        targets.sort_by_key(|&i| (self.actors[i].hp, i));
        targets
    }
    fn point_aimed(&self, source: usize, range: f32) -> V3 {
        let a = &self.actors[source];
        let dir = direction(a.yaw, a.pitch);
        let mut distance = range;
        for b in &self.map {
            if let Some(t) = ray_box(a.eye(), dir, b.min(), b.max()) {
                distance = distance.min((t - 0.05).max(0.0));
            }
        }
        if let Some(i) = self.aimed_target(source, false, range) {
            self.actors[i].eye()
        } else {
            a.eye() + dir * distance
        }
    }
    fn add_guard(&mut self, i: usize, amount: f32, time: f32) {
        let c = &mut self.actors[i].combat;
        c.guard = c.guard.max(amount).min(120.0);
        c.guard_time = c.guard_time.max(time);
    }
    fn cleanse(&mut self, i: usize, stun: bool) {
        let c = &mut self.actors[i].combat;
        c.poison = 0.0;
        c.frost = 0.0;
        c.slow = 0.0;
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
                .min(MAX_HP);
    }
    fn push(&mut self, target: usize, dir: V3, distance: f32) {
        if self.actors[target].shield > 0.0 || !self.actors[target].alive() || self.in_spawn(target)
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
        if self.actors[target].shield <= 0.0
            && self.actors[target].combat.sleep <= 0.0
            && !self.in_spawn(target)
        {
            self.actors[target].combat.taunt = seconds;
            self.actors[target].combat.taunt_source = source;
        }
    }
    fn dot(&mut self, source: usize, target: usize, frost: bool, seconds: f32, class: AttackClass) {
        if self.in_spawn(target) {
            return;
        }
        let id = self.actors[source].character;
        let c = &mut self.actors[target].combat;
        if c.sleep > 0.0 {
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
        if source_character == 14 {
            factor *= 1.0 + s.magic_ammo as f32 * 0.05;
        }
        if class == AttackClass::Basic && s.empowered > 0.0 {
            factor *= 1.0 + s.empower_stacks as f32 * 0.02;
        }
        if s.weaken > 0.0 || s.poison > 0.0 {
            factor *= 0.8;
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
        if s.taunt > 0.0 && s.taunt_source != target {
            factor *= 0.75;
        }
        if t.resist > 0.0 {
            factor *= 0.75;
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
        let mut damage = (base * factor).round().max(0.0);
        let a = &mut self.actors[target];
        let before_hp = a.hp;
        let before_barrier = a.shield_health();
        // Apply once per hit after damage modifiers, before either shield absorbs it.
        // A shield present at impact also halves that hit's overflow into HP.
        if before_barrier > 0.0 && damage > 10.0 {
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
        if damage >= a.hp as f32 && a.combat.ward > 0.0 && a.combat.ward_saves > 0 {
            a.combat.ward_saves -= 1;
            a.combat.guard = 24.0;
            a.combat.guard_time = 3.0;
            damage = 0.0;
        }
        a.hp = a.hp.saturating_sub(damage.ceil() as u16);
        let actual = (before_hp - a.hp) as f32 + (before_barrier - a.shield_health()).max(0.0);
        a.safe_time = 0.0;
        a.hurt = 0.35;
        a.dancing = false;
        a.wave_ticks = 0;
        if matches!(a.character, 3 | 9) && actual > 0.0 {
            a.combat.incoming = a.combat.incoming.saturating_add(1);
            let threshold = if a.character == 3 { 5 } else { 4 };
            if a.combat.incoming >= threshold {
                a.combat.incoming = 0;
                a.combat.retaliate = true;
            }
        }
        let revive =
            !a.alive() && a.character == 9 && a.combat.revive_armed && !a.combat.revive_used;
        if revive {
            a.hp = 60;
            a.combat.revive_used = true;
            a.combat.revive_armed = false;
            a.combat.stun = 0.4;
        }
        let killed = !a.alive();
        if killed {
            a.deaths += 1;
            a.respawn = 7.0;
            a.killed_by = Some(source);
            a.ghost_phase = 0.0;
            a.phase_skill = Default::default();
            a.combat.cast = None;
            a.combat.dash = 0.0;
        }
        if revive && self.actors[source].combat.sleep <= 0.0 {
            self.actors[source].combat.stun = 0.6;
        }
        if killed {
            self.actors[source].kills += 1;
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
    fn burst(
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
            if self.actors[i].combat.sleep <= 0.0 {
                self.actors[i].combat.vulnerable = 4.0;
            }
        }
    }
    fn melee(&mut self, source: usize, range: f32, damage: f32, hits: u8, enhanced: bool) {
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
                && self.actors[i].combat.sleep <= 0.0
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
            self.heal(source, 5.0);
        }
        self.actors[source].shot_end = origin + dir * range;
        let a = &self.actors[source];
        if let Some(hit) = crate::impact::melee_contact(&self.map, origin, a.yaw, a.pitch, range) {
            let finisher = id == 0 && a.combat.vivi_combo == 3;
            let tilt = if finisher || a.combat.attacks.is_multiple_of(2) {
                -0.55
            } else {
                0.55
            };
            let rotation = crate::impact::slash_rotation(hit.face, a.yaw, a.pitch, tilt);
            let half_length = (if finisher { 0.64 } else { 0.48 }) * a.body_scale();
            self.record_slash(hit, half_length, rotation);
        }
    }
    fn ray(
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
            .filter(|(i, a)| *i != source && a.alive())
            .filter_map(|(i, a)| {
                let size = a.body_scale();
                ray_box(
                    origin,
                    dir,
                    a.pos + V3::new(-0.48 * size, 0.0, -0.48 * size),
                    a.pos + V3::new(0.48 * size, a.hit_height(), 0.48 * size),
                )
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
                self.damage_hit(source, i, damage / hits as f32, class, id);
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
        if a.cooldown > 0.0 || a.reload > 0.0 || a.combat.disabled() || a.combat.channeling() {
            return;
        }
        if k.magazine > 0 && a.ammo == 0 {
            self.reload(source);
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
            0 => {
                let stage = self.actors[source].combat.vivi_combo % 3 + 1;
                self.actors[source].combat.vivi_combo = stage;
                let finisher = stage == 3;
                self.melee(
                    source,
                    k.range,
                    k.damage * if finisher { 1.5 } else { 1.0 },
                    if finisher { 5 } else { 2 },
                    finisher,
                );
            }
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
                self.ray(source, k.range, k.damage, 4, AttackClass::Basic, false);
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
                let index = self.projectile(
                    source,
                    if n.is_multiple_of(4) {
                        ProjectileKind::Ricochet
                    } else {
                        ProjectileKind::Bolt
                    },
                    dir,
                    24.0,
                    k.damage,
                    AttackClass::Basic,
                );
                if let Some(index) = index {
                    if n.is_multiple_of(4) {
                        self.projectiles[index].bounces = 1;
                    }
                }
            }
            6 => self.melee(
                source,
                if boosted { 4.4 } else { k.range },
                if boosted { 40.0 } else { k.damage },
                if boosted { 3 } else { 1 },
                !boosted && n.is_multiple_of(3),
            ),
            7 => {
                if n.is_multiple_of(4) {
                    self.ray(source, k.range, 26.0, 1, AttackClass::Basic, true);
                    self.actors[source].combat.damage_up = 3.0;
                } else {
                    self.projectile(
                        source,
                        ProjectileKind::Bolt,
                        dir,
                        40.0,
                        k.damage,
                        AttackClass::Basic,
                    );
                }
            }
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
                        .find(|&i| self.actors[i].hp < MAX_HP)
                } else {
                    None
                };
                if let Some(i) = target {
                    self.heal(i, 24.0);
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
                            if self.actors[j].combat.sleep <= 0.0 {
                                self.actors[j].combat.blind = 1.0;
                            }
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }
    fn start_cast(
        &mut self,
        i: usize,
        kind: CastKind,
        count: u8,
        interval: f32,
        class: AttackClass,
    ) {
        let a = &mut self.actors[i];
        a.combat.cast = Some(Cast {
            kind,
            left: count,
            step: 0,
            timer: 0.0,
            interval,
            dir: direction(a.yaw, a.pitch),
            origin: a.eye(),
            mask: 0,
            target: None,
            switched: false,
            class,
        });
    }
    pub(crate) fn use_skill(&mut self, source: usize) {
        let a = &self.actors[source];
        if a.ability_cooldown > 0.0 || a.combat.disabled() || a.combat.channeling() {
            return;
        }
        let id = a.character;
        let origin = a.eye();
        let dir = direction(a.yaw, a.pitch);
        self.actors[source].ability_cooldown = kit(id).cooldown
            * if character(id).race == Race::Witch {
                0.95
            } else {
                1.0
            };
        self.actors[source].ability_flash = 0.4;
        self.actors[source].shield = 0.0;
        match id {
            0 => {
                self.add_guard(source, 45.0, 6.0);
                self.actors[source].combat.mercury = true;
            }
            1 => {
                self.actors[source].combat.naia.start();
                self.actors[source].reload = 0.0;
            }
            2 => {
                for n in 0..4 {
                    let d = direction(
                        self.actors[source].yaw + (n as f32 - 1.5) * 0.065,
                        self.actors[source].pitch,
                    );
                    self.projectile(
                        source,
                        ProjectileKind::MagicBomb,
                        d,
                        20.0,
                        14.0,
                        AttackClass::Skill,
                    );
                }
            }
            3 => {
                self.add_guard(source, 50.0, 8.0);
                for i in self.targets(source, origin, 10.0, true) {
                    if i != source {
                        let c = &mut self.actors[i].combat;
                        c.ward = 6.0;
                        c.ward_saves = 2;
                    }
                }
            }
            4 => {
                self.projectile(
                    source,
                    ProjectileKind::Butterfly,
                    dir,
                    14.0,
                    15.0,
                    AttackClass::Skill,
                );
            }
            5 => {
                for i in self.wounded(source, 14.0).into_iter().take(3) {
                    self.heal(i, 12.0);
                    let c = &mut self.actors[i].combat;
                    c.regen = 4.0;
                    c.regen_rate = 8.0;
                    c.physical_resist = 4.0;
                    c.healing_up = 4.0;
                }
            }
            6 => {
                self.add_guard(source, 50.0, 8.0);
                let c = &mut self.actors[source].combat;
                c.empowered = 8.0;
                c.empower_stacks = 0;
                c.extended = false;
            }
            7 => self.start_cast(source, CastKind::Echo, 5, 0.16, AttackClass::Skill),
            8 => self.start_cast(source, CastKind::Needles, 3, 0.12, AttackClass::Skill),
            9 => {
                self.projectile(
                    source,
                    ProjectileKind::Flame,
                    dir,
                    15.0,
                    24.0,
                    AttackClass::Skill,
                );
                for i in self.targets(source, origin, 8.0, true) {
                    self.add_guard(i, 24.0, 5.0);
                    self.actors[i].combat.damage_up = 5.0;
                }
                if !self.actors[source].combat.revive_used {
                    self.actors[source].combat.revive_armed = true;
                }
            }
            10 => self.start_cast(source, CastKind::Rain, 8, 0.5, AttackClass::Skill),
            11 => {
                self.projectile(
                    source,
                    ProjectileKind::Ricochet,
                    dir,
                    30.0,
                    14.0,
                    AttackClass::Skill,
                );
            }
            12 => {
                self.cleanse(source, true);
                let c = &mut self.actors[source].combat;
                c.sleep = 4.0;
                c.regen = 4.0;
                c.regen_rate = 18.0;
            }
            13 => {
                for i in self.targets(source, origin, 9.0, true) {
                    self.heal(i, 20.0);
                }
                for i in self
                    .wounded(source, 9.0)
                    .into_iter()
                    .filter(|&i| self.actors[i].hp < 96)
                    .take(2)
                    .collect::<Vec<_>>()
                {
                    self.actors[i].combat.regen = 4.0;
                    self.actors[i].combat.regen_rate = 8.0;
                }
            }
            14 => {
                if self.actors[source].combat.magic_ammo == 5 {
                    self.actors[source].combat.damage_up = 6.0;
                }
                self.actors[source].combat.magic_ammo = 5;
                self.start_cast(source, CastKind::Marksman, 5, 0.13, AttackClass::Skill);
            }
            _ => unreachable!(),
        }
    }
    pub(crate) fn use_ultimate(&mut self, source: usize) -> bool {
        let a = &self.actors[source];
        if !a.alive() || a.combat.ultimate < 100.0 || a.combat.disabled() {
            return false;
        }
        let id = a.character;
        let origin = a.eye();
        let dir = direction(a.yaw, a.pitch);
        self.actors[source].combat.ultimate = 0.0;
        self.actors[source].combat.ultimate_flash = 1.2;
        self.actors[source].combat.cast = None;
        self.actors[source].combat.naia = Default::default();
        self.actors[source].ability_flash = 0.4;
        self.actors[source].shield = 0.0;
        self.event = format!("{}: {}", character(id).name, kit(id).ultimate);
        match id {
            0 => {
                self.projectile(
                    source,
                    ProjectileKind::Spear,
                    dir,
                    30.0,
                    55.0,
                    AttackClass::Ultimate,
                );
            }
            1 => {
                self.projectile(
                    source,
                    ProjectileKind::Wave,
                    dir.flat().normalized(),
                    12.0,
                    35.0,
                    AttackClass::Ultimate,
                );
            }
            2 => {
                let c = &mut self.actors[source].combat;
                c.dash = 0.55;
                c.dash_dir = dir.flat().normalized();
                c.dash_mask = 0;
            }
            3 => {
                for i in self.targets(source, origin, 9.0, true) {
                    self.actors[i].combat.skill_up = 6.0;
                }
                self.start_cast(source, CastKind::Universe, 4, 0.45, AttackClass::Ultimate);
            }
            4 => {
                let mut center = self.point_aimed(source, 24.0);
                let mut seen = 0u16;
                for step in 0..5 {
                    let hits = self.targets(source, center, 2.8, false);
                    let damage = if step == 0 && hits.len() == 1 {
                        55.0
                    } else {
                        35.0
                    };
                    for i in hits {
                        if seen & (1 << i) == 0 {
                            seen |= 1 << i;
                            if self.damage_hit(source, i, damage, AttackClass::Ultimate, id) {
                                self.dot(source, i, true, 5.0, AttackClass::Ultimate);
                            }
                        }
                    }
                    let next = self
                        .targets(source, center, 8.0, false)
                        .into_iter()
                        .filter(|i| seen & (1 << i) == 0)
                        .min_by(|&a, &b| {
                            (self.actors[a].eye() - center)
                                .length()
                                .total_cmp(&(self.actors[b].eye() - center).length())
                        });
                    if let Some(i) = next {
                        center = self.actors[i].eye();
                    } else {
                        break;
                    }
                }
            }
            5 => {
                let target = self
                    .aimed_target(source, true, 18.0)
                    .or_else(|| {
                        self.wounded(source, 12.0)
                            .into_iter()
                            .find(|&i| i != source)
                    })
                    .unwrap_or(source);
                self.add_guard(target, 70.0, 8.0);
                self.actors[target].combat.fashion = 8.0;
            }
            6 => {
                for n in 0..7 {
                    let d = direction(
                        self.actors[source].yaw + (n as f32 - 3.0) * 0.13,
                        self.actors[source].pitch,
                    );
                    self.projectile(
                        source,
                        ProjectileKind::Doll,
                        d,
                        10.0,
                        14.0,
                        AttackClass::Ultimate,
                    );
                }
            }
            7 => {
                for i in self.targets(source, origin, 12.0, true) {
                    let c = &mut self.actors[i].combat;
                    c.haste = 6.0;
                    c.speed_up = 6.0;
                    c.resist = 6.0;
                }
                let targets = self.targets(source, origin, 12.0, false);
                let mut mask = 0;
                for i in targets {
                    if self.actors[i].shield <= 0.0 && self.actors[i].combat.sleep <= 0.0 {
                        self.actors[i].combat.slow = 3.0;
                        mask |= 1 << i;
                    }
                }
                self.start_cast(source, CastKind::TimeBreak, 6, 0.12, AttackClass::Ultimate);
                let cast = self.actors[source].combat.cast.as_mut().unwrap();
                cast.timer = 2.0;
                cast.mask = mask;
            }
            8 => self.start_cast(source, CastKind::Needles, 8, 0.1, AttackClass::Ultimate),
            9 => self.start_cast(source, CastKind::Drain, 4, 0.75, AttackClass::Ultimate),
            10 => {
                let allies = self.wounded(source, 12.0);
                let healed = allies.first().copied();
                if let Some(i) = healed {
                    self.heal(i, MAX_HP as f32);
                    self.add_guard(i, 25.0, 6.0);
                }
                if let Some(i) =
                    allies
                        .into_iter()
                        .filter(|&i| Some(i) != healed)
                        .max_by(|&a, &b| {
                            self.actors[a]
                                .ability_cooldown
                                .total_cmp(&self.actors[b].ability_cooldown)
                        })
                {
                    self.actors[i].ability_cooldown = 0.0;
                }
                if let Some(i) = self.aimed_target(source, false, 18.0) {
                    if self.actors[i].shield <= 0.0 && self.actors[i].combat.sleep <= 0.0 {
                        self.actors[i].combat.stun = 1.25;
                    }
                }
            }
            11 => {
                self.projectile(
                    source,
                    ProjectileKind::Boulder,
                    dir,
                    18.0,
                    65.0,
                    AttackClass::Ultimate,
                );
            }
            12 => {
                self.heal(source, 45.0);
                self.actors[source].combat.giant = 8.0;
                self.burst(source, origin, 5.5, 40.0, AttackClass::Ultimate, 2.0);
            }
            13 => {
                for i in self.burst(source, origin, 10.0, 30.0, AttackClass::Ultimate, 0.0) {
                    if self.actors[i].combat.sleep <= 0.0 {
                        self.actors[i].combat.physical_weaken = 5.0;
                    }
                }
                for i in self.targets(source, origin, 10.0, true) {
                    self.actors[i].combat.haste = 5.0;
                }
            }
            14 => {
                self.projectile(
                    source,
                    ProjectileKind::DarkOrb,
                    dir,
                    26.0,
                    60.0,
                    AttackClass::Ultimate,
                );
                self.actors[source].combat.magic_ammo =
                    (self.actors[source].combat.magic_ammo + 3).min(5);
            }
            _ => unreachable!(),
        }
        true
    }
    pub(crate) fn advance_combat(&mut self) {
        for i in 0..PLAYERS {
            if !self.actors[i].alive() {
                continue;
            }
            let c = &mut self.actors[i].combat;
            c.ultimate = (c.ultimate + ULT_PER_SECOND * DT).min(100.0);
            let guard_expired = c.guard_time > 0.0 && c.guard_time <= DT;
            let fashion_expired = c.fashion > 0.0 && c.fashion <= DT;
            for t in [
                &mut c.ultimate_flash,
                &mut c.guard_time,
                &mut c.stun,
                &mut c.slow,
                &mut c.poison,
                &mut c.frost,
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
                &mut c.ward,
                &mut c.taunt,
                &mut c.blind,
                &mut c.fashion,
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
                if active > 0.0 && dot.tick <= 0.0 {
                    dot.tick = 1.0;
                    dot.class.map(|class| (dot.owner, dot.character, class))
                } else {
                    None
                }
            });
            let fashion = c.fashion > 0.0 && c.aura_tick <= 0.0;
            if fashion {
                c.aura_tick = 1.0;
            }
            let retaliate = c.retaliate;
            c.retaliate = false;
            if heal > 0.0 {
                self.heal(i, heal);
            }
            if fashion_expired {
                self.heal(i, 35.0);
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
            if fashion {
                for target in self.targets(i, self.actors[i].eye(), 6.0, false) {
                    self.taunt(i, target, 1.2);
                }
            }
            if retaliate {
                let id = self.actors[i].character;
                self.heal(i, if id == 3 { 22.0 } else { 18.0 });
                for target in
                    self.burst(i, self.actors[i].eye(), 4.5, 20.0, AttackClass::Skill, 0.0)
                {
                    if self.actors[target].combat.sleep <= 0.0 {
                        self.actors[target].combat.weaken = 3.0;
                    }
                }
                if id == 9 && self.actors[i].combat.damage_up > 0.0 {
                    self.add_guard(i, 25.0, 5.0);
                }
            }
            self.advance_dash(i);
            if let Some(mut cast) = self.actors[i].combat.cast.take() {
                if self.actors[i].combat.stun > 0.0 {
                    continue;
                }
                cast.timer -= DT;
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
        if self.actors[i].combat.stun > 0.0 {
            self.actors[i].combat.dash = 0.0;
            return;
        }
        let step = self.actors[i].combat.dash_dir * (20.0 * DT / 5.0);
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
        self.actors[i].combat.dash = (self.actors[i].combat.dash - DT).max(0.0);
    }
    fn cast_pulse(&mut self, i: usize, cast: &mut Cast) {
        let origin = self.actors[i].eye();
        let id = self.actors[i].character;
        match cast.kind {
            CastKind::Echo => {
                if let Some(p) =
                    self.projectile(i, ProjectileKind::Echo, cast.dir, 38.0, 12.0, cast.class)
                {
                    self.projectiles[p].step = cast.step;
                }
            }
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
            CastKind::Marksman => {
                self.ray(i, 60.0, 12.0, 1, cast.class, false);
                self.actors[i].combat.magic_ammo =
                    self.actors[i].combat.magic_ammo.saturating_sub(1);
                self.actors[i].shot = self.actors[i].shot.wrapping_add(1);
            }
            CastKind::Universe => {
                for t in self.burst(i, origin, 9.0, 12.0, cast.class, 0.0) {
                    self.actors[t].ability_cooldown =
                        (self.actors[t].ability_cooldown + 0.7).min(20.0);
                }
            }
            CastKind::Rain => {
                for t in self.targets(i, origin, 7.0, true) {
                    self.heal(t, 8.0);
                    self.actors[t].ability_cooldown =
                        (self.actors[t].ability_cooldown - 0.2).max(0.0);
                }
                self.burst(i, origin, 7.0, 4.0, cast.class, 0.0);
            }
            CastKind::Drain => {
                for t in self.burst(i, origin, 10.0, 12.0, cast.class, 0.0) {
                    self.taunt(i, t, 3.0);
                }
                for t in self.targets(i, origin, 10.0, true) {
                    if t != i {
                        self.heal(t, 10.0);
                    }
                }
                if cast.left == 1 {
                    self.heal(i, (MAX_HP - self.actors[i].hp) as f32 * 0.4);
                }
            }
            CastKind::TimeBreak => {
                for t in 0..PLAYERS {
                    if cast.mask & (1 << t) != 0
                        && self.actors[t].alive()
                        && self.visible_between(cast.origin, self.actors[t].eye())
                        && self.damage_hit(i, t, 8.0, cast.class, id)
                        && self.actors[t].combat.sleep <= 0.0
                    {
                        self.actors[t].combat.stun = 0.75;
                    }
                }
            }
        }
        self.actors[i].ability_flash = 0.4;
    }
    fn projectile_hit(&mut self, p: &mut Projectile, target: usize) {
        let source = p.owner;
        let id = p.character;
        if p.kind == ProjectileKind::Wave && team(source) == team(target) {
            self.heal(target, 60.0);
            return;
        }
        if !self.damage_hit(source, target, p.damage, p.class, id) {
            return;
        }
        let immune = self.actors[target].combat.sleep > 0.0;
        match p.kind {
            ProjectileKind::Poison => self.dot(source, target, false, 4.0, p.class),
            ProjectileKind::Butterfly => {
                if !immune {
                    self.actors[target].combat.skill_weaken = 4.0;
                }
                if p.special_mask & (1 << target) == 0 {
                    self.actors[source].ability_cooldown =
                        (self.actors[source].ability_cooldown - 1.0).max(0.0);
                    p.special_mask |= 1 << target;
                }
            }
            ProjectileKind::Wave => self.push(target, p.velocity, 1.4),
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
                    self.heal(source, 12.0);
                }
            }
            _ => {}
        }
    }
    fn explode_projectile(&mut self, p: &Projectile) {
        let (radius, push) = match p.kind {
            ProjectileKind::Spear => (4.0, 0.0),
            ProjectileKind::Doll => (2.0, 1.2),
            ProjectileKind::Boulder => (4.5, 3.0),
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
                if p.kind == ProjectileKind::Spear {
                    self.taunt(p.owner, target, 4.0);
                    if self.actors[target].combat.sleep <= 0.0 {
                        self.actors[target].combat.weaken = 4.0;
                    }
                }
            }
        }
        self.actors[p.owner].ability_flash = 0.4;
    }
    fn advance_projectiles(&mut self) {
        let active = std::mem::take(&mut self.projectiles);
        for mut p in active {
            p.age += DT;
            p.life -= DT;
            if p.kind.cake_remaining().is_some() {
                crate::erpin::step_cake(&mut p, &self.map);
                if p.life > 0.0 {
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
                        + (self.actors[t].eye() - p.pos).normalized() * 1.5)
                        .normalized()
                        * 10.0;
                }
            }
            if matches!(p.kind, ProjectileKind::Stone | ProjectileKind::Boulder) {
                p.velocity.y -= 5.0 * DT;
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
                        && p.hit_mask & (1 << i) == 0
                        && (team(*i) != team(p.owner) || p.kind == ProjectileKind::Wave)
                })
                .filter_map(|(i, a)| {
                    let scale = a.body_scale();
                    let pad = V3::new(radius, radius, radius);
                    ray_box(
                        p.pos,
                        dir,
                        a.pos + V3::new(-0.48 * scale, 0.0, -0.48 * scale) - pad,
                        a.pos + V3::new(0.48 * scale, a.hit_height(), 0.48 * scale) + pad,
                    )
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
                        self.record_impact(hit, (radius * 0.65).max(0.11));
                    }
                    p.pos = p.pos + dir * (wall - 0.03).max(0.0);
                    if explosive {
                        self.explode_projectile(&p);
                    }
                    continue;
                }
                p.pos = p.pos + delta;
            }
            if p.life <= 0.0
                || p.pos.length() > 180.0
                || (p.returning && (p.pos - p.origin).length() < 0.5)
            {
                if explosive {
                    self.explode_projectile(&p);
                }
                continue;
            }
            self.projectiles.push(p);
        }
    }
}
