pub mod args;
pub mod assets;
pub mod config;
pub mod errors;

use crate::args::Args;
use crate::config::Config;
use crate::errors::*;
use clap::Parser;
use env_logger::Env;
use futures_util::{FutureExt, SinkExt, StreamExt};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use warp::ws::Message;
use warp::ws::WebSocket;
use warp::{http::Response, http::StatusCode, Filter};

async fn show_script() -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let mut reply = Response::builder();
    reply = reply.header("content-type", "text/javascript");
    if !assets::DEBUG_MODE {
        reply = reply.header("cache-control", "immutable");
    }
    let reply = reply.body(assets::SCRIPT_JS).unwrap();
    Ok(Box::new(reply))
}

async fn show_style() -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let mut reply = Response::builder();
    reply = reply.header("content-type", "text/css");
    if !assets::DEBUG_MODE {
        reply = reply.header("cache-control", "immutable");
    }
    let reply = reply.body(assets::STYLE_CSS).unwrap();
    Ok(Box::new(reply))
}

async fn show_page(
    config: Arc<Config>,
    hb: Arc<Handlebars<'_>>,
    user: String,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let Some(_config) = config.users.get(&user) else { return Ok(Box::new(StatusCode::NOT_FOUND)) };
    let html = match hb.render(
        "index.html",
        &json!({
            "script_name": assets::SCRIPT_JS_NAME,
            "style_name": assets::STYLE_CSS_NAME,
        }),
    ) {
        Ok(html) => html,
        Err(err) => {
            error!("Failed to render template: {err:#}");
            return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
        }
    };
    Ok(Box::new(warp::reply::html(html)))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Door {
    pub id: String,
    pub label: String,
}

impl Door {
    pub fn new(id: String, config: config::Door) -> Self {
        Door {
            id,
            label: config.label,
        }
    }
}

async fn ws_connect(mut ws: WebSocket, config: Vec<Door>) -> Result<()> {
    let json = serde_json::to_string(&config)?;
    ws.send(Message::text(json)).await?;
    let authorized = config.iter().map(|d| d.id.as_str()).collect::<HashSet<_>>();

    while let Some(msg) = ws.next().await {
        let msg = msg.context("Failed to read from websocket")?;
        let Ok(msg) = msg.to_str() else { continue };
        if !authorized.contains(msg) {
            warn!("Attempt to access unauthorized resource: {msg:?}");
            continue;
        }

        info!("msg={msg:?}");
    }

    Ok(())
}

async fn websocket(
    config: Arc<Config>,
    user: String,
    ws: warp::ws::Ws,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let mut doors = config.doors.clone();
    let Some(config) = config.users.get(&user) else { return Ok(Box::new(StatusCode::NOT_FOUND)) };
    let config = config.clone();
    debug!(
        "Received client connection: user={:?} config={:?}",
        user, config
    );

    let mut out = Vec::new();
    for auth in config.authorize {
        if let Some(door) = doors.remove(&auth) {
            out.push(Door::new(auth, door));
        }
    }

    let reply = ws.on_upgrade(move |websocket| {
        ws_connect(websocket, out).map(|result| {
            if let Err(err) = result {
                info!("websocket error: {err:#}")
            }
        })
    });
    Ok(Box::new(reply))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = match args.verbose {
        0 => "info",
        _ => "debug",
    };
    env_logger::init_from_env(Env::default().default_filter_or(log_level));

    let config = Arc::new(Config::load_from_path(&args.config).await?);
    let config = warp::any().map(move || config.clone());

    let mut hb = Handlebars::new();
    hb.register_template_string("index.html", include_str!("index.html"))
        .context("Failed to register template")?;
    let hb = Arc::new(hb);
    let hb = warp::any().map(move || hb.clone());

    let show_page = warp::get()
        .and(config.clone())
        .and(hb)
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then(show_page);
    let show_script = warp::get()
        .and(warp::path("assets"))
        .and(warp::path(assets::SCRIPT_JS_NAME))
        .and(warp::path::end())
        .and_then(show_script);
    let show_style = warp::get()
        .and(warp::path("assets"))
        .and(warp::path(assets::STYLE_CSS_NAME))
        .and(warp::path::end())
        .and_then(show_style);
    let websocket = warp::get()
        .and(config)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::ws())
        .and_then(websocket);

    let routes = warp::any().and(show_script.or(show_style).or(websocket).or(show_page));

    warp::serve(routes).run(args.bind).await;

    Ok(())
}
