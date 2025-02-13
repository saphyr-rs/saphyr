// Copyright 2015, Yuheng Chen.
// Copyright 2023, Ethiraric.
// See the LICENSE file at the top-level directory of this distribution.

//! YAML 1.2 implementation in pure Rust.
//!
//! # Usage
//!
//! This crate is [on github](https://github.com/saphyr-rs/saphyr) and can be used by adding
//! `saphyr` to the dependencies in your project's `Cargo.toml`.
//! ```toml
//! [dependencies]
//! saphyr = "0.0.3"
//! ```
//! or by using `cargo add` to get the latest version:
//! ```sh
//! cargo add saphyr
//! ```
//!
//! # Examples
//! Parse a string into `Vec<Yaml>` and then serialize it as a YAML string.
//!
//! ```
//! use saphyr::{Yaml, YamlEmitter};
//!
//! let docs = Yaml::load_from_str("[1, 2, 3]").unwrap();
//! let doc = &docs[0]; // select the first YAML document
//! assert_eq!(doc[0].as_integer().unwrap(), 1); // access elements by index
//!
//! let mut out_str = String::new();
//! let mut emitter = YamlEmitter::new(&mut out_str);
//! emitter.dump(doc).unwrap(); // dump the YAML object to a String
//! ```
//!
//! # Features
//! **Note:** With all features disabled, this crate's MSRV is `1.65.0`.
//!
//! #### `encoding` (_enabled by default_)
//! Enables encoding-aware decoding of Yaml documents.
//!
//! The MSRV for this feature is `1.70.0`.

#![warn(missing_docs, clippy::pedantic)]

#[macro_use]
mod macros;

mod annotated;
mod char_traits;
mod emitter;
mod loader;
mod scalar;
mod yaml;

// Re-export main components.
pub use crate::annotated::{
    marked_yaml::MarkedYaml, AnnotatedMapping, AnnotatedNode, AnnotatedSequence, AnnotatedYamlIter,
    YamlData,
};
pub use crate::emitter::YamlEmitter;
pub use crate::loader::{LoadableYamlNode, YamlLoader};
pub use crate::scalar::{Scalar, ScalarOwned};
pub use crate::yaml::{Mapping, Sequence, Yaml, YamlIter};

#[cfg(feature = "encoding")]
mod encoding;
#[cfg(feature = "encoding")]
pub use crate::encoding::{YAMLDecodingTrap, YAMLDecodingTrapFn, YamlDecoder};

// Re-export `ScanError` as it is used as part of our public API and we want consumers to be able
// to inspect it (e.g. perform a `match`). They wouldn't be able without it.
pub use saphyr_parser::ScanError;
// Re-export `Marker` which is used for annotated YAMLs.
pub use saphyr_parser::Marker;
// Re-export `ScalarStyle` and `Tag` which are used for representations.
pub use saphyr_parser::{ScalarStyle, Tag};
