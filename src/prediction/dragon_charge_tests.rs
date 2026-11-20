use super::*;
use crate::race_skill;

#[test]
fn dragon_charge_predicts_forced_movement_turns_expiry_and_stun() {
    for stun_at in [None, Some(40)] {
        let mut world = World::room(MapId::default(), true);
        world.map.clear();
        world.phase = Phase::Playing;
        world.actors[0] = Actor {
            character: 0,
            human: true,
            ..Default::default()
        };
        let mut motion = ClientMotion::default();
        motion.observe(&world, 0, 0);
        for sequence in 1..=680 {
            if stun_at == Some(sequence) {
                world.actors[0].combat.stun = 0.5;
                motion.observe(&world, 0, sequence - 1);
            }
            let input = Input {
                phase_shift: sequence == 1 || sequence > 665,
                forward: -1.0,
                strafe: if sequence % 30 < 15 { 0.5 } else { -0.5 },
                yaw: sequence as f32 * 0.01,
                ..Default::default()
            };
            motion.advance(&world, sequence, input);
            let mut inputs = [Input::default(); PLAYERS];
            inputs[0] = input;
            world.tick(inputs);
            let predicted = motion.actor(&world, 0, 0);
            assert!(
                (predicted.pos - world.actors[0].pos).length() < 0.001,
                "tick {sequence}"
            );
            assert_eq!(
                race_skill::charging(&predicted),
                race_skill::charging(&world.actors[0])
            );
            assert!(
                (predicted.race_skill.cooldown - world.actors[0].race_skill.cooldown).abs() < DT
            );
            if sequence % 3 == 0 {
                motion.observe(&world, 0, sequence);
            }
        }
    }
}
