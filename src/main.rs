pub mod args;
pub mod assets;
pub mod errors;
pub mod ws;

use crate::args::Args;
use crate::errors::*;
use clap::Parser;
use d3xs_protocol::ipc;
use env_logger::Env;
use handlebars::Handlebars;
use serde_json::json;
use std::borrow::Cow;
use std::env;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use warp::{http::Response, http::StatusCode, Filter};

async fn resolve_asset<'a>(
    default: &'static [u8],
    content_type: &str,
    env_var: &str,
) -> Result<Box<dyn warp::Reply + 'static>> {
    let mut reply = Response::builder();
    reply = reply.header("content-type", content_type);
    let content = if let Ok(path) = env::var(env_var) {
        match fs::read(path).await {
            Ok(content) => Cow::Owned(content),
            Err(err) => {
                error!("Failed to read file: {err:#}");
                return Err(err.into());
            }
        }
    } else {
        if !assets::DEBUG_MODE {
            reply = reply.header("cache-control", "immutable");
        }
        Cow::Borrowed(default)
    };
    let reply = reply.body(content).unwrap();
    Ok(Box::new(reply))
}

async fn show_script() -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let Ok(reply) = resolve_asset(
        assets::SCRIPT_JS.as_bytes(),
        "text/javascript",
        "D3XS_PATCH_JS_FILE",
    )
    .await
    else {
        return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
    };
    Ok(reply)
}

async fn show_style() -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let Ok(reply) = resolve_asset(
        assets::STYLE_CSS.as_bytes(),
        "text/css",
        "D3XS_PATCH_CSS_FILE",
    )
    .await
    else {
        return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
    };
    Ok(reply)
}

async fn show_wasm_bindgen() -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let Ok(reply) = resolve_asset(
        assets::WASM_BINDGEN.as_bytes(),
        "text/javascript",
        "D3XS_PATCH_WASM_BINDGEN_FILE",
    )
    .await
    else {
        return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
    };
    Ok(reply)
}

async fn show_wasm() -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let Ok(reply) = resolve_asset(assets::WASM, "application/wasm", "D3XS_PATCH_WASM_FILE").await
    else {
        return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
    };
    Ok(reply)
}

async fn show_page(
    config: Arc<RwLock<ipc::Config>>,
    hb: Arc<Handlebars<'_>>,
    user: String,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let config = config.read().await;

    let Some(_config) = config.users.get(&user) else {
        return Ok(Box::new(StatusCode::NOT_FOUND));
    };
    let html = match hb.render(
        "index.html",
        &json!({
            "script_name": assets::script_js_name(),
            "style_name": assets::style_css_name(),
            "wasm_bindgen_name": assets::wasm_bindgen_name(),
            "wasm_name": assets::wasm_name(),
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = match args.verbose {
        0 => "info",
        _ => "debug",
    };
    env_logger::init_from_env(Env::default().default_filter_or(log_level));

    let config = Arc::new(RwLock::new(ipc::Config::default()));
    let config = warp::any().map(move || config.clone());

    let uuid = Arc::new(args.uuid);
    let uuid = warp::any().map(move || uuid.clone());

    let tx = {
        let (tx, _rx) = broadcast::channel(16);
        warp::any().map(move || tx.clone())
    };

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
        .and(warp::path(assets::script_js_name()))
        .and(warp::path::end())
        .and_then(show_script);
    let show_style = warp::get()
        .and(warp::path("assets"))
        .and(warp::path(assets::style_css_name()))
        .and(warp::path::end())
        .and_then(show_style);
    let show_wasm_bindgen = warp::get()
        .and(warp::path("assets"))
        .and(warp::path(assets::wasm_bindgen_name()))
        .and(warp::path::end())
        .and_then(show_wasm_bindgen);
    let show_wasm = warp::get()
        .and(warp::path("assets"))
        .and(warp::path(assets::wasm_name()))
        .and(warp::path::end())
        .and_then(show_wasm);
    let ws_user = warp::get()
        .and(config.clone())
        .and(tx.clone())
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::ws())
        .and_then(ws::user::websocket);
    let ws_bridge = warp::get()
        .and(uuid)
        .and(config)
        .and(tx)
        .and(warp::path("bridge"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::ws())
        .and_then(ws::bridge::websocket);

    let routes = warp::any().and(
        show_script
            .or(show_style)
            .or(show_wasm)
            .or(show_wasm_bindgen)
            .or(ws_user)
            .or(ws_bridge)
            .or(show_page),
    );

    warp::serve(routes).run(args.bind).await;

    Ok(())
}
