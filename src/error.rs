use std::io::ErrorKind::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("invalid http version")]
    InvalidVersion,
    #[error("invalid method")]
    InvalidMethod,
    #[error("invalid request")]
    InvalidRequest,
    #[error("connection closed")]
    Disconnected,
    #[error(transparent)]
    Io(std::io::Error),
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            UnexpectedEof | ConnectionReset | BrokenPipe => ServerError::Disconnected,
            _ => ServerError::Io(e),
        }
    }
}
