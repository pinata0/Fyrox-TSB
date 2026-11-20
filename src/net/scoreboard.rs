//! Separate totals packet keeps the movement snapshot below the transport MTU.
use super::*;
use crate::match_stats::MatchStats;

pub(super) fn stats_snapshot(world: &World, token: u64, seq: u64) -> Vec<u8> {
    let mut w = Writer::new(MATCH_STATS);
    w.u64(token);
    w.u64(seq);
    for a in &world.actors {
        w.u16(a.stats.assists);
        w.u16(a.stats.solo_kills);
        w.f32(a.stats.damage);
        w.f32(a.stats.healing);
        w.f32(a.stats.mitigated);
    }
    w.0
}

pub(super) fn read_stats(bytes: &[u8], token: u64) -> Option<(u64, [MatchStats; PLAYERS])> {
    let mut r = Reader::new(bytes, MATCH_STATS)?;
    if r.u64()? != token {
        return None;
    }
    let seq = r.u64()?;
    let mut stats = [MatchStats::default(); PLAYERS];
    for s in &mut stats {
        s.assists = r.u16()?;
        s.solo_kills = r.u16()?;
        s.damage = r.f32()?;
        s.healing = r.f32()?;
        s.mitigated = r.f32()?;
        if [s.damage, s.healing, s.mitigated]
            .into_iter()
            .any(|v| !v.is_finite() || v < 0.0)
        {
            return None;
        }
    }
    r.done().then_some((seq, stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pump(server: &mut Server, clients: &mut [Client], count: usize) {
        for _ in 0..count {
            for c in clients.iter_mut() {
                c.update(Input::default()).unwrap();
            }
            server.update(Input::default()).unwrap();
            for c in clients.iter_mut() {
                c.update(Input::default()).unwrap();
            }
        }
    }

    #[test]
    fn scoreboard_packet_is_bounded_authenticated_and_validated() {
        let mut world = World::default();
        world.actors[0].stats = MatchStats {
            assists: 4,
            solo_kills: 2,
            damage: 123456.5,
            healing: 98765.0,
            mitigated: 810.5,
        };
        let packet = stats_snapshot(&world, 42, 99);
        assert!(packet.len() < MAX_PACKET);
        let (seq, stats) = read_stats(&packet, 42).unwrap();
        assert_eq!(seq, 99);
        assert_eq!(stats[0], world.actors[0].stats);
        assert!(read_stats(&packet, 43).is_none());
        for n in 0..packet.len() {
            assert!(read_stats(&packet[..n], 42).is_none());
        }
        let mut extra = packet;
        extra.push(0);
        assert!(read_stats(&extra, 42).is_none());
        for value in [f32::NAN, f32::INFINITY, -1.0] {
            world.actors[0].stats.damage = value;
            assert!(read_stats(&stats_snapshot(&world, 42, 99), 42).is_none());
        }
    }

    #[test]
    fn scoreboard_totals_reach_clients_survive_snapshots_and_reset() {
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients = vec![Client::connect(server.addr().unwrap()).unwrap()];
        pump(&mut server, &mut clients, 12);
        server.world.actors[0].stats = MatchStats {
            assists: 9,
            solo_kills: 3,
            damage: 3201.5,
            healing: 820.0,
            mitigated: 410.5,
        };
        let expected = server.world.actors[0].stats;
        pump(&mut server, &mut clients, 12);
        clients.push(Client::connect(server.addr().unwrap()).unwrap());
        pump(&mut server, &mut clients, 12);
        assert!(clients.iter().all(|c| c.world.actors[0].stats == expected));
        let peer = &server.peers[0];
        server
            .socket
            .send_to(&stats_snapshot(&World::default(), peer.token, 1), peer.addr)
            .unwrap();
        clients[0].update(Input::default()).unwrap();
        assert_eq!(clients[0].world.actors[0].stats, expected);
        server.world.start();
        pump(&mut server, &mut clients, 12);
        assert!(clients
            .iter()
            .all(|c| c.world.actors[0].stats == MatchStats::default()));
    }
}
