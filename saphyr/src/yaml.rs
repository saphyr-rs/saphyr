//! YAML objects manipulation utilities.

#![allow(clippy::module_name_repetitions)]

use std::{
    borrow::Cow,
    convert::TryFrom,
    hash::{BuildHasher, Hasher},
    ops::{Index, IndexMut},
};

use hashlink::LinkedHashMap;
use saphyr_parser::{BufferedInput, Input, Parser, ScalarStyle, ScanError, Tag};

use crate::{LoadableYamlNode, Scalar, YamlLoader};

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
    Representation(Cow<'input, str>, ScalarStyle, Option<Tag>),
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
    selfname = "YAML"
);

impl<'input> Yaml<'input> {
    /// Load the given string as an array of YAML documents.
    ///
    /// The `source` is interpreted as YAML documents and is parsed. Parsing succeeds if and only
    /// if all documents are parsed successfully. An error in a latter document prevents the former
    /// from being returned.
    ///
    /// Most often, only one document is loaded in a YAML string. In this case, only the first element
    /// of the returned `Vec` will be used. Otherwise, each element in the `Vec` is a document:
    ///
    /// ```
    /// use saphyr::{Scalar, Yaml};
    ///
    /// let docs = Yaml::load_from_str(r#"
    /// First document
    /// ---
    /// - Second document
    /// "#).unwrap();
    /// let first_document = &docs[0]; // Select the first YAML document
    /// // The document is a string containing "First document".
    /// assert_eq!(*first_document, Yaml::Value(Scalar::String("First document".into())));
    ///
    /// let second_document = &docs[1]; // Select the second YAML document
    /// // The document is an array containing a single string, "Second document".
    /// assert_eq!(second_document[0], Yaml::Value(Scalar::String("Second document".into())));
    /// ```
    ///
    /// # Errors
    /// Returns [`ScanError`] when loading fails.
    pub fn load_from_str(source: &str) -> Result<Vec<Self>, ScanError> {
        Self::load_from_iter(source.chars())
    }

    /// Load the contents of the given iterator as an array of YAML documents.
    ///
    /// See [`Self::load_from_str`] for details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    pub fn load_from_iter<I: Iterator<Item = char>>(
        source: I,
    ) -> Result<Vec<Yaml<'input>>, ScanError> {
        let mut parser = Parser::new(BufferedInput::new(source));
        Self::load_from_parser(&mut parser)
    }

    /// Load the contents from the specified [`Parser`] as an array of YAML documents.
    ///
    /// See [`Self::load_from_str`] for details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    pub fn load_from_parser<I: Input>(
        parser: &mut Parser<'input, I>,
    ) -> Result<Vec<Yaml<'input>>, ScanError> {
        let mut loader = YamlLoader::default();
        parser.load(&mut loader, true)?;
        Ok(loader.into_documents())
    }

    /// Convert a string to a [`Yaml`] scalar node.
    ///
    /// [`Yaml`] does not implement [`std::str::FromStr`] since the trait requires that conversion
    /// does not fail. This function attempts to parse the given string as a scalar node, falling
    /// back to a [`Scalar::String`].
    ///
    /// **Note:** This attempts to resolve the content as a scalar node. This means that `"a: b"`
    /// gets resolved to `Yaml::Value(Scalar::String("a: b"))` and not a mapping. If you want to
    /// parse a YAML document, use [`load_from_str`].
    ///
    /// # Examples
    /// ```
    /// # use saphyr::{Scalar, Yaml};
    /// assert!(matches!(Yaml::value_from_str("42"),   Yaml::Value(Scalar::Integer(42))));
    /// assert!(matches!(Yaml::value_from_str("0x2A"), Yaml::Value(Scalar::Integer(42))));
    /// assert!(matches!(Yaml::value_from_str("0o52"), Yaml::Value(Scalar::Integer(42))));
    /// assert!(matches!(Yaml::value_from_str("~"),    Yaml::Value(Scalar::Null)));
    /// assert!(matches!(Yaml::value_from_str("null"), Yaml::Value(Scalar::Null)));
    /// assert!(matches!(Yaml::value_from_str("true"), Yaml::Value(Scalar::Boolean(true))));
    /// assert!(matches!(Yaml::value_from_str("3.14"), Yaml::Value(Scalar::FloatingPoint(_))));
    /// assert!(matches!(Yaml::value_from_str("foo"),  Yaml::Value(Scalar::String(_))));
    /// ```
    ///
    /// [`load_from_str`]: Self::load_from_str
    #[must_use]
    pub fn value_from_str(v: &'input str) -> Self {
        Self::value_from_cow(v.into())
    }

    /// Same as [`Self::value_from_str`] but uses a [`String`] instead.
    #[must_use]
    pub fn scalar_from_string(v: String) -> Self {
        Self::value_from_cow(v.into())
    }

    /// Same as [`Self::value_from_str`] but uses a [`Cow`] instead.
    #[must_use]
    pub fn value_from_cow(v: Cow<'input, str>) -> Yaml<'input> {
        Self::Value(Scalar::parse_from_cow(v))
    }

    /// Convert a string to a [`Yaml`] scalar node, abiding by the given metadata.
    ///
    /// The variant returned by this function will always be a [`Yaml::Value`], unless the tag
    /// forces a particular type and the representation cannot be parsed as this type, in which
    /// case it returns a [`Yaml::BadValue`].
    #[must_use]
    pub fn value_from_cow_and_metadata(
        v: Cow<'input, str>,
        style: ScalarStyle,
        tag: Option<&Tag>,
    ) -> Self {
        Scalar::parse_from_cow_and_metadata(v, style, tag).map_or(Yaml::BadValue, Yaml::Value)
    }

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
