//! Reproducible GPU review of both Mayo guns, grips and return to the blowpipe.
use super::*;
use trickcal_shooting_balddagu::{
    mayo_skill::{Phase as Action, State},
    motion::{Motion, Pose},
};
const SAMPLES: &[(bool, Action, f32)] = &[
    (false, Action::Skill, 0.4),
    (false, Action::Skill, 0.82),
    (false, Action::Skill, 1.08),
    (false, Action::Ultimate, 0.65),
    (false, Action::Ultimate, 1.65),
    (false, Action::Ultimate, 2.58),
    (true, Action::Skill, 0.82),
    (true, Action::Skill, 1.08),
    (true, Action::Ultimate, 0.65),
    (true, Action::Ultimate, 1.65),
    (true, Action::Ultimate, 2.58),
    (true, Action::Off, 0.0),
    (false, Action::Off, 0.0),
    (true, Action::Skill, 0.0),
    (true, Action::Skill, 0.12),
    (true, Action::Skill, 1.95),
    (true, Action::Ultimate, 0.0),
    (true, Action::Ultimate, 0.25),
    (true, Action::Ultimate, 3.25),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("MAYO_SKILL_REVIEW_OK: {} world/first-person views, blowpipe/rifle/rotary muzzle launches, dual hand contacts, uncollapsed sleeves and normal weapon restoration", SAMPLES.len());
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
        ..Default::default()
    };
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
        a.character = 8;
        a.ammo = 0;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        a.combat.mayo = state;
        rt.view_pos = a.eye();
        if phase == Action::Off || phase.shots().iter().any(|&at| (age - at).abs() < 0.03) {
            // A fresh launch on each review frame exercises the post-animation
            // socket capture, independently of previous sample IDs/positions.
            let mut dart = trickcal_shooting_balddagu::combat::Projectile::visual(
                u32::from(rt.review_capture) * 100 + rt.review_frames,
                0,
                if phase == Action::Off { ProjectileKind::Needle } else { ProjectileKind::Poison },
                a.eye() + V3::new(0.0, 0.0, 0.8),
            );
            dart.velocity = V3::new(0.0, 0.0, 48.0);
            w.projectiles.push(dart);
        }
    }
    rt.orbit.active = !first;
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    rt.visuals[0].seek(
        g,
        Pose {
            motion: phase.motion().unwrap_or(Motion::Idle),
            age: state.elapsed,
            shot_age: 0.04,
            locomotion: None,
        },
    );
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let position = Vector3::new(1.75, 1.45, -9.65);
        let target = Vector3::new(0.0, 0.85, -11.8);
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
    let (first, phase, _) = SAMPLES[rt.review_capture as usize];
    let g = &context.scenes[rt.scene].graph;
    let fx = rt.mayo_fx.as_ref().unwrap();
    let props = if first { &fx.view } else { &fx.world[0] };
    assert_eq!(g[props.skill].visibility(), phase == Action::Skill);
    assert_eq!(g[props.ultimate].visibility(), phase == Action::Ultimate);
    if first {
        assert_eq!(g[rt.weapons[8]].visibility(), phase == Action::Off);
        let left = rt.first_person_support_arm.as_ref().unwrap();
        assert_eq!(left.visible(g), phase != Action::Off);
        if phase != Action::Off {
            for arm in rt.first_person_arm.iter().chain(rt.first_person_support_arm.iter()) {
                arm.verify_sleeve_corner(g, rt.camera);
                arm.verify_mayo_sleeve(g);
            }
        }
    }
    if !rt.session.world().projectiles.is_empty() {
        let p = rt.visuals[0].pose();
        let muzzle = if phase == Action::Off {
            let weapon = if first { rt.weapons[8] } else { rt.visuals[0].held_weapon() };
            weapons::mayo_muzzle(g, weapon)
        } else {
            props.muzzle(g, p)
        };
        let tip = g[rt.projectiles[0]].global_transform().transform_point(
            &(Vector3::z() * projectiles::embedded_tip(
                trickcal_shooting_balddagu::impact::ImpactKind::PoisonDart)).into(),
        ).coords;
        assert!((tip - muzzle).norm() < 0.002,
            "Mayo dart misses animated {phase:?} muzzle (first={first}): {}", (tip - muzzle).norm());
    }
    if phase != Action::Off {
        let root = if phase == Action::Skill {
            props.skill
        } else {
            props.ultimate
        };
        let p = rt.visuals[0].pose();
        let contact = if first {
            rt.first_person_arm.as_ref().unwrap().palm(g)
        } else {
            rt.visuals[0].palm_contact(g, false)
        };
        assert!(
            (g[root].global_position() - contact).norm() < 0.003,
            "right palm loses Mayo gun: {}",
            (g[root].global_position() - contact).norm()
        );
        {
            let target = g[root]
                .global_transform()
                .transform_point(&mayo_fx::support(p).into())
                .coords;
            let contact = if first {
                rt.first_person_support_arm.as_ref().unwrap().palm(g)
            } else {
                rt.visuals[0].palm_contact(g, true)
            };
            assert!(
                (target - contact).norm() < 0.025,
                "support palm loses Mayo gun: {}",
                (target - contact).norm()
            );
        }
    }
    println!("MAYO_CAPTURE_BEGIN {}", rt.review_capture);
    capture_preview(
        rt.scene,
        8,
        &format!(
            "mayo-skills-{:02}-{}-{:?}-",
            rt.review_capture,
            if first { "fp" } else { "world" },
            phase
        ),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    println!("MAYO_CAPTURE_END {}", rt.review_capture);
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
