//! Utilities for extracting YAML with certain metadata.

pub mod marked_yaml;

use std::{
    borrow::Cow,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use hashlink::LinkedHashMap;

use crate::loader::parse_f64;

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
/// the `Array` and `Hash` variants is dependant on that structure.
///
/// If we had written [`YamlData`] as:
/// ```ignore
/// pub enum YamlData {
///   // ...
///   Array(Vec<Yaml>),
///   Hash(LinkedHashMap<Yaml, Yaml>),
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
pub enum YamlData<'input, Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    /// Float types are stored as String and parsed on demand.
    /// Note that `f64` does NOT implement Eq trait and can NOT be stored in `BTreeMap`.
    Real(Cow<'input, str>),
    /// YAML int is stored as i64.
    Integer(i64),
    /// YAML scalar.
    String(Cow<'input, str>),
    /// YAML bool, e.g. `true` or `false`.
    Boolean(bool),
    /// YAML array, can be accessed as a `Vec`.
    Array(AnnotatedArray<Node>),
    /// YAML hash, can be accessed as a `LinkedHashMap`.
    ///
    /// Insertion order will match the order of insertion into the map.
    Hash(AnnotatedHash<Node>),
    /// Alias, not fully supported yet.
    Alias(usize),
    /// YAML null, e.g. `null` or `~`.
    Null,
    /// Accessing a nonexistent node via the Index trait returns `BadValue`. This
    /// simplifies error handling in the calling code. Invalid type conversion also
    /// returns `BadValue`.
    BadValue,
}

/// The type contained in the [`YamlData::Array`] variant. This corresponds to YAML sequences.
#[allow(clippy::module_name_repetitions)]
pub type AnnotatedArray<Node> = Vec<Node>;
/// The type contained in the [`YamlData::Hash`] variant. This corresponds to YAML mappings.
#[allow(clippy::module_name_repetitions)]
pub type AnnotatedHash<Node> = LinkedHashMap<Node, Node>;

impl<'input, Node> YamlData<'input, Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    define_as!(as_bool, bool, Boolean);
    define_as!(as_i64, i64, Integer);

    define_as_ref!(as_hash, &AnnotatedHash<Node>, Hash);
    define_as_ref!(as_str, &str, String);
    define_as_ref!(as_vec, &AnnotatedArray<Node>, Array);

    define_as_mut_ref!(as_mut_hash, &mut AnnotatedHash<Node>, Hash);
    define_as_mut_ref!(as_mut_vec, &mut AnnotatedArray<Node>, Array);

    define_into!(into_bool, bool, Boolean);
    define_into!(into_hash, AnnotatedHash<Node>, Hash);
    define_into!(into_i64, i64, Integer);
    define_into!(into_vec, AnnotatedArray<Node>, Array);

    define_is!(is_alias, Self::Alias(_));
    define_is!(is_array, Self::Array(_));
    define_is!(is_badvalue, Self::BadValue);
    define_is!(is_boolean, Self::Boolean(_));
    define_is!(is_hash, Self::Hash(_));
    define_is!(is_integer, Self::Integer(_));
    define_is!(is_null, Self::Null);
    define_is!(is_real, Self::Real(_));
    define_is!(is_string, Self::String(_));

    /// Get the inner object in the YAML enum if it is a [`String`].
    ///
    /// # Return
    /// If the variant of `self` is `Self::String`, return `Some(String)` with the `String`
    /// contained. Otherwise, return `None`.
    #[must_use]
    pub fn into_string(self) -> Option<String> {
        // We can't use the macro for this variant as we need to `.into_owned` the `Cow`.
        match self {
            Self::String(v) => Some(v.into_owned()),
            _ => None,
        }
    }

    /// Return the `f64` value contained in this YAML node.
    ///
    /// If the node is not a [`YamlData::Real`] YAML node or its contents is not a valid `f64`
    /// string, `None` is returned.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        if let Self::Real(ref v) = self {
            parse_f64(v)
        } else {
            None
        }
    }

    /// Return the `f64` value contained in this YAML node.
    ///
    /// If the node is not a [`YamlData::Real`] YAML node or its contents is not a valid `f64`
    /// string, `None` is returned.
    #[must_use]
    pub fn into_f64(self) -> Option<f64> {
        self.as_f64()
    }

    /// If a value is null or otherwise bad (see variants), consume it and
    /// replace it with a given value `other`. Otherwise, return self unchanged.
    ///
    /// See [`Yaml::or`] for examples.
    ///
    /// [`Yaml::or`]: crate::Yaml::or
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        match self {
            Self::BadValue | Self::Null => other,
            this => this,
        }
    }

    /// See [`Self::or`] for behavior.
    ///
    /// This performs the same operations, but with borrowed values for less linear pipelines.
    #[must_use]
    pub fn borrowed_or<'a>(&'a self, other: &'a Self) -> &'a Self {
        match self {
            Self::BadValue | Self::Null => other,
            this => this,
        }
    }
}

// NOTE(ethiraric, 10/06/2024): We cannot create a "generic static" variable which would act as a
// `BAD_VALUE`. This means that, unlike for `Yaml`, we have to make the indexing method panic.

impl<'input, 'a, Node> Index<&'a str> for YamlData<'input, Node>
where
    'input: 'a,
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    type Output = Node;

    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`YamlData::Hash`].
    fn index(&self, idx: &'a str) -> &Node {
        match self {
            YamlData::Hash(_h) => {
                unimplemented!("Cannot get lifetimes to work");
                // let hash = hash_str_as_yaml_string::<'a, Node, _>(idx, h.hasher().build_hasher());
                // let expect = Node::from(YamlData::String(idx.into()));
                // h.raw_entry()
                //     .from_hash(hash, |k| *k == expect)
                //     .map(|(_, v)| v)
                //     .expect("key does not exist")
            }
            _ => panic!("trying to index a non-hash YamlData with string '{idx}'"),
        }
    }
}

impl<'input, 'a, Node> IndexMut<&'a str> for YamlData<'input, Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
    'input: 'a,
{
    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`YamlData::Hash`].
    fn index_mut(&mut self, _idx: &'a str) -> &mut Node {
        match self.as_mut_hash() {
            Some(_h) => unimplemented!("Cannot get lifetimes to work"),
            None => panic!("Not a hash type"),
        }
    }
}

impl<'input, Node> Index<usize> for YamlData<'input, Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    type Output = Node;

    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`Index`]). If `self` is a
    /// [`YamlData::Array`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`YamlData::Hash`], this is when the mapping sequence does
    /// not contain [`YamlData::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`YamlData::Array`] nor a [`YamlData::Hash`].
    fn index(&self, idx: usize) -> &Node {
        if let Some(v) = self.as_vec() {
            v.get(idx).unwrap()
        } else if let Some(v) = self.as_hash() {
            let key = Self::Integer(i64::try_from(idx).unwrap());
            v.get(&key.into()).unwrap()
        } else {
            panic!("{idx}: Index out of bounds");
        }
    }
}

impl<'input, Node> IndexMut<usize> for YamlData<'input, Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`IndexMut`]). If `self` is
    /// a [`YamlData::Array`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`YamlData::Hash`], this is when the mapping sequence does
    /// not contain [`YamlData::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`YamlData::Array`] nor a [`YamlData::Hash`].
    fn index_mut(&mut self, idx: usize) -> &mut Node {
        match self {
            Self::Array(sequence) => sequence.index_mut(idx),
            Self::Hash(mapping) => {
                let key = Self::Integer(i64::try_from(idx).unwrap());
                mapping.get_mut(&key.into()).unwrap()
            }
            _ => panic!("Attempting to index but `self` is not a sequence nor a mapping"),
        }
    }
}

impl<'input, Node> IntoIterator for YamlData<'input, Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
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
    Node: std::hash::Hash + std::cmp::Eq + From<YamlData<'input, Node>>,
{
    yaml: std::vec::IntoIter<Node>,
    marker: PhantomData<&'input u32>,
}

impl<'input, Node> Iterator for AnnotatedYamlIter<'input, Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<YamlData<'input, Node>>,
{
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        self.yaml.next()
    }
}
