//! Close views of the exact meshes used by live shots and embedded contacts.
use super::*;
use trickcal_shooting_balddagu::combat::Projectile;

const HEROES: [(u8, ProjectileKind); 6] = [
    (1, ProjectileKind::Water),
    (11, ProjectileKind::Stone),
    (2, ProjectileKind::Bolt),
    (8, ProjectileKind::Needle),
    (13, ProjectileKind::Syringe),
    (14, ProjectileKind::Bolt),
];

fn muzzle_hero() -> Option<u8> {
    if std::env::args().any(|arg| arg == "--naia-muzzle-review") { Some(1) }
    else if std::env::args().any(|arg| arg == "--erpin-muzzle-review") { Some(2) }
    else if std::env::args().any(|arg| arg == "--picora-muzzle-review") { Some(5) }
    else if std::env::args().any(|arg| arg == "--hilde-muzzle-review") { Some(13) }
    else { None }
}

pub(super) fn enabled() -> bool {
    muzzle_hero().is_some() || ui_attack_enabled()
        || std::env::args().any(|arg| arg == "--projectile-review")
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if ui_attack_enabled() {
        return update_ui_attack(rt, context);
    }
    if muzzle_hero().is_some() {
        return update_muzzle(rt, context);
    }
    let Some(&(hero, kind)) = HEROES.get(rt.review_capture as usize) else {
        println!(
            "PROJECTILE_REVIEW_OK: water bomb, pebble, energy ball, poison dart, syringe, bullet"
        );
        context.loop_controller.exit();
        return Ok(());
    };
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        w.projectiles.clear();
        w.impacts.clear();
        for (i, actor) in w.actors.iter_mut().enumerate() {
            actor.active = i == 0;
        }
        w.actors[0].character = hero;
        w.actors[0].pos = V3::new(0.0, 0.0, -12.0);
        w.actors[0].hp = MAX_HP;
        w.actors[0].combat = Default::default();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    g[rt.weapon].set_visibility(false);
    g[rt.wave_hand].set_visibility(false);
    let center = V3::new(0.0, 2.5, -10.0);
    let direction = V3::new(-0.85, 0.18, 0.45).normalized();
    g[rt.camera]
        .local_transform_mut()
        .set_position(vec(center - V3::new(0.0, 0.0, 1.05)))
        .set_rotation(UnitQuaternion::identity());
    if hero == 14 {
        rt.projectile_models
            .shot(0, center - direction * 0.16, center + direction * 2.0, false);
        rt.projectile_models
            .update_bullet(g, 0, projectiles::SHOT_SECONDS, true);
    } else {
        let p = Projectile::visual(1, 0, kind, center);
        let node = rt.projectiles[0];
        rt.projectile_models.update(g, node, 0, &p, hero);
        g[node].set_visibility(true);
        g[node]
            .local_transform_mut()
            .set_position(vec(center))
            .set_rotation(projectiles::facing(direction));
    }
    let ui = context.user_interfaces.first_mut();
    ui.send(ui.root(), WidgetMessage::Visibility(false));
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if ui_attack_enabled() {
        return capture_ui_attack(rt, context);
    }
    if let Some(hero) = muzzle_hero() {
        let g = &context.scenes[rt.scene].graph;
        let muzzle = match (hero, rt.review_capture == 0) {
            (1, true) => g[rt.weapons[1]].global_transform()
                .transform_point(&naia_whales::MUZZLE.into()).coords,
            (1, false) => rt.visuals[0].water_muzzle(g),
            (2, true) => weapons::erpin_muzzle(g, rt.weapons[2]),
            (2, false) => rt.visuals[0].staff_muzzle(g),
            (5, true) => weapons::picora_muzzle(g, rt.weapons[5]),
            (5, false) => weapons::picora_muzzle(g, rt.visuals[0].held_weapon()),
            (_, true) => weapons::hilde_muzzle(g, rt.weapons[13]),
            (_, false) => rt.visuals[0].syringe_muzzle(g),
        };
        let node = &g[rt.projectiles[0]];
        let lead = match hero {
            1 => 0.015,
            2 | 5 => 0.0,
            _ => projectiles::embedded_tip(trickcal_shooting_balddagu::impact::ImpactKind::Syringe),
        };
        let tip = node.global_transform().transform_point(&(Vector3::z() * lead).into()).coords;
        assert!((tip - muzzle).norm() < 0.002, "projectile misses the animated muzzle: {}", (tip - muzzle).norm());
        let view = if rt.review_capture == 0 { "fp" } else { "world" };
        capture_preview(rt.scene, hero, &format!("{}-muzzle-{view}-", characters::SLUGS[hero as usize]), context).map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
        rt.review_capture += 1;
        rt.review_frames = 0;
        return Ok(());
    }
    let hero = HEROES[rt.review_capture as usize].0;
    capture_preview(
        rt.scene,
        1,
        &format!("projectile-{}-", characters::SLUGS[hero as usize]),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}

fn ui_attack_enabled() -> bool {
    std::env::args().any(|arg| arg == "--ui-attack-review")
}

// Idle, launch, flight, recovery, enhanced ray, external launch.
const UI_ATTACK_SAMPLES: [(&str, f32, bool, bool); 6] = [
    ("idle", 100.0, false, true),
    ("launch", 0.0, false, true),
    ("flight", 0.15, false, true),
    ("recovery", 0.30, false, true),
    ("enhanced", 0.0, true, true),
    ("world", 0.0, false, false),
];

fn update_ui_attack(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let Some(&(_, age, enhanced, first)) = UI_ATTACK_SAMPLES.get(rt.review_capture as usize) else {
        println!("UI_ATTACK_REVIEW_OK: clover glow, launch, flight, recovery, fourth-shot beam, world launch");
        context.loop_controller.exit();
        return Ok(());
    };
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    rt.keys = Keys::default();
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Playing;
        w.projectiles.clear();
        w.impacts.clear();
        for (i, actor) in w.actors.iter_mut().enumerate() {
            *actor = Actor::default();
            actor.active = i == 0;
            actor.human = true;
        }
        let a = &mut w.actors[0];
        a.character = 10;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.ammo = 20;
        a.shot = w.tick as u32;
        a.combat.attacks = if enhanced { 3 } else { 0 };
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0].fire = age < 1.0;
        w.tick(inputs);
        for _ in 0..(age.min(0.30) / DT).round() as usize {
            if age < 1.0 { w.tick([Input::default(); PLAYERS]); }
        }
        assert_eq!(w.projectiles.len(), usize::from(age < 1.0 && !enhanced));
        rt.view_pos = w.actors[0].eye();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    let pose = trickcal_shooting_balddagu::motion::Pose {
        motion: if age < 1.0 { trickcal_shooting_balddagu::motion::Motion::Attack }
            else { trickcal_shooting_balddagu::motion::Motion::Idle },
        age: if age < 1.0 { age } else { 0.0 },
        shot_age: age,
        locomotion: None,
    };
    rt.visuals[0].seek(g, pose);
    ui_fx::weapon(g, rt.weapons[10], pose);
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let eye = Vector3::new(2.0, 1.8, -9.5);
        let target = Vector3::new(0.0, 1.1, -11.6);
        g[rt.camera].local_transform_mut().set_position(eye)
            .set_rotation(UnitQuaternion::face_towards(&(target - eye), &Vector3::y()));
    }
    let ui = context.user_interfaces.first_mut();
    ui.send(ui.root(), WidgetMessage::Visibility(false));
    Ok(())
}

fn capture_ui_attack(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let (name, age, enhanced, first) = UI_ATTACK_SAMPLES[rt.review_capture as usize];
    let g = &context.scenes[rt.scene].graph;
    let muzzle = if first { ui_fx::muzzle(g, rt.weapons[10]) }
        else { rt.visuals[0].clover_muzzle(g) };
    assert_eq!(g[rt.tracers[0]].visibility(), enhanced,
        "Ui beam belongs only to the fourth basic attack, never idle or ordinary bolts");
    if first {
        assert_eq!(g[ui_fx::staff_glow(g, rt.weapons[10])].visibility(), age < ui_fx::FLASH_SECONDS);
        let hub = g.traverse_handle_iter(rt.weapons[10])
            .find(|&h| g[h].name() == "Ui spinning clover hub").unwrap();
        assert!((g[hub].global_position() - muzzle).norm() < 0.002, "launch socket must coincide with the imported clover hub");
    }
    if age == 0.0 {
        let origin = if enhanced {
            assert!(g[rt.tracers[0]].visibility(), "fourth basic shot must show its beam");
            g[rt.tracers[0]].global_transform()
                .transform_point(&Vector3::new(0.0, 0.0, -0.5).into()).coords
        } else { g[rt.projectiles[0]].global_position() };
        assert!((origin - muzzle).norm() < 0.002, "Ui attack must start at the clover: {}", (origin - muzzle).norm());
    } else if age < 1.0 {
        let p = &rt.session.world().projectiles[0];
        assert!((g[rt.projectiles[0]].global_position() - vec(p.pos)).norm() < 0.002,
            "released bolt must rejoin its authoritative path");
    }
    capture_preview(rt.scene, 1, &format!("ui-attack-{name}-"), context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}

/// Re-fire a real basic shot for each frozen launch sample. This checks the
/// final animated muzzle after post_update in both camera and world space.
fn update_muzzle(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let hero = muzzle_hero().unwrap();
    let kind = match hero {
        1 => ProjectileKind::Water,
        2 | 5 => ProjectileKind::Bolt,
        _ => ProjectileKind::Syringe,
    };
    if rt.review_capture >= 2 {
        println!("{}_MUZZLE_REVIEW_OK: real shots at first-person and world muzzles", characters::SLUGS[hero as usize].to_uppercase());
        context.loop_controller.exit();
        return Ok(());
    }
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Playing;
        w.projectiles.clear();
        w.impacts.clear();
        for (i, actor) in w.actors.iter_mut().enumerate() {
            *actor = Actor::default();
            actor.active = i == 0;
            actor.human = true;
        }
        let a = &mut w.actors[0];
        a.character = hero;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.ammo = 20;
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0].fire = true;
        w.tick(inputs);
        // Staff bolts fire at their attack clip's release event after a wind-up.
        if matches!(hero, 2 | 5) {
            let delay = if hero == 5 {
                trickcal_shooting_balddagu::picora_skill::shot_delay(&w.actors[0])
            } else {
                trickcal_shooting_balddagu::erpin::shot_delay()
            };
            for _ in 0..(delay / DT).ceil() as usize + 1 {
                if !w.projectiles.is_empty() { break; }
                w.tick([Input::default(); PLAYERS]);
            }
        }
        assert_eq!(w.projectiles.len(), 1);
        assert_eq!(w.projectiles[0].kind, kind);
        rt.view_pos = w.actors[0].eye();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    if rt.review_capture == 1 {
        let g = &mut context.scenes[rt.scene].graph;
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let eye = Vector3::new(2.0, 1.8, -9.5);
        let target = Vector3::new(0.0, 1.1, -11.6);
        g[rt.camera].local_transform_mut().set_position(eye)
            .set_rotation(UnitQuaternion::face_towards(&(target - eye), &Vector3::y()));
    }
    let ui = context.user_interfaces.first_mut();
    ui.send(ui.root(), WidgetMessage::Visibility(false));
    Ok(())
}
