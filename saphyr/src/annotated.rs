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

use std::{
    borrow::Cow,
    hash::{BuildHasher, Hasher},
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use hashlink::LinkedHashMap;
use saphyr_parser::{ScalarStyle, Tag};

use crate::Scalar;

/// YAML data for nodes that will contain annotations.
///
/// If you want a YAML node without annotations, see [`Yaml`].
/// If you want a YAML node with annotations, see types using [`YamlData`] such as [`MarkedYaml`]
///
/// Unlike [`Yaml`] which only supports storing data, [`YamlData`] allows storing metadata
/// alongside the YAML data. It is unlikely one would build it directly; it is mostly intended to
/// be used, for instance, when parsing a YAML where retrieving markers / comments is relevant.
///
/// This definition is recursive. Each annotated node will be a structure storing the annotations
/// and the YAML data. We need to have a distinct enumeration from [`Yaml`] because the type for
/// the `Array` and `Mapping` variants is dependant on that structure.
///
/// If we had written [`YamlData`] as:
/// ```ignore
/// pub enum YamlData {
///   // ...
///   Sequence(Vec<Yaml>),
///   Mapping(LinkedHashMap<Yaml, Yaml>),
///   // ...
/// }
/// ```
/// we would have stored metadata for the root node only. All subsequent nodes would be [`Yaml`],
/// which does not contain any annotation.
///
/// Notable differences with [`Yaml`]:
///   * Indexing cannot return `BadValue` and will panic instead.
///
/// [`Yaml`]: crate::Yaml
/// [`MarkedYaml`]: marked_yaml::MarkedYaml
#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord, Hash)]
pub enum YamlData<'input, Node, HashKey>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self> + AnnotatedNode,
    HashKey: Eq + std::hash::Hash + std::borrow::Borrow<Node> + From<Node>,
{
    /// The raw string from the input.
    Representation(Cow<'input, str>, ScalarStyle, Option<Tag>),
    /// The resolved value from the representation.
    Value(Scalar<'input>),
    /// YAML sequence, can be accessed as a `Vec`.
    Sequence(AnnotatedSequence<Node>),
    /// YAML mapping, can be accessed as a `LinkedHashMap`.
    ///
    /// Insertion order will match the order of insertion into the map.
    Mapping(AnnotatedMapping<'input, HashKey, Node>),
    /// Alias, not fully supported yet.
    Alias(usize),
    /// Accessing a nonexistent node via the Index trait returns `BadValue`. This
    /// simplifies error handling in the calling code. Invalid type conversion also
    /// returns `BadValue`.
    BadValue,
}

// This defines most common operations on a YAML object. See macro definition for details.
define_yaml_object_impl!(
    YamlData<'input, Node, HashKey>,
    < 'input, Node, HashKey>,
    where {
        Node: std::hash::Hash + std::cmp::Eq + From<Self> + AnnotatedNode,
        HashKey: Eq
            + std::hash::Hash
            + std::borrow::Borrow<Node>
            + From<Node>
            + for<'b> PartialEq<Node::HashKey<'b>>,
    },
    mappingtype = AnnotatedMapping<'input, HashKey, Node>,
    sequencetype = AnnotatedSequence<Node>,
    nodetype = Node
);

impl<'input, Node, HashKey> YamlData<'input, Node, HashKey>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self> + AnnotatedNode,
    HashKey: Eq
        + std::hash::Hash
        + std::borrow::Borrow<Node>
        + From<Node>
        + for<'b> PartialEq<Node::HashKey<'b>>,
{
    /// Take the contained node out of `Self`, leaving a `BadValue` in its place.
    #[must_use]
    pub fn take(&mut self) -> Self {
        let mut taken_out = Self::BadValue;
        std::mem::swap(self, &mut taken_out);
        taken_out
    }
    /// Implementation detail for [`Self::as_mapping_get`], which is generated from a macro.
    fn as_mapping_get_impl<'a>(&self, key: &'a str) -> Option<&Node>
    where
        'input: 'a,
    {
        use std::hash::Hash;

        match self {
            YamlData::Mapping(mapping) => {
                let needle = Node::HashKey::<'a>::from(YamlData::Value(Scalar::String(key.into())));

                // In order to work around `needle`'s lifetime being different from `h`'s, we need
                // to manually compute the hash. Otherwise, we'd use `h.get()`, which complains the
                // needle's lifetime doesn't match that of the key in `h`.
                let mut hasher = mapping.hasher().build_hasher();
                needle.hash(&mut hasher);
                let hash = hasher.finish();

                mapping
                    .raw_entry()
                    .from_hash(hash, |candidate| *candidate == needle)
                    .map(|(_, v)| v)
            }
            _ => None,
        }
    }

    /// Implementation detail for [`Self::as_mapping_get_mut`], which is generated from a macro.
    #[must_use]
    fn as_mapping_get_mut_impl(&mut self, key: &str) -> Option<&mut Node> {
        match self.as_mapping_mut() {
            Some(mapping) => {
                use hashlink::linked_hash_map::RawEntryMut::{Occupied, Vacant};
                use std::hash::Hash;

                // In order to work around `needle`'s lifetime being different from `h`'s, we need
                // to manually compute the hash. Otherwise, we'd use `h.get()`, which complains the
                // needle's lifetime doesn't match that of the key in `h`.
                let needle = Node::HashKey::<'_>::from(YamlData::Value(Scalar::String(key.into())));
                let mut hasher = mapping.hasher().build_hasher();
                needle.hash(&mut hasher);
                let hash = hasher.finish();

                match mapping
                    .raw_entry_mut()
                    .from_hash(hash, |candidate| *candidate == needle)
                {
                    Occupied(entry) => Some(entry.into_mut()),
                    Vacant(_) => None,
                }
            }
            _ => None,
        }
    }
}

// NOTE(ethiraric, 10/06/2024): We cannot create a "generic static" variable which would act as a
// `BAD_VALUE`. This means that, unlike for `Yaml`, we have to make the indexing method panic.

impl<'input, 'a, Node, HashKey> Index<&'a str> for YamlData<'input, Node, HashKey>
where
    'input: 'a,
    Node: std::hash::Hash + std::cmp::Eq + From<Self> + AnnotatedNode,
    HashKey: Eq
        + std::hash::Hash
        + std::borrow::Borrow<Node>
        + From<Node>
        + for<'b> PartialEq<Node::HashKey<'b>>,
{
    type Output = Node;

    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`YamlData::Mapping`].
    fn index(&self, idx: &'a str) -> &Node {
        match self.as_mapping_get_impl(idx) {
            Some(value) => value,
            None => {
                if matches!(self, Self::Mapping(_)) {
                    panic!("Key '{idx}' not found in YamlData mapping")
                } else {
                    panic!("Attempt to index YamlData with '{idx}' but it's not a mapping")
                }
            }
        }
    }
}

impl<'input, 'a, Node, HashKey> IndexMut<&'a str> for YamlData<'input, Node, HashKey>
where
    'input: 'a,
    Node: std::hash::Hash + std::cmp::Eq + From<Self> + AnnotatedNode,
    HashKey: Eq
        + std::hash::Hash
        + std::borrow::Borrow<Node>
        + From<Node>
        + for<'b> PartialEq<Node::HashKey<'b>>,
{
    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`YamlData::Mapping`].
    fn index_mut(&mut self, idx: &'a str) -> &mut Node {
        assert!(
            matches!(self, Self::Mapping(_)),
            "Attempt to index YamlData with '{idx}' but it's not a mapping"
        );
        match self.as_mapping_get_mut_impl(idx) {
            Some(value) => value,
            None => {
                panic!("Key '{idx}' not found in YamlData mapping")
            }
        }
    }
}

impl<Node, HashKey> Index<usize> for YamlData<'_, Node, HashKey>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self> + AnnotatedNode,
    HashKey: Eq
        + std::hash::Hash
        + std::borrow::Borrow<Node>
        + From<Node>
        + for<'b> PartialEq<Node::HashKey<'b>>,
{
    type Output = Node;

    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`Index`]). If `self` is a
    /// [`YamlData::Sequence`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`YamlData::Mapping`], this is when the mapping sequence
    /// does not contain [`Scalar::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`YamlData::Sequence`] nor a
    /// [`YamlData::Mapping`].
    fn index(&self, idx: usize) -> &Node {
        if let Some(sequence) = self.as_vec() {
            sequence
                .get(idx)
                .unwrap_or_else(|| panic!("Index {idx} out of bounds in YamlData sequence"))
        } else if let Some(mapping) = self.as_mapping() {
            let key = i64::try_from(idx).unwrap_or_else(|_| {
                panic!("Attempt to index YamlData mapping with overflowing index")
            });
            mapping
                .get(&Self::Value(Scalar::Integer(key)).into())
                .unwrap_or_else(|| panic!("Key '{idx}' not found in YamlData mapping"))
        } else {
            panic!("Attempt to index YamlData with {idx} but it's not a mapping nor a sequence");
        }
    }
}

impl<Node, HashKey> IndexMut<usize> for YamlData<'_, Node, HashKey>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self> + AnnotatedNode,
    HashKey: Eq
        + std::hash::Hash
        + std::borrow::Borrow<Node>
        + From<Node>
        + for<'b> PartialEq<Node::HashKey<'b>>,
{
    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`IndexMut`]). If `self` is
    /// a [`YamlData::Sequence`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`YamlData::Mapping`], this is when the mapping sequence
    /// does not contain [`Scalar::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`YamlData::Sequence`] nor a
    /// [`YamlData::Mapping`].
    fn index_mut(&mut self, idx: usize) -> &mut Node {
        match self {
            Self::Sequence(sequence) => sequence
                .get_mut(idx)
                .unwrap_or_else(|| panic!("Index {idx} out of bounds in YamlData sequence")),
            Self::Mapping(mapping) => {
                let key = i64::try_from(idx).unwrap_or_else(|_| {
                    panic!("Attempt to index YamlData mapping with overflowing index")
                });
                mapping
                    .get_mut(&Self::Value(Scalar::Integer(key)).into())
                    .unwrap_or_else(|| panic!("Key {idx} not found in YamlData mapping"))
            }
            _ => {
                panic!("Attempt to index YamlData with {idx} but it's not a mapping nor a sequence")
            }
        }
    }
}

impl<'input, Node, HashKey> IntoIterator for YamlData<'input, Node, HashKey>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self> + AnnotatedNode,
    HashKey: Eq
        + std::hash::Hash
        + std::borrow::Borrow<Node>
        + From<Node>
        + for<'b> PartialEq<Node::HashKey<'b>>,
{
    type Item = Node;
    type IntoIter = AnnotatedYamlIter<'input, Node, HashKey>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            yaml: self.into_vec().unwrap_or_default().into_iter(),
            marker: PhantomData,
        }
    }
}

/// An iterator over a [`YamlData`] node.
#[allow(clippy::module_name_repetitions)]
pub struct AnnotatedYamlIter<'input, Node, HashKey>
where
    Node: std::hash::Hash + std::cmp::Eq + From<YamlData<'input, Node, HashKey>> + AnnotatedNode,
    HashKey: Eq
        + std::hash::Hash
        + std::borrow::Borrow<Node>
        + From<Node>
        + for<'b> PartialEq<Node::HashKey<'b>>,
{
    yaml: std::vec::IntoIter<Node>,
    marker: PhantomData<(&'input (), HashKey)>,
}

impl<'input, Node, HashKey> Iterator for AnnotatedYamlIter<'input, Node, HashKey>
where
    Node: std::hash::Hash + std::cmp::Eq + From<YamlData<'input, Node, HashKey>> + AnnotatedNode,
    HashKey: Eq
        + std::hash::Hash
        + std::borrow::Borrow<Node>
        + From<Node>
        + for<'b> PartialEq<Node::HashKey<'b>>,
{
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        self.yaml.next()
    }
}

/// The type contained in the [`YamlData::Sequence`] variant. This corresponds to YAML sequences.
#[allow(clippy::module_name_repetitions)]
pub type AnnotatedSequence<Node> = Vec<Node>;
/// The type contained in the [`YamlData::Mapping`] variant. This corresponds to YAML mappings.
#[allow(clippy::module_name_repetitions)]
pub type AnnotatedMapping<'input, HashKey, Node> = LinkedHashMap<HashKey, Node>;

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
    type HashKey<'a>: From<YamlData<'a, Self::HashKey<'a>, Self::HashKey<'a>>>
        + for<'b> std::cmp::PartialEq<Self::HashKey<'b>>
        + AnnotatedNode;
}
