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
    config: Arc<RwLock<Option<ipc::Config>>>,
    event_tx: broadcast::Sender<ipc::Event>,
    mut request_rx: broadcast::Receiver<ipc::Request>,
) -> Result<()> {
    loop {
        tokio::select! {
            // subscribe to solve attempts and forward to bridge
            msg = request_rx.recv() => if let Ok(msg) = msg {
                let data = serde_json::to_string(&msg)?;
                ws.send(Message::text(data)).await?;
            } else {
                return Ok(());
            },
            // receive config updates from bridge
            msg = ws.next() => if let Some(msg) = msg {
                if let Ok(msg) = msg?.to_str() {
                    let event = serde_json::from_str::<ipc::BridgeEvent>(msg)?;
                    match event {
                        ipc::BridgeEvent::Config(data) => {
                            let mut config = config.write().await;
                            info!("Bridge has connected (public_key={:?})", data.public_key);
                            *config = Some(data);

                            // TODO: notify connected clients
                            // event_tx.send(ipc::Event::Config(data));
                        },
                        ipc::BridgeEvent::Challenge(chall) => {
                            // TODO: find a more efficient way than broadcasting the challenge to all connected clients
                            event_tx.send(ipc::Event::Challenge(chall)).ok();
                        }
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
    request_tx: broadcast::Sender<ipc::Request>,
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
                info!("websocket error: {err:#}")
            }
        })
    });
    Ok(Box::new(reply))
}
