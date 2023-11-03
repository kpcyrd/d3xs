pub mod args;
pub mod errors;

use crate::args::{Args, SubCommand};
use crate::errors::*;
use btleplug::api::{
    bleuuid::uuid_from_u16, BDAddr, Central, CentralEvent, Characteristic, Manager as _,
    Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use clap::Parser;
use env_logger::Env;
use futures_util::StreamExt;
use tokio::time;
use uuid::Uuid;

const SERVICE_UUID: Uuid = uuid_from_u16(0xFFFF);
const CHARACTERISTIC_UUID: Uuid = uuid_from_u16(0xAAAA);

async fn find_by_mac(central: &Adapter, mac: &BDAddr) -> Result<Option<Peripheral>> {
    for p in central.peripherals().await? {
        if p.address() == *mac {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

async fn try_solve_char(
    _open: &args::Open,
    peripheral: Peripheral,
    characteristic: Characteristic,
) -> Result<()> {
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

async fn try_solve(open: &args::Open, peripheral: Peripheral) -> Result<()> {
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

    try_solve_char(open, peripheral, characteristic).await
}

async fn try_open(central: &Adapter, open: &args::Open, mac: &BDAddr) -> Result<()> {
    let mut events = central.events().await?;
    central.start_scan(ScanFilter::default()).await?;

    while let Some(event) = events.next().await {
        trace!("Bluetooth event: {event:?}");
        if let CentralEvent::DeviceDiscovered(_) = event {
            if let Some(peripheral) = find_by_mac(central, mac)
                .await
                .context("Failed to enumerate peripherals")?
            {
                match try_solve(open, peripheral).await {
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = match args.verbose {
        0 => "info",
        _ => "debug",
    };
    env_logger::init_from_env(Env::default().default_filter_or(log_level));

    match args.subcommand {
        SubCommand::Open(open) => {
            let mac = BDAddr::from_str_delim(&open.mac)?;
            let manager = Manager::new().await.unwrap();

            let adapters = manager.adapters().await?;
            let central = adapters
                .into_iter()
                .next()
                .context("No bluetooth adapters found")?;

            let future = try_open(&central, &open, &mac);

            if open.timeout == 0 {
                future.await?;
            } else {
                let timeout = time::Duration::from_secs(open.timeout);
                time::timeout(timeout, future)
                    .await
                    .context("Operation has timed out")?
                    .context("Operation has failed")?;
            }
        }
    }

    Ok(())
}
