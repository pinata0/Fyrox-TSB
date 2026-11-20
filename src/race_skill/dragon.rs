//! Authoritative body contacts along the route actually walked by either player.
use super::{charging, CHARGE_DAMAGE};
use crate::{
    combat::AttackClass,
    game::{ray_box, team, World, PLAYERS, V3},
    movement::BODY_RADIUS,
};

impl World {
    pub(crate) fn dragon_charge_contact(&mut self, mover: usize, from: V3, to: V3) {
        for other in 0..PLAYERS {
            if team(mover) == team(other) {
                continue;
            }
            for (source, target) in [(mover, other), (other, mover)] {
                let a = &self.actors[source];
                let b = &self.actors[target];
                if !a.alive()
                    || !charging(a)
                    || a.combat.disabled()
                    || a.combat.channeling()
                    || a.race_skill.hit_mask & (1 << target) != 0
                    || !b.alive()
                    || b.burrowed()
                {
                    continue;
                }
                let moving = &self.actors[mover];
                let stationary = &self.actors[other];
                let radius = BODY_RADIUS * (moving.body_scale() + stationary.body_scale());
                // Expanded body AABB: ray parameter is the fraction of this segment.
                let Some(t) = ray_box(
                    from - stationary.pos,
                    to - from,
                    V3::new(-radius, -moving.hit_height() + 0.001, -radius),
                    V3::new(radius, stationary.hit_height() - 0.001, radius),
                )
                .filter(|t| *t <= 1.0) else {
                    continue;
                };
                let contact = from + (to - from) * t;
                let bottom = contact.y.max(stationary.pos.y);
                let top = (contact.y + moving.hit_height())
                    .min(stationary.pos.y + stationary.hit_height());
                let height = (bottom + top) * 0.5;
                // Body extents must not deal damage through a thin wall or floor.
                if !self.visible_between(
                    V3::new(contact.x, height, contact.z),
                    V3::new(stationary.pos.x, height, stationary.pos.z),
                ) {
                    continue;
                }
                let id = a.character;
                if self.damage_hit(source, target, CHARGE_DAMAGE, AttackClass::Skill, id) {
                    self.actors[source].race_skill.hit_mask |= 1 << target;
                }
            }
        }
    }
}
