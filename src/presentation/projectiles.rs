//! Shared, low-cost projectile meshes. Local +Z is always the leading tip.
use crate::presentation::SceneState;
use super::*;
use fyrox::{
    material::shader::{Shader, ShaderResource},
    scene::mesh::{surface::Surface, Mesh, RenderPath},
};
use std::collections::HashMap;
use trickcal_shooting_balddagu::{combat::Projectile, impact::ImpactKind};

const ENERGY: u8 = 32;
const BULLET: u8 = 33;
const RENUA_ENERGY: u8 = 34;
const BUTTER_BALL: u8 = 35;
pub(super) const SHOT_SECONDS: f32 = 0.09;
const MUZZLE_BLEND_SECONDS: f32 = 0.10;

#[derive(Debug)]
struct Launch {
    offset: Vector3<f32>,
    elapsed: f32,
    scale: f32,
}

#[derive(Clone, Copy)]
pub(super) struct Muzzle {
    pub position: Vector3<f32>,
    pub scale: f32,
}

#[derive(Debug)]
pub(super) struct Models {
    meshes: HashMap<u8, Vec<Surface>>,
    slots: [Option<u8>; MAX_PROJECTILES],
    halos: [Handle<Node>; MAX_PROJECTILES],
    halo_surfaces: [Option<Surface>; 2],
    bullets: [Handle<Node>; PLAYERS],
    lasers: Option<shion_laser::Lasers>,
    paths: [(V3, V3); PLAYERS],
    launches: HashMap<u32, Launch>,
    launch_scene: Option<(u16, MapId, bool)>,
    launch_tick: u64,
}

impl Default for Models {
    fn default() -> Self {
        Self {
            meshes: HashMap::new(),
            slots: [None; MAX_PROJECTILES],
            halos: [Handle::NONE; MAX_PROJECTILES],
            halo_surfaces: [None, None],
            bullets: [Handle::NONE; PLAYERS],
            lasers: None,
            paths: [(V3::default(), V3::default()); PLAYERS],
            launches: HashMap::new(),
            launch_scene: None,
            launch_tick: 0,
        }
    }
}

impl Models {
    /// Run after animated weapons have their final world transforms. Capture
    /// each muzzle once; later recoil/movement must not drag a fired projectile.
    pub(super) fn update_muzzle_launches(
        &mut self,
        g: &mut Graph,
        w: &World,
        nodes: &[Handle<Node>],
        muzzles: &[Muzzle; PLAYERS],
        replay: bool,
        dt: f32,
    ) {
        let scene = (w.round, w.map_id, replay);
        if self.launch_scene != Some(scene) || w.tick < self.launch_tick {
            self.launches.clear();
        }
        self.launch_scene = Some(scene);
        self.launch_tick = w.tick;
        self.launches.retain(|id, _| {
            w.projectiles.iter().any(|p| p.id == *id)
        });
        for (p, &node) in w.projectiles.iter().zip(nodes) {
            let tip = match (p.kind, w.actors[p.owner].character) {
                (ProjectileKind::Needle | ProjectileKind::Poison, 8) => embedded_tip(ImpactKind::PoisonDart),
                (ProjectileKind::Syringe, 13) => embedded_tip(ImpactKind::Syringe),
                (ProjectileKind::MedicalGrenade, 13) => 0.0,
                // Water emerges with its main droplet centered on the opening.
                (ProjectileKind::Water, 1) => 0.015,
                // Magic balls emerge centered on their staff's gem/clover.
                (ProjectileKind::Bolt, 2 | 5 | 10) => 0.0,
                _ => continue,
            };
            // Late join / replay frames must not pull an old shot back to a gun.
            if !self.launches.contains_key(&p.id) && p.age > MUZZLE_BLEND_SECONDS {
                continue;
            }
            let launch = self.launches.entry(p.id).or_insert_with(|| Launch {
                offset: muzzles[p.owner].position
                    - vec(p.velocity.normalized()) * tip * muzzles[p.owner].scale
                    - vec(p.pos),
                elapsed: 0.0,
                scale: muzzles[p.owner].scale,
            });
            let blend = (1.0 - launch.elapsed / MUZZLE_BLEND_SECONDS).clamp(0.0, 1.0);
            g[node].local_transform_mut()
                .set_position(vec(p.pos) + launch.offset * blend)
                .set_scale(Vector3::repeat(1.0 + (launch.scale - 1.0) * blend));
            g.update_hierarchical_data_for_descendants(node);
            // Render the launch at the muzzle for at least its first frame.
            launch.elapsed += dt;
        }
    }

    fn surfaces(&mut self, style: u8) -> Vec<Surface> {
        self.meshes
            .entry(style)
            .or_insert_with(|| build(style))
            .clone()
    }

    fn spawn(&mut self, g: &mut Graph, style: u8, name: &str) -> Handle<Node> {
        MeshBuilder::new(BaseBuilder::new().with_name(name).with_cast_shadows(false))
            .with_surfaces(self.surfaces(style))
            .build(g)
            .to_base()
    }

    pub(super) fn create_bullets(&mut self, g: &mut Graph) {
        self.lasers = Some(shion_laser::Lasers::new(g));
        for i in 0..PLAYERS {
            self.bullets[i] = self.spawn(g, BULLET, "Sion bullet");
            g[self.bullets[i]].set_visible(false);
        }
    }

    pub(super) fn shot(&mut self, slot: usize, origin: V3, end: V3, laser: bool) {
        self.paths[slot] = (origin, end);
        if let Some(lasers) = &mut self.lasers {
            lasers.shot(slot, origin, end, laser);
        }
    }

    pub(super) fn update_shion_laser(&mut self, g: &mut Graph, slot: usize, muzzle: Vector3<f32>, camera: Vector3<f32>) {
        if let Some(lasers) = &mut self.lasers {
            lasers.pose(g, slot, muzzle, camera);
        }
    }

    pub(super) fn verify_shion_shot(&self, g: &Graph, slot: usize, laser: bool, bullet: bool) {
        assert_eq!(g[self.bullets[slot]].visibility(), bullet);
        self.lasers.as_ref().unwrap().verify(g, slot, laser);
    }

    pub(super) fn update_bullet(&self, g: &mut Graph, slot: usize, remaining: f32, visible: bool) {
        let node = self.bullets[slot];
        let (origin, end) = self.paths[slot];
        let delta = end - origin;
        let length = delta.length();
        let laser = self.lasers.as_ref().is_some_and(|lasers| {
            lasers.update(g, slot, remaining, visible)
        });
        g[node].set_visible(visible && !laser && remaining > 0.0 && length > 0.01);
        if length <= 0.01 {
            return;
        }
        let phase = (1.0 - remaining / SHOT_SECONDS).clamp(0.0, 1.0);
        // Keep the nose within the confirmed ray segment, even at point blank range.
        let scale = (length / 0.40).min(1.0);
        let travel = (length - 0.40 * scale).max(0.0) * phase;
        g[node]
            .local_transform_mut()
            .set_position(vec(origin + delta.normalized() * (0.16 * scale + travel)))
            .set_scale(Vector3::repeat(scale))
            .set_rotation(facing(delta));
    }

    pub(super) fn update(
        &mut self,
        g: &mut Graph,
        node: Handle<Node>,
        slot: usize,
        projectile: &Projectile,
        character: u8,
    ) {
        let style = if character == 5 && matches!(projectile.kind, ProjectileKind::Bolt | ProjectileKind::Ricochet) { RENUA_ENERGY } else if projectile.kind == ProjectileKind::Ricochet && character == 11 { BUTTER_BALL } else if projectile.kind == ProjectileKind::RenuaOrb { RENUA_ENERGY } else if matches!(projectile.kind, ProjectileKind::Bolt | ProjectileKind::MagicBomb) && character == 2 {
            ENERGY
        } else {
            projectile.kind as u8
        };
        let style_changed = self.slots[slot] != Some(style);
        if style_changed {
            g[node]
                .cast_mut::<Mesh>()
                .unwrap()
                .set_surfaces(self.surfaces(style));
            g[node].set_cast_shadows(false);
            self.slots[slot] = Some(style);
        }
        if matches!(style, ENERGY | RENUA_ENERGY) {
            let surface = self.halo_surfaces[usize::from(style == RENUA_ENERGY)].get_or_insert_with(|| {
                let shader = Shader::from_string(if style == RENUA_ENERGY { include_str!("renua_orb.shader") } else { include_str!("projectile_glow.shader") })
                    .expect("energy halo shader");
                SurfaceBuilder::new(SurfaceResource::new_embedded(SurfaceData::make_sphere(
                    24,
                    24,
                    0.235,
                    &Matrix4::identity(),
                )))
                .with_material(MaterialResource::new(Material::from_shader(
                    ShaderResource::new_embedded(shader),
                )))
                .build()
            });
            if self.halos[slot].is_none() {
            let halo = MeshBuilder::new(
                BaseBuilder::new()
                    .with_name("Energy ball glow")
                    .with_cast_shadows(false),
            )
            .with_render_path(RenderPath::Forward)
            .with_surfaces(vec![surface.clone()])
            .build(g)
            .to_base();
            g.link_nodes(halo, node);
            self.halos[slot] = halo;
            } else if style_changed { g[self.halos[slot]].cast_mut::<Mesh>().unwrap().set_surfaces(vec![surface.clone()]); }
        }
        if self.halos[slot].is_some() {
            g[self.halos[slot]].set_visible(matches!(style, ENERGY | RENUA_ENERGY));
        }
        g[node]
            .local_transform_mut()
            .set_scale(Vector3::repeat(if projectile.kind==ProjectileKind::BoulderBlast {
                let t=(projectile.age/trickcal_shooting_balddagu::butter_skill::BLAST_SECONDS).clamp(0.0,1.0);
                (0.3+4.2*t)*(1.0-trickcal_shooting_balddagu::butter_skill::smooth((t-0.75)/0.25))
            } else if matches!(projectile.kind, ProjectileKind::ErpinBlast | ProjectileKind::ErpinCrash) {
                let t=(projectile.age/trickcal_shooting_balddagu::erpin_skill::BLAST_SECONDS).clamp(0.0,1.0);
                (0.15+2.0*t)*(1.0-trickcal_shooting_balddagu::erpin::smooth((t-0.6)/0.4))
            } else if projectile.kind == ProjectileKind::MagicBomb {1.65} else {1.0}));
    }

    pub(super) fn embedded(&mut self, g: &mut Graph, kind: ImpactKind) -> Option<Handle<Node>> {
        let style = match kind {
            ImpactKind::PoisonDart => ProjectileKind::Needle as u8,
            ImpactKind::Syringe => ProjectileKind::Syringe as u8,
            _ => return None,
        };
        Some(self.spawn(g, style, "Embedded projectile"))
    }
}

#[cfg(test)]
mod launch_tests {
    use super::*;

    #[test]
    fn mayo_needles_and_poison_leave_the_gun_once_in_both_views() {
        for kind in [ProjectileKind::Needle, ProjectileKind::Poison] {
            for scale in [weapons::first_person_scale(8) / weapons::world_scale(8),
                mayo_fx::VIEW_SCALE / mayo_fx::SCALE, 1.0] {
                check_launch(8, kind, embedded_tip(ImpactKind::PoisonDart), scale);
            }
        }
    }

    #[test]
    fn syringe_starts_at_muzzle_then_leaves_the_moving_gun_and_joins_its_path() {
        check_launch(13, ProjectileKind::Syringe, embedded_tip(ImpactKind::Syringe),
            weapons::first_person_scale(13) / weapons::world_scale(13));
    }

    #[test]
    fn water_starts_at_muzzle_then_leaves_the_moving_gun_and_joins_its_path() {
        for scale in [0.24 * weapons::first_person_scale(1), 0.24 * weapons::world_scale(1)] {
            check_launch(1, ProjectileKind::Water, 0.015, scale);
        }
    }

    #[test]
    fn ui_bolt_leaves_the_clover_and_does_not_follow_later_staff_motion() {
        for scale in [weapons::first_person_scale(10) / weapons::world_scale(10), 1.0] {
            check_launch(10, ProjectileKind::Bolt, 0.0, scale);
        }
    }

    #[test]
    fn medical_grenade_leaves_the_launcher_in_both_views_and_joins_its_arc() {
        for scale in [hilde_fx::VIEW_SCALE / hilde_fx::SCALE, 1.0] {
            check_launch(13, ProjectileKind::MedicalGrenade, 0.0, scale);
        }
    }

    #[test]
    fn erpin_bolt_leaves_the_gem_and_does_not_follow_later_staff_motion() {
        check_launch(2, ProjectileKind::Bolt, 0.0, 1.0);
    }

    #[test]
    fn picora_bolt_leaves_the_staff_and_does_not_follow_later_staff_motion() {
        check_launch(5, ProjectileKind::Bolt, 0.0, 1.0);
    }

    fn check_launch(hero: u8, kind: ProjectileKind, tip: f32, launch_scale: f32) {
        let mut g = Graph::new();
        let node = PivotBuilder::new(BaseBuilder::new()).build(&mut g).to_base();
        let halo: Handle<Node> = PivotBuilder::new(BaseBuilder::new()).build(&mut g).to_base();
        g.link_nodes(halo, node);
        let mut models = Models::default();
        let mut w = World::default();
        w.actors[0].character = hero;
        let mut p = Projectile::visual(1, 0, kind, V3::new(0.0, 1.6, 0.75));
        p.velocity = V3::new(0.0, 0.0, 45.0);
        p.age = DT;
        w.projectiles.push(p.clone());
        let muzzle = Vector3::new(-0.4, 1.4, 1.0);
        let mut muzzles = [Muzzle { position: muzzle, scale: launch_scale }; PLAYERS];
        models.update_muzzle_launches(&mut g, &w, &[node], &muzzles, false, MUZZLE_BLEND_SECONDS / 2.0);
        let start = **g[node].local_transform().position();
        assert!((start + Vector3::z() * tip * launch_scale - muzzle).norm() < 0.0001);
        // Rendering reads cached world transforms, including the energy halo.
        assert!((g[node].global_position() - start).norm() < 0.0001);
        assert!((g[halo].global_position() - start).norm() < 0.0001);
        assert!((**g[node].local_transform().scale() - Vector3::repeat(launch_scale)).norm() < 0.0001);

        muzzles[0].position += Vector3::new(20.0, 5.0, 0.0);
        w.projectiles[0].pos.z += 2.25;
        w.projectiles[0].age += 0.05;
        models.update_muzzle_launches(&mut g, &w, &[node], &muzzles, false, MUZZLE_BLEND_SECONDS / 2.0);
        assert!((**g[node].local_transform().position()
            - (vec(w.projectiles[0].pos) + (start - vec(p.pos)) * 0.5)).norm() < 0.0001);
        models.update_muzzle_launches(&mut g, &w, &[node], &muzzles, false, DT);
        assert!((**g[node].local_transform().position() - vec(w.projectiles[0].pos)).norm() < 0.0001);
        assert_eq!(**g[node].local_transform().scale(), Vector3::repeat(1.0));

        w.projectiles.clear();
        models.update_muzzle_launches(&mut g, &w, &[node], &muzzles, false, DT);
        assert!(models.launches.is_empty(), "collision/expiry releases the launch state");
        p.id = 2;
        p.age = 0.5;
        w.projectiles.push(p.clone());
        g[node].local_transform_mut().set_position(vec(p.pos));
        models.update_muzzle_launches(&mut g, &w, &[node], &muzzles, true, DT);
        assert!((**g[node].local_transform().position() - vec(p.pos)).norm() < 0.0001);
        assert!(models.launches.is_empty(), "old snapshots must not launch from the gun again");
    }
}

pub(super) fn facing(direction: V3) -> UnitQuaternion<f32> {
    UnitQuaternion::rotation_between(&Vector3::z(), &vec(direction).normalize()).unwrap_or_default()
}

pub(super) fn embedded_tip(kind: ImpactKind) -> f32 {
    match kind {
        ImpactKind::PoisonDart => 0.34,
        ImpactKind::Syringe => 0.39,
        _ => 0.0,
    }
}

fn part(shape: u8, pos: [f32; 3], size: [f32; 3], color: [u8; 3], glow: f32) -> Surface {
    let rotation = if matches!(shape, 2 | 3) {
        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), std::f32::consts::FRAC_PI_2)
    } else {
        UnitQuaternion::identity()
    };
    let transform = Matrix4::new_translation(&Vector3::from(pos))
        * rotation.to_homogeneous()
        * Matrix4::new_nonuniform_scaling(&Vector3::from(size));
    let geometry = match shape {
        0 => SurfaceData::make_cube(transform),
        // Fyrox 1.0.1's sphere builder needs equal slice/stack counts to close both poles.
        1 => SurfaceData::make_sphere(16, 16, 1.0, &transform),
        2 => SurfaceData::make_cylinder(12, 1.0, 1.0, true, &transform),
        3 => SurfaceData::make_cone(12, 1.0, 1.0, &transform),
        // A small, flat-shaded, asymmetrical river pebble.
        _ => {
            let mut data = SurfaceData::make_sphere(9, 9, 1.0, &transform);
            data.calculate_normals().unwrap();
            data
        }
    };
    let mat = material(color);
    mat.data_ref()
        .set_property("roughness", if shape == 4 { 0.96_f32 } else { 0.30_f32 });
    if glow > 0.0 {
        // Standard materials default to a black emission texture; strength alone does not glow.
        let white = TextureResource::new_embedded(
            Texture::from_bytes(
                TextureKind::Rectangle {
                    width: 1,
                    height: 1,
                },
                TexturePixelKind::RGBA8,
                vec![255; 4],
            )
            .unwrap(),
        );
        mat.data_ref().bind("emissionTexture", white);
        mat.data_ref().set_property(
            "emissionStrength",
            vec(V3::new(
                color[0] as f32 / 255.0 * glow,
                color[1] as f32 / 255.0 * glow,
                color[2] as f32 / 255.0 * glow,
            )),
        );
    }
    SurfaceBuilder::new(SurfaceResource::new_embedded(geometry))
        .with_material(mat)
        .build()
}

pub(super) fn boulder_surfaces() -> Vec<Surface> {
    vec![
        part(4, [0.0; 3], [0.261, 0.234, 0.270], [142, 128, 108], 0.0),
        part(4, [0.126, -0.054, 0.054], [0.149, 0.162, 0.162], [160, 146, 121], 0.0),
        part(4, [-0.099, 0.108, -0.0675], [0.140, 0.131, 0.144], [181, 166, 139], 0.0),
    ]
}
fn build(style: u8) -> Vec<Surface> {
    if style == ProjectileKind::Boulder as u8 { return boulder_surfaces(); }
    let mut parts = Vec::new();
    let mut add = |shape, pos, size, color, glow| parts.push(part(shape, pos, size, color, glow));
    match style {
        s if s == ProjectileKind::Water as u8 => {
            add(
                1,
                [0.0, 0.0, 0.015],
                [0.15, 0.15, 0.18],
                [53, 180, 242],
                0.12,
            );
            add(
                1,
                [0.0, 0.0, -0.13],
                [0.065, 0.065, 0.09],
                [81, 209, 255],
                0.1,
            );
            add(
                3,
                [0.0, 0.0, -0.24],
                [0.055, 0.07, 0.055],
                [82, 205, 246],
                0.0,
            );
            add(
                1,
                [-0.085, 0.095, 0.075],
                [0.042, 0.035, 0.065],
                [222, 253, 255],
                0.3,
            );
            add(
                1,
                [0.08, 0.07, 0.12],
                [0.022, 0.026, 0.022],
                [181, 243, 255],
                0.2,
            );
        }
        s if s == ProjectileKind::Stone as u8 || s == BUTTER_BALL => {
            add(1, [0.0;3], [0.15;3], [255,215,24], 0.08);
            add(1, [-0.055,0.065,0.10], [0.045,0.035,0.025], [255,250,179], 0.12);
        }
        s if s == ProjectileKind::BoulderBlast as u8 => {
            for i in 0..18 {
                let angle=i as f32*std::f32::consts::TAU/18.0;
                let radius=if i%2==0 {1.0}else{0.65};
                add(4,[angle.cos()*radius, (i%3) as f32*0.20-0.15,angle.sin()*radius],
                    [0.10,0.13,0.09],[160,138,100],0.0);
                add(1,[angle.cos()*0.85,0.0,angle.sin()*0.85],[0.24,0.045,0.24],[255,216,112],0.5);
            }
        }
        s if s == ProjectileKind::ErpinBlast as u8 || s == ProjectileKind::ErpinCrash as u8 => {
            let yellow=s==ProjectileKind::ErpinCrash as u8;
            let color=if yellow {[255,222,24]}else{[202,124,255]};
            add(1,[0.0;3],[0.48;3],if yellow {[255,249,171]}else{[255,221,255]},2.8);
            for i in 0..12 {
                let angle=i as f32*std::f32::consts::TAU/12.0;
                add(1,[angle.cos()*0.85,(i%3) as f32*0.22-0.22,angle.sin()*0.85],[0.24,0.12,0.24],color,2.0);
                add(1,[angle.cos()*1.3,angle.sin()*0.8,angle.sin()*1.3],[0.06,0.13,0.06],color,2.5);
            }
        }
        ENERGY | RENUA_ENERGY => {
            add(
                1,
                [0.0, 0.0, 0.0],
                [0.145, 0.145, 0.145],
                if style == RENUA_ENERGY { [30, 145, 255] } else { [163, 111, 255] },
                1.8,
            );
            add(
                1,
                [-0.07, 0.075, 0.082],
                [0.067, 0.066, 0.065],
                if style == RENUA_ENERGY { [200, 246, 255] } else { [255, 239, 255] },
                2.5,
            );
            for i in 0..6 {
                let angle = i as f32 * std::f32::consts::TAU / 6.0;
                add(
                    1,
                    [angle.cos() * 0.19, angle.sin() * 0.19, 0.0],
                    [0.021, 0.021, 0.035],
                    if style == RENUA_ENERGY { [72, 215, 255] } else { [255, 218, 146] },
                    2.5,
                );
            }
        }
        s if s == ProjectileKind::Needle as u8 || s == ProjectileKind::Poison as u8 => {
            add(
                2,
                [0.0, 0.0, -0.22],
                [0.022, 0.43, 0.022],
                [79, 64, 104],
                0.0,
            );
            add(
                3,
                [0.0, 0.0, 0.16],
                [0.035, 0.18, 0.035],
                [173, 237, 73],
                0.25,
            );
            add(
                0,
                [0.0, 0.0, -0.19],
                [0.15, 0.018, 0.12],
                [144, 82, 170],
                0.0,
            );
            add(
                0,
                [0.0, 0.0, -0.19],
                [0.018, 0.15, 0.12],
                [144, 82, 170],
                0.0,
            );
            add(
                2,
                [0.0, 0.0, 0.11],
                [0.038, 0.05, 0.038],
                [193, 249, 110],
                0.1,
            );
        }
        s if s == ProjectileKind::MedicalGrenade as u8 => {
            add(2,[0.0,0.0,0.0],[0.11,0.24,0.11],[170,220,65],0.3);
            add(2,[0.0,0.0,-0.14],[0.12,0.04,0.12],[108,88,55],0.0);
            add(1,[0.0,0.0,0.12],[0.11,0.11,0.09],[245,231,196],0.0);
        }
        s if s == ProjectileKind::Syringe as u8 => {
            add(
                2,
                [0.0, 0.0, -0.12],
                [0.06, 0.25, 0.06],
                [197, 237, 241],
                0.0,
            );
            add(
                2,
                [0.0, 0.0, -0.09],
                [0.061, 0.14, 0.061],
                [83, 213, 163],
                0.1,
            );
            add(
                2,
                [0.0, 0.0, 0.12],
                [0.038, 0.055, 0.038],
                [84, 153, 183],
                0.0,
            );
            add(
                2,
                [0.0, 0.0, 0.17],
                [0.009, 0.19, 0.009],
                [213, 223, 235],
                0.15,
            );
            add(
                3,
                [0.0, 0.0, 0.36],
                [0.009, 0.03, 0.009],
                [245, 250, 255],
                0.1,
            );
            add(
                0,
                [0.0, 0.0, -0.13],
                [0.19, 0.035, 0.035],
                [236, 244, 250],
                0.0,
            );
            add(
                2,
                [0.0, 0.0, -0.27],
                [0.022, 0.15, 0.022],
                [220, 229, 240],
                0.0,
            );
            add(
                2,
                [0.0, 0.0, -0.30],
                [0.066, 0.03, 0.066],
                [93, 162, 191],
                0.0,
            );
            for z in [-0.06, -0.015, 0.03, 0.075] {
                add(
                    0,
                    [-0.058, 0.0, z],
                    [0.009, 0.04, 0.009],
                    [43, 95, 111],
                    0.0,
                );
            }
        }
        BULLET => {
            add(
                2,
                [0.0, 0.0, -0.16],
                [0.047, 0.23, 0.047],
                [196, 128, 61],
                0.05,
            );
            add(
                3,
                [0.0, 0.0, 0.07],
                [0.047, 0.17, 0.047],
                [235, 183, 111],
                0.15,
            );
            add(
                2,
                [0.0, 0.0, -0.14],
                [0.049, 0.025, 0.049],
                [113, 76, 48],
                0.0,
            );
        }
        _ => {
            let kind = ProjectileKind::from_wire(style).unwrap_or_default();
            let (color, size) = projectile_look(kind);
            add(1, [0.0; 3], [size.x, size.y, size.z], color, 0.0);
        }
    }
    parts
}
