use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("bad data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("certificate: {0}")]
    Cert(String),
    #[error("tls: {0}")]
    Tls(#[from] rustls::Error),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
