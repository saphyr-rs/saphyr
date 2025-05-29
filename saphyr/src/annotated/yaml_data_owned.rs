use std::{
    hash::{BuildHasher, Hasher},
    ops::{Index, IndexMut},
};

use hashlink::LinkedHashMap;
use saphyr_parser::{ScalarStyle, Tag};

use crate::{annotated::AnnotatedNodeOwned, ScalarOwned};

/// YAML data for nodes that will contain annotations.
///
/// Owned version of [`YamlData`].
///
/// [`YamlData`]: `crate::YamlData`
#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord, Hash)]
pub enum YamlDataOwned<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self> + AnnotatedNodeOwned,
{
    /// The raw string from the input.
    ///
    /// See [`YamlData::Representation`].
    ///
    /// [`YamlData::Representation`]: `crate::YamlData::Representation`
    Representation(String, ScalarStyle, Option<Tag>),
    /// The resolved value from the representation.
    ///
    /// See [`YamlData::Value`].
    ///
    /// [`YamlData::Value`]: `crate::YamlData::Value`
    Value(ScalarOwned),
    /// YAML sequence, can be accessed as a `Vec`.
    ///
    /// See [`YamlData::Sequence`].
    ///
    /// [`YamlData::Sequence`]: `crate::YamlData::Sequence`
    Sequence(AnnotatedSequenceOwned<Node>),
    /// YAML mapping, can be accessed as a [`LinkedHashMap`].
    ///
    /// See [`YamlData::Mapping`].
    ///
    /// [`YamlData::Mapping`]: `crate::YamlData::Mapping`
    Mapping(AnnotatedMappingOwned<Node>),
    /// A tagged node.
    ///
    /// See [`YamlData::Tagged`].
    ///
    /// [`YamlData::Tagged`]: `crate::YamlData::Tagged`
    Tagged(Tag, Box<Node>),
    /// Alias, not fully supported yet.
    ///
    /// See [`YamlData::Alias`].
    ///
    /// [`YamlData::Alias`]: `crate::YamlData::Alias`
    Alias(usize),
    /// A variant used when parsing the representation of a scalar node fails.
    ///
    /// See [`YamlData::BadValue`].
    ///
    /// [`YamlData::BadValue`]: `crate::YamlData::BadValue`
    BadValue,
}

// This defines most common operations on a YAML object. See macro definition for details.
define_yaml_object_impl!(
    YamlDataOwned<Node>,
    < Node>,
    where {
        Node: std::hash::Hash
            + std::cmp::Eq
            + From<Self>
            + AnnotatedNodeOwned
            + std::cmp::PartialEq<Node::HashKey>,
    },
    mappingtype = AnnotatedMappingOwned<Node>,
    sequencetype = AnnotatedSequenceOwned<Node>,
    nodetype = Node,
    scalartype = { ScalarOwned },
    selfname = "YamlDataOwned",
    owned
);

impl<Node> YamlDataOwned<Node>
where
    Node: std::hash::Hash
        + std::cmp::Eq
        + From<Self>
        + AnnotatedNodeOwned
        + std::cmp::PartialEq<Node::HashKey>,
{
    /// Take the contained node out of `Self`, leaving a `BadValue` in its place.
    #[must_use]
    pub fn take(&mut self) -> Self {
        let mut taken_out = Self::BadValue;
        std::mem::swap(self, &mut taken_out);
        taken_out
    }

    /// Implementation detail for [`Self::as_mapping_get`], which is generated from a macro.
    fn as_mapping_get_impl(&self, key: &str) -> Option<&Node> {
        use std::hash::Hash;

        match self {
            Self::Mapping(mapping) => {
                let needle =
                    Node::HashKey::from(YamlDataOwned::Value(ScalarOwned::String(key.into())));

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
                let needle =
                    Node::HashKey::from(YamlDataOwned::Value(ScalarOwned::String(key.to_string())));
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

impl<Node> IntoIterator for YamlDataOwned<Node>
where
    Node: std::hash::Hash
        + std::cmp::Eq
        + From<Self>
        + AnnotatedNodeOwned
        + std::cmp::PartialEq<Node::HashKey>,
{
    type Item = Node;
    type IntoIter = AnnotatedYamlOwnedIter<Node>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            yaml: self.into_vec().unwrap_or_default().into_iter(),
        }
    }
}

/// An iterator over a [`YamlDataOwned`] node.
#[allow(clippy::module_name_repetitions)]
pub struct AnnotatedYamlOwnedIter<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<YamlDataOwned<Node>> + AnnotatedNodeOwned,
{
    yaml: std::vec::IntoIter<Node>,
}

impl<Node> Iterator for AnnotatedYamlOwnedIter<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<YamlDataOwned<Node>> + AnnotatedNodeOwned,
{
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        self.yaml.next()
    }
}

/// The type contained in the [`YamlDataOwned::Sequence`] variant. This corresponds to YAML sequences.
#[allow(clippy::module_name_repetitions)]
pub type AnnotatedSequenceOwned<Node> = Vec<Node>;
/// The type contained in the [`YamlDataOwned::Mapping`] variant. This corresponds to YAML mappings.
#[allow(clippy::module_name_repetitions)]
pub type AnnotatedMappingOwned<Node> = LinkedHashMap<Node, Node>;
