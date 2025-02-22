use std::{
    borrow::Cow,
    hash::{BuildHasher, Hasher},
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use hashlink::LinkedHashMap;
use saphyr_parser::{ScalarStyle, Tag};

use crate::{annotated::AnnotatedNode, Scalar};

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
/// [`Yaml`]: crate::Yaml
/// [`MarkedYaml`]: crate::annotated::marked_yaml::MarkedYaml
#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord, Hash)]
pub enum YamlData<'input, Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self> + AnnotatedNode,
{
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
    /// [`Mapping`]: YamlData::Mapping
    /// [`Representation`]: YamlData::Representation
    /// [`is_boolean`]: YamlData::is_boolean
    /// [`as_integer`]: YamlData::as_integer
    /// [`into_floating_point`]: YamlData::into_floating_point
    Representation(Cow<'input, str>, ScalarStyle, Option<Tag>),
    /// The resolved value from the representation.
    Value(Scalar<'input>),
    /// YAML sequence, can be accessed as a `Vec`.
    Sequence(AnnotatedSequence<Node>),
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
    /// [`Mapping`]: YamlData::Mapping
    /// [`Representation`]: YamlData::Representation
    /// [`Sequence`]: YamlData::Sequence
    /// [`Value`]: YamlData::Value
    /// [scalar style]: ScalarStyle
    /// [`OrderedFloat`]: ordered_float::OrderedFloat
    Mapping(AnnotatedMapping<'input, Node>),
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

// This defines most common operations on a YAML object. See macro definition for details.
define_yaml_object_impl!(
    YamlData<'input, Node>,
    < 'input, Node>,
    where {
        Node: std::hash::Hash
            + std::cmp::Eq
            + From<Self>
            + AnnotatedNode
            + for<'a> std::cmp::PartialEq<Node::HashKey<'a>>,
    },
    mappingtype = AnnotatedMapping<'input, Node>,
    sequencetype = AnnotatedSequence<Node>,
    nodetype = Node,
    scalartype = { Scalar },
    selfname = "YamlData",
    borrowing
);

impl<'input, Node> YamlData<'input, Node>
where
    Node: std::hash::Hash
        + std::cmp::Eq
        + From<Self>
        + AnnotatedNode
        + for<'a> std::cmp::PartialEq<Node::HashKey<'a>>,
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
            Self::Mapping(mapping) => {
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

impl<'input, Node> IntoIterator for YamlData<'input, Node>
where
    Node: std::hash::Hash
        + std::cmp::Eq
        + From<Self>
        + AnnotatedNode
        + for<'a> std::cmp::PartialEq<Node::HashKey<'a>>,
{
    type Item = Node;
    type IntoIter = AnnotatedYamlIter<'input, Node>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            yaml: self.into_vec().unwrap_or_default().into_iter(),
            marker: PhantomData,
        }
    }
}

/// An iterator over a [`YamlData`] node.
#[allow(clippy::module_name_repetitions)]
pub struct AnnotatedYamlIter<'input, Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<YamlData<'input, Node>> + AnnotatedNode,
{
    yaml: std::vec::IntoIter<Node>,
    marker: PhantomData<&'input ()>,
}

impl<'input, Node> Iterator for AnnotatedYamlIter<'input, Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<YamlData<'input, Node>> + AnnotatedNode,
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
pub type AnnotatedMapping<'input, Node> = LinkedHashMap<Node, Node>;
