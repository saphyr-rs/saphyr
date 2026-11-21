//! Deserializing YAML into Rust values.

mod node;
mod scalar;
mod tree;

use saphyr_parser::{Event, Parser, StrInput};
use serde::{
    Deserialize,
    de::{self, Visitor},
};

use self::{node::NodeDeserializer, tree::load_document};
use crate::{Error, ErrorKind, Options, Result};

/// Deserialize an instance of `T` from a YAML string.
///
/// The string must hold exactly one YAML document (see [`Deserializer`] for streams of documents),
/// or none, which is the same as a document with a `null`.
///
/// ```
/// #[derive(serde::Deserialize, PartialEq, Debug)]
/// struct Point {
///     x: i64,
///     y: i64,
/// }
///
/// let point: Point = saphyr_serde::from_str("x: 1\ny: -2").unwrap();
/// assert_eq!(point, Point { x: 1, y: -2 });
/// ```
///
/// Strings are borrowed from the input if they can be.
///
/// # Errors
/// Fails if the input is not valid YAML, if it does not fit `T`, or if it goes over the limits of
/// the default [`Options`].
pub fn from_str<'de, T: Deserialize<'de>>(input: &'de str) -> Result<T> {
    from_str_with_options(input, &Options::default())
}

/// Like [`from_str`], with [`Options`] to change the limits.
///
/// # Errors
/// See [`from_str`].
pub fn from_str_with_options<'de, T: Deserialize<'de>>(
    input: &'de str,
    options: &Options,
) -> Result<T> {
    let mut stream = Deserializer::from_str_with_options(input, options);
    let document = match stream.next() {
        Some(document) => document?,
        None => Document::empty(),
    };
    let value = T::deserialize(document)?;
    if stream.has_next()? {
        return Err(Error::new(
            ErrorKind::Data,
            "deserializing from YAML containing more than one document is not supported",
        ));
    }
    Ok(value)
}

/// Deserialize an instance of `T` from UTF-8 bytes of YAML.
///
/// # Errors
/// Fails if the input is not UTF-8, in addition to the errors of [`from_str`].
pub fn from_slice<'de, T: Deserialize<'de>>(input: &'de [u8]) -> Result<T> {
    from_slice_with_options(input, &Options::default())
}

/// Like [`from_slice`], with [`Options`] to change the limits.
///
/// # Errors
/// See [`from_slice`].
pub fn from_slice_with_options<'de, T: Deserialize<'de>>(
    input: &'de [u8],
    options: &Options,
) -> Result<T> {
    let input = core::str::from_utf8(input)
        .map_err(|e| Error::new(ErrorKind::Syntax, format!("input is not UTF-8: {e}")))?;
    from_str_with_options(input, options)
}

/// A stream of YAML documents.
///
/// This is an [`Iterator`] over the documents in the input. Each [`Document`] is a
/// [`serde::Deserializer`].
///
/// ```
/// use serde::Deserialize;
///
/// let input = "a: 1\n---\na: 2\n";
/// let mut values = Vec::new();
/// for document in saphyr_serde::Deserializer::from_str(input) {
///     #[derive(Deserialize)]
///     struct Doc {
///         a: i32,
///     }
///     values.push(Doc::deserialize(document.unwrap()).unwrap().a);
/// }
/// assert_eq!(values, [1, 2]);
/// ```
///
/// Once there is an error, the iterator ends.
pub struct Deserializer<'de> {
    parser: Parser<'de, StrInput<'de>>,
    options: Options,
    state: StreamState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StreamState {
    Fresh,
    Started,
    Done,
}

impl<'de> Deserializer<'de> {
    /// Create a stream of the documents in `input`, with the default [`Options`].
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'de str) -> Self {
        Self::from_str_with_options(input, &Options::default())
    }

    /// Create a stream of the documents in `input`.
    #[must_use]
    pub fn from_str_with_options(input: &'de str, options: &Options) -> Self {
        Self {
            parser: Parser::new_from_str(input),
            options: options.clone(),
            state: StreamState::Fresh,
        }
    }

    /// Read the beginning of the stream, if not read yet.
    fn start(&mut self) -> Result<()> {
        if self.state == StreamState::Fresh {
            self.state = StreamState::Done;
            match self.parser.next_event() {
                Some(Ok((Event::StreamStart, _))) => self.state = StreamState::Started,
                Some(Err(e)) => return Err(e.into()),
                _ => return Err(Error::new(ErrorKind::Syntax, "stream does not start")),
            }
        }
        Ok(())
    }

    /// Whether there is another document, without reading it.
    fn has_next(&mut self) -> Result<bool> {
        self.start()?;
        if self.state == StreamState::Done {
            return Ok(false);
        }
        match self.parser.peek() {
            Some(Ok((Event::DocumentStart(_), _))) => Ok(true),
            Some(Err(e)) => {
                self.state = StreamState::Done;
                Err(e.into())
            }
            _ => Ok(false),
        }
    }

    fn next_document(&mut self) -> Result<Option<Document<'de>>> {
        if !self.has_next()? {
            self.state = StreamState::Done;
            return Ok(None);
        }
        self.parser.next_event();
        match load_document(&mut self.parser, &self.options) {
            Ok(tree) => Ok(Some(Document { tree })),
            Err(e) => {
                self.state = StreamState::Done;
                Err(e)
            }
        }
    }
}

impl<'de> Iterator for Deserializer<'de> {
    type Item = Result<Document<'de>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_document() {
            Ok(document) => document.map(Ok),
            Err(e) => {
                self.state = StreamState::Done;
                Some(Err(e))
            }
        }
    }
}

/// One YAML document of a [`Deserializer`], ready to be deserialized.
pub struct Document<'de> {
    tree: tree::Document<'de>,
}

impl Document<'_> {
    fn empty() -> Self {
        Self {
            tree: tree::Document::empty(),
        }
    }
}

macro_rules! delegate {
    ($($method:ident)*) => {
        $(
            fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
                NodeDeserializer::new(&self.tree, self.tree.root())
                    .$method(visitor)
            }
        )*
    };
}

impl<'de> de::Deserializer<'de> for Document<'de> {
    type Error = Error;

    delegate! {
        deserialize_any deserialize_bool deserialize_i8 deserialize_i16 deserialize_i32
        deserialize_i64 deserialize_i128 deserialize_u8 deserialize_u16 deserialize_u32
        deserialize_u64 deserialize_u128 deserialize_f32 deserialize_f64 deserialize_char
        deserialize_str deserialize_string deserialize_bytes deserialize_byte_buf
        deserialize_option deserialize_unit deserialize_seq deserialize_map
        deserialize_identifier deserialize_ignored_any
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        NodeDeserializer::new(&self.tree, self.tree.root()).deserialize_unit_struct(name, visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        NodeDeserializer::new(&self.tree, self.tree.root())
            .deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        NodeDeserializer::new(&self.tree, self.tree.root()).deserialize_tuple(len, visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        NodeDeserializer::new(&self.tree, self.tree.root())
            .deserialize_tuple_struct(name, len, visitor)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        NodeDeserializer::new(&self.tree, self.tree.root())
            .deserialize_struct(name, fields, visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        NodeDeserializer::new(&self.tree, self.tree.root())
            .deserialize_enum(name, variants, visitor)
    }
}
