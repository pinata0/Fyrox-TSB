//! Repeatable rendered checks of shared Shift skills and their first/third-person views.
use super::*;

const STAGES: [(&str, u8, usize, usize, bool); 16] = [
    ("dragon-ready", 0, 0, 0, false),
    ("dragon-charge", 0, 30, 0, false),
    ("dragon-cooldown", 0, 181, 0, false),
    ("beast-ready", 11, 0, 0, false),
    ("beast-leaping", 12, 30, 0, true),
    ("beast-cooldown", 11, 100, 0, false),
    ("spirit-ready", 1, 0, 0, false),
    ("spirit-burrow", 1, 1, 0, false),
    ("spirit-ground-marker", 10, 1, 0, true),
    ("spirit-charge", 10, 1, 24, false),
    ("spirit-emerged", 10, 1, 37, false),
    ("surge-fairy-ready", 2, 0, 0, false),
    ("surge-witch-two", 4, 0, 1, false),
    ("surge-fairy-one", 6, 0, 2, false),
    ("surge-empty", 8, 0, 3, false),
    ("surge-stunned", 5, 1, 1, false),
];

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let Some(&(stage, id, steps, held, external)) = STAGES.get(rt.review_capture as usize) else {
        println!("RACE_SKILL_REVIEW_OK: leap, landing, burrow, charge, emerge, cooldown and body/weapon visibility");
        context.loop_controller.exit();
        return Ok(());
    };
    rt.home_open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.yaw = 0.0;
    rt.pitch = -0.08;
    if let Session::Solo(w) = &mut rt.session {
        if rt.review_frames == 0 {
            for a in &mut w.actors {
                a.active = false;
            }
            w.phase = Phase::Playing;
            w.ground_effects.clear();
            w.actors[0] = Actor::default();
            w.actors[0].character = id;
            w.actors[0].human = true;
            w.actors[0].pos = V3::new(0.0, 0.0, -13.0);
            w.actors[0].ammo = kit(id).magazine;
            let mut inputs = [Input::default(); PLAYERS];
            if trickcal_shooting_balddagu::race_skill::has_surge(character(id).race) {
                if stage == "surge-stunned" {
                    w.ground_effects.push(
                        trickcal_shooting_balddagu::ground_effect::GroundEffect {
                            owner: 6,
                            character: 4,
                            min: V3::new(-2.0, -0.01, -12.69),
                            max: V3::new(2.0, 0.05, -12.68),
                            damage: 0.0,
                            stun: 2.0,
                            remaining: 10.0,
                            interval: 1.0,
                            class: trickcal_shooting_balddagu::combat::AttackClass::Skill,
                            next_hit: [0; PLAYERS],
                        },
                    );
                }
                for _ in 0..held {
                    inputs[0].phase_shift = true;
                    w.tick(inputs);
                    inputs[0].phase_shift = false;
                    for _ in 0..9 {
                        w.tick(inputs);
                    }
                }
                assert_eq!(w.actors[0].race_skill.surge_charges, 3 - held as u8);
                if stage == "surge-stunned" {
                    assert!(w.actors[0].combat.stun > 0.0);
                    assert!((w.actors[0].pos.z + 12.69).abs() < 0.001);
                }
            } else {
                inputs[0].phase_shift = true;
                for _ in 0..steps {
                    w.tick(inputs);
                }
                inputs[0].phase_shift = false;
                inputs[0].fire = true;
                for _ in 0..held {
                    w.tick(inputs);
                }
            }
        }
        rt.view_pos = w.actors[0].eye();
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let a = &rt.session.world().actors[0];
    let g = &mut context.scenes[rt.scene].graph;
    assert_eq!(g[rt.actors[0]].visibility(), !a.burrowed());
    assert_eq!(g[rt.weapon].visibility(), !a.burrowed());
    assert_eq!(g[rt.burrows[0]].visibility(), a.burrowed());
    if external {
        let camera = a.pos + V3::new(4.0, 3.0, -5.0);
        let aim = a.pos + V3::new(0.0, 0.8, 0.0) - camera;
        g[rt.camera]
            .local_transform_mut()
            .set_position(vec(camera))
            .set_rotation(UnitQuaternion::from_euler_angles(
                -(aim.y / aim.length()).asin(),
                aim.x.atan2(aim.z),
                0.0,
            ));
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
    }
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if !context
        .graphics_context
        .as_initialized_ref()
        .renderer
        .scene_data_map
        .contains_key(&rt.scene)
    {
        return Ok(());
    }
    let size = context.user_interfaces.first().screen_size();
    let tag = format!(
        "race-skill-{}x{}-{}-",
        size.x as u32, size.y as u32, STAGES[rt.review_capture as usize].0
    );
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
