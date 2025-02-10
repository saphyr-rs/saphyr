//! YAML objects manipulation utilities.

#![allow(clippy::module_name_repetitions)]

use std::{
    borrow::Cow,
    convert::TryFrom,
    hash::{BuildHasher, Hasher},
    ops::{Index, IndexMut},
};

use hashlink::LinkedHashMap;
use saphyr_parser::{BufferedInput, Input, Parser, ScanError};

use crate::{loader::parse_f64, LoadableYamlNode, Scalar, YamlLoader};

/// A YAML node is stored as this `Yaml` enumeration, which provides an easy way to
/// access your YAML document.
///
/// # Examples
///
/// ```
/// use saphyr::{Scalar, Yaml};
/// let foo = Yaml::from_str("-123"); // convert the string to the appropriate YAML type
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
    Representation(Cow<'input, str>),
    /// The resolved value from the representation.
    Value(Scalar<'input>),
    /// YAML sequence, can be accessed as a `Vec`.
    Sequence(Sequence<'input>),
    /// YAML mapping, can be accessed as a `LinkedHashMap`.
    ///
    /// Insertion order will match the order of insertion into the map.
    Mapping(Mapping<'input>),
    /// Alias, not fully supported yet.
    Alias(usize),
    /// Accessing a nonexistent node via the Index trait returns `BadValue`. This
    /// simplifies error handling in the calling code. Invalid type conversion also
    /// returns `BadValue`.
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
    nodetype = Self
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
    /// Returns `ScanError` when loading fails.
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

#[allow(clippy::should_implement_trait)]
impl<'input> Yaml<'input> {
    /// Convert a string to a [`Yaml`] node.
    ///
    /// [`Yaml`] does not implement [`std::str::FromStr`] since the trait requires that conversion
    /// does not fail. This function attempts to parse the given string as a scalar node, falling
    /// back to a [`Scalar::String`].
    ///
    /// # Examples
    /// ```
    /// # use saphyr::{Scalar, Yaml};
    /// assert!(matches!(Yaml::from_str("42"),   Yaml::Value(Scalar::Integer(42))));
    /// assert!(matches!(Yaml::from_str("0x2A"), Yaml::Value(Scalar::Integer(42))));
    /// assert!(matches!(Yaml::from_str("0o52"), Yaml::Value(Scalar::Integer(42))));
    /// assert!(matches!(Yaml::from_str("~"),    Yaml::Value(Scalar::Null)));
    /// assert!(matches!(Yaml::from_str("null"), Yaml::Value(Scalar::Null)));
    /// assert!(matches!(Yaml::from_str("true"), Yaml::Value(Scalar::Boolean(true))));
    /// assert!(matches!(Yaml::from_str("3.14"), Yaml::Value(Scalar::FloatingPoint(_))));
    /// assert!(matches!(Yaml::from_str("foo"),  Yaml::Value(Scalar::String(_))));
    /// ```
    #[must_use]
    pub fn from_str(v: &'input str) -> Self {
        Self::from_cow(v.into())
    }

    /// Same as [`Self::from_str`] but uses a [`Cow`] instead.
    #[must_use]
    pub fn from_cow(v: Cow<'input, str>) -> Yaml {
        if let Some(number) = v.strip_prefix("0x") {
            if let Ok(i) = i64::from_str_radix(number, 16) {
                return Yaml::Value(Scalar::Integer(i));
            }
        } else if let Some(number) = v.strip_prefix("0o") {
            if let Ok(i) = i64::from_str_radix(number, 8) {
                return Yaml::Value(Scalar::Integer(i));
            }
        } else if let Some(number) = v.strip_prefix('+') {
            if let Ok(i) = number.parse::<i64>() {
                return Yaml::Value(Scalar::Integer(i));
            }
        }
        match &*v {
            "~" | "null" => Yaml::Value(Scalar::Null),
            "true" => Yaml::Value(Scalar::Boolean(true)),
            "false" => Yaml::Value(Scalar::Boolean(false)),
            _ => {
                if let Ok(integer) = v.parse::<i64>() {
                    Yaml::Value(Scalar::Integer(integer))
                } else if let Some(float) = parse_f64(&v) {
                    Yaml::Value(Scalar::FloatingPoint(float.into()))
                } else {
                    Yaml::Value(Scalar::String(v))
                }
            }
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

impl<'input, 'a> Index<&'a str> for Yaml<'input>
where
    'input: 'a,
{
    type Output = Yaml<'input>;

    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`Yaml::Mapping`].
    fn index(&self, idx: &'a str) -> &Self::Output {
        match self.as_mapping_get_impl(idx) {
            Some(value) => value,
            None => {
                if matches!(self, Self::Mapping(_)) {
                    panic!("Key '{idx}' not found in YAML mapping")
                } else {
                    panic!("Attempt to index YAML with '{idx}' but it's not a mapping")
                }
            }
        }
    }
}

impl<'input, 'a> IndexMut<&'a str> for Yaml<'input>
where
    'input: 'a,
{
    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`Yaml::Mapping`].
    fn index_mut(&mut self, idx: &'a str) -> &mut Yaml<'input> {
        assert!(
            matches!(self, Self::Mapping(_)),
            "Attempt to index YAML with '{idx}' but it's not a mapping"
        );
        match self.as_mapping_get_mut_impl(idx) {
            Some(value) => value,
            None => {
                panic!("Key '{idx}' not found in YAML mapping")
            }
        }
    }
}

impl<'input> Index<usize> for Yaml<'input>
where
    Self: 'input,
{
    type Output = Yaml<'input>;

    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`IndexMut`]). If `self` is
    /// a [`Yaml::Sequence`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`Yaml::Mapping`], this is when the mapping sequence does
    /// not contain [`Scalar::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`Yaml::Sequence`] nor a [`Yaml::Mapping`].
    fn index(&self, idx: usize) -> &Self::Output {
        match self {
            Yaml::Sequence(sequence) => sequence
                .get(idx)
                .unwrap_or_else(|| panic!("Index {idx} out of bounds in YAML sequence")),
            Yaml::Mapping(mapping) => {
                let idx = i64::try_from(idx).unwrap_or_else(|_| {
                    panic!("Attempt to index YAML sequence with overflowing index")
                });
                let hash = hash_i64_as_yaml_integer(idx, mapping.hasher().build_hasher());
                mapping
                    .raw_entry()
                    .from_hash(hash, |k| k.as_integer().is_some_and(|v| v == idx))
                    .map_or_else(|| panic!("Key {idx} not found in YAML mapping"), |(_, v)| v)
            }
            _ => {
                panic!("Attempt to index YAML with {idx} but it's not a mapping nor a sequence")
            }
        }
    }
}

impl<'input> IndexMut<usize> for Yaml<'input> {
    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`IndexMut`]). If `self` is
    /// a [`Yaml::Sequence`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`Yaml::Mapping`], this is when the mapping sequence does
    /// not contain [`Scalar::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`Yaml::Sequence`] nor a [`Yaml::Mapping`].
    fn index_mut(&mut self, idx: usize) -> &mut Yaml<'input> {
        match self {
            Yaml::Sequence(sequence) => sequence
                .get_mut(idx)
                .unwrap_or_else(|| panic!("Index {idx} out of bounds in YAML sequence")),
            Yaml::Mapping(mapping) => {
                let idx = i64::try_from(idx).unwrap_or_else(|_| {
                    panic!("Attempt to index YAML sequence with overflowing index")
                });
                mapping.get_mut(&Yaml::Value(Scalar::Integer(idx))).unwrap()
            }
            _ => {
                panic!("Attempt to index YAML with {idx} but it's not a mapping nor a sequence")
            }
        }
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

/// Hash the given `i64` as if it were a [`Scalar::Integer`] object.
fn hash_i64_as_yaml_integer<H: Hasher>(key: i64, mut hasher: H) -> u64 {
    use std::hash::Hash;
    let key = Yaml::Value(Scalar::Integer(key));
    key.hash(&mut hasher);
    hasher.finish()
}
