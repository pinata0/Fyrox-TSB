use super::*;
use trickcal_shooting_balddagu::{
    erpin,
    motion::{Motion, Pose},
};

const FIRST_PERSON_SKILL_AGES: [f32; 9] = [
    0.0, 0.65, 1.2, 1.55, 1.7, 1.8, 64.0 / 30.0, 2.52,
    trickcal_shooting_balddagu::erpin_skill::SKILL_SECONDS,
];

pub fn enabled() -> bool {
    std::env::args().any(|a| a == "--erpin-only")
}
fn world_view(step: u8) -> bool {
    if step >= 40 {
        step < 48
    } else {
        step < 6 || (12..18).contains(&step) || step >= 24
    }
}
pub fn pose(rt: &Runtime) -> Pose {
    let step = rt.review_capture;
    let (motion, age) = if step >= 52 {
        (Motion::ErpinSkill, FIRST_PERSON_SKILL_AGES[(step - 52) as usize])
    } else if step < 12 {
        (
            Motion::Attack,
            step as f32 % 6.0 / 5.0 * Motion::Attack.seconds(2),
        )
    } else if step < 24 {
        (
            Motion::ErpinEat,
            [0.05, 0.30, 0.47, 0.67, 0.87, 1.4][step as usize % 6],
        )
    } else if step < 36 {
        let motion = [
            Motion::Idle,
            Motion::Walk,
            Motion::Run,
            Motion::Jump,
            Motion::Attack,
            Motion::Skill,
            Motion::Ultimate,
            Motion::Dance,
            Motion::Hurt,
            Motion::Death,
            Motion::Spawn,
            Motion::ErpinEat,
        ][(step - 24) as usize];
        (motion, motion.seconds(2) * 0.45)
    } else if step >= 40 {
        [
            (Motion::ErpinSkill, 0.35),
            (Motion::ErpinSkill, 1.1),
            (Motion::ErpinSkill, 1.72),
            (Motion::ErpinSkill, 2.15),
            (Motion::ErpinStart, 0.7),
            (Motion::ErpinCharge, 0.25),
            (Motion::ErpinRecover, 0.2),
            (Motion::ErpinRecover, 1.5),
            (Motion::ErpinSkill, 1.72),
            (Motion::ErpinCharge, 0.25),
            (Motion::ErpinRecover, 0.2),
            (Motion::ErpinRecover, 1.5),
        ][(step - 40) as usize]
    } else {
        (Motion::Idle, 0.0)
    };
    Pose {
        motion,
        age,
        shot_age: age,
        locomotion: None,
    }
}
pub fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if std::env::args().any(|a| a == "--erpin-skills-only") && rt.review_capture < 40 {
        rt.review_capture = 40;
    }
    if std::env::args().any(|a| a == "--erpin-stun-only") && rt.review_capture < 50 { rt.review_capture = 50; }
    if std::env::args().any(|a| a == "--erpin-skill-first-person-only") && rt.review_capture < 52 {
        rt.review_capture = 52;
    }
    if rt.review_capture >= 52 + FIRST_PERSON_SKILL_AGES.len() as u8 {
        println!("ERPIN_REVIEW_OK: staff attacks, left-hand cake, source skill/charge/recovery clips, first-person overhead strike, flustered face, blasts and cake");
        context.loop_controller.exit();
        return Ok(());
    }
    let pose = pose(rt);
    let step = rt.review_capture;
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.chat.open = false;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    rt.keys = Keys::default();
    if let Session::Solo(w) = &mut rt.session {
        w.phase = if step >= 40 { Phase::Playing } else { Phase::Lobby };
        w.tick = 120;
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0;
        }
        let a = &mut w.actors[0];
        a.character = 2;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = 80;
        a.ammo = 20;
        a.shield = 0.0;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        if pose.motion == Motion::ErpinEat {
            a.combat.erpin = erpin::State {
                eating: true,
                elapsed: pose.age,
                ..Default::default()
            };
        }
        use trickcal_shooting_balddagu::erpin_skill::{Phase as Action, State};
        let action = match pose.motion {
            Motion::ErpinSkill => Action::Skill,
            Motion::ErpinStart => Action::Start,
            Motion::ErpinCharge => Action::Charge,
            Motion::ErpinRecover => Action::Recover,
            _ => Action::Off,
        };
        a.combat.erpin_skill = State {
            phase: action,
            elapsed: pose.age,
        };
        if action == Action::Start {
            a.combat.ultimate_flash = trickcal_shooting_balddagu::erpin_skill::START_SECONDS - pose.age;
        }
        if action == Action::Recover {
            a.combat.stun = 3.0 - pose.age;
        }
        rt.view_pos = a.eye();
        w.projectiles.clear();
        if action == Action::Skill && pose.age >= 1.7 {
            for j in 0..4 {
                let mut p = trickcal_shooting_balddagu::combat::Projectile::visual(
                    200 + j,
                    0,
                    ProjectileKind::MagicBomb,
                    V3::new((j as f32 - 1.5) * 0.25, 1.2, -10.5 + j as f32 * 0.35),
                );
                p.character = 2;
                p.velocity = V3::new(0.0, 0.0, 20.0);
                p.age = 0.15;
                w.projectiles.push(p);
            }
        }
        if action == Action::Recover && pose.age < 0.6 {
            let mut p = trickcal_shooting_balddagu::combat::Projectile::visual(
                300,
                0,
                ProjectileKind::ErpinCrash,
                V3::new(0.0, 1.0, -11.1),
            );
            p.character = 2;
            p.age = pose.age;
            w.projectiles.push(p);
        }
        if (36..40).contains(&step) {
            let mut cake = trickcal_shooting_balddagu::combat::Projectile::visual(
                100 + step as u32,
                0,
                ProjectileKind::cake(40 - step),
                V3::new(0.45, 0.18, -11.4),
            );
            cake.character = 2;
            cake.life = 2.0;
            w.projectiles.push(cake);
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    if world_view(step) {
        let g = &mut context.scenes[rt.scene].graph;
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let rear = (24..36).contains(&step);
        let position = if pose.motion == Motion::ErpinStart {
            Vector3::new(8.0, 9.0, -3.0)
        } else if rear {
            Vector3::new(1.15, 1.25, -14.9)
        } else {
            Vector3::new(1.65, 1.25, -8.7)
        };
        let target = Vector3::new(0.0, 0.95, -12.0);
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
pub fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    let g = &context.scenes[rt.scene].graph;
    let p = pose(rt);
    assert_eq!(rt.visuals[0].pose().motion, p.motion);
    for side in ["L", "R"] {
        let wing = g
            .traverse_handle_iter(rt.visuals[0].root)
            .find(|h| g[*h].name() == format!("Wing.{side}"))
            .expect("independent wing");
        assert_eq!(g[g[wing].parent()].name(), "Head");
        assert!(g[wing].global_position().iter().all(|x| x.is_finite()));
    }
    if !world_view(rt.review_capture) {
        let right = rt.first_person_arm.as_ref().unwrap();
        assert!(right.grip_error(g, rt.wave_hand, rt.weapons[2]) < 0.001);
        if p.motion == Motion::Attack {
            let camera_inverse = g[rt.camera].global_transform().try_inverse().unwrap();
            let palm = camera_inverse.transform_point(&right.palm(g).into());
            let gem = camera_inverse.transform_point(&weapons::erpin_muzzle(g, rt.weapons[2]).into());
            assert!(gem.z >= palm.z - 0.001, "Erpin's basic attack must tilt the staff forward");
        }
        if p.motion == Motion::ErpinSkill {
            let camera_inverse = g[rt.camera].global_transform().try_inverse().unwrap();
            let palm = camera_inverse.transform_point(&right.palm(g).into());
            let gem = camera_inverse.transform_point(&weapons::erpin_muzzle(g, rt.weapons[2]).into());
            assert!(palm.z > 0.5, "Erpin's raised grip must stay in front of the camera");
            assert!(gem.z >= palm.z - 0.001, "Erpin's staff must never bend behind the grip");
            if (1.02..=1.38).contains(&p.age) {
                assert!(palm.y > 0.15, "Erpin must raise the staff above eye level");
            }
            if (1.7..=2.28).contains(&p.age) {
                assert!(gem.y < 0.0, "Erpin must strike down through the first release");
            }
        }
        if p.motion == Motion::ErpinEat {
            rt.first_person_support_arm
                .as_ref()
                .unwrap()
                .verify_anatomical_hand(g, rt.camera, false);
            assert_eq!(
                g[rt.erpin_fx.as_ref().unwrap().view.root].visibility(),
                p.age < erpin::BITE_TIMES[3]
            );
        }
    }
    capture_preview(
        rt.scene,
        1,
        &format!("erpin-{:02}-{:?}-", rt.review_capture, p.motion),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
