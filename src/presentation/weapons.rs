//! Imported Meshy props and animated weapon contact sockets.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    asset::manager::ResourceManager,
    core::futures::{executor::block_on, future::join_all},
    resource::model::{Model, ModelResource, ModelResourceExtension},
};

/// World-only proportions matched to the archived character illustrations.
/// Props are authored for the viewmodel; the old uniform 0.70 multiplier made
/// parasols, staves and long guns much smaller than their character silhouettes.
/// Keep the viewmodel, floating blades, worn companions and small props separate.
pub(super) fn world_scale(id: u8) -> f32 {
    match id {
        0 => 1.0,
        2 => 1.55,  // Erpin: staff reaches from the feet to above the crown.
        4 => 1.40,  // Aya: open flower canopy spans the head and shoulders.
        5 => 2.00,  // Picora: long decorated staff, close to body height.
        10 => 2.20, // Ui: clover rises above the hat while seated on Eru.
        11 => 1.00, // Butter: slingshot fork is about one head high.
        13 => 1.65, // Hilde: long medical gun, not a handgun.
        14 => 2.20, // Shion: oversized rifle is longer than her body height.
        _ => 0.70,
    }
}

/// Lean enlarged staff heads away from the hair/hat, rotating about the palm.
/// Positive X pitch carries the upright (+Y) staff forward along actor-local +Z.
pub(super) fn world_pitch(id: u8, animated_pitch: f32) -> f32 {
    let forward = match id {
        5 => 30.0_f32.to_radians(),
        10 => 35.0_f32.to_radians(),
        _ => return animated_pitch,
    };
    // Looking up or an attack's backswing must not pull the head into the face.
    (animated_pitch + forward).max(forward)
}

/// View props must stay proportional to the enlarged camera-mounted hands.
/// Scale about the grip so the hand sockets and animated effects follow it.
pub(super) fn first_person_scale(id: u8) -> f32 {
    match id {
        2 => 1.08,  // Erpin: substantial staff head and shaft beside the palm.
        4 => 1.24,  // Aya: oversized parasol; the canopy may extend beyond the view.
        5 => 1.20,  // Picora: full staff and curled head proportional to the view hand.
        10 => 1.30, // Ui: broad clover instead of a leaf smaller than the hand.
        12 => 1.30, // Komi: a full-sized pillow, about 1.8x the previous view prop.
        14 => 1.30, // Shion: a full-sized rifle alongside the enlarged view hands.
        _ => 0.72,
    }
}

pub(super) fn first_person_position(id: u8) -> Vector3<f32> {
    match id {
        // Empty hands are in front; levitating blades rest behind the shoulders.
        0 => Vector3::zeros(),
        // Keep the larger staff heads in frame, with the grip at the same
        // lower-right screen position. Camera-local -X is screen right.
        2 => Vector3::new(-0.54, -0.35, 0.70),
        5 => Vector3::new(-0.50, -0.36, 0.70),
        10 => Vector3::new(-0.50, -0.32, 0.64),
        // Bring the pillow's grasped right edge into view, with room for its sweep.
        12 => Vector3::new(-0.20, -0.22, 0.68),
        14 => Vector3::new(-0.40, -0.27, 0.62),
        _ => Vector3::new(-0.44, -0.28, 0.56),
    }
}

/// Contact points in weapon space, before its presentation scale is applied.
/// Imported weapons have their grip at the origin; procedural parts retain their authored offsets.
pub(super) fn grips(id: u8) -> [Option<Vector3<f32>>; 2] {
    let right = match id {
        3 => Vector3::new(0.0, -0.13, -0.05),
        6 => Vector3::new(-0.21, -0.12, 0.0),
        12 => Vector3::new(-0.20, 0.02, 0.0),
        _ => Vector3::zeros(),
    };
    let left = match id {
        0 => Some(Vector3::zeros()),
        1 => Some(Vector3::zeros()),
        11 => Some(Vector3::zeros()),
        6 => Some(Vector3::new(0.21, -0.12, 0.0)),
        12 => Some(Vector3::new(0.20, 0.02, 0.0)),
        14 => Some(Vector3::new(0.0, -0.035, 0.30)),
        10 => Some(Vector3::new(0.0, 0.065, 0.0)),
        _ => None,
    };
    [Some(right), left]
}

const ERPIN_GEM_SOCKET: &str = "Erpin staff gem";
const PICORA_HEAD_SOCKET: &str = "Picora staff head";
const MAYO_MUZZLE_SOCKET: &str = "Mayo blowpipe muzzle";
pub(super) const MAYO_MUZZLE: Vector3<f32> = Vector3::new(0.0, 0.0, 0.535);
const HILDE_MUZZLE_SOCKET: &str = "Hilde syringe muzzle";
const SHION_MUZZLE_SOCKET: &str = "Shion rifle muzzle";

pub(super) fn shion_muzzle(g: &Graph, weapon: Handle<Node>) -> Vector3<f32> {
    let socket = g[weapon].children().iter().copied()
        .find(|&node| g[node].name() == SHION_MUZZLE_SOCKET)
        .expect("Shion rifle muzzle socket");
    g[socket].global_position()
}

pub(super) fn mayo_muzzle(g: &Graph, weapon: Handle<Node>) -> Vector3<f32> {
    let socket = g[weapon].children().iter().copied()
        .find(|&node| g[node].name() == MAYO_MUZZLE_SOCKET)
        .expect("Mayo blowpipe muzzle socket");
    g[socket].global_position()
}

pub(super) fn picora_muzzle(g: &Graph, weapon: Handle<Node>) -> Vector3<f32> {
    let socket = g[weapon]
        .children()
        .iter()
        .copied()
        .find(|&node| g[node].name() == PICORA_HEAD_SOCKET)
        .expect("Picora staff head socket");
    g[socket].global_position()
}

pub(super) fn hilde_muzzle(g: &Graph, weapon: Handle<Node>) -> Vector3<f32> {
    let socket = g[weapon]
        .children()
        .iter()
        .copied()
        .find(|&node| g[node].name() == HILDE_MUZZLE_SOCKET)
        .expect("Hilde syringe muzzle socket");
    g[socket].global_position()
}

pub(super) fn erpin_muzzle(g: &Graph, weapon: Handle<Node>) -> Vector3<f32> {
    let socket = g[weapon]
        .children()
        .iter()
        .copied()
        .find(|&node| g[node].name() == ERPIN_GEM_SOCKET)
        .expect("Erpin staff gem socket");
    g[socket].global_position()
}

pub(super) const VIVI_SWING_SECONDS: f32 = 0.52;
const VIVI_BLADES: [&str; 4] = ["Mercury sword.R", "Mercury sword.R.outer", "Mercury sword.L", "Mercury sword.L.outer"];
const VIVI_HANDS: [&str; 2] = ["Empty hand gesture.R", "Empty hand gesture.L"];

pub(super) fn vivi_blade_node(g: &Graph, weapon: Handle<Node>, left: bool) -> Handle<Node> {
    g[weapon]
        .children()
        .iter()
        .copied()
        .find(|h| g[*h].name() == VIVI_BLADES[usize::from(left)*2])
        .expect("Vivi floating blade")
}

fn vivi_gestures(g: &mut Graph, weapon: Handle<Node>) {
    for name in VIVI_HANDS {
        let node: Handle<Node> = PivotBuilder::new(BaseBuilder::new().with_name(name))
            .build(g)
            .to_base();
        g.link_nodes(node, weapon);
    }
}

/// Vivi uses empty gesture targets. Her blades never serve as hand contacts.
pub(super) fn grip_node(g: &Graph, weapon: Handle<Node>, id: u8, left: bool) -> Handle<Node> {
    if id == 1 {
        g[weapon]
            .children()
            .iter()
            .copied()
            .find(|h| g[*h].name() == super::naia_whales::HANDS[usize::from(left)])
            .expect("Naia whale cradle socket")
    } else if id == 0 {
        g[weapon]
            .children()
            .iter()
            .copied()
            .find(|h| g[*h].name() == VIVI_HANDS[usize::from(left)])
            .expect("Vivi empty hand gesture target")
    } else if id == 11 && left {
        g[weapon]
            .children()
            .iter()
            .copied()
            .find(|h| g[*h].name() == "Slingshot draw")
            .expect("slingshot draw socket")
    } else {
        weapon
    }
}

/// The four independent blades and the empty hand targets share the authoritative pose.
pub(super) fn update_vivi_pose(g: &mut Graph, weapon: Handle<Node>, pose: trickcal_shooting_balddagu::motion::Pose, time: f32, first: bool) {
    use trickcal_shooting_balddagu::vivi_skill as v;
    if first {
        for left in [false,true] {
            let side=if left {1.0}else{-1.0};
            let motion=v::hand(pose.motion,pose.age,left);
            let gesture=grip_node(g,weapon,0,left);
            g[gesture].local_transform_mut()
                .set_position(Vector3::new(side*0.52,-0.38,0.72)+Vector3::from(motion.position))
                .set_rotation(UnitQuaternion::from_euler_angles(0.25+motion.angles[0],-side*0.30+motion.angles[1],-side*0.35+motion.angles[2]));
        }
    }
    for (i,name) in VIVI_BLADES.into_iter().enumerate() {
        let blade=g[weapon].children().iter().copied().find(|&h|g[h].name()==name).expect("Vivi blade");
        let (p,a)=v::blade(i,pose.motion,pose.age,time);
        let mut p=Vector3::from(p);
        if first {p.y-=1.45;p.z-=0.12;}
        g[blade].local_transform_mut().set_position(p)
            .set_rotation(UnitQuaternion::from_euler_angles(a[0],a[1],a[2]));
    }
}
pub(super) fn update_slingshot(g: &mut Graph, weapon: Handle<Node>, phase: f32) {
    let held = trickcal_shooting_balddagu::attack::hand(11, false, phase);
    let draw = trickcal_shooting_balddagu::attack::hand(11, true, phase);
    let pull = (draw.position[2] - held.position[2]).abs().min(0.20);
    let position = Vector3::new(0.0, 0.24, -0.05 - pull * 1.8);
    let socket = grip_node(g, weapon, 11, true);
    g[socket].local_transform_mut().set_position(position);
    let descendants: Vec<_> = g.traverse_handle_iter(weapon).collect();
    for node in descendants {
        if let Some(mesh) = g[node].cast_mut::<fyrox::scene::mesh::Mesh>() {
            for shape in mesh.blend_shapes_mut() {
                if shape.name.ends_with("bandPull") {
                    shape.weight = (pull / 0.20).clamp(0.0, 1.0) * 100.0;
                }
            }
        }
    }
    let children = g[weapon].children().to_vec();
    let mut elastic = 0;
    for child in children {
        if g[child].name() == "Stone" {
            g[child].local_transform_mut().set_position(position);
        } else if g[child].name() == "Elastic" {
            let anchor = Vector3::new(if elastic == 0 { -0.16 } else { 0.16 }, 0.24, -0.04);
            elastic += 1;
            let direction = position - anchor;
            g[child]
                .local_transform_mut()
                .set_position((anchor + position) * 0.5)
                .set_scale(Vector3::new(direction.norm(), 0.02, 0.025))
                .set_rotation(
                    UnitQuaternion::rotation_between(&Vector3::x(), &direction.normalize())
                        .unwrap_or_default(),
                );
        }
    }
}

pub(super) fn load(manager: &ResourceManager) -> Result<Vec<Option<ModelResource>>, String> {
    let ids = [0_usize, 1, 2, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14];
    let pending = ids.map(|id| {
        manager.request::<Model>(characters::asset_path(&format!(
            "{}-weapon.fbx",
            characters::SLUGS[id]
        )))
    });
    let mut models = vec![None; 15];
    for (id, result) in ids.into_iter().zip(block_on(join_all(pending))) {
        let model = result.map_err(|e| format!("{} weapon: {e:?}", characters::SLUGS[id]))?;
        if matches!(id, 1 | 4 | 11) {
            super::morph_import::restore(&model, &format!("{}-weapon", characters::SLUGS[id]))?;
        }
        if !model
            .data_ref()
            .get_scene()
            .graph
            .linear_iter()
            .any(|node| node.cast::<fyrox::scene::mesh::Mesh>().is_some())
        {
            return Err(format!("{} weapon mesh missing", characters::SLUGS[id]));
        }
        println!("CHARACTER_WEAPON_OK: {}", characters::SLUGS[id]);
        models[id] = Some(model);
    }
    Ok(models)
}

pub(super) fn build(
    g: &mut Graph,
    shapes: &Shapes,
    id: u8,
    model: Option<&ModelResource>,
    first_person: bool,
) -> Handle<Node> {
    let root: Handle<Node> = PivotBuilder::new(BaseBuilder::new().with_name(kit(id).weapon))
        .build(g)
        .to_base();
    if matches!(id, 2 | 5) && first_person {
        // Twist around the staff's own shaft before the parent applies the
        // attack pitch. The grip stays fixed and the head socket follows it.
        g[root].local_transform_mut().set_rotation(
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), std::f32::consts::FRAC_PI_2),
        );
    }
    if id == 10 {
        super::ui_fx::attach_staff_glow(g, root, model.is_some());
    }
    if id == 5 {
        // Center of the decorated curl after FBX's Z-up -> Y-up conversion.
        // A child socket follows the grip, animation and both presentation scales.
        let socket: Handle<Node> =
            PivotBuilder::new(BaseBuilder::new().with_name(PICORA_HEAD_SOCKET))
                .build(g)
                .to_base();
        g[socket].local_transform_mut().set_position(Vector3::new(0.0, 0.51, 0.0));
        g.link_nodes(socket, root);
    }
    if id == 8 {
        // FBX maps the tube's -Y end to +Z; the imported grip adds 0.05 m.
        // Its radial correction centers the bore at X=Y=0 in weapon space.
        let socket: Handle<Node> = PivotBuilder::new(
            BaseBuilder::new().with_name(MAYO_MUZZLE_SOCKET),
        ).build(g).to_base();
        g[socket].local_transform_mut().set_position(if model.is_some() {
            MAYO_MUZZLE
        } else {
            Vector3::new(0.0, 0.0, 0.63)
        });
        g.link_nodes(socket, root);
    }
    if id == 13 {
        // In corrected weapon space, after the imported mesh's half turn and
        // grip translation. Parenting here includes recoil and both view scales.
        let center = if model.is_some() {
            Vector3::new(0.0, 0.15, 0.565)
        } else {
            Vector3::new(0.0, 0.0, 0.655)
        };
        let socket: Handle<Node> = PivotBuilder::new(
            BaseBuilder::new().with_name(HILDE_MUZZLE_SOCKET),
        )
        .build(g)
        .to_base();
        g[socket].local_transform_mut().set_position(center);
        g.link_nodes(socket, root);
    }
    if id == 14 {
        // FBX Z-up conversion, then the imported rifle's half turn and +0.42
        // grip correction below: the barrel tip lies at +1.055 along Z.
        let center = if model.is_some() {
            Vector3::new(0.0, 0.04, 1.055)
        } else {
            Vector3::new(0.0, 0.0, 0.70)
        };
        let socket: Handle<Node> = PivotBuilder::new(
            BaseBuilder::new().with_name(SHION_MUZZLE_SOCKET),
        ).build(g).to_base();
        g[socket].local_transform_mut().set_position(center);
        g.link_nodes(socket, root);
    }
    if id == 2 {
        // Center of the suspended pink diamond in erpin-weapon.fbx, after
        // FBX's Z-up -> Y-up conversion. The old 0.72 height was above the gem.
        let center = if model.is_some() {
            Vector3::new(0.0, 0.575, 0.0)
        } else {
            Vector3::new(0.05, 0.48, 0.0)
        };
        let socket: Handle<Node> =
            PivotBuilder::new(BaseBuilder::new().with_name(ERPIN_GEM_SOCKET))
                .build(g)
                .to_base();
        g[socket].local_transform_mut().set_position(center);
        g.link_nodes(socket, root);
    }
    if let Some(model) = model {
        let data = model.data_ref();
        if id == 0 {
            vivi_gestures(g, root);
            for name in VIVI_BLADES {
                let blade: Handle<Node> = PivotBuilder::new(BaseBuilder::new().with_name(name))
                    .build(g)
                    .to_base();
                g.link_nodes(blade, root);
                let (imported, _) = ModelResource::instantiate_from(
                    model.clone(),
                    &data,
                    data.get_scene().graph.get_root(),
                    g,
                    &mut |_, _| {},
                );
                g[imported]
                    .local_transform_mut()
                    .set_scale(Vector3::repeat(0.01));
                g.link_nodes(imported, blade);
            }
            return root;
        }
        let (imported, _) = ModelResource::instantiate_from(
            model.clone(),
            &data,
            data.get_scene().graph.get_root(),
            g,
            &mut |_, _| {},
        );
        g[imported]
            .local_transform_mut()
            .set_scale(Vector3::repeat(0.01));
        if id == 4 && first_person {
            // The delivered parasol's handle is behind and above the authored
            // origin. Center the middle of its straight handle inside the palm,
            // and align the tilted shaft with the fingers' grasp axis (+Y).
            // Apply this to the imported assembly so folding/blades follow it.
            let grip = Vector3::new(0.0135, 0.375, -0.089);
            let shaft = Vector3::new(-0.055, 1.0, 0.19).normalize();
            let rotation = UnitQuaternion::rotation_between(&shaft, &Vector3::y()).unwrap();
            g[imported]
                .local_transform_mut()
                .set_rotation(rotation)
                .set_position(-(rotation * grip));
        } else if id == 8 {
            // Move the rear rim just behind the grip, with the long tube ahead
            // of the hand, and center its axis inside the curled fingers.
            // Match its cross-section to the hand's authored grasp clearance;
            // view hands are enlarged independently of the weapon's length.
            let hand_scale = if first_person { super::first_person::HAND_SCALE } else { 1.0 };
            let prop_scale = if first_person { first_person_scale(id) } else { world_scale(id) };
            let radial_scale = super::hand_grip::radius(id, false) * hand_scale / (0.068 * prop_scale);
            g[imported]
                .local_transform_mut()
                .set_scale(Vector3::new(0.01 * radial_scale, 0.01 * radial_scale, 0.01))
                .set_position(Vector3::new(0.0, -0.04 * radial_scale, 0.05));
        } else if id == 13 {
            // Hilde's imported muzzle also points along -Z. Her actual grip is
            // at (0, -0.06, 0.40), not the authored origin in front of the trigger.
            // Correct only the mesh so both views retain their animated palm socket.
            g[imported]
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::y_axis(),
                    std::f32::consts::PI,
                ))
                .set_position(Vector3::new(0.0, 0.06, 0.40));
        } else if id == 14 {
            // This rifle was imported with its muzzle along -Z and its trigger
            // grip at +Z. Turn the mesh (not the hand sockets) around, then put
            // the trigger back at the origin: stock behind, barrel along +Z.
            g[imported]
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::y_axis(),
                    std::f32::consts::PI,
                ))
                .set_position(Vector3::new(0.0, 0.0, 0.42));
        }
        g.link_nodes(imported, root);
        if id == 10 {
            let leaf = g.traverse_handle_iter(imported).find(|&h|g[h].name()=="Ui_Clover");
            if let Some(leaf)=leaf {
                let parent=g[leaf].parent();
                let position=**g[leaf].local_transform().position();
                let pivot:Handle<Node>=PivotBuilder::new(BaseBuilder::new().with_name("Ui spinning clover hub")).build(g).to_base();
                g[pivot].local_transform_mut().set_position(position);
                g.link_nodes(pivot,parent);
                g[leaf].local_transform_mut().set_position(Vector3::zeros());
                g.link_nodes(leaf,pivot);
            }
        }
        if id == 1 {
            g[imported].set_name("Naia transforming gun");
            super::naia_whales::sockets(g, root);
        }
        if id == 4 {
            // Preserve FBX unit conversion on the mesh. Animate a separate hilt
            // pivot; replacing the imported mesh scale shrinks blades 100-fold.
            let blades: Vec<_> = g
                .traverse_handle_iter(imported)
                .filter(|h| g[*h].name().starts_with("Aya_Blade_"))
                .collect();
            for blade in blades {
                let parent = g[blade].parent();
                let position = **g[blade].local_transform().position();
                let pivot: Handle<Node> = PivotBuilder::new(
                    BaseBuilder::new()
                        .with_name(g[blade].name().replace("Aya_Blade_", "Aya_Emerge_")),
                )
                .build(g)
                .to_base();
                g[pivot].local_transform_mut().set_position(position);
                g.link_nodes(pivot, parent);
                g[blade]
                    .local_transform_mut()
                    .set_position(Vector3::zeros());
                g.link_nodes(blade, pivot);
            }
            update_aya(g, root, 0.0, false);
        }
        if id == 11 {
            let socket: Handle<Node> =
                PivotBuilder::new(BaseBuilder::new().with_name("Slingshot draw"))
                    .build(g)
                    .to_base();
            g.link_nodes(socket, root);
            super::butter_fx::sockets(g, shapes, root);
        }
        return root;
    }
    if id == 0 {
        vivi_gestures(g, root);
        for (index, x) in [-0.16, -0.24, 0.16, 0.24].into_iter().enumerate() {
            let blade: Handle<Node> =
                PivotBuilder::new(BaseBuilder::new().with_name(VIVI_BLADES[index]))
                    .build(g)
                    .to_base();
            g.link_nodes(blade, root);
            g[blade]
                .local_transform_mut()
                .set_position(Vector3::new(x, -0.18, 0.03));
            for (name, position, size, color, angle) in [
                (
                    "Mercury blade",
                    V3::new(0.0, 0.48, 0.0),
                    V3::new(0.09, 0.78, 0.045),
                    [210, 226, 238],
                    -x,
                ),
                (
                    "Blade guard",
                    V3::new(0.0, 0.12, 0.0),
                    V3::new(0.23, 0.05, 0.10),
                    character(id).color,
                    0.0,
                ),
                ("Grip", V3::default(), V3::new(0.07, 0.23, 0.07), INK, 0.0),
            ] {
                let part = shapes.solid(g, name, position, size, color, 0);
                g[part]
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_euler_angles(0.0, 0.0, angle));
                g.link_nodes(part, blade);
            }
        }
        return root;
    }
    let accent = character(id).color;
    let silver = [210, 226, 238];
    let wood = [128, 87, 64];
    let mut part = |name, pos, size, color, shape, angle: f32| {
        let h = shapes.solid(g, name, pos, size, color, shape);
        g[h].local_transform_mut()
            .set_rotation(UnitQuaternion::from_euler_angles(0.0, 0.0, angle));
        g.link_nodes(h, root);
    };
    // Coordinates are relative to the hand. +Z points towards the target.
    match id {
        1 => {
            part(
                "Water tank",
                V3::new(0.0, 0.08, -0.06),
                V3::new(0.13, 0.16, 0.20),
                [81, 198, 239],
                1,
                0.0,
            );
            part(
                "Water pistol",
                V3::new(0.0, -0.04, 0.08),
                V3::new(0.19, 0.16, 0.48),
                accent,
                0,
                0.0,
            );
            part(
                "Nozzle",
                V3::new(0.0, -0.04, 0.38),
                V3::new(0.09, 0.09, 0.18),
                silver,
                0,
                0.0,
            );
            part(
                "Handle",
                V3::new(0.0, -0.18, -0.03),
                V3::new(0.10, 0.21, 0.11),
                CREAM,
                0,
                0.0,
            );
        }
        2 => {
            part(
                "Royal staff",
                V3::new(0.0, 0.0, 0.0),
                V3::new(0.055, 1.0, 0.055),
                [170, 115, 73],
                0,
                -0.12,
            );
            part(
                "Magic globe",
                V3::new(0.05, 0.48, 0.0),
                V3::new(0.17, 0.17, 0.17),
                accent,
                1,
                0.0,
            );
            part(
                "Crown",
                V3::new(0.05, 0.38, 0.0),
                V3::new(0.24, 0.07, 0.18),
                [255, 211, 98],
                0,
                0.0,
            );
        }
        // Id fires from her mount, with no handheld prop. Keep the grip root
        // for her hand pose and muzzle flash; the old box placeholder appeared
        // as a large cube when the death camera switched to her recorded view.
        3 => {}
        4 => {
            part(
                "Flower parasol shaft",
                V3::new(0.0, 0.22, 0.0),
                V3::new(0.035, 1.05, 0.035),
                CREAM,
                0,
                0.0,
            );
            part(
                "Parasol hub",
                V3::new(0.0, 0.68, 0.0),
                V3::new(0.11, 0.08, 0.11),
                silver,
                1,
                0.0,
            );
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                let x = angle.sin() * 0.25;
                let z = angle.cos() * 0.25;
                part(
                    "Parasol petal underside",
                    V3::new(x, 0.69, z),
                    V3::new(0.35, 0.07, 0.35),
                    [170, 184, 209],
                    1,
                    0.0,
                );
                part(
                    "White flower parasol petal",
                    V3::new(x, 0.73, z),
                    V3::new(0.36, 0.08, 0.36),
                    [245, 246, 252],
                    1,
                    0.0,
                );
            }
        }
        5 => {
            part(
                "Sticker wand",
                V3::new(0.0, 0.03, 0.0),
                V3::new(0.06, 0.63, 0.06),
                accent,
                0,
                -0.12,
            );
            part(
                "Sticker book",
                V3::new(0.03, 0.37, 0.0),
                V3::new(0.27, 0.27, 0.055),
                [244, 192, 218],
                0,
                -0.15,
            );
            part(
                "Star sticker",
                V3::new(0.03, 0.37, -0.04),
                V3::new(0.10, 0.10, 0.025),
                [255, 229, 130],
                0,
                0.7,
            );
        }
        6 => {
            for x in [-0.21, 0.21] {
                part(
                    "Sebastian sleeve",
                    V3::new(x, -0.12, 0.0),
                    V3::new(0.23, 0.35, 0.25),
                    accent,
                    0,
                    x,
                );
                part(
                    "Sebastian fist",
                    V3::new(x, 0.06, 0.13),
                    V3::new(0.19, 0.18, 0.22),
                    CREAM,
                    1,
                    0.0,
                );
                for dx in [-0.09, 0.0, 0.09] {
                    part(
                        "Knuckle",
                        V3::new(x + dx, 0.07, 0.29),
                        V3::new(0.055, 0.075, 0.06),
                        silver,
                        1,
                        0.0,
                    );
                }
            }
        }
        7 => {
            part(
                "Arc cannon",
                V3::new(0.0, 0.0, 0.06),
                V3::new(0.30, 0.25, 0.50),
                INK,
                0,
                0.0,
            );
            part(
                "Energy core",
                V3::new(0.0, 0.13, 0.08),
                V3::new(0.11, 0.10, 0.17),
                accent,
                1,
                0.0,
            );
            for x in [-0.12, 0.12] {
                part(
                    "Arc rail",
                    V3::new(x, 0.0, 0.40),
                    V3::new(0.06, 0.12, 0.32),
                    silver,
                    0,
                    0.0,
                );
            }
        }
        8 => {
            part(
                "Needle launcher",
                V3::new(0.0, 0.0, 0.12),
                V3::new(0.10, 0.10, 0.63),
                wood,
                0,
                0.0,
            );
            part(
                "Needle tip",
                V3::new(0.0, 0.0, 0.52),
                V3::new(0.02, 0.02, 0.22),
                silver,
                0,
                0.0,
            );
            part(
                "Poison vial",
                V3::new(0.0, -0.12, -0.05),
                V3::new(0.08, 0.15, 0.08),
                accent,
                1,
                0.0,
            );
            part(
                "Collection ribbon",
                V3::new(0.0, 0.05, 0.0),
                V3::new(0.20, 0.06, 0.05),
                CREAM,
                0,
                0.3,
            );
        }
        9 => {
            part(
                "Lantern flame",
                V3::new(0.0, 0.08, 0.0),
                V3::new(0.13, 0.20, 0.13),
                [177, 124, 251],
                1,
                0.0,
            );
            for y in [-0.12, 0.30] {
                part(
                    "Lantern frame",
                    V3::new(0.0, y, 0.0),
                    V3::new(0.32, 0.045, 0.28),
                    INK,
                    0,
                    0.0,
                );
            }
            for x in [-0.14, 0.14] {
                part(
                    "Lantern pillar",
                    V3::new(x, 0.09, 0.0),
                    V3::new(0.035, 0.40, 0.035),
                    accent,
                    0,
                    0.0,
                );
            }
            part(
                "Lantern handle",
                V3::new(0.0, 0.42, 0.0),
                V3::new(0.05, 0.22, 0.05),
                silver,
                0,
                0.0,
            );
        }
        10 => {
            part(
                "Eru",
                V3::new(0.0, 0.06, 0.0),
                V3::new(0.22, 0.18, 0.21),
                [122, 211, 116],
                1,
                0.0,
            );
            for x in [-0.12, 0.12] {
                part(
                    "Frog eye",
                    V3::new(x, 0.23, -0.04),
                    V3::new(0.085, 0.085, 0.085),
                    CREAM,
                    1,
                    0.0,
                );
                part(
                    "Frog pupil",
                    V3::new(x, 0.23, -0.11),
                    V3::new(0.03, 0.04, 0.025),
                    INK,
                    1,
                    0.0,
                );
            }
            part(
                "Spirit cushion",
                V3::new(0.0, -0.12, 0.0),
                V3::new(0.25, 0.06, 0.24),
                accent,
                1,
                0.0,
            );
        }
        11 => {
            part(
                "Slingshot grip",
                V3::new(0.0, -0.13, 0.0),
                V3::new(0.08, 0.32, 0.07),
                wood,
                0,
                0.0,
            );
            for x in [-1.0, 1.0] {
                part(
                    "Fork",
                    V3::new(x * 0.10, 0.12, 0.0),
                    V3::new(0.06, 0.32, 0.06),
                    wood,
                    0,
                    -x * 0.5,
                );
                part(
                    "Elastic",
                    V3::new(x * 0.08, 0.24, -0.04),
                    V3::new(0.18, 0.02, 0.025),
                    accent,
                    0,
                    0.0,
                );
            }
            part(
                "Stone",
                V3::new(0.0, 0.24, -0.05),
                V3::new(0.07, 0.055, 0.05),
                [155, 163, 166],
                1,
                0.0,
            );
        }
        12 => {
            part(
                "Pillow",
                V3::new(0.0, 0.12, 0.0),
                V3::new(0.38, 0.27, 0.14),
                [238, 218, 248],
                1,
                -0.2,
            );
            for x in [-0.26, 0.26] {
                part(
                    "Pillow corner",
                    V3::new(x, 0.24, 0.0),
                    V3::new(0.19, 0.17, 0.12),
                    CREAM,
                    0,
                    -0.2,
                );
            }
            part(
                "Pillow seam",
                V3::new(0.0, 0.12, -0.13),
                V3::new(0.48, 0.025, 0.02),
                accent,
                0,
                -0.2,
            );
        }
        13 => {
            part(
                "Syringe barrel",
                V3::new(0.0, 0.0, 0.10),
                V3::new(0.11, 0.11, 0.31),
                silver,
                1,
                0.0,
            );
            part(
                "Medicine",
                V3::new(0.0, 0.0, 0.12),
                V3::new(0.115, 0.06, 0.20),
                [110, 231, 180],
                0,
                0.0,
            );
            part(
                "Needle",
                V3::new(0.0, 0.0, 0.52),
                V3::new(0.012, 0.012, 0.27),
                silver,
                0,
                0.0,
            );
            part(
                "Plunger",
                V3::new(0.0, 0.0, -0.27),
                V3::new(0.19, 0.18, 0.04),
                accent,
                0,
                0.0,
            );
            part(
                "Pistol grip",
                V3::new(0.0, -0.17, -0.08),
                V3::new(0.10, 0.25, 0.10),
                CREAM,
                0,
                0.0,
            );
        }
        _ => {
            part(
                "Dark rifle stock",
                V3::new(0.0, -0.03, -0.08),
                V3::new(0.16, 0.19, 0.48),
                accent,
                0,
                0.0,
            );
            part(
                "Rifle barrel",
                V3::new(0.0, 0.0, 0.46),
                V3::new(0.07, 0.07, 0.65),
                INK,
                0,
                0.0,
            );
            part(
                "Scope",
                V3::new(0.0, 0.13, 0.12),
                V3::new(0.055, 0.055, 0.18),
                silver,
                1,
                0.0,
            );
            part(
                "Magic magazine",
                V3::new(0.0, -0.19, 0.12),
                V3::new(0.10, 0.23, 0.16),
                [155, 123, 206],
                0,
                0.0,
            );
        }
    }
    if id == 11 {
        let draw: Handle<Node> = PivotBuilder::new(BaseBuilder::new().with_name("Slingshot draw"))
            .build(g)
            .to_base();
        g.link_nodes(draw, root);
        update_slingshot(g, root, 0.0);
        super::butter_fx::sockets(g, shapes, root);
    }
    root
}

/// Each weapon instance shares the same source clock as its actor, including replay.
pub(super) fn update_aya(g: &mut Graph, weapon: Handle<Node>, phase: f32, enhanced: bool) {
    let state = trickcal_shooting_balddagu::aya_attack::weapon(phase, enhanced);
    let nodes: Vec<_> = g.traverse_handle_iter(weapon).collect();
    for node in nodes {
        if let Some(number) = g[node]
            .name()
            .strip_prefix("Aya_Blade_")
            .and_then(|n| n.parse::<u8>().ok())
        {
            g[node].set_visible(number == state.blade && state.emergence > 0.001);
        }
        if g[node].name().starts_with("Aya_Emerge_") {
            g[node]
                .local_transform_mut()
                .set_scale(Vector3::repeat(state.emergence.max(0.001)));
        }
        if let Some(mesh) = g[node].cast_mut::<fyrox::scene::mesh::Mesh>() {
            for shape in mesh.blend_shapes_mut() {
                if shape.name.ends_with("parasolFold") {
                    shape.weight = state.fold * 100.0;
                }
                if shape.name.ends_with("parasolSpread") {
                    shape.weight = state.spread * 100.0;
                }
            }
        }
    }
}
