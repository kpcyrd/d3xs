use thiserror_no_std::Error;

#[cfg(target_os = "none")]
pub use esp_println::println;
#[cfg(not(target_os = "none"))]
pub use libc_print::std_name::println;

#[derive(Error, Debug)]
pub enum Error {
    #[error("crypto failed")]
    Crypto(#[from] crypto_box::aead::Error),
    #[error("buffer size exceeded")]
    BufferLimit,
}
pub type Result<T> = core::result::Result<T, Error>;
