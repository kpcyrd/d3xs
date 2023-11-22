use crate::errors::*;
use crate::ws;
use d3xs_protocol::ipc;
use futures_util::{FutureExt, SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tokio::time;
use warp::http::StatusCode;
use warp::ws::Message;
use warp::ws::WebSocket;

async fn ws_connect(
    mut ws: WebSocket,
    config: Arc<RwLock<Option<ipc::Config>>>,
    event_tx: broadcast::Sender<ipc::Event>,
    mut request_rx: broadcast::Receiver<ipc::ClientRequest>,
) -> Result<()> {
    let mut ping = time::interval(ws::WS_PING_INTERVAL);

    loop {
        tokio::select! {
            // ping clients at interval
            _ = ping.tick() => ws.send(Message::ping(vec![])).await?,
            // forward all messages from websocket clients to bridge
            msg = request_rx.recv() => if let Ok(msg) = msg {
                let data = serde_json::to_string(&msg)?;
                ws.send(Message::text(data)).await?;
            } else {
                return Ok(());
            },
            // receive messages from bridge (config updates and challenges)
            msg = ws.next() => if let Some(msg) = msg {
                let Ok(msg) = msg else { continue };
                let Ok(msg) = msg.to_str() else { continue };
                let Ok(event) = serde_json::from_str::<ipc::BridgeResponse>(msg) else {
                    warn!("bridge sent malformed json");
                    continue;
                };
                match event {
                    ipc::BridgeResponse::Config(data) => {
                        let mut config = config.write().await;
                        info!("Bridge has connected (public_key={:?})", data.public_key);
                        *config = Some(data);
                        event_tx.send(ipc::Event::Config).ok();
                    },
                    ipc::BridgeResponse::Challenge(chall) => {
                        // TODO: find a more efficient way than broadcasting the challenge to all connected clients
                        event_tx.send(ipc::Event::Challenge(chall)).ok();
                    }
                }
            } else {
                return Ok(());
            },
        }
    }
}

pub async fn websocket(
    uuid: Arc<String>,
    config: Arc<RwLock<Option<ipc::Config>>>,
    event_tx: broadcast::Sender<ipc::Event>,
    request_tx: broadcast::Sender<ipc::ClientRequest>,
    bridge: String,
    ws: warp::ws::Ws,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    if bridge != uuid.as_str() {
        return Ok(Box::new(StatusCode::NOT_FOUND));
    }
    debug!("Received bridge connection");

    let request_rx = request_tx.subscribe();
    let reply = ws.on_upgrade(move |websocket| {
        ws_connect(websocket, config, event_tx, request_rx).map(|result| {
            if let Err(err) = result {
                error!("bridge websocket error: {err:#}");
            }
        })
    });
    Ok(Box::new(reply))
}
