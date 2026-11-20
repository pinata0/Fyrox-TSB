//! Deterministic facial channels shared by live play, selection and replay.
use super::*;
use fyrox::scene::mesh::Mesh;
use fyrox::{
    asset::manager::ResourceManager,
    core::futures::{executor::block_on, future::join_all},
};
use trickcal_shooting_balddagu::motion::Motion;

static CLOSED: std::sync::OnceLock<Vec<TextureResource>> = std::sync::OnceLock::new();
static HALF_CLOSED: std::sync::OnceLock<Vec<TextureResource>> = std::sync::OnceLock::new();
static NAIA_ATTACK: std::sync::OnceLock<Vec<TextureResource>> = std::sync::OnceLock::new();

pub(super) fn load(manager: &ResourceManager) -> Result<(), String> {
    super::cheek_eyes::load(manager);
    let requests: Vec<_> = characters::SLUGS
        .iter()
        .map(|slug| {
            manager.request::<Texture>(characters::asset_path(&format!("{slug}-blink-closed.png")))
        })
        .collect();
    let mut textures = Vec::new();
    for (slug, result) in characters::SLUGS.iter().zip(block_on(join_all(requests))) {
        textures.push(result.map_err(|error| format!("{slug} closed eyelid texture: {error:?}"))?);
    }
    let _ = CLOSED.set(textures);
    let requests = characters::SLUGS.iter().map(|slug| {
        manager.request::<Texture>(characters::asset_path(&format!("{slug}-blink-half.png")))
    });
    let mut textures = Vec::new();
    for (slug, result) in characters::SLUGS.iter().zip(block_on(join_all(requests))) {
        textures.push(result.map_err(|error| format!("{slug} halfway eyelid texture: {error:?}"))?);
    }
    let _ = HALF_CLOSED.set(textures);
    let names = ["wink", "smile", "squeeze", "recover"];
    let requests = names.map(|name| {
        manager.request::<Texture>(characters::asset_path(&format!("naia-attack-{name}.png")))
    });
    let mut textures = Vec::new();
    for (name, result) in names.into_iter().zip(block_on(join_all(requests))) {
        textures.push(result.map_err(|error| format!("Naia {name} face: {error:?}"))?);
    }
    let _ = NAIA_ATTACK.set(textures);
    Ok(())
}

#[derive(Debug, Default)]
pub(super) struct Face {
    surfaces: Vec<(Handle<Node>, usize, Vec<MaterialResource>)>,
}

impl Face {
    pub(super) fn new(g: &Graph, root: Handle<Node>, id: u8) -> Self {
        let mut face = Self::default();
        let Some(textures) = CLOSED.get() else {
            return face;
        };
        for node in g.traverse_handle_iter(root) {
            if !g[node].name().ends_with("_SkinnedMesh") {
                continue;
            }
            let Some(mesh) = g[node].cast::<Mesh>() else {
                continue;
            };
            for (index, surface) in mesh.surfaces().iter().enumerate() {
                let original = surface.material();
                let material = original.data_ref();
                let mut variants = vec![original.clone()];
                let mut replacements = vec![textures[id as usize].clone()];
                if id == 1 {
                    replacements.extend(
                        NAIA_ATTACK
                            .get()
                            .expect("loaded Naia faces")
                            .iter()
                            .cloned(),
                    );
                }
                replacements
                    .push(HALF_CLOSED.get().expect("loaded halfway lids")[id as usize].clone());
                for texture in replacements {
                    let mut variant = Material::from_shader(material.shader().clone());
                    for (name, binding) in material.bindings() {
                        variant.bind(name.clone(), binding.clone());
                    }
                    variant.bind("diffuseTexture", texture);
                    variants.push(MaterialResource::new(variant));
                }
                face.surfaces.push((node, index, variants));
            }
        }
        face
    }

    pub(super) fn show(&self, g: &mut Graph, closed: bool) {
        self.show_texture(g, usize::from(closed));
    }

    pub(super) fn show_motion(&self, g: &mut Graph, motion: Motion, age: f32, id: u8) {
        let texture = if id == 1 && motion == Motion::Attack {
            trickcal_shooting_balddagu::naia_attack::face(age / motion.seconds(id)).texture
        } else {
            blink_texture(weights(motion, age, id)[0], id)
        };
        self.show_texture(g, texture);
    }

    fn show_texture(&self, g: &mut Graph, texture: usize) {
        for (node, index, variants) in &self.surfaces {
            if let Some(mesh) = g[*node].cast_mut::<Mesh>() {
                mesh.surfaces_mut()[*index].set_material(variants[texture].clone());
            }
        }
    }

    pub(super) fn show_blink(&self, g: &mut Graph, closure: f32, id: u8) {
        self.show_texture(g, blink_texture(closure, id));
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn blink_texture(closure: f32, id: u8) -> usize {
    if closure > 0.80 {
        1
    } else if closure > 0.20 {
        if id == 1 {
            6
        } else {
            2
        }
    } else {
        0
    }
}

fn weights(motion: Motion, age: f32, id: u8) -> [f32; 5] {
    if id == 3 {
        return [
            trickcal_shooting_balddagu::id_skill::eye_closure(motion, age),
            0.0,
            0.0,
            0.0,
            0.0,
        ];
    }
    let time = age.max(0.0);
    let blink_time = (time + id as f32 * 0.193).rem_euclid(3.7);
    let blink = smooth((blink_time - 3.40) / 0.10) * (1.0 - smooth((blink_time - 3.52) / 0.14));
    let phase = (time / motion.seconds(id)).clamp(0.0, 1.0);
    let accent = (phase * std::f32::consts::PI).sin().powi(2);
    match motion {
        Motion::ErpinSkill | Motion::ErpinStart | Motion::ErpinCharge | Motion::ErpinRecover => {
            let f=trickcal_shooting_balddagu::erpin_skill::flustered(motion,age);
            [blink*(1.0-f),0.0,0.70*f,0.0,0.60*f]
        }
        Motion::KomiRest => [smooth((age-0.10)/0.12)*(1.0-smooth((age-5.55)/0.35)),0.2,0.0,0.0,0.0],
        Motion::KomiFeast => [blink,0.3,0.0,0.0,if (1.7..2.5).contains(&age){0.35}else{0.0}],
        Motion::Death => [smooth(phase / 0.45), 0.0, 0.32, 0.0, 0.0],
        Motion::Hurt => [blink, 0.0, 0.65 * accent, 0.35 * accent, 0.22 * accent],
        Motion::Attack | Motion::Attack2 | Motion::Skill | Motion::Ultimate => {
            [blink, 0.0, 0.0, 0.28 * accent, 0.12 * accent]
        }
        Motion::Spawn => [blink, 0.55 * accent, 0.0, 0.0, 0.0],
        Motion::Dance => [blink, 0.35, 0.0, 0.0, 0.0],
        _ => [blink, 0.0, 0.0, 0.0, 0.0],
    }
}

pub(super) fn apply(g: &mut Graph, root: Handle<Node>, motion: Motion, age: f32, id: u8) {
    let [blink, smile, frown, squint, jaw] = weights(motion, age, id);
    let naia = (id == 1 && motion == Motion::Attack)
        .then(|| trickcal_shooting_balddagu::naia_attack::face(age / motion.seconds(id)));
    let nodes: Vec<_> = g.traverse_handle_iter(root).collect();
    for node in nodes {
        if let Some(mesh) = g[node].cast_mut::<Mesh>() {
            for shape in mesh.blend_shapes_mut() {
                let name = shape.name.rsplit('.').next().unwrap_or(&shape.name);
                shape.weight = 100.0
                    * if let Some(face) = naia {
                        if name.contains("eyeBlink_") {
                            face.blink[usize::from(name.ends_with("_R"))]
                        } else if name.contains("browDown_") {
                            face.blink[usize::from(name.ends_with("_R"))] * 0.75
                        } else if name.contains("mouthSmile_") {
                            face.smile
                        } else if name.ends_with("jawOpen") {
                            face.jaw
                        } else {
                            0.0
                        }
                    } else if id == 2 && name.contains("browUp_") {
                        trickcal_shooting_balddagu::erpin_skill::flustered(motion,age) * if name.ends_with("_L") {0.95}else{0.72}
                    } else if id == 2 && name.contains("eyeWide_") {
                        trickcal_shooting_balddagu::erpin_skill::flustered(motion,age) * 0.90
                    } else if name.contains("eyeBlink_") {
                        blink
                    } else if name.contains("browDown_") {
                        blink * 0.75
                    } else if name.contains("mouthSmile_") {
                        smile
                    } else if name.contains("mouthFrown_") {
                        frown
                    } else if name.contains("eyeSquint_") {
                        squint
                    } else if name.ends_with("jawOpen") {
                        jaw
                    } else {
                        0.0
                    };
            }
        }
    }
}

pub(super) fn review(g: &mut Graph, root: Handle<Node>, closed: bool, id: u8) {
    review_blink(g, root, if closed { 1.0 } else { 0.0 }, id);
}

pub(super) fn review_blink(g: &mut Graph, root: Handle<Node>, closure: f32, _id: u8) {
    let nodes: Vec<_> = g.traverse_handle_iter(root).collect();
    let mut channels = 0;
    let mut lids = 0;
    for node in nodes {
        if let Some(mesh) = g[node].cast_mut::<Mesh>() {
            for shape in mesh.blend_shapes_mut() {
                channels += 1;
                let blink = shape.name.contains("eyeBlink_");
                lids += usize::from(blink);
                shape.weight = if blink {
                    100.0 * closure
                } else if shape.name.contains("browDown_") {
                    75.0 * closure
                } else {
                    0.0
                };
            }
        }
    }
    assert_eq!(
        channels, 23,
        "all facial morphs must survive the Fyrox FBX importer"
    );
    assert_eq!(lids, 2, "left/right eyelid controls");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facial_channels_are_finite_bounded_and_blink_returns_to_neutral() {
        for id in 0..15 {
            for motion in [
                Motion::Idle,
                Motion::Attack,
                Motion::Spawn,
                Motion::Hurt,
                Motion::Death,
                Motion::Dance,
            ] {
                for frame in 0..600 {
                    assert!(weights(motion, frame as f32 / 60.0, id)
                        .iter()
                        .all(|w| w.is_finite() && (0.0..=1.0).contains(w)));
                }
            }
        }
        assert_eq!(weights(Motion::Idle, 3.0, 0)[0], 0.0);
        assert!(weights(Motion::Idle, 3.51, 0)[0] > 0.99);
        assert_eq!(weights(Motion::Idle, 3.69, 0)[0], 0.0);
        assert_eq!(weights(Motion::Death, 10.0, 0)[0], 1.0);
    }
}
