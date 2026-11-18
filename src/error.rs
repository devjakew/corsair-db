use std::fmt;
use std::io;

use crate::command::DecodeError;

/// Errors surfaced by the storage layer.
#[derive(Debug)]
pub enum Error {
    /// The underlying file operation failed.
    Io(io::Error),
    /// A log record passed its checksum but did not decode to a valid command.
    /// This indicates a bug or a format mismatch, not a torn write.
    Decode(DecodeError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Decode(e) => write!(f, "corrupt log entry: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Decode(e) => Some(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<DecodeError> for Error {
    fn from(e: DecodeError) -> Self {
        Error::Decode(e)
    }
}
