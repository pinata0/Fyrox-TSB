//! Deterministic world/first-person inspection for Vivi's source-timed gestures.
use super::*;
use trickcal_shooting_balddagu::{
    motion::{Motion, Pose},
    vivi_skill::{self as v, Phase as Action, State},
};
const SAMPLES: &[(bool, Action, f32)] = &[
    (false, Action::Off, 0.0),
    (false, Action::Off, 0.33),
    (false, Action::Off, 0.67),
    (false, Action::Left, 0.20),
    (false, Action::Left, 0.40),
    (false, Action::Left, 0.60),
    (false, Action::Right, 0.23),
    (false, Action::Right, 0.50),
    (false, Action::Right, 0.80),
    (false, Action::Skill, 0.28),
    (false, Action::Skill, 0.55),
    (false, Action::Skill, 0.78),
    (false, Action::Skill, 1.05),
    (false, Action::Ultimate, 0.45),
    (false, Action::Ultimate, 0.95),
    (false, Action::Ultimate, 1.45),
    (false, Action::Ultimate, 2.08),
    (false, Action::Ultimate, 2.40),
    (false, Action::Ultimate, 3.50),
    (true, Action::Off, 0.0),
    (true, Action::Left, 0.40),
    (true, Action::Right, 0.50),
    (true, Action::Skill, 0.28),
    (true, Action::Skill, 0.78),
    (true, Action::Ultimate, 0.95),
    (true, Action::Ultimate, 1.55),
    (true, Action::Ultimate, 2.25),
    (true, Action::Off, 0.0),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("VIVI_SKILL_REVIEW_OK: 28 frames; four blades, wing cycle, both slashes, shoulder-to-raised hand, silver bubble, rising camera, falling lance and mercury pool");
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
    rt.pitch = if first && phase == Action::Ultimate {
        -0.10
    } else {
        0.0
    };
    rt.keys = Keys::default();
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        w.tick = (age * 60.0) as u64;
        w.map.clear();
        w.projectiles.clear();
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0 || i == 6;
            a.shield = 0.0;
        }
        w.actors[6].pos = V3::new(-1.0, 0.0, -2.0);
        w.actors[6].character = 2;
        w.actors[6].hp = MAX_HP;
        let a = &mut w.actors[0];
        a.character = 0;
        a.pos = V3::new(
            0.0,
            if phase == Action::Ultimate {
                v::ultimate_lift(age)
            } else {
                0.0
            },
            -12.0,
        );
        a.hp = MAX_HP;
        a.ammo = 0;
        a.yaw = 0.0;
        a.pitch = rt.pitch;
        a.dancing = false;
        a.crouching = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        a.combat.vivi = State {
            phase,
            elapsed: if phase == Action::Off { 0.0 } else { age },
        };
        if phase == Action::Skill && age >= v::SHIELD_RELEASE {
            a.combat.guard = v::SHIELD_HEALTH;
            a.combat.mercury = true;
        }
        rt.view_pos = a.eye();
        if phase == Action::Ultimate && age >= v::LANCE_RELEASE {
            let mut p = trickcal_shooting_balddagu::combat::Projectile::visual(
                1,
                0,
                ProjectileKind::MercuryField,
                V3::new(0.0, 0.0, -2.0),
            );
            p.age = age - v::LANCE_RELEASE;
            p.life = v::FIELD_SECONDS - p.age;
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
        let (position, target) = if phase == Action::Ultimate && age >= v::LANCE_RELEASE {
            (Vector3::new(9.0, 7.0, 1.5), Vector3::new(0.0, 2.1, -5.0))
        } else {
            (
                Vector3::new(3.4, 2.6, -7.7),
                Vector3::new(
                    0.0,
                    1.15 + if phase == Action::Ultimate {
                        v::ultimate_lift(age)
                    } else {
                        0.0
                    },
                    -12.0,
                ),
            )
        };
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
    let (first, phase, age) = SAMPLES[rt.review_capture as usize];
    let g = &context.scenes[rt.scene].graph;
    assert_eq!(
        rt.visuals[0].pose().motion,
        phase.motion().unwrap_or(Motion::Idle)
    );
    let weapon = rt.visuals[0].held_weapon();
    assert_eq!(
        g[weapon]
            .children()
            .iter()
            .filter(|&&h| g[h].name().starts_with("Mercury sword."))
            .count(),
        4
    );
    if first {
        let right = rt.first_person_arm.as_ref().unwrap();
        let left = rt.first_person_support_arm.as_ref().unwrap();
        assert!(right.visible(g));
        assert_eq!(left.visible(g), phase != Action::Skill);
    }
    assert_eq!(
        g[rt.guards[0]].visibility(),
        phase == Action::Skill && age >= v::SHIELD_RELEASE
    );
    capture_preview(
        rt.scene,
        0,
        &format!(
            "vivi-skills-{:02}-{}-{:?}-",
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
