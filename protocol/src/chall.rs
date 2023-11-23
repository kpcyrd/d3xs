use crate::crypto;
use crate::errors::*;
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;

const RING_BUFFER_SIZE: usize = 4;
const CHALL_SIZE: usize = 32;
const CHALL_ENCRYPTED_SIZE: usize =
    CHALL_SIZE + crypto::CRYPTO_NONCE_SIZE + crypto::CRYPTO_TAG_SIZE;
const SHA3_SIZE: usize = 32;

fn hash(bytes: &[u8], dest: &mut [u8; SHA3_SIZE]) {
    let mut hasher = Sha3_256::new();
    hasher.update(bytes);
    hasher.finalize_into(dest.into());
}

pub struct Challenge {
    // store this as sha256 so the attacker has less control over inputs of the compare
    code: [u8; SHA3_SIZE],
    pub encrypted: [u8; CHALL_ENCRYPTED_SIZE],
}

impl Challenge {
    pub fn generate<R: crypto::Rng>(salsa: &crypto::SalsaBox) -> Result<Self> {
        let mut chall = [0u8; CHALL_SIZE];
        R::getrandom(&mut chall);

        let mut encrypted = [0u8; CHALL_ENCRYPTED_SIZE];
        crypto::encrypt::<R>(salsa, &chall, &mut encrypted)?;

        let mut code = [0u8; SHA3_SIZE];
        hash(&chall, &mut code);

        Ok(Challenge { code, encrypted })
    }

    pub fn verify(&self, code: &[u8]) -> Result<&Self> {
        let mut buf = [0u8; SHA3_SIZE];
        hash(code, &mut buf);
        if self.code == buf {
            Ok(self)
        } else {
            Err(Error::InvalidChallengeReponse)
        }
    }
}

#[derive(Default)]
pub struct RingBuffer {
    challenges: [Option<Challenge>; RING_BUFFER_SIZE],
    cursor: usize,
}

impl RingBuffer {
    pub fn new() -> RingBuffer {
        RingBuffer::default()
    }

    pub fn current(&self) -> Option<&Challenge> {
        self.challenges[self.cursor].as_ref()
    }

    pub fn generate_next<R: crypto::Rng>(
        &mut self,
        salsa: &crypto::SalsaBox,
    ) -> Result<&Challenge> {
        if self.challenges.len() - 1 == self.cursor {
            self.cursor = 0;
        } else {
            self.cursor += 1;
        }

        match Challenge::generate::<R>(salsa) {
            Ok(chall) => {
                let chall = self.challenges[self.cursor].insert(chall);
                Ok(chall)
            }
            Err(err) => {
                self.challenges[self.cursor] = None;
                Err(err)
            }
        }
    }

    pub fn verify(&self, secret: &[u8]) -> Result<()> {
        for chall in self.challenges.iter().flatten() {
            if chall.verify(secret).is_ok() {
                return Ok(());
            }
        }

        Err(Error::AuthError)
    }

    pub fn reset(&mut self) {
        *self = RingBuffer::default()
    }
}

#[derive(Default)]
pub struct UserDoorMap {
    map: HashMap<(String, String), RingBuffer>,
}

impl UserDoorMap {
    pub fn generate_next<R: crypto::Rng>(
        &mut self,
        user: String,
        door: String,
        salsa: &crypto::SalsaBox,
    ) -> Result<&Challenge> {
        let ring = self.map.entry((user, door)).or_default();
        ring.generate_next::<R>(salsa)
    }

    pub fn verify(&self, user: String, door: String, secret: &[u8]) -> Result<String> {
        if let Some(ring) = self.map.get(&(user, door.clone())) {
            ring.verify(secret)?;
            Ok(door)
        } else {
            Err(Error::AuthError)
        }
    }

    pub fn reset(&mut self, user: String, door: String) {
        self.map.entry((user, door)).or_default().reset();
    }
}
