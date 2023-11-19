use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("protocol error")]
    Protocol(#[from] d3xs_protocol::errors::Error),
    #[error("auth decrypt failed")]
    AuthError,
    #[error("failed to call esp api: {0}")]
    EspError(&'static str),
}
pub type Result<T> = core::result::Result<T, Error>;
