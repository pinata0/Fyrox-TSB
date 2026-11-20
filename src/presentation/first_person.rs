//! First-person grip and greeting made from the equipped character's actual FBX arms.
//! Source hands retain their skinning, UVs and materials; Naia E uses short view-only cuffs.
use crate::presentation::SceneState;
mod naia;
mod thickness;
use super::skeleton::{aim_joint, set_global_rotation};
use super::*;
use fyrox::core::algebra::Matrix3;
use fyrox::scene::transform::Transform;
use fyrox::scene::{
    animation::AnimationPlayer,
    mesh::{
        buffer::{TriangleBuffer, VertexAttributeUsage, VertexReadTrait},
        Mesh,
    },
};
use std::collections::{HashMap, HashSet};

// Keep the camera-mounted hands and cuffs readable at every gesture's palm target.
// Apply this before solving contact so held props and free-hand effects stay aligned.
pub(super) const HAND_SCALE: f32 = 2.2;
// Widen the arm cross-section separately so extra size does not just lengthen it.
const ARM_THICKNESS: f32 = 1.35;
const MODEL_SCALE: f32 = 0.01 * HAND_SCALE;
const MEDICAL_SLEEVE_RADIUS: f32 = 0.052 * HAND_SCALE * ARM_THICKNESS;

#[derive(Debug)]
pub(super) struct FirstPersonArm {
    pub character: u8,
    pub source: Handle<Node>,
    left: bool,
    model: Handle<Node>,
    upper_arm: Handle<Node>,
    forearm: Handle<Node>,
    hand: Handle<Node>,
    rest: Vec<(Handle<Node>, Transform)>,
    fingers: Vec<Handle<Node>>,
    grasp: super::hand_grip::Rig,
    refined: bool,
    naia_cuffs: Vec<naia::Cuff>,
    hilde_sleeve: Handle<Node>,
    hilde_mesh_visibility: Vec<(Handle<Node>, bool)>,
    pub triangles: usize,
}

impl FirstPersonArm {
    pub fn vivi(&self,g: &mut Graph,pose: trickcal_shooting_balddagu::motion::Pose,ticks:u8) {
        if self.character==0 && ticks==0 {
            g[self.model].set_visible(!self.left || pose.motion!=trickcal_shooting_balddagu::motion::Motion::ViviSkill);
        }
    }
    pub fn spawn(
        scene: &mut Scene,
        resource: &fyrox::resource::model::ModelResource,
        visual: &characters::Visual,
        holder: Handle<Node>,
        left: bool,
    ) -> Result<Self, String> {
        use fyrox::resource::model::ModelResourceExtension;
        let model = resource.instantiate(scene);
        let g = &mut scene.graph;
        g.link_nodes(model, holder);
        g[model]
            .local_transform_mut()
            .set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(model);
        let nodes: Vec<_> = g.traverse_handle_iter(model).collect();
        let find = |name: &str| nodes.iter().copied().find(|h| g[*h].name() == name);
        let [upper_name, forearm_name, hand_name] = super::skeleton::arm_bone_names(left);
        let bone = |name| {
            find(name)
                .ok_or_else(|| format!("{}: {name} missing", characters::SLUGS[visual.id as usize]))
        };
        let hand = bone(hand_name)?;
        let upper_arm = bone(upper_name)?;
        let forearm = bone(forearm_name)?;
        let arm_bones: HashSet<_> = g.traverse_handle_iter(upper_arm).collect();
        let grasp = super::hand_grip::Rig::new(g, model, visual.id, left);
        let fingers = grasp.fingers();
        let refined = nodes.iter().any(|h| g[*h].name().contains("_RefinedHand."));
        let rest = nodes
            .iter()
            .copied()
            .filter(|h| *h != model)
            .map(|h| (h, g[h].local_transform().clone()))
            .collect();

        let mut triangles = 0;
        let mut source_triangles = 0;
        let mut materials = 0;
        for node in nodes {
            // The viewmodel poses the original rig independently of full-body animation.
            if let Some(player) = g[node].cast_mut::<AnimationPlayer>() {
                for animation in player.animations_mut().iter_mut() {
                    animation.set_enabled(false);
                }
            }
            let Some(_) = g[node].cast::<Mesh>() else {
                continue;
            };
            // Even stripped/empty view meshes must stay outside shadow passes.
            g[node].set_cast_shadows(false);
            // Retopologized hands and closed cuffs are exported as complete meshes.
            // Do not expose the old body's cut arm boundary in the viewmodel.
            if refined
                && (!g[node].name().contains("_Refined")
                    || g[node].name().contains("_RefinedSleeve."))
            {
                g[node].cast_mut::<Mesh>().unwrap().set_surfaces(Vec::new());
                continue;
            }
            if refined {
                g[node].set_visible(true);
            }
            let mesh = g[node].cast::<Mesh>().unwrap();
            let mut surfaces = Vec::new();
            for surface in mesh.surfaces() {
                // Aya's source parasol is weighted to her actual right hand too.
                // Its six materials belong to the prop, not the extracted arm;
                // the equipped first-person parasol is built separately.
                if visual.id == 4 {
                    let material = surface.material().data_ref();
                    let color = material
                        .property_group_ref("properties")
                        .and_then(|p| p.property_ref("diffuseColor"))
                        .and_then(|p| p.as_color());
                    if color.is_some_and(|color| {
                        [
                            [0xf2, 0xee, 0xe6],
                            [0xc7, 0xcf, 0xdd],
                            [0xf5, 0xf6, 0xfc],
                            [0xaa, 0xb8, 0xd1],
                            [0xdd, 0xdd, 0xe6],
                            [0xe4, 0xde, 0xcd],
                        ]
                        .into_iter()
                        .any(|rgb_color| color == rgb(rgb_color))
                    }) {
                        continue;
                    }
                }
                let data = surface.data_ref().data_ref();
                let keep_vertex: Vec<_> = data
                    .vertex_buffer
                    .iter()
                    .map(|vertex| {
                        let (Ok(indices), Ok(weights)) = (
                            vertex.read_4_u8(VertexAttributeUsage::BoneIndices),
                            vertex.read_4_f32(VertexAttributeUsage::BoneWeight),
                        ) else {
                            return false;
                        };
                        let arm_weight: f32 = indices
                            .iter()
                            .zip(weights.iter())
                            .filter(|(index, _)| {
                                surface
                                    .bones()
                                    .get(**index as usize)
                                    .is_some_and(|bone| arm_bones.contains(bone))
                            })
                            .map(|(_, weight)| *weight)
                            .sum();
                        arm_weight >= 0.5
                    })
                    .collect();
                let mut selected: Vec<_> = data
                    .geometry_buffer
                    .iter()
                    .copied()
                    .filter(|triangle| {
                        triangle
                            .0
                            .iter()
                            .all(|index| keep_vertex.get(*index as usize) == Some(&true))
                    })
                    .collect();
                // Generated skins sometimes assign nearby detached hair to an arm.
                // Keep the connected surface that contains the articulated fingers;
                // weld UV-seam duplicates by position for this connectivity check.
                if !selected.is_empty() {
                    let mut parents: Vec<usize> = (0..selected.len()).collect();
                    fn root(parents: &mut [usize], mut i: usize) -> usize {
                        while parents[i] != i {
                            parents[i] = parents[parents[i]];
                            i = parents[i];
                        }
                        i
                    }
                    let mut owners = HashMap::new();
                    let mut scores = vec![0.0_f32; selected.len()];
                    for (i, triangle) in selected.iter().enumerate() {
                        for index in triangle.0 {
                            let vertex = data.vertex_buffer.get(index as usize).unwrap();
                            let p = vertex.read_3_f32(VertexAttributeUsage::Position).unwrap();
                            let key = [p.x, p.y, p.z].map(|v| (v * 10000.0).round() as i32);
                            if let Some(previous) = owners.insert(key, i) {
                                let a = root(&mut parents, i);
                                let b = root(&mut parents, previous);
                                parents[a] = b;
                            }
                            let indices =
                                vertex.read_4_u8(VertexAttributeUsage::BoneIndices).unwrap();
                            let weights =
                                vertex.read_4_f32(VertexAttributeUsage::BoneWeight).unwrap();
                            scores[i] += indices
                                .iter()
                                .zip(weights.iter())
                                .filter(|(bone, _)| {
                                    surface
                                        .bones()
                                        .get(**bone as usize)
                                        .is_some_and(|h| fingers.contains(h))
                                })
                                .map(|(_, weight)| *weight)
                                .sum::<f32>();
                        }
                    }
                    let mut totals = HashMap::<usize, (f32, usize)>::new();
                    for (i, score) in scores.iter().enumerate() {
                        let entry = totals.entry(root(&mut parents, i)).or_default();
                        entry.0 += score;
                        entry.1 += 1;
                    }
                    if let Some((&component, _)) = totals
                        .iter()
                        .filter(|(_, (score, _))| *score > 0.1)
                        .max_by(|a, b| a.1 .0.total_cmp(&b.1 .0))
                    {
                        selected = selected
                            .into_iter()
                            .enumerate()
                            .filter_map(|(i, triangle)| {
                                (root(&mut parents, i) == component).then_some(triangle)
                            })
                            .collect();
                    }
                }
                source_triangles += data.geometry_buffer.iter().count();
                if selected.is_empty() {
                    continue;
                }
                triangles += selected.len();
                materials += 1;
                // New buffers avoid editing the full character model's shared surface resource.
                let mut arm_data =
                    SurfaceData::new(data.vertex_buffer.clone(), TriangleBuffer::new(selected));
                arm_data.blend_shapes_container = data.blend_shapes_container.clone();
                thickness::widen(
                    &mut arm_data,
                    surface.bones(),
                    g,
                    [upper_arm, forearm, hand],
                    ARM_THICKNESS,
                );
                surfaces.push(
                    SurfaceBuilder::new(SurfaceResource::new_embedded(arm_data))
                        .with_material(surface.material().clone())
                        .with_bones(surface.bones().to_vec())
                        .build(),
                );
            }
            g[node].cast_mut::<Mesh>().unwrap().set_surfaces(surfaces);
            // Camera-mounted skins can have their arm roots outside the frustum
            // while the hands and cuffs are still visible.
            g[node].set_frustum_culling(false);
        }
        if triangles == 0 || triangles >= source_triangles {
            g.remove_node(model);
            return Err(format!(
                "{}: could not isolate the original arm mesh",
                characters::SLUGS[visual.id as usize]
            ));
        }
        println!("FIRST_PERSON_ARM_OK: {}, {triangles}/{source_triangles} original triangles, {materials} original materials", characters::SLUGS[visual.id as usize]);
        let naia_cuffs = if visual.id == 1 {
            naia::cuffs(g, model, forearm, hand, &fingers)
        } else {
            Vec::new()
        };
        let hilde_mesh_visibility = if visual.id==13 {
            g.traverse_handle_iter(model).filter(|n|g[*n].cast::<Mesh>().is_some() && !g[*n].name().contains("_RefinedHand.")).map(|n|(n,g[n].visibility())).collect()
        } else {Vec::new()};
        let hilde_sleeve = if visual.id==13 {
            MeshBuilder::new(BaseBuilder::new().with_name("Hilde first-person medical sleeve").with_cast_shadows(false).with_visibility(false))
                .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(SurfaceData::make_cylinder(20,1.0,1.0,true,&Matrix4::identity()))).with_material(material([232,224,207])).build()]).build(g).to_base()
        } else {Handle::NONE};
        Ok(Self {
            character: visual.id,
            source: visual.root,
            left,
            model,
            upper_arm,
            forearm,
            hand,
            rest,
            fingers,
            grasp,
            refined,
            naia_cuffs,
            hilde_sleeve,
            hilde_mesh_visibility,
            triangles,
        })
    }

    pub fn sleeve_binding(&self)->(Handle<Node>,Handle<Node>){(self.model,self.forearm)}
    pub fn sleeve_points(&self,g:&Graph)->(Vector3<f32>,Vector3<f32>){
        (g[self.forearm].global_position(),g[self.hand].global_position())
    }
    pub fn remove(self, g: &mut Graph) {
        if self.hilde_sleeve.is_some() {g.remove_node(self.hilde_sleeve);}
        g.remove_node(self.model);
    }

    pub fn verify_anatomical_hand(&self, g: &Graph, camera: Handle<Node>, held: bool) {
        assert_eq!(
            g[self.hand].name(),
            super::skeleton::arm_bone_names(self.left)[2]
        );
        assert!(
            g[self.hand].global_transform().determinant() > 0.0,
            "arm must not be mirrored"
        );
        self.grasp.verify(g, held);
        if self.character == 8 && held && !self.left {
            let palm = super::skeleton::world_rotation(g, self.grasp.socket) * Vector3::z();
            let up = super::skeleton::world_rotation(g, camera) * Vector3::y();
            let toward_player = super::skeleton::world_rotation(g, camera) * -Vector3::z();
            let joint = |name: &str| self.fingers.iter().copied().find(|h| g[*h].name().starts_with(name)).unwrap();
            let thumb_side = (g[joint("Thumb1.")].global_position()
                - g[joint("Pinky1.")].global_position()).normalize();
            let fingers = (g[joint("Middle1.")].global_position()
                - g[self.hand].global_position()).normalize();
            let forearm = (g[self.hand].global_position()
                - g[self.forearm].global_position()).normalize();
            assert!((-palm).dot(&up) > 0.65, "Mayo's hand back must face upward");
            assert!(thumb_side.dot(&toward_player) > 0.15, "Mayo's thumb must face the player");
            assert!(fingers.dot(&forearm) > 0.85, "Mayo's wrist must follow the forearm");
            println!("MAYO_PALM_OK: back_up={:.3}, thumb_to_player={:.3}, wrist_alignment={:.3}",
                (-palm).dot(&up), thumb_side.dot(&toward_player), fingers.dot(&forearm));
        }
    }

    fn grip_facing(
        &self,
        g: &Graph,
        holder: Handle<Node>,
        grip_node: Handle<Node>,
    ) -> UnitQuaternion<f32> {
        let original = super::hand_grip::facing(
            super::skeleton::world_rotation(g, grip_node),
            self.character,
            self.left,
        );
        if self.character == 8 {
            // The thumb is on socket +Y. Point that side back along the tube
            // toward the player, keeping the hand back up and the wrist on
            // the sleeve's side of the grip instead of rolling the hand sideways.
            let shaft = original * -Vector3::y();
            let camera_up = super::skeleton::world_rotation(g, g[holder].parent()) * Vector3::y();
            let Some(up) = (camera_up - shaft * camera_up.dot(&shaft)).try_normalize(0.05) else {
                return original;
            };
            let palm = -up;
            return UnitQuaternion::from_matrix(&Matrix3::from_columns(&[
                shaft.cross(&palm), shaft, palm,
            ]));
        }
        if !self.refined {
            return original;
        }
        // A cylindrical grip permits rotation about its shaft. Approach it from
        // the matching lower screen corner, keeping the cuff behind the palm.
        let shaft = original * Vector3::y();
        let approach = super::skeleton::world_rotation(g, holder)
            * Vector3::new(if self.left { -0.75 } else { 0.75 }, 0.30, 0.65);
        let Some(forward) = (approach - shaft * approach.dot(&shaft)).try_normalize(0.05) else {
            return original;
        };
        let x = forward * if self.left { 1.0 } else { -1.0 };
        let z = x.cross(&shaft).normalize();
        let held = UnitQuaternion::from_rotation_matrix(
            &fyrox::core::algebra::Rotation3::from_matrix_unchecked(Matrix3::from_columns(&[
                x, shaft, z,
            ])),
        );
        if self.character == 1 {
            // The socket depth follows E's blend and stays zero for Q's empty hands.
            let amount = (g[grip_node].local_transform().position().z
                / super::naia_whales::CRADLE_DEPTH)
                .clamp(0.0, 1.0);
            let weapon = g[grip_node].parent();
            let cradle = super::skeleton::world_rotation(g, weapon)
                * super::naia_whales::cradle_rotation(self.left);
            held.slerp(&cradle, amount)
        } else {
            held
        }
    }

    pub fn grip_error(&self, g: &Graph, holder: Handle<Node>, weapon: Handle<Node>) -> f32 {
        let grip = weapons::grips(self.character)[usize::from(self.left)].unwrap();
        let grip_node = weapons::grip_node(g, weapon, self.character, self.left);
        let _ = holder;
        let contact = self.grasp.contact(g);
        let angle = super::skeleton::world_rotation(g, self.grasp.socket)
            .angle_to(&self.grip_facing(g, holder, grip_node));
        assert!(
            angle < 0.005,
            "{} palm axes differ: {angle} rad, left={}",
            characters::SLUGS[self.character as usize],
            self.left
        );
        let target = g[grip_node]
            .global_transform()
            .transform_point(&grip.into())
            .coords;
        (contact - target).norm()
    }

    /// Raise the equipped hand into view, then return smoothly to the same grip.
    pub fn update(
        &self,
        g: &mut Graph,
        holder: Handle<Node>,
        camera: Handle<Node>,
        weapon: Handle<Node>,
        ticks: u8,
    ) {
        if self.hilde_sleeve.is_some() {g[self.hilde_sleeve].set_visible(false);}
        for &(node, visible) in &self.hilde_mesh_visibility {g[node].set_visible(visible);}
        g[self.model].set_visible(!self.left || ticks == 0);
        if !g[holder].visibility() {
            return;
        }
        if self.left && ticks > 0 {
            return;
        }
        self.hold(g, holder, weapon);
        // Empty-hand heroes must wave from their actual resting gesture, not
        // from the hidden weapon socket, so both transition seams agree.
        if self.character == 3 {
            self.id(g, holder, camera, 100.0);
        } else if self.character == 7 {
            self.renua(g, holder, camera, trickcal_shooting_balddagu::motion::Pose {
                motion: trickcal_shooting_balddagu::motion::Motion::Idle,
                age: 0.0,
                shot_age: 100.0,
                locomotion: None,
            });
        }
        let (lift, sway) = wave_pose(ticks);
        if lift == 0.0 {
            return;
        }

        // Preserve the equipped scale and blend the original forearm rotation.
        let held_forearm = super::skeleton::world_rotation(g, self.forearm);
        let held_hand: Vec<_> = std::iter::once(self.hand)
            .chain(self.fingers.iter().copied())
            .map(|node| (node, **g[node].local_transform().rotation()))
            .collect();
        for (node, pose) in &self.rest {
            if *node == self.forearm || *node == self.hand || self.fingers.contains(node) {
                g[*node].set_local_transform(pose.clone());
            }
        }
        g.update_hierarchical_data_for_descendants(self.forearm);
        // Derive the open palm from the original rig, including imported bone rolls.
        // Rotate the sleeve and hand together instead of twisting only the wrist.
        let facing = super::skeleton::world_rotation(g, camera);
        let find_finger = |suffix: &str| {
            self.fingers
                .iter()
                .copied()
                .find(|h| g[*h].name().starts_with(suffix))
        };
        let (forward, across) = if let (Some(middle), Some(index), Some(pinky)) = (
            find_finger("Middle1."),
            find_finger("Index1."),
            find_finger("Pinky1."),
        ) {
            (
                g[middle].global_position() - g[self.hand].global_position(),
                g[index].global_position() - g[pinky].global_position(),
            )
        } else {
            // The mitten's thumb is on the hand bone's +X side. Follow that
            // source axis after posing the upper arm, not a fixed camera axis.
            (
                g[self.hand].global_position() - g[self.forearm].global_position(),
                super::skeleton::world_rotation(g, self.hand) * Vector3::x(),
            )
        };
        let delta = palm_basis(
            facing * Vector3::new(sway, 1.0, 0.10),
            facing * Vector3::x(),
        ) * palm_basis(forward, across).inverse();
        let raised_forearm = delta * super::skeleton::world_rotation(g, self.forearm);
        for (node, held) in held_hand {
            let open = **g[node].local_transform().rotation();
            g[node]
                .local_transform_mut()
                .set_rotation(held.slerp(&open, lift));
        }
        set_global_rotation(g, self.forearm, held_forearm.slerp(&raised_forearm, lift));
        // Enlarging the arm makes an elbow-only wave leave the right edge of the
        // camera. Blend the whole arm toward a visible palm target; the final
        // sleeve pass still routes its cuff through the lower right corner.
        let target = g[camera]
            .global_transform()
            .transform_point(&Vector3::new(-0.30 + sway * 0.06, 0.02, 0.62).into())
            .coords;
        let offset = g[holder]
            .global_transform()
            .try_inverse()
            .unwrap_or_default()
            .transform_vector(&((target - self.grasp.contact(g)) * lift));
        let position = **g[self.model].local_transform().position();
        g[self.model].local_transform_mut().set_position(position + offset);
        g.update_hierarchical_data_for_descendants(self.model);
    }

    /// Check every simulation tick on the actual imported rig, including both seams.
    pub fn verify_greeting_continuity(
        &self,
        g: &mut Graph,
        holder: Handle<Node>,
        camera: Handle<Node>,
        weapon: Handle<Node>,
    ) {
        use trickcal_shooting_balddagu::communication::WAVE_TICKS;
        self.update(g, holder, camera, weapon, 0);
        let shoulder = g[self.upper_arm].global_position();
        let elbow = g[self.forearm].global_position();
        let grip_wrist = g[self.hand].global_position();
        let grip_rotation = super::skeleton::world_rotation(g, self.hand);
        let length = (grip_wrist - elbow).norm();
        let mut previous_wrist = grip_wrist;
        let mut previous_rotation = grip_rotation;
        let mut max_step = 0.0_f32;
        let mut max_angle = 0.0_f32;
        let mut raised = false;
        for ticks in (0..=WAVE_TICKS).rev() {
            self.update(g, holder, camera, weapon, ticks);
            let wrist = g[self.hand].global_position();
            let rotation = super::skeleton::world_rotation(g, self.hand);
            let step = (wrist - previous_wrist).norm();
            let angle = rotation.angle_to(&previous_rotation);
            max_step = max_step.max(step);
            max_angle = max_angle.max(angle);
            let slug = characters::SLUGS[self.character as usize];
            let current_elbow = g[self.forearm].global_position();
            let translation = current_elbow - elbow;
            assert!((g[self.upper_arm].global_position() - shoulder - translation).norm() < 0.0001);
            assert!(((wrist - current_elbow).norm() - length).abs() < 0.0001);
            assert!(
                step < length * 0.30,
                "{slug}: wrist jumped {step} m (forearm {length} m) at tick {ticks}"
            );
            assert!(
                angle < 0.4,
                "{slug}: hand snapped {angle} rad at tick {ticks}"
            );
            if ticks == WAVE_TICKS || ticks == 0 {
                assert!((wrist - grip_wrist).norm() < 0.0001);
                assert!(rotation.angle_to(&grip_rotation) < 0.001);
                if !matches!(self.character, 3 | 7) {
                    assert!(self.grip_error(g, holder, weapon) < 0.001);
                }
            }
            if wave_pose(ticks).0 == 1.0 {
                let up = super::skeleton::world_rotation(g, camera) * Vector3::y();
                raised |= (wrist - grip_wrist).dot(&up) > 0.01;
                self.verify_greeting_in_view(g, camera);
            }
            previous_wrist = wrist;
            previous_rotation = rotation;
        }
        assert!(raised, "greeting must actually raise the equipped hand");
        println!(
            "GREETING_CONTINUITY_OK: {}, 109 ticks, fixed scale/arm length, visible palm/fingers, max wrist step {max_step:.5} m, max hand step {max_angle:.5} rad",
            characters::SLUGS[self.character as usize]
        );
    }

    pub fn naia(
        &self,
        g: &mut Graph,
        holder: Handle<Node>,
        camera: Handle<Node>,
        pose: trickcal_shooting_balddagu::motion::Pose,
    ) {
        use trickcal_shooting_balddagu::{motion::Motion, naia_skill};
        if self.character != 1 {
            return;
        }
        let low_grade = pose.motion.is_naia_skill() && pose.motion != Motion::NaiaUltimate;
        for cuff in &self.naia_cuffs {
            cuff.show(g, low_grade);
        }
        if !pose.motion.is_naia_skill() {
            if self.left {
                g[self.model].set_visible(false);
            }
            return;
        }
        if pose.motion != Motion::NaiaUltimate {
            let amount = naia_skill::whale_form(pose.motion, pose.age);
            // Match the enlarged dolphin while keeping the palm on its side socket.
            let contact = self.grasp.contact(g);
            g[self.model]
                .local_transform_mut()
                .set_scale(Vector3::repeat(
                    MODEL_SCALE * (1.0 + 0.9 * amount) * super::naia_whales::view_scale(pose),
                ));
            self.grasp.pose(g, 1.0 - 0.45 * amount);
            g.update_hierarchical_data_for_descendants(self.model);
            let offset = g[holder]
                .global_transform()
                .try_inverse()
                .unwrap_or_default()
                .transform_vector(&(contact - self.grasp.contact(g)));
            let position = **g[self.model].local_transform().position();
            g[self.model]
                .local_transform_mut()
                .set_position(position + offset);
            g.update_hierarchical_data_for_descendants(self.model);
            return;
        }
        // Start from the verified equipped-arm pose. Translating that complete
        // arm keeps cuffs below the hands and outside the near plane.
        self.grasp.pose(g, 0.0);
        let sign = if self.left { 1.0 } else { -1.0 };
        let path = naia_skill::hand_path(pose.motion, pose.age, self.left);
        let position = Vector3::new(
            sign * (0.34 + 0.06 * (pose.age * 2.0).sin()),
            -0.24 + (path[1] * 0.18).clamp(-0.12, 0.18),
            0.70 + (path[0] * 0.12).clamp(-0.08, 0.12),
        );
        let target = g[camera]
            .global_transform()
            .transform_point(&position.into())
            .coords;
        let offset = g[holder]
            .global_transform()
            .try_inverse()
            .unwrap_or_default()
            .transform_vector(&(target - self.grasp.contact(g)));
        let current = **g[self.model].local_transform().position();
        g[self.model]
            .local_transform_mut()
            .set_position(current + offset);
        g.update_hierarchical_data_for_descendants(self.model);
    }

    pub(super) fn palm(&self, g: &Graph) -> Vector3<f32> { self.grasp.contact(g) }

    /// Route each sleeve out through its own lower camera corner after the final
    /// gesture. Keep the posed palm fixed so weapons and spell effects still meet it.
    pub fn frame_sleeve(&self, g: &mut Graph, holder: Handle<Node>, camera: Handle<Node>) {
        if !g[holder].visibility() || !g[self.model].visibility() {
            return;
        }
        let contact = self.grasp.contact(g);
        let hand_rotation = super::skeleton::world_rotation(g, self.hand);
        // Camera-local -X is screen right. Both origins lie below/outside the
        // frustum, so neither an elbow nor the sleeve's closed end is exposed.
        let camera_inverse = g[camera].global_transform().try_inverse().unwrap_or_default();
        let wrist = camera_inverse.transform_point(&g[self.hand].global_position().into());
        let corner = g[camera]
            .global_transform()
            .transform_point(&Vector3::new(
                if self.left { 0.60_f32.max(wrist.x + 0.25) } else { (-0.60_f32).min(wrist.x - 0.25) },
                (-0.58_f32).min(wrist.y - 0.25),
                0.18_f32.min(wrist.z - 0.15),
            ).into())
            .coords;
        let mut direction = g[self.hand].global_position() - corner;
        if self.character == 8 && !self.left {
            let camera_rotation = super::skeleton::world_rotation(g, camera);
            let up = camera_rotation * Vector3::y();
            let dorsal = super::skeleton::world_rotation(g, self.grasp.socket) * -Vector3::z();
            let middle = self.fingers.iter().copied()
                .find(|h| g[*h].name().starts_with("Middle1.")).unwrap();
            let forward = (g[middle].global_position() - g[self.hand].global_position()).normalize();
            // The overhand tube grip needs a nearly straight wrist. The generic
            // near-camera corner pulls this forearm across the back of the hand.
            // Gate on the grip axes so greetings and skill guns keep their framing.
            if dorsal.dot(&up) > 0.65 && forward.dot(&(camera_rotation * Vector3::x())) > 0.35 {
                direction = forward + up * 0.25;
            }
        }
        aim_joint(g, self.upper_arm, self.forearm, direction);
        aim_joint(g, self.forearm, self.hand, direction);
        set_global_rotation(g, self.hand, hand_rotation);

        let offset = g[holder]
            .global_transform()
            .try_inverse()
            .unwrap_or_default()
            .transform_vector(&(contact - self.grasp.contact(g)));
        let position = **g[self.model].local_transform().position();
        g[self.model].local_transform_mut().set_position(position + offset);
        g.update_hierarchical_data_for_descendants(self.model);
        debug_assert!((contact - self.grasp.contact(g)).norm() < 0.001);
        debug_assert!(hand_rotation.angle_to(&super::skeleton::world_rotation(g, self.hand)) < 0.001);

        if self.hilde_sleeve.is_some() && g[self.hilde_sleeve].visibility() {
            let delta = g[self.hand].global_position() - corner;
            g[self.hilde_sleeve]
                .local_transform_mut()
                .set_position(corner)
                .set_rotation(UnitQuaternion::rotation_between(&Vector3::y(), &delta.normalize()).unwrap_or_default())
                .set_scale(Vector3::new(MEDICAL_SLEEVE_RADIUS, delta.norm(), MEDICAL_SLEEVE_RADIUS));
            g.update_hierarchical_data_for_descendants(self.hilde_sleeve);
        }
    }

    pub fn visible(&self, g: &Graph) -> bool {
        g[self.model].visibility()
    }

    pub fn verify_greeting_in_view(&self, g: &Graph, camera: Handle<Node>) {
        let projection = g[camera].cast::<fyrox::scene::camera::Camera>().unwrap().view_projection_matrix();
        for joint in std::iter::once(self.hand).chain(self.fingers.iter().copied()) {
            let p = projection * g[joint].global_position().push(1.0);
            assert!(p.w > 0.02 && p.x.abs() < p.w * 0.9 && p.y.abs() < p.w * 0.9,
                "{} greeting joint {} left the camera: {p:?}", characters::SLUGS[self.character as usize], g[joint].name());
        }
    }

    pub fn verify_sleeve_corner(&self, g: &Graph, camera: Handle<Node>) {
        let inverse = g[camera].global_transform().try_inverse().unwrap();
        let wrist = inverse.transform_point(&g[self.hand].global_position().into());
        let elbow = inverse.transform_point(&g[self.forearm].global_position().into());
        assert!(wrist.coords.iter().all(|v| v.is_finite()));
        assert!(elbow.y < wrist.y, "sleeve must enter from below the hand");
        assert!(
            if self.left { elbow.x > wrist.x } else { elbow.x < wrist.x },
            "{} sleeve must enter from its own lower corner (left={})",
            characters::SLUGS[self.character as usize], self.left
        );
    }

    pub fn hilde(&self,g:&mut Graph,holder:Handle<Node>,camera:Handle<Node>,pose:trickcal_shooting_balddagu::motion::Pose) {
        if self.character!=13 || !g[holder].visibility() || !super::hilde_fx::active(pose){return;}
        for (node,rest) in &self.rest {g[*node].set_local_transform(rest.clone());}
        g[self.model].set_visible(true);g[self.model].local_transform_mut().set_position(Vector3::zeros()).set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let facing=super::skeleton::world_rotation(g,camera);let rotation=facing*super::hilde_fx::view_rotation(pose);
        let sign=if self.left{1.0}else{-1.0};
        aim_joint(g,self.upper_arm,self.forearm,facing*Vector3::new(sign*0.20,-0.35,0.75));
        aim_joint(g,self.forearm,self.hand,facing*Vector3::new(sign*0.12,0.18,0.90));
        // Keep the sleeve and wrist in the same frame, as in the normal gun grip.
        // The case faces the camera independently of the palm around its handle.
        let palm=super::hand_grip::facing(facing*super::hilde_fx::rotation(pose),13,self.left);
        let turn=palm*super::skeleton::world_rotation(g,self.grasp.socket).inverse();
        set_global_rotation(g,self.forearm,turn*super::skeleton::world_rotation(g,self.forearm));
        self.grasp.orient(g,palm);
        self.grasp.pose(g,if self.left&&pose.motion==trickcal_shooting_balddagu::motion::Motion::HildeSkill{0.25}else{1.0});
        let origin=g[camera].global_transform().transform_point(&super::hilde_fx::view_origin(pose).into()).coords;
        let target=origin+if self.left{rotation*super::hilde_fx::view_support(pose)*super::hilde_fx::VIEW_SCALE}else{Vector3::zeros()};
        let offset=g[holder].global_transform().try_inverse().unwrap_or_default().transform_vector(&(target-self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);g.update_hierarchical_data_for_descendants(self.model);
        for &(node,_) in &self.hilde_mesh_visibility {g[node].set_visible(false);}
        // View sleeves run from below the camera to the original skinned wrist.
        // The full character's upper-arm mesh would expose its shoulder cut here.
        let elbow=g[camera].global_transform().transform_point(&Vector3::new(if self.left{0.42}else{-0.50},-0.56,0.32).into()).coords;
        let wrist=g[self.hand].global_position();let delta=wrist-elbow;
        g[self.hilde_sleeve].set_visible(true);
        // Fyrox cylinders extend from local Y=0 to Y=1 (their pivot is the base).
        g[self.hilde_sleeve].local_transform_mut().set_position(elbow)
            .set_rotation(UnitQuaternion::rotation_between(&Vector3::y(),&delta.normalize()).unwrap_or_default())
            .set_scale(Vector3::new(MEDICAL_SLEEVE_RADIUS,delta.norm(),MEDICAL_SLEEVE_RADIUS));
        g.update_hierarchical_data_for_descendants(self.model);
        g.update_hierarchical_data_for_descendants(self.hilde_sleeve);
    }

    pub fn mayo(&self,g:&mut Graph,holder:Handle<Node>,camera:Handle<Node>,pose:trickcal_shooting_balddagu::motion::Pose) {
        if self.character!=8 || !g[holder].visibility() || !super::mayo_fx::active(pose){return;}
        for (node,rest) in &self.rest {g[*node].set_local_transform(rest.clone());}
        g[self.model].set_visible(true);g[self.model].local_transform_mut().set_position(Vector3::zeros()).set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let facing=super::skeleton::world_rotation(g,camera);let rotation=facing*super::mayo_fx::view_rotation(pose);
        let sign=if self.left{1.0}else{-1.0};
        aim_joint(g,self.upper_arm,self.forearm,facing*Vector3::new(sign*0.20,-0.35,0.75));
        aim_joint(g,self.forearm,self.hand,facing*Vector3::new(sign*0.12,0.18,0.90));
        // Keep the sleeve and wrist in the same frame, as in the normal gun grip.
        // The case faces the camera independently of the palm around its handle.
        let palm=super::hand_grip::facing(facing*super::mayo_fx::rotation(pose),13,self.left);
        let turn=palm*super::skeleton::world_rotation(g,self.grasp.socket).inverse();
        set_global_rotation(g,self.forearm,turn*super::skeleton::world_rotation(g,self.forearm));
        self.grasp.orient(g,palm);
        self.grasp.pose(g,1.0);
        let origin=g[camera].global_transform().transform_point(&super::mayo_fx::view_origin(pose).into()).coords;
        let target=origin+if self.left{rotation*super::mayo_fx::support(pose)*super::mayo_fx::VIEW_SCALE}else{Vector3::zeros()};
        let offset=g[holder].global_transform().try_inverse().unwrap_or_default().transform_vector(&(target-self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);g.update_hierarchical_data_for_descendants(self.model);
        self.frame_sleeve(g,holder,camera);
    }

    pub fn aya(&self, g: &mut Graph, holder: Handle<Node>, camera: Handle<Node>, pose: trickcal_shooting_balddagu::motion::Pose) {
        use trickcal_shooting_balddagu::{motion::Motion,aya_skill};
        if self.character != 4 || !g[holder].visibility() || !matches!(pose.motion, Motion::AyaSkill|Motion::AyaUltimate) { return; }
        // The equipped hand keeps its parasol contact during E. Q releases it.
        if pose.motion == Motion::AyaSkill && !self.left { return; }
        for (node, rest) in &self.rest { g[*node].set_local_transform(rest.clone()); }
        g[self.model].set_visible(true);
        g[self.model].local_transform_mut().set_position(Vector3::zeros()).set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let path=aya_skill::hand(pose.motion,pose.age,self.left);
        let sign=if self.left {1.0}else{-1.0};let facing=super::skeleton::world_rotation(g,camera);
        aim_joint(g,self.upper_arm,self.forearm,facing*Vector3::new(sign*0.20,-0.4,0.8));
        aim_joint(g,self.forearm,self.hand,facing*Vector3::new(sign*0.15,0.3,0.8));
        let spread=if pose.motion==Motion::AyaUltimate {aya_skill::bloom(pose.age+0.1)}else{0.0};
        self.grasp.pose(g,0.0);
        let position=Vector3::new(sign*(0.29+spread*0.21),-0.28+(path[1]*0.18).clamp(-0.10,0.20),0.65+(path[0]*0.10).clamp(-0.08,0.12));
        let target=g[camera].global_transform().transform_point(&position.into()).coords;
        let offset=g[holder].global_transform().try_inverse().unwrap_or_default().transform_vector(&(target-self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);
        g.update_hierarchical_data_for_descendants(self.model);
    }

    pub fn butter(&self,g:&mut Graph,holder:Handle<Node>,camera:Handle<Node>,weapon:Handle<Node>,pose:trickcal_shooting_balddagu::motion::Pose,ticks:u8) {
        use trickcal_shooting_balddagu::butter_skill as b;
        if self.character!=11 || !g[holder].visibility() || ticks>0 {return;}
        let giant=super::butter_fx::is_giant(pose);
        if !giant && !self.left && b::weapon_visible(pose.motion,pose.age) {return;}
        for (node,rest) in &self.rest {g[*node].set_local_transform(rest.clone());}
        g[self.model].set_visible(true);
        g[self.model].local_transform_mut().set_position(Vector3::zeros()).set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        g.update_hierarchical_data_for_descendants(weapon);
        let facing=super::skeleton::world_rotation(g,camera);let sign=if self.left{1.0}else{-1.0};
        aim_joint(g,self.upper_arm,self.forearm,facing*Vector3::new(sign*0.20,-0.35,0.80));
        aim_joint(g,self.forearm,self.hand,facing*Vector3::new(sign*0.12,0.25,0.90));
        let holding=giant || b::left_holding(pose.motion,pose.age);
        self.grasp.orient(g,super::hand_grip::facing(facing,11,self.left));
        self.grasp.pose(g,if holding && b::loaded(pose.motion,pose.age){0.85}else{0.0});
        let target=if giant && pose.age>=b::ULTIMATE_RELEASE {
            let recoil=b::smooth((pose.age-b::ULTIMATE_RELEASE)/0.22);
            g[camera].global_transform().transform_point(&Vector3::new(sign*(0.208+0.10*recoil),-0.154-0.13*recoil,0.796-0.09*recoil).into()).coords
        }else if giant {
            g[super::butter_fx::draw_node(g,weapon,self.left)].global_position()
        }else if holding {
            g[weapons::grip_node(g,weapon,11,true)].global_position()
        }else {
            g[camera].global_transform().transform_point(&Vector3::new(sign*0.35,-0.50,0.56).into()).coords
        };
        let offset=g[holder].global_transform().try_inverse().unwrap_or_default().transform_vector(&(target-self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);g.update_hierarchical_data_for_descendants(self.model);
    }

    pub fn ui(&self, g: &mut Graph, pose: trickcal_shooting_balddagu::motion::Pose, ticks: u8) {
        if self.character == 10 && self.left {
            // The support hand joins the clover staff only during the E cast.
            g[self.model].set_visible(
                ticks == 0 && pose.motion == trickcal_shooting_balddagu::motion::Motion::UiSkill,
            );
        }
    }

    pub fn komi(&self,g:&mut Graph,holder:Handle<Node>,camera:Handle<Node>,pose:trickcal_shooting_balddagu::motion::Pose,state:trickcal_shooting_balddagu::komi_skill::State) {
        if self.character!=12 {return;}
        if !state.active() {
            // Carry the pillow with the primary hand; skills still use both hands.
            if self.left {g[self.model].set_visible(false);}
            return;
        }
        if !g[holder].visibility() {return;}
        for (node,rest) in &self.rest {g[*node].set_local_transform(rest.clone());}
        g[self.model].set_visible(true);
        g[self.model].local_transform_mut().set_position(Vector3::zeros()).set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let facing=super::skeleton::world_rotation(g,camera);let sign=if self.left{1.0}else{-1.0};
        aim_joint(g,self.upper_arm,self.forearm,facing*Vector3::new(sign*0.18,-0.35,0.80));
        aim_joint(g,self.forearm,self.hand,facing*Vector3::new(sign*0.12,0.25,0.90));
        let feast=pose.motion==trickcal_shooting_balddagu::motion::Motion::KomiFeast;
        let rotation=if feast {facing*super::komi_fx::tin_rotation(pose)}else{facing};
        self.grasp.orient(g,super::hand_grip::facing(rotation,12,self.left));
        self.grasp.pose(g,if feast{0.6}else{0.15});
        let position=if feast {super::komi_fx::view_tin(pose)+super::komi_fx::tin_rotation(pose)*super::komi_fx::grip(self.left)*0.85}
            else {Vector3::new(sign*0.34,if state.resting(){-0.43}else{-0.26},0.64)};
        let target=g[camera].global_transform().transform_point(&position.into()).coords;
        let offset=g[holder].global_transform().try_inverse().unwrap_or_default().transform_vector(&(target-self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);g.update_hierarchical_data_for_descendants(self.model);
    }

    pub fn chloe(
        &self,
        g: &mut Graph,
        holder: Handle<Node>,
        camera: Handle<Node>,
        p: trickcal_shooting_balddagu::motion::Pose,
    ) {
        use trickcal_shooting_balddagu::{chloe_skill as c, motion::Motion};
        if self.character != 6 { return; }
        let show = c::view_hands(p.motion, p.age);
        g[self.model].set_visible(show);
        if !show || !g[holder].visibility() { return; }
        for (node, rest) in &self.rest {
            g[*node].set_local_transform(rest.clone());
        }
        g[self.model].local_transform_mut()
            .set_position(Vector3::zeros())
            .set_scale(Vector3::repeat(MODEL_SCALE * 0.72));
        g.update_hierarchical_data_for_descendants(holder);
        let facing = super::skeleton::world_rotation(g, camera);
        let sign = if self.left { 1.0 } else { -1.0 };
        // The weapon socket opposes the support grip by 180 degrees. Empty
        // rider hands instead use their anatomical finger axes on BOTH sides,
        // with thumbs inward and fingers extending beyond the cuff.
        let finger = |prefix: &str| self.fingers.iter().copied()
            .find(|h| g[*h].name().starts_with(prefix)).expect("Chloe finger");
        let forward = g[finger("Middle1.")].global_position() - g[self.hand].global_position();
        let across = g[finger("Index1.")].global_position() - g[finger("Pinky1.")].global_position();
        let resting = palm_basis(
            facing * Vector3::new(-sign * 0.18, 0.24, 0.95),
            facing * Vector3::new(-sign, 0.25, 0.0),
        );
        let cover = if p.motion == Motion::ChloeRoar {
            c::smooth(p.age / 0.20) * (1.0 - c::smooth((p.age - 1.65) / 0.35))
        } else { 0.0 };
        let ears = palm_basis(
            facing * Vector3::y(),
            facing * Vector3::new(-sign * 0.25, 0.0, -1.0),
        );
        let delta = resting.slerp(&ears, cover) * palm_basis(forward, across).inverse();
        // Move sleeve and wrist together before the common corner framing pass.
        set_global_rotation(g, self.forearm,
            delta * super::skeleton::world_rotation(g, self.forearm));
        self.grasp.pose(g, 0.18 * (1.0 - cover));
        let position = Vector3::from(c::view_palm(p.motion, p.age, self.left));
        let target = g[camera].global_transform().transform_point(&position.into()).coords;
        let offset = g[holder].global_transform().try_inverse().unwrap_or_default()
            .transform_vector(&(target - self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);
        g.update_hierarchical_data_for_descendants(self.model);
    }

    pub fn verify_chloe_view(&self, g: &Graph, camera: Handle<Node>, p: trickcal_shooting_balddagu::motion::Pose) {
        let inverse = g[camera].global_transform().try_inverse().unwrap();
        let palm = inverse.transform_point(&self.grasp.contact(g).into()).coords;
        let target = Vector3::from(trickcal_shooting_balddagu::chloe_skill::view_palm(p.motion, p.age, self.left));
        assert!((palm - target).norm() < 0.001, "Chloe palm drifted from its camera target");
        self.verify_anatomical_hand(g, camera, false);
        self.verify_sleeve_corner(g, camera);
        if matches!(p.motion, trickcal_shooting_balddagu::motion::Motion::ChloeRideIdle
            | trickcal_shooting_balddagu::motion::Motion::ChloeRideMove
            | trickcal_shooting_balddagu::motion::Motion::ChloeRideAttack) {
            let middle = self.fingers.iter().find(|h| g[**h].name().starts_with("Middle1.")).unwrap();
            let forward = inverse.transform_vector(&(g[*middle].global_position() - g[self.hand].global_position())).normalize();
            assert!(forward.z > 0.85 && forward.y > 0.1, "Chloe fingers must extend forward beyond the cuff: {forward:?}");
        }
    }

    pub fn picora(&self,g:&mut Graph,holder:Handle<Node>,camera:Handle<Node>,pose:trickcal_shooting_balddagu::motion::Pose,ticks:u8) {
        use trickcal_shooting_balddagu::{picora_skill as p,motion::Motion};
        if self.character!=5 || !self.left || !g[holder].visibility() || ticks>0 {return;}
        let skill=pose.motion==Motion::PicoraSkill;
        g[self.model].set_visible(skill);
        if !skill {return;}
        for(node,rest) in &self.rest {g[*node].set_local_transform(rest.clone());}
        g[self.model].local_transform_mut().set_position(Vector3::zeros()).set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let facing=super::skeleton::world_rotation(g,camera);
        aim_joint(g,self.upper_arm,self.forearm,facing*Vector3::new(0.20,-0.35,0.80));
        aim_joint(g,self.forearm,self.hand,facing*Vector3::new(0.12,0.25,0.90));
        self.grasp.orient(g,super::hand_grip::facing(facing,5,true));self.grasp.pose(g,0.12);
        let path=p::hand_path(pose.motion,pose.age,true);
        let weight=(pose.age/0.15).clamp(0.0,1.0)*((p::SKILL_SECONDS-pose.age)/0.20).clamp(0.0,1.0);
        let position=Vector3::new(0.32+path[0]*0.13,-0.42+path[1]*0.16+0.12*weight,0.66+path[0]*0.12);
        let target=g[camera].global_transform().transform_point(&position.into()).coords;
        let offset=g[holder].global_transform().try_inverse().unwrap_or_default().transform_vector(&(target-self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);g.update_hierarchical_data_for_descendants(self.model);
    }

    pub fn bella(&self, g: &mut Graph, holder: Handle<Node>, camera: Handle<Node>, pose: trickcal_shooting_balddagu::motion::Pose, ticks: u8) {
        use trickcal_shooting_balddagu::{bella_skill as b, motion::Motion};
        if self.character != 9 || !g[holder].visibility() || ticks > 0 { return; }
        let ultimate=pose.motion==Motion::BellaUltimate && pose.age>=0.25;
        let skill=pose.motion==Motion::BellaSkill;
        g[self.model].set_visible(!self.left || skill || ultimate);
        if self.left && !skill && !ultimate {return;}
        for (node,rest) in &self.rest {g[*node].set_local_transform(rest.clone());}
        g[self.model].local_transform_mut().set_position(Vector3::zeros()).set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let facing=super::skeleton::world_rotation(g,camera);let sign=if self.left{1.0}else{-1.0};
        aim_joint(g,self.upper_arm,self.forearm,facing*Vector3::new(sign*0.20,-0.35,0.80));
        aim_joint(g,self.forearm,self.hand,facing*Vector3::new(sign*0.12,0.25,0.90));
        let path=b::hand(pose.motion,pose.age,self.left);
        // Socket +Z is the palm normal: point it upward, with fingers forward.
        let x=Vector3::z()*sign;let y=Vector3::x()*sign;let z=x.cross(&y);
        self.grasp.orient(g,if ultimate&&!self.left {facing} else {facing*UnitQuaternion::from_matrix(&Matrix3::from_columns(&[x,y,z]))});
        self.grasp.pose(g,if ultimate&&!self.left {0.8}else{0.0});
        let attack=if pose.motion.is_attack(){(pose.age/pose.motion.seconds(9)*std::f32::consts::PI).sin().max(0.0)}else{0.0};
        let position=Vector3::new(sign*(if ultimate {0.40}else{0.32})+(path[0]*0.06).clamp(-0.06,0.06),
            (if ultimate&&!self.left {0.10}else{-0.27})+(path[1]*0.15).clamp(-0.12,0.15),
            0.68+(-path[0]*0.10).clamp(-0.10,0.12)+attack*0.12);
        let target=g[camera].global_transform().transform_point(&position.into()).coords;
        let offset=g[holder].global_transform().try_inverse().unwrap_or_default().transform_vector(&(target-self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);g.update_hierarchical_data_for_descendants(self.model);
    }

    pub fn renua(&self, g: &mut Graph, holder: Handle<Node>, camera: Handle<Node>, pose: trickcal_shooting_balddagu::motion::Pose) {
        if self.character != 7 || !g[holder].visibility() { return; }
        for (node, rest) in &self.rest { g[*node].set_local_transform(rest.clone()); }
        g[self.model].set_visible(true);
        g[self.model].local_transform_mut().set_position(Vector3::zeros()).set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let path=trickcal_shooting_balddagu::renua_skill::hand(pose.motion,pose.age,self.left);
        let sign=if self.left {1.0}else{-1.0};
        let facing=super::skeleton::world_rotation(g,camera);
        aim_joint(g,self.upper_arm,self.forearm,facing*Vector3::new(sign*0.18,-0.35,0.80));
        aim_joint(g,self.forearm,self.hand,facing*Vector3::new(sign*0.12,0.25,0.90));
        let x=Vector3::new(0.0,0.15,if self.left {1.0}else{-1.0}).normalize();
        let y=Vector3::x()*if self.left {-1.0}else{1.0};let z=x.cross(&y);
        self.grasp.orient(g,facing*UnitQuaternion::from_matrix(&Matrix3::from_columns(&[x,y,z])));
        self.grasp.pose(g,0.0);
        let position=Vector3::new(sign*(0.32+(path[0]*0.08).clamp(-0.07,0.09)),
            -0.28+(path[1]*0.22).clamp(-0.14,0.26),0.65+(-path[0]*0.16).clamp(-0.10,0.20));
        let target=g[camera].global_transform().transform_point(&position.into()).coords;
        let offset=g[holder].global_transform().try_inverse().unwrap_or_default().transform_vector(&(target-self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);
        g.update_hierarchical_data_for_descendants(self.model);
    }

    pub fn id(&self, g: &mut Graph, holder: Handle<Node>, camera: Handle<Node>, shot_age: f32) {
        if self.character != 3 || !g[holder].visibility() {
            return;
        }
        for (node, pose) in &self.rest {
            g[*node].set_local_transform(pose.clone());
        }
        g[self.model]
            .local_transform_mut()
            .set_position(Vector3::zeros())
            .set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let point = trickcal_shooting_balddagu::id_skill::point_amount(shot_age);
        let facing = super::skeleton::world_rotation(g, camera);
        aim_joint(
            g,
            self.upper_arm,
            self.forearm,
            facing * Vector3::new(-0.10, 0.20, 0.9),
        );
        aim_joint(
            g,
            self.forearm,
            self.hand,
            facing * Vector3::new(-0.12, 0.35, 0.85),
        );
        // The refined right-hand socket has its -X axis along the fingers.
        // Palm points down, leaving the back of the open hand facing the camera.
        let x = -Vector3::new(0.0, 0.36 - 0.26 * point, 1.0).normalize();
        let y = Vector3::x();
        let z = x.cross(&y);
        self.grasp.orient(
            g,
            facing * UnitQuaternion::from_matrix(&Matrix3::from_columns(&[x, y, z])),
        );
        self.grasp.point(g, point * 0.95);
        let target = g[camera]
            .global_transform()
            .transform_point(
                &Vector3::new(
                    -0.25 + point * 0.04,
                    -0.22 + point * 0.045,
                    0.64 + point * 0.04,
                )
                .into(),
            )
            .coords;
        let offset = g[holder]
            .global_transform()
            .try_inverse()
            .unwrap_or_default()
            .transform_vector(&(target - self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);
        g.update_hierarchical_data_for_descendants(self.model);
    }

    pub fn erpin_skill(&self, g: &mut Graph, holder: Handle<Node>, camera: Handle<Node>, pose: trickcal_shooting_balddagu::motion::Pose) {
        use trickcal_shooting_balddagu::{erpin_skill, motion::Motion};
        if self.character != 2 || !self.left || !g[holder].visibility()
            || !matches!(pose.motion, Motion::ErpinSkill | Motion::ErpinStart | Motion::ErpinCharge | Motion::ErpinRecover) { return; }
        for (node, rest) in &self.rest { g[*node].set_local_transform(rest.clone()); }
        g[self.model].set_visible(true);
        g[self.model].local_transform_mut().set_position(Vector3::zeros()).set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let path = erpin_skill::hand(pose.motion, pose.age, true);
        let facing = super::skeleton::world_rotation(g,camera);
        aim_joint(g,self.upper_arm,self.forearm,facing*Vector3::new(0.20,-0.35,0.8));
        aim_joint(g,self.forearm,self.hand,facing*Vector3::new(0.12,0.25,0.85));
        let palm = if pose.motion == Motion::ErpinSkill {
            // The left fist keeps its palm side up, tilted toward the player.
            // Socket +X follows the fingers and +Z is the palm normal.
            facing * palm_basis(Vector3::new(-0.20, 0.35, 1.0), Vector3::x())
        } else {
            super::hand_grip::facing(facing, 2, true)
        };
        self.grasp.orient(g, palm);
        if pose.motion == Motion::ErpinSkill {
            self.grasp.fist(g);
        } else {
            self.grasp.pose(g, 0.12);
        }
        let position = Vector3::new(0.32,-0.27+(path[1]*0.24).clamp(-0.14,0.22),0.60+(path[0]*0.16).clamp(-0.10,0.16));
        let target = g[camera].global_transform().transform_point(&position.into()).coords;
        let offset = g[holder].global_transform().try_inverse().unwrap_or_default().transform_vector(&(target-self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);
        g.update_hierarchical_data_for_descendants(self.model);
    }

    pub fn verify_naia_view(
        &self,
        g: &Graph,
        camera: Handle<Node>,
        pose: trickcal_shooting_balddagu::motion::Pose,
    ) {
        use trickcal_shooting_balddagu::{motion::Motion, naia_skill};
        let low_grade = pose.motion.is_naia_skill() && pose.motion != Motion::NaiaUltimate;
        for cuff in &self.naia_cuffs {
            cuff.verify(g, camera, low_grade);
        }
        let factor = if low_grade {
            (1.0 + 0.9 * naia_skill::whale_form(pose.motion, pose.age))
                * super::naia_whales::view_scale(pose)
        } else {
            1.0
        };
        assert!((g[self.model].local_transform().scale().x - MODEL_SCALE * factor).abs() < 1e-6);
        self.verify_anatomical_hand(g, Handle::NONE, false);
        if low_grade && naia_skill::whale_form(pose.motion, pose.age) > 0.99 {
            let middle = self
                .fingers
                .iter()
                .find(|h| g[**h].name().starts_with("Middle1."))
                .unwrap();
            let fingers =
                (g[*middle].global_position() - g[self.hand].global_position()).normalize();
            let forward = super::skeleton::world_rotation(g, g[self.model].parent()) * Vector3::z();
            assert!(
                fingers.dot(&forward) > 0.8,
                "both Naia hands must point forward around the dolphin"
            );
        }
    }

    pub fn eat(
        &self,
        g: &mut Graph,
        holder: Handle<Node>,
        camera: Handle<Node>,
        age: f32,
    ) -> Vector3<f32> {
        for (node, pose) in &self.rest {
            g[*node].set_local_transform(pose.clone());
        }
        let visible = g[holder].visibility();
        g[self.model].set_visible(visible);
        g[self.model]
            .local_transform_mut()
            .set_position(Vector3::zeros())
            .set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let facing = super::skeleton::world_rotation(g, camera);
        aim_joint(
            g,
            self.upper_arm,
            self.forearm,
            facing * Vector3::new(-0.3, 0.45, 0.65),
        );
        aim_joint(
            g,
            self.forearm,
            self.hand,
            facing * Vector3::new(-0.25, 0.55, 0.8),
        );
        let palm = UnitQuaternion::from_matrix(&Matrix3::from_columns(&[
            Vector3::z(),
            Vector3::x(),
            Vector3::y(),
        ]));
        self.grasp.orient(g, facing * palm);
        self.grasp.pose(g, 0.18);
        let position = vec(trickcal_shooting_balddagu::erpin::hand_position(age));
        let target = g[camera]
            .global_transform()
            .transform_point(&position.into())
            .coords;
        let offset = g[holder]
            .global_transform()
            .try_inverse()
            .unwrap_or_default()
            .transform_vector(&(target - self.grasp.contact(g)));
        g[self.model].local_transform_mut().set_position(offset);
        g.update_hierarchical_data_for_descendants(self.model);
        self.grasp.contact(g)
    }

    fn hold(&self, g: &mut Graph, holder: Handle<Node>, weapon: Handle<Node>) {
        let Some(grip) = weapons::grips(self.character)[usize::from(self.left)] else {
            g[self.model].set_visible(false);
            return;
        };
        for (node, pose) in &self.rest {
            g[*node].set_local_transform(pose.clone());
        }
        // Share walking bob/recoil, then follow the contact or empty gesture target.
        let motion = g[g[weapon].parent()].local_transform().clone();
        g[holder].set_local_transform(motion);
        g[self.model]
            .local_transform_mut()
            .set_position(Vector3::zeros())
            .set_scale(Vector3::repeat(MODEL_SCALE));
        g.update_hierarchical_data_for_descendants(holder);
        let facing = super::skeleton::world_rotation(g, holder);
        let sign = if self.left { -1.0 } else { 1.0 };
        aim_joint(
            g,
            self.upper_arm,
            self.forearm,
            facing
                * Vector3::new(
                    sign * 0.15,
                    if self.character == 0 { 1.0 } else { 0.65 },
                    0.75,
                ),
        );
        aim_joint(
            g,
            self.forearm,
            self.hand,
            facing
                * if self.character == 2 {
                    // Keep the wide right cuff below the wrist instead of covering the grip.
                    Vector3::new(sign * 0.10, 0.90, 0.45)
                } else {
                    Vector3::new(sign * 0.20, 0.45, 0.85)
                },
        );

        let grip_node = weapons::grip_node(g, weapon, self.character, self.left);
        let grip_facing = self.grip_facing(g, holder, grip_node);
        if self.character == 0 {
            // The open palm directs the blades from its own independent gesture target.
            let delta = grip_facing * facing.inverse();
            let follow = UnitQuaternion::identity().slerp(&delta, 0.65);
            set_global_rotation(
                g,
                self.forearm,
                follow * super::skeleton::world_rotation(g, self.forearm),
            );
        }
        let facing = grip_facing;

        // The source cuff straddles the wrist. Turn the whole forearm into the
        // palm frame before closing fingers so the wrist never folds it over the
        // hand. Translation below still puts the same palm contact on the grip.
        let turn = facing * super::skeleton::world_rotation(g, self.grasp.socket).inverse();
        set_global_rotation(
            g,
            self.forearm,
            turn * super::skeleton::world_rotation(g, self.forearm),
        );
        self.grasp.orient(g, facing);
        self.grasp.pose(g, super::hand_grip::amount(self.character));
        let contact = self.grasp.contact(g);
        let target = g[grip_node]
            .global_transform()
            .transform_point(&grip.into())
            .coords;
        let inverse = g[holder]
            .global_transform()
            .try_inverse()
            .unwrap_or_default();
        let offset = inverse.transform_vector(&(target - contact));
        g[self.model].local_transform_mut().set_position(offset);
        g.update_hierarchical_data_for_descendants(self.model);
    }
}

fn palm_basis(forward: Vector3<f32>, across: Vector3<f32>) -> UnitQuaternion<f32> {
    let x = forward.normalize();
    let z = x.cross(&across).normalize();
    let y = z.cross(&x).normalize();
    UnitQuaternion::from_matrix(&Matrix3::from_columns(&[x, y, z]))
}
