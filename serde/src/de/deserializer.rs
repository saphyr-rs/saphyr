use std::str::FromStr;

use regex::{Regex, RegexSet};
use saphyr_parser::Event;
use serde::{
    Deserialize,
    de::{IntoDeserializer, Visitor},
};

use super::mapping::YamlMapping;
use super::seq::YamlSequence;
use super::variant::Enum;

use crate::error::{DeserializeError, Result};

pub struct Deserializer<'de> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    yaml: saphyr_parser::Parser<'de, saphyr_parser::StrInput<'de>>,
    boolean_re: RegexSet,
    null_re: Regex,
}

impl<'de> Deserializer<'de> {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'de str) -> Self {
        let yaml = saphyr_parser::Parser::new_from_str(input);
        let boolean_re = RegexSet::new([
            r"^(y|Y|yes|Yes|YES|true|True|TRUE|on|On|ON|)$",
            r"^(n|N|no|No|NO|false|False|FALSE|off|Off|OFF)$",
        ])
        .unwrap();
        let null_re = Regex::new(r"^(null|Null|NULL|~)$").unwrap();
        Deserializer {
            yaml,
            boolean_re,
            null_re,
        }
    }

    pub fn read_boolean(&mut self) -> Result<bool> {
        let regex_set = self.boolean_re.clone();
        let (s, span) = self.read_scalar_string()?;
        let matches = regex_set.matches(&s.clone());
        if matches.matched(0) {
            Ok(true)
        } else if matches.matched(1) {
            Ok(false)
        } else {
            Err(DeserializeError::not_a_bool(&s, span))
        }
    }

    pub fn next_event(&mut self) -> Result<(Event<'de>, saphyr_parser::Span)> {
        let next = self.yaml.next_event();
        Ok(next.ok_or(DeserializeError::EarlyTermination)??)
    }

    pub fn peek_event(&mut self) -> Option<&(Event<'_>, saphyr_parser::Span)> {
        let peek = self.yaml.peek();
        peek.and_then(|r| r.ok())
    }

    pub fn start_stream(&mut self) -> Result<()> {
        let (next_event, span) = self.next_event()?;
        if !matches!(next_event, saphyr_parser::Event::StreamStart) {
            Err(DeserializeError::unexpected(
                &next_event,
                span,
                "start_stream",
            ))
        } else {
            Ok(())
        }
    }

    pub fn end_stream(&mut self) -> Result<()> {
        let (next_event, span) = self.next_event()?;
        if !matches!(next_event, saphyr_parser::Event::StreamEnd) {
            Err(DeserializeError::unexpected(
                &next_event,
                span,
                "end_stream",
            ))
        } else {
            Ok(())
        }
    }

    pub fn start_document(&mut self) -> Result<bool> {
        let peek = self.peek_event();
        if matches!(peek, Some((saphyr_parser::Event::DocumentStart(_), _))) {
            self.next_event()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn end_document(&mut self) -> Result<()> {
        let (next_event, span) = self.next_event()?;
        if !matches!(next_event, saphyr_parser::Event::DocumentEnd) {
            Err(DeserializeError::unexpected(
                &next_event,
                span,
                "end_document",
            ))
        } else {
            Ok(())
        }
    }

    pub fn start_sequence(&mut self) -> Result<()> {
        let (next_event, span) = self.next_event()?;
        if !matches!(next_event, saphyr_parser::Event::SequenceStart(_, _)) {
            Err(DeserializeError::unexpected(
                &next_event,
                span,
                "start_sequence",
            ))
        } else {
            Ok(())
        }
    }

    pub fn end_sequence(&mut self) -> Result<()> {
        let (next_event, span) = self.next_event()?;
        if next_event != saphyr_parser::Event::SequenceEnd {
            Err(DeserializeError::unexpected(
                &next_event,
                span,
                "end_sequence",
            ))
        } else {
            Ok(())
        }
    }

    pub fn start_map(&mut self) -> Result<bool> {
        let peek = self.peek_event();
        if matches!(
            peek,
            Some((
                saphyr_parser::Event::MappingStart(_size, _option_tag),
                _span
            ))
        ) {
            self.next_event()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn end_map(&mut self) -> Result<()> {
        let (next_event, span) = self.next_event()?;
        if !matches!(next_event, saphyr_parser::Event::MappingEnd,) {
            Err(DeserializeError::unexpected(&next_event, span, "end_map"))
        } else {
            Ok(())
        }
    }

    pub fn consume_map(&mut self) -> Result<()> {
        loop {
            let (next_event, _span) = self.next_event()?;
            if matches!(next_event, saphyr_parser::Event::MappingEnd) {
                break;
            }
        }
        Ok(())
    }

    pub fn parse_scalar<T>(&mut self, type_string: &str) -> Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: std::fmt::Display,
    {
        let (s, span) = self.read_scalar_string()?;
        let parse_result = s.parse::<T>();
        parse_result.map_err(|_e| {
            DeserializeError::number_parse_failure(&s, span, type_string, &format!("{}", _e))
        })
    }

    pub fn read_scalar_string(
        &mut self,
    ) -> Result<(std::borrow::Cow<'_, str>, saphyr_parser::Span)> {
        match self.next_event()? {
            (saphyr_parser::Event::Scalar(s, _, _, _), span) => Ok((s, span)),
            (event, span) => Err(DeserializeError::unexpected(
                &event,
                span,
                "deserialize_str",
            )),
        }
    }

    pub fn peek_scalar_string(
        &mut self,
    ) -> Option<(std::borrow::Cow<'_, str>, saphyr_parser::Span)> {
        match self.peek_event()? {
            (saphyr_parser::Event::Scalar(s, _, _, _), span) => Some((s.clone(), span.to_owned())),
            _ => None,
        }
    }
}

impl<'de> serde::de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = crate::error::DeserializeError;

    fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.next_event()? {
            (saphyr_parser::Event::Scalar(value, _, _, _), _span) => {
                // TODO: have to detect and parse the string as a particular type
                // 'n' => self.deserialize_unit(visitor),
                // 't' | 'f' => self.deserialize_bool(visitor),
                // '"' => self.deserialize_str(visitor),
                // '0'..='9' => self.deserialize_u64(visitor),
                // '-' => self.deserialize_i64(visitor),
                visitor.visit_str(&value)
            }
            (saphyr_parser::Event::MappingStart(_map, _), _span) => {
                let result = visitor.visit_map(YamlMapping::new(self));
                self.consume_map()?; // sometimes serde doesn't read the whole map?
                result
            }
            (saphyr_parser::Event::SequenceStart(_, _), _span) => {
                let result = visitor.visit_seq(YamlSequence::new(self));
                self.end_sequence()?;
                result
            }
            (event, span) => Err(DeserializeError::unexpected(
                &event,
                span,
                "deserialize_any",
            )),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.read_boolean()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_scalar("i8")?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_scalar("i16")?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_scalar("i32")?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_scalar("i64")?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_scalar("u8")?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_scalar("u16")?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_scalar("u32")?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_scalar("u64")?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.parse_scalar("f32")?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.parse_scalar("f64")?)
    }

    fn deserialize_char<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let (s, _span) = self.read_scalar_string()?;
        visitor.visit_str(&s)
    }

    fn deserialize_string<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let (s, _span) = self.read_scalar_string()?;
        visitor.visit_str(&s)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize an decode a base64 string")
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!("deserialize an decode a base64 string")
    }

    fn deserialize_option<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let null_regex = self.null_re.clone();
        match self
            .peek_scalar_string()
            .map(|(s, _span)| null_regex.is_match(&s))
        {
            Some(true) => {
                self.next_event()?;
                visitor.visit_none()
            }
            _ => visitor.visit_some(self), // Some(false) => visitor.visit_some(self),
                                           // None => {
                                           //     // self.next_event()?;
                                           //     visitor.visit_none()
                                           // }
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let null_regex = self.null_re.clone();
        match self
            .peek_scalar_string()
            .map(|(s, _span)| null_regex.is_match(&s))
        {
            Some(true) => {
                self.next_event()?;
                visitor.visit_unit()
            }
            _ => Err(DeserializeError::TypeError),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.start_sequence()?;
        let value = visitor.visit_seq(YamlSequence::new(self))?;
        self.end_sequence()?;
        Ok(value)
    }

    fn deserialize_tuple<V>(
        self,
        _len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.start_map()? {
            let value = visitor.visit_map(YamlMapping::new(self))?;
            self.end_map()?;
            Ok(value)
        } else {
            visitor.visit_map(YamlMapping::empty(self))
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.next_event()? {
            (saphyr_parser::Event::Scalar(key, _, _, _), _span) => {
                let s = key.to_string();
                visitor.visit_enum(s.into_deserializer())
            }
            (saphyr_parser::Event::MappingStart(_, _), _span) => {
                let value = visitor.visit_enum(Enum::new(self))?;
                self.end_map()?;
                Ok(value)
            }

            (event, span) => Err(DeserializeError::unexpected(
                &event,
                span,
                "deserialize_enum",
            )),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let (s, _span) = self.read_scalar_string()?;
        visitor.visit_str(&s)
    }
}

#[allow(dead_code)]
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    deserializer.start_stream()?;
    let has_document = deserializer.start_document()?;
    let t = T::deserialize(&mut deserializer)?;
    if has_document {
        deserializer.end_document()?;
    }
    deserializer.end_stream()?;
    Ok(t)
}
