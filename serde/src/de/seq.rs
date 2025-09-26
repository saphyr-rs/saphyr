use saphyr_parser::Event;
use serde::de::{DeserializeSeed, SeqAccess};

use crate::{de::Deserializer, error::DeserializeError};

pub struct YamlSequence<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> YamlSequence<'a, 'de> {
    pub(crate) fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de, 'a> SeqAccess<'de> for YamlSequence<'a, 'de> {
    type Error = DeserializeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.de.peek_event() {
            Some((Event::SequenceEnd, _span)) => Ok(None),
            _ => seed.deserialize(&mut *self.de).map(Some),
        }
    }
}
