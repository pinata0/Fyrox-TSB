//! Shared racial movement skills, stepped identically by host and prediction.
use crate::{
    game::{direction, Actor, Block, Input, DT, V3},
    movement,
    roster::{character, Race},
};

pub const LEAP_DURATION: f32 = 4.0; // Safety timeout; landing normally ends the leap first.
pub const LEAP_COOLDOWN: f32 = 6.0;
pub const BURROW_DURATION: f32 = 4.0;
pub const BURROW_COOLDOWN: f32 = 8.0;
pub const EMERGE_HOLD: f32 = 0.6;
pub const CHARGE_DURATION: f32 = 3.0;
pub const CHARGE_COOLDOWN: f32 = 8.0;
pub const CHARGE_SPEED: f32 = movement::WALK_SPEED * 5.0;
pub const CHARGE_DAMAGE: f32 = 30.0;

mod dragon;
pub const SURGE_CHARGES: u8 = 3;
pub const SURGE_RECHARGE: f32 = 3.0;
pub const SURGE_DURATION: f32 = 0.15;
pub const SURGE_SPEED: f32 = 25.0;

#[derive(Debug, Clone, Copy)]
pub struct State {
    pub remaining: f32,
    pub cooldown: f32,
    pub down: bool,
    pub charge: f32,
    /// Consume the emergence click until it is released, including after surfacing.
    pub fire_latched: bool,
    pub velocity: V3,
    /// Authority-only: each enemy can take one body hit per charge.
    pub hit_mask: u16,
    pub surge_charges: u8,
    pub surge_recharge: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            remaining: 0.0,
            cooldown: 0.0,
            down: false,
            charge: 0.0,
            fire_latched: false,
            velocity: V3::default(),
            hit_mask: 0,
            surge_charges: SURGE_CHARGES,
            surge_recharge: 0.0,
        }
    }
}

impl State {
    pub fn active(&self) -> bool {
        self.remaining > 0.0
    }

    pub fn cancel(&mut self) {
        self.remaining = 0.0;
        self.velocity = V3::default();
    }
}

pub fn has_surge(race: Race) -> bool {
    matches!(race, Race::Fairy | Race::Witch)
}

fn channeling_blocks_skill(a: &Actor) -> bool {
    if has_surge(character(a.character).race) {
        a.combat.channeling_blocks_surge()
    } else {
        a.combat.channeling()
    }
}

pub fn can_activate(map: &[Block], a: &Actor) -> bool {
    a.alive()
        && matches!(
            character(a.character).race,
            Race::Beast | Race::Spirit | Race::Fairy | Race::Witch | Race::Dragon
        )
        && !a.race_skill.active()
        && a.race_skill.cooldown <= 0.0
        && !a.combat.skills_locked()
        && !channeling_blocks_skill(a)
        && (!has_surge(character(a.character).race) || a.race_skill.surge_charges > 0)
        && (character(a.character).race != Race::Spirit || movement::grounded(map, a.pos))
        && !movement::blocked_with_height(map, a.pos, a.collision_height())
}

pub fn charging(a: &Actor) -> bool {
    character(a.character).race == Race::Dragon && a.race_skill.active()
}

/// Also used for interruption, swapping heroes and portal travel.
pub fn finish_charge(a: &mut Actor) {
    if charging(a) {
        a.race_skill.cancel();
        a.race_skill.cooldown = CHARGE_COOLDOWN;
        a.race_skill.hit_mask = 0;
    }
}

fn tick(timer: &mut f32) {
    *timer = (*timer - DT).max(0.0);
    if *timer < DT * 0.01 {
        *timer = 0.0;
    }
}

pub fn advance(map: &[Block], a: &mut Actor, input: Input) {
    let race = character(a.character).race;
    let pressed = input.phase_shift && !a.race_skill.down;
    a.race_skill.down = input.phase_shift;
    if !input.fire {
        a.race_skill.fire_latched = false;
    }
    tick(&mut a.race_skill.cooldown);
    // Keep the shared charge bank across hero/race swaps. Spending another charge
    // does not restart an already-running, sequential recharge.
    if a.race_skill.surge_charges < SURGE_CHARGES {
        tick(&mut a.race_skill.surge_recharge);
        if a.race_skill.surge_recharge == 0.0 {
            a.race_skill.surge_charges += 1;
            if a.race_skill.surge_charges < SURGE_CHARGES {
                a.race_skill.surge_recharge = SURGE_RECHARGE;
            }
        }
    }
    if a.race_skill.active() {
        if race == Race::Dragon
            && (a.race_skill.remaining <= DT * 1.01
                || !a.alive()
                || a.combat.disabled()
                || a.combat.channeling())
        {
            finish_charge(a);
            return;
        }
        tick(&mut a.race_skill.remaining);
        if race == Race::Spirit {
            a.race_skill.fire_latched = input.fire;
            a.race_skill.charge = if input.fire {
                (a.race_skill.charge + DT).min(EMERGE_HOLD)
            } else {
                0.0
            };
            if a.race_skill.remaining == 0.0
                || a.race_skill.charge >= EMERGE_HOLD - DT * 0.01
                || !movement::grounded(map, a.pos)
            {
                a.race_skill.remaining = 0.0;
                a.race_skill.charge = 0.0;
                a.race_skill.cooldown = BURROW_COOLDOWN;
                a.vy = 6.0;
            }
        } else if a.race_skill.remaining == 0.0 || a.combat.disabled() || channeling_blocks_skill(a) {
            a.race_skill.cancel();
        }
        if a.race_skill.active() || !has_surge(race) {
            return;
        }
    }
    if !pressed || !can_activate(map, a) {
        return;
    }
    a.think = 0.0;
    a.dancing = false;
    a.wave_ticks = 0;
    a.portals.momentum = V3::default();
    if race == Race::Beast {
        let aim = crate::portal::aimed_input(a, input);
        a.race_skill.remaining = LEAP_DURATION;
        a.race_skill.cooldown = if a.combat.komi.giant() { crate::komi_skill::LEAP_COOLDOWN } else { LEAP_COOLDOWN };
        a.race_skill.velocity = direction(aim.yaw, 0.0) * (12.0 * aim.pitch.cos().max(0.25));
        a.vy = (10.0 + aim.pitch.sin() * 4.0).clamp(8.0, 14.0);
    } else if race == Race::Dragon {
        a.race_skill.remaining = CHARGE_DURATION;
        a.race_skill.hit_mask = 0;
    } else if has_surge(race) {
        let aim = crate::portal::aimed_input(a, input);
        a.race_skill.remaining = SURGE_DURATION;
        a.race_skill.velocity = direction(aim.yaw, 0.0) * SURGE_SPEED;
        if a.race_skill.surge_charges == SURGE_CHARGES {
            a.race_skill.surge_recharge = SURGE_RECHARGE;
        }
        a.race_skill.surge_charges -= 1;
    } else {
        a.race_skill.remaining = BURROW_DURATION;
        a.race_skill.charge = 0.0;
        a.race_skill.fire_latched = input.fire;
        a.vy = 0.0;
    }
}

pub fn after_movement(map: &[Block], a: &mut Actor) {
    if character(a.character).race == Race::Beast
        && a.race_skill.active()
        && a.vy <= 0.0
        && movement::grounded(map, a.pos)
    {
        a.race_skill.remaining = 0.0;
        a.race_skill.velocity = V3::default();
    }
}
