//! Secondary feather motion. Translation always belongs to the skinned attachment.
use super::*;

#[derive(Debug, Default)]
pub(super) struct Wings {
    chains: [[Handle<Node>; 2]; 2],
    parent: Handle<Node>,
    previous: Option<(Vector3<f32>, UnitQuaternion<f32>)>,
    speed: Vector3<f32>,
    angle: Vector3<f32>,
    velocity: Vector3<f32>,
}
impl Wings {
    pub fn new(g: &Graph, root: Handle<Node>, id: u8) -> Self {
        if id != 2 {
            return Self::default();
        }
        let find = |name: &str| {
            g.traverse_handle_iter(root)
                .find(|h| g[*h].name() == name)
                .unwrap_or_default()
        };
        Self {
            parent: find("Head"),
            chains: ["L", "R"].map(|s| [find(&format!("Wing.{s}")), find(&format!("WingTip.{s}"))]),
            ..Default::default()
        }
    }
    pub fn reset(&mut self) {
        self.previous = None;
        self.speed.fill(0.0);
        self.angle.fill(0.0);
        self.velocity.fill(0.0);
    }
    pub fn apply(&mut self, g: &mut Graph, dt: f32) {
        if self.parent.is_none() || self.chains.iter().flatten().any(|h| h.is_none()) || dt <= 0.0 {
            return;
        }
        g.update_hierarchical_data_for_descendants(g[self.parent].parent());
        let position = g[self.parent].global_position();
        let rotation = super::skeleton::world_rotation(g, self.parent);
        let dt = dt.min(0.05);
        if let Some((previous, orientation)) = self.previous {
            if (position - previous).norm() > 0.75 || orientation.angle_to(&rotation) > 1.5 {
                self.reset();
            } else {
                let speed = (position - previous) / dt;
                let acceleration = rotation.inverse_transform_vector(&((speed - self.speed) / dt));
                let turn = (orientation.inverse() * rotation).scaled_axis() / dt;
                self.speed = speed;
                let force = Vector3::new(
                    -turn.x * 1.5 - acceleration.z * 0.02,
                    -turn.y * 1.8,
                    acceleration.y * 0.015,
                );
                let target = force.map(|x| x.clamp(-4.0, 4.0));
                for _ in 0..4 {
                    self.velocity +=
                        (target - self.angle * 40.0 - self.velocity * 10.0) * (dt / 4.0);
                    self.angle += self.velocity * (dt / 4.0);
                    self.angle = self.angle.map(|x| x.clamp(-0.28, 0.28));
                }
            }
        }
        self.previous = Some((position, rotation));
        for (side, chain) in self.chains.iter().enumerate() {
            let sign = if side == 0 { -1.0 } else { 1.0 };
            for (segment, &node) in chain.iter().enumerate() {
                let gain = if segment == 0 { 1.0 } else { 0.7 };
                let q = UnitQuaternion::from_euler_angles(
                    self.angle.x * gain,
                    sign * self.angle.y * gain,
                    sign * self.angle.z * gain,
                );
                let base = super::skeleton::world_rotation(g, node);
                super::skeleton::set_global_rotation(
                    g,
                    node,
                    rotation * q * rotation.inverse() * base,
                );
            }
        }
    }
}
