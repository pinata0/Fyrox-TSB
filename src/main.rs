mod presentation;
use fyrox::{
    core::log::Log,
    dpi::{LogicalSize, PhysicalSize, Size},
    engine::{executor::Executor, GraphicsContextParams},
    event_loop::EventLoop,
    window::WindowAttributes,
};
use presentation::{Game, Runtime, Session};
use trickcal_shooting_balddagu::{
    game::World,
    net::{Client, Server},
    online::OnlineSession,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(feature = "eos")]
    let _eos_shutdown = {
        use trickcal_shooting_balddagu::eos;
        let path = if let Some(i) = args.iter().position(|a| a == "--eos-config") {
            args.get(i + 1).ok_or("--eos-config needs a file")?.into()
        } else {
            std::path::PathBuf::from("eos.local.json")
        };
        eos::configure(path, args.iter().any(|a| a == "--force-relay"))?;
        eos::ShutdownGuard
    };
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Trickcal : Shooting Balddagu\n\n  tsb                         Home / create or join a room\n  tsb --eos-host              Create a port-forwarding-free room\n  tsb --join eos:HOST:SOCKET   Join an EOS room\n  tsb --force-relay           Force EOS relay for testing\n  tsb --eos-config FILE       EOS game-client settings\n  tsb --lan                   Use UDP in the room creation UI\n  tsb --host 0.0.0.0:28080     Legacy direct UDP host\n  tsb --join 192.168.0.2:28080 Join host\n  tsb --online-session FILE   Open a DotG host/join session\n  tsb --solo                  Solo practice / optional bots\n  tsb --smoke-test            Render for 14 seconds and exit\n  tsb --art-review            Capture all 15 animated models and exit\n  tsb --communication-review Capture wheel and 15 hello gestures\n  tsb --ghost-review          Capture ghost wall phasing and ripples\n  tsb --lobby-review          Capture lobby flow and exit\n  tsb --auto-start            Start host/practice immediately\n\nH character select / arrows + F5 confirm / F5 start\nWASD move / mouse aim / LMB attack / hold RMB zoom (Shion) / R reload / E skill / Q ultimate\nG dance / mouse orbit + wheel zoom while dancing\nC hold + mouse drag + release: communication wheel / RMB cancel\nENTER chat (including hero/lobby/map) / ENTER send / TAB team-match / ESC cancel / PgUp-PgDn history\nM microphone mute/unmute (starts muted, team voice)\nSPACE jump / SHIFT ghost phase (3s / 12s cooldown) / ELF SHIFT+LMB yellow portal, SHIFT+RMB blue portal (5s each) / hold CTRL crouch / TAB scoreboard / ESC settings menu / F10 quit");
        return Ok(());
    }
    let smoke = args.iter().any(|a| a == "--smoke-test").then_some(14.0);
    let auto_start = args.iter().any(|a| a == "--auto-start");
    let mut session = if let Some(index) = args.iter().position(|a| a == "--online-session") {
        let path = args
            .get(index + 1)
            .ok_or("--online-session needs a downloaded .tsb-session.json file")?;
        let online = OnlineSession::load(path)?;
        println!(
            "DotG player: {}. Transport: {}.",
            online.profile.display_name, online.transport
        );
        if online.mode == "host" {
            let mut server = if online.transport == "eos_p2p" {
                Server::host_eos(Some(&online.address.parse()?), true)?
            } else {
                Server::bind(online.bind_address(), true)?
            };
            server.restrict_to_invites(online.host_gate()?, online.profile);
            Session::Host(Box::new(server))
        } else {
            Session::Join(Box::new(
                Client::connect_address(&online.address)?
                    .with_invitation(online.ticket.ok_or("Invitation missing")?)?,
            ))
        }
    } else if let Some(index) = args.iter().position(|a| a == "--join") {
        let address = args.get(index + 1).ok_or("--join needs an IP:port")?;
        Session::Join(Box::new(Client::connect_address(address)?))
    } else if args.iter().any(|a| a == "--eos-host") {
        Session::Host(Box::new(Server::host_eos(None, true)?))
    } else if args.iter().any(|a| {
        a == "--solo"
            || a == "--art-review"
            || a == "--selection-review"
            || a == "--cheek-review"
            || a == "--communication-review"
            || a == "--hud-review"
            || a == "--phase-review"
            || a == "--weapon-review"
            || a == "--greeting-review"
            || a == "--attack-review"
            || a == "--naia-skill-review"
            || a == "--portal-review"
            || a == "--impact-review"
            || a == "--deathcam-review"
            || a == "--home-review"
            || a == "--lobby-review"
            || a == "--map-review"
            || a == "--ghost-review"
            || a == "--shield-review"
            || a == "--shadow-review"
            || a == "--settings-review"
            || a == "--round-results-review"
            || a == "--objective-hud-review"
            || a == "--nameplates-review"
            || a == "--voice-review"
    }) || (!args.iter().any(|a| a == "--host") && (smoke.is_some() || !auto_start))
    {
        // Combat/art reviews keep their full fixture; regular play starts solo.
        let mut w = if smoke.is_some()
            || args.iter().any(|a| {
                a.ends_with("-review") && !matches!(a.as_str(), "--home-review" | "--lobby-review")
            }) {
            World::default()
        } else {
            World::room(Default::default(), true)
        };
        w.actors[0].human = true;
        Session::Solo(Box::new(w))
    } else {
        let addr = if let Some(i) = args.iter().position(|a| a == "--host") {
            args.get(i + 1).ok_or("--host needs an IP:port")?.as_str()
        } else {
            "0.0.0.0:28080"
        };
        Session::Host(Box::new(Server::bind(addr, true)?))
    };
    if let Some(i) = args.iter().position(|a| a == "--map") {
        let slug = args
            .get(i + 1)
            .ok_or("--map needs fairy-kingdom-castle or elf-city")?;
        let map = trickcal_shooting_balddagu::level::MapId::from_slug(slug)
            .ok_or("Unknown --map slug")?;
        match &mut session {
            Session::Solo(w) => {
                w.select_map(map);
            }
            Session::Host(s) => {
                s.world.select_map(map);
            }
            Session::Join(_) => {
                return Err("The host chooses the map; --map cannot be used with --join".into())
            }
        }
    }
    std::fs::create_dir_all("artifacts/logs")?;
    Log::set_file_name(format!("artifacts/logs/tsb-{}.log", std::process::id()));
    let mut executor = Executor::from_params(
        Some(EventLoop::new()?),
        GraphicsContextParams {
            window_attributes: WindowAttributes::default()
                .with_title("Trickcal : Shooting Balddagu | MVP")
                .with_inner_size(
                    if args
                        .iter()
                        .any(|a| a == "--home-review-wide" || a == "--lobby-review-wide")
                    {
                        Size::Physical(PhysicalSize::new(1920, 800))
                    } else if args.iter().any(|a| {
                        a == "--hud-review-small"
                            || a == "--home-review-small"
                            || a == "--lobby-review-small"
                    }) {
                        Size::Physical(PhysicalSize::new(960, 640))
                    } else {
                        Size::Logical(LogicalSize::new(1280.0, 800.0))
                    },
                )
                .with_min_inner_size(
                    if args.iter().any(|a| {
                        a == "--hud-review-small"
                            || a == "--home-review-small"
                            || a == "--home-review-wide"
                            || a == "--lobby-review-small"
                            || a == "--lobby-review-wide"
                            || a == "--startup-review"
                    }) {
                        Size::Physical(PhysicalSize::new(960, 640))
                    } else {
                        Size::Logical(LogicalSize::new(960.0, 640.0))
                    },
                ),
            ..Default::default()
        },
    );
    executor.add_plugin(Game::new(Runtime::new(session, smoke, auto_start)));
    executor.run();
    Ok(())
}
