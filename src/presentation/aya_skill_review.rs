//! Deterministic GPU checkpoints for the two casts, cloth and status presentation.
use super::*;
use trickcal_shooting_balddagu::{
    aya_skill::{self as aya, Phase as AyaPhase, State},
    combat::Projectile,
    motion::{Motion, Pose},
};
const SAMPLES: &[(u8, AyaPhase, f32)] = &[
    (0, AyaPhase::Skill, 0.30),
    (0, AyaPhase::Skill, 0.85),
    (0, AyaPhase::Skill, 1.35),
    (0, AyaPhase::Ultimate, 0.6),
    (0, AyaPhase::Ultimate, 1.90),
    (0, AyaPhase::Ultimate, 2.45),
    (1, AyaPhase::Skill, 0.9),
    (1, AyaPhase::Ultimate, 2.1),
    (2, AyaPhase::Off, 0.0),
    (3, AyaPhase::Off, 0.0),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("AYA_SKILL_REVIEW_OK: source E/Q, 16 skirt joints, flapping butterflies, purple outlines, snowfall, flowers, ice and frozen first-person view");
        context.loop_controller.exit();
        return Ok(());
    }
    let (mode, phase, age) = SAMPLES[rt.review_capture as usize];
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.chat.open = false;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    rt.keys = Keys::default();
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Playing;
        w.tick = (age * 60.0) as u64;
        w.map.clear();
        w.projectiles.clear();
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0 || i == 6;
            a.shield = 0.0;
            a.combat = Default::default();
            a.dancing = false;
            a.wave_ticks = 0;
            a.hurt = 0.0;
            a.reload = 0.0;
            a.hp = MAX_HP;
        }
        let a = &mut w.actors[0];
        a.character = 4;
        a.ammo = 0;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.ability_flash = 0.0;
        a.combat.aya = State {
            phase,
            elapsed: age,
        };
        if mode == 3 {
            a.combat.frozen = 2.0;
        }
        rt.view_pos = a.eye();
        let enemy = &mut w.actors[6];
        enemy.character = 2;
        enemy.pos = V3::new(0.0, 0.0, -7.0);
        enemy.yaw = std::f32::consts::PI;
        enemy.combat.weaken = if mode == 2 { 3.0 } else { 0.0 };
        enemy.combat.frozen = if phase == AyaPhase::Ultimate && age >= aya::FREEZE_RELEASE {
            2.5
        } else {
            0.0
        };
        if phase == AyaPhase::Ultimate {
            let mut p = Projectile::visual(90, 0, ProjectileKind::SnowGarden, w.actors[0].pos);
            p.age = age;
            p.life = aya::SNOW_SECONDS - age;
            w.projectiles.push(p);
        }
        if phase == AyaPhase::Skill && age >= aya::BUTTERFLY_RELEASE {
            for j in 0..5 {
                let mut p = Projectile::visual(
                    j + 1,
                    0,
                    ProjectileKind::Butterfly,
                    V3::new((j as f32 - 2.0) * 0.45, 1.3, -10.0),
                );
                p.velocity = V3::new(0.0, 0.0, 14.0);
                w.projectiles.push(p);
            }
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    rt.visuals[0].seek(
        g,
        Pose {
            motion: phase.motion().unwrap_or(Motion::Idle),
            age,
            shot_age: 100.0,
            locomotion: None,
        },
    );
    if mode == 0 || mode == 2 {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let target = if mode == 2 {
            Vector3::new(0.0, 1.1, -7.0)
        } else {
            Vector3::new(0.0, 1.0, -11.0)
        };
        let pos = if mode == 2 {
            target + Vector3::new(1.8, 0.5, -3.5)
        } else if phase == AyaPhase::Ultimate && age > 2.4 {
            target + Vector3::new(5.5, 3.3, 6.0)
        } else {
            target + Vector3::new(2.8, 1.3, 3.3)
        };
        g[rt.camera]
            .local_transform_mut()
            .set_position(pos)
            .set_rotation(UnitQuaternion::face_towards(&(target - pos), &Vector3::y()));
        if let Some(effects) = &rt.aya_fx {
            let w = rt.session.world();
            effects.update(g, w, &w.actors, 0, false, true);
        }
    }
    Ok(())
}
pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let g = &context.scenes[rt.scene].graph;
    assert_eq!(
        g.traverse_handle_iter(rt.visuals[0].root)
            .filter(|n| g[*n].name().starts_with("AyaSkirt."))
            .count(),
        16
    );
    let (_, phase, age) = SAMPLES[rt.review_capture as usize];
    if phase == AyaPhase::Ultimate && (1.9..2.5).contains(&age) {
        let joint = |name: &str| {
            let n = g
                .traverse_handle_iter(rt.visuals[0].root)
                .find(|n| g[*n].name() == name)
                .unwrap();
            g[n].global_position()
        };
        for side in ["L", "R"] {
            let shoulder = joint(&format!("UpperArm.{side}"));
            let hand = joint(&format!("Hand.{side}"));
            assert!(
                (shoulder.y - hand.y).abs() < 0.08,
                "Aya spreads both arms at frost release"
            );
            assert!(
                (shoulder.x - hand.x).abs() > 0.30,
                "Aya opens both hands away from the body"
            );
        }
        rt.visuals[0].verify_aya_ultimate(g);
        for name in ["UpperArm.L", "UpperArm.R", "Hand.L", "Hand.R"] {
            if let Some(n) = g
                .traverse_handle_iter(rt.visuals[0].root)
                .find(|n| g[*n].name() == name)
            {
                println!("AYA_GPU_POSE {age} {name} {:?}", g[n].global_position());
            }
        }
    }
    capture_preview(
        rt.scene,
        4,
        &format!("aya-skills-{:02}-", rt.review_capture),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
