use crate::errors::*;
use d3xs_protocol::ipc;
use futures_util::{FutureExt, SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use warp::http::StatusCode;
use warp::ws::Message;
use warp::ws::WebSocket;

async fn ws_connect(
    mut ws: WebSocket,
    config: Arc<RwLock<ipc::Config>>,
    mut rx: broadcast::Receiver<ipc::Solve>,
) -> Result<()> {
    loop {
        tokio::select! {
            // subscribe to solve attempts and forward to bridge
            msg = rx.recv() => if let Ok(msg) = msg {
                let data = serde_json::to_string(&msg)?;
                ws.send(Message::text(data)).await?;
            } else {
                return Ok(());
            },
            // receive config updates from bridge
            msg = ws.next() => if let Some(msg) = msg {
                if let Ok(msg) = msg?.to_str() {
                    let data = serde_json::from_str(msg)?;
                    let mut config = config.write().await;
                    *config = data;
                    debug!("Updated in-memory configuration");
                }
            } else {
                return Ok(());
            },
        }
    }
}

pub async fn websocket(
    uuid: Arc<String>,
    config: Arc<RwLock<ipc::Config>>,
    tx: broadcast::Sender<ipc::Solve>,
    bridge: String,
    ws: warp::ws::Ws,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    if bridge != uuid.as_str() {
        return Ok(Box::new(StatusCode::NOT_FOUND));
    }
    debug!("Received bridge connection");

    let rx = tx.subscribe();
    let reply = ws.on_upgrade(move |websocket| {
        ws_connect(websocket, config, rx).map(|result| {
            if let Err(err) = result {
                info!("websocket error: {err:#}")
            }
        })
    });
    Ok(Box::new(reply))
}
