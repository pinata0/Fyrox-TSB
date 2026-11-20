use super::*;
use crate::{
    motion::MotionClock,
    shion_skill::{Phase as Action, State},
};

#[test]
fn shion_phases_rounds_and_blast_center_roundtrip_with_no_packet_growth() {
    let mut w = World::default();
    let base = snapshot(&w, 42, 1).len();
    w.actors[0].character = 14;
    w.actors[0].ammo = 6;
    w.actors[0].shot_end = V3::new(3.0, 1.2, 4.0);
    for phase in [
        Action::Off,
        Action::Load,
        Action::Ready,
        Action::Fire,
        Action::End,
        Action::Ultimate,
    ] {
        w.actors[0].combat.shion = State {
            phase,
            elapsed: if phase.duration() > 0.0 {
                phase.duration() - 0.002
            } else {
                0.0
            },
        };
        w.actors[0].combat.magic_ammo = if matches!(phase, Action::Ready | Action::Fire) {
            4
        } else {
            0
        };
        let bytes = snapshot(&w, 42, 2);
        assert_eq!(bytes.len(), base);
        assert!(bytes.len() < MAX_PACKET);
        let (_, decoded, _) = read_snapshot(&bytes, 42).unwrap();
        let a = &decoded.actors[0];
        assert_eq!(a.combat.shion.phase, phase);
        assert_eq!(a.combat.magic_ammo, w.actors[0].combat.magic_ammo);
        assert!((a.combat.shion.elapsed - w.actors[0].combat.shion.elapsed).abs() < DT + 0.0001);
        assert_eq!(a.shot_end, w.actors[0].shot_end);
        if phase != Action::Off {
            assert_eq!(
                MotionClock::default().observe(a, 0.0, 1, true).motion,
                phase.motion().unwrap()
            );
        }
    }
}

#[test]
fn shion_late_joiner_receives_stationary_loaded_stance() {
    let mut server = Server::bind("127.0.0.1:0", false).unwrap();
    let mut first = Client::connect(server.addr().unwrap()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while first.slot.is_none() && Instant::now() < deadline {
        first.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(first.slot.is_some(), "initial client did not join");
    server.world.actors[0].character = 14;
    server.world.actors[0].ammo = 6;
    server.world.actors[0].combat.shion = State {
        phase: Action::Ready,
        elapsed: 0.0,
    };
    server.world.actors[0].combat.magic_ammo = 3;
    let mut client = Client::connect(server.addr().unwrap()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        first.update(Input::default()).unwrap();
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        if client.slot.is_some()
            && client.world.actors[0].combat.shion.phase == Action::Ready
            && client.world.actors[0].combat.magic_ammo == 3
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(client.slot.is_some());
    assert_eq!(client.world.actors[0].combat.shion.phase, Action::Ready);
    assert_eq!(client.world.actors[0].combat.magic_ammo, 3);
}
