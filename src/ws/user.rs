use crate::errors::*;
use d3xs_protocol::ipc;
use futures_util::{FutureExt, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use warp::http::StatusCode;
use warp::ws::Message;
use warp::ws::WebSocket;

#[derive(Debug, Serialize, Deserialize)]
pub struct Door {
    pub id: String,
    pub label: String,
}

impl Door {
    pub fn new(id: String, config: ipc::Door) -> Self {
        Door {
            id,
            label: config.label,
        }
    }
}

async fn ws_connect(
    mut ws: WebSocket,
    user: String,
    config: Vec<Door>,
    tx: broadcast::Sender<ipc::Solve>,
) -> Result<()> {
    let json = serde_json::to_string(&config)?;
    ws.send(Message::text(json)).await?;
    let authorized = config.iter().map(|d| d.id.as_str()).collect::<HashSet<_>>();

    while let Some(msg) = ws.next().await {
        let msg = msg.context("Failed to read from websocket")?;
        let Ok(msg) = msg.to_str() else { continue };
        let Ok(mut solve) = serde_json::from_str::<ipc::Solve>(msg) else {
            continue;
        };
        debug!("Received solve attempt: {solve:?}");
        // don't observe the user provided value, always overwrite
        solve.user = Some(user.clone());

        let door = solve.door.as_str();
        if !authorized.contains(door) {
            warn!("Attempt to access unauthorized resource: {door:?}");
            continue;
        }

        info!("Sending solve attempt to bridge... (user={user:?}, door={door:?})");
        tx.send(solve).ok();
    }

    Ok(())
}

pub async fn websocket(
    config: Arc<RwLock<ipc::Config>>,
    tx: broadcast::Sender<ipc::Solve>,
    user: String,
    ws: warp::ws::Ws,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let authorized = {
        let config = config.read().await;

        let mut doors = config.doors.clone();
        let Some(config) = config.users.get(&user) else {
            return Ok(Box::new(StatusCode::NOT_FOUND));
        };

        let config = config.clone();
        debug!(
            "Received client connection: user={:?} config={:?}",
            user, config
        );

        let mut authorized = Vec::new();
        for auth in config.authorize {
            if let Some(door) = doors.remove(&auth) {
                authorized.push(Door::new(auth, door));
            }
        }

        authorized
    };

    let reply = ws.on_upgrade(move |websocket| {
        ws_connect(websocket, user, authorized, tx).map(|result| {
            if let Err(err) = result {
                info!("websocket error: {err:#}")
            }
        })
    });
    Ok(Box::new(reply))
}
