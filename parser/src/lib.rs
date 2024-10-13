// Copyright 2015, Yuheng Chen.
// Copyright 2023, Ethiraric.
// See the LICENSE file at the top-level directory of this distribution.

//! YAML 1.2 parser implementation in pure Rust.
//!
//! **If you want to load to a YAML Rust structure or manipulate YAML objects, use `saphyr` instead
//! of `saphyr-parser`. This crate contains only the parser.**
//!
//! This is YAML 1.2 parser implementation and low-level parsing API for YAML. It allows users to
//! fetch a stream of YAML events from a stream of characters/bytes.
//!
//! # Usage
//!
//! This crate is [on github](https://github.com/saphyr-rs/saphyr-parser) and can be used by adding
//! `saphyr-parser` to the dependencies in your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! saphyr-parser = "0.0.2"
//! ```
//!
//! # Features
//! **Note:** With all features disabled, this crate's MSRV is `1.65.0`.
//!
//! #### `debug_prints`
//! Enables the `debug` module and usage of debug prints in the scanner and the parser. Do not
//! enable if you are consuming the crate rather than working on it as this can significantly
//! decrease performance.
//!
//! The MSRV for this feature is `1.70.0`.

#![warn(missing_docs, clippy::pedantic)]

mod char_traits;
#[macro_use]
mod debug;
pub mod input;
mod parser;
mod scanner;

pub use crate::input::{str::StrInput, BufferedInput, Input};
pub use crate::parser::{Event, EventReceiver, Parser, SpannedEventReceiver, Tag};
pub use crate::scanner::{Marker, ScanError, Span, TScalarStyle};
