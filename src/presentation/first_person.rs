//! First-person grip and greeting made from the equipped character's actual FBX arms.
//! Only the index buffers are filtered. Source vertices, skinning, UVs and materials stay intact.
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
    pub triangles: usize,
}

impl FirstPersonArm {
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
            .set_scale(Vector3::repeat(0.01));
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
                g[node].set_visibility(true);
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
                surfaces.push(
                    SurfaceBuilder::new(SurfaceResource::new_embedded(arm_data))
                        .with_material(surface.material().clone())
                        .with_bones(surface.bones().to_vec())
                        .build(),
                );
            }
            g[node].cast_mut::<Mesh>().unwrap().set_surfaces(surfaces);
            g[node].set_cast_shadows(false);
        }
        if triangles == 0 || triangles >= source_triangles {
            g.remove_node(model);
            return Err(format!(
                "{}: could not isolate the original arm mesh",
                characters::SLUGS[visual.id as usize]
            ));
        }
        println!("FIRST_PERSON_ARM_OK: {}, {triangles}/{source_triangles} original triangles, {materials} original materials", characters::SLUGS[visual.id as usize]);
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
            triangles,
        })
    }

    pub fn remove(self, g: &mut Graph) {
        g.remove_node(self.model);
    }

    pub fn verify_anatomical_hand(&self, g: &Graph, _camera: Handle<Node>, held: bool) {
        assert_eq!(
            g[self.hand].name(),
            super::skeleton::arm_bone_names(self.left)[2]
        );
        assert!(
            g[self.hand].global_transform().determinant() > 0.0,
            "arm must not be mirrored"
        );
        self.grasp.verify(g, held);
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
        UnitQuaternion::from_rotation_matrix(
            &fyrox::core::algebra::Rotation3::from_matrix_unchecked(Matrix3::from_columns(&[
                x, shaft, z,
            ])),
        )
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

    /// Raise the equipped forearm around its elbow, then return to the same grip.
    pub fn update(
        &self,
        g: &mut Graph,
        holder: Handle<Node>,
        camera: Handle<Node>,
        weapon: Handle<Node>,
        ticks: u8,
    ) {
        g[self.model].set_visibility(!self.left || ticks == 0);
        if !g[holder].visibility() {
            return;
        }
        if self.left && ticks > 0 {
            return;
        }
        self.hold(g, holder, weapon);
        let (lift, sway) = wave_pose(ticks);
        if lift == 0.0 {
            return;
        }

        // Keep the grip's holder, model scale and upper arm. The elbow must not
        // slide sideways when the wrist starts waving or the weapon returns.
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
            assert!(
                (g[self.forearm].global_position() - elbow).norm() < 0.0001,
                "{slug}: greeting elbow left its equipped position at tick {ticks}"
            );
            assert!((g[self.upper_arm].global_position() - shoulder).norm() < 0.0001);
            assert!(((wrist - elbow).norm() - length).abs() < 0.0001);
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
                assert!(self.grip_error(g, holder, weapon) < 0.001);
            }
            if wave_pose(ticks).0 == 1.0 {
                let up = super::skeleton::world_rotation(g, camera) * Vector3::y();
                raised |= (wrist - grip_wrist).dot(&up) > 0.01;
            }
            previous_wrist = wrist;
            previous_rotation = rotation;
        }
        assert!(raised, "greeting must actually raise the equipped hand");
        println!(
            "GREETING_CONTINUITY_OK: {}, 109 ticks, fixed elbow/scale, max wrist step {max_step:.5} m, max hand step {max_angle:.5} rad",
            characters::SLUGS[self.character as usize]
        );
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
        g[self.model].set_visibility(visible);
        g[self.model]
            .local_transform_mut()
            .set_position(Vector3::zeros())
            .set_scale(Vector3::repeat(0.01));
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
            g[self.model].set_visibility(false);
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
            .set_scale(Vector3::repeat(0.01));
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
