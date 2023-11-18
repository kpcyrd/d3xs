use crate::crypto;
use data_encoding::BASE64;
use wasm_bindgen::prelude::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    if crypto::test_sodium_crypto::<crypto::Random>().is_ok() {
        alert("crypto test has passed");
    } else {
        alert("crypto test has failed");
    }
}

#[wasm_bindgen]
pub fn greet2(value: JsValue) {
    alert(&format!("{value:?}"));
}

// this does technically not belong into the protocol crate, but I had trouble passing the string through js (patches welcome)
pub fn read_key_from_location() -> Option<String> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let location = document.location()?;
    let hash = location.hash().ok()?;
    let key = hash.strip_prefix("#").unwrap_or(&hash);
    if !key.is_empty() {
        Some(key.to_string())
    } else {
        None
    }
}

#[wasm_bindgen]
pub fn validate_key() -> bool {
    let Some(key) = read_key_from_location() else {
        return false;
    };
    let Ok(bytes) = BASE64.decode(key.as_bytes()) else {
        return false;
    };
    bytes.len() == crypto::CRYPTO_SECRET_KEY_SIZE
}
