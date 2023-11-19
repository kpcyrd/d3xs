use crate::crypto;
use data_encoding::{BASE64, HEXLOWER_PERMISSIVE as HEX};
use wasm_bindgen::prelude::*;

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

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

pub fn get_document() -> Option<web_sys::Document> {
    let window = web_sys::window()?;
    let document = window.document()?;
    Some(document)
}

// this does technically not belong into the protocol crate, but I had trouble passing the string through js (patches welcome)
pub fn read_key_from_location() -> Option<crypto::SecretKey> {
    let document = get_document()?;
    let location = document.location()?;
    let hash = location.hash().ok()?;
    let key = hash.strip_prefix("#").unwrap_or(&hash);
    if key.is_empty() {
        return None;
    };

    let secret_key = crypto::secret_key(&key).ok()?;
    Some(secret_key)
}

pub fn read_public_key_from_html() -> Option<crypto::PublicKey> {
    let document = get_document()?;
    let elem = document
        .get_element_by_id("public_key")?
        .dyn_into::<web_sys::HtmlInputElement>()
        .ok()?;
    let public_key = elem.value();
    let public_key = crypto::public_key(&public_key).ok()?;
    Some(public_key)
}

pub fn read_challenge_from_html() -> Option<String> {
    let document = get_document()?;
    let elem = document
        .get_element_by_id("challenge")?
        .dyn_into::<web_sys::HtmlInputElement>()
        .ok()?;
    Some(elem.value())
}

pub fn write_solution_to_html(response: &str) -> Option<()> {
    let document = get_document()?;
    let elem = document
        .get_element_by_id("response")?
        .dyn_into::<web_sys::HtmlInputElement>()
        .ok()?;
    console_log!("Wrote response: {response:?}");
    elem.set_value(response);
    Some(())
}

pub fn solve_challenge_to_html(secret_key: &crypto::SecretKey) -> Option<String> {
    let public_key = read_public_key_from_html()?;

    let challenge = read_challenge_from_html()?;
    let challenge = if let Ok(challenge) = BASE64.decode(challenge.as_bytes()) {
        challenge
    } else if let Some(challenge) = challenge.strip_prefix("0x") {
        HEX.decode(challenge.as_bytes()).ok()?
    } else {
        return None;
    };

    let salsa = crypto::SalsaBox::new(&public_key, secret_key);

    let mut decrypted = [0u8; 4096];
    let decrypted = crypto::decrypt(&salsa, &challenge, &mut decrypted).ok()?;

    let response = BASE64.encode(&decrypted);
    write_solution_to_html(&response)?;

    Some(response)
}

#[wasm_bindgen]
pub fn validate_key() -> bool {
    read_key_from_location().is_some()
}

#[wasm_bindgen]
pub fn solve_challenge() -> bool {
    let Some(key) = read_key_from_location() else {
        return false;
    };
    solve_challenge_to_html(&key).is_some()
}
