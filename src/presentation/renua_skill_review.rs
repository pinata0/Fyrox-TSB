//! Reproducible in-engine inspection of all four gestures, empty hands and time field.
use super::*;
use trickcal_shooting_balddagu::{
    combat::Projectile,
    motion::{Motion, Pose},
    renua_skill::{self as r, Phase as Action, State},
};
const SAMPLES: &[(bool, Action, f32)] = &[
    (false, Action::Off, 0.0),
    (false, Action::Basic, 0.65),
    (false, Action::Basic, 1.30),
    (false, Action::Basic, 1.75),
    (false, Action::Enhanced, 0.85),
    (false, Action::Enhanced, 1.40),
    (false, Action::Skill, 1.15),
    (false, Action::Skill, 3.65),
    (false, Action::Ultimate, 1.9),
    (false, Action::Ultimate, 4.4),
    (false, Action::Ultimate, 8.0),
    (false, Action::Ultimate, 14.5),
    (true, Action::Off, 0.0),
    (true, Action::Basic, 1.30),
    (true, Action::Enhanced, 1.40),
    (true, Action::Skill, 1.15),
    (true, Action::Ultimate, 4.4),
];
pub(super) fn update(rt: &mut Runtime, context: &mut PluginContext) -> GameResult {
    if rt.review_capture as usize >= SAMPLES.len() {
        println!("RENUA_SKILL_REVIEW_OK: 17 world/first-person frames; empty hands, four source clips, cloth, halo, constructs, lasers and hollow hexagon field");
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
        w.actors[6].pos = V3::new(0.0, 0.0, -5.5);
        w.actors[6].hp = MAX_HP;
        w.actors[6].character = 2;
        let a = &mut w.actors[0];
        a.character = 7;
        a.pos = V3::new(0.0, 0.0, -12.0);
        a.hp = MAX_HP;
        a.ammo = 0;
        a.yaw = 0.0;
        a.pitch = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        a.reload = 0.0;
        a.hurt = 0.0;
        a.ability_flash = 0.0;
        a.combat = Default::default();
        a.combat.renua = State {
            phase,
            elapsed: if phase == Action::Off { 0.0 } else { age },
            ..Default::default()
        };
        a.shot_end = V3::new(0.0, 1.2, -5.5);
        rt.view_pos = a.eye();
        if phase == Action::Ultimate
            && (r::FIELD_RELEASE..r::FIELD_RELEASE + r::FIELD_SECONDS).contains(&age)
        {
            let mut p = Projectile::visual(
                1,
                0,
                ProjectileKind::TimeField,
                a.pos + V3::new(0.0, 0.0, 1.0) * r::FIELD_NEAR,
            );
            p.velocity = V3::new(0.0, 0.0, 1.0);
            p.life = r::FIELD_SECONDS;
            w.projectiles.push(p);
        }
        if phase == Action::Basic && (1.3..1.5).contains(&age) {
            w.projectiles.push(Projectile::visual(
                2,
                0,
                ProjectileKind::RenuaOrb,
                a.pos + V3::new(0.0, 1.2, 1.6),
            ));
        }
    }
    rt.review_frames = rt.review_frames.max(1);
    render(rt, context)?;
    let g = &mut context.scenes[rt.scene].graph;
    let pose = Pose {
        motion: phase.motion().unwrap_or(Motion::Idle),
        age,
        shot_age: match phase {
            Action::Enhanced if age >= r::ENHANCED_RELEASE => age - r::ENHANCED_RELEASE,
            Action::Skill => r::SKILL_RELEASES
                .iter()
                .rev()
                .find(|time| age >= **time)
                .map_or(100.0, |time| age - time),
            _ => 100.0,
        },
        locomotion: None,
    };
    rt.visuals[0].seek(g, pose);
    if !first {
        rt.visuals[0].set_shadow_only(g, false);
        g[rt.weapon].set_visibility(false);
        g[rt.wave_hand].set_visibility(false);
        let (position, target) = if phase == Action::Ultimate && age > 4.0 {
            (Vector3::new(9.5, 9.0, -17.0), Vector3::new(0.0, 0.5, -6.8))
        } else {
            (Vector3::new(3.3, 2.2, -7.5), Vector3::new(0.0, 1.05, -11.7))
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
    let (first, phase, _) = SAMPLES[rt.review_capture as usize];
    let g = &context.scenes[rt.scene].graph;
    let fx = rt.renua_fx.as_ref().unwrap();
    let root = rt.visuals[0].root;
    for name in [
        "RenuaCoat.L.2",
        "RenuaCoat.C.2",
        "RenuaCoat.R.2",
        "RenuaHair.L.1",
        "RenuaHair.R.1",
    ] {
        assert!(
            g.traverse_handle_iter(root).any(|h| g[h].name() == name),
            "missing {name}"
        );
    }
    assert!(!g[rt.weapons[7]].visibility(), "Renua must be empty handed");
    if phase == Action::Off {
        assert!(fx.constructs[0].iter().all(|h| !g[*h].visibility()));
    }
    if matches!(phase, Action::Enhanced | Action::Skill)
        && SAMPLES[rt.review_capture as usize].2 > 1.0
    {
        assert!(g[fx.beams[0]].visibility());
    }
    if !first {
        assert!(fx.halos[0].iter().all(|h| g[*h].visibility()));
    }
    if phase == Action::Ultimate {
        let age = SAMPLES[rt.review_capture as usize].2;
        let active = (r::FIELD_RELEASE..r::FIELD_RELEASE + r::FIELD_SECONDS).contains(&age);
        assert_eq!(fx.fields.iter().any(|h| g[*h].visibility()), active);
    }
    capture_preview(
        rt.scene,
        7,
        &format!(
            "renua-skills-{:02}-{}-{:?}-",
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
