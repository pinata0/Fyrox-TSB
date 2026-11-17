//! Blender FBX meshes and baked skeletal motion clips. No reference art is loaded at runtime.
use super::{weapons, Shapes};
use fyrox::{
    asset::manager::ResourceManager,
    core::{
        algebra::{UnitQuaternion, Vector3},
        futures::{executor::block_on, future::join_all},
        pool::Handle,
    },
    graph::prelude::*,
    resource::model::{Model, ModelResource, ModelResourceExtension},
    scene::{
        animation::{Animation, AnimationPlayer, AnimationPoseExt},
        graph::Graph,
        node::Node,
        Scene,
    },
};
use trickcal_shooting_balddagu::game::Actor;

pub const SLUGS: [&str; 15] = [
    "vivi", "naia", "erpin", "id", "aya", "picora", "chloe", "renua", "mayo", "bella", "ui",
    "butter", "komi", "hilde", "shion",
];
use trickcal_shooting_balddagu::motion::{Motion, MotionClock, Pose};

pub(super) fn asset_path(name: &str) -> std::path::PathBuf {
    std::env::var_os("TSB_CHARACTER_ASSET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "assets/characters".into())
        .join(name)
}

pub fn load(manager: &ResourceManager) -> Result<Vec<ModelResource>, String> {
    super::cheek_masks::load()?;
    super::hand_grip::load()?;
    super::expressions::load(manager)?;
    let pending: Vec<_> = SLUGS
        .iter()
        .map(|s| manager.request::<Model>(asset_path(&format!("{s}.fbx"))))
        .collect();
    let results = block_on(join_all(pending));
    let mut models = Vec::with_capacity(15);
    for (slug, result) in SLUGS.iter().zip(results) {
        let model = result.map_err(|e| format!("Character model {slug}: {e:?}"))?;
        super::morph_import::restore(&model, slug)?;
        let data = model.data_ref();
        let mut clips = 0;
        for node in data.get_scene().graph.linear_iter() {
            if let Some(player) = node.cast::<AnimationPlayer>() {
                for a in player.animations().iter() {
                    let expected = if matches!(*slug, "vivi" | "aya") {
                        18.5
                    } else if *slug == "erpin" {
                        Motion::ErpinEat.frames().1 as f32 / 30.0
                    } else if *slug == "naia" {
                        Motion::NaiaEnd.frames().1 as f32 / 30.0
                    } else {
                        17.6
                    };
                    if (a.length() - expected).abs() < 0.02 {
                        clips += 1;
                    } else {
                        return Err(format!(
                            "{slug}: animation is {:.3}s; expected {expected}s at 30 fps",
                            a.length()
                        ));
                    }
                }
            }
        }
        if clips == 0 {
            return Err(format!("{slug}: baked animation including Spawn missing"));
        }
        drop(data);
        println!("CHARACTER_MODEL_OK: {slug}, baked timeline includes Spawn");
        models.push(model);
    }
    Ok(models)
}

#[derive(Debug)]
pub struct Visual {
    pub id: u8,
    pub root: Handle<Node>,
    pub player: Handle<Node>,
    weapon: Handle<Node>,
    hand: Handle<Node>,
    wave_arm: [Handle<Node>; 2],
    hips: Handle<Node>,
    head: Handle<Node>,
    legs: [[Handle<Node>; 3]; 2],
    crouch_drop: f32,
    motion: Motion,
    clock: MotionClock,
    arm_animations: Vec<Animation>,
    wings: std::cell::RefCell<super::erpin_wings::Wings>,
    aim_pitch: f32,
    shadow_only: super::character_shadows::ShadowOnly,
    face: super::expressions::Face,
    grips: [super::hand_grip::Rig; 2],
}
impl Visual {
    pub fn spawn(
        scene: &mut Scene,
        parent: Handle<Node>,
        resource: &ModelResource,
        id: u8,
        shapes: &Shapes,
        weapon_resource: Option<&ModelResource>,
    ) -> Self {
        let root = resource.instantiate(scene);
        let g = &mut scene.graph;
        g.link_nodes(root, parent);
        // Blender FBX stores centimeters. Fyrox preserves those import units.
        g[root]
            .local_transform_mut()
            .set_scale(Vector3::repeat(0.01));
        let view_sleeves: Vec<_> = g
            .traverse_handle_iter(root)
            .filter(|h| {
                g[*h].name().contains("_RefinedViewSleeve.")
                    || id == 7 && g[*h].name().contains("_RefinedSleeve.")
            })
            .collect();
        for node in view_sleeves {
            g[node].set_visibility(false);
            g[node].set_cast_shadows(false);
        }
        let player = g
            .traverse_handle_iter(root)
            .find(|h| g[*h].cast::<AnimationPlayer>().is_some())
            .expect("validated FBX player");
        let [upper_name, forearm_name, hand_name] = super::skeleton::arm_bone_names(false);
        let hand = g
            .traverse_handle_iter(root)
            .find(|h| g[*h].name() == hand_name)
            .unwrap_or(root);
        let weapon = weapons::build(g, shapes, id, weapon_resource);
        let wave_arm = [upper_name, forearm_name].map(|name| {
            g.traverse_handle_iter(root)
                .find(|h| g[*h].name() == name)
                .unwrap_or(Handle::NONE)
        });
        g.link_nodes(weapon, parent);
        let find = |name: &str| {
            g.traverse_handle_iter(root)
                .find(|h| g[*h].name() == name)
                .unwrap_or(Handle::NONE)
        };
        let hips = find("Hips");
        let head = find("Head");
        let legs = ["L", "R"]
            .map(|side| ["Thigh", "Shin", "Foot"].map(|bone| find(&format!("{bone}.{side}"))));
        // Mask by hierarchy so imported finger bones are included without letting
        // an attack overwrite the hips, torso, head, legs, hair or companions.
        let arms: std::collections::HashSet<_> = ["UpperArm.L", "UpperArm.R"]
            .into_iter()
            .map(find)
            .filter(|handle| handle.is_some())
            .flat_map(|arm| g.traverse_handle_iter(arm))
            .collect();
        let face = super::expressions::Face::new(g, root, id);
        let grips = [false, true].map(|left| super::hand_grip::Rig::new(g, root, id, left));
        let arm_animations = g[player]
            .cast::<AnimationPlayer>()
            .unwrap()
            .animations()
            .iter()
            .map(|animation| {
                let mut animation = animation.clone();
                animation
                    .track_bindings_mut()
                    .retain(|_, binding| arms.contains(&binding.target()));
                animation
            })
            .collect();
        let mut result = Self {
            id,
            root,
            player,
            weapon,
            hand,
            wave_arm,
            hips,
            head,
            legs,
            crouch_drop: 0.0,
            motion: Motion::Death,
            clock: MotionClock::default(),
            arm_animations,
            wings: std::cell::RefCell::new(super::erpin_wings::Wings::new(g, root, id)),
            aim_pitch: 0.0,
            shadow_only: Default::default(),
            face,
            grips,
        };
        result.set_motion(g, Motion::Idle);
        result
    }
    pub fn remove(self, g: &mut Graph) {
        g.remove_node(self.weapon);
        g.remove_node(self.root);
    }

    pub fn inherit_timeline(&mut self, previous: &Self) {
        self.clock = previous.clock.clone();
    }

    pub fn set_shadow_only(&mut self, g: &mut Graph, enabled: bool) {
        self.shadow_only
            .set_enabled(g, [self.root, self.weapon], enabled);
    }

    pub fn is_shadow_only(&self) -> bool {
        self.shadow_only.enabled()
    }

    pub fn head_position(&self, g: &Graph) -> Vector3<f32> {
        if self.head.is_some() {
            g[self.head].global_position()
        } else {
            g[self.root].global_position() + Vector3::y() * 1.22
        }
    }

    /// Run after the FBX animation has updated both hand bones.
    pub fn attach_weapons(&self, g: &mut Graph) {
        // The editable Naia FBX also contains face keys. Apply the source clock
        // after skeletal evaluation so coarse FBX samples cannot override it.
        if self.id == 1 && self.clock.pose.motion == Motion::Attack {
            super::expressions::apply(
                g,
                self.root,
                self.clock.pose.motion,
                self.clock.pose.age,
                self.id,
            );
        }
        // Animation and actor transforms have changed since the last graph update.
        // IK must read positions from the same pose as global_rotation().
        g.update_hierarchical_data_for_descendants(g[self.root].parent());
        let phase = if self.clock.pose.motion.is_attack() {
            self.clock.pose.age / Motion::Attack.seconds(self.id)
        } else {
            0.0
        };
        if self.id == 0 {
            // Open hands retain the sampled skeletal gesture. No sword is attached to a wrist.
            for grip in &self.grips {
                grip.pose(g, 0.0);
            }
            g[self.weapon]
                .local_transform_mut()
                .set_position(Vector3::new(0.0, -self.crouch_drop, 0.0))
                .set_rotation(UnitQuaternion::from_euler_angles(-self.aim_pitch, 0.0, 0.0));
            weapons::update_vivi_blades(
                g,
                self.weapon,
                phase,
                self.clock.pose.motion == Motion::Attack2,
                false,
            );
            g.update_hierarchical_data_for_descendants(self.weapon);
            return;
        }
        let motion = if self.clock.pose.motion.is_naia_skill() {
            trickcal_shooting_balddagu::naia_skill::hand(
                self.clock.pose.motion,
                self.clock.pose.age,
            )
        } else if self.id == 4 {
            trickcal_shooting_balddagu::aya_attack::hand(
                false,
                phase,
                self.clock.pose.motion == Motion::Attack2,
            )
        } else {
            trickcal_shooting_balddagu::attack::hand(self.id, false, phase)
        };
        g[self.weapon]
            .local_transform_mut()
            .set_rotation(UnitQuaternion::from_euler_angles(
                -self.aim_pitch + motion.angles[0],
                motion.angles[1],
                motion.angles[2],
            ));
        g.update_hierarchical_data_for_descendants(self.weapon);
        if self.id == 11 {
            weapons::update_slingshot(g, self.weapon, phase);
            g.update_hierarchical_data_for_descendants(self.weapon);
        }
        if self.id == 4 {
            weapons::update_aya(
                g,
                self.weapon,
                phase,
                self.clock.pose.motion == Motion::Attack2,
            );
            g.update_hierarchical_data_for_descendants(self.weapon);
        }
        let held = g[self.weapon].visibility();
        for (side, grip) in self.grips.iter().enumerate() {
            grip.pose(
                g,
                if held && (side == 0 || weapons::grips(self.id)[1].is_some()) {
                    super::hand_grip::amount(self.id)
                } else {
                    0.0
                },
            );
        }
        if self.clock.pose.motion == Motion::ErpinEat {
            let facing = super::skeleton::world_rotation(g, g[self.root].parent());
            let palm =
                UnitQuaternion::from_matrix(&fyrox::core::algebra::Matrix3::from_columns(&[
                    Vector3::z(),
                    Vector3::x(),
                    Vector3::y(),
                ]));
            self.grips[1].orient(g, facing * palm);
            self.grips[1].pose(g, 0.18);
        }
        let desired = super::skeleton::world_rotation(g, self.weapon);
        if held {
            self.grips[0].orient(g, desired);
        }
        if held && weapons::grips(self.id)[1].is_some() {
            self.align_supporting_hands(g, desired);
        }
        let parent = g[self.weapon].parent();
        let inverse = g[parent]
            .global_transform()
            .try_inverse()
            .unwrap_or_default();
        let hand_pos = inverse
            .transform_point(&self.grips[0].contact(g).into())
            .coords;
        let grip = weapons::grips(self.id)[0].unwrap();
        let transform = g[self.weapon].local_transform();
        let offset = **transform.rotation() * grip.component_mul(transform.scale());
        g[self.weapon]
            .local_transform_mut()
            .set_position(hand_pos - offset);
        g.update_hierarchical_data_for_descendants(self.weapon);
    }
    pub(super) fn palm_contact(&self, g: &Graph, left: bool) -> Vector3<f32> {
        self.grips[usize::from(left)].contact(g)
    }
    pub(super) fn verify_hand_grip(&self, g: &Graph, left: bool) {
        self.grips[usize::from(left)].verify(g, true);
    }
    fn align_supporting_hands(&self, g: &mut Graph, desired: UnitQuaternion<f32>) {
        let support_rotation = super::hand_grip::facing(desired, self.id, true);
        self.grips[1].orient(g, support_rotation);
        let arms = [false, true].map(|left| {
            super::skeleton::arm_bone_names(left).map(|name| {
                g.traverse_handle_iter(self.root)
                    .find(|h| g[*h].name() == name)
                    .unwrap()
            })
        });
        let offsets = [0, 1].map(|i| self.grips[i].contact(g) - g[arms[i][2]].global_position());
        let grip_values = weapons::grips(self.id);
        let support_node = weapons::grip_node(g, self.weapon, self.id, true);
        let difference = g[support_node]
            .global_transform()
            .transform_point(&grip_values[1].unwrap().into())
            .coords
            - g[self.weapon]
                .global_transform()
                .transform_point(&grip_values[0].unwrap().into())
                .coords;
        let centers = [0, 1].map(|i| {
            g[arms[i][0]].global_position() + offsets[i]
                - if i == 1 { difference } else { Vector3::zeros() }
        });
        let radii = [0, 1].map(|i| {
            (g[arms[i][1]].global_position() - g[arms[i][0]].global_position()).norm()
                + (g[arms[i][2]].global_position() - g[arms[i][1]].global_position()).norm()
                - 0.002
        });
        let mut contact = self.grips[0].contact(g);
        for _ in 0..24 {
            for i in 0..2 {
                let d = contact - centers[i];
                if d.norm() > radii[i] {
                    contact = centers[i] + d.normalize() * radii[i];
                }
            }
        }
        let facing = super::skeleton::world_rotation(g, g[self.weapon].parent());
        for i in 0..2 {
            let target = contact + if i == 1 { difference } else { Vector3::zeros() } - offsets[i];
            super::hand_grip::solve_arm(
                g,
                arms[i][0],
                arms[i][1],
                arms[i][2],
                target,
                facing * Vector3::new(if i == 0 { -1.0 } else { 1.0 }, -0.5, -0.4),
            );
            self.grips[i].orient(g, if i == 0 { desired } else { support_rotation });
        }
    }
    pub fn selection_pose(&mut self, g: &mut Graph, elapsed: f32) {
        self.seek(
            g,
            trickcal_shooting_balddagu::motion::selection_pose(self.id, elapsed),
        );
        g[self.weapon].set_visibility(false);
    }
    fn set_motion(&mut self, g: &mut Graph, motion: Motion) {
        self.motion = motion;
        let (start, end) = motion.frames();
        let player = g[self.player].cast_mut::<AnimationPlayer>().unwrap();
        for a in player.animations_mut().iter_mut() {
            a.set_loop(motion.looping());
            a.set_time_slice(start as f32 / 30.0..end as f32 / 30.0);
            a.rewind()
                .set_enabled(true)
                .set_speed(if motion == Motion::Death {
                    1.0 / motion.seconds(self.id)
                } else {
                    1.0
                });
        }
    }
    /// Discard live/replay transition events so old attacks cannot leak into another timeline.
    pub fn reset_timeline(&mut self, g: &mut Graph) {
        self.wings.borrow_mut().reset();
        self.clock = MotionClock::default();
        self.set_motion(g, Motion::Idle);
    }
    /// Seek the same baked clip using a recorded animation clock; evaluation still runs at speed zero.
    pub fn seek(&mut self, g: &mut Graph, pose: Pose) {
        self.clock.pose = pose;
        let (base_motion, base_age) = pose.base();
        if self.motion != base_motion {
            self.set_motion(g, base_motion);
        }
        let (start, end) = base_motion.frames();
        let duration = base_motion.seconds(self.id);
        let time = if base_motion.looping() {
            base_age.rem_euclid(duration)
        } else {
            base_age.clamp(0.0, duration)
        };
        let player = g[self.player].cast_mut::<AnimationPlayer>().unwrap();
        for anim in player.animations_mut().iter_mut() {
            anim.set_speed(0.0).set_time_position(
                start as f32 / 30.0 + (end - start) as f32 / 30.0 * time / duration,
            );
        }
        if pose.motion.is_attack() && pose.locomotion.is_some() {
            let (start, end) = pose.motion.frames();
            let phase = (pose.age / pose.motion.seconds(self.id)).clamp(0.0, 1.0);
            for anim in &mut self.arm_animations {
                anim.set_loop(false);
                anim.set_time_slice(start as f32 / 30.0..end as f32 / 30.0);
                anim.set_speed(0.0)
                    .set_time_position((start as f32 + (end - start) as f32 * phase) / 30.0);
                anim.tick(0.0);
            }
        }
        super::expressions::apply(g, self.root, pose.motion, pose.age, self.id);
        if !self.is_shadow_only() {
            self.face.show_motion(g, pose.motion, pose.age, self.id);
        }
    }
    pub fn pose(&self) -> Pose {
        self.clock.pose
    }
    /// Apply after the engine evaluates Move, before crouch/weapon IK. Every
    /// non-arm bone remains exactly at its current locomotion sample.
    pub fn layer_attack_arms(&self, g: &mut Graph) {
        let weight = self.clock.pose.arm_attack_weight(self.id);
        if weight <= 0.0 {
            return;
        }
        let layers: Vec<_> = g[self.player]
            .cast::<AnimationPlayer>()
            .unwrap()
            .animations()
            .iter()
            .zip(&self.arm_animations)
            .map(|(base, attack)| {
                let mut pose = attack.pose().clone();
                for (node, local) in pose.poses_mut() {
                    if let Some(moving) = base.pose().poses().get(node) {
                        let attack = local.clone();
                        *local = moving.clone();
                        local.blend_with(&attack, weight);
                    }
                }
                pose
            })
            .collect();
        for pose in layers {
            pose.apply(g);
        }
    }
    pub fn update_wings(&self, g: &mut Graph, dt: f32) {
        self.wings.borrow_mut().apply(g, dt);
    }
    pub fn staff_muzzle(&self, g: &Graph) -> Vector3<f32> {
        g[self.weapon]
            .global_transform()
            .transform_point(&Vector3::new(0.0, 0.72, 0.0).into())
            .coords
    }
    pub fn water_muzzle(&self, g: &Graph) -> Vector3<f32> {
        g[self.weapon]
            .global_transform()
            .transform_point(&Vector3::new(0.0, 0.10, 0.49).into())
            .coords
    }

    pub fn review_face(&mut self, g: &mut Graph, closed: bool) {
        self.selection_pose(g, Motion::Spawn.seconds(self.id));
        super::expressions::review(g, self.root, closed, self.id);
        self.face.show(g, closed);
    }

    pub fn review_blink(&mut self, g: &mut Graph, closure: f32) {
        self.selection_pose(g, Motion::Spawn.seconds(self.id));
        super::expressions::review_blink(g, self.root, closure, self.id);
        self.face.show_blink(g, closure, self.id);
    }
    /// Layer crouching over the evaluated clip, keeping both animated feet planted.
    /// World-space IK works with both procedural and imported FBX bone axes.
    pub fn crouch(&self, g: &mut Graph) {
        if self.crouch_drop <= 0.0
            || self.hips.is_none()
            || self.legs.iter().flatten().any(|h| h.is_none())
        {
            return;
        }
        let parent = g[self.root].parent();
        g.update_hierarchical_data_for_descendants(parent);
        let feet = self.legs.map(|leg| {
            (
                g[leg[2]].global_position(),
                super::skeleton::world_rotation(g, leg[2]),
            )
        });
        let hips_parent = g[self.hips].parent();
        let Some(inverse) = g[hips_parent].global_transform().try_inverse() else {
            return;
        };
        let forward = super::skeleton::world_rotation(g, parent) * Vector3::z();
        // Move the hips slightly back so short legs can fold without lifting the feet.
        let offset = inverse.transform_vector(
            &(Vector3::new(0.0, -self.crouch_drop, 0.0) - forward * (self.crouch_drop * 0.3)),
        );
        let position = **g[self.hips].local_transform().position() + offset;
        g[self.hips].local_transform_mut().set_position(position);
        g.update_hierarchical_data_for_descendants(self.hips);
        for (leg, (position, rotation)) in self.legs.iter().zip(feet) {
            super::hand_grip::solve_arm(g, leg[0], leg[1], leg[2], position, forward);
            super::skeleton::set_global_rotation(g, leg[2], rotation);
        }
    }
    /// Apply the greeting after FBX animation evaluation, independently of imported bone axes.
    pub fn wave(&self, g: &mut Graph, actor: &Actor) {
        use trickcal_shooting_balddagu::communication::wave_pose;
        if !actor.alive() || self.wave_arm.iter().any(|h| h.is_none()) {
            return;
        }
        let (blend, sway) = wave_pose(actor.wave_ticks);
        let facing = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), actor.yaw);
        for (joint, end, direction) in [
            (
                self.wave_arm[0],
                self.wave_arm[1],
                Vector3::new(-0.65, 0.75, 0.05),
            ),
            (self.wave_arm[1], self.hand, Vector3::new(sway, 1.0, 0.10)),
        ] {
            let current = g[end].global_position() - g[joint].global_position();
            let target = facing.transform_vector(&direction);
            if let Some(delta) = UnitQuaternion::rotation_between(&current, &target) {
                let rotation = super::skeleton::world_rotation(g, joint);
                let goal = delta * rotation;
                super::skeleton::set_global_rotation(g, joint, rotation.slerp(&goal, blend));
            }
        }
    }
    pub fn update(
        &mut self,
        g: &mut Graph,
        parent: Handle<Node>,
        a: &Actor,
        dt: f32,
        tick: u64,
        grounded: bool,
    ) {
        let pose = self.clock.observe(a, dt, tick, grounded);
        self.aim_pitch = a.pitch;
        self.crouch_drop = if a.alive() {
            a.crouch_drop() * a.body_scale()
        } else {
            0.0
        };
        self.seek(g, pose);
        // Meshy props are separate from the worn character mesh.
        g[self.weapon].set_visibility(
            a.alive()
                && pose.motion != Motion::Spawn
                && !a.dancing
                && a.wave_ticks == 0
                && !matches!(self.id, 3 | 6),
        );
        let inverse = g[parent]
            .global_transform()
            .try_inverse()
            .unwrap_or_default();
        let hand_pos = inverse
            .transform_point(&g[self.hand].global_position().into())
            .coords;
        g[self.weapon]
            .local_transform_mut()
            .set_position(hand_pos)
            .set_scale(Vector3::repeat(weapons::world_scale(self.id)))
            .set_rotation(UnitQuaternion::from_euler_angles(-a.pitch, 0.0, 0.0));
    }
}
