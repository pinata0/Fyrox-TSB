//! Two-device acceptance test using the real gameplay protocol over EOS.
use std::{
    env, thread,
    time::{Duration, Instant},
};
use trickcal_shooting_balddagu::{
    chat::ChatChannel,
    eos,
    game::Input,
    net::{Client, Server},
};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!("Host: tsb-eos-peer-check --host [--force-relay] [--seconds 180]\nGuest on another PC: tsb-eos-peer-check --join eos:HOST:SOCKET [--force-relay]\nOptions: --config FILE, --code-file FILE, --ready-only (host readiness only)");
        return Ok(());
    }
    let option = |name: &str| -> Result<Option<&str>, &'static str> {
        args.iter()
            .position(|a| a == name)
            .map(|i| {
                args.get(i + 1)
                    .map(String::as_str)
                    .ok_or("Missing option value")
            })
            .transpose()
    };
    let force = args.iter().any(|a| a == "--force-relay");
    eos::configure(
        option("--config")?.unwrap_or("eos.local.json").into(),
        force,
    )?;
    let _shutdown = eos::ShutdownGuard;
    let seconds: u64 = option("--seconds")?.unwrap_or("180").parse()?;
    if !(10..=3600).contains(&seconds) {
        return Err("--seconds must be 10..3600".into());
    }
    let deadline = Instant::now() + Duration::from_secs(seconds);
    if args.iter().any(|a| a == "--host") {
        let mut host = Server::host_eos(None, true)?;
        let mut printed = false;
        let mut ready_cycles = 0;
        let mut previous_code = None;
        let mut last_chat = Instant::now() - Duration::from_secs(2);
        let mut confirmed = None;
        while Instant::now() < deadline {
            host.update(Input::default())?;
            if !printed {
                if let Some(code) = host.room_code() {
                    println!("HOST_READY {code}");
                    if let Some(path) = option("--code-file")? {
                        std::fs::write(path, &code)?;
                    }
                    printed = true;
                    if args.iter().any(|a| a == "--ready-only") {
                        if previous_code.as_ref() == Some(&code) {
                            return Err("Recreated room reused its socket secret".into());
                        }
                        previous_code = Some(code);
                        ready_cycles += 1;
                        if ready_cycles == 3 {
                            println!("PASS: login and 3 P2P room open/close cycles. No peer connection tested.");
                            return Ok(());
                        }
                        host = Server::host_eos(None, true)?;
                        printed = false;
                    }
                }
            }
            if host.peer_count() > 0 && last_chat.elapsed() > Duration::from_secs(1) {
                host.send_chat(ChatChannel::Match, "EOS_HOST_PROBE")?;
                last_chat = Instant::now();
            }
            if host
                .chat
                .messages
                .iter()
                .any(|m| m.text == "EOS_CLIENT_CONFIRMED")
                && confirmed.is_none()
            {
                if force && host.network_label() != "EOS Relay" {
                    return Err("Force-relay check did not establish a relayed connection".into());
                }
                confirmed = Some(Instant::now());
                println!("PEER_CONFIRMED {}", host.network_label());
            }
            if confirmed.is_some_and(|t| t.elapsed() > Duration::from_secs(3)) {
                println!(
                    "PASS: real EOS peer, bidirectional gameplay/chat and snapshot reassembly."
                );
                return Ok(());
            }
            thread::sleep(Duration::from_millis(16));
        }
    } else if let Some(code) = option("--join")? {
        let mut client = Client::connect_address(code)?;
        let mut frames = 0;
        let mut previous = 0;
        let mut confirmed = None;
        while Instant::now() < deadline {
            client.update(Input {
                forward: 0.1,
                ..Default::default()
            })?;
            if let Some(error) = &client.error {
                return Err(error.clone().into());
            }
            if client.received_sequence > previous {
                previous = client.received_sequence;
                frames += 1;
            }
            if frames >= 40
                && client
                    .chat
                    .messages
                    .iter()
                    .any(|m| m.text == "EOS_HOST_PROBE")
                && confirmed.is_none()
            {
                if force && client.network_label() != "EOS Relay" {
                    return Err("Force-relay check did not establish a relayed connection".into());
                }
                client.send_chat(ChatChannel::Match, "EOS_CLIENT_CONFIRMED")?;
                confirmed = Some(Instant::now());
            }
            if confirmed.is_some_and(|t| t.elapsed() > Duration::from_secs(2))
                && client
                    .chat
                    .messages
                    .iter()
                    .any(|m| m.text == "EOS_CLIENT_CONFIRMED")
            {
                println!(
                    "PASS: {} snapshots={frames}, host chat and client acknowledgement received.",
                    client.network_label()
                );
                return Ok(());
            }
            thread::sleep(Duration::from_millis(16));
        }
    } else {
        return Err("Choose --host or --join CODE; use --help".into());
    }
    Err("Peer test timed out. A second PC with its own EOS Device ID must join; no peer success was verified.".into())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("EOS peer check failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
