//! Real-shot GPU fixtures: surface alignment, debris, fade and cleanup.
use super::*;

const STAGES: [(&str, f32); 23] = [
    ("before", 0.0),
    ("burst", 0.12),
    ("dent", 1.5),
    ("fading", 3.5),
    ("expired", 4.2),
    ("floor", 0.15),
    ("slash-burst", 0.12),
    ("slash-dent", 1.5),
    ("slash-finisher", 1.5),
    ("slash-fading", 3.5),
    ("slash-expired", 4.2),
    ("slash-floor", 0.15),
    ("dart-stuck", 1.5),
    ("dart-fading", 3.8),
    ("dart-expired", 4.2),
    ("syringe-stuck", 1.5),
    ("syringe-floor", 1.5),
    ("syringe-expired", 4.2),
    ("medical-wall", 0.12),
    ("medical-floor", 0.12),
    ("medical-character", 0.12),
    ("medical-fading", 0.50),
    ("medical-expired", 0.75),
];

fn stages() -> &'static [(&'static str, f32)] {
    if std::env::args().any(|arg| arg == "--syringe-impacts-only") {
        &[
            ("syringe-burst", 0.12),
            ("syringe-stuck", 1.5),
            ("syringe-floor", 1.5),
            ("syringe-fading", 3.8),
            ("syringe-expired", 4.2),
        ]
    } else if std::env::args().any(|arg| arg == "--hilde-grenade-review") {
        &STAGES[18..]
    } else {
        &STAGES
    }
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if projectile_review::enabled() {
        return projectile_review::update(rt, context);
    }
    let Some(&(tag, age)) = stages().get(rt.review_capture as usize) else {
        println!("IMPACT_REVIEW_OK: {} real-contact captures, including Hilde green wall/floor/character explosions, fade and cleanup", stages().len());
        context.loop_controller.exit();
        return Ok(());
    };
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = -std::f32::consts::FRAC_PI_2;
    rt.pitch = if tag.ends_with("floor") { -0.8 } else { 0.0 };
    let melee = tag.starts_with("slash-");
    let embedded = tag.starts_with("dart-") || tag.starts_with("syringe-");
    let medical = tag.starts_with("medical-");
    if matches!(tag, "medical-character" | "medical-floor") {
        rt.yaw = 0.0;
    }
    if let Session::Solo(w) = &mut rt.session {
        if rt.review_frames == 0 {
            w.phase = Phase::Playing;
            w.round += 1;
            w.tick = 100;
            w.impacts.clear();
            w.projectiles.clear();
            for (i, a) in w.actors.iter_mut().enumerate() {
                *a = Actor::default();
                a.human = true;
                a.active = i == 0;
            }
            let a = &mut w.actors[0];
            a.character = if tag.starts_with("dart-") {
                8
            } else if tag.starts_with("syringe-") || medical {
                13
            } else if melee {
                0
            } else {
                14
            };
            if tag == "slash-finisher" {
                a.combat.vivi_combo = 2;
            }
            a.ammo = 30;
            a.hp = MAX_HP;
            a.pos = V3::new(-3.4, 0.0, -23.5);
            a.yaw = rt.yaw;
            a.pitch = rt.pitch;
            if medical {
                a.combat.hilde.phase = trickcal_shooting_balddagu::hilde_skill::Phase::Launcher;
                a.combat.hilde.elapsed = trickcal_shooting_balddagu::hilde_skill::EQUIP_SECONDS;
                if matches!(tag, "medical-character" | "medical-floor") {
                    a.pos = V3::new(0.0, 0.0, -12.0);
                }
                if tag == "medical-character" {
                    let target = &mut w.actors[6];
                    target.active = true;
                    target.character = 2;
                    target.hp = MAX_HP;
                    target.shield = 0.0;
                    target.pos = V3::new(0.0, 0.0, -8.0);
                }
            }
            if tag != "before" {
                let mut input = [Input::default(); PLAYERS];
                input[0] = Input {
                    fire: true,
                    yaw: rt.yaw,
                    pitch: rt.pitch,
                    ..Default::default()
                };
                w.tick(input);
                if embedded || medical {
                    for _ in 0..60 {
                        if !w.impacts.is_empty() {
                            break;
                        }
                        w.tick([Input::default(); PLAYERS]);
                    }
                }
                assert_eq!(
                    w.impacts.len(),
                    1,
                    "review must use a real combat collision"
                );
                assert_eq!(
                    w.impacts[0].kind,
                    if medical {
                        trickcal_shooting_balddagu::impact::ImpactKind::MedicalExplosion
                    } else if tag.starts_with("dart-") {
                        trickcal_shooting_balddagu::impact::ImpactKind::PoisonDart
                    } else if tag.starts_with("syringe-") {
                        trickcal_shooting_balddagu::impact::ImpactKind::Syringe
                    } else if melee {
                        trickcal_shooting_balddagu::impact::ImpactKind::Slash
                    } else {
                        trickcal_shooting_balddagu::impact::ImpactKind::Bullet
                    }
                );
                if tag == "medical-floor" {
                    assert!(w.impacts[0].position.y < 0.25, "grenade must hit the floor");
                }
                w.tick += (age / DT).round() as u64;
            }
            rt.view_pos = w.actors[0].eye();
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    g[rt.weapon].set_visibility(false);
    g[rt.wave_hand].set_visibility(false);
    if embedded && !tag.ends_with("floor") {
        let target = vec(rt.session.world().impacts[0].position);
        let eye = target + Vector3::new(1.25, 0.25, 0.85);
        g[rt.camera]
            .local_transform_mut()
            .set_position(eye)
            .set_rotation(projectiles::facing(V3::new(
                target.x - eye.x,
                target.y - eye.y,
                target.z - eye.z,
            )));
    }
    if medical {
        if let Some(fx) = &rt.hilde_fx {
            g[fx.view.launcher].set_visibility(false);
        }
        let target = vec(rt.session.world().impacts[0].position);
        let eye = target + Vector3::new(3.3, 1.6, if tag == "medical-character" { -4.0 } else { 2.4 });
        g[rt.camera].local_transform_mut()
            .set_position(eye)
            .set_rotation(UnitQuaternion::face_towards(&(target - eye), &Vector3::y()));
    }
    let ui = context.user_interfaces.first_mut();
    ui.send(ui.root(), WidgetMessage::Visibility(false));
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if projectile_review::enabled() {
        return projectile_review::capture(rt, context);
    }
    let tag = stages()[rt.review_capture as usize].0;
    let g = &context.scenes[rt.scene].graph;
    let blasts = g.traverse_handle_iter(g.get_root())
        .filter(|&h| g[h].name() == "Hilde green explosion")
        .count();
    assert_eq!(blasts, if tag.starts_with("medical-") && tag != "medical-expired" {
        hilde_fx::BLAST_PARTS
    } else { 0 });
    let chunks = g
        .traverse_handle_iter(g.get_root())
        .filter(|&h| g[h].name() == "Temporary impact fragment")
        .count();
    assert_eq!(
        chunks,
        match tag {
            "burst" | "floor" | "syringe-burst" => 7,
            "slash-burst" | "slash-floor" => 5,
            _ => 0,
        }
    );
    let embedded = g
        .traverse_handle_iter(g.get_root())
        .filter(|&h| g[h].name() == "Embedded projectile")
        .count();
    assert_eq!(
        embedded,
        usize::from(matches!(
            tag,
            "dart-stuck" | "dart-fading" | "syringe-burst" | "syringe-stuck" | "syringe-floor" | "syringe-fading"
        ))
    );
    capture_preview(rt.scene, 1, &format!("impact-{tag}-"), context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
