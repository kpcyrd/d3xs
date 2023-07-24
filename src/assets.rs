pub const SCRIPT_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/script.js"));
pub const STYLE_CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/style.css"));

include!(concat!(env!("OUT_DIR"), "/consts.rs"));
