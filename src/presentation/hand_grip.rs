//! Mesh-derived palm contacts and articulated open/closed hand poses.
use super::*;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
struct Control {
    name: String,
    axis: [f32; 3],
    angle: f32,
}
#[derive(Clone, Deserialize)]
struct Profile {
    hand: String,
    socket: String,
    controls: Vec<Control>,
}
static PROFILES: std::sync::OnceLock<Vec<Vec<Profile>>> = std::sync::OnceLock::new();

pub(super) fn load() -> Result<(), String> {
    let profiles = characters::SLUGS
        .iter()
        .map(|slug| {
            let bytes = std::fs::read(characters::asset_path(&format!("{slug}-hands.json")))
                .map_err(|e| e.to_string())?;
            let hands: Vec<Profile> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if hands.len() != 2 || hands.iter().any(|h| h.controls.len() != 15) {
                return Err(format!("{slug}: incomplete finger rig"));
            }
            Ok(hands)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let _ = PROFILES.set(profiles);
    Ok(())
}

type Joint = (Handle<Node>, Vector3<f32>, f32, UnitQuaternion<f32>);

#[derive(Debug)]
pub(super) struct Rig {
    pub hand: Handle<Node>,
    pub socket: Handle<Node>,
    joints: Vec<Joint>,
    closure_amount: f32,
}
impl Rig {
    pub fn new(g: &Graph, root: Handle<Node>, id: u8, left: bool) -> Self {
        let p = &PROFILES.get().expect("hand profiles loaded")[id as usize][usize::from(left)];
        let find = |name: &str| {
            g.traverse_handle_iter(root)
                .find(|h| g[*h].name() == name)
                .unwrap_or_else(|| panic!("missing hand control {name}"))
        };
        Self {
            hand: find(&p.hand),
            socket: find(&p.socket),
            closure_amount: amount(id),
            joints: p
                .controls
                .iter()
                .map(|c| {
                    let h = find(&c.name);
                    (
                        h,
                        Vector3::from(c.axis),
                        c.angle,
                        **g[h].local_transform().rotation(),
                    )
                })
                .collect(),
        }
    }
    pub fn fingers(&self) -> Vec<Handle<Node>> {
        self.joints.iter().map(|j| j.0).collect()
    }
    pub fn contact(&self, g: &Graph) -> Vector3<f32> {
        g[self.socket].global_position()
    }
    pub fn orient(&self, g: &mut Graph, desired: UnitQuaternion<f32>) {
        // Share axial twist with the forearm so the cuff does not fold over the palm.
        let forearm = g[self.hand].parent();
        if let Some(axis) =
            (g[self.hand].global_position() - g[forearm].global_position()).try_normalize(1e-6)
        {
            let from = super::skeleton::world_rotation(g, self.socket) * Vector3::y();
            let to = desired * Vector3::y();
            if let (Some(from), Some(to)) = (
                (from - axis * from.dot(&axis)).try_normalize(1e-4),
                (to - axis * to.dot(&axis)).try_normalize(1e-4),
            ) {
                let angle = axis.dot(&from.cross(&to)).atan2(from.dot(&to));
                let turn = UnitQuaternion::from_axis_angle(
                    &fyrox::core::algebra::Unit::new_normalize(axis),
                    angle,
                );
                super::skeleton::set_global_rotation(
                    g,
                    forearm,
                    turn * super::skeleton::world_rotation(g, forearm),
                );
            }
        }
        let delta = desired * super::skeleton::world_rotation(g, self.socket).inverse();
        super::skeleton::set_global_rotation(
            g,
            self.hand,
            delta * super::skeleton::world_rotation(g, self.hand),
        );
    }
    pub fn pose(&self, g: &mut Graph, amount: f32) {
        for (h, _, _, neutral) in &self.joints {
            g[*h].local_transform_mut().set_rotation(*neutral);
        }
        g.update_hierarchical_data_for_descendants(self.hand);
        for (h, axis, angle, _) in &self.joints {
            let axis = g[self.hand]
                .global_transform()
                .transform_vector(axis)
                .normalize();
            let delta = UnitQuaternion::from_axis_angle(
                &fyrox::core::algebra::Unit::new_normalize(axis),
                angle * amount,
            );
            super::skeleton::set_global_rotation(
                g,
                *h,
                delta * super::skeleton::world_rotation(g, *h),
            );
        }
    }
    pub fn verify(&self, g: &Graph, closed: bool) {
        assert_eq!(
            self.joints.len(),
            15,
            "all five fingers need three articulated joints"
        );
        assert!(
            (self.contact(g) - g[self.hand].global_position()).norm() > 0.015,
            "palm contact must not be the wrist"
        );
        if closed {
            for (h, _, angle, neutral) in &self.joints {
                assert!(
                    (g[*h].local_transform().rotation().angle_to(neutral)
                        - angle * self.closure_amount)
                        .abs()
                        < 0.02,
                    "finger closure differs from its grip profile: {}",
                    g[*h].name()
                );
            }
        }
    }
}

pub(super) fn amount(id: u8) -> f32 {
    if id == 0 {
        0.0
    } else if matches!(id, 7 | 9) {
        0.25
    } else {
        1.0
    }
}

pub(super) fn facing(rotation: UnitQuaternion<f32>, id: u8, left: bool) -> UnitQuaternion<f32> {
    if !left || id == 0 {
        return rotation;
    }
    let opposed = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), std::f32::consts::PI);
    if id == 14 {
        rotation
            * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -std::f32::consts::FRAC_PI_2)
            * opposed
    } else {
        rotation * opposed
    }
}

pub(super) fn solve_arm(
    g: &mut Graph,
    upper: Handle<Node>,
    fore: Handle<Node>,
    hand: Handle<Node>,
    target: Vector3<f32>,
    pole: Vector3<f32>,
) {
    let shoulder = g[upper].global_position();
    let elbow = g[fore].global_position();
    let wrist = g[hand].global_position();
    let a = (elbow - shoulder).norm();
    let b = (wrist - elbow).norm();
    let delta = target - shoulder;
    let d = delta.norm().clamp((a - b).abs() + 0.0001, a + b - 0.0001);
    let direction = delta.normalize();
    let x = (d * d + a * a - b * b) / (2.0 * d);
    let h = (a * a - x * x).max(0.0).sqrt();
    let perpendicular = (pole - direction * pole.dot(&direction))
        .try_normalize(1e-6)
        .unwrap_or(Vector3::y());
    let goal = shoulder + direction * x + perpendicular * h;
    super::skeleton::aim_joint(g, upper, fore, goal - shoulder);
    super::skeleton::aim_joint(g, fore, hand, target - g[fore].global_position());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn support_arm_reaches_handle_without_changing_bone_lengths() {
        let mut g = Graph::new();
        let upper: Handle<Node> = PivotBuilder::new(BaseBuilder::new())
            .build(&mut g)
            .to_base();
        let fore: Handle<Node> = PivotBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 0.2, 0.0))
                    .build(),
            ),
        )
        .build(&mut g)
        .to_base();
        let hand: Handle<Node> = PivotBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 0.18, 0.0))
                    .build(),
            ),
        )
        .build(&mut g)
        .to_base();
        g.link_nodes(fore, upper);
        g.link_nodes(hand, fore);
        g.update_hierarchical_data_for_descendants(upper);
        let target = Vector3::new(0.12, 0.14, 0.16);
        solve_arm(
            &mut g,
            upper,
            fore,
            hand,
            target,
            Vector3::new(-1.0, -0.5, 0.0),
        );
        assert!((g[hand].global_position() - target).norm() < 0.0001);
        assert!(
            ((g[fore].global_position() - g[upper].global_position()).norm() - 0.2).abs() < 0.0001
        );
        assert!(
            ((g[hand].global_position() - g[fore].global_position()).norm() - 0.18).abs() < 0.0001
        );
    }
}
