//! Presentation-only cues from authoritative snapshots. Never changes gameplay or the wire format.
use crate::{game::*, impact::ImpactKind, level::MapId, movement};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    Fire(u8),
    SebastianRoar,
    Skill(u8),
    Ultimate(u8),
    Step(u8),
    Jump,
    Land,
    Dash,
    Portal,
    Hit,
    Hurt,
    ShieldBreak,
    Wall,
    SlashImpact,
    Reload,
    ReloadDone,
    Death,
    Respawn,
    Ready,
    RoundStart,
    Capture,
    Win,
    Lose,
    Click,
    Confirm,
    Back,
}

#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub cue: Cue,
    /// Authoritative caster slot for skill/ultimate voice playback.
    pub speaker: Option<usize>,
    /// None is a local/UI cue; Some is attenuated and panned in world space.
    pub position: Option<V3>,
    pub gain: f32,
    pub variation: u32,
}
impl Event {
    pub fn local(cue: Cue) -> Self {
        Self {
            cue,
            speaker: None,
            position: None,
            gain: 1.0,
            variation: 0,
        }
    }
    fn actor(cue: Cue, a: &Actor, local: bool, variation: u32) -> Self {
        Self {
            cue,
            speaker: None,
            position: (!local).then_some(a.eye()),
            gain: 1.0,
            variation,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ActorFrame {
    active: bool,
    character: u8,
    alive: bool,
    pos: V3,
    vy: f32,
    shot: u32,
    hp: u16,
    ammo: u8,
    reload: f32,
    skill: f32,
    ultimate: f32,
    hit: f32,
    guard: f32,
    race: f32,
    phase: f32,
    transits: u16,
    portal: [f32; 2],
    grounded: bool,
    roaring: bool,
}
impl ActorFrame {
    fn new(a: &Actor, w: &World) -> Self {
        Self {
            roaring: a.combat.chloe.phase==crate::chloe_skill::Phase::Roar && a.combat.chloe.elapsed>=crate::chloe_skill::ROAR_RELEASE,
            active: a.active,
            character: a.character,
            alive: a.alive(),
            pos: a.pos,
            vy: a.vy,
            shot: a.shot,
            hp: a.hp,
            ammo: a.ammo,
            reload: a.reload,
            skill: a.ability_cooldown,
            ultimate: a.combat.ultimate,
            hit: a.hit,
            guard: a.combat.guard,
            race: a.race_skill.remaining,
            phase: a.phase_skill.remaining,
            transits: a.portals.transits,
            portal: a.portals.cooldown,
            grounded: a.vy <= 0.01 && movement::grounded(&w.map, a.pos),
        }
    }
}
#[derive(Debug)]
struct Frame {
    tick: u64,
    round: u16,
    map: MapId,
    phase: Phase,
    point: usize,
    slot: usize,
    actors: [ActorFrame; PLAYERS],
}
impl Frame {
    fn new(w: &World, slot: usize) -> Self {
        Self {
            tick: w.tick,
            round: w.round,
            map: w.map_id,
            phase: w.phase,
            point: w.point,
            slot,
            actors: std::array::from_fn(|i| ActorFrame::new(&w.actors[i], w)),
        }
    }
}

#[derive(Debug, Default)]
pub struct Tracker {
    previous: Option<Frame>,
    steps: [f32; PLAYERS],
    step_serial: [u32; PLAYERS],
    impacts: Vec<u32>,
    last_hit: Option<u64>,
    last_hurt: Option<u64>,
}
impl Tracker {
    /// Priming while hidden/muted prevents old combat from bursting out on resume.
    pub fn update(&mut self, w: &World, slot: usize, audible: bool) -> Vec<Event> {
        let Some(old) = self.previous.take() else {
            self.prime(w, slot);
            return vec![];
        };
        let reset = old.round != w.round
            || old.map != w.map_id
            || w.tick < old.tick
            || old.slot != slot
            || w.tick.saturating_sub(old.tick) > 60;
        if reset || !audible {
            self.prime(w, slot);
            return vec![];
        }
        if old.tick == w.tick && old.phase == w.phase {
            self.previous = Some(old);
            return vec![];
        }
        let mut events = Vec::new();
        if old.phase != w.phase {
            match w.phase {
                Phase::Playing => events.push(Event::local(Cue::RoundStart)),
                Phase::Intermission | Phase::Finished => {
                    if let Some(winner) = w.result {
                        events.push(Event::local(if winner == team(slot) {
                            Cue::Win
                        } else {
                            Cue::Lose
                        }));
                    }
                }
                _ => {}
            }
        }
        if w.point > old.point {
            events.push(Event::local(Cue::Capture));
        }
        for (i, a) in w.actors.iter().enumerate() {
            let p = old.actors[i];
            let local = i == slot;
            if !a.active || !p.active || a.character != p.character {
                self.steps[i] = 0.0;
                continue;
            }
            if !a.alive() {
                self.steps[i] = 0.0;
                if p.alive && local {
                    events.push(Event::local(Cue::Death));
                }
                continue;
            }
            if !p.alive {
                self.steps[i] = 0.0;
                if local {
                    events.push(Event::local(Cue::Respawn));
                }
                continue;
            }
            if !matches!(w.phase, Phase::Playing | Phase::Setup) && !w.free_attack {
                continue;
            }
            // An increasing counter identifies an actual discharge, not a held mouse button.
            // Coalesce dropped packets to one cue instead of replaying an entire burst.
            if a.shot != p.shot && a.shot.wrapping_sub(p.shot) < 64 {
                events.push(Event::actor(Cue::Fire(a.character), a, local, a.shot));
            }
            if a.ability_cooldown > p.skill + 0.25 {
                events.push(Event {
                    speaker: Some(i),
                    ..Event::actor(Cue::Skill(a.character), a, local, a.shot)
                });
            }
            if p.ultimate >= 99.0 && a.combat.ultimate < p.ultimate - 50.0 {
                events.push(Event {
                    speaker: Some(i),
                    ..Event::actor(Cue::Ultimate(a.character), a, local, a.shot)
                });
            } else if local && p.ultimate < 100.0 && a.combat.ultimate >= 100.0 {
                events.push(Event::local(Cue::Ready));
            }
            if a.character==6 && a.combat.chloe.phase==crate::chloe_skill::Phase::Roar && a.combat.chloe.elapsed>=crate::chloe_skill::ROAR_RELEASE && !p.roaring {
                events.push(Event::actor(Cue::SebastianRoar,a,local,a.shot));
            }
            if a.reload > 0.0 && p.reload <= 0.0 {
                events.push(Event::actor(Cue::Reload, a, local, 0));
            } else if a.reload <= 0.0 && p.reload > 0.0 && a.ammo > p.ammo {
                events.push(Event::actor(Cue::ReloadDone, a, local, 0));
            }
            if local && a.hit > p.hit + 0.005 && self.last_hit.is_none_or(|t| w.tick - t >= 5) {
                self.last_hit = Some(w.tick);
                events.push(Event::local(Cue::Hit));
            }
            if local && a.hp < p.hp && self.last_hurt.is_none_or(|t| w.tick - t >= 10) {
                self.last_hurt = Some(w.tick);
                events.push(Event::local(Cue::Hurt));
            }
            if p.guard > 0.0 && a.combat.guard <= 0.0 && a.hp < p.hp {
                events.push(Event::actor(Cue::ShieldBreak, a, local, 0));
            }
            if (a.race_skill.remaining > 0.0 && p.race <= 0.0)
                || (a.phase_skill.remaining > 0.0 && p.phase <= 0.0)
            {
                events.push(Event::actor(Cue::Dash, a, local, 0));
            }
            if a.portals.transits != p.transits
                || (0..2).any(|j| a.portals.cooldown[j] > p.portal[j] + 0.25)
            {
                events.push(Event::actor(Cue::Portal, a, local, 0));
            }
            let travel = (a.pos - p.pos).length();
            let grounded = a.vy <= 0.01 && movement::grounded(&w.map, a.pos);
            let moving_skill = a.race_skill.active() || a.burrowed() || a.phase_skill.active();
            if travel < 2.5 && !moving_skill {
                if p.grounded && a.vy > 1.0 {
                    events.push(Event::actor(Cue::Jump, a, local, 0));
                } else if !p.grounded && grounded && p.vy < -1.0 {
                    events.push(Event::actor(Cue::Land, a, local, 0));
                }
                if grounded && p.grounded && !a.dancing {
                    self.steps[i] += (a.pos - p.pos).flat().length();
                    if self.steps[i] >= if a.crouching { 1.35 } else { 1.9 } {
                        self.steps[i] = 0.0;
                        self.step_serial[i] = self.step_serial[i].wrapping_add(1);
                        let mut e = Event::actor(
                            Cue::Step((self.step_serial[i] % 2) as u8),
                            a,
                            local,
                            self.step_serial[i],
                        );
                        e.gain = if a.crouching {
                            0.24
                        } else if local {
                            0.55
                        } else {
                            1.0
                        };
                        events.push(e);
                    }
                } else {
                    self.steps[i] = 0.0;
                }
            } else {
                self.steps[i] = 0.0;
            }
        }
        for hit in &w.impacts {
            if !self.impacts.contains(&hit.id) && w.tick.saturating_sub(hit.tick) <= 12 {
                events.push(Event {
                    cue: if hit.kind == ImpactKind::Slash {
                        Cue::SlashImpact
                    } else {
                        Cue::Wall
                    },
                    position: Some(hit.position),
                    speaker: None,
                    gain: 1.0,
                    variation: hit.id,
                });
            }
        }
        self.impacts = w.impacts.iter().map(|h| h.id).collect();
        self.previous = Some(Frame::new(w, slot));
        events
    }

    fn prime(&mut self, w: &World, slot: usize) {
        self.previous = Some(Frame::new(w, slot));
        self.steps.fill(0.0);
        self.impacts = w.impacts.iter().map(|h| h.id).collect();
        self.last_hit = None;
        self.last_hurt = None;
    }
}

#[cfg(test)]
mod tests;
