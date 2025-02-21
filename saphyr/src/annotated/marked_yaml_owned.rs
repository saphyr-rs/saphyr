//! A YAML node with position in the source document.
//!
//! This is set aside so as to not clutter `annotated.rs`.

use std::borrow::Cow;

use hashlink::LinkedHashMap;
use saphyr_parser::{ScalarStyle, Span, Tag};

use crate::{Accessor, LoadableYamlNode, SafelyIndex, ScalarOwned, Yaml, YamlDataOwned};

/// A YAML node with [`Span`]s pointing to the start of the node.
///
/// This structure does not implement functions to operate on the YAML object. To access those,
/// refer to the [`Self::data`] field.
///
/// # Warning
/// In order to allow indexing by content in mappings, equality comparisons for this structure
/// **ignore** the [`Span`].
#[derive(Clone, Debug)]
pub struct MarkedYamlOwned {
    /// The span indicating where in the input stream the object is.
    ///
    /// The markers are relative to the start of the input stream that was given to the parser, not
    /// to the start of the document within the input stream.
    pub span: Span,
    /// The YAML contents of the node.
    pub data: YamlDataOwned<MarkedYamlOwned>,
}

impl MarkedYamlOwned {
    /// Convert a string to a scalar node.
    ///
    /// See [`YamlData::value_from_str`] for more details.
    ///
    /// The returned node is created with a default [`Span`].
    ///
    /// [`YamlData::value_from_str`]: crate::YamlData::value_from_str
    #[must_use]
    pub fn value_from_str(v: &str) -> Self {
        Self::value_from_cow(v.into())
    }

    /// Same as [`Self::value_from_str`] but uses a [`String`] instead.
    ///
    /// See [`YamlData::value_from_str`] for more details.
    ///
    /// The returned node is created with a default [`Span`].
    ///
    /// [`YamlData::value_from_str`]: crate::YamlData::value_from_str
    #[must_use]
    pub fn scalar_from_string(v: String) -> Self {
        Self::value_from_cow(v.into())
    }

    /// Same as [`Self::value_from_str`] but uses a [`Cow`] instead.
    ///
    /// See [`YamlData::value_from_str`] for more details.
    ///
    /// The returned node is created with a default [`Span`].
    ///
    /// [`YamlData::value_from_str`]: crate::YamlData::value_from_str
    #[must_use]
    pub fn value_from_cow(v: Cow<'_, str>) -> Self {
        Self {
            data: YamlDataOwned::Value(ScalarOwned::parse_from_cow(v)),
            span: Span::default(),
        }
    }

    /// Convert a string to a  scalar node, abiding by the given metadata.
    ///
    /// The variant returned by this function will always be a [`YamlDataOwned::Value`], unless the
    /// tag forces a particular type and the representation cannot be parsed as this type, in which
    /// case it returns a [`YamlDataOwned::BadValue`].
    ///
    /// The returned node is created with a default [`Span`].
    #[must_use]
    pub fn value_from_cow_and_metadata(
        v: Cow<'_, str>,
        style: ScalarStyle,
        tag: Option<&Cow<'_, Tag>>,
    ) -> Self {
        ScalarOwned::parse_from_cow_and_metadata(v, style, tag).map_or_else(
            || Self {
                data: YamlDataOwned::BadValue,
                span: Span::default(),
            },
            |v| Self {
                data: YamlDataOwned::Value(v),
                span: Span::default(),
            },
        )
    }
}

impl super::AnnotatedNodeOwned for MarkedYamlOwned {
    type HashKey = MarkedYamlOwned;

    fn parse_representation_recursive(&mut self) -> bool {
        self.data.parse_representation_recursive()
    }
}

impl From<YamlDataOwned<MarkedYamlOwned>> for MarkedYamlOwned {
    fn from(value: YamlDataOwned<MarkedYamlOwned>) -> Self {
        Self {
            span: Span::default(),
            data: value,
        }
    }
}

impl PartialEq<MarkedYamlOwned> for MarkedYamlOwned {
    fn eq(&self, other: &MarkedYamlOwned) -> bool {
        self.data.eq(&other.data)
    }
}

// I don't know if it's okay to implement that, but we need it for the hashmap.
impl Eq for MarkedYamlOwned {}

impl std::hash::Hash for MarkedYamlOwned {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl SafelyIndex for MarkedYamlOwned {
    fn get(&self, key: impl Into<crate::Accessor>) -> Option<&Self> {
        match key.into() {
            Accessor::Field(f) => self.data.as_mapping_get(f.as_str()),
            Accessor::Index(i) => self.data.as_sequence_get(i),
        }
    }
}

impl LoadableYamlNode<'_> for MarkedYamlOwned {
    type HashKey = MarkedYamlOwned;

    fn from_bare_yaml(yaml: Yaml) -> Self {
        Self {
            span: Span::default(),
            data: match yaml {
                // Sequence and Mapping will always have their container empty.
                Yaml::Sequence(_) => YamlDataOwned::Sequence(vec![]),
                Yaml::Mapping(_) => YamlDataOwned::Mapping(LinkedHashMap::new()),
                Yaml::Alias(x) => YamlDataOwned::Alias(x),
                Yaml::BadValue => YamlDataOwned::BadValue,
                Yaml::Representation(v, style, tag) => {
                    YamlDataOwned::Representation(v.to_string(), style, tag.map(Cow::into_owned))
                }
                Yaml::Tagged(tag, node) => {
                    YamlDataOwned::Tagged(tag.into_owned(), Box::new(Self::from_bare_yaml(*node)))
                }
                Yaml::Value(x) => YamlDataOwned::Value(x.into_owned()),
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

    fn into_tagged(self, tag: Cow<'_, Tag>) -> Self {
        Self {
            span: self.span,
            data: YamlDataOwned::Tagged(tag.into_owned(), Box::new(self)),
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
        let mut taken_out = MarkedYamlOwned {
            span: Span::default(),
            data: YamlDataOwned::BadValue,
        };
        std::mem::swap(&mut taken_out, self);
        taken_out
    }

    fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    fn with_start_marker(mut self, start: saphyr_parser::Marker) -> Self {
        self.span.start = start;
        self
    }

    fn with_end_marker(mut self, end: saphyr_parser::Marker) -> Self {
        self.span.end = end;
        self
    }
}
