#![doc = include_str!("../README.md")]
#![doc(html_root_url = "https://docs.rs/saphyr_serde/0.9.34+deprecated")]
#![deny(missing_docs, unsafe_op_in_unsafe_fn)]
// Suppressed clippy_pedantic lints
#![allow(
    // buggy
    clippy::iter_not_returning_iterator, // https://github.com/rust-lang/rust-clippy/issues/8285
    clippy::ptr_arg, // https://github.com/rust-lang/rust-clippy/issues/9218
    clippy::question_mark, // https://github.com/rust-lang/rust-clippy/issues/7859
    // private Deserializer::next
    clippy::should_implement_trait,
    // things are often more readable this way
    clippy::cast_lossless,
    clippy::checked_conversions,
    clippy::if_not_else,
    clippy::manual_assert,
    clippy::match_like_matches_macro,
    clippy::match_same_arms,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::redundant_else,
    clippy::single_match_else,
    // code is acceptable
    clippy::blocks_in_conditions,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::derive_partial_eq_without_eq,
    clippy::derived_hash_with_manual_eq,
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::let_underscore_untyped,
    clippy::manual_map,
    clippy::missing_panics_doc,
    clippy::never_loop,
    clippy::return_self_not_must_use,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::unsafe_removed_from_name,
    clippy::wildcard_in_or_patterns,
    // noisy
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
)]

pub use crate::de::{from_reader, from_slice, from_str, Deserializer};
pub use crate::error::{Error, Location, Result};
pub use crate::ser::{to_string, to_writer, Serializer};
#[doc(inline)]
pub use crate::value::{from_value, to_value, Index, Number, Sequence, Value};

#[doc(inline)]
pub use crate::mapping::Mapping;

mod de;
mod error;
mod libyaml;
mod loader;
pub mod mapping;
mod number;
mod path;
mod ser;
pub mod value;
pub mod with;

// Prevent downstream code from implementing the Index trait.
mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl Sealed for crate::Value {}
    impl<'a, T> Sealed for &'a T where T: ?Sized + Sealed {}
}
