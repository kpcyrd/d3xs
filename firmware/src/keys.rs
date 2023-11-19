use crate::crypto;

pub fn bridge_key() -> crypto::PublicKey {
    include!(concat!(env!("OUT_DIR"), "/public_key.rs"))
}

pub fn door_key() -> crypto::SecretKey {
    include!(concat!(env!("OUT_DIR"), "/secret_key.rs"))
}
