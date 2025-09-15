use saphyr_parser::Event;
use serde::de::{DeserializeSeed, MapAccess};

use crate::{de::Deserializer, error::DeserializeError};

pub struct YamlMapping<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    empty: bool,
}
impl<'a, 'de> YamlMapping<'a, 'de> {
    pub(crate) fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de, empty: false }
    }

    pub(crate) fn empty(de: &'a mut Deserializer<'de>) -> Self {
        Self { de, empty: true }
    }
}

impl<'de, 'a> MapAccess<'de> for YamlMapping<'a, 'de> {
    type Error = DeserializeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.empty {
            Ok(None)
        } else {
            match self.de.peek_event() {
                Some((Event::DocumentEnd, _span)) => Ok(None),
                Some((Event::StreamEnd, _span)) => Ok(None),
                Some((Event::MappingEnd, _span)) => Ok(None),
                _ => seed.deserialize(&mut *self.de).map(Some),
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}
