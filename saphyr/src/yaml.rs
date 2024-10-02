//! YAML objects manipulation utilities.

#![allow(clippy::module_name_repetitions)]

use std::{convert::TryFrom, ops::Index, ops::IndexMut};

use hashlink::LinkedHashMap;
use saphyr_parser::{Parser, ScanError};

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
pub enum Yaml {
    /// Float types are stored as String and parsed on demand.
    /// Note that `f64` does NOT implement Eq trait and can NOT be stored in `BTreeMap`.
    Real(String),
    /// YAML int is stored as i64.
    Integer(i64),
    /// YAML scalar.
    String(String),
    /// YAML bool, e.g. `true` or `false`.
    Boolean(bool),
    /// YAML array, can be accessed as a `Vec`.
    Array(Array),
    /// YAML hash, can be accessed as a `LinkedHashMap`.
    ///
    /// Insertion order will match the order of insertion into the map.
    Hash(Hash),
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
pub type Array = Vec<Yaml>;
/// The type contained in the `Yaml::Hash` variant. This corresponds to YAML mappings.
pub type Hash = LinkedHashMap<Yaml, Yaml>;

impl Yaml {
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
    /// assert_eq!(*first_document, Yaml::String("First document".to_owned()));
    ///
    /// let second_document = &docs[1]; // Select the second YAML document
    /// // The document is an array containing a single string, "Second document".
    /// assert_eq!(second_document[0], Yaml::String("Second document".to_owned()));
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
    pub fn load_from_iter<I: Iterator<Item = char>>(source: I) -> Result<Vec<Yaml>, ScanError> {
        let mut parser = Parser::new(source);
        Self::load_from_parser(&mut parser)
    }

    /// Load the contents from the specified [`Parser`] as an array of YAML documents.
    ///
    /// See [`Self::load_from_str`] for details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    pub fn load_from_parser<I: Iterator<Item = char>>(
        parser: &mut Parser<I>,
    ) -> Result<Vec<Yaml>, ScanError> {
        let mut loader = YamlLoader::default();
        parser.load(&mut loader, true)?;
        Ok(loader.into_documents())
    }

    define_as!(as_bool, bool, Boolean);
    define_as!(as_i64, i64, Integer);

    define_as_ref!(as_hash, &Hash, Hash);
    define_as_ref!(as_str, &str, String);
    define_as_ref!(as_vec, &Array, Array);

    define_as_mut_ref!(as_mut_hash, &mut Hash, Hash);
    define_as_mut_ref!(as_mut_vec, &mut Array, Array);

    define_into!(into_bool, bool, Boolean);
    define_into!(into_hash, Hash, Hash);
    define_into!(into_i64, i64, Integer);
    define_into!(into_string, String, String);
    define_into!(into_vec, Array, Array);

    define_is!(is_alias, Self::Alias(_));
    define_is!(is_array, Self::Array(_));
    define_is!(is_badvalue, Self::BadValue);
    define_is!(is_boolean, Self::Boolean(_));
    define_is!(is_hash, Self::Hash(_));
    define_is!(is_integer, Self::Integer(_));
    define_is!(is_null, Self::Null);
    define_is!(is_real, Self::Real(_));
    define_is!(is_string, Self::String(_));

    /// Return the `f64` value contained in this YAML node.
    ///
    /// If the node is not a [`Yaml::Real`] YAML node or its contents is not a valid `f64` string,
    /// `None` is returned.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        if let Yaml::Real(ref v) = self {
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
            Yaml::BadValue | Yaml::Null => other,
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
impl Yaml {
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
    pub fn from_str(v: &str) -> Yaml {
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
        match v {
            "~" | "null" => Yaml::Null,
            "true" => Yaml::Boolean(true),
            "false" => Yaml::Boolean(false),
            _ => {
                if let Ok(integer) = v.parse::<i64>() {
                    Yaml::Integer(integer)
                } else if parse_f64(v).is_some() {
                    Yaml::Real(v.to_owned())
                } else {
                    Yaml::String(v.to_owned())
                }
            }
        }
    }
}

static BAD_VALUE: Yaml = Yaml::BadValue;
impl<'a> Index<&'a str> for Yaml {
    type Output = Yaml;

    fn index(&self, idx: &'a str) -> &Yaml {
        let key = Yaml::String(idx.to_owned());
        match self.as_hash() {
            Some(h) => h.get(&key).unwrap_or(&BAD_VALUE),
            None => &BAD_VALUE,
        }
    }
}

impl<'a> IndexMut<&'a str> for Yaml {
    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`Yaml::Hash`].
    fn index_mut(&mut self, idx: &'a str) -> &mut Yaml {
        let key = Yaml::String(idx.to_owned());
        match self.as_mut_hash() {
            Some(h) => h.get_mut(&key).unwrap(),
            None => panic!("Not a hash type"),
        }
    }
}

impl Index<usize> for Yaml {
    type Output = Yaml;

    fn index(&self, idx: usize) -> &Yaml {
        if let Some(v) = self.as_vec() {
            v.get(idx).unwrap_or(&BAD_VALUE)
        } else if let Some(v) = self.as_hash() {
            let key = Yaml::Integer(i64::try_from(idx).unwrap());
            v.get(&key).unwrap_or(&BAD_VALUE)
        } else {
            &BAD_VALUE
        }
    }
}

impl IndexMut<usize> for Yaml {
    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`IndexMut`]). If `self` is
    /// a [`Yaml::Array`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`Yaml::Hash`], this is when the mapping sequence does not
    /// contain [`Yaml::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`Yaml::Array`] nor a [`Yaml::Hash`].
    fn index_mut(&mut self, idx: usize) -> &mut Yaml {
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

impl IntoIterator for Yaml {
    type Item = Yaml;
    type IntoIter = YamlIter;

    fn into_iter(self) -> Self::IntoIter {
        YamlIter {
            yaml: self.into_vec().unwrap_or_default().into_iter(),
        }
    }
}

/// An iterator over a [`Yaml`] node.
pub struct YamlIter {
    yaml: std::vec::IntoIter<Yaml>,
}

impl Iterator for YamlIter {
    type Item = Yaml;

    fn next(&mut self) -> Option<Yaml> {
        self.yaml.next()
    }
}
