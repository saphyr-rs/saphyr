//! YAML objects manipulation utilities.

#![allow(clippy::module_name_repetitions)]

use std::{
    hash::{BuildHasher, Hasher},
    ops::{Index, IndexMut},
};

use hashlink::LinkedHashMap;
use saphyr_parser::{ScalarStyle, Tag};

use crate::{LoadableYamlNode, ScalarOwned, Yaml};

/// A YAML node is stored as this `Yaml` enumeration, which provides an easy way to
/// access your YAML document.
///
/// # Examples
///
/// ```
/// use saphyr::{Scalar, Yaml};
/// let foo = Yaml::value_from_str("-123"); // convert the string to the appropriate YAML type
/// assert_eq!(foo.as_integer().unwrap(), -123);
///
/// // iterate over an Sequence
/// let vec = Yaml::Sequence(vec![Yaml::Value(Scalar::Integer(1)), Yaml::Value(Scalar::Integer(2))]);
/// for v in vec.as_vec().unwrap() {
///     assert!(v.is_integer());
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord, Hash)]
pub enum YamlOwned {
    /// The raw string from the input.
    ///
    /// When the field is left in the [`Representation`] variant, methods that rely on the value
    /// (e.g.: [`is_boolean`], [`as_integer`], [`into_floating_point`], ...) will always return
    /// [`None`].
    ///
    /// This variant is only meant:
    ///   - As an optimization, when lazy-parsing is preferred.
    ///   - As a more generic way of handling keys in [`Mapping`]s (if user-defined key duplication
    ///     detection is required.
    ///
    /// [`Mapping`]: Yaml::Mapping
    /// [`Representation`]: Yaml::Representation
    /// [`is_boolean`]: Yaml::is_boolean
    /// [`as_integer`]: Yaml::as_integer
    /// [`into_floating_point`]: Yaml::into_floating_point
    Representation(String, ScalarStyle, Option<Tag>),
    /// The resolved value from the representation.
    Value(ScalarOwned),
    /// YAML sequence, can be accessed as a `Vec`.
    Sequence(SequenceOwned),
    /// YAML mapping, can be accessed as a [`LinkedHashMap`].
    ///
    /// Iteration order will match the order of insertion into the map and that of the document.
    ///
    /// If keys use the [`Representation`] variant, equality will be based on their representation.
    /// When comparing representations for equality, the string, [scalar style] and tags must
    /// match. This means that `'100'` and `"100"`, although similar in their value, have different
    /// representations.
    ///
    /// If keys use the [`Value`] variant, they will be compared by value. It is discouraged to use
    /// floating point values as keys. [`ScalarOwned`] uses [`OrderedFloat`] for hash and equality.
    /// Refer to their documentation for details on float comparisons.
    ///
    /// Comparison between [`Representation`] variants and [`Value`] variants will always fail.
    /// Users must ensure all keys in a map are of the same variant, as well as the query keys.
    ///
    /// For complex keys, the [`Mapping`] and [`Sequence`] variants are compared for equality. Both
    /// these comparisons are sensitive to the order of insertions. For instance, in the following
    /// mapping, the two complex keys are considered different:
    ///
    /// ```yaml
    /// ? { a: b, c: d }: foo
    /// ? { c: d, a: b }: bar
    /// ```
    ///
    /// [`Mapping`]: Yaml::Mapping
    /// [`Representation`]: Yaml::Representation
    /// [`Sequence`]: Yaml::Sequence
    /// [`Value`]: Yaml::Value
    /// [scalar style]: ScalarStyle
    /// [`OrderedFloat`]: ordered_float::OrderedFloat
    Mapping(MappingOwned),
    /// Alias, not fully supported yet.
    Alias(usize),
    /// A variant used when parsing the representation of a scalar node fails.
    ///
    /// The YAML is syntactically valid, but its contents are incoherent. See
    /// [`ScalarOwned::parse_from_cow_and_metadata`] for details.
    /// This variant is also used when stealing the contents of `self`, meaning `self` should no
    /// longer be used. See [`Self::take`] for details
    BadValue,
}

/// The type contained in the `YamlOwned::Sequence` variant.
pub type SequenceOwned = Vec<YamlOwned>;
/// The type contained in the `YamlOwned::Mapping` variant.
pub type MappingOwned = LinkedHashMap<YamlOwned, YamlOwned>;

// This defines most common operations on a YAML object. See macro definition for details.
define_yaml_object_impl!(
    YamlOwned,
    mappingtype = MappingOwned,
    sequencetype = SequenceOwned,
    nodetype = Self,
    scalartype = { ScalarOwned },
    selfname = "YAMLOwned",
    owned
);

impl YamlOwned {
    /// Implementation detail for [`Self::as_mapping_get`], which is generated from a macro.
    #[must_use]
    fn as_mapping_get_impl(&self, key: &str) -> Option<&Self> {
        match self.as_mapping() {
            Some(mapping) => {
                let hash = hash_str_as_yaml_string(key, mapping.hasher().build_hasher());
                mapping
                    .raw_entry()
                    .from_hash(hash, |k| k.as_str().is_some_and(|s| s == key))
                    .map(|(_, v)| v)
            }
            _ => None,
        }
    }

    /// Implementation detail for [`Self::as_mapping_get_mut`], which is generated from a macro.
    #[must_use]
    fn as_mapping_get_mut_impl(&mut self, key: &str) -> Option<&mut Self> {
        use hashlink::linked_hash_map::RawEntryMut::{Occupied, Vacant};
        match self.as_mapping_mut() {
            Some(mapping) => {
                let hash = hash_str_as_yaml_string(key, mapping.hasher().build_hasher());
                match mapping
                    .raw_entry_mut()
                    .from_hash(hash, |k| k.as_str().is_some_and(|s| s == key))
                {
                    Occupied(entry) => Some(entry.into_mut()),
                    Vacant(_) => None,
                }
            }
            _ => None,
        }
    }
}

impl LoadableYamlNode<'_> for YamlOwned {
    type HashKey = Self;

    fn from_bare_yaml(yaml: Yaml<'_>) -> Self {
        match yaml {
            // Sequence and Mapping will always have their container empty.
            Yaml::Sequence(_) => Self::Sequence(vec![]),
            Yaml::Mapping(_) => Self::Mapping(MappingOwned::new()),

            Yaml::Representation(cow, scalar_style, tag) => {
                Self::Representation(cow.into(), scalar_style, tag)
            }
            Yaml::Value(scalar) => Self::Value(scalar.into_owned()),
            Yaml::Alias(x) => Self::Alias(x),
            Yaml::BadValue => Self::BadValue,
        }
    }

    fn is_sequence(&self) -> bool {
        self.is_sequence()
    }

    fn is_mapping(&self) -> bool {
        self.is_mapping()
    }

    fn is_badvalue(&self) -> bool {
        self.is_badvalue()
    }

    fn sequence_mut(&mut self) -> &mut Vec<Self> {
        self.as_vec_mut()
            .expect("Called sequence_mut on a non-array")
    }

    fn mapping_mut(&mut self) -> &mut LinkedHashMap<Self::HashKey, Self> {
        self.as_mapping_mut()
            .expect("Called mapping_mut on a non-hash")
    }

    fn take(&mut self) -> Self {
        let mut taken_out = Self::BadValue;
        std::mem::swap(&mut taken_out, self);
        taken_out
    }
}

impl IntoIterator for YamlOwned {
    type Item = Self;
    type IntoIter = YamlOwnedIter;

    fn into_iter(self) -> Self::IntoIter {
        YamlOwnedIter {
            yaml: self.into_vec().unwrap_or_default().into_iter(),
        }
    }
}

/// An iterator over a [`YamlOwned`] node.
pub struct YamlOwnedIter {
    yaml: std::vec::IntoIter<YamlOwned>,
}

impl Iterator for YamlOwnedIter {
    type Item = YamlOwned;

    fn next(&mut self) -> Option<YamlOwned> {
        self.yaml.next()
    }
}

/// Hash the given `str` as if it were a [`ScalarOwned::String`] object.
fn hash_str_as_yaml_string<H: Hasher>(key: &str, mut hasher: H) -> u64 {
    use std::hash::Hash;
    let key = YamlOwned::Value(ScalarOwned::String(key.into()));
    key.hash(&mut hasher);
    hasher.finish()
}
