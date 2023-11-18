use std::env;

pub const SCRIPT_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/script.js"));
pub const STYLE_CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/style.css"));
pub const WASM_BINDGEN: &str = include_str!(concat!(env!("OUT_DIR"), "/wasm-bindgen.js"));
pub const WASM: &[u8] = include_bytes!("../protocol/pkg/d3xs_protocol_bg.wasm");

pub fn script_js_name() -> &'static str {
    if DEBUG_MODE || env::var("D3XS_PATCH_JS_FILE").is_ok() {
        "script.js"
    } else {
        SCRIPT_JS_NAME
    }
}

pub fn style_css_name() -> &'static str {
    if DEBUG_MODE || env::var("D3XS_PATCH_CSS_FILE").is_ok() {
        "style.css"
    } else {
        STYLE_CSS_NAME
    }
}

pub fn wasm_bindgen_name() -> &'static str {
    if DEBUG_MODE || env::var("D3XS_PATCH_WASM_BINDGEN_FILE").is_ok() {
        "wasm-bindgen.js"
    } else {
        WASM_BINDGEN_NAME
    }
}

pub fn wasm_name() -> &'static str {
    if DEBUG_MODE || env::var("D3XS_PATCH_WASM_FILE").is_ok() {
        "d3xs.wasm"
    } else {
        WASM_NAME
    }
}

include!(concat!(env!("OUT_DIR"), "/consts.rs"));
