//! Deserializing a node of a [`Document`].

use std::borrow::Cow;

use serde::{
    Deserialize,
    de::{
        self, DeserializeSeed, EnumAccess, Error as _, MapAccess, SeqAccess, Unexpected,
        VariantAccess, Visitor,
    },
};

use super::{
    scalar::{Resolved, negate, resolve},
    tree::{Document, Kind, Node, NodeId},
};
use crate::{Error, Result};

/// A [`de::Deserializer`] for one node of a document.
#[derive(Clone, Copy)]
pub(crate) struct NodeDeserializer<'a, 'de> {
    doc: &'a Document<'de>,
    id: NodeId,
    /// Ignore the tag of the node. Used when the tag has been consumed as the variant of an enum.
    strip_tag: bool,
}

impl<'a, 'de> NodeDeserializer<'a, 'de> {
    pub(crate) fn new(doc: &'a Document<'de>, id: NodeId) -> Self {
        Self {
            doc,
            id,
            strip_tag: false,
        }
    }

    fn with(self, id: NodeId) -> Self {
        Self::new(self.doc, id)
    }

    /// The node, once through aliases.
    fn node(&self) -> &'a Node<'de> {
        self.doc.node(self.doc.resolve(self.id))
    }

    fn tag(&self) -> Option<&'a saphyr_parser::Tag> {
        if self.strip_tag {
            None
        } else {
            self.node().tag.as_deref()
        }
    }

    fn resolved(&self) -> Result<Resolved> {
        match &self.node().kind {
            Kind::Scalar(text, style) => resolve(text, *style, self.tag()),
            _ => Err(Error::data("not a scalar")),
        }
    }

    fn unexpected(&self) -> Unexpected<'a> {
        match &self.node().kind {
            Kind::Sequence(_) => Unexpected::Seq,
            Kind::Mapping(_) => Unexpected::Map,
            Kind::Alias(_) => unreachable!("aliases are resolved"),
            Kind::Scalar(text, style) => match resolve(text, *style, self.tag()) {
                Ok(Resolved::Null) => Unexpected::Unit,
                Ok(Resolved::Bool(b)) => Unexpected::Bool(b),
                Ok(Resolved::Int {
                    negative: false,
                    magnitude,
                }) => match u64::try_from(magnitude) {
                    Ok(n) => Unexpected::Unsigned(n),
                    Err(_) => Unexpected::Other("integer"),
                },
                Ok(Resolved::Int {
                    negative: true,
                    magnitude,
                }) => match i64::try_from(negate(magnitude)) {
                    Ok(n) => Unexpected::Signed(n),
                    Err(_) => Unexpected::Other("integer"),
                },
                Ok(Resolved::Float(f)) => Unexpected::Float(f),
                Ok(Resolved::Str) | Err(_) => Unexpected::Str(text),
            },
        }
    }

    fn invalid_type<T>(&self, expected: &dyn de::Expected) -> Result<T> {
        Err(Error::invalid_type(self.unexpected(), expected))
    }

    fn visit_str<V: Visitor<'de>>(&self, text: &'a Cow<'de, str>, visitor: V) -> Result<V::Value> {
        match text {
            Cow::Borrowed(text) => visitor.visit_borrowed_str(text),
            Cow::Owned(text) => visitor.visit_str(text),
        }
    }

    /// Integers that do not fit in 64 bits are visited as 128 bits, which float visitors refuse.
    fn deserialize_float<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if let Ok(Resolved::Int {
            negative,
            magnitude,
        }) = self.resolved()
        {
            #[allow(clippy::cast_precision_loss)]
            let f = magnitude as f64;
            let marker = self.node().marker;
            return visitor
                .visit_f64::<Error>(if negative { -f } else { f })
                .map_err(|e| e.or_at(marker));
        }
        de::Deserializer::deserialize_any(self, visitor)
    }

    fn deserialize_any_inner<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let node = self.node();
        match &node.kind {
            Kind::Scalar(text, _) => match self.resolved()? {
                Resolved::Null => visitor.visit_unit(),
                Resolved::Bool(b) => visitor.visit_bool(b),
                Resolved::Int {
                    negative: false,
                    magnitude,
                } => match u64::try_from(magnitude) {
                    Ok(n) => visitor.visit_u64(n),
                    Err(_) => visitor.visit_u128(magnitude),
                },
                Resolved::Int {
                    negative: true,
                    magnitude,
                } => {
                    let n = negate(magnitude);
                    match i64::try_from(n) {
                        Ok(n) => visitor.visit_i64(n),
                        Err(_) => visitor.visit_i128(n),
                    }
                }
                Resolved::Float(f) => visitor.visit_f64(f),
                Resolved::Str => self.visit_str(text, visitor),
            },
            Kind::Sequence(items) => {
                let mut access = SeqDeserializer {
                    de: self,
                    items: items.iter(),
                };
                let value = visitor.visit_seq(&mut access)?;
                access.finish(items.len())?;
                Ok(value)
            }
            Kind::Mapping(entries) => {
                let len = entries.len();
                let mut access = MapDeserializer {
                    de: self,
                    entries,
                    position: 0,
                    value: None,
                };
                let value = visitor.visit_map(&mut access)?;
                access.finish(len)?;
                Ok(value)
            }
            Kind::Alias(_) => unreachable!("aliases are resolved"),
        }
    }
}

macro_rules! deserialize_via_any {
    ($($method:ident)*) => {
        $(
            fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
                self.deserialize_any(visitor)
            }
        )*
    };
}

impl<'de> de::Deserializer<'de> for NodeDeserializer<'_, 'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let marker = self.node().marker;
        self.deserialize_any_inner(visitor)
            .map_err(|e| e.or_at(marker))
    }

    deserialize_via_any! {
        deserialize_bool deserialize_i8 deserialize_i16 deserialize_i32 deserialize_i64
        deserialize_i128 deserialize_u8 deserialize_u16 deserialize_u32 deserialize_u64
        deserialize_u128 deserialize_unit deserialize_bytes deserialize_byte_buf
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_float(visitor)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_float(visitor)
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_any(visitor)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    /// Any scalar is a string when a string is expected: `123`, `true` and `null` give "123",
    /// "true" and "null". This is what avoids the "Norway problem" (`NO` being a boolean).
    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let node = self.node();
        match &node.kind {
            Kind::Scalar(text, style) => {
                // Still reject what we do not support, like `!!binary`, instead of giving
                // data a different meaning than the document says.
                resolve(text, *style, self.tag())
                    .and_then(|_| self.visit_str(text, visitor))
                    .map_err(|e| e.or_at(node.marker))
            }
            _ => self
                .invalid_type(&visitor)
                .map_err(|e| e.or_at(node.marker)),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let marker = self.node().marker;
        let is_null = matches!(self.node().kind, Kind::Scalar(..))
            && matches!(self.resolved(), Ok(Resolved::Null));
        if is_null {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
        .map_err(|e| e.or_at(marker))
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let node = self.node();
        if matches!(node.kind, Kind::Sequence(_)) {
            self.deserialize_any(visitor)
        } else {
            self.invalid_type(&visitor)
                .map_err(|e| e.or_at(node.marker))
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let node = self.node();
        if matches!(node.kind, Kind::Mapping(_)) {
            self.deserialize_any(visitor)
        } else {
            self.invalid_type(&visitor)
                .map_err(|e| e.or_at(node.marker))
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let node = self.node();
        let result = (|| {
            // `!Variant content`.
            if let Some(tag) = self.tag().filter(|tag| tag.handle == "!") {
                return visitor.visit_enum(EnumDeserializer {
                    variant: Variant::Tag(&tag.suffix),
                    content: Some(Self {
                        strip_tag: true,
                        ..self
                    }),
                });
            }
            match &node.kind {
                // `Variant`.
                Kind::Scalar(..) => visitor.visit_enum(EnumDeserializer {
                    variant: Variant::Node(self),
                    content: None,
                }),
                // `Variant: content`.
                Kind::Mapping(entries) => {
                    let [(key, value)] = entries[..] else {
                        return Err(Error::data(
                            "an enum must be a string or a mapping with a single key",
                        ));
                    };
                    visitor.visit_enum(EnumDeserializer {
                        variant: Variant::Node(self.with(key)),
                        content: Some(self.with(value)),
                    })
                }
                _ => self.invalid_type(&"an enum"),
            }
        })();
        result.map_err(|e| e.or_at(node.marker))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }
}

struct SeqDeserializer<'a, 'de> {
    de: NodeDeserializer<'a, 'de>,
    items: std::slice::Iter<'a, NodeId>,
}

impl SeqDeserializer<'_, '_> {
    /// Fail if the visitor did not consume the whole sequence.
    fn finish(&self, len: usize) -> Result<()> {
        if self.items.len() == 0 {
            Ok(())
        } else {
            Err(Error::invalid_length(len, &"fewer elements in sequence"))
        }
    }
}

impl<'de> SeqAccess<'de> for SeqDeserializer<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        match self.items.next() {
            Some(&id) => seed.deserialize(self.de.with(id)).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.items.len())
    }
}

struct MapDeserializer<'a, 'de> {
    de: NodeDeserializer<'a, 'de>,
    entries: &'a [(NodeId, NodeId)],
    position: usize,
    value: Option<NodeId>,
}

impl MapDeserializer<'_, '_> {
    /// Fail if the visitor did not consume the whole mapping.
    fn finish(&self, len: usize) -> Result<()> {
        if self.position == self.entries.len() {
            Ok(())
        } else {
            Err(Error::invalid_length(len, &"fewer elements in map"))
        }
    }
}

impl<'de> MapAccess<'de> for MapDeserializer<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        let Some(&(key, value)) = self.entries.get(self.position) else {
            return Ok(None);
        };
        self.position += 1;
        self.value = Some(value);
        seed.deserialize(self.de.with(key)).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let value = self
            .value
            .take()
            .expect("next_value_seed is called after next_key_seed");
        seed.deserialize(self.de.with(value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len() - self.position)
    }
}

enum Variant<'a, 'de> {
    /// The suffix of a `!Variant` tag.
    Tag(&'a str),
    /// A node holding the name of the variant.
    Node(NodeDeserializer<'a, 'de>),
}

struct EnumDeserializer<'a, 'de> {
    variant: Variant<'a, 'de>,
    content: Option<NodeDeserializer<'a, 'de>>,
}

impl<'a, 'de> EnumAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;
    type Variant = VariantDeserializer<'a, 'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let value = match self.variant {
            Variant::Tag(name) => {
                seed.deserialize(de::value::StrDeserializer::<Error>::new(name))?
            }
            Variant::Node(node) => seed.deserialize(node)?,
        };
        Ok((value, VariantDeserializer(self.content)))
    }
}

struct VariantDeserializer<'a, 'de>(Option<NodeDeserializer<'a, 'de>>);

impl<'de> VariantAccess<'de> for VariantDeserializer<'_, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.0 {
            None => Ok(()),
            Some(content) => <()>::deserialize(content),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        match self.0 {
            Some(content) => seed.deserialize(content),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"a newtype variant",
            )),
        }
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        match self.0 {
            Some(content) => de::Deserializer::deserialize_seq(content, visitor),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"a tuple variant",
            )),
        }
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        match self.0 {
            Some(content) => de::Deserializer::deserialize_map(content, visitor),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"a struct variant",
            )),
        }
    }
}
