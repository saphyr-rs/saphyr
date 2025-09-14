//! A YAML node with position in the source document.
//!
//! This is set aside so as to not clutter `annotated.rs`.

use std::borrow::Cow;

use hashlink::LinkedHashMap;
use saphyr_parser::{Marker, ScalarStyle, Span, Tag};

use crate::{Accessor, LoadableYamlNode, SafelyIndex, Scalar, Yaml, YamlData};

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
    pub data: YamlData<'input, MarkedYaml<'input>>,
}

impl<'input> MarkedYaml<'input> {
    /// Convert a string to a scalar node.
    ///
    /// See [`YamlData::value_from_str`] for more details.
    ///
    /// The returned node is created with a default [`Span`].
    #[must_use]
    pub fn value_from_str(v: &'input str) -> Self {
        Self::value_from_cow(v.into())
    }

    /// Same as [`Self::value_from_str`] but uses a [`String`] instead.
    ///
    /// See [`YamlData::value_from_str`] for more details.
    ///
    /// The returned node is created with a default [`Span`].
    #[must_use]
    pub fn scalar_from_string(v: String) -> Self {
        Self::value_from_cow(v.into())
    }

    /// Same as [`Self::value_from_str`] but uses a [`Cow`] instead.
    ///
    /// See [`YamlData::value_from_str`] for more details.
    ///
    /// The returned node is created with a default [`Span`].
    #[must_use]
    pub fn value_from_cow(v: Cow<'input, str>) -> Self {
        Self {
            data: YamlData::Value(Scalar::parse_from_cow(v)),
            span: Span::default(),
        }
    }

    /// Convert a string to a  scalar node, abiding by the given metadata.
    ///
    /// The variant returned by this function will always be a [`YamlData::Value`], unless the tag
    /// forces a particular type and the representation cannot be parsed as this type, in which
    /// case it returns a [`YamlData::BadValue`].
    ///
    /// The returned node is created with a default [`Span`].
    #[must_use]
    pub fn value_from_cow_and_metadata(
        v: Cow<'input, str>,
        style: ScalarStyle,
        tag: Option<&Cow<'input, Tag>>,
    ) -> Self {
        Scalar::parse_from_cow_and_metadata(v, style, tag).map_or_else(
            || Self {
                data: YamlData::BadValue,
                span: Span::default(),
            },
            |v| Self {
                data: YamlData::Value(v),
                span: Span::default(),
            },
        )
    }
}

impl super::AnnotatedNode for MarkedYaml<'_> {
    type HashKey<'a> = MarkedYaml<'a>;

    fn parse_representation_recursive(&mut self) -> bool {
        self.data.parse_representation_recursive()
    }
}

impl<'a> From<YamlData<'a, MarkedYaml<'a>>> for MarkedYaml<'a> {
    fn from(value: YamlData<'a, MarkedYaml<'a>>) -> Self {
        Self {
            span: Span::default(),
            data: value,
        }
    }
}

impl<'b> PartialEq<MarkedYaml<'b>> for MarkedYaml<'_> {
    fn eq(&self, other: &MarkedYaml<'b>) -> bool {
        self.data.eq(&other.data)
    }
}

// I don't know if it's okay to implement that, but we need it for the hashmap.
impl Eq for MarkedYaml<'_> {}

impl std::hash::Hash for MarkedYaml<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl SafelyIndex for MarkedYaml<'_> {
    fn get(&self, key: impl Into<crate::Accessor>) -> Option<&Self> {
        match key.into() {
            Accessor::Field(f) => self.data.as_mapping_get(f.as_str()),
            Accessor::Index(i) => self.data.as_sequence_get(i),
        }
    }
}

impl<'input> LoadableYamlNode<'input> for MarkedYaml<'input> {
    type HashKey = MarkedYaml<'input>;

    fn from_bare_yaml(yaml: Yaml<'input>) -> Self {
        Self {
            span: Span::default(),
            data: match yaml {
                // Sequence and Mapping will always have their container empty.
                Yaml::Sequence(_) => YamlData::Sequence(vec![]),
                Yaml::Mapping(_) => YamlData::Mapping(LinkedHashMap::new()),
                Yaml::Alias(x) => YamlData::Alias(x),
                Yaml::BadValue => YamlData::BadValue,
                Yaml::Representation(v, style, tag) => YamlData::Representation(v, style, tag),
                Yaml::Tagged(tag, node) => {
                    YamlData::Tagged(tag, Box::new(Self::from_bare_yaml(*node)))
                }
                Yaml::Value(x) => YamlData::Value(x),
            },
        }
    }

    fn is_sequence(&self) -> bool {
        self.data.is_sequence()
    }

    fn is_mapping(&self) -> bool {
        self.data.is_mapping()
    }

    fn is_badvalue(&self) -> bool {
        self.data.is_badvalue()
    }

    fn into_tagged(self, tag: Cow<'input, Tag>) -> Self {
        Self {
            span: self.span,
            data: YamlData::Tagged(tag, Box::new(self)),
        }
    }

    fn sequence_mut(&mut self) -> &mut Vec<Self> {
        self.data
            .as_vec_mut()
            .expect("Called sequence_mut on a non-array")
    }

    fn mapping_mut(&mut self) -> &mut LinkedHashMap<Self::HashKey, Self> {
        self.data
            .as_mapping_mut()
            .expect("Called mapping_mut on a non-hash")
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

    fn with_start_marker(mut self, mark: Marker) -> Self {
        self.span.start = mark;
        self
    }

    fn with_end_marker(mut self, mark: Marker) -> Self {
        self.span.end = mark;
        self
    }
}
