//! Deterministic GPU inspection of Ui's rain, clover, rider hops and frog form.
use super::*;
use trickcal_shooting_balddagu::{
    motion::{Motion, Pose},
    ui_skill::{self as s, Phase as Action, State},
};
const SAMPLES: &[(bool, Action, f32)] = &[
    (false, Action::Skill, 0.0),
    (false, Action::Skill, 0.80),
    (false, Action::Skill, 1.20),
    (false, Action::Skill, 1.60),
    (false, Action::Ultimate, 0.65),
    (false, Action::Ultimate, 0.97),
    (false, Action::Ultimate, 1.33),
    (false, Action::Ultimate, 1.83),
    (false, Action::Ultimate, 2.17),
    (false, Action::Ultimate, 2.63),
    (true, Action::Off, 0.0),
    (true, Action::Skill, 1.20),
    (true, Action::Off, 0.0),
    (true, Action::Ultimate, 1.33),
    (true, Action::Off, 0.0),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("UI_SKILL_REVIEW_OK: 15 world/first-person frames; clover spin, rain, rider hops, leaf pulses, frog form; two-handed E and one-handed idle/Q/return");
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
            a.active = i == 0 || i == 6;
            a.shield = 0.0;
        }
        w.actors[6].pos = if first { V3::new(0.0, 0.0, -6.0) }
            else { V3::new(-1.6, 0.0, -10.6) };
        w.actors[6].hp = MAX_HP;
        w.actors[6].character = 2;
        let a = &mut w.actors[0];
        a.character = 10;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.ammo = 20;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.crouching = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        a.combat.ui = State {
            phase,
            elapsed: age,
        };
        rt.view_pos = a.eye();
        w.actors[6].combat.polymorph = if phase == Action::Ultimate && age >= s::ULTIMATE_RELEASE {
            3.0
        } else {
            0.0
        };
        if phase == Action::Skill && age >= s::RAIN_RELEASE {
            let mut rain = trickcal_shooting_balddagu::combat::Projectile::visual(
                1,
                0,
                ProjectileKind::RainField,
                w.actors[0].pos,
            );
            rain.age = age - s::RAIN_RELEASE;
            rain.life = s::RAIN_SECONDS - rain.age;
            w.projectiles.push(rain);
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    let pose = Pose {
        motion: phase.motion().unwrap_or(Motion::Idle),
        age,
        shot_age: 100.0,
        locomotion: None,
    };
    rt.visuals[0].seek(g, pose);
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let (position, target) = if phase == Action::Ultimate {
            (Vector3::new(4.2, 2.8, -7.2), Vector3::new(0.0, 1.5, -10.5))
        } else {
            (Vector3::new(3.3, 2.5, -8.0), Vector3::new(0.0, 1.2, -11.6))
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
    let frog = phase == Action::Ultimate && age >= s::ULTIMATE_RELEASE;
    assert_eq!(g[rt.ui_fx.as_ref().unwrap().frogs[6]].visibility(), frog);
    assert_eq!(rt.visuals[0].pose().motion, phase.motion().unwrap_or(Motion::Idle));
    if first {
        let right = rt.first_person_arm.as_ref().unwrap();
        let left = rt.first_person_support_arm.as_ref().unwrap();
        assert!(right.visible(g));
        assert_eq!(left.visible(g), phase == Action::Skill);
        for arm in [right, left].into_iter().filter(|arm| arm.visible(g)) {
            assert!(arm.grip_error(g, rt.wave_hand, rt.weapons[10]) < 0.001);
        }
    }
    let weapon = g.traverse_handle_iter(rt.actors[0])
        .find(|&h| g[h].name() == kit(10).weapon).expect("Ui held staff");
    for left in [false, true] {
        let socket = weapons::grips(10)[usize::from(left)].unwrap();
        let target = g[weapon].global_transform().transform_point(&socket.into()).coords;
        let distance = (rt.visuals[0].palm_contact(g, left) - target).norm();
        assert!(distance < 0.005, "Ui staff grip separated: {distance}m, left={left}");
    }
    capture_preview(
        rt.scene,
        10,
        &format!(
            "ui-skills-{:02}-{}-{:?}-",
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
