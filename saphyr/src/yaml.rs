//! YAML objects manipulation utilities.

#![allow(clippy::module_name_repetitions)]

use std::{
    borrow::Cow,
    convert::TryFrom,
    hash::{BuildHasher, Hasher},
    ops::{Index, IndexMut},
};

use hashlink::LinkedHashMap;
use saphyr_parser::{ScalarStyle, Tag};

use crate::{LoadableYamlNode, Scalar, YamlOwned};

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
pub enum Yaml<'input> {
    /// The raw string from the input.
    ///
    /// When the field is left in the [`Representation`] variant, methods that rely on the value
    /// (e.g.: [`is_boolean`], [`as_integer`], [`into_floating_point`], ...) will always return
    /// [`None`].
    ///
    /// Resolving the representation to its scalar value can either yield a [`Value`] or a
    /// [`Tagged`] variant, depending on whether the scalar is tagged.
    ///
    /// This variant is only meant:
    ///   - As an optimization, when lazy-parsing is preferred.
    ///   - As a more generic way of handling keys in [`Mapping`]s (if user-defined key duplication
    ///     detection is required).
    ///
    /// [`Mapping`]: Yaml::Mapping
    /// [`Representation`]: Yaml::Representation
    /// [`Value`]: Yaml::Value
    /// [`Tagged`]: Yaml::Tagged
    /// [`is_boolean`]: Yaml::is_boolean
    /// [`as_integer`]: Yaml::as_integer
    /// [`into_floating_point`]: Yaml::into_floating_point
    Representation(Cow<'input, str>, ScalarStyle, Option<Cow<'input, Tag>>),
    /// The resolved value from the representation.
    Value(Scalar<'input>),
    /// YAML sequence, can be accessed as a `Vec`.
    Sequence(Sequence<'input>),
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
    /// floating point values as keys. [`Scalar`] uses [`OrderedFloat`] for hash and equality.
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
    Mapping(Mapping<'input>),
    /// A tagged node.
    ///
    /// Tags can be applied to any node, whether a scalar or a collection.
    Tagged(Cow<'input, Tag>, Box<Yaml<'input>>),
    /// Alias, not fully supported yet.
    Alias(usize),
    /// A variant used when parsing the representation of a scalar node fails.
    ///
    /// The YAML is syntactically valid, but its contents are incoherent. See
    /// [`Scalar::parse_from_cow_and_metadata`] for details.
    /// This variant is also used when stealing the contents of `self`, meaning `self` should no
    /// longer be used. See [`Self::take`] for details
    BadValue,
}

/// The type contained in the `Yaml::Sequence` variant.
pub type Sequence<'input> = Vec<Yaml<'input>>;
/// The type contained in the `Yaml::Mapping` variant.
pub type Mapping<'input> = LinkedHashMap<Yaml<'input>, Yaml<'input>>;

// This defines most common operations on a YAML object. See macro definition for details.
define_yaml_object_impl!(
    Yaml<'input>,
    <'input>,
    mappingtype = Mapping<'input>,
    sequencetype = Sequence<'input>,
    nodetype = Self,
    scalartype = { Scalar },
    selfname = "YAML",
    borrowing
);

impl Yaml<'_> {
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

impl<'input> LoadableYamlNode<'input> for Yaml<'input> {
    type HashKey = Self;

    fn from_bare_yaml(yaml: Yaml<'input>) -> Self {
        yaml
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

    fn into_tagged(self, tag: Cow<'input, Tag>) -> Self {
        Self::Tagged(tag, Box::new(self))
    }

    fn take(&mut self) -> Self {
        let mut taken_out = Yaml::BadValue;
        std::mem::swap(&mut taken_out, self);
        taken_out
    }
}

impl<'input> IntoIterator for Yaml<'input> {
    type Item = Yaml<'input>;
    type IntoIter = YamlIter<'input>;

    fn into_iter(self) -> Self::IntoIter {
        YamlIter {
            yaml: self.into_vec().unwrap_or_default().into_iter(),
        }
    }
}

/// An iterator over a [`Yaml`] node.
pub struct YamlIter<'input> {
    yaml: std::vec::IntoIter<Yaml<'input>>,
}

impl<'input> Iterator for YamlIter<'input> {
    type Item = Yaml<'input>;

    fn next(&mut self) -> Option<Yaml<'input>> {
        self.yaml.next()
    }
}

/// Hash the given `str` as if it were a [`Scalar::String`] object.
fn hash_str_as_yaml_string<H: Hasher>(key: &str, mut hasher: H) -> u64 {
    use std::hash::Hash;
    let key = Yaml::Value(Scalar::String(key.into()));
    key.hash(&mut hasher);
    hasher.finish()
}

impl<'input> From<&'input YamlOwned> for Yaml<'input> {
    fn from(value: &'input YamlOwned) -> Self {
        match value {
            YamlOwned::Representation(str, scalar_style, tag) => Yaml::Representation(
                Cow::Borrowed(str),
                *scalar_style,
                tag.as_ref().map(Cow::Borrowed),
            ),
            YamlOwned::Value(scalar_owned) => Yaml::Value(scalar_owned.into()),
            YamlOwned::Sequence(yaml_owneds) => Yaml::Sequence(
                yaml_owneds
                    .iter()
                    .map(Into::into)
                    .collect::<Vec<Yaml<'input>>>(),
            ),
            YamlOwned::Mapping(linked_hash_map) => Yaml::Mapping(
                linked_hash_map
                    .iter()
                    .map(|(key, value)| (key.into(), value.into()))
                    .collect::<Mapping>(),
            ),
            YamlOwned::Tagged(tag, node) => {
                Yaml::Tagged(Cow::Borrowed(tag), Box::new(node.as_ref().into()))
            }
            YamlOwned::Alias(usize) => Yaml::Alias(*usize),
            YamlOwned::BadValue => Yaml::BadValue,
        }
    }
}
