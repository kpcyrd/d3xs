pub const SCRIPT_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/script.js"));
pub const STYLE_CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/style.css"));
pub const WASM_BINDGEN: &str = include_str!(concat!(env!("OUT_DIR"), "/wasm-bindgen.js"));
pub const WASM: &[u8] = include_bytes!("../protocol/pkg/d3xs_protocol_bg.wasm");

include!(concat!(env!("OUT_DIR"), "/consts.rs"));
