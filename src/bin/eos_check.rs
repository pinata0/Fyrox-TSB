use std::{env, process::ExitCode, time::Duration};
use trickcal_shooting_balddagu::eos::{EosClient, EosConfig, Sdk};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mut config = None;
    let mut force_relay = false;
    let mut offline = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" if config.is_none() => {
                config = Some(args.next().ok_or("--config requires a settings file")?)
            }
            "--force-relay" => force_relay = true,
            "--offline" => offline = true,
            "--help" | "-h" => {
                println!("tsb-eos-check [--offline]\ntsb-eos-check --config eos.local.json [--force-relay]\n\nOffline: validate SDK loading and initialization only (default).\nConfig: connect to EOS using a local Device ID and query NAT.\nThis does not verify a peer connection or a DotG account.");
                return Ok(());
            }
            _ => return Err("Unknown or repeated option; use --help".into()),
        }
    }
    if offline && config.is_some() {
        return Err("Choose --offline or --config, not both".into());
    }
    if config.is_none() {
        if force_relay {
            return Err("--force-relay requires --config".into());
        }
        let sdk = Sdk::initialize()?;
        println!("PASS: EOS SDK {} loaded and initialized.", sdk.version());
        println!(
            "Offline check only. Credentials, login and peer connectivity have not been tested."
        );
        return Ok(());
    }
    let config = EosConfig::load(config.unwrap())?;
    let mut client = EosClient::create(&config, force_relay)?;
    println!(
        "EOS SDK {} initialized. Checking Connect login...",
        client.version()
    );
    let user = client.login_device(Duration::from_secs(30))?;
    println!("PASS: EOS Device ID login. Product User ID: {user}");
    println!("Checking NAT type...");
    let nat = client.query_nat(Duration::from_secs(20))?;
    println!("PASS: NAT query: {nat}.");
    println!(
        "Relay policy: {}. No peer connection has been tested.",
        if force_relay {
            "force relays"
        } else {
            "allow relays"
        }
    );
    println!("Device ID is transport identity, not a DotG profile. Use tsb-eos-peer-check on two PCs to test gameplay over EOS.");
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("EOS check failed: {error}");
            ExitCode::FAILURE
        }
    }
}
