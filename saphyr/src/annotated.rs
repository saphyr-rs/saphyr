//! Utilities for extracting YAML with certain metadata.
//!
//! This module contains [`YamlData`], an alternate [`Yaml`] object which is generic over its node
//! and key (for mapping) types. Since annotated nodes look like:
//!
//! ```ignore
//! struct AnnotatedYaml {
//!   // ... metadata ...
//!   object: /* YAML type */
//! }
//! ```
//!
//! it means that the `Hash` and `Array` variants must return `AnnotatedYaml`s instead of a
//! [`Yaml`] node. [`YamlData`] is used to fill this need. It behaves very similarly to [`Yaml`]
//! and has the same interface, with the only difference being the types it returns for nodes and
//! hash keys.
//!
//! The module also contains common annotated node types (e.g.: [`MarkedYaml`]).
//!
//! # Architecture overview
//! Multiple indirections and constructions are needed to support annotated YAML objects as
//! seamlessly as possible in the user-facing API. This module is mostly implementation details and
//! does not weigh much in performance, so it is designed so that the API looks clean, no matter
//! the dirty details behind.
//!
//! The goals are:
//! - Easy-to-use user-facing API
//! - Versatility - it should be possible to retrieve any specific set of metadata
//!
//! There are 3 major components:
//! - Node types: These are the YAML objects, storing both the YAML data and YAML metadata
//!   - [`MarkedYaml`], [`MarkedYamlOwned`]
//! - Data types: These are the structures holding YAML data
//!   - [`YamlData`], [`YamlDataOwned`]
//! - Traits: Some traits are needed to allow for generic Node types
//!   - [`AnnotatedNode`], [`AnnotatedNodeOwned`]
//!
//! In order to add a new Node type, the following is required:
//!   - Use either [`YamlData`] or [`YamlDataOwned`]. There shouldn't be a need for any other Data
//!     type.
//!   - Implement [`std::hash::Hash`], [`std::cmp::Eq`] and [`std::cmp::PartialEq`] for your Node
//!     type. These traits are required for [`AnnotatedNode`].
//!   - Implement [`AnnotatedNode`] or [`AnnotatedNodeOwned`] for your Node type, depending on
//!     whether it is borrowed or not.
//!   - Implement [`LoadableYamlNode`] for your Node type.
//!
//! In order to implement [`AnnotatedNode`] and [`LoadableYamlNode`], you may rely on the methods
//! [`YamlData`] offers (e.g.: [`YamlData::parse_representation_recursive`]).
//!
//! [`LoadableYamlNode`]: crate::LoadableYamlNode
//! [`Mapping`]: crate::Yaml::Mapping
//! [`MarkedYaml`]: marked_yaml::MarkedYaml
//! [`MarkedYamlOwned`]: marked_yaml_owned::MarkedYamlOwned
//! [`Yaml`]: crate::Yaml

pub mod marked_yaml;
pub mod marked_yaml_owned;
pub mod yaml_data;
pub mod yaml_data_owned;

pub use yaml_data::{AnnotatedMapping, AnnotatedMappingOwned, AnnotatedSequence, AnnotatedSequenceOwned, AnnotatedYamlIter, YamlData};
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
