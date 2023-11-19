use d3xs_protocol::crypto;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn embed_public_key(path: &Path) {
    let public_key = if let Ok(bridge_key) = env::var("D3XS_BRIDGE_KEY") {
        crypto::public_key(&bridge_key).unwrap()
    } else {
        println!("cargo:warning=Missing D3XS_BRIDGE_KEY, using random key");
        let secret_key = crypto::generate_secret_key::<crypto::Random>();
        secret_key.public_key()
    };
    println!("cargo:rerun-if-env-changed=D3XS_BRIDGE_KEY");

    fs::write(
        path.join("public_key.rs"),
        format!("crypto::PublicKey::from({:?})", public_key.as_bytes()),
    )
    .unwrap();
}

fn embed_secret_key(path: &Path) {
    let secret_key = if let Ok(door_key) = env::var("D3XS_DOOR_KEY") {
        crypto::secret_key(&door_key).unwrap()
    } else {
        crypto::generate_secret_key::<crypto::Random>()
    };
    println!("cargo:rerun-if-env-changed=D3XS_DOOR_KEY");

    fs::write(
        path.join("secret_key.rs"),
        format!("crypto::SecretKey::from({:?})", secret_key.to_bytes()),
    )
    .unwrap();
}

fn main() {
    embuild::espidf::sysenv::output();

    let path = PathBuf::from(&env::var("OUT_DIR").unwrap());
    embed_secret_key(&path);
    embed_public_key(&path);
    println!("cargo:rerun-if-changed=build.rs");
}
