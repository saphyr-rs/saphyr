// Copyright 2015, Yuheng Chen.
// Copyright 2023, Ethiraric.
// See the LICENSE file at the top-level directory of this distribution.

//! YAML 1.2 implementation in pure Rust.
//!
//! # Usage
//!
//! This crate is [on github](https://github.com/saphyr-rs/saphyr) and can be used by adding
//! `saphyr` to the dependencies in your project's `Cargo.toml`:
//! ```sh
//! cargo add saphyr
//! ```
//!
//! # Examples
//! Parse a string into `Vec<Yaml>` and then serialize it as a YAML string.
//!
//! ```
//! use saphyr::{LoadableYamlNode, Yaml, YamlEmitter};
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
//! # YAML object types
//!
//! There are multiple YAML objects in this library which share most features but differ in
//! usecase:
//!   - [`Yaml`]: The go-to YAML object. It contains YAML data and borrows from the input.
//!   - [`YamlOwned`]: An owned version of [`Yaml`]. It does not borrow from the input and can be
//!     used when tieing the object to the input is undesireable or would introduce unnecessary
//!     lifetimes.
//!   - [`MarkedYaml`]: A YAML object with added [`Marker`]s for the beginning and end of the YAML
//!     object in the input.
//!   - [`MarkedYamlOwned`]: An owned version of [`MarkedYaml`]. It does not borrow from the input
//!     and can be used when tieing the object to the input is undesireable or would introduce
//!     unnecessary lifetimes.
//!
//! All of these share the same inspection methods (`is_boolean`, `as_str`, `into_vec`, ...) with
//! some variants between owned and borrowing versions.
//!
//! They also contain the same variants. [`Yaml`] and [`YamlOwned`] are `enums`, while annotated
//! objects ([`MarkedYaml`], [`MarkedYamlOwned`]) are structures with a `.data` enum
//! (see [`YamlData`], [`YamlDataOwned`]).
//!
//! # YAML Tags
//! ## YAML Core Schema tags (`!!str`, `!!int`, `!!float`, ...)
//! `saphyr` is aware of the [YAML Core Schema tags](https://yaml.org/spec/1.2.2/#103-core-schema)
//! and will parse scalars accordingly. This is handled in [`Scalar::parse_from_cow_and_metadata`].
//! Should a scalar be explicitly tagged with a tag whose handle is that of the Core Schema,
//! `saphyr` will attempt to parse it as the given type. If parsing fails (e.g.: `!!int foo`), a
//! [`BadValue`] will be returned. If however the tag is unknown, the scalar will be parsed as a
//! string (e.g.: `!!unk 12`).
//!
//! Core Schema tags on collections are ignored, since the syntax disallows any ambiguity in
//! parsing.
//!
//! Upon parsing, the core schema tags are not preserved. If you need the tags on scalar preserved,
//! you may disable [`early_parse`]. This will cause all scalars to use the [`Representation`]
//! variant which preserves the tags. There is currently no way to preserve Core Schema tags on
//! collections.
//!
//! ## User-defined tags
//! The YAML specification does not explicitly specify how user-defined tags should be parsed
//! ([10.4](https://yaml.org/spec/1.2.2/#104-other-schemas)). `saphyr` is very conservative on this
//! and will leave tags as-is. They are wrapped in a [`Tagged`] variant where you can freely
//! inspect the tag alongside the tagged node.
//!
//! **The tagged node will be parsed as an untagged node.** What this means is that `13` will be
//! parsed as an integer, `foo` as a string, ... etc. See the related discussion [on
//! Github](https://github.com/saphyr-rs/saphyr/issues/4#issuecomment-2899433908) for more context
//! on the decision.
//!
//! Examples:
//! ```
//! # use saphyr::{LoadableYamlNode, Tag, Yaml};
//! # let parse = |s| Yaml::load_from_str(s).unwrap().into_iter().next().unwrap();
//! #
//! assert!(matches!(parse("!custom 3"),     Yaml::Tagged(_tag, node) if node.is_integer()));
//! assert!(matches!(parse("!custom 'foo'"), Yaml::Tagged(_tag, node) if node.is_string()));
//! assert!(matches!(parse("!custom foo"),   Yaml::Tagged(_tag, node) if node.is_string()));
//! assert!(matches!(parse("!custom ~"),     Yaml::Tagged(_tag, node) if node.is_null()));
//! assert!(matches!(parse("!custom '3'"),   Yaml::Tagged(_tag, node) if node.is_string()));
//! ```
//!
//! User-defined tags can be applied to any node, whether a collection or a scalar. They do not
//! change the resolution behavior of inner nodes.
//! ```
//! # use saphyr::{LoadableYamlNode, Tag, Yaml};
//! # let parse = |s| Yaml::load_from_str(s).unwrap().into_iter().next().unwrap();
//! #
//! assert!(matches!(parse("!custom [foo]"),     Yaml::Tagged(_tag, node) if node.is_sequence()));
//! assert!(matches!(parse("!custom {a:b}"),     Yaml::Tagged(_tag, node) if node.is_mapping()));
//!
//! let node = parse("!custom [1, foo, !!str 3, !custom 3]");
//! let Yaml::Tagged(_, seq) = node else { panic!() };
//! assert!(seq.is_sequence());
//! assert!(seq[0].is_integer());
//! assert!(seq[1].is_string());
//! assert!(seq[2].is_string());
//! assert!(matches!(&seq[3], Yaml::Tagged(_tag, node) if node.is_integer()));
//! ```
//!
//! # Features
//! **Note:** With all features disabled, this crate's MSRV is `1.65.0`.
//!
//! #### `encoding` (_enabled by default_)
//! Enables encoding-aware decoding of Yaml documents.
//!
//! The MSRV for this feature is `1.70.0`.
//!
//! [`MarkedYaml`]: crate::MarkedYaml
//! [`MarkedYamlOwned`]: crate::MarkedYamlOwned
//! [`Marker`]: crate::Marker
//! [`YamlData`]: crate::YamlData
//! [`YamlDataOwned`]: crate::YamlDataOwned
//! [`BadValue`]: Yaml::BadValue
//! [`Representation`]: Yaml::Representation
//! [`Tagged`]: Yaml::Tagged
//! [`early_parse`]: crate::YamlLoader::early_parse

#![warn(missing_docs, clippy::pedantic)]

#[macro_use]
mod macros;

mod annotated;
mod char_traits;
mod emitter;
mod loader;
mod scalar;
mod yaml;
mod yaml_owned;

// Re-export main components.
pub use crate::annotated::{
    marked_yaml::MarkedYaml, marked_yaml_owned::MarkedYamlOwned, AnnotatedMapping,
    AnnotatedMappingOwned, AnnotatedNode, AnnotatedNodeOwned, AnnotatedSequence,
    AnnotatedSequenceOwned, AnnotatedYamlIter, Index, Indexable, YamlData, YamlDataOwned,
};
pub use crate::emitter::{EmitError, YamlEmitter};
pub use crate::loader::{LoadError, LoadableYamlNode, YamlLoader};
pub use crate::scalar::{parse_core_schema_fp, Scalar, ScalarOwned};
pub use crate::yaml::{Mapping, Sequence, Yaml, YamlIter};
pub use crate::yaml_owned::{MappingOwned, SequenceOwned, YamlOwned, YamlOwnedIter};

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
