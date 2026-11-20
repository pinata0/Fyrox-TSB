//! Deterministic source-motion, sticker, rainbow and stationary first-person QA.
use super::*;
use trickcal_shooting_balddagu::{
    motion::{Motion, Pose},
    picora_skill::{self as s, Phase as Action, State},
};
const SAMPLES: &[(bool, Motion, f32)] = &[
    (false, Motion::Attack, 0.18),
    (false, Motion::PicoraSkill, 0.30),
    (false, Motion::PicoraSkill, 0.84),
    (false, Motion::PicoraSkill, 1.30),
    (false, Motion::PicoraUltimate, 0.20),
    (false, Motion::PicoraUltimate, 0.50),
    (false, Motion::PicoraUltimate, 0.80),
    (false, Motion::PicoraUltimate, 1.45),
    (true, Motion::PicoraSkill, 0.84),
    (true, Motion::PicoraSkill, 1.30),
    (true, Motion::PicoraUltimate, 0.20),
    (true, Motion::PicoraUltimate, 0.50),
    (true, Motion::PicoraUltimate, 1.45),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("PICORA_SKILL_REVIEW_OK: {} frames; staff flash and E/Q launch rays in both views, sleeve sticker, rainbow outline, stationary first-person Q", SAMPLES.len());
        context.loop_controller.exit();
        return Ok(());
    }
    let (first, motion, age) = SAMPLES[rt.review_capture as usize];
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.chat.open = false;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    rt.keys = Keys::default();
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        w.tick = (age * 60.0) as u64;
        w.map.clear();
        w.projectiles.clear();
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i < 2;
            a.shield = 0.0;
            a.combat = Default::default();
            a.hp = MAX_HP;
            a.character = if i == 0 { 5 } else { 2 };
            a.pos = V3::new(
                if i == 0 {
                    0.0
                } else if first {
                    0.7
                } else {
                    2.0
                },
                0.0,
                if i == 0 {
                    0.0
                } else if first {
                    3.0
                } else {
                    1.6
                },
            );
            a.yaw = 0.0;
            a.pitch = 0.0;
            a.reload = 0.0;
            a.hurt = 0.0;
            a.wave_ticks = 0;
            a.dancing = false;
            a.crouching = false;
            a.ability_flash = 0.0;
        }
        let phase = match motion {
            Motion::PicoraSkill => Action::Skill,
            Motion::PicoraUltimate => Action::Ultimate,
            _ => Action::Off,
        };
        w.actors[0].combat.picora = State {
            phase,
            elapsed: if phase == Action::Off { 0.0 } else { age },
            target: if phase == Action::Off { None } else { Some(1) },
            ..Default::default()
        };
        w.actors[0].ammo = 18;
        rt.view_pos = w.actors[0].eye();
        if motion == Motion::PicoraSkill && age >= s::STICKER_RELEASE {
            w.actors[1].combat.sticker = 3.0;
            // Also inspect a received sticker on the local viewmodel.
            if first {
                w.actors[0].combat.sticker = 3.0;
            }
        }
        if motion == Motion::PicoraUltimate && age >= s::ULTIMATE_RELEASE {
            w.actors[1].combat.fashion = 7.0;
            w.actors[1].hp = 240;
        }
        if motion == Motion::Attack {
            let mut bolt = trickcal_shooting_balddagu::combat::Projectile::visual(
                1,
                0,
                ProjectileKind::Bolt,
                V3::new(-0.1, 1.6, 1.2),
            );
            bolt.character = 5;
            bolt.velocity = V3::new(0.0, 0.0, 24.0);
            bolt.life = 1.0;
            w.projectiles.push(bolt);
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    rt.visuals[0].seek(
        g,
        Pose {
            motion,
            age,
            shot_age: if motion == Motion::Attack { age } else { 100.0 },
            locomotion: None,
        },
    );
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let position = Vector3::new(-4.0, 2.7, 5.0);
        let target = Vector3::new(0.7, 1.3, 1.0);
        g[rt.camera]
            .local_transform_mut()
            .set_position(position)
            .set_rotation(UnitQuaternion::face_towards(
                &(target - position),
                &Vector3::y(),
            ));
    }
    Ok(())
}
pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let (first, motion, age) = SAMPLES[rt.review_capture as usize];
    let g = &context.scenes[rt.scene].graph;
    assert_eq!(rt.visuals[0].pose().motion, motion);
    let sticker = motion == Motion::PicoraSkill && age >= s::STICKER_RELEASE;
    assert_eq!(
        g[rt.picora_fx.as_ref().unwrap().stickers[1]].visibility(),
        sticker
    );
    if sticker {
        let effects = rt.picora_fx.as_ref().unwrap();
        assert!(
            effects.anchored(1, false),
            "world sticker must attach to a sleeve vertex"
        );
        if first {
            assert!(
                effects.anchored(0, true),
                "view sticker must attach to a sleeve vertex"
            );
            let inv = g[rt.camera].global_transform().try_inverse().unwrap();
            let point = inv.transform_point(&g[effects.view_sticker].global_position().into());
            println!("PICORA_VIEW_STICKER: {point:?}");
            assert!(
                point.z > 0.0 && point.x.abs() < point.z * 1.2,
                "sticker is outside view"
            );
        }
    }
    if first && motion == Motion::PicoraUltimate {
        let hand = s::hand(motion, age, true);
        assert_eq!(hand.position, [0.0; 3]);
        assert_eq!(hand.angles, [0.0; 3]);
        let a = &rt.session.world().actors[0];
        assert_eq!(a.pos.y, 0.0);
        assert_eq!(a.eye().y, EYE_HEIGHT);
    }
    let weapon = rt.visuals[0].held_weapon();
    let socket = weapons::grips(5)[0].unwrap();
    let target = g[weapon]
        .global_transform()
        .transform_point(&socket.into())
        .coords;
    let distance = (rt.visuals[0].palm_contact(g, false) - target).norm();
    assert!(distance < 0.005, "Picora grip separated: {distance}m");
    if first {
        let error = rt.first_person_arm.as_ref().unwrap().grip_error(g, rt.wave_hand, rt.weapons[5]);
        assert!(error < 0.001, "Picora first-person grip separated: {error}m");
    }
    let effects = rt.picora_fx.as_ref().unwrap();
    let release = match motion {
        Motion::Attack => trickcal_shooting_balddagu::attack::seconds(5) * 18.0 / 47.0,
        Motion::PicoraSkill => s::STICKER_RELEASE,
        Motion::PicoraUltimate => s::ULTIMATE_RELEASE,
        _ => f32::INFINITY,
    };
    let elapsed = age - release;
    let flash = (0.0..if motion == Motion::Attack { 0.12 } else { 0.34 }).contains(&elapsed);
    let trail = motion != Motion::Attack && (0.0..0.24).contains(&elapsed);
    assert_eq!(g[effects.flashes[0]].visibility(), flash);
    assert_eq!(g[effects.trails[0]].visibility(), trail);
    if flash {
        let weapon = if first { rt.weapons[5] } else { weapon };
        let muzzle = weapons::picora_muzzle(g, weapon);
        assert!((g[effects.flashes[0]].global_position() - muzzle).norm() < 0.001,
            "spell flash must stay on the animated staff head");
        if trail {
            let transform = g[effects.trails[0]].global_transform();
            let start = transform.transform_point(&(-Vector3::y()).into()).coords;
            let end = transform.transform_point(&Vector3::y().into()).coords;
            let target = if motion == Motion::PicoraSkill {
                g[effects.stickers[1]].global_position()
            } else {
                let ally = &rt.session.world().actors[1];
                vec(ally.pos) + Vector3::y() * ally.hit_height() * 0.65
            };
            assert!((start - muzzle).norm() < 0.001, "E/Q must leave the staff head");
            assert!((end - target).norm() < 0.001,
                "E/Q must reach the affected ally: end={end:?}, target={target:?}");
        }
    }
    capture_preview(
        rt.scene,
        5,
        &format!(
            "picora-skills-{:02}-{}-{motion:?}-",
            rt.review_capture,
            if first { "fp" } else { "world" }
        ),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
