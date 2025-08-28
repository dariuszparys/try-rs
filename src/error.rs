use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
