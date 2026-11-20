//! Server-owned ground contacts. Skills may register a volume without changing
//! movement: normal walking and fast racial movement share the same swept path.
use crate::{
    combat::AttackClass,
    game::{ray_box, team, World, DT, PLAYERS, V3},
};

#[derive(Debug, Clone)]
pub struct GroundEffect {
    pub owner: usize,
    pub character: u8,
    pub min: V3,
    pub max: V3,
    pub damage: f32,
    pub stun: f32,
    pub remaining: f32,
    pub interval: f32,
    pub class: AttackClass,
    /// Per-target cadence is independent of the number of movement substeps.
    pub next_hit: [u64; PLAYERS],
}

impl GroundEffect {
    fn contact(&self, from: V3, to: V3) -> Option<f32> {
        // Test the feet against the volume, so jumping over a ground-only field
        // does not trigger it. A segment test catches arbitrarily thin fields.
        ray_box(from, to - from, self.min, self.max).filter(|t| *t <= 1.0)
    }
}

impl World {
    pub(crate) fn apply_ground_contacts(&mut self, slot: usize, path: &mut Vec<(V3, V3)>) {
        let destination = self.actors[slot].pos;
        for segment in 0..path.len() {
            let (from, to) = path[segment];
            let mut contacts: Vec<_> = self
                .ground_effects
                .iter()
                .enumerate()
                .filter(|(_, effect)| {
                    effect.owner < PLAYERS
                        && team(effect.owner) != team(slot)
                        && effect.remaining > 0.0
                        && effect.next_hit[slot] <= self.tick
                })
                .filter_map(|(index, effect)| effect.contact(from, to).map(|t| (t, index)))
                .collect();
            contacts.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (t, index) in contacts {
                self.actors[slot].pos = from + (to - from) * t;
                let effect = &self.ground_effects[index];
                let (owner, character, damage, stun, class) = (
                    effect.owner,
                    effect.character,
                    effect.damage,
                    effect.stun,
                    effect.class,
                );
                if self.damage_hit(owner, slot, damage, class, character) {
                    let effect = &mut self.ground_effects[index];
                    effect.next_hit[slot] = self
                        .tick
                        .saturating_add((effect.interval.max(DT) / DT).ceil() as u64);
                    let a = &mut self.actors[slot];
                    if a.alive() && a.combat.sleep <= 0.0 {
                        a.combat.stun = a.combat.stun.max(stun);
                    }
                    if !a.alive() || a.combat.disabled() {
                        crate::race_skill::finish_charge(a);
                        a.race_skill.cancel();
                        a.combat.dash = 0.0;
                        a.portals.momentum = V3::default();
                        path[segment].1 = a.pos;
                        path.truncate(segment + 1);
                        return;
                    }
                }
            }
        }
        self.actors[slot].pos = destination;
    }
}
