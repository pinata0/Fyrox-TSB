//! Reproducible GPU review of case, hinge, launcher, grips and return to normal gun.
use super::*;
use trickcal_shooting_balddagu::{
    hilde_skill::{Phase as Action, State},
    motion::{Motion, Pose},
};
const SAMPLES: &[(bool, Action, f32)] = &[
    (false, Action::Skill, 0.4),
    (false, Action::Skill, 1.6),
    (false, Action::Skill, 2.7),
    (false, Action::Equip, 1.0),
    (false, Action::Launcher, 2.5),
    (false, Action::Launcher, 4.0),
    (true, Action::Skill, 1.6),
    (true, Action::Skill, 2.7),
    (true, Action::Equip, 1.0),
    (true, Action::Launcher, 2.5),
    (true, Action::End, 8.7),
    (true, Action::Off, 0.0),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("HILDE_SKILL_REVIEW_OK: 12 world/first-person views, rising healing crosses without rings, real grenade muzzle launches, dual hand contacts and normal weapon restoration");
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
    let state = State {
        phase,
        elapsed: age,
        firing: phase == Action::Launcher,
        ..Default::default()
    };
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        w.tick = (age * 60.0) as u64;
        w.map.clear();
        w.projectiles.clear();
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0 || i == 6;
            a.human = true;
            a.shield = 0.0;
        }
        w.actors[6].pos = V3::new(0.0, 0.0, -5.5);
        w.actors[6].hp = MAX_HP;
        w.actors[6].character = 2;
        let a = &mut w.actors[0];
        a.character = 13;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        // Exercise the generic pulse suppression as well as the dedicated field.
        a.ability_flash = 0.2;
        a.combat = Default::default();
        a.combat.hilde = state;
        rt.view_pos = a.eye();
        if phase == Action::Launcher {
            w.phase = Phase::Playing;
            let mut inputs = [Input::default(); PLAYERS];
            inputs[0].fire = true;
            w.tick(inputs);
            w.phase = Phase::Lobby;
            assert_eq!(w.projectiles.len(), 1);
            assert_eq!(w.projectiles[0].kind, ProjectileKind::MedicalGrenade);
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    rt.visuals[0].seek(
        g,
        Pose {
            motion: phase.motion().unwrap_or(Motion::Idle),
            age: state.motion_age(),
            shot_age: 0.04,
            locomotion: None,
        },
    );
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let position = if phase == Action::Skill && age > 2.0 {
            // Include the whole healing area, not just the held case.
            Vector3::new(12.0, 10.0, 3.0)
        } else { Vector3::new(2.8, 2.0, -8.3) };
        let target = Vector3::new(0.0, 1.0, -11.8);
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
    let fx = rt.hilde_fx.as_ref().unwrap();
    let props = if first { &fx.view } else { &fx.world[0] };
    assert!(!g[rt.pulses[0]].visibility(), "Hilde must not show the generic blue pulse");
    assert_eq!(g[fx.fields[0]].visibility(), phase == Action::Skill
        && age >= trickcal_shooting_balddagu::hilde_skill::HEAL_RELEASE);
    if phase == Action::Launcher {
        let position = g[rt.projectiles[0]].global_position();
        assert!((position - props.muzzle(g)).norm() < 0.002,
            "medical grenade misses the animated launcher muzzle: {}",
            (position - props.muzzle(g)).norm());
    }
    assert_eq!(g[props.case].visibility(), phase == Action::Skill);
    assert_eq!(
        g[props.launcher].visibility(),
        matches!(phase, Action::Equip | Action::Launcher | Action::End)
    );
    if first {
        assert_eq!(g[rt.weapons[13]].visibility(), phase == Action::Off);
    }
    if phase != Action::Off {
        let root = if phase == Action::Skill {
            props.case
        } else {
            props.launcher
        };
        let p = rt.visuals[0].pose();
        let contact = if first {
            rt.first_person_arm.as_ref().unwrap().palm(g)
        } else {
            rt.visuals[0].palm_contact(g, false)
        };
        assert!(
            (g[root].global_position() - contact).norm() < 0.003,
            "right palm loses medical prop: {}", (g[root].global_position()-contact).norm()
        );
        if phase != Action::Skill {
            let target = g[root]
                .global_transform()
                .transform_point(&hilde_fx::support(p).into())
                .coords;
            let contact = if first {
                rt.first_person_support_arm.as_ref().unwrap().palm(g)
            } else {
                rt.visuals[0].palm_contact(g, true)
            };
            assert!(
                (target - contact).norm() < 0.025,
                "support palm loses launcher: {}",
                (target - contact).norm()
            );
        }
    }
    capture_preview(
        rt.scene,
        13,
        &format!(
            "hilde-skills-{:02}-{}-{:?}-",
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
