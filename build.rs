use css_minify::optimizations::{Level, Minifier};
use minify_js::{minify, Session, TopLevelMode};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::env;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

const ASSET_ID_LEN: usize = 8;

fn id(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    hex::encode(&result[..ASSET_ID_LEN])
}

fn minify_js(script: &[u8]) -> Vec<u8> {
    let session = Session::new();
    let mut out = Vec::new();
    minify(&session, TopLevelMode::Module, script, &mut out).unwrap();
    out
}

fn js(consts: &mut String, path: &Path, debug_mode: bool) {
    let path = path.join("script.js");

    let script = include_str!("src/script.js");
    let out = if debug_mode {
        Cow::Borrowed(script.as_bytes())
    } else {
        let out = minify_js(script.as_bytes());
        Cow::Owned(out)
    };
    let filename = id(&out) + ".js";
    writeln!(consts, r#"const SCRIPT_JS_NAME: &str = "{}";"#, filename).unwrap();
    fs::write(path, out).unwrap();
}

fn css(consts: &mut String, path: &Path, debug_mode: bool) {
    let path = path.join("style.css");

    let style = include_str!("src/style.css");
    let out = if debug_mode {
        Cow::Borrowed(style.as_bytes())
    } else {
        let out = Minifier::default().minify(style, Level::Three).unwrap();
        let out = out.into_bytes();
        Cow::Owned(out)
    };
    let filename = id(&out) + ".css";
    writeln!(consts, r#"const STYLE_CSS_NAME: &str = "{}";"#, filename).unwrap();
    fs::write(path, out).unwrap();
}

fn wasm(consts: &mut String, path: &Path, debug_mode: bool) {
    let path = path.join("wasm-bindgen.js");

    let script = fs::read("protocol/pkg/d3xs_protocol.js").unwrap();
    let out = if debug_mode {
        Cow::Borrowed(&script)
    } else {
        let out = minify_js(&script);
        Cow::Owned(out)
    };
    let filename = id(&out) + ".js";
    writeln!(consts, r#"const WASM_BINDGEN_NAME: &str = "{}";"#, filename).unwrap();
    fs::write(path, out.as_ref()).unwrap();

    let wasm = fs::read("protocol/pkg/d3xs_protocol_bg.wasm").unwrap();
    let filename = id(&wasm) + ".wasm";
    writeln!(consts, r#"const WASM_NAME: &str = "{}";"#, filename).unwrap();
}

fn write_consts(consts: &mut String, path: &Path, debug_mode: bool) {
    let path = path.join("consts.rs");

    let debug_mode = if debug_mode { "true" } else { "false" };
    writeln!(consts, "pub const DEBUG_MODE: bool = {};", debug_mode).unwrap();
    fs::write(path, consts.as_bytes()).unwrap();
}

fn main() {
    let debug_mode = env::var("NO_MINIFY").is_ok();
    let path = PathBuf::from(&env::var("OUT_DIR").unwrap());

    let mut consts = String::new();
    js(&mut consts, &path, debug_mode);
    css(&mut consts, &path, debug_mode);
    wasm(&mut consts, &path, debug_mode);
    write_consts(&mut consts, &path, debug_mode);

    println!("cargo:rerun-if-env-changed=NO_MINIFY");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/script.js");
    println!("cargo:rerun-if-changed=src/style.css");
    println!("cargo:rerun-if-changed=src/style.css");
    println!("cargo:rerun-if-changed=protocol/pkg/d3xs_protocol.js");
    println!("cargo:rerun-if-changed=protocol/pkg/d3xs_protocol_bg.wasm");
}
