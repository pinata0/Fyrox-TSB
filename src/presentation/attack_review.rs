//! Six poses per hero in world and first-person views, using the same sampled clock.
use super::*;
use trickcal_shooting_balddagu::motion::{Motion, Pose};

fn recoil_only() -> bool {
    std::env::args().any(|arg| arg == "--recoil-only")
}

fn vivi_only() -> bool {
    std::env::args().any(|arg| arg == "--vivi-only")
}

fn naia_only() -> bool {
    std::env::args().any(|arg| arg == "--naia-only")
}

fn aya_only() -> bool {
    std::env::args().any(|arg| arg == "--aya-only")
}

fn komi_only() -> bool {
    std::env::args().any(|arg| arg == "--komi-only")
}

const KOMI_PHASES: [f32; 11] = [0.0, 0.11, 0.22, 0.34, 0.46, 0.54, 0.62, 0.80, 0.90, 0.96, 1.0];

fn phase(step: usize) -> f32 {
    if komi_only() {
        KOMI_PHASES[step % KOMI_PHASES.len()]
    } else if recoil_only() {
        [0.0, 0.08, 0.16, 0.4, 0.7, 1.0][step % 6]
    } else if aya_only() {
        [0.0, 0.06, 0.2, 0.4, 0.6, 0.8, 0.94, 1.0][step % 8]
    } else {
        (step % 90 % 6) as f32 / 5.0
    }
}

fn world_view(step: u8) -> bool {
    if recoil_only() || komi_only() {
        false
    } else if aya_only() {
        step % 16 < 8
    } else if naia_only() {
        step < 6
    } else if vivi_only() {
        step % 12 < 6
    } else {
        step < 90
    }
}

pub(super) fn pose(rt: &Runtime, id: u8) -> Pose {
    if erpin_review::enabled() {
        return erpin_review::pose(rt);
    }
    if move_review::enabled() {
        return move_review::pose(rt, id);
    }
    let phase = phase(rt.review_capture as usize);
    Pose {
        motion: if (vivi_only() && rt.review_capture / 12 == 2)
            || (aya_only() && rt.review_capture >= 16)
        {
            Motion::Attack2
        } else {
            Motion::Attack
        },
        age: phase * Motion::Attack.seconds(id),
        shot_age: phase * Motion::Attack.seconds(id),
        locomotion: None,
    }
}

pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if erpin_review::enabled() {
        return erpin_review::update(rt, context);
    }
    if move_review::enabled() {
        return move_review::update(rt, context);
    }
    let step = rt.review_capture as usize;
    if step
        >= if komi_only() {
            KOMI_PHASES.len()
        } else if recoil_only() {
            18
        } else if aya_only() {
            32
        } else if naia_only() {
            12
        } else if vivi_only() {
            36
        } else {
            180
        }
    {
        if komi_only() {
            println!("KOMI_PILLOW_REVIEW_OK: enlarged pillow, one-handed windup, cross-screen swing and recovery; palm contact < 1 mm");
        } else if recoil_only() {
            println!("RECOIL_REVIEW_OK: Naia, Hilde and Sion; rise, peak and recovery; both hands keep grip contact");
        } else if aya_only() {
            println!("AYA_TRANSFORM_REVIEW_OK: both attacks, eight world and eight first-person phases each; folds, blades and grips verified");
        } else if naia_only() {
            println!("NAIA_ATTACK_REVIEW_OK: six world and six first-person poses, independent braid chains and source facial states");
        } else if vivi_only() {
            println!("VIVI_COMBO_REVIEW_OK: Attack1_1, Attack1_1, Attack2_1; 18 world and 18 first-person poses; open hands and independent floating swords");
        } else {
            println!("ATTACK_REVIEW_OK: 15 source attacks, 90 world poses, 90 first-person poses");
        }
        context.loop_controller.exit();
        return Ok(());
    }
    let id = if komi_only() {
        12
    } else if recoil_only() {
        [1, 13, 14][step / 6]
    } else if aya_only() {
        4
    } else if naia_only() {
        1
    } else if vivi_only() {
        0
    } else {
        ((step % 90) / 6) as u8
    };
    rt.home_open = false;
    rt.lobby.open = false;
    rt.selecting = false;
    rt.grabbed = true;
    rt.chat.open = false;
    rt.yaw = 0.0;
    rt.pitch = 0.0;
    rt.keys.w = false;
    rt.keys.zoom = false;
    if let Session::Solo(w) = &mut rt.session {
        w.phase = Phase::Lobby;
        w.tick = 120;
        for (i, a) in w.actors.iter_mut().enumerate() {
            a.active = i == 0;
        }
        let a = &mut w.actors[0];
        a.character = id;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.dancing = false;
        a.wave_ticks = 0;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        if vivi_only() {
            a.combat.vivi_combo = (step / 12 + 1) as u8;
        }
        if aya_only() {
            a.combat.aya_combo = if step < 16 { 1 } else { 3 };
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    if world_view(step as u8) {
        let g = &mut context.scenes[rt.scene].graph;
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        g[rt.camera]
            .local_transform_mut()
            .set_position(Vector3::new(1.65, 1.25, -8.7))
            .set_rotation(UnitQuaternion::from_euler_angles(
                0.0,
                std::f32::consts::PI + 0.46,
                0.0,
            ));
    }
    Ok(())
}

pub(super) fn capture(rt: &mut Runtime, context: PluginContext) -> GameResult {
    if erpin_review::enabled() {
        return erpin_review::capture(rt, context);
    }
    if move_review::enabled() {
        return move_review::capture(rt, context);
    }
    let id = rt.session.world().actors[0].character;
    let g = &context.scenes[rt.scene].graph;
    if komi_only() {
        assert!(g[rt.weapons[12]].visibility());
        assert!(rt.first_person_arm.as_ref().unwrap().visible(g));
        assert!(!rt.first_person_support_arm.as_ref().unwrap().visible(g));
        assert!((g[rt.weapons[12]].local_transform().scale().x - 1.30).abs() < 0.001);
        let palm = rt.first_person_arm.as_ref().unwrap().palm(g);
        let local = g[rt.camera]
            .global_transform()
            .try_inverse()
            .unwrap()
            .transform_point(&palm.into());
        assert!(local.z > 0.50, "Komi's hand crossed the near plane");
    }
    if aya_only() {
        let weapon = if world_view(rt.review_capture) {
            g.traverse_handle_iter(rt.actors[0])
                .find(|h| g[*h].name() == kit(id).weapon)
                .unwrap()
        } else {
            rt.weapons[4]
        };
        let expected = trickcal_shooting_balddagu::aya_attack::weapon(
            phase(rt.review_capture as usize),
            pose(rt, id).motion == Motion::Attack2,
        );
        let mut shapes = 0;
        let mut blades = 0;
        for node in g.traverse_handle_iter(weapon) {
            if let Some(number) = g[node]
                .name()
                .strip_prefix("Aya_Blade_")
                .and_then(|n| n.parse::<u8>().ok())
            {
                assert_eq!(
                    g[node].visibility(),
                    number == expected.blade && expected.emergence > 0.001,
                    "Aya blade visibility diverged"
                );
                blades += 1;
                if number == expected.blade && expected.emergence > 0.9 {
                    let size = g[node].world_bounding_box().max - g[node].world_bounding_box().min;
                    assert!(size.norm() > 0.4, "Aya blade lost FBX scale: {size:?}");
                }
            }
            if let Some(mesh) = g[node].cast::<fyrox::scene::mesh::Mesh>() {
                for shape in mesh.blend_shapes() {
                    let value = if shape.name.ends_with("parasolFold") {
                        Some(expected.fold)
                    } else if shape.name.ends_with("parasolSpread") {
                        Some(expected.spread)
                    } else {
                        None
                    };
                    if let Some(value) = value {
                        assert!((shape.weight - value * 100.0).abs() < 0.01);
                        shapes += 1;
                    }
                }
            }
        }
        assert_eq!(blades, 3);
        assert_eq!(shapes, 2);
    }
    if naia_only() && world_view(rt.review_capture) {
        let nodes: Vec<_> = g.traverse_handle_iter(rt.visuals[0].root).collect();
        for side in ["L", "R"] {
            for segment in 1..=3 {
                let name = format!("Braid.{side}.{segment:02}");
                let node = nodes.iter().find(|node| g[**node].name() == name).unwrap();
                assert!(g[*node]
                    .global_position()
                    .iter()
                    .all(|value| value.is_finite()));
            }
        }
        let expected = trickcal_shooting_balddagu::naia_attack::face(
            pose(rt, id).age / Motion::Attack.seconds(id),
        );
        let mut eyes = 0;
        for node in &nodes {
            if let Some(mesh) = g[*node].cast::<fyrox::scene::mesh::Mesh>() {
                for shape in mesh.blend_shapes() {
                    if shape.name.contains("eyeBlink_") {
                        let index = usize::from(shape.name.ends_with("_R"));
                        assert!(
                            (shape.weight - expected.blink[index] * 100.0).abs() < 0.1,
                            "Naia facial clock diverged: {} {}",
                            shape.name,
                            shape.weight
                        );
                        eyes += 1;
                    }
                }
            }
        }
        assert_eq!(eyes, 2);
    }
    if !world_view(rt.review_capture) {
        for arm in rt
            .first_person_arm
            .iter()
            .chain(rt.first_person_support_arm.iter())
        {
            arm.verify_anatomical_hand(g, rt.camera, true);
            let error = arm.grip_error(g, rt.wave_hand, rt.weapons[id as usize]);
            assert!(
                error.is_finite() && error < 0.001,
                "attack grip detached {id}: {error}"
            );
        }
        assert_eq!(rt.first_person_arm.as_ref().unwrap().character, id);
    } else {
        let checkpoints: serde_json::Value = serde_json::from_str(include_str!(
            "../../assets/characters/attack/pose-checkpoints.json"
        ))
        .unwrap();
        let vivi: serde_json::Value = serde_json::from_str(include_str!(
            "../../assets/characters/attack/vivi-checkpoints.json"
        ))
        .unwrap();
        let aya: serde_json::Value = serde_json::from_str(include_str!(
            "../../assets/characters/attack/aya-checkpoints.json"
        ))
        .unwrap();
        for name in ["Hips", "Head", "Hand.L", "Hand.R"] {
            let joint = g
                .traverse_handle_iter(rt.visuals[0].root)
                .find(|h| g[*h].name() == name)
                .unwrap();
            let actual = g[joint].global_position() - g[rt.actors[0]].global_position();
            let p = if aya_only() {
                &aya[usize::from(pose(rt, id).motion == Motion::Attack2)]["poses"]
                    [(rt.review_capture % 8) as usize][name]
            } else if id == 0 {
                &vivi[usize::from(pose(rt, id).motion == Motion::Attack2)]["poses"]
                    [(rt.review_capture % 6) as usize][name]
            } else {
                &checkpoints[id as usize]["poses"][(rt.review_capture % 6) as usize][name]
            };
            let expected = Vector3::new(
                p[0].as_f64().unwrap() as f32,
                p[2].as_f64().unwrap() as f32,
                -p[1].as_f64().unwrap() as f32,
            );
            assert!(
                (actual - expected).norm() < 0.06,
                "FBX attack pose mismatch {id}/{name}: {actual:?} vs {expected:?}"
            );
        }
        let weapon = g
            .traverse_handle_iter(rt.actors[0])
            .find(|h| g[*h].name() == kit(id).weapon)
            .unwrap();
        for left in [false, true] {
            if id == 0 {
                let blade = weapons::vivi_blade_node(g, weapon, left);
                assert!(
                    (g[blade].global_position() - rt.visuals[0].palm_contact(g, left)).norm()
                        > 0.12,
                    "levitating sword must not be held"
                );
                rt.visuals[0].verify_hand_grip(g, left);
                continue;
            }
            if left && id != 0 {
                continue;
            }
            let grip = weapons::grips(id)[usize::from(left)].unwrap();
            let prop = weapons::grip_node(g, weapon, id, left);
            let contact = g[prop]
                .global_transform()
                .transform_point(&grip.into())
                .coords;
            assert!(
                (contact - rt.visuals[0].palm_contact(g, left)).norm() < 0.001,
                "world attack grip detached {id}"
            );
        }
    }
    capture_preview(
        rt.scene,
        3,
        &format!(
            "attack-{:02}-{}-{}-phase{}-",
            id,
            characters::SLUGS[id as usize],
            if world_view(rt.review_capture) {
                "world"
            } else {
                "fps"
            },
            if vivi_only() || aya_only() || komi_only() {
                rt.review_capture
            } else {
                rt.review_capture % 6
            }
        ),
        context,
    )
    .map_err(|e| fyrox::plugin::error::GameError::str(e.to_string()))?;
    rt.review_capture += 1;
    rt.review_frames = 0;
    Ok(())
}
