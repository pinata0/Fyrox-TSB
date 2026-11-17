//! Deterministic GPU fixtures: all 15 death clips, recorded world rewind and return to play.
use super::*;
use trickcal_shooting_balddagu::deathcam::DEATH_HOLD;
use trickcal_shooting_balddagu::motion::{death_seconds, Motion};
const TOTAL: u8 = 66;

fn transition_review() -> bool {
    std::env::args().any(|arg| arg == "--death-transition-review")
}

fn roster_review() -> bool {
    std::env::args().any(|arg| arg == "--death-roster-review")
}

fn selected_hero() -> Option<u8> {
    let args: Vec<_> = std::env::args().collect();
    args.iter()
        .position(|arg| arg == "--death-hero")
        .and_then(|index| args.get(index + 1))
        .and_then(|hero| hero.parse::<u8>().ok())
        .filter(|&hero| hero < 15)
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    let transitions = transition_review();
    let roster = roster_review();
    let hero = selected_hero().unwrap_or(2);
    let only = selected_hero().filter(|_| !transitions && !roster);
    if rt.review_capture == 0 {
        if let Some(id) = only {
            rt.review_capture = id * 4;
        }
    }
    if rt.review_capture >= only.map_or(TOTAL, |id| id * 4 + 4) {
        if transitions {
            println!("DEATH_TRANSITION_REVIEW_OK: hero {hero}, final death pose, Id replay without placeholder weapon, selection, respawn, second killer");
        } else if roster {
            println!("DEATH_ROSTER_REVIEW_OK: 15 current models x 4 death poses, replay, selection, respawn, second killer");
        } else if let Some(id) = only {
            println!("DEATHCAM_HERO_REVIEW_OK: hero {id}, four authored/GPU poses");
        } else {
            println!("DEATHCAM_REVIEW_OK: 15 heroes x 4 death poses; recorded killer POV, projectiles, lethal endpoint, selection, respawn, second killer");
        }
        context.loop_controller.exit();
        return Ok(());
    }
    // A focused transition fixture uses the current meshes, independent of
    // the historical procedural-rig joint checkpoints in the full review.
    if transitions && rt.review_capture < hero * 4 {
        rt.review_capture = hero * 4;
    }
    if transitions && (hero * 4 + 4..60).contains(&rt.review_capture) {
        rt.review_capture = 60;
    }
    let step = rt.review_capture;
    let id = if step < 60 { step / 4 } else { hero };
    let target = if step == 65 { 7 } else { 6 };
    rt.home_open = false;
    rt.lobby.open = false;
    rt.grabbed = true;
    rt.selecting = step == 63;
    rt.selected = 11;
    if rt.review_frames == 0 {
        rt.deathcam = DeathCamera::default();
        if let Session::Solo(w) = &mut rt.session {
            w.phase = Phase::Playing;
            for (i, a) in w.actors.iter_mut().enumerate() {
                a.active = true;
                a.human = true;
                a.hp = MAX_HP;
                a.killed_by = None;
                a.deaths = 0;
                a.respawn = 0.0;
                a.pos = V3::new(20.0 + i as f32 * 2.0, 0.0, -12.0);
                a.wave_ticks = 0;
                a.shot = 0;
                a.ability_flash = 0.0;
                a.combat = Default::default();
            }
            w.actors[0].character = id;
            w.actors[0].pos = V3::new(0.0, 0.0, -12.0);
            w.actors[0].yaw = 0.0;
            w.actors[0].pitch = 0.0;
            w.actors[target].character = 3;
            w.actors[target].pitch = -0.04;
            for tick in 0..=180 {
                w.tick = tick;
                w.actors[target].pos = V3::new(-1.5 + tick as f32 / 60.0, 0.0, -18.0);
                w.actors[target].yaw = (1.5 - tick as f32 / 60.0).atan2(6.0);
                w.actors[target].shot = if tick >= 90 { 1 } else { 0 };
                w.actors[target].shot_end = w.actors[0].eye();
                w.actors[target].ability_flash = if (88..100).contains(&tick) { 0.3 } else { 0.0 };
                w.projectiles = if (80..110).contains(&tick) {
                    vec![trickcal_shooting_balddagu::combat::Projectile::visual(
                        7,
                        target,
                        ProjectileKind::Water,
                        V3::new(0.1, 1.0, -18.0 + (tick - 80) as f32 * 0.15),
                    )]
                } else {
                    vec![]
                };
                rt.deathcam.update(w, 0, DT);
            }
            w.tick = 181;
            w.actors[0].hp = 0;
            w.actors[0].deaths = 1;
            w.actors[0].respawn = 7.0;
            w.actors[0].killed_by = Some(target);
            rt.deathcam.update(w, 0, 0.0);
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    let age = if step < 60 {
        death_seconds(id) * [0.0, 0.32, 0.65, 1.0][(step % 4) as usize]
    } else {
        death_seconds(id)
            + DEATH_HOLD
            + match step {
                60 => 0.1,
                61 => 1.51,
                _ => 3.1,
            }
    };
    rt.deathcam.elapsed = age - context.dt;
    if let Session::Solo(w) = &mut rt.session {
        w.tick = 182;
        // Live state diverges completely from the recorded clip, including killer death/disconnect.
        w.actors[target].pos = V3::new(30.0, 0.0, 30.0);
        w.actors[target].hp = 0;
        w.actors[target].human = false;
        w.actors[target].character = 14;
        w.actors[0].respawn = (7.0 - age).max(0.0);
        if step == 63 {
            w.actors[0].character = 11;
        }
        if step == 64 {
            w.actors[0].hp = MAX_HP;
            w.actors[0].killed_by = None;
            w.actors[0].respawn = 0.0;
        }
    }
    if step != 64 {
        rt.yaw = 2.5;
        rt.pitch = 0.7;
    }
    render(rt, context)
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if context
        .graphics_context
        .as_initialized_ref()
        .renderer
        .scene_data_map
        .get(&rt.scene)
        .is_none_or(|data| {
            data.scene_data.gbuffer.width <= 1 || data.scene_data.gbuffer.height <= 1
        })
    {
        return Ok(());
    }
    let g = &context.scenes[rt.scene].graph;
    let step = rt.review_capture;
    let position = **g[rt.camera].local_transform().position();
    assert!(!rt.orbit.active);
    if step < 60 {
        assert_eq!(rt.deathcam.stage, DeathStage::TopDown);
        assert!(g[rt.actors[0]].visibility());
        assert!(!rt.visuals[0].is_shadow_only());
        assert!(!g[rt.weapon].visibility());
        assert!(!g[rt.wave_hand].visibility());
        let pose = rt.deathcam.poses()[0];
        assert_eq!(pose.motion, Motion::Death);
        assert!(rt.deathcam.elapsed <= death_seconds(step / 4) + 0.02);
        let player = g[rt.visuals[0].player]
            .cast::<fyrox::scene::animation::AnimationPlayer>()
            .unwrap();
        for anim in player.animations().iter() {
            let expected = 14.1 + (pose.age / death_seconds(step / 4)).min(1.0);
            assert!((anim.time_position() - expected).abs() < 0.002);
        }
        let joints: Vec<_> = ["Hips", "Head"]
            .iter()
            .map(|name| {
                let joint = g
                    .traverse_handle_iter(rt.visuals[0].root)
                    .find(|h| g[*h].name() == *name)
                    .unwrap();
                g[joint].global_position() - g[rt.actors[0]].global_position()
            })
            .collect();
        println!("DEATH_POSE_GPU: {step} {joints:?}");
        if !transition_review() && !roster_review() {
            let checkpoints: serde_json::Value = serde_json::from_str(include_str!(
                "../../assets/characters/death/pose-checkpoints.json"
            ))
            .unwrap();
            for (joint, name) in joints.iter().zip(["Hips", "Head"]) {
                let p = &checkpoints[(step / 4) as usize]["poses"][(step % 4) as usize][name];
                let expected = Vector3::new(
                    p[0].as_f64().unwrap() as f32,
                    p[2].as_f64().unwrap() as f32,
                    -p[1].as_f64().unwrap() as f32,
                );
                // Fractional authored Bezier poses and baked FBX linear samples may differ slightly.
                assert!(
                    (joint - expected).norm() < 0.06,
                    "Death skin does not match the authored Blender pose: {step} {name}"
                );
            }
            if step == 31 {
                assert!(joints[1].y > 2.3, "Renua's recorded upward exit must move the actual skin, not only the animation cursor");
            }
        }
    } else if step == 64 {
        assert!(rt.deathcam.view.is_none());
        assert!(g[rt.weapon].visibility());
        assert!(rt.visuals[0].is_shadow_only());
        assert!((position - vec(rt.session.world().actors[0].eye())).norm() < 0.001);
    } else {
        assert!(rt.deathcam.is_replay());
        let view = rt.deathcam.view.unwrap();
        let scene = rt.deathcam.scene().unwrap();
        assert_eq!(view.target, if step == 65 { 7 } else { 6 });
        assert!(rt.visuals[view.target].is_shadow_only());
        assert_eq!(scene.actors[view.target].character, 3);
        assert!(scene.actors[view.target].alive());
        if transition_review() || roster_review() {
            assert!(
                g.traverse_handle_iter(rt.weapons[3])
                    .all(|node| g[node].cast::<fyrox::scene::mesh::Mesh>().is_none()),
                "Id's recorded view must not show the placeholder handheld cube"
            );
        }
        assert!(
            (scene.actors[view.target].pos - rt.session.world().actors[view.target].pos).length()
                > 20.0
        );
        if step == 63 {
            assert!(!g[rt.weapon].visibility());
            assert!((position - Vector3::new(120.06, 1.18, 2.65)).norm() < 0.001);
        } else {
            assert!((position - vec(view.position)).norm() < 0.001);
            assert!(g[rt.weapon].visibility());
        }
        if step == 61 {
            assert!(!scene.projectiles.is_empty());
            assert!(scene.actors[0].alive());
            assert!(g[rt.tracers[view.target]].visibility());
        }
        if step >= 62 {
            assert_eq!(rt.deathcam.stage, DeathStage::Finished);
            assert!(!scene.actors[0].alive());
        }
    }
    let tag = format!("death-replay-{step:02}-");
    capture_preview(rt.scene, 1, &tag, context)
        .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
