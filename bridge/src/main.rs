pub mod args;
pub mod config;
pub mod errors;

use crate::args::{Args, SubCommand};
use crate::errors::*;
use btleplug::api::{
    bleuuid::uuid_from_u16, BDAddr, Central, CentralEvent, Characteristic, Manager as _,
    Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use clap::Parser;
use d3xs_protocol::crypto;
use d3xs_protocol::ipc;
use data_encoding::{BASE64, BASE64URL_NOPAD};
use env_logger::Env;
use futures_util::{SinkExt, StreamExt};
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::time;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use uuid::Uuid;

const SERVICE_UUID: Uuid = uuid_from_u16(0xFFFF);
const CHARACTERISTIC_UUID: Uuid = uuid_from_u16(0xAAAA);
// when working with a websocket, the timeout is much shorter to avoid hanging
const WS_BLE_TIMEOUT: u64 = 5;

async fn find_by_mac(central: &Adapter, mac: &BDAddr) -> Result<Option<Peripheral>> {
    for p in central.peripherals().await? {
        if p.address() == *mac {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

async fn try_solve_char(peripheral: Peripheral, characteristic: Characteristic) -> Result<()> {
    info!("Requesting challenge");
    let chall = peripheral.read(&characteristic).await?;
    println!("chall={chall:?}");

    if chall.is_empty() {
        bail!("Challenge can't be empty");
    }

    info!("Sending solution");
    peripheral
        .write(&characteristic, &chall, WriteType::WithoutResponse)
        .await?;

    Ok(())
}

async fn try_solve(peripheral: Peripheral) -> Result<()> {
    let mac = peripheral.address();

    info!("Connecting to peripheral (mac={mac:?})");
    peripheral.connect().await?;

    debug!("Discover services...");
    peripheral.discover_services().await?;

    info!("Enumerating characteristics...");
    let characteristic = peripheral
        .characteristics()
        .into_iter()
        .filter(|chr| chr.service_uuid == SERVICE_UUID)
        .find(|chr| chr.uuid == CHARACTERISTIC_UUID)
        .context("Failed to find service")?;
    debug!("Found characteristic with matching uuid: {characteristic:?}");

    try_solve_char(peripheral, characteristic).await
}

async fn try_open(central: &Adapter, mac: &BDAddr) -> Result<()> {
    let mut events = central.events().await?;
    central.start_scan(ScanFilter::default()).await?;

    while let Some(event) = events.next().await {
        trace!("Bluetooth event: {event:?}");
        if let CentralEvent::DeviceDiscovered(_) = event {
            if let Some(peripheral) = find_by_mac(central, mac)
                .await
                .context("Failed to enumerate peripherals")?
            {
                match try_solve(peripheral).await {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(err) => {
                        error!("Failed to solve challenge: {err:#}");
                    }
                }
            }
        }
    }

    bail!("Failed to open")
}

fn verify_solve<'a>(config: &'a config::Config, solve: &ipc::Solve) -> Result<&'a config::Door> {
    let user = solve
        .user
        .as_ref()
        .context("Solve data is missing `user` field")?;
    let user = config
        .users
        .get(user.as_str())
        .with_context(|| anyhow!("User is not known: {user:?}"))?;

    if user.authorize.iter().all(|d| *d != solve.door) {
        bail!("User is not authorized for door");
    }

    let door = config
        .doors
        .get(&solve.door)
        .with_context(|| anyhow!("Door is not known {:?}", solve.door))?;

    // TODO: nothing is verified yet

    Ok(door)
}

async fn ws_connect(url: &str, config: &config::Config) -> Result<()> {
    let ipc = config.to_shared_config()?;

    debug!("Connecting to {url:?}...");
    let (mut ws_stream, _) = connect_async(url)
        .await
        .with_context(|| anyhow!("Failed to connect to {url:?}"))?;

    debug!("Connected, sending configuration...");
    let ipc = serde_json::to_string(&ipc)?;
    ws_stream.send(Message::Text(ipc)).await?;

    info!("Connection established, waiting for events...");
    while let Some(msg) = ws_stream.next().await {
        let Message::Text(text) = msg? else { continue };
        let solve = serde_json::from_str::<ipc::Solve>(&text)?;
        debug!("Received solve attempt: {solve:?}");

        info!("solve={solve:?}");
        if let Ok(door) = verify_solve(config, &solve) {
            info!(
                "Challenge successfully solved (user={:?}, door={door:?})",
                solve.user.as_ref().unwrap()
            );
            if let (Some(mac), Some(_public_key)) = (&door.mac, &door.public_key) {
                if let Err(err) = ble_open(mac, WS_BLE_TIMEOUT).await {
                    error!("Failed to open door: {err:#}");
                } else {
                    info!("Successfully opened door");
                }
            }
        }
    }

    Ok(())
}

async fn ble_open(mac: &str, timeout: u64) -> Result<()> {
    let mac = BDAddr::from_str_delim(mac)?;
    let manager = Manager::new().await.unwrap();

    let adapters = manager.adapters().await?;
    let central = adapters
        .into_iter()
        .next()
        .context("No bluetooth adapters found")?;

    let future = try_open(&central, &mac);

    if timeout == 0 {
        future.await?;
    } else {
        let timeout = time::Duration::from_secs(timeout);
        time::timeout(timeout, future)
            .await
            .context("Operation has timed out")?
            .context("Operation has failed")?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = match args.verbose {
        0 => "info",
        _ => "debug",
    };
    env_logger::init_from_env(Env::default().default_filter_or(log_level));

    match args.subcommand {
        SubCommand::Open(open) => ble_open(&open.mac, open.timeout).await?,
        SubCommand::Connect(connect) => {
            let config = config::Config::load_from_path(connect.config).await?;

            loop {
                if let Err(err) = ws_connect(&connect.url, &config).await {
                    error!("Websocket error: {err:#}");
                }
                time::sleep(time::Duration::from_secs(3)).await;
                info!("Reconnecting...");
            }
        }
        SubCommand::Keygen(keygen) => {
            let secret_key = if keygen.stdin {
                let mut stdin = io::stdin();
                let mut buf = Vec::new();
                stdin.read_to_end(&mut buf).await?;
                let buf = buf.strip_suffix(b"\n").unwrap_or(&buf);

                let buf = BASE64.decode(buf)?;
                if buf.len() != crypto::CRYPTO_SECRET_KEY_SIZE {
                    bail!("Unexpected length for secret key: {}", buf.len());
                }

                let mut key = [0u8; crypto::CRYPTO_SECRET_KEY_SIZE];
                key.copy_from_slice(&buf);
                crypto::SecretKey::from(key)
            } else {
                crypto::generate_secret_key::<crypto::Random>()
            };
            let public_key = secret_key.public_key();

            let secret_key = BASE64.encode(&secret_key.to_bytes());
            let path = BASE64URL_NOPAD.encode(public_key.as_bytes());
            let public_key = BASE64.encode(public_key.as_bytes());

            let url = keygen.url.as_deref().unwrap_or("https://example.com");
            let url: &str = url.strip_suffix('/').unwrap_or(url);

            if keygen.bridge {
                println!("[system]");
                println!("# public_key = {public_key:?}");
                println!("secret_key = {secret_key:?}");
            } else if keygen.firmware {
                println!("# {public_key}");
                println!("D3XS_DOOR_KEY={secret_key:?}");
            } else {
                let name = keygen.name.as_deref();
                let path = if keygen.name_as_path {
                    name.context("Name was not provided, can't use as path")?
                } else {
                    &path
                };
                println!("[users.{}]", name.unwrap_or("alice"));
                println!("# {url}/{path}#{secret_key}");
                println!("public_key = {public_key:?}");
                println!("authorize = []");
            }
        }
    }

    Ok(())
}
