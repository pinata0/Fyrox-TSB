//! Competitive two-control-point simulation. All gameplay is decided here by the host.

use crate::roster::{character, Race, SelectionResult, DEFAULT_TEAM, ROSTER};
use crate::{
    combat::{CombatState, Projectile},
    kits::{kit, AttackMode},
};
use crate::{movement, navigation::next_waypoint};
use std::ops::{Add, Mul, Sub};

#[cfg(test)]
mod spawn_tests;

pub const TEAM_SIZE: usize = 6;
pub const PLAYERS: usize = TEAM_SIZE * 2;
pub const MAX_HP: u16 = 120;
pub const BASE_DAMAGE: f32 = 19.0;
pub const DT: f32 = 1.0 / 60.0;
pub const MATCH_RESULT_SECONDS: f32 = 5.0;
pub const MAGAZINE: u8 = 24;
pub const CAPTURE_SECONDS: f32 = 12.0;
pub const POINTS: [V3; 2] = [V3::new(0.0, 0.0, 12.0), V3::new(46.0, 0.0, 15.0)];
pub const RADIUS: f32 = 3.4;
pub const EYE_HEIGHT: f32 = 1.5;
pub const CROUCH_DROP: f32 = 0.35;
pub const SWAP_ULT_CAP: f32 = 15.0;
pub const SPAWN_HALF_WIDTH: f32 = 6.5;
pub const SPAWN_HALF_DEPTH: f32 = 3.0;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl V3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    pub fn dot(self, b: Self) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn normalized(self) -> Self {
        self * (1.0 / self.length().max(0.00001))
    }
    pub fn flat(self) -> Self {
        Self::new(self.x, 0.0, self.z)
    }
    pub fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}
impl Add for V3 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
}
impl Sub for V3 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
}
impl Mul<f32> for V3 {
    type Output = Self;
    fn mul(self, b: f32) -> Self {
        Self::new(self.x * b, self.y * b, self.z * b)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Block {
    pub center: V3,
    pub size: V3,
    pub color: [u8; 3],
    /// The Shift skill crosses sides; top faces and undersides remain solid.
    pub phaseable: bool,
}
impl Block {
    pub fn min(self) -> V3 {
        self.center - self.size * 0.5
    }
    pub fn max(self) -> V3 {
        self.center + self.size * 0.5
    }
}

pub fn arena() -> Vec<Block> {
    crate::level::arena()
}

/// Slab ray test, including rays parallel to a slab and rays starting inside it.
pub fn ray_box(origin: V3, dir: V3, min: V3, max: V3) -> Option<f32> {
    let mut near = 0.0_f32;
    let mut far = f32::INFINITY;
    for (o, d, lo, hi) in [
        (origin.x, dir.x, min.x, max.x),
        (origin.y, dir.y, min.y, max.y),
        (origin.z, dir.z, min.z, max.z),
    ] {
        if d.abs() < 0.00001 {
            if o < lo || o > hi {
                return None;
            }
        } else {
            let a = (lo - o) / d;
            let b = (hi - o) / d;
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return None;
            }
        }
    }
    (far >= 0.0).then_some(near)
}

pub fn direction(yaw: f32, pitch: f32) -> V3 {
    V3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    )
}
pub fn team(slot: usize) -> usize {
    slot / TEAM_SIZE
}
pub fn team_name(team: usize) -> &'static str {
    if team == 0 {
        "BLOOM"
    } else {
        "COCOA"
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Lobby,
    Setup,
    Playing,
    Intermission,
    Finished,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Input {
    pub forward: f32,
    pub strafe: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fire: bool,
    pub alt_fire: bool,
    pub jump: bool,
    pub reload: bool,
    pub phase_shift: bool,
    pub crouch: bool,
    pub ability: bool,
    pub ultimate: bool,
    pub dance: bool,
}
impl Input {
    pub fn sanitized(mut self) -> Self {
        if ![self.forward, self.strafe, self.yaw, self.pitch]
            .iter()
            .all(|x| x.is_finite())
        {
            return Self::default();
        }
        self.forward = self.forward.clamp(-1.0, 1.0);
        self.strafe = self.strafe.clamp(-1.0, 1.0);
        self.yaw = self.yaw.rem_euclid(std::f32::consts::TAU);
        // The final angle is clamped after applying cumulative portal rotation.
        self.pitch = self.pitch.clamp(-1.0e6, 1.0e6);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Actor {
    pub crouching: bool,
    pub wave_ticks: u8,
    pub communication_cooldown: f32,
    pub dancing: bool,
    pub dance_down: bool,
    pub character: u8,
    pub pos: V3,
    pub yaw: f32,
    pub pitch: f32,
    pub hp: u16,
    pub ammo: u8,
    pub reload: f32,
    pub cooldown: f32,
    pub respawn: f32,
    /// Authoritative source of the current death; cleared on spawn.
    pub killed_by: Option<usize>,
    pub shield: f32,
    pub kills: u16,
    pub deaths: u16,
    pub human: bool,
    /// Occupied by a human or an explicitly added bot. Empty slots never simulate.
    pub active: bool,
    pub vy: f32,
    pub shot: u32,
    pub shot_end: V3,
    pub hit: f32,
    pub hurt: f32,
    pub waypoint: V3,
    pub think: f32,
    /// Server-local target identity; invalidates a cached chase on sanctuary entry.
    pub bot_target: Option<usize>,
    pub barrier: f32,
    pub safe_time: f32,
    /// Authoritative recent wall contact, used to render local wall ripples on all clients.
    pub ghost_phase: f32,
    pub phase_skill: crate::phase_skill::State,
    pub portals: crate::portal::State,
    pub ability_cooldown: f32,
    pub ability_flash: f32,
    pub combat: CombatState,
}
impl Default for Actor {
    fn default() -> Self {
        Self {
            crouching: false,
            dancing: false,
            wave_ticks: 0,
            communication_cooldown: 0.0,
            dance_down: false,
            character: 0,
            pos: V3::default(),
            yaw: 0.0,
            pitch: 0.0,
            hp: MAX_HP,
            ammo: MAGAZINE,
            reload: 0.0,
            cooldown: 0.0,
            respawn: 0.0,
            killed_by: None,
            shield: 0.0,
            kills: 0,
            deaths: 0,
            human: false,
            active: true,
            vy: 0.0,
            shot: 0,
            shot_end: V3::default(),
            hit: 0.0,
            hurt: 0.0,
            waypoint: V3::default(),
            think: 0.0,
            bot_target: None,
            barrier: 0.0,
            safe_time: 0.0,
            ghost_phase: 0.0,
            phase_skill: Default::default(),
            portals: Default::default(),
            ability_cooldown: 0.0,
            ability_flash: 0.0,
            combat: CombatState::default(),
        }
    }
}
impl Actor {
    pub fn alive(&self) -> bool {
        self.active && self.hp > 0
    }
    /// Absorbable shield health, excluding timed spawn invulnerability.
    pub fn shield_health(&self) -> f32 {
        self.combat.guard + self.barrier
    }
    pub fn eye(&self) -> V3 {
        self.pos
            + V3::new(
                0.0,
                (EYE_HEIGHT
                    - self
                        .crouch_drop()
                        .max(crate::erpin::CAMERA_DROP * self.combat.erpin.seated()))
                    * self.body_scale(),
                0.0,
            )
    }
    pub fn crouch_drop(&self) -> f32 {
        if self.crouching {
            CROUCH_DROP
        } else {
            0.0
        }
    }
    pub fn collision_height(&self) -> f32 {
        movement::BODY_HEIGHT - self.crouch_drop()
    }
    pub fn hit_height(&self) -> f32 {
        (1.9 - self.crouch_drop()) * self.body_scale()
    }
    pub fn body_scale(&self) -> f32 {
        if self.combat.giant > 0.0 {
            1.4
        } else {
            1.0
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RoundResult {
    pub points: u8,
    pub progress: f32,
    pub bank: f32,
}
impl RoundResult {
    fn value(self) -> u32 {
        self.points as u32 * 10000
            + if self.progress + 0.0001 >= 1.0 / 3.0 {
                (self.progress * 10000.0).round() as u32
            } else {
                0
            }
    }
}

#[derive(Debug, Clone)]
pub struct World {
    pub map_id: crate::level::MapId,
    pub actors: [Actor; PLAYERS],
    pub projectiles: Vec<Projectile>,
    pub next_projectile: u32,
    pub impacts: Vec<crate::impact::Impact>,
    pub next_impact: u32,
    pub map: Vec<Block>,
    pub tick: u64,
    pub phase: Phase,
    pub phase_time: f32,
    pub round: u16,
    pub attacker: usize,
    pub point: usize,
    pub progress: f32,
    pub peak: f32,
    pub remaining: f32,
    pub overtime: bool,
    pub overtime_left: f32,
    pub absent: f32,
    pub contested: bool,
    pub scores: [u16; 2],
    pub banks: [f32; 2],
    pub result: Option<usize>,
    /// Server-local momentum captured on the final playing tick. Never predicts damage.
    pub result_velocity: [V3; PLAYERS],
    pub rounds: [RoundResult; 2],
    pub order: [usize; 2],
    pub leg: usize,
    pub extra: bool,
    pub free_attack: bool,
    pub event: String,
}
impl Default for World {
    fn default() -> Self {
        let mut w = Self {
            map_id: crate::level::MapId::default(),
            actors: std::array::from_fn(|_| Actor::default()),
            projectiles: vec![],
            next_projectile: 0,
            impacts: vec![],
            next_impact: 0,
            map: arena(),
            tick: 0,
            phase: Phase::Lobby,
            phase_time: 0.0,
            round: 1,
            attacker: 0,
            point: 0,
            progress: 0.0,
            peak: 0.0,
            remaining: 240.0,
            overtime: false,
            overtime_left: 3.0,
            absent: 0.0,
            contested: false,
            scores: [0; 2],
            banks: [240.0; 2],
            result: None,
            result_velocity: [V3::default(); PLAYERS],
            rounds: [RoundResult::default(); 2],
            order: [0, 1],
            leg: 0,
            extra: false,
            free_attack: false,
            event: "Welcome to the Fairy Kingdom Castle".into(),
        };
        for i in 0..PLAYERS {
            w.actors[i].character = DEFAULT_TEAM[i % TEAM_SIZE];
            w.spawn(i);
        }
        w
    }
}

impl World {
    /// New rooms start empty, optionally reserving the local player's slot.
    /// `default` retains the full roster for simulation fixtures and art reviews.
    pub fn room(map_id: crate::level::MapId, local: bool) -> Self {
        let mut world = Self::for_map(map_id);
        for slot in 0..PLAYERS {
            world.clear_slot(slot);
        }
        if local {
            world.join_player(0);
        }
        world
    }
    pub fn clear_slot(&mut self, slot: usize) {
        self.cancel_erpin(slot);
        self.actors[slot] = Actor {
            active: false,
            hp: 0,
            ammo: 0,
            character: DEFAULT_TEAM[slot % TEAM_SIZE],
            ..Default::default()
        };
        self.projectiles
            .retain(|p| p.owner != slot || p.kind.cake_remaining().is_some());
    }
    /// Empty slots get a legal hero; taking over a bot preserves its selection.
    pub fn join_player(&mut self, slot: usize) {
        if !self.actors[slot].active {
            let preferred = self.actors[slot].character;
            let id = std::iter::once(preferred)
                .chain(0..ROSTER.len() as u8)
                .find(|&id| self.bot_selection_check(slot, id).is_ok())
                .expect("a free team slot always has a legal hero");
            self.actors[slot].character = id;
            self.actors[slot].active = true;
            self.spawn(slot);
        }
        self.actors[slot].human = true;
    }
    pub fn bot_selection_check(&self, slot: usize, id: u8) -> Result<(), SelectionResult> {
        if slot >= PLAYERS || id as usize >= ROSTER.len() {
            return Err(SelectionResult::Invalid);
        }
        if self.actors[slot].human {
            return Err(SelectionResult::Locked);
        }
        let teammates = || {
            self.actors
                .iter()
                .enumerate()
                .filter(|(i, a)| *i != slot && team(*i) == team(slot) && a.active)
        };
        if teammates().any(|(_, a)| a.character == id) {
            return Err(SelectionResult::Taken);
        }
        if teammates()
            .filter(|(_, a)| character(a.character).position == character(id).position)
            .count()
            >= 3
        {
            return Err(SelectionResult::PositionFull);
        }
        Ok(())
    }
    /// Host/solo lobby controls. None removes a bot without refilling the slot.
    pub fn set_bot(&mut self, slot: usize, hero: Option<u8>) -> SelectionResult {
        if slot >= PLAYERS {
            return SelectionResult::Invalid;
        }
        if !matches!(self.phase, Phase::Lobby | Phase::Finished) || self.actors[slot].human {
            return SelectionResult::Locked;
        }
        if let Some(id) = hero {
            if let Err(reason) = self.bot_selection_check(slot, id) {
                return reason;
            }
            self.clear_slot(slot);
            self.actors[slot].active = true;
            self.actors[slot].character = id;
            self.spawn(slot);
        } else {
            self.clear_slot(slot);
        }
        SelectionResult::Accepted
    }
    pub fn team_count(&self, side: usize) -> usize {
        self.actors[side * TEAM_SIZE..(side + 1) * TEAM_SIZE]
            .iter()
            .filter(|a| a.active)
            .count()
    }
    pub fn for_map(map_id: crate::level::MapId) -> Self {
        let mut world = Self::default();
        world.select_map(map_id);
        world
    }
    /// Host/solo only. A running round cannot change geometry underneath players.
    pub fn select_map(&mut self, map_id: crate::level::MapId) -> bool {
        if !matches!(self.phase, Phase::Lobby | Phase::Finished) {
            return false;
        }
        if self.map_id == map_id {
            return true;
        }
        let selections = self
            .actors
            .each_ref()
            .map(|a| (a.active, a.human, a.character));
        *self = Self::default();
        self.map_id = map_id;
        self.map = map_id.arena();
        for (i, (active, human, character)) in selections.into_iter().enumerate() {
            self.actors[i].active = active;
            self.actors[i].human = human;
            self.actors[i].character = character;
            self.spawn(i);
        }
        self.event = format!("Welcome / {}", map_id.display_name());
        true
    }
    pub fn points(&self) -> [V3; 2] {
        self.map_id.points()
    }
    pub fn personality_counts(&self, side: usize) -> [usize; 5] {
        let mut counts = [0; 5];
        for (i, a) in self.actors.iter().enumerate() {
            if a.active && team(i) == side {
                counts[character(a.character).personality as usize] += 1;
            }
        }
        counts
    }
    pub fn position_counts(&self, side: usize) -> [usize; 3] {
        let mut counts = [0; 3];
        for (i, a) in self.actors.iter().enumerate() {
            if a.active && team(i) == side {
                counts[character(a.character).position as usize] += 1;
            }
        }
        counts
    }
    pub fn damage_bonus(&self, side: usize) -> u16 {
        (self
            .personality_counts(side)
            .into_iter()
            .map(|n| n / 2)
            .sum::<usize>()
            * 20) as u16
    }
    pub fn damage_multiplier(&self, side: usize) -> f32 {
        1.0 + self.damage_bonus(side) as f32 / 100.0
    }
    pub fn can_select(&self, slot: usize) -> bool {
        slot < PLAYERS
            && self.actors[slot].active
            && (!self.actors[slot].alive() || !blocked(&self.map, self.actors[slot].pos))
            && (self.phase != Phase::Playing || !self.actors[slot].alive() || self.in_spawn(slot))
    }
    /// Active spawn changes with the attacking team and the A/B objective.
    pub fn spawn_center(&self, side: usize) -> V3 {
        self.map_id
            .spawn_center(side == self.attacker, self.point == 1)
    }
    pub fn in_spawn(&self, slot: usize) -> bool {
        let Some(a) = self.actors.get(slot) else {
            return false;
        };
        let p = a.pos - self.spawn_center(team(slot));
        p.finite()
            && p.x.abs() <= SPAWN_HALF_WIDTH
            && p.z.abs() <= SPAWN_HALF_DEPTH
            && (0.0..=2.25).contains(&p.y)
    }
    /// Returns a bot to exchange with, if the requested character is occupied.
    pub fn selection_check(&self, slot: usize, id: u8) -> Result<Option<usize>, SelectionResult> {
        if slot >= PLAYERS || id as usize >= ROSTER.len() || !self.actors[slot].active {
            return Err(SelectionResult::Invalid);
        }
        if self.actors[slot].character == id {
            return Ok(None);
        }
        if !self.can_select(slot) {
            return Err(SelectionResult::Locked);
        }
        if let Some((other, a)) =
            self.actors.iter().enumerate().find(|(i, a)| {
                a.active && *i != slot && team(*i) == team(slot) && a.character == id
            })
        {
            return if a.human {
                Err(SelectionResult::Taken)
            } else if a.alive() && blocked(&self.map, a.pos) {
                Err(SelectionResult::Locked)
            } else {
                Ok(Some(other))
            };
        }
        let pos = character(id).position;
        let count = self
            .actors
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                a.active
                    && *i != slot
                    && team(*i) == team(slot)
                    && character(a.character).position == pos
            })
            .count();
        if count >= 3 {
            return Err(SelectionResult::PositionFull);
        }
        Ok(None)
    }
    pub fn select_character(&mut self, slot: usize, id: u8) -> SelectionResult {
        let other = match self.selection_check(slot, id) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if self.actors[slot].character == id {
            return SelectionResult::Accepted;
        }
        let old = self.actors[slot].character;
        self.change_character(slot, id);
        if let Some(other) = other {
            self.change_character(other, old);
        }
        SelectionResult::Accepted
    }
    fn change_character(&mut self, slot: usize, id: u8) {
        self.cancel_erpin(slot);
        let a = &mut self.actors[slot];
        a.wave_ticks = 0;
        a.dancing = false;
        a.character = id;
        a.portals.ends = [None; 2];
        // Never heal, teleport, refill ammo, or refresh skill cooldown by changing.
        a.barrier = 0.0;
        a.safe_time = 0.0;
        a.ghost_phase = 0.0;
        a.ability_flash = 0.0;
        if a.phase_skill.active() {
            a.phase_skill.remaining = 0.0;
            a.phase_skill.cooldown = crate::phase_skill::COOLDOWN;
        }
        a.ammo = a.ammo.min(kit(id).magazine);
        if self.phase != Phase::Playing {
            a.ammo = kit(id).magazine;
        }
        a.combat = CombatState {
            ultimate: a.combat.ultimate.clamp(0.0, SWAP_ULT_CAP),
            revive_used: a.combat.revive_used,
            ..Default::default()
        };
        self.projectiles
            .retain(|p| p.owner != slot || p.kind.cake_remaining().is_some());
    }
    /// Reset only match state, keeping the room's map, occupants and hero selections.
    fn reset_match(&mut self) {
        let selections = self
            .actors
            .each_ref()
            .map(|a| (a.active, a.human, a.character));
        *self = Self::for_map(self.map_id);
        for (a, (active, human, character)) in self.actors.iter_mut().zip(selections) {
            a.active = active;
            a.human = human;
            a.character = character;
        }
        for slot in 0..PLAYERS {
            self.spawn(slot);
        }
    }
    pub fn start(&mut self) {
        self.reset_match();
        self.begin_round();
    }
    fn begin_round(&mut self) {
        self.result = None;
        self.result_velocity = [V3::default(); PLAYERS];
        self.attacker = self.order[self.leg];
        self.point = 0;
        self.progress = 0.0;
        self.peak = 0.0;
        self.remaining = if self.extra {
            self.banks[self.attacker]
        } else {
            240.0
        };
        self.overtime = false;
        self.overtime_left = 3.0;
        self.absent = 0.0;
        self.contested = false;
        self.phase = Phase::Setup;
        self.phase_time = 8.0;
        self.projectiles.clear();
        self.impacts.clear();
        for i in 0..PLAYERS {
            self.actors[i].combat = CombatState::default();
            self.spawn(i);
        }
        self.event = format!(
            "Round {} / {} attacks",
            self.round,
            team_name(self.attacker)
        );
    }
    fn spawn(&mut self, i: usize) {
        if !self.actors[i].active {
            self.actors[i].hp = 0;
            return;
        }
        self.clear_combat_on_spawn(i);
        let attacking = team(i) == self.attacker;
        let base = self.spawn_center(team(i));
        let a = &mut self.actors[i];
        a.pos = base + V3::new((i % TEAM_SIZE) as f32 * 1.8 - 4.5, 0.0, 0.0);
        a.yaw = if attacking && self.point == 0 {
            0.0
        } else {
            std::f32::consts::PI
        };
        a.pitch = 0.0;
        a.hp = MAX_HP;
        a.ammo = kit(a.character).magazine;
        a.reload = 0.0;
        a.cooldown = 0.0;
        a.vy = 0.0;
        a.respawn = 0.0;
        a.killed_by = None;
        a.shield = 2.0;
        a.hit = 0.0;
        a.hurt = 0.0;
        a.think = 0.0;
        a.bot_target = None;
        a.barrier = 0.0;
        a.safe_time = 0.0;
        a.ghost_phase = 0.0;
        a.ability_cooldown = 0.0;
        a.phase_skill = Default::default();
        a.portals = Default::default();
        a.ability_flash = 0.0;
        a.dancing = false;
        a.dance_down = false;
        a.crouching = false;
        a.wave_ticks = 0;
    }
    pub fn tick(&mut self, inputs: [Input; PLAYERS]) {
        self.tick += 1;
        self.expire_impacts();
        if self.in_round_transition() {
            self.advance_round_aftermath();
            self.phase_time = (self.phase_time - DT).max(0.0);
            if self.phase_time == 0.0 {
                if self.phase == Phase::Finished {
                    let tick = self.tick;
                    self.reset_match();
                    self.tick = tick;
                    self.event = "Match complete / Back in the lobby".into();
                } else {
                    self.begin_round();
                }
            }
            return;
        }
        for (i, input) in inputs.iter().enumerate() {
            let a = &mut self.actors[i];
            let input = input.sanitized();
            a.communication_cooldown = (a.communication_cooldown - DT).max(0.0);
            a.wave_ticks = a.wave_ticks.saturating_sub(1);
            if !a.human
                || !a.alive()
                || a.hurt > 0.0
                || a.combat.disabled()
                || a.combat.cast.is_some()
                || a.combat.naia.active()
                || a.combat.erpin.eating
                || a.combat.dash > 0.0
                || a.reload > 0.0
                || input.fire
                || input.alt_fire && a.character == 2
                || input.ability
                || input.ultimate
                || input.reload
                || input.dance
            {
                a.wave_ticks = 0;
            }
            let pressed = input.dance && !a.dance_down;
            a.dance_down = input.dance;
            let cancel = !a.human
                || !a.alive()
                || a.hurt > 0.0
                || a.combat.disabled()
                || a.combat.cast.is_some()
                || a.combat.naia.active()
                || a.combat.erpin.eating
                || a.combat.dash > 0.0
                || a.reload > 0.0
                || !movement::grounded(&self.map, a.pos)
                || input.forward.abs() > 0.01
                || input.strafe.abs() > 0.01
                || input.fire
                || input.alt_fire && a.character == 2
                || input.jump
                || input.ability
                || input.ultimate
                || input.reload
                || input.phase_shift
                || input.crouch
                || a.crouching
                || matches!(self.phase, Phase::Intermission | Phase::Finished);
            if cancel {
                a.dancing = false;
            } else if pressed && a.human {
                a.dancing = !a.dancing;
            }
        }
        if self.phase == Phase::Setup {
            self.phase_time = (self.phase_time - DT).max(0.0);
            if self.phase_time <= 0.0 {
                self.phase = Phase::Playing;
            }
            return;
        }
        if self.phase != Phase::Playing {
            return;
        }
        let previous_positions = self.actors.each_ref().map(|a| a.pos);
        self.advance_combat();
        // Dead/disconnected owners cannot leave usable entrances behind.
        for a in &mut self.actors {
            if !a.alive() {
                a.portals.ends = [None; 2];
            }
        }
        for (i, supplied) in inputs.into_iter().enumerate() {
            if !self.actors[i].active {
                continue;
            }
            if !self.actors[i].alive() {
                self.advance_erpin(i, supplied);
                self.actors[i].combat.q_down = supplied.ultimate;
                self.actors[i].phase_skill = crate::phase_skill::State {
                    down: supplied.phase_shift,
                    ..Default::default()
                };
                self.actors[i].respawn -= DT;
                if self.actors[i].respawn <= 0.0 {
                    self.spawn(i);
                }
                continue;
            }
            let input = if self.actors[i].human {
                supplied.sanitized()
            } else {
                self.bot_input(i)
            };
            let pressed = input.ultimate && !self.actors[i].combat.q_down;
            self.actors[i].combat.q_down = input.ultimate;
            self.advance_naia_skill(i, input);
            self.advance_erpin(i, input);
            let pairs =
                self.actors
                    .each_ref()
                    .map(|a| if a.alive() { a.portals.ends } else { [None; 2] });
            crate::portal::advance(&self.map, &mut self.actors[i], input, &pairs);
            crate::phase_skill::advance(&self.map, &mut self.actors[i], input.phase_shift);
            self.move_actor(i, input);
            if pressed {
                self.use_ultimate(i);
            } else if input.ability {
                self.use_skill(i);
            }
            if input.reload {
                self.reload(i);
            }
            if input.fire && !(input.phase_shift && crate::portal::is_elf(&self.actors[i])) {
                self.primary_attack(i);
            }
            self.fire_naia_stream(i);
        }
        for (i, previous) in previous_positions.into_iter().enumerate() {
            let velocity = (self.actors[i].pos - previous) * (1.0 / DT);
            // Respawns and teleports must never turn into cinematic momentum.
            self.result_velocity[i] = if self.actors[i].alive() && velocity.length() < 40.0 {
                velocity.flat()
            } else {
                V3::default()
            };
        }
        self.update_objective();
    }
    pub(crate) fn reload(&mut self, i: usize) {
        let a = &mut self.actors[i];
        if a.reload <= 0.0
            && a.ammo < kit(a.character).magazine
            && !a.combat.disabled()
            && !a.combat.naia.active()
        {
            a.reload = if character(a.character).race == Race::Elf {
                1.44
            } else {
                1.6
            };
        }
    }
    fn move_actor(&mut self, i: usize, input: Input) {
        let pairs = self
            .actors
            .each_ref()
            .map(|a| if a.alive() { a.portals.ends } else { [None; 2] });
        let a = &mut self.actors[i];
        a.cooldown = (a.cooldown - DT).max(0.0);
        a.shield = (a.shield - DT).max(0.0);
        a.hit = (a.hit - DT).max(0.0);
        a.hurt = (a.hurt - DT).max(0.0);
        a.safe_time += DT;
        a.ghost_phase = (a.ghost_phase - DT).max(0.0);
        a.ability_cooldown = (a.ability_cooldown - DT).max(0.0);
        a.ability_flash = (a.ability_flash - DT).max(0.0);
        let race = character(a.character).race;
        if race == Race::Spirit && a.safe_time >= 4.0 {
            a.barrier = MAX_HP as f32 * 0.08;
        }
        if a.reload > 0.0 {
            a.reload = (a.reload - DT).max(0.0);
            if a.reload == 0.0 {
                a.ammo = kit(a.character).magazine;
            }
        }
        movement::step_actor_with_portals(&self.map, a, input, &pairs);
    }

    fn bot_input(&mut self, i: usize) -> Input {
        let actor = &self.actors[i];
        let pos = actor.pos;
        let eye = actor.eye();
        let objective = self.points()[self.point]
            + V3::new(
                (i % 3) as f32 * 1.6 - 1.6,
                0.0,
                (i % TEAM_SIZE / 3) as f32 * 1.4
                    + if team(i) == self.attacker { -1.0 } else { 0.0 },
            );
        let enemy = self
            .actors
            .iter()
            .enumerate()
            .filter(|(j, a)| team(*j) != team(i) && a.alive() && !self.in_spawn(*j))
            .filter_map(|(j, a)| {
                let diff = a.eye() - eye;
                let distance = diff.length();
                (distance < 26.0
                    && !self.map.iter().any(|b| {
                        ray_box(eye, diff.normalized(), b.min(), b.max())
                            .is_some_and(|t| t < distance)
                    }))
                .then_some((j, distance))
            })
            .min_by(|a, b| {
                let taunted = actor.combat.taunt > 0.0;
                let pa = taunted && a.0 == actor.combat.taunt_source;
                let pb = taunted && b.0 == actor.combat.taunt_source;
                pb.cmp(&pa).then_with(|| a.1.total_cmp(&b.1))
            });
        let k = kit(actor.character);
        let objective = if k.mode == AttackMode::Melee {
            enemy.map_or(objective, |(j, _)| self.actors[j].pos)
        } else {
            objective
        };
        // Drop cached pursuit immediately, including taunts, on sanctuary entry.
        let target = enemy.map(|(j, _)| j);
        let stale_pursuit = actor.bot_target != target;
        if actor.think <= 0.0 || stale_pursuit {
            self.actors[i].waypoint =
                if actor.phase_skill.active() && (pos.y - objective.y).abs() < 0.1 {
                    objective
                } else {
                    next_waypoint(&self.map, pos, objective)
                };
            self.actors[i].think = 0.45;
        }
        self.actors[i].bot_target = target;
        self.actors[i].think -= DT;
        let travel = (self.actors[i].waypoint - pos).flat();
        let aim = if let Some((j, _)) = enemy {
            // Deliberately imperfect tracking leaves room for human aim and strafing.
            let wobble = (self.tick as f32 * 0.045 + i as f32 * 2.0).sin() * 0.8;
            self.actors[j].pos + V3::new(wobble, 1.15 + wobble * 0.3, 0.0) - eye
        } else {
            travel
        };
        let yaw = aim.x.atan2(aim.z);
        let pitch = (aim.y / aim.length().max(0.001)).asin();
        let forward = direction(yaw, 0.0);
        let right = V3::new(-forward.z, 0.0, forward.x);
        let movement = if (objective - pos).length() > 0.7 {
            travel.normalized()
        } else {
            V3::default()
        };
        Input {
            forward: movement.dot(forward),
            strafe: movement.dot(right),
            yaw: yaw - self.actors[i].portals.yaw_offset,
            pitch: pitch - self.actors[i].portals.pitch_offset,
            fire: enemy.is_some_and(|(_, d)| d <= k.range) && self.tick % 90 < 65,
            reload: k.magazine > 0 && self.actors[i].ammo < (k.magazine / 4).max(1),
            ability: if character(self.actors[i].character).healer {
                self.actors.iter().enumerate().any(|(j, a)| {
                    team(j) == team(i)
                        && a.alive()
                        && a.hp <= MAX_HP - 25
                        && (a.pos - pos).length() <= 7.0
                })
            } else {
                if self.actors[i].character == 12 {
                    self.actors[i].hp < 80
                } else {
                    enemy.is_some_and(|(_, distance)| distance <= 18.0)
                }
            },
            ultimate: self.actors[i].combat.ultimate >= 100.0
                && !self.actors[i].combat.disabled()
                && (enemy.is_some_and(|(_, d)| d < 14.0)
                    || (character(self.actors[i].character).healer
                        && self
                            .actors
                            .iter()
                            .enumerate()
                            .any(|(j, a)| team(j) == team(i) && a.alive() && a.hp < 60))),
            ..Default::default()
        }
    }
    pub fn update_objective(&mut self) {
        // A team without opponents can explore indefinitely. Adding opponents
        // or a late human join resumes the normal objective and match clock.
        if self.phase != Phase::Playing || self.team_count(0) == 0 || self.team_count(1) == 0 {
            return;
        }
        let mut attackers = 0;
        let mut defenders = 0;
        for (i, a) in self.actors.iter().enumerate() {
            if a.alive()
                && a.pos.y < 2.5
                && (a.pos - self.points()[self.point]).flat().length() <= RADIUS
            {
                if team(i) == self.attacker {
                    attackers += 1;
                } else {
                    defenders += 1;
                }
            }
        }
        self.contested = attackers > 0 && defenders > 0;
        if attackers > 0 {
            self.absent = 0.0;
            if defenders == 0 {
                self.progress = (self.progress
                    + DT / CAPTURE_SECONDS * (1.0 + 0.35 * (attackers.min(3) - 1) as f32))
                    .min(1.0);
                self.peak = self.peak.max(self.progress);
            }
        } else {
            self.absent += DT;
            if self.absent > 5.0 {
                let floor = (self.progress * 3.0 + 0.00001).floor() / 3.0;
                self.progress = (self.progress - DT * 0.1).max(floor);
            }
        }
        self.remaining = (self.remaining - DT).max(0.0);
        if self.progress >= 1.0 {
            self.scores[self.attacker] += 1;
            if self.point == 0 {
                self.point = 1;
                self.progress = 0.0;
                self.peak = 0.0;
                self.absent = 0.0;
                if !self.extra {
                    self.remaining += 180.0;
                    self.overtime = false;
                    self.overtime_left = 3.0;
                }
                self.event = "Point A captured / Point B unlocked".into();
                // Brief dead-defender regrouping, without teleporting living players.
                for (i, a) in self.actors.iter_mut().enumerate() {
                    if team(i) != self.attacker && !a.alive() {
                        a.respawn = a.respawn.min(3.5);
                    }
                }
            } else {
                self.finish_round(true);
                return;
            }
        }
        let current = RoundResult {
            points: self.point as u8,
            progress: self.peak,
            bank: self.remaining,
        };
        if self.free_attack && current.value() >= 3333 {
            self.finish_match(self.attacker);
            return;
        }
        if self.leg == 1 && current.value() > self.rounds[self.order[0]].value() {
            self.finish_match(self.attacker);
            return;
        }
        if self.remaining == 0.0 {
            if attackers > 0 {
                self.overtime = true;
                self.overtime_left = 3.0;
            } else if self.overtime {
                self.overtime_left = (self.overtime_left - DT).max(0.0);
                if self.overtime_left == 0.0 {
                    self.finish_round(false);
                }
            } else {
                self.finish_round(false);
            }
        }
    }
    fn finish_match(&mut self, winner: usize) {
        self.result = Some(winner);
        self.phase = Phase::Finished;
        self.phase_time = MATCH_RESULT_SECONDS;
        for actor in &mut self.actors {
            actor.dancing = false;
            actor.wave_ticks = 0;
        }
        self.event = if winner == 2 {
            "DRAW / The castle belongs to everyone".into()
        } else {
            format!("{} WINS", team_name(winner))
        };
    }
    pub fn finish_round(&mut self, full: bool) {
        // The existing snapshot result byte also carries the completed round's winner.
        // `begin_round` clears it; a final match result below takes precedence.
        self.result = Some(if full {
            self.attacker
        } else {
            1 - self.attacker
        });
        self.rounds[self.attacker] = RoundResult {
            points: if full { 2 } else { self.point as u8 },
            progress: if full { 0.0 } else { self.peak },
            bank: if full { self.remaining } else { 0.0 },
        };
        self.banks[self.attacker] = self.rounds[self.attacker].bank;
        if self.free_attack {
            self.finish_match(if full { self.attacker } else { 2 });
            return;
        }
        if self.leg == 0 {
            self.leg = 1;
        } else {
            let [a, b] = self.rounds;
            if a.points == 2 && b.points == 2 {
                if a.bank == 0.0 && b.bank == 0.0 {
                    self.finish_match(2);
                    return;
                }
                if (a.bank == 0.0 && b.bank >= 60.0) || (b.bank == 0.0 && a.bank >= 60.0) {
                    let winner = usize::from(b.bank > a.bank);
                    self.order = [winner, 1 - winner];
                    self.free_attack = true;
                } else {
                    let min = a.bank.min(b.bank);
                    let correction = if min > 120.0 {
                        120.0 - min
                    } else if min < 60.0 {
                        60.0 - min
                    } else {
                        0.0
                    };
                    self.banks = [a.bank + correction, b.bank + correction];
                    let first = usize::from(b.bank < a.bank);
                    self.order = [first, 1 - first];
                }
                self.extra = true;
                self.leg = 0;
                self.rounds = [RoundResult::default(); 2];
            } else {
                self.finish_match(match a.value().cmp(&b.value()) {
                    std::cmp::Ordering::Greater => 0,
                    std::cmp::Ordering::Less => 1,
                    std::cmp::Ordering::Equal => 2,
                });
                return;
            }
        }
        self.round += 1;
        self.phase = Phase::Intermission;
        self.phase_time = crate::round_transition::ROUND_RESULT_SECONDS;
        self.event = "Round complete / Switching sides".into();
    }
}

/// Character-sized three-dimensional collision, shared with dash and knockback.
pub fn blocked(map: &[Block], pos: V3) -> bool {
    movement::blocked(map, pos)
}
