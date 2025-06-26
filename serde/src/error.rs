use std::fmt::{self, Display};

use saphyr_parser::Marker;
use serde::{de, ser};

/// Convenience alias using [`Error`] as an error type.
pub type Result<T> = std::result::Result<T, Error>;

/// An error that occured during serialization or deserialization.
#[derive(Debug)]
pub struct Error {
    /// Indirection used to reduce the size of [`Error`] and [`Result`].
    err: Box<ErrorImpl>,
}

impl Error {
    /// Returns information about the location of the error.
    #[must_use]
    pub fn locus(&self) -> Marker {
        self.err.locus
    }

    pub fn details(&self) -> &ErrorDetails {
        &self.err.code
    }
}

/// Implementation for [`Error`].
#[derive(Debug)]
struct ErrorImpl {
    /// The [`Marker`] at which the error happened.
    locus: Marker,
    /// Description of the error.
    code: ErrorDetails,
}

/// As much details about the error as we can get.
#[derive(Debug)]
pub enum ErrorDetails {
    /// Generic fallback for errors.
    Message(String),
    /// The end of the input was unexpectedly reached.
    Eof,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self {
            err: Box::new(ErrorImpl {
                locus: Marker::new(0, 0, 0),
                code: ErrorDetails::Message(msg.to_string()),
            }),
        }
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self {
            err: Box::new(ErrorImpl {
                locus: Marker::new(0, 0, 0),
                code: ErrorDetails::Message(msg.to_string()),
            }),
        }
    }
}

impl Display for ErrorDetails {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Message(msg) => formatter.write_str(msg),
            Self::Eof => formatter.write_str("unexpected end of input"),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.locus().line() == 0 {
            Display::fmt(self.details(), f)
        } else {
            write!(
                f,
                "{}:{}: {}",
                self.locus().line(),
                self.locus().col(),
                self.details()
            )
        }
    }
}

impl std::error::Error for Error {}
