use super::*;
use trickcal_shooting_balddagu::{
    erpin,
    motion::{Motion, Pose},
};

pub fn enabled() -> bool {
    std::env::args().any(|a| a == "--erpin-only")
}
fn world_view(step: u8) -> bool {
    step < 6 || (12..18).contains(&step) || step >= 24
}
pub fn pose(rt: &Runtime) -> Pose {
    let step = rt.review_capture;
    let (motion, age) = if step < 12 {
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
    if rt.review_capture >= 40 {
        println!("ERPIN_REVIEW_OK: staff attacks, left-hand cake, all twelve body clips and four dropped-cake portions");
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
        w.phase = Phase::Lobby;
        w.tick = 120;
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0;
        }
        let a = &mut w.actors[0];
        a.character = 2;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = 80;
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
        w.projectiles.clear();
        if step >= 36 {
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
        let position = if rear {
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
