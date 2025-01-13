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

use crate::{loader::parse_f64, YamlLoader};

/// A YAML node is stored as this `Yaml` enumeration, which provides an easy way to
/// access your YAML document.
///
/// # Examples
///
/// ```
/// use saphyr::Yaml;
/// let foo = Yaml::from_str("-123"); // convert the string to the appropriate YAML type
/// assert_eq!(foo.as_i64().unwrap(), -123);
///
/// // iterate over an Array
/// let vec = Yaml::Array(vec![Yaml::Integer(1), Yaml::Integer(2)]);
/// for v in vec.as_vec().unwrap() {
///     assert!(v.as_i64().is_some());
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord, Hash)]
pub enum Yaml<'input> {
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
    Array(Array<'input>),
    /// YAML hash, can be accessed as a `LinkedHashMap`.
    ///
    /// Insertion order will match the order of insertion into the map.
    Hash(Hash<'input>),
    /// Alias, not fully supported yet.
    Alias(usize),
    /// YAML null, e.g. `null` or `~`.
    Null,
    /// Accessing a nonexistent node via the Index trait returns `BadValue`. This
    /// simplifies error handling in the calling code. Invalid type conversion also
    /// returns `BadValue`.
    BadValue,
}

/// The type contained in the `Yaml::Array` variant. This corresponds to YAML sequences.
pub type Array<'input> = Vec<Yaml<'input>>;
/// The type contained in the `Yaml::Hash` variant. This corresponds to YAML mappings.
pub type Hash<'input> = LinkedHashMap<Yaml<'input>, Yaml<'input>>;

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
    /// use saphyr::Yaml;
    ///
    /// let docs = Yaml::load_from_str(r#"
    /// First document
    /// ---
    /// - Second document
    /// "#).unwrap();
    /// let first_document = &docs[0]; // Select the first YAML document
    /// // The document is a string containing "First document".
    /// assert_eq!(*first_document, Yaml::String("First document".into()));
    ///
    /// let second_document = &docs[1]; // Select the second YAML document
    /// // The document is an array containing a single string, "Second document".
    /// assert_eq!(second_document[0], Yaml::String("Second document".into()));
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

    define_as!(as_bool, bool, Boolean);
    define_as!(as_i64, i64, Integer);

    define_as_ref!(as_hash, &Hash, Hash);
    define_as_ref!(as_str, &str, String);
    define_as_ref!(as_vec, &Array, Array);

    define_as_mut_ref!(as_mut_hash, &mut Hash<'input>, Hash);
    define_as_mut_ref!(as_mut_vec, &mut Array<'input>, Array);

    define_into!(into_bool, bool, Boolean);
    define_into!(into_hash, Hash<'input>, Hash);
    define_into!(into_i64, i64, Integer);
    define_into!(into_vec, Array<'input>, Array);

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
    /// If the node is not a [`Yaml::Real`] YAML node or its contents is not a valid `f64` string,
    /// `None` is returned.
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
    /// If the node is not a [`Yaml::Real`] YAML node or its contents is not a valid `f64` string,
    /// `None` is returned.
    #[must_use]
    pub fn into_f64(self) -> Option<f64> {
        self.as_f64()
    }

    /// If a value is null or otherwise bad (see variants), consume it and
    /// replace it with a given value `other`. Otherwise, return self unchanged.
    ///
    /// ```
    /// use saphyr::Yaml;
    ///
    /// assert_eq!(Yaml::BadValue.or(Yaml::Integer(3)),  Yaml::Integer(3));
    /// assert_eq!(Yaml::Integer(3).or(Yaml::BadValue),  Yaml::Integer(3));
    /// ```
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
            Yaml::BadValue | Yaml::Null => other,
            this => this,
        }
    }
}

#[allow(clippy::should_implement_trait)]
impl<'input> Yaml<'input> {
    /// Convert a string to a [`Yaml`] node.
    ///
    /// [`Yaml`] does not implement [`std::str::FromStr`] since conversion may not fail. This
    /// function falls back to [`Yaml::String`] if nothing else matches.
    ///
    /// # Examples
    /// ```
    /// # use saphyr::Yaml;
    /// assert!(matches!(Yaml::from_str("42"), Yaml::Integer(42)));
    /// assert!(matches!(Yaml::from_str("0x2A"), Yaml::Integer(42)));
    /// assert!(matches!(Yaml::from_str("0o52"), Yaml::Integer(42)));
    /// assert!(matches!(Yaml::from_str("~"), Yaml::Null));
    /// assert!(matches!(Yaml::from_str("null"), Yaml::Null));
    /// assert!(matches!(Yaml::from_str("true"), Yaml::Boolean(true)));
    /// assert!(matches!(Yaml::from_str("3.14"), Yaml::Real(_)));
    /// assert!(matches!(Yaml::from_str("foo"), Yaml::String(_)));
    /// ```
    #[must_use]
    pub fn from_str(v: &'input str) -> Yaml {
        Self::from_cow(v.into())
    }

    /// Same as [`Self::from_str`] but uses a [`Cow`] instead.
    #[must_use]
    pub fn from_cow(v: Cow<'input, str>) -> Yaml {
        if let Some(number) = v.strip_prefix("0x") {
            if let Ok(i) = i64::from_str_radix(number, 16) {
                return Yaml::Integer(i);
            }
        } else if let Some(number) = v.strip_prefix("0o") {
            if let Ok(i) = i64::from_str_radix(number, 8) {
                return Yaml::Integer(i);
            }
        } else if let Some(number) = v.strip_prefix('+') {
            if let Ok(i) = number.parse::<i64>() {
                return Yaml::Integer(i);
            }
        }
        match &*v {
            "~" | "null" => Yaml::Null,
            "true" => Yaml::Boolean(true),
            "false" => Yaml::Boolean(false),
            _ => {
                if let Ok(integer) = v.parse::<i64>() {
                    Yaml::Integer(integer)
                } else if parse_f64(&v).is_some() {
                    Yaml::Real(v)
                } else {
                    Yaml::String(v)
                }
            }
        }
    }
}

static BAD_VALUE: Yaml = Yaml::BadValue;
impl<'input, 'a> Index<&'a str> for Yaml<'input>
where
    'input: 'a,
{
    type Output = Yaml<'input>;

    fn index(&self, idx: &'a str) -> &Yaml<'input> {
        match self {
            Yaml::Hash(h) => {
                let hash = hash_str_as_yaml_string(idx, h.hasher().build_hasher());
                h.raw_entry()
                    .from_hash(hash, |k| matches!(k, Yaml::String(v) if v == idx))
                    .map_or(&BAD_VALUE, |(_, v)| v)
            }
            _ => &BAD_VALUE,
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
    /// This function also panics if `self` is not a [`Yaml::Hash`].
    fn index_mut(&mut self, idx: &'a str) -> &mut Yaml<'input> {
        use hashlink::linked_hash_map::RawEntryMut::{Occupied, Vacant};
        match self.as_mut_hash() {
            Some(h) => {
                let hash = hash_str_as_yaml_string(idx, h.hasher().build_hasher());
                match h
                    .raw_entry_mut()
                    .from_hash(hash, |k| matches!(k, Yaml::String(v) if v == idx))
                {
                    Occupied(entry) => entry.into_mut(),
                    Vacant(_) => panic!("Key '{idx}' not found in YAML hash"),
                }
            }
            None => panic!("Not a hash type"),
        }
    }
}

impl<'input> Index<usize> for Yaml<'input>
where
    Self: 'input,
{
    type Output = Yaml<'input>;

    fn index(&self, idx: usize) -> &Self::Output {
        match self {
            Yaml::Array(sequence) => sequence.get(idx).unwrap_or(&BAD_VALUE),
            Yaml::Hash(mapping) => {
                if let Ok(idx) = i64::try_from(idx) {
                    let hash = hash_i64_as_yaml_integer(idx, mapping.hasher().build_hasher());
                    mapping
                        .raw_entry()
                        .from_hash(hash, |k| matches!(k, Yaml::Integer(v) if *v == idx))
                        .map_or(&BAD_VALUE, |(_, v)| v)
                } else {
                    &BAD_VALUE
                }
            }
            _ => &BAD_VALUE,
        }
    }
}

impl<'input> IndexMut<usize> for Yaml<'input> {
    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`IndexMut`]). If `self` is
    /// a [`Yaml::Array`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`Yaml::Hash`], this is when the mapping sequence does not
    /// contain [`Yaml::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`Yaml::Array`] nor a [`Yaml::Hash`].
    fn index_mut(&mut self, idx: usize) -> &mut Yaml<'input> {
        match self {
            Yaml::Array(sequence) => sequence.index_mut(idx),
            Yaml::Hash(mapping) => {
                let key = Yaml::Integer(i64::try_from(idx).unwrap());
                mapping.get_mut(&key).unwrap()
            }
            _ => panic!("Attempting to index but `self` is not a sequence nor a mapping"),
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

/// Hash the given `str` as if it were a [`Yaml::String`] object.
fn hash_str_as_yaml_string<H: Hasher>(key: &str, mut hasher: H) -> u64 {
    use std::hash::Hash;
    let key = Yaml::String(key.into());
    key.hash(&mut hasher);
    hasher.finish()
}

/// Hash the given `i64` as if it were a [`Yaml::Integer`] object.
fn hash_i64_as_yaml_integer<H: Hasher>(key: i64, mut hasher: H) -> u64 {
    use std::hash::Hash;
    let key = Yaml::Integer(key);
    key.hash(&mut hasher);
    hasher.finish()
}
