use crate::ble;
use crate::config;
use crate::errors::*;
use d3xs_protocol::chall;
use d3xs_protocol::crypto;
use d3xs_protocol::ipc;
use data_encoding::BASE64;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// when working with a websocket, the timeout is much shorter to avoid hanging
const WS_BLE_TIMEOUT: u64 = 5;

pub async fn connect(
    url: &str,
    config: &config::Config,
    challenges: &mut chall::UserDoorMap,
) -> Result<()> {
    let ipc = config.to_shared_config()?;
    let secret_key = crypto::secret_key(&config.system.secret_key)
        .map_err(|_| anyhow!("Failed to decode secret key :<"))?;

    debug!("Connecting to {url:?}...");
    let (mut ws_stream, _) = connect_async(url)
        .await
        .with_context(|| anyhow!("Failed to connect to {url:?}"))?;

    debug!("Connected, sending configuration...");
    let ipc = serde_json::to_string(&ipc::BridgeEvent::Config(ipc))?;
    ws_stream.send(Message::Text(ipc)).await?;

    info!("Connection established, waiting for events...");
    while let Some(msg) = ws_stream.next().await {
        let Message::Text(text) = msg? else { continue };
        let request = serde_json::from_str::<ipc::Request>(&text)?;

        match request {
            ipc::Request::Fetch(fetch) => {
                let Some(user) = fetch.user else { continue };
                let door = fetch.door;

                info!("Challenge has been requested (user={user:?}, door={door:?}");
                let userdata = config
                    .users
                    .get(&user)
                    .with_context(|| anyhow!("Failed to find user: {user:?}"))?;

                if userdata.authorize.iter().all(|d| *d != door) {
                    warn!("User is not authorized for door (user={user:?}, door={door:?}");
                    continue;
                }

                let public_key = crypto::public_key(&userdata.public_key)
                    .map_err(|_| anyhow!("Failed to decode public key"))?;

                let salsa = crypto::SalsaBox::new(&public_key, &secret_key);
                let chall = challenges.generate_next::<crypto::Random>(user.clone(), door, &salsa);

                let chall = ipc::Challenge {
                    user,
                    challenge: BASE64.encode(&chall.encrypted),
                };
                let json = serde_json::to_string(&ipc::Event::Challenge(chall))?;
                ws_stream.send(Message::text(json)).await?;
            }
            ipc::Request::Solve(solve) => {
                debug!("Received solve attempt: {solve:?}");
                let Some(user) = solve.user else { continue };
                let Ok(code) = BASE64.decode(solve.code.as_bytes()) else {
                    continue;
                };

                let userdata = config
                    .users
                    .get(&user)
                    .with_context(|| anyhow!("Failed to find user: {user:?}"))?;

                let public_key = crypto::public_key(&userdata.public_key)
                    .map_err(|_| anyhow!("Failed to decode public key"))?;

                if let Ok(door) = challenges.verify(user.clone(), solve.door.clone(), &code) {
                    info!("Challenge successfully solved (user={user:?}, door={door:?})",);
                    let salsa = crypto::SalsaBox::new(&public_key, &secret_key);
                    challenges.reset::<crypto::Random>(user.clone(), door.clone(), &salsa);

                    let door = config
                        .doors
                        .get(&door)
                        .with_context(|| anyhow!("Door is not known {door:?}"))?;

                    if let (Some(mac), Some(public_key)) = (&door.mac, &door.public_key) {
                        let public_key = crypto::public_key(public_key)
                            .map_err(|_| anyhow!("Failed to parse public key"))?;

                        let salsa = crypto::SalsaBox::new(&public_key, &secret_key);
                        if let Err(err) = ble::open(&salsa, mac, WS_BLE_TIMEOUT).await {
                            error!("Failed to open door: {err:#}");
                        } else {
                            info!("Successfully opened door");
                        }
                    }
                } else {
                    warn!(
                        "Solve attempt failed (user={user:?}, door={:?})",
                        solve.door
                    );
                }
            }
        }
    }

    Ok(())
}
