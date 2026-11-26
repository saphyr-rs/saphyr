//! [Serde](https://serde.rs) support for [`saphyr`], a YAML 1.2 library.
//!
//! ```
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! struct Config {
//!     name: String,
//!     ports: Vec<u16>,
//! }
//!
//! let config: Config = saphyr_serde::from_str("name: web\nports: [80, 443]").unwrap();
//! assert_eq!(config, Config { name: "web".into(), ports: vec![80, 443] });
//! assert_eq!(saphyr_serde::to_string(&config).unwrap(), "name: web\nports:\n  - 80\n  - 443\n");
//! ```
//!
//! # Data model
//! Scalars are resolved with the YAML 1.2 Core Schema, but only when the type being deserialized
//! asks for it. A `String` field gets the text of its scalar, whether it is `NO`, `1.0` or
//! `null`; a `bool` field only accepts `true`/`false` (in lowercase, titlecase or uppercase).
//! Quoted scalars are always strings.
//!
//! Enums are externally tagged: `Variant` for unit variants, and `Variant: content` for the
//! others. When deserializing, a `!Variant content` tag is accepted too.
//!
//! Aliases (`*anchor`) are supported. Merge keys (`<<`) are not: they are from YAML 1.1, and `<<` is
//! a key like any other.
//!
//! # Untrusted input
//! The default [`Options`] make deserializing untrusted input safe: documents that are nested
//! too deeply, that have too many nodes, or whose aliases expand to too much data (the "billion
//! laughs" attack) are rejected with [`ErrorKind::LimitExceeded`] before they are
//! deserialized. Mappings with the same key twice are rejected as well. This is checked on a
//! whole document before any of it is deserialized, and no code of the type being deserialized
//! can be reached through the YAML (there are no constructors of arbitrary types as in some
//! other YAML libraries).
//!
//! The input itself is not limited: check its size if it comes from somewhere you do not trust.

mod de;
mod error;
mod options;
mod ser;

pub use crate::{
    de::{
        Deserializer, Document, from_slice, from_slice_with_options, from_str,
        from_str_with_options,
    },
    error::{Error, ErrorKind, Location, Result},
    options::Options,
    ser::{to_string, to_string_with_options, to_writer, to_writer_with_options},
};
