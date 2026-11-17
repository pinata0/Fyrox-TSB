//! Emote camera geometry shared by the renderer and headless regression tests.
use crate::game::{direction, ray_box, Block, V3};

#[derive(Debug, Clone, Copy)]
pub struct OrbitCamera {
    pub active: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}
impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            active: false,
            yaw: 0.0,
            pitch: -0.12,
            distance: 3.4,
        }
    }
}
impl OrbitCamera {
    pub fn enter(&mut self, actor_yaw: f32) {
        self.active = true;
        self.yaw = actor_yaw + std::f32::consts::PI;
        self.pitch = -0.12;
    }
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        self.yaw = (self.yaw - dx * 0.0022).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch - dy * 0.0022).clamp(-0.75, 0.45);
    }
    pub fn zoom(&mut self, steps: f32) {
        self.distance = (self.distance - steps * 0.35).clamp(1.8, 6.5);
    }
    pub fn position(&self, feet: V3, map: &[Block]) -> V3 {
        let focus = feet + V3::new(0.0, 1.05, 0.0);
        let back = direction(self.yaw, self.pitch) * -1.0;
        let padding = V3::new(0.18, 0.18, 0.18);
        let mut distance = self.distance;
        for b in map {
            if let Some(hit) = ray_box(focus, back, b.min() - padding, b.max() + padding) {
                distance = distance.min((hit - 0.04).max(0.0));
            }
        }
        if back.y < 0.0 {
            distance = distance.min(((focus.y - 0.16) / -back.y).max(0.0));
        }
        focus + back * distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Input, Phase, World, PLAYERS};
    fn emote_world() -> World {
        let mut w = World::default();
        w.actors[0].human = true;
        w
    }
    #[test]
    fn dance_is_edge_triggered_and_movement_cancels() {
        let mut w = emote_world();
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0].dance = true;
        w.tick(inputs);
        assert!(w.actors[0].dancing);
        for _ in 0..10 {
            w.tick(inputs);
        }
        assert!(w.actors[0].dancing);
        inputs[0].forward = 1.0;
        w.tick(inputs);
        assert!(!w.actors[0].dancing);
        inputs[0].forward = 0.0;
        w.tick(inputs);
        assert!(!w.actors[0].dancing);
        inputs[0].dance = false;
        w.tick(inputs);
        inputs[0].dance = true;
        w.tick(inputs);
        assert!(w.actors[0].dancing);
        inputs[0].dance = false;
        w.tick(inputs);
        inputs[0].dance = true;
        w.tick(inputs);
        assert!(!w.actors[0].dancing);
    }
    #[test]
    fn dance_cannot_start_during_combat_lock_or_in_air() {
        for mode in 0..4 {
            let mut w = emote_world();
            match mode {
                0 => w.actors[0].hp = 0,
                1 => w.actors[0].combat.stun = 1.0,
                2 => w.actors[0].pos.y = 1.0,
                _ => w.actors[0].hurt = 0.2,
            }
            let mut inputs = [Input::default(); PLAYERS];
            inputs[0].dance = true;
            w.tick(inputs);
            assert!(!w.actors[0].dancing);
        }
    }
    #[test]
    fn orbit_does_not_turn_actor_and_q_cancels_dance() {
        let mut w = emote_world();
        w.phase = Phase::Playing;
        for a in &mut w.actors {
            a.human = true;
        }
        let old = w.actors[0].yaw;
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0].dance = true;
        inputs[0].yaw = 2.0;
        w.tick(inputs);
        assert!(w.actors[0].dancing);
        assert_eq!(w.actors[0].yaw, old);
        w.actors[0].combat.ultimate = 100.0;
        inputs[0].ultimate = true;
        w.tick(inputs);
        assert!(!w.actors[0].dancing);
        assert!(w.actors[0].combat.ultimate < 100.0);
    }
    #[test]
    fn orbit_obeys_walls_ground_and_zoom_bounds() {
        let mut cam = OrbitCamera::default();
        cam.enter(0.0);
        let wall = Block {
            center: V3::new(0.0, 1.0, 2.0),
            size: V3::new(5.0, 3.0, 0.4),
            color: [0; 3],
            phaseable: true,
        };
        let p = cam.position(V3::default(), &[wall]);
        assert!(p.z < 1.63);
        cam.zoom(-100.0);
        assert_eq!(cam.distance, 6.5);
        cam.zoom(100.0);
        assert_eq!(cam.distance, 1.8);
        cam.rotate(0.0, -10000.0);
        let p = cam.position(V3::default(), &[]);
        assert!(p.y >= 0.159);
        cam.rotate(1e6, 1e6);
        assert!((0.0..std::f32::consts::TAU).contains(&cam.yaw));
    }
    #[test]
    fn damage_death_selection_and_bot_takeover_cancel_dance() {
        use crate::combat::AttackClass;
        let mut w = emote_world();
        w.actors[0].pos = crate::game::POINTS[0];
        w.actors[0].shield = 0.0;
        w.actors[0].dancing = true;
        assert!(w.damage_hit(6, 0, 5.0, AttackClass::Basic, 14));
        assert!(!w.actors[0].dancing);
        w.actors[0].dancing = true;
        w.select_character(0, 4);
        assert!(!w.actors[0].dancing);
        w.actors[0].dancing = true;
        w.actors[0].human = false;
        w.tick([Input::default(); PLAYERS]);
        assert!(!w.actors[0].dancing);
        w.actors[0].human = true;
        w.actors[0].hp = 1;
        w.actors[0].dancing = true;
        w.damage_hit(6, 0, 100.0, AttackClass::Basic, 14);
        assert!(!w.actors[0].alive());
        assert!(!w.actors[0].dancing);
    }
}
