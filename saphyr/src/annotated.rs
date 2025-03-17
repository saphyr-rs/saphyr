//! Utilities for extracting YAML with certain metadata.
//!
//! This module contains [`YamlData`], an alternate [`Yaml`] object which is generic over its node
//! and key (for mapping) types. Since annotated nodes look like:
//!
//! ```ignore
//! struct AnnotatedYaml {
//!   // metadata
//!   object: /* YAML type */
//! }
//! ```
//!
//! it means that the `Hash` and `Array` variants must return `AnnotatedYaml`s instead of a
//! [`Yaml`] node. [`YamlData`] is used to fill this need. It behaves very similarly to [`Yaml`]
//! and has the same interface, with the only difference being the types it returns for nodes and
//! hash keys.
//!
//! [`Yaml`]: crate::Yaml

pub mod marked_yaml;
pub mod marked_yaml_owned;
pub mod yaml_data;
pub mod yaml_data_owned;

pub use yaml_data::{AnnotatedMapping, AnnotatedSequence, AnnotatedYamlIter, YamlData};
pub use yaml_data_owned::YamlDataOwned;

/// A trait allowing for introspection in the hash types of the [`YamlData::Mapping`] variant.
///
/// This trait must be implemented by annotated YAML objects.
///
/// See [`LoadableYamlNode::HashKey`] for more details.
///
/// [`LoadableYamlNode::HashKey`]: crate::loader::LoadableYamlNode::HashKey
#[allow(clippy::module_name_repetitions)]
pub trait AnnotatedNode: std::hash::Hash + std::cmp::Eq {
    /// The type used as the key in the [`YamlData::Mapping`] variant.
    type HashKey<'a>: From<YamlData<'a, Self::HashKey<'a>>>
        + for<'b> std::cmp::PartialEq<Self::HashKey<'b>>
        + AnnotatedNode;

    /// See [`YamlData::parse_representation_recursive`].
    fn parse_representation_recursive(&mut self) -> bool;
}

/// A trait allowing for introspection in the hash types of the [`YamlData::Mapping`] variant.
///
/// This trait must be implemented by annotated YAML objects.
///
/// See [`LoadableYamlNode::HashKey`] for more details.
///
/// [`LoadableYamlNode::HashKey`]: crate::loader::LoadableYamlNode::HashKey
#[allow(clippy::module_name_repetitions)]
pub trait AnnotatedNodeOwned: std::hash::Hash + std::cmp::Eq {
    /// The type used as the key in the [`YamlDataOwned::Mapping`] variant.
    type HashKey: From<YamlDataOwned<Self::HashKey>>
        + std::cmp::PartialEq<Self::HashKey>
        + AnnotatedNodeOwned;

    /// See [`YamlData::parse_representation_recursive`].
    fn parse_representation_recursive(&mut self) -> bool;
}
