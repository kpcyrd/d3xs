use css_minify::optimizations::{Level, Minifier};
use minify_js::{minify, Session, TopLevelMode};
use sha2::{Digest, Sha256};
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

fn js(consts: &mut String, path: &Path, debug_mode: bool) {
    let path = path.join("script.js");

    let script = include_str!("src/script.js");
    let (out, filename) = if debug_mode {
        (script.as_bytes().to_vec(), "script.js".to_string())
    } else {
        let session = Session::new();
        let mut out = Vec::new();
        minify(&session, TopLevelMode::Global, script.as_bytes(), &mut out).unwrap();

        let filename = if debug_mode {
            "script.js".to_string()
        } else {
            id(&out) + ".js"
        };
        (out, filename)
    };
    writeln!(
        consts,
        r#"pub const SCRIPT_JS_NAME: &str = "{}";"#,
        filename
    )
    .unwrap();
    fs::write(path, out).unwrap();
}

fn css(consts: &mut String, path: &Path, debug_mode: bool) {
    let path = path.join("style.css");

    let style = include_str!("src/style.css");
    let (out, filename) = if debug_mode {
        (style.as_bytes().to_vec(), "style.css".to_string())
    } else {
        let out = Minifier::default().minify(style, Level::Three).unwrap();
        let out = out.into_bytes();

        let filename = if debug_mode {
            "style.css".to_string()
        } else {
            id(&out) + ".css"
        };

        (out, filename)
    };
    writeln!(
        consts,
        r#"pub const STYLE_CSS_NAME: &str = "{}";"#,
        filename
    )
    .unwrap();
    fs::write(path, out).unwrap();
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
    write_consts(&mut consts, &path, debug_mode);

    println!("cargo:rerun-if-env-changed=NO_MINIFY");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/script.js");
    println!("cargo:rerun-if-changed=src/style.css");
}
