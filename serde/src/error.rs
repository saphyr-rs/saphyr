//! Errors returned by (de)serialization.

use std::{fmt, string::String};

use saphyr::EventEmitError;
use saphyr_parser::{Marker, ScanError};

/// A convenience alias for results using this crate's [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Where an error occurred in the YAML source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    index: usize,
    line: usize,
    column: usize,
}

impl Location {
    pub(crate) fn from_marker(marker: Marker) -> Self {
        Self {
            index: marker.index(),
            line: marker.line(),
            column: marker.col() + 1,
        }
    }

    /// The index of the character in the input, starting at 0.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// The line, starting at 1.
    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }

    /// The column, starting at 1.
    #[must_use]
    pub fn column(&self) -> usize {
        self.column
    }
}

/// The category of an [`Error`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The input is not well-formed YAML.
    Syntax,
    /// The input is well-formed YAML but is not what the target type expects.
    Data,
    /// The input exceeds one of the [`Options`](crate::Options) limits.
    ///
    /// Such an input may be perfectly valid, but is rejected to protect against denial of service.
    LimitExceeded,
    /// A value could not be serialized.
    Serialize,
}

/// An error while deserializing from or serializing to YAML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    location: Option<Location>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            location: None,
        }
    }

    pub(crate) fn at(kind: ErrorKind, message: impl Into<String>, marker: Marker) -> Self {
        Self {
            kind,
            message: message.into(),
            location: Some(Location::from_marker(marker)),
        }
    }

    pub(crate) fn data(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Data, message)
    }

    pub(crate) fn limit(message: impl Into<String>, marker: Marker) -> Self {
        Self::at(ErrorKind::LimitExceeded, message, marker)
    }

    /// Attach `marker` as the location, unless the error already has one.
    pub(crate) fn or_at(mut self, marker: Marker) -> Self {
        if self.location.is_none() {
            self.location = Some(Location::from_marker(marker));
        }
        self
    }

    /// The category of the error.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The location in the input, if known.
    #[must_use]
    pub fn location(&self) -> Option<Location> {
        self.location
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        if let Some(loc) = self.location {
            write!(f, " at line {} column {}", loc.line, loc.column)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<ScanError> for Error {
    fn from(e: ScanError) -> Self {
        Self::at(ErrorKind::Syntax, e.info(), *e.marker())
    }
}

impl From<EventEmitError> for Error {
    fn from(e: EventEmitError) -> Self {
        match e {
            EventEmitError::TooDeep(_) => {
                Self::new(ErrorKind::LimitExceeded, "value is nested too deeply")
            }
            e => Self::new(ErrorKind::Serialize, e.to_string()),
        }
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::data(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::new(ErrorKind::Serialize, msg.to_string())
    }
}
