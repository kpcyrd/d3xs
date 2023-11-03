use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("crypto failed")]
    Crypto(#[from] crypto_box::aead::Error),
    #[error("buffer size exceeded")]
    BufferLimit,
}
pub type Result<T> = core::result::Result<T, Error>;