use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("crypto failed")]
    Crypto(#[from] crypto_box::aead::Error),
    #[error("failed to decode buffer")]
    DecodeEncoding(#[from] data_encoding::DecodeError),
    #[error("invalid length for key: {0}")]
    InvalidKeyLength(usize),
    #[error("invalid challenge response")]
    InvalidChallengeReponse,
    #[error("authentication failed")]
    AuthError,
    #[error("buffer size exceeded")]
    BufferLimit,
}
pub type Result<T> = core::result::Result<T, Error>;
