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

#[wasm_bindgen]
pub fn validate_key(key: JsValue) -> bool {
    alert(&format!("{key:?}"));
    let Some(key) = key.as_string() else {
        return false;
    };
    alert(&format!("{key:?}"));
    let Ok(bytes) = BASE64.decode(key.as_bytes()) else {
        return false;
    };
    alert(&format!("{bytes:?}"));
    bytes.len() == crypto::CRYPTO_SECRET_KEY_SIZE
}
