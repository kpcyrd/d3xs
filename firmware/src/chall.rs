use crate::errors::*;
use d3xs_protocol::crypto;

const RING_BUFFER_SIZE: usize = 4;
const CHALL_SIZE: usize = 32;
const CHALL_ENCRYPTED_SIZE: usize =
    CHALL_SIZE + crypto::CRYPTO_NONCE_SIZE + crypto::CRYPTO_TAG_SIZE;

#[cfg(not(target_os = "espidf"))]
pub type Random = crypto::Random;

#[cfg(target_os = "espidf")]
pub struct Random;

#[cfg(target_os = "espidf")]
impl crypto::Rng for Random {
    fn getrandom(buf: &mut [u8]) {
        for i in 0..buf.len() {
            buf[i] = unsafe { esp_idf_svc::sys::random() } as u8;
        }
    }
}

pub struct Challenge {
    pub plain: [u8; CHALL_SIZE],
    pub encrypted: [u8; CHALL_ENCRYPTED_SIZE],
}

impl Default for Challenge {
    fn default() -> Challenge {
        Challenge {
            plain: [0u8; CHALL_SIZE],
            encrypted: [0u8; CHALL_ENCRYPTED_SIZE],
        }
    }
}

impl Challenge {
    pub fn generate<R: crypto::Rng>(salsa: &crypto::SalsaBox) -> Result<Challenge> {
        let mut chall = [0u8; CHALL_SIZE];
        R::getrandom(&mut chall);

        let mut encrypted = [0u8; CHALL_ENCRYPTED_SIZE];
        crypto::encrypt::<R>(salsa, &chall, &mut encrypted)?;

        Ok(Challenge {
            plain: chall,
            encrypted,
        })
    }

    pub fn verify(&self, salsa: &crypto::SalsaBox, encrypted: &[u8]) -> Result<()> {
        let mut chall = [0u8; CHALL_SIZE];
        crypto::decrypt(salsa, encrypted, &mut chall)?;
        Ok(())
    }
}

#[derive(Default)]
pub struct RingBuffer {
    challenges: [Option<Challenge>; RING_BUFFER_SIZE],
    cursor: usize,
}

impl RingBuffer {
    pub fn new<R: crypto::Rng>(salsa: &crypto::SalsaBox) -> RingBuffer {
        let mut ring = RingBuffer::default();
        ring.challenges[0] = Some(Challenge::generate::<R>(salsa).unwrap());
        ring
    }

    pub fn generate_next<R: crypto::Rng>(&mut self, salsa: &crypto::SalsaBox) -> &Challenge {
        if self.challenges.len() - 1 == self.cursor {
            self.cursor = 0;
        } else {
            self.cursor += 1;
        }
        self.challenges[self.cursor] = Some(Challenge::generate::<R>(salsa).unwrap());
        self.challenges[self.cursor].as_ref().unwrap()
    }

    pub fn verify(&self, salsa: &crypto::SalsaBox, secret: &[u8]) -> Result<()> {
        for chall in self.challenges.iter().flatten() {
            if chall.verify(salsa, secret).is_ok() {
                return Ok(());
            }
        }

        Err(Error::AuthError)
    }

    pub fn reset<R: crypto::Rng>(&mut self, salsa: &crypto::SalsaBox) {
        *self = RingBuffer::new::<R>(salsa)
    }
}
