use std::{
    io::{self, BufRead},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use trickcal_shooting_balddagu::{
    game::{Input, Phase, DT},
    net::Server,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let addr = args
        .get(1)
        .filter(|a| !a.starts_with("--"))
        .map_or("0.0.0.0:28080", String::as_str);
    let mut server = Server::bind(addr, false)?;
    if let Some(i) = args.iter().position(|a| a == "--map") {
        let slug = args.get(i + 1).ok_or("--map needs a map slug")?;
        let map =
            trickcal_shooting_balddagu::level::MapId::from_slug(slug).ok_or("Unknown map slug")?;
        server.world.select_map(map);
    }
    println!(
        "Trickcal : Shooting Balddagu dedicated server on {}",
        server.addr()?
    );
    println!("First player starts the first match. Press Enter in the lobby for the next match. Ctrl+C to stop.");
    let (start_tx, start_rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            if line.is_err() {
                break;
            }
            let _ = start_tx.try_send(());
        }
    });
    let mut first_match = true;
    let mut last_phase = Phase::Lobby;
    loop {
        let start = Instant::now();
        let requested = start_rx.try_recv().is_ok();
        if server.world.phase == Phase::Lobby
            && server.peer_count() > 0
            && (first_match || requested)
        {
            server.start();
            first_match = false;
        }
        server.update(Input::default())?;
        if server.world.phase != last_phase {
            println!("{:?}: {}", server.world.phase, server.world.event);
            if server.world.phase == Phase::Lobby {
                println!(
                    "Players are back in the same lobby. Press Enter to start the next match."
                );
            }
            last_phase = server.world.phase;
        }
        thread::sleep(Duration::from_secs_f32(DT).saturating_sub(start.elapsed()));
    }
}
