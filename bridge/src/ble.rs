use crate::errors::*;
use btleplug::api::{
    bleuuid::uuid_from_u16, BDAddr, Central, CentralEvent, Characteristic, Manager as _,
    Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use d3xs_protocol::crypto;
use futures_util::StreamExt;
use tokio::time;
use uuid::Uuid;

const SERVICE_UUID: Uuid = uuid_from_u16(0xFFFF);
const CHARACTERISTIC_UUID: Uuid = uuid_from_u16(0xAAAA);
const BLE_SOLVE_ATTEMPTS: u8 = 4;

async fn find_by_mac(central: &Adapter, mac: &BDAddr) -> Result<Option<Peripheral>> {
    for p in central.peripherals().await? {
        if p.address() == *mac {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

async fn try_solve_service(
    salsa: &crypto::SalsaBox,
    peripheral: Peripheral,
    characteristic: Characteristic,
) -> Result<()> {
    info!("Requesting challenge");
    let chall = peripheral.read(&characteristic).await?;
    println!("chall={chall:?}");

    if chall.is_empty() {
        bail!("Challenge can't be empty");
    }

    let mut decrypted = [0u8; 4096];
    let decrypted = crypto::decrypt(salsa, &chall, &mut decrypted)
        .map_err(|_| anyhow!("Failed to decrypt solution"))?;

    info!("Sending solution");
    peripheral
        .write(&characteristic, decrypted, WriteType::WithoutResponse)
        .await?;

    Ok(())
}

async fn try_solve(salsa: &crypto::SalsaBox, peripheral: Peripheral) -> Result<()> {
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

    try_solve_service(salsa, peripheral, characteristic).await
}

async fn try_open(salsa: &crypto::SalsaBox, central: &Adapter, mac: &BDAddr) -> Result<()> {
    let mut events = central.events().await?;
    central.start_scan(ScanFilter::default()).await?;

    let mut attempts = BLE_SOLVE_ATTEMPTS;
    while let Some(event) = events.next().await {
        trace!("Bluetooth event: {event:?}");
        if let CentralEvent::DeviceDiscovered(_) = event {
            if let Some(peripheral) = find_by_mac(central, mac)
                .await
                .context("Failed to enumerate peripherals")?
            {
                match try_solve(salsa, peripheral).await {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(err) => {
                        error!("Failed to solve challenge: {err:#}");
                        attempts -= 1;
                        if attempts == 0 {
                            bail!("Failed to open, too many failed attempts");
                        }
                    }
                }
            }
        }
    }

    bail!("Event stream disconnected")
}

pub async fn open(salsa: &crypto::SalsaBox, mac: &str, timeout: u64) -> Result<()> {
    let mac = BDAddr::from_str_delim(mac)?;
    let manager = Manager::new().await.unwrap();

    let adapters = manager.adapters().await?;
    let central = adapters
        .into_iter()
        .next()
        .context("No bluetooth adapters found")?;

    let future = try_open(salsa, &central, &mac);

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
