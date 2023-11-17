use crate::crypto;
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
