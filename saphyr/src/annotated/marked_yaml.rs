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
///
/// # Warning
/// In order to allow indexing by content in mappings, equality comparisons for this structure
/// **ignore** the [`Span`].
#[derive(Clone, Debug)]
pub struct MarkedYaml<'input> {
    /// The span indicating where in the input stream the object is.
    ///
    /// The markers are relative to the start of the input stream that was given to the parser, not
    /// to the start of the document within the input stream.
    pub span: Span,
    /// The YAML contents of the node.
    pub data: YamlData<'input, MarkedYaml<'input>, MarkedYamlCapsule<'input>>,
}

/// A capsule for [`MarkedYaml`].
///
/// See [`LoadableYamlNode::HashKey`] for details.
#[allow(clippy::module_name_repetitions)]
#[derive(Hash, PartialEq, Eq, Debug, Clone)]
#[repr(transparent)]
pub struct MarkedYamlCapsule<'lt> {
    inner: MarkedYaml<'lt>,
}

impl<'input> super::AnnotatedNode for MarkedYaml<'input> {
    type HashKey<'a> = MarkedYamlCapsule<'a>;

    type HashValue<'b> = MarkedYaml<'b>;
}

impl<'input> super::AnnotatedNode for MarkedYamlCapsule<'input> {
    type HashKey<'a> = MarkedYamlCapsule<'a>;

    type HashValue<'b> = MarkedYaml<'b>;
}

impl<'a, 'b> core::borrow::Borrow<MarkedYaml<'a>> for MarkedYamlCapsule<'b>
where
    'b: 'a,
{
    fn borrow(&self) -> &MarkedYaml<'a> {
        &self.inner
    }
}

impl<'a> core::borrow::Borrow<YamlData<'a, MarkedYaml<'a>, MarkedYamlCapsule<'a>>>
    for MarkedYamlCapsule<'a>
{
    fn borrow(&self) -> &YamlData<'a, MarkedYaml<'a>, MarkedYamlCapsule<'a>> {
        &self.inner.data
    }
}

impl<'a> From<MarkedYaml<'a>> for MarkedYamlCapsule<'a> {
    fn from(value: MarkedYaml<'a>) -> Self {
        Self { inner: value }
    }
}

impl<'a> From<YamlData<'a, MarkedYaml<'a>, MarkedYamlCapsule<'a>>> for MarkedYamlCapsule<'a> {
    fn from(value: YamlData<'a, MarkedYaml<'a>, MarkedYamlCapsule<'a>>) -> Self {
        Self {
            inner: MarkedYaml {
                span: Span::default(),
                data: value,
            },
        }
    }
}

impl<'a, 'b> std::cmp::PartialEq<MarkedYaml<'a>> for MarkedYamlCapsule<'b> {
    fn eq(&self, other: &MarkedYaml<'a>) -> bool {
        self.inner.eq(other)
    }
}

impl<'a> From<MarkedYamlCapsule<'a>> for MarkedYaml<'a> {
    fn from(value: MarkedYamlCapsule<'a>) -> Self {
        value.inner
    }
}

impl<'input> MarkedYaml<'input> {
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
    pub fn load_from_parser<I: Input>(
        parser: &mut Parser<'input, I>,
    ) -> Result<Vec<Self>, ScanError> {
        let mut loader = YamlLoader::<Self>::default();
        parser.load(&mut loader, true)?;
        Ok(loader.into_documents())
    }
}

impl<'input> PartialEq for MarkedYaml<'input> {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data)
    }
}

// I don't know if it's okay to implement that, but we need it for the hashmap.
impl<'input> Eq for MarkedYaml<'input> {}

impl<'input> std::hash::Hash for MarkedYaml<'input> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl<'input> From<YamlData<'input, MarkedYaml<'input>, MarkedYamlCapsule<'input>>>
    for MarkedYaml<'input>
{
    fn from(value: YamlData<'input, MarkedYaml<'input>, MarkedYamlCapsule<'input>>) -> Self {
        Self {
            span: Span::default(),
            data: value,
        }
    }
}

impl<'input> LoadableYamlNode<'input> for MarkedYaml<'input> {
    type HashKey = MarkedYamlCapsule<'input>;

    fn from_bare_yaml(yaml: Yaml<'input>) -> Self {
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

    fn hash_mut(&mut self) -> &mut LinkedHashMap<Self::HashKey, Self> {
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
