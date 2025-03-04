//! A YAML node with position in the source document.
//!
//! This is set aside so as to not clutter `annotated.rs`.

use hashlink::LinkedHashMap;
use saphyr_parser::{BufferedInput, Input, Parser, ScanError, Span};

use crate::{LoadableYamlNode, Yaml, YamlData, YamlLoader};

/// A YAML node with [`Span`]s pointing to the start of the node.
///
/// This structure does not implement functions to operate on the YAML object. To access those,
/// refer to the [`Self::data`] field.
#[derive(Clone, Debug)]
pub struct MarkedYaml {
    /// The span indicating where in the input stream the object is.
    ///
    /// The markers are relative to the start of the input stream that was given to the parser, not
    /// to the start of the document within the input stream.
    pub span: Span,
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
    /// [`load_from_str`]: `Yaml::load_from_str`
    pub fn load_from_str(source: &str) -> Result<Vec<Self>, ScanError> {
        Self::load_from_iter(source.chars())
    }

    /// Load the contents of the given iterator as an array of YAML documents.
    ///
    /// See the function [`load_from_str`] for more details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    ///
    /// [`load_from_str`]: `Yaml::load_from_str`
    pub fn load_from_iter<I: Iterator<Item = char>>(source: I) -> Result<Vec<Self>, ScanError> {
        let mut parser = Parser::new(BufferedInput::new(source));
        Self::load_from_parser(&mut parser)
    }

    /// Load the contents from the specified [`Parser`] as an array of YAML documents.
    ///
    /// See the function [`load_from_str`] for more details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    ///
    /// [`load_from_str`]: `Yaml::load_from_str`
    pub fn load_from_parser<I: Input>(parser: &mut Parser<I>) -> Result<Vec<Self>, ScanError> {
        let mut loader = YamlLoader::<Self>::default();
        parser.load(&mut loader, true)?;
        Ok(loader.into_documents())
    }

    /// Index into a YAML sequence or map.
    /// A string index can be used to access a value in a map, and a usize index can be used to access an element of an sequence.
    ///
    /// Original implementation is from `serde_yaml` [get](https://docs.rs/serde_yaml/latest/serde_yaml/value/enum.Value.html#method.get)
    pub fn get<I: Index>(&self, index: I) -> Option<&Self> {
        index.index_into(self)
    }
}

pub trait Index {
    fn index_into<'v>(&self, v: &'v MarkedYaml) -> Option<&'v MarkedYaml>;
}

impl Index for usize {
    fn index_into<'v>(&self, v: &'v MarkedYaml) -> Option<&'v MarkedYaml> {
        v.data.as_vec().and_then(|elements| elements.get(*self))
    }
}

impl Index for str {
    fn index_into<'v>(&self, v: &'v MarkedYaml) -> Option<&'v MarkedYaml> {
        v.get(self.to_string())
    }
}

impl Index for MarkedYaml {
    fn index_into<'v>(&self, v: &'v MarkedYaml) -> Option<&'v MarkedYaml> {
        match &v.data {
            YamlData::Array(vec) => {
                if let Some(num) = self.data.as_i64() {
                    vec.get(num as usize)
                } else {
                    None
                }
            }
            YamlData::Hash(nodes) => nodes.get(self),
            _ => None,
        }
    }
}

impl Index for YamlData<MarkedYaml> {
    fn index_into<'v>(&self, v: &'v MarkedYaml) -> Option<&'v MarkedYaml> {
        match &v.data {
            YamlData::Array(vec) => {
                if let Some(num) = self.as_i64() {
                    vec.get(num as usize)
                } else {
                    None
                }
            }
            YamlData::Hash(nodes) => {
                let this = MarkedYaml {
                    span: Span::default(),
                    data: self.clone(),
                };
                nodes.get(&this)
            }
            _ => None,
        }
    }
}

impl Index for String {
    fn index_into<'v>(&self, v: &'v MarkedYaml) -> Option<&'v MarkedYaml> {
        let key = MarkedYaml::from_bare_yaml(Yaml::String(self.clone()));
        v.data.as_hash().and_then(|elements| elements.get(&key))
    }
}

impl<I> Index for &I
where
    I: ?Sized + Index,
{
    fn index_into<'v>(&self, v: &'v MarkedYaml) -> Option<&'v MarkedYaml> {
        (**self).index_into(v)
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
            span: Span::default(),
            data: value,
        }
    }
}

impl LoadableYamlNode for MarkedYaml {
    fn from_bare_yaml(yaml: Yaml) -> Self {
        Self {
            span: Span::default(),
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
            span: Span::default(),
            data: YamlData::BadValue,
        };
        std::mem::swap(&mut taken_out, self);
        taken_out
    }

    fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }
}
