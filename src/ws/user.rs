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

pub struct Challenge {
    pub code: String,
    pub door: String,
    pub issued_at: time::Instant,
}

fn generate_view(config: Option<&ipc::Config>, user: &str) -> Option<ipc::UiConfig> {
    let Some(config) = config.as_ref() else {
        return None;
    };

    let mut doors = config.doors.clone();
    let Some(userdata) = config.users.get(user) else {
        return None;
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

    Some(ipc::UiConfig {
        public_key: config.public_key.clone(),
        doors: authorized,
    })
}

async fn ws_connect(
    mut ws: WebSocket,
    config: Arc<RwLock<Option<ipc::Config>>>,
    user: String,
    view: ipc::UiConfig,
    mut event_rx: broadcast::Receiver<ipc::Event>,
    request_tx: broadcast::Sender<ipc::ClientRequest>,
) -> Result<()> {
    let json = serde_json::to_string(&ipc::ClientResponse::Config(view))?;
    ws.send(Message::text(json)).await?;

    let mut ping = time::interval(ws::WS_PING_INTERVAL);
    loop {
        tokio::select! {
            // ping clients at interval
            _ = ping.tick() => ws.send(Message::ping(vec![])).await?,
            // subscribe to events from bridge
            msg = event_rx.recv() => if let Ok(msg) = msg {
                match msg {
                    ipc::Event::Config => {
                        let config = config.read().await;
                        if let Some(ui) = generate_view(config.as_ref(), &user) {
                            let json = serde_json::to_string(&ipc::ClientResponse::Config(ui))?;
                            ws.send(Message::text(json)).await?;
                        } else {
                            // current user has been removed from config, disconnect them
                            return Ok(());
                        };
                    },
                    ipc::Event::Challenge(chall) => if chall.user == user {
                        let json = serde_json::to_string(&ipc::ClientResponse::Challenge(chall))?;
                        ws.send(Message::text(json)).await?;
                    }
                }
            } else {
                return Ok(());
            },
            // receive messages from websocket clients, forward to bridge
            msg = ws.next() => if let Some(msg) = msg {
                let Ok(msg) = msg else { continue };
                let Ok(msg) = msg.to_str() else { continue };
                let Ok(mut req) = serde_json::from_str::<ipc::ClientRequest>(msg) else {
                    warn!("websocket client sent invalid json");
                    continue;
                };
                debug!("Received request: {req:?}");
                match &mut req {
                    ipc::ClientRequest::Fetch(fetch) => fetch.user = Some(user.clone()),
                    ipc::ClientRequest::Solve(solve) => solve.user = Some(user.clone()),
                }
                request_tx.send(req).ok();
            } else {
                return Ok(());
            }
        }
    }
}

pub async fn websocket(
    config: Arc<RwLock<Option<ipc::Config>>>,
    event_tx: broadcast::Sender<ipc::Event>,
    request_tx: broadcast::Sender<ipc::ClientRequest>,
    user: String,
    ws: warp::ws::Ws,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let ui = {
        let config = config.read().await;
        let Some(view) = generate_view(config.as_ref(), &user) else {
            return Ok(Box::new(StatusCode::NOT_FOUND));
        };
        view
    };

    let event_rx = event_tx.subscribe();
    let reply = ws.on_upgrade(move |websocket| {
        ws_connect(websocket, config, user, ui, event_rx, request_tx).map(|result| {
            if let Err(err) = result {
                error!("client websocket error: {err:#}");
            }
        })
    });
    Ok(Box::new(reply))
}
