//! Deterministic GPU review: empty hands, palm fire, source clips, souls and healing wave.
use super::*;
use trickcal_shooting_balddagu::{
    bella_skill::{self as b, Phase as Action, State},
    motion::{Motion, Pose},
};
const SAMPLES: &[(bool, Action, f32)] = &[
    (false, Action::Off, 0.0),
    (true, Action::Off, 1.0),
    (true, Action::Off, 1.6),
    (false, Action::Skill, 1.0),
    (false, Action::Skill, 2.25),
    (true, Action::Skill, 2.25),
    (false, Action::Ultimate, 0.75),
    (false, Action::Ultimate, 2.50),
    (false, Action::Ultimate, 3.15),
    (true, Action::Ultimate, 2.50),
    (false, Action::Off, 5.35),
    (true, Action::Off, 5.35),
];
pub(super) fn first(rt: &Runtime) -> bool {
    SAMPLES.get(rt.review_capture as usize).is_some_and(|s| s.0)
}
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("BELLA_SKILL_REVIEW_OK: 12 world/first-person frames");
        context.loop_controller.exit();
        return Ok(());
    }
    let (first, phase, age) = SAMPLES[rt.review_capture as usize];
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
            a.active = matches!(i, 0 | 1 | 6 | 7 | 8);
            a.shield = 0.0;
            a.hp = MAX_HP;
            a.combat = Default::default();
        }
        for (i, x, z) in [
            (1, 2.0, -10.0),
            (6, -2.0, -6.0),
            (7, 0.0, -4.0),
            (8, 2.0, -6.0),
        ] {
            w.actors[i].character = 2;
            w.actors[i].pos = V3::new(x, 0.0, z + 12.0);
            w.actors[i].yaw = std::f32::consts::PI;
            if i >= 6 && phase == Action::Ultimate && age >= b::TAUNT_RELEASE {
                w.actors[i].combat.taunt = 3.0;
            }
        }
        let a = &mut w.actors[0];
        a.character = 9;
        a.pos = V3::new(0.0, 0.0, 0.0);
        a.ammo = 0;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.crouching = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat.bella = State {
            phase,
            elapsed: if phase == Action::Off { 0.0 } else { age },
            targets: if phase == Action::Ultimate && age >= b::TAUNT_RELEASE {
                (1 << 6) | (1 << 7) | (1 << 8)
            } else {
                0
            },
        };
        rt.view_pos = a.eye();
        if phase == Action::Skill && age >= b::FIRE_RELEASE {
            let mut p = trickcal_shooting_balddagu::combat::Projectile::visual(
                1,
                0,
                ProjectileKind::BellaFire,
                a.pos,
            );
            p.age = age - b::FIRE_RELEASE;
            p.velocity = V3::new(0.0, 0.0, 1.0);
            w.projectiles.push(p);
        }
        if age > 5.0 {
            let mut p = trickcal_shooting_balddagu::combat::Projectile::visual(
                2,
                0,
                ProjectileKind::BellaHeal,
                w.actors[0].pos,
            );
            p.age = age - 5.0;
            w.projectiles.push(p);
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
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let (pos, target) = if phase == Action::Skill || age > 5.0 {
            (Vector3::new(7.8, 8.5, 8.5), Vector3::new(0.0, 0.0, 4.5))
        } else if phase == Action::Ultimate && age > b::TAUNT_RELEASE {
            (Vector3::new(6.5, 4.2, 8.0), Vector3::new(0.0, 1.1, 3.5))
        } else {
            (Vector3::new(3.0, 2.6, 3.2), Vector3::new(0.0, 1.0, 0.0))
        };
        g[rt.camera]
            .local_transform_mut()
            .set_position(pos)
            .set_rotation(UnitQuaternion::face_towards(&(target - pos), &Vector3::y()));
    }
    Ok(())
}
pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let (first, phase, _) = SAMPLES[rt.review_capture as usize];
    let g = &context.scenes[rt.scene].graph;
    let effects = rt.bella_fx.as_ref().unwrap();
    if phase == Action::Off {
        assert!(!g[effects.lanterns[0]].visibility());
        assert_eq!(g[effects.palm_fire].visibility(), first);
    }
    if phase == Action::Ultimate {
        assert_eq!(g[effects.view_lantern].visibility(), first);
        assert_eq!(g[effects.lanterns[0]].visibility(), !first);
    }
    capture_preview(
        rt.scene,
        9,
        &format!(
            "bella-skills-{:02}-{}-{:?}-",
            rt.review_capture,
            if first { "fp" } else { "world" },
            phase
        ),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
