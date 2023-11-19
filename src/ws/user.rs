use crate::errors::*;
use d3xs_protocol::ipc;
use d3xs_protocol::ipc::Event;
use futures_util::{FutureExt, SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tokio::time;
use warp::http::StatusCode;
use warp::ws::Message;
use warp::ws::WebSocket;

pub struct Challenge {
    pub code: String,
    pub door: String,
    pub issued_at: time::Instant,
}

async fn ws_connect(
    mut ws: WebSocket,
    user: String,
    config: ipc::UiConfig,
    mut event_rx: broadcast::Receiver<ipc::Event>,
    request_tx: broadcast::Sender<ipc::Request>,
) -> Result<()> {
    let json = serde_json::to_string(&Event::Config(config.clone()))?;
    ws.send(Message::text(json)).await?;

    loop {
        tokio::select! {
            // subscribe to events from bridge
            msg = event_rx.recv() => if let Ok(msg) = msg {
                match &msg {
                    ipc::Event::Config(_) => (),
                    ipc::Event::Challenge(chall) => {
                        if chall.user != user {
                            continue;
                        }
                        let json = serde_json::to_string(&msg)?;
                        ws.send(Message::text(json)).await?;
                    }
                }
            } else {
                return Ok(());
            },
            // receive config updates from bridge
            msg = ws.next() => if let Some(msg) = msg {
                let msg = msg.context("Failed to read from websocket")?;
                let Ok(msg) = msg.to_str() else { continue };
                let Ok(mut req) = serde_json::from_str::<ipc::Request>(msg) else {
                    continue;
                };
                info!("Received request: {req:?}");
                match &mut req {
                    ipc::Request::Fetch(fetch) => fetch.user = Some(user.clone()),
                    ipc::Request::Solve(solve) => solve.user = Some(user.clone()),
                }
                request_tx.send(req).ok();
            }
        }
    }
}

pub async fn websocket(
    config: Arc<RwLock<Option<ipc::Config>>>,
    event_tx: broadcast::Sender<ipc::Event>,
    request_tx: broadcast::Sender<ipc::Request>,
    user: String,
    ws: warp::ws::Ws,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let ui = {
        let config = config.read().await;
        let Some(config) = config.as_ref() else {
            return Ok(Box::new(StatusCode::NOT_FOUND));
        };

        let mut doors = config.doors.clone();
        let Some(userdata) = config.users.get(&user) else {
            return Ok(Box::new(StatusCode::NOT_FOUND));
        };

        let userdata = userdata.clone();
        debug!(
            "Received client connection: user={:?} config={:?}",
            user, userdata
        );

        let mut authorized = Vec::new();
        for auth in userdata.authorize {
            if let Some(door) = doors.remove(&auth) {
                authorized.push(ipc::UiDoor::new(auth, door));
            }
        }

        ipc::UiConfig {
            public_key: config.public_key.clone(),
            doors: authorized,
        }
    };

    let event_rx = event_tx.subscribe();
    let reply = ws.on_upgrade(move |websocket| {
        ws_connect(websocket, user, ui, event_rx, request_tx).map(|result| {
            if let Err(err) = result {
                info!("websocket error: {err:#}")
            }
        })
    });
    Ok(Box::new(reply))
}
