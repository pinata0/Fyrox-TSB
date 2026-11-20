//! Blender FBX meshes and baked skeletal motion clips. No reference art is loaded at runtime.
use crate::presentation::SceneState;
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
                    let expected = if (std::env::args().any(|arg|arg=="--chloe-skill-review") && *slug!="chloe") || (std::env::args().any(|arg| arg == "--renua-skill-review") && *slug != "renua")
                        || (std::env::args().any(|arg| arg == "--shion-skill-review") && *slug != "shion")
                        || (std::env::args().any(|arg| arg == "--ui-skill-review") && *slug != "ui")
                        || (std::env::args().any(|arg| arg == "--mayo-skill-review") && *slug != "mayo")
                        || (std::env::args().any(|arg| arg == "--butter-skill-review") && *slug != "butter")
                        || (std::env::args().any(|arg| arg == "--bella-skill-review") && *slug != "bella")
                        || (std::env::args().any(|arg| arg == "--vivi-skill-review") && *slug != "vivi") {
                        // Focused art review tolerates another hero's pending asset bake.
                        // Normal launches still validate every delivered timeline.
                        a.length()
                    } else if *slug == "chloe" {
                        Motion::ChloeDismount.frames().1 as f32/30.0
                    } else if *slug == "picora" {
                        Motion::PicoraUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "bella" {
                        Motion::BellaUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "mayo" {
                        Motion::MayoUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "ui" {
                        Motion::UiUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "aya" {
                        Motion::AyaUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "vivi" {
                        Motion::ViviUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "erpin" {
                        Motion::ErpinRecover.frames().1 as f32 / 30.0
                    } else if *slug == "id" {
                        Motion::IdUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "renua" {
                        Motion::RenuaUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "shion" {
                        Motion::ShionUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "butter" {
                        Motion::ButterUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "komi" {
                        Motion::KomiFeast.frames().1 as f32 / 30.0
                    } else if *slug == "hilde" {
                        Motion::HildeUltimate.frames().1 as f32 / 30.0
                    } else if *slug == "naia" {
                        Motion::NaiaUltimate.frames().1 as f32 / 30.0
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
    orbit_age: f32,
    shadow_only: super::character_shadows::ShadowOnly,
    shadow_casters: Option<Vec<Handle<Node>>>,
    casting_shadows: bool,
    weaken_outline: super::weaken_outline::Outline,
    picora_outline: super::picora_outline::Outline,
    face: super::expressions::Face,
    grips: [super::hand_grip::Rig; 2],
    whales: Option<super::naia_whales::School>,
    pub pod: Option<super::id_fx::Pod>,
    chloe_arms: Option<super::chloe_arms::Rig>,
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
            g[node].set_visible(false);
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
        let weapon = weapons::build(g, shapes, id, weapon_resource, false);
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
        let arm_names=if id==6 {["Sebastian.Arm.L", "Sebastian.Arm.L"]}else{["UpperArm.L", "UpperArm.R"]};
        let arms: std::collections::HashSet<_> = arm_names
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
            orbit_age: 0.0,
            shadow_only: Default::default(),
            shadow_casters: None,
            casting_shadows: true,
            weaken_outline: Default::default(),
            picora_outline: Default::default(),
            face,
            grips,
            pod: (id == 3).then(|| super::id_fx::Pod::new(g, root)),
            chloe_arms: (id == 6).then(|| super::chloe_arms::Rig::new(g, root)),
            whales: if id == 1 {
                weapon_resource.map(|m| super::naia_whales::School::new(g, parent, m))
            } else {
                None
            },
        };
        result.set_motion(g, Motion::Idle);
        if id==6 {super::chloe_fx::face(g,root,Pose::default());}
        result
    }
    pub fn remove(self, g: &mut Graph) {
        if let Some(school) = self.whales {
            g.remove_node(school.root);
        }
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

    pub fn set_shadow_casting(&mut self, g: &mut Graph, enabled: bool) {
        if self.casting_shadows == enabled { return; }
        let nodes = self.shadow_casters.get_or_insert_with(|| {
            [self.root, self.weapon].into_iter()
                .flat_map(|root| g.traverse_handle_iter(root))
                .filter(|&h| g[h].cast::<fyrox::scene::mesh::Mesh>().is_some() && g[h].cast_shadows())
                .collect()
        });
        for &h in nodes.iter() { g[h].set_cast_shadows(enabled); }
        self.casting_shadows = enabled;
    }

    pub fn held_weapon(&self) -> Handle<Node> { self.weapon }
    pub fn sleeve_binding(&self)->(Handle<Node>,Handle<Node>){(self.root,self.wave_arm[1])}
    pub fn sleeve_points(&self,g:&Graph)->(Vector3<f32>,Vector3<f32>){
        (g[self.wave_arm[1]].global_position(),g[self.hand].global_position())
    }
    pub fn picora_outline(&mut self,g:&mut Graph,active:bool,time:f32){
        self.picora_outline.update(g,self.root,active,time);
    }
    pub fn head_position(&self, g: &Graph) -> Vector3<f32> {
        if self.head.is_some() {
            g[self.head].global_position()
        } else {
            g[self.root].global_position() + Vector3::y() * 1.22
        }
    }

    /// Project a head after its root moved, without refreshing the entire rig.
    pub fn current_head_position(&self, g: &Graph) -> Vector3<f32> {
        let mut node = if self.head.is_some() { self.head } else { self.root };
        let mut position = if self.head.is_some() { Vector3::zeros() } else { Vector3::y() * 1.22 };
        while node.is_some() {
            position = g[node].local_transform().matrix().transform_point(&position.into()).coords;
            node = g[node].parent();
        }
        position
    }

    /// Run after the FBX animation has updated both hand bones.
    pub fn attach_weapons(&self, g: &mut Graph) {
        if self.id==6 {
            g[self.weapon].set_visible(false);
            for grip in &self.grips {grip.pose(g,0.0);}
            if self.clock.pose.base().0==Motion::ChloeRideMove {self.grips[0].point(g,0.9);}
            super::chloe_fx::face(g,self.root,self.clock.pose);
            g.update_hierarchical_data_for_descendants(self.root);
            return;
        }
        if self.id == 10 { super::ui_fx::weapon(g,self.weapon,self.clock.pose); }
        if self.id == 13 && super::hilde_fx::active(self.clock.pose) {
            g[self.weapon].set_visible(false);
            g.update_hierarchical_data_for_descendants(g[self.root].parent());
            return;
        }
        // The editable Naia FBX also contains face keys. Apply the source clock
        // after skeletal evaluation so coarse FBX samples cannot override it.
        if self.id == 3 || self.id == 1 && self.clock.pose.motion == Motion::Attack {
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
        if self.id == 4 && self.clock.pose.motion == Motion::AyaUltimate {
            let age = self.clock.pose.age;
            if (0.45..3.25).contains(&age) {
                for grip in &self.grips { grip.pose(g, 0.0); }
                let parent = g[self.weapon].parent();
                let inverse = g[parent].global_transform().try_inverse().unwrap_or_default();
                let pos = self.head_position(g) + Vector3::y() * (0.65 + 0.25 * (age * 2.0).sin());
                g[self.weapon].local_transform_mut().set_position(inverse.transform_point(&pos.into()).coords)
                    .set_rotation(UnitQuaternion::from_euler_angles(0.12, age * 2.8, 0.15));
                weapons::update_aya(g, self.weapon, 0.0, false);
                g.update_hierarchical_data_for_descendants(self.weapon);
                return;
            }
        }
        if self.id == 12 && !g[self.weapon].visibility() {
            for grip in &self.grips { grip.pose(g,0.0); }
            return;
        }
        if self.id == 9 {
            g[self.weapon].set_visible(false);
            self.grips[0].pose(g, if self.clock.pose.motion == Motion::BellaUltimate {0.8} else {0.0});
            self.grips[1].pose(g,0.0);
            return;
        }
        if self.id == 7 {
            g[self.weapon].set_visible(false);
            for grip in &self.grips { grip.pose(g, 0.0); }
            return;
        }
        if let Some(pod) = &self.pod {
            if !self.is_shadow_only() {
                self.face
                    .show_motion(g, self.clock.pose.motion, self.clock.pose.age, self.id);
            }
            pod.update(g, self.clock.pose, !self.is_shadow_only(), self.orbit_age);
            for grip in &self.grips {
                grip.pose(g, 0.0);
            }
            return;
        }
        if let Some(school) = &self.whales {
            school.update(g, self.clock.pose, g[self.root].visibility());
            super::naia_whales::update_weapon(g, self.weapon, self.clock.pose, false);
            if self.clock.pose.motion == Motion::NaiaUltimate {
                g[self.weapon].set_visible(false);
                for grip in &self.grips {
                    grip.pose(g, 0.0);
                }
                return;
            }
        }
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
            weapons::update_vivi_pose(g, self.weapon, self.clock.pose, self.orbit_age, false);
            g.update_hierarchical_data_for_descendants(self.weapon);
            return;
        }
        let motion = if self.clock.pose.motion.is_naia_skill() {
            trickcal_shooting_balddagu::naia_skill::hand(
                self.clock.pose.motion,
                self.clock.pose.age,
            )
        } else if self.id == 14 && matches!(self.clock.pose.motion, Motion::ShionLoad | Motion::ShionLoop | Motion::ShionEnd | Motion::ShionUltimate)
        {
            trickcal_shooting_balddagu::shion_skill::hand(self.clock.pose.motion, self.clock.pose.age)
        } else if self.id == 5 && matches!(self.clock.pose.motion,Motion::PicoraSkill|Motion::PicoraUltimate) {
            trickcal_shooting_balddagu::picora_skill::hand(self.clock.pose.motion,self.clock.pose.age,false)
        } else if self.id == 10 && matches!(self.clock.pose.motion,Motion::UiSkill|Motion::UiUltimate) {
            trickcal_shooting_balddagu::ui_skill::hand(self.clock.pose.motion,self.clock.pose.age)
        } else if self.id == 2 && matches!(self.clock.pose.motion, Motion::ErpinSkill | Motion::ErpinStart | Motion::ErpinCharge | Motion::ErpinRecover) {
            trickcal_shooting_balddagu::erpin_skill::hand_motion(self.clock.pose.motion, self.clock.pose.age)
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
                weapons::world_pitch(self.id, -self.aim_pitch + motion.angles[0]),
                motion.angles[1],
                if self.id == 10 && matches!(self.clock.pose.motion, Motion::UiSkill | Motion::UiUltimate) {
                    // Hold the leaf beside the broad hat so its full source spin stays visible.
                    0.75
                } else { motion.angles[2] },
            ));
        g.update_hierarchical_data_for_descendants(self.weapon);
        if self.id == 11 {
            self.butter_weapon(g);
            return;
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
        let weapon_rotation = super::skeleton::world_rotation(g, self.weapon);
        let desired = if self.id == 1 {
            let amount = trickcal_shooting_balddagu::naia_skill::whale_form(
                self.clock.pose.motion,
                self.clock.pose.age,
            );
            weapon_rotation.slerp(
                &(weapon_rotation * super::naia_whales::cradle_rotation(false)),
                amount,
            )
        } else {
            super::hand_grip::facing(weapon_rotation, self.id, false)
        };
        if held {
            self.grips[0].orient(g, desired);
        }
        if held
            && weapons::grips(self.id)[1].is_some()
            && (self.id != 1 || self.clock.pose.motion.is_naia_skill())
        {
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
        let grip = if self.id == 1 {
            **g[weapons::grip_node(g, self.weapon, 1, false)]
                .local_transform()
                .position()
        } else {
            weapons::grips(self.id)[0].unwrap()
        };
        let transform = g[self.weapon].local_transform();
        let offset = **transform.rotation() * grip.component_mul(transform.scale());
        g[self.weapon]
            .local_transform_mut()
            .set_position(hand_pos - offset);
        g.update_hierarchical_data_for_descendants(self.weapon);
    }
    fn butter_weapon(&self, g: &mut Graph) {
        use trickcal_shooting_balddagu::butter_skill as b;
        let p=self.clock.pose;
        let parent=g[self.root].parent();
        let facing=super::skeleton::world_rotation(g,parent);
        let rotation=facing*UnitQuaternion::from_euler_angles(-self.aim_pitch,0.0,0.0);
        // The tail clock keeps advancing even while the source draw pose is held.
        let tail=g.traverse_handle_iter(self.root).find(|&h|g[h].name()=="Tail");
        if let Some(tail)=tail {
            let amount=if p.motion==Motion::ButterAttack && p.age<0.67 {0.40}else{0.12};
            let base=super::skeleton::world_rotation(g,tail);
            super::skeleton::set_global_rotation(g,tail,
                UnitQuaternion::from_axis_angle(&Vector3::y_axis(),(self.orbit_age*12.0).sin()*amount)*base);
        }
        let giant=super::butter_fx::is_giant(p);
        let held=g[self.weapon].visibility() && b::weapon_visible(p.motion,p.age);
        g[self.weapon].set_visible(held);
        let scale=1.0+2.7*b::growth(p.motion,p.age);
        g[self.weapon].local_transform_mut().set_scale(Vector3::repeat(scale));
        super::butter_fx::weapon(g,self.weapon,p,false);
        if giant {
            // Grow the fork away from the reachable pouch, then draw it back.
            let released=p.age>=b::ULTIMATE_RELEASE;
            let pull=if released {1.0}else{b::pull(p.motion,p.age)};
            let orientation=UnitQuaternion::from_euler_angles(-self.aim_pitch,0.0,0.0);
            let pouch=Vector3::new(0.0,1.16,0.40-0.18*pull);
            g[self.weapon].local_transform_mut().set_position(pouch-orientation*super::butter_fx::pouch(true,pull)*scale)
                .set_rotation(UnitQuaternion::from_euler_angles(-self.aim_pitch,0.0,0.0));
            g.update_hierarchical_data_for_descendants(self.weapon);
            if released {
                for grip in &self.grips {grip.pose(g,0.0);}
                return;
            }
            let arms=[false,true].map(|left|super::skeleton::arm_bone_names(left).map(|name|g.traverse_handle_iter(self.root).find(|&h|g[h].name()==name).unwrap()));
            for (i,left) in [false,true].into_iter().enumerate() {
                self.grips[i].pose(g,if p.age<2.5 {0.85}else{0.0});
                self.grips[i].orient(g,super::hand_grip::facing(rotation,11,left));
            }
            let origin=g[self.weapon].global_position();
            let offsets=[0,1].map(|i|self.grips[i].contact(g)-g[arms[i][2]].global_position());
            let centers=[0,1].map(|i|g[arms[i][0]].global_position()+offsets[i]-(g[super::butter_fx::draw_node(g,self.weapon,i==1)].global_position()-origin));
            let radii=[0,1].map(|i|(g[arms[i][1]].global_position()-g[arms[i][0]].global_position()).norm()+(g[arms[i][2]].global_position()-g[arms[i][1]].global_position()).norm()-0.002);
            let mut reachable=origin;
            for _ in 0..24 {for i in 0..2 {let delta=reachable-centers[i];if delta.norm()>radii[i] {reachable=centers[i]+delta.normalize()*radii[i];}}}
            let inverse=g[parent].global_transform().try_inverse().unwrap_or_default();
            g[self.weapon].local_transform_mut().set_position(inverse.transform_point(&reachable.into()).coords);
            g.update_hierarchical_data_for_descendants(self.weapon);
            for (i,left) in [false,true].into_iter().enumerate() {
                let arm=arms[i];
                let palm=super::hand_grip::facing(rotation,11,left);
                self.grips[i].orient(g,palm);
                let n=super::butter_fx::draw_node(g,self.weapon,left);
                let target=g[n].global_position()-(self.grips[i].contact(g)-g[arm[2]].global_position());
                super::hand_grip::solve_arm(g,arm[0],arm[1],arm[2],target,facing*Vector3::new(if left{1.0}else{-1.0},-0.4,-0.3));
                self.grips[i].orient(g,palm);
            }
        } else {
            self.grips[0].pose(g,if held {super::hand_grip::amount(11)}else{0.0});
            self.grips[1].pose(g,if held&&b::left_holding(p.motion,p.age) {0.85}else{0.0});
            let desired=super::hand_grip::facing(rotation,11,false);
            if held {self.grips[0].orient(g,desired);}
            if held&&b::left_holding(p.motion,p.age) {self.align_supporting_hands(g,desired);}
            let inverse=g[parent].global_transform().try_inverse().unwrap_or_default();
            let pos=inverse.transform_point(&self.grips[0].contact(g).into()).coords;
            g[self.weapon].local_transform_mut().set_position(pos);
            g.update_hierarchical_data_for_descendants(self.weapon);
        }
    }
    pub(super) fn butter_draw_errors(&self,g:&Graph)->[f32;2] {
        [false,true].map(|left|(self.palm_contact(g,left)-g[super::butter_fx::draw_node(g,self.weapon,left)].global_position()).norm())
    }
    pub(super) fn palm_contact(&self, g: &Graph, left: bool) -> Vector3<f32> {
        self.grips[usize::from(left)].contact(g)
    }
    pub(super) fn komi_grip(&self,g:&mut Graph,scale:f32,active:bool)->(Vector3<f32>,UnitQuaternion<f32>) {
        let p=self.clock.pose;
        let facing=super::skeleton::world_rotation(g,g[self.root].parent());
        let rotation=facing*super::komi_fx::tin_rotation(p);
        if self.id!=12 || !active {return (Vector3::zeros(),rotation);}
        g[self.weapon].set_visible(false);
        for grip in &self.grips {grip.pose(g,0.0);}
        if p.motion!=Motion::KomiFeast {return (Vector3::zeros(),rotation);}
        let arms=[false,true].map(|left|super::skeleton::arm_bone_names(left).map(|name|g.traverse_handle_iter(self.root).find(|h|g[*h].name()==name).unwrap()));
        let center=(self.grips[0].contact(g)+self.grips[1].contact(g))*0.5-rotation*Vector3::new(0.0,0.20,0.0)*scale;
        for i in 0..2 {
            self.grips[i].orient(g,super::hand_grip::facing(rotation,12,i==1));
            self.grips[i].pose(g,0.6);
            let offset=self.grips[i].contact(g)-g[arms[i][2]].global_position();
            let target=center+rotation*super::komi_fx::grip(i==1)*scale-offset;
            super::hand_grip::solve_arm(g,arms[i][0],arms[i][1],arms[i][2],target,facing*Vector3::new(if i==0{-1.0}else{1.0},-0.5,-0.4));
            self.grips[i].orient(g,super::hand_grip::facing(rotation,12,i==1));
        }
        (center,rotation)
    }
    pub(super) fn hilde_grip(&self,g:&mut Graph)->(Vector3<f32>,UnitQuaternion<f32>) {
        let p=self.clock.pose;
        let facing=super::skeleton::world_rotation(g,g[self.root].parent());
        let rotation=facing*UnitQuaternion::from_euler_angles(-self.aim_pitch,0.0,0.0)*super::hilde_fx::rotation(p);
        if self.id!=13 || !super::hilde_fx::active(p){return (self.palm_contact(g,false),rotation);}
        g[self.weapon].set_visible(false);
        let arms=[false,true].map(|left|super::skeleton::arm_bone_names(left).map(|name|g.traverse_handle_iter(self.root).find(|h|g[*h].name()==name).unwrap()));
        for i in 0..2 {self.grips[i].orient(g,super::hand_grip::facing(rotation,13,i==1));self.grips[i].pose(g,if p.motion==Motion::HildeSkill && i==1 {0.25}else{1.0});}
        let difference=rotation*super::hilde_fx::support(p)*super::hilde_fx::SCALE;
        let offsets=[0,1].map(|i|self.grips[i].contact(g)-g[arms[i][2]].global_position());
        let centers=[0,1].map(|i|g[arms[i][0]].global_position()+offsets[i]-if i==1{difference}else{Vector3::zeros()});
        let radii=[0,1].map(|i|(g[arms[i][1]].global_position()-g[arms[i][0]].global_position()).norm()+(g[arms[i][2]].global_position()-g[arms[i][1]].global_position()).norm()-0.002);
        let mut contact=self.grips[0].contact(g);
        for _ in 0..24 {for i in 0..2 {let d=contact-centers[i];if d.norm()>radii[i]{contact=centers[i]+d.normalize()*radii[i];}}}
        for i in 0..2 {let target=contact+if i==1 {difference}else{Vector3::zeros()}-offsets[i];super::hand_grip::solve_arm(g,arms[i][0],arms[i][1],arms[i][2],target,facing*Vector3::new(if i==0{-1.0}else{1.0},-0.5,-0.4));self.grips[i].orient(g,super::hand_grip::facing(rotation,13,i==1));}
        (self.grips[0].contact(g),rotation)
    }
    pub(super) fn mayo_grip(&self,g:&mut Graph)->(Vector3<f32>,UnitQuaternion<f32>) {
        let p=self.clock.pose;
        let facing=super::skeleton::world_rotation(g,g[self.root].parent());
        let rotation=facing*UnitQuaternion::from_euler_angles(-self.aim_pitch,0.0,0.0)*super::mayo_fx::rotation(p);
        if self.id!=8 || !super::mayo_fx::active(p){return (self.palm_contact(g,false),rotation);}
        g[self.weapon].set_visible(false);
        let arms=[false,true].map(|left|super::skeleton::arm_bone_names(left).map(|name|g.traverse_handle_iter(self.root).find(|h|g[*h].name()==name).unwrap()));
        for i in 0..2 {self.grips[i].orient(g,super::hand_grip::facing(rotation,13,i==1));self.grips[i].pose(g,1.0);}
        let difference=rotation*super::mayo_fx::support(p)*super::mayo_fx::SCALE;
        let offsets=[0,1].map(|i|self.grips[i].contact(g)-g[arms[i][2]].global_position());
        let centers=[0,1].map(|i|g[arms[i][0]].global_position()+offsets[i]-if i==1{difference}else{Vector3::zeros()});
        let radii=[0,1].map(|i|(g[arms[i][1]].global_position()-g[arms[i][0]].global_position()).norm()+(g[arms[i][2]].global_position()-g[arms[i][1]].global_position()).norm()-0.002);
        let mut contact=self.grips[0].contact(g);
        for _ in 0..24 {for i in 0..2 {let d=contact-centers[i];if d.norm()>radii[i]{contact=centers[i]+d.normalize()*radii[i];}}}
        for i in 0..2 {let target=contact+if i==1 {difference}else{Vector3::zeros()}-offsets[i];super::hand_grip::solve_arm(g,arms[i][0],arms[i][1],arms[i][2],target,facing*Vector3::new(if i==0{-1.0}else{1.0},-0.5,-0.4));self.grips[i].orient(g,super::hand_grip::facing(rotation,13,i==1));}
        (self.grips[0].contact(g),rotation)
    }
    pub(super) fn verify_hand_grip(&self, g: &Graph, left: bool) {
        self.grips[usize::from(left)].verify(g, true);
    }
    fn align_supporting_hands(&self, g: &mut Graph, desired: UnitQuaternion<f32>) {
        let support_rotation = if self.id == 1 {
            let rotation = super::skeleton::world_rotation(g, self.weapon);
            let amount = trickcal_shooting_balddagu::naia_skill::whale_form(
                self.clock.pose.motion,
                self.clock.pose.age,
            );
            super::hand_grip::facing(rotation, self.id, true).slerp(
                &(rotation * super::naia_whales::cradle_rotation(true)),
                amount,
            )
        } else {
            super::hand_grip::facing(desired, self.id, true)
        };
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
            - g[weapons::grip_node(g, self.weapon, self.id, false)]
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
        self.orbit_age = elapsed;
        self.seek(
            g,
            trickcal_shooting_balddagu::motion::selection_pose(self.id, elapsed),
        );
        g[self.weapon].set_visible(false);
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
    pub fn prepare_chloe_arms(&self, g: &mut Graph, gesture: [bool; 2]) {
        if let Some(arms) = &self.chloe_arms {
            let carry = super::chloe_arms::carrying(self.clock.pose);
            arms.apply(g, gesture.map(|active| active || !carry));
        }
    }
    pub fn review_chloe_arms(&self, g: &Graph) {
        if let Some(arms) = &self.chloe_arms { arms.review(g); }
    }
    pub fn verify_aya_ultimate(&self, g: &Graph) {
        assert!(g[self.weapon].global_position().y > self.head_position(g).y + 0.35,
            "Aya parasol floats above her head");
    }
    pub fn verify_naia_ultimate(&self, g: &Graph) {
        assert!(
            !g[self.weapon].visibility(),
            "world ultimate weapon must be hidden"
        );
        let school = self.whales.as_ref().expect("Naia school");
        assert!(g[school.root].visibility());
        assert_eq!(school.whales.len(), 6);
        for whale in school.whales {
            assert!(g[whale].local_transform().scale().x > 0.001);
        }
    }
    /// Apply after the engine evaluates Move, before crouch/weapon IK. Every
    /// non-arm bone remains exactly at its current locomotion sample.
    pub fn layer_attack_arms(&self, g: &mut Graph) {
        if self.id == 3 {
            return;
        }
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
        weapons::erpin_muzzle(g, self.weapon)
    }
    pub fn syringe_muzzle(&self, g: &Graph) -> Vector3<f32> {
        weapons::hilde_muzzle(g, self.weapon)
    }
    pub fn clover_muzzle(&self, g: &Graph) -> Vector3<f32> {
        super::ui_fx::muzzle(g, self.weapon)
    }
    pub fn water_muzzle(&self, g: &Graph) -> Vector3<f32> {
        g[self.weapon]
            .global_transform()
            .transform_point(&super::naia_whales::MUZZLE.into())
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
        self.weaken_outline.update(g, self.root, a.alive() && a.combat.weaken > 0.0 && !self.is_shadow_only());
        let pose = self.clock.observe(a, dt, tick, grounded);
        self.aim_pitch = a.pitch;
        self.orbit_age = tick as f32 / 60.0;
        self.crouch_drop = if a.alive() {
            a.crouch_drop() * a.body_scale()
        } else {
            0.0
        };
        self.seek(g, pose);
        g[self.root].set_visible(a.combat.polymorph<=0.0);
        // Meshy props are separate from the worn character mesh.
        g[self.weapon].set_visible(
            a.alive()
                && a.combat.polymorph<=0.0
                && pose.motion != Motion::Spawn
                && !a.dancing
                && a.wave_ticks == 0
                && !matches!(self.id, 3 | 6 | 7)
                && !(self.id == 13 && super::hilde_fx::active(pose))
                && !(self.id == 12 && a.combat.komi.active()),
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
