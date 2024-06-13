//! A YAML node with position in the source document.
//!
//! This is set aside so as to not clutter `annotated.rs`.

use hashlink::LinkedHashMap;
use saphyr_parser::{Marker, Parser, ScanError};

use crate::{LoadableYamlNode, Yaml, YamlData, YamlLoader};

/// A YAML node with [`Marker`]s pointing to the start of the node.
///
/// This structure does not implement functions to operate on the YAML object. To access those,
/// refer to the [`Self::data`] field.
#[derive(Clone, Debug)]
pub struct MarkedYaml {
    /// The marker pointing to the start of the node.
    ///
    /// The marker is relative to the start of the input stream that was given to the parser, not
    /// to the start of the document within the input stream.
    pub marker: Marker,
    /// The YAML contents of the node.
    pub data: YamlData<MarkedYaml>,
}

impl MarkedYaml {
    /// Load the given string as an array of YAML documents.
    ///
    /// See the function [`load_from_str`] for more details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    ///
    /// [`load_from_str`]: `crate::load_from_str`
    pub fn load_from_str(source: &str) -> Result<Vec<Self>, ScanError> {
        Self::load_from_iter(source.chars())
    }

    /// Load the contents of the given iterator as an array of YAML documents.
    ///
    /// See the function [`load_from_iter`] for more details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    ///
    /// [`load_from_iter`]: `crate::load_from_iter`
    pub fn load_from_iter<I: Iterator<Item = char>>(source: I) -> Result<Vec<Self>, ScanError> {
        let mut parser = Parser::new(source);
        Self::load_from_parser(&mut parser)
    }

    /// Load the contents from the specified [`Parser`] as an array of YAML documents.
    ///
    /// See the function [`load_from_parser`] for more details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    ///
    /// [`load_from_parser`]: `crate::load_from_parser`
    pub fn load_from_parser<I: Iterator<Item = char>>(
        parser: &mut Parser<I>,
    ) -> Result<Vec<Self>, ScanError> {
        let mut loader = YamlLoader::<Self>::default();
        parser.load(&mut loader, true)?;
        Ok(loader.into_documents())
    }
}

impl PartialEq for MarkedYaml {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data)
    }
}

// I don't know if it's okay to implement that, but we need it for the hashmap.
impl Eq for MarkedYaml {}

impl std::hash::Hash for MarkedYaml {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl From<YamlData<MarkedYaml>> for MarkedYaml {
    fn from(value: YamlData<MarkedYaml>) -> Self {
        Self {
            marker: Marker::default(),
            data: value,
        }
    }
}

impl LoadableYamlNode for MarkedYaml {
    fn from_bare_yaml(yaml: Yaml) -> Self {
        Self {
            marker: Marker::default(),
            data: match yaml {
                Yaml::Real(x) => YamlData::Real(x),
                Yaml::Integer(x) => YamlData::Integer(x),
                Yaml::String(x) => YamlData::String(x),
                Yaml::Boolean(x) => YamlData::Boolean(x),
                // Array and Hash will always have their container empty.
                Yaml::Array(_) => YamlData::Array(vec![]),
                Yaml::Hash(_) => YamlData::Hash(LinkedHashMap::new()),
                Yaml::Alias(x) => YamlData::Alias(x),
                Yaml::Null => YamlData::Null,
                Yaml::BadValue => YamlData::BadValue,
            },
        }
    }

    fn is_array(&self) -> bool {
        self.data.is_array()
    }

    fn is_hash(&self) -> bool {
        self.data.is_hash()
    }

    fn is_badvalue(&self) -> bool {
        self.data.is_badvalue()
    }

    fn array_mut(&mut self) -> &mut Vec<Self> {
        if let YamlData::Array(x) = &mut self.data {
            x
        } else {
            panic!("Called array_mut on a non-array");
        }
    }

    fn hash_mut(&mut self) -> &mut LinkedHashMap<Self, Self> {
        if let YamlData::Hash(x) = &mut self.data {
            x
        } else {
            panic!("Called array_mut on a non-array");
        }
    }

    fn take(&mut self) -> Self {
        let mut taken_out = MarkedYaml {
            marker: Marker::default(),
            data: YamlData::BadValue,
        };
        std::mem::swap(&mut taken_out, self);
        taken_out
    }

    fn with_marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }
}
