//! Match totals survive deaths, hero changes and rounds; a new match resets them.
use crate::game::{team, World, DT, PLAYERS};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MatchStats {
    pub assists: u16,
    pub solo_kills: u16,
    pub damage: f32,
    pub healing: f32,
    pub mitigated: f32,
}

impl World {
    pub(crate) fn record_damage(
        &mut self,
        source: usize,
        target: usize,
        damage: f32,
        killed: bool,
    ) {
        if damage > 0.0 {
            self.actors[source].stats.damage += damage;
            self.actors[target].contributors[source] = Some(self.tick);
        }
        if !killed {
            return;
        }
        let contributors = self.actors[target].contributors;
        let mut assisted = false;
        for (slot, tick) in contributors.into_iter().enumerate() {
            if slot != source
                && team(slot) == team(source)
                && self.actors[slot].active
                && tick.is_some_and(|t| self.tick.saturating_sub(t) as f32 * DT <= 10.0)
            {
                self.actors[slot].stats.assists = self.actors[slot].stats.assists.saturating_add(1);
                assisted = true;
            }
        }
        if !assisted {
            self.actors[source].stats.solo_kills =
                self.actors[source].stats.solo_kills.saturating_add(1);
        }
        self.actors[target].contributors = [None; PLAYERS];
    }

    /// Only effective healing counts; passive out-of-combat regeneration is excluded.
    pub(crate) fn heal_from(&mut self, source: usize, target: usize, amount: f32) {
        let before = self.actors[target].hp;
        self.heal(target, amount);
        self.actors[source].stats.healing += self.actors[target].hp.saturating_sub(before) as f32;
    }
}

#[cfg(test)]
mod tests;
