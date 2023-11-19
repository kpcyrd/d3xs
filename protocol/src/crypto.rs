use crate::errors::*;
use crypto_box::{aead::AeadInPlace, Nonce, Tag};
pub use crypto_box::{PublicKey, SalsaBox, SecretKey};
use data_encoding::BASE64;

pub const CRYPTO_TAG_SIZE: usize = 16;
pub const CRYPTO_NONCE_SIZE: usize = 24;
pub const CRYPTO_SECRET_KEY_SIZE: usize = 32;
pub const CRYPTO_PUBLIC_KEY_SIZE: usize = 32;

pub trait Rng {
    fn getrandom(buf: &mut [u8]);
}

pub struct Random;

#[cfg(not(target_os = "espidf"))]
impl Rng for Random {
    fn getrandom(buf: &mut [u8]) {
        getrandom::getrandom(buf).unwrap();
    }
}

pub fn encrypt<'a, R: Rng>(salsa: &SalsaBox, src: &[u8], dest: &'a mut [u8]) -> Result<&'a [u8]> {
    let buffer_size = dest.len();
    if buffer_size < src.len() + CRYPTO_NONCE_SIZE + CRYPTO_TAG_SIZE {
        return Err(Error::BufferLimit);
    }

    let length = {
        let (nonce, cursor) = dest.split_at_mut(CRYPTO_NONCE_SIZE);
        let nonce = {
            let mut buf = [0u8; CRYPTO_NONCE_SIZE];
            R::getrandom(&mut buf);
            nonce.copy_from_slice(&buf);
            Nonce::from(buf)
        };

        let (buf, cursor) = cursor.split_at_mut(src.len());
        buf.copy_from_slice(src);
        let tag = salsa.encrypt_in_place_detached(&nonce, &[], buf)?;

        let (buf, cursor) = cursor.split_at_mut(tag.len());
        buf.copy_from_slice(&tag);

        buffer_size - cursor.len()
    };

    Ok(&dest[..length])
}

pub fn decrypt<'a>(salsa: &SalsaBox, src: &[u8], dest: &'a mut [u8]) -> Result<&'a [u8]> {
    if src.len() < CRYPTO_NONCE_SIZE + CRYPTO_TAG_SIZE {
        return Err(Error::BufferLimit);
    }

    let dest = &mut dest[..src.len() - CRYPTO_NONCE_SIZE - CRYPTO_TAG_SIZE];

    let (nonce, cursor) = src.split_at(CRYPTO_NONCE_SIZE);
    let nonce = {
        let mut buf = [0u8; CRYPTO_NONCE_SIZE];
        buf.copy_from_slice(nonce);
        Nonce::from(buf)
    };

    let (cursor, tag) = cursor.split_at(cursor.len() - CRYPTO_TAG_SIZE);
    let tag = {
        let mut buf = [0u8; CRYPTO_TAG_SIZE];
        buf.copy_from_slice(tag);
        Tag::from(buf)
    };

    dest.copy_from_slice(cursor);
    salsa.decrypt_in_place_detached(&nonce, &[], dest, &tag)?;

    Ok(dest)
}

pub fn generate_secret_key<R: Rng>() -> SecretKey {
    let mut buf = [0u8; crypto_box::KEY_SIZE];
    R::getrandom(&mut buf);
    SecretKey::from_bytes(buf)
}

pub fn secret_key(bytes: &str) -> Result<SecretKey> {
    let bytes = BASE64.decode(bytes.as_bytes())?;
    if bytes.len() != CRYPTO_SECRET_KEY_SIZE {
        return Err(Error::InvalidKeyLength(bytes.len()));
    }

    let mut buf = [0u8; CRYPTO_SECRET_KEY_SIZE];
    buf.copy_from_slice(&bytes);

    Ok(SecretKey::from(buf))
}

pub fn public_key(bytes: &str) -> Result<PublicKey> {
    let bytes = BASE64.decode(bytes.as_bytes())?;
    if bytes.len() != CRYPTO_PUBLIC_KEY_SIZE {
        return Err(Error::InvalidKeyLength(bytes.len()));
    }

    let mut buf = [0u8; CRYPTO_PUBLIC_KEY_SIZE];
    buf.copy_from_slice(&bytes);

    Ok(PublicKey::from(buf))
}

pub fn test_sodium_crypto<R: Rng>() -> Result<()> {
    //
    // Encryption
    //

    // Generate a random secret key.
    // NOTE: The secret key bytes can be accessed by calling `secret_key.as_bytes()`
    let alice_secret_key = generate_secret_key::<R>();

    // Get the public key for the secret key we just generated
    let alice_public_key_bytes = *alice_secret_key.public_key().as_bytes();

    // Obtain your recipient's public key.
    let bob_public_key = PublicKey::from([
        0xe8, 0x98, 0xc, 0x86, 0xe0, 0x32, 0xf1, 0xeb, 0x29, 0x75, 0x5, 0x2e, 0x8d, 0x65, 0xbd,
        0xdd, 0x15, 0xc3, 0xb5, 0x96, 0x41, 0x17, 0x4e, 0xc9, 0x67, 0x8a, 0x53, 0x78, 0x9d, 0x92,
        0xc7, 0x54,
    ]);

    // Create a `SalsaBox` by performing Diffie-Hellman key agreement between
    // the two keys.
    let alice_box = SalsaBox::new(&bob_public_key, &alice_secret_key);

    // Message to encrypt
    let plaintext = b"Top secret message we're encrypting";

    // Encrypt the message using the box
    let mut ciphertext = [0u8; 4096];
    let ciphertext = encrypt::<R>(&alice_box, plaintext, &mut ciphertext)?;

    //
    // Decryption
    //

    // Either side can encrypt or decrypt messages under the Diffie-Hellman key
    // they agree upon. The example below shows Bob's side.
    let bob_secret_key = SecretKey::from([
        0xb5, 0x81, 0xfb, 0x5a, 0xe1, 0x82, 0xa1, 0x6f, 0x60, 0x3f, 0x39, 0x27, 0xd, 0x4e, 0x3b,
        0x95, 0xbc, 0x0, 0x83, 0x10, 0xb7, 0x27, 0xa1, 0x1d, 0xd4, 0xe7, 0x84, 0xa0, 0x4, 0x4d,
        0x46, 0x1b,
    ]);

    // Deserialize Alice's public key from bytes
    let alice_public_key = PublicKey::from(alice_public_key_bytes);

    // Bob can compute the same `SalsaBox` as Alice by performing the
    // key agreement operation.
    let bob_box = SalsaBox::new(&alice_public_key, &bob_secret_key);

    // Decrypt the message, using the same randomly generated nonce
    let mut decrypted_plaintext = [0u8; 4096];
    let decrypted_plaintext = decrypt(&bob_box, ciphertext, &mut decrypted_plaintext)?;

    assert_eq!(&plaintext[..], decrypted_plaintext);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt() -> Result<()> {
        let alice_secret_key = generate_secret_key::<Random>();

        // Obtain your recipient's public key.
        let bob_public_key = PublicKey::from([
            0xe8, 0x98, 0xc, 0x86, 0xe0, 0x32, 0xf1, 0xeb, 0x29, 0x75, 0x5, 0x2e, 0x8d, 0x65, 0xbd,
            0xdd, 0x15, 0xc3, 0xb5, 0x96, 0x41, 0x17, 0x4e, 0xc9, 0x67, 0x8a, 0x53, 0x78, 0x9d,
            0x92, 0xc7, 0x54,
        ]);

        // Create a `SalsaBox` by performing Diffie-Hellman key agreement between
        // the two keys.
        let alice_box = SalsaBox::new(&bob_public_key, &alice_secret_key);

        let mut dest = [0u8; 4096];
        let ciphertext = encrypt::<Random>(&alice_box, b"hello world", &mut dest)?;
        assert_eq!(ciphertext.len(), 51);

        Ok(())
    }

    #[test]
    fn run_test_sodium_crypto() -> Result<()> {
        test_sodium_crypto::<Random>()?;
        Ok(())
    }
}
