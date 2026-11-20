//! Reproducible GPU captures of the pod and the independently posed empty hand.
use super::*;
use trickcal_shooting_balddagu::{
    id_skill::{self as id, Phase as IdPhase, State},
    motion::{Motion, Pose},
};

const SAMPLES: &[(bool, IdPhase, f32, bool)] = &[
    (false, IdPhase::Off, 0.0, false),
    (false, IdPhase::Off, 0.08, true),
    (false, IdPhase::Skill, 0.22, false),
    (false, IdPhase::Skill, 1.35, false),
    (false, IdPhase::Pulse, 0.18, false),
    (false, IdPhase::Ultimate, 0.60, false),
    (false, IdPhase::Ultimate, 1.75, false),
    (false, IdPhase::Ultimate, 2.40, false),
    (false, IdPhase::Ultimate, 3.70, false),
    (false, IdPhase::Ultimate, 5.15, false),
    (true, IdPhase::Off, 0.0, false),
    (true, IdPhase::Off, 0.10, true),
    (true, IdPhase::Skill, 0.22, false),
    (true, IdPhase::Skill, 1.20, false),
    (true, IdPhase::Pulse, 0.18, false),
    (true, IdPhase::Off, 0.40, false),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("ID_SKILL_REVIEW_OK: split pod, hair chains, closed/open eyes, paired orbits, empty pointing hand, laser, pulse and monochrome HUD");
        context.loop_controller.exit();
        return Ok(());
    }
    let (first, phase, age, attack) = SAMPLES[rt.review_capture as usize];
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
            a.active = i == 0 || i == 6;
            a.shield = 0.0;
        }
        w.actors[6].pos = V3::new(0.0, 0.0, -5.5);
        w.actors[6].hp = MAX_HP;
        w.actors[6].character = 2;
        let a = &mut w.actors[0];
        a.character = 3;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.ammo = kit(3).magazine;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        a.combat.id = State {
            phase,
            elapsed: if phase == IdPhase::Off { 0.0 } else { age },
            invulnerable: if phase == IdPhase::Skill {
                (3.0 - age).max(0.0)
            } else {
                0.0
            },
            pulse_cooldown: if phase == IdPhase::Pulse {
                3.0 - age
            } else {
                0.0
            },
            ..Default::default()
        };
        a.shot_end = V3::new(0.0, EYE_HEIGHT, -5.5);
        rt.view_pos = a.eye();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    let pose = Pose {
        motion: if attack {
            Motion::Attack
        } else {
            phase.motion().unwrap_or(Motion::Idle)
        },
        age,
        shot_age: if attack { age } else { 100.0 },
        locomotion: None,
    };
    rt.visuals[0].seek(g, pose);
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        shields::update(g, rt.guards[0], &rt.session.world().actors[0]);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let position = Vector3::new(3.7, 2.5, -7.8);
        let target = Vector3::new(0.0, 1.1, -12.0);
        g[rt.camera]
            .local_transform_mut()
            .set_position(position)
            .set_rotation(UnitQuaternion::face_towards(
                &(target - position),
                &Vector3::y(),
            ));
        visible(context.user_interfaces.first(), rt.hud.id_hud.root, false);
    }
    Ok(())
}
pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let (first, phase, age, attack) = SAMPLES[rt.review_capture as usize];
    let g = &context.scenes[rt.scene].graph;
    let visual = &rt.visuals[0];
    let pod = visual.pod.as_ref().unwrap();
    for name in [
        "IdPod.Seat",
        "IdPod.Bow",
        "IdPod.Back",
        "IdPod.Crown",
        "IdPod.Lantern.L",
        "IdPod.Lantern.R",
        "IdHair.L.3",
        "IdHair.C.3",
        "IdHair.R.3",
    ] {
        assert!(
            g.traverse_handle_iter(visual.root)
                .any(|h| g[h].name() == name),
            "missing {name}"
        );
    }
    let closed = id::eye_closure(visual.pose().motion, age);
    for h in g.traverse_handle_iter(visual.root) {
        if let Some(m) = g[h].cast::<fyrox::scene::mesh::Mesh>() {
            for shape in m.blend_shapes() {
                if shape.name.contains("eyeBlink_") {
                    assert!((shape.weight - closed * 100.0).abs() < 1.0, "eye state");
                }
            }
        }
    }
    if !first {
        assert_eq!(
            g[pod.cover].visibility(),
            phase == IdPhase::Ultimate && age < 2.14
        );
        assert_eq!(
            g[pod.orbits[1]].visibility(),
            phase == IdPhase::Ultimate && (id::ULTIMATE_RELEASE..4.5).contains(&age)
        );
    }
    if first && phase == IdPhase::Skill && age > 0.78 {
        assert!(!context.user_interfaces.first()[rt.hud.id_hud.root].visibility());
    }
    if first {
        assert!(!g[rt.guards[0]].visibility(), "first-person view must stay clear");
    }
    capture_preview(
        rt.scene,
        3,
        &format!(
            "id-skills-{:02}-{}-{:?}{}-",
            rt.review_capture,
            if first { "fp" } else { "world" },
            phase,
            if attack { "-laser" } else { "" }
        ),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
