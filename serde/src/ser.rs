//! Serializing Rust values into YAML.
//!
//! The values are written as they are serialized, with a [`YamlEventEmitter`]: nothing but the
//! state of the containers that are open (and the keys of the mappings, see [`Options`]) is kept
//! in memory.

use std::{
    collections::BTreeSet,
    fmt, io, mem,
    string::{String, ToString},
};

use saphyr::YamlEventEmitter;
use serde::{Serialize, ser};

use crate::{Error, ErrorKind, Options, Result};

/// Serialize `value` into a YAML string.
///
/// The output is a single document, ending with a newline.
///
/// ```
/// #[derive(serde::Serialize)]
/// struct Point {
///     x: i64,
///     y: i64,
/// }
///
/// let yaml = saphyr_serde::to_string(&Point { x: 1, y: -2 }).unwrap();
/// assert_eq!(yaml, "x: 1\ny: -2\n");
/// ```
///
/// Strings that would be read as something else if they were not quoted (`"true"`, `"1"`, `"~"`,
/// `".inf"`, ...) are quoted, so that the output deserializes to the same value.
///
/// Enums are externally tagged: a unit variant is a string, and the other variants are a mapping
/// of the name of the variant to its content.
///
/// # Errors
/// Fails if the value cannot be serialized, if it is nested more deeply than the
/// [`max_depth`](Options::max_depth) of the default [`Options`], or if it has a mapping with the
/// same key twice (for instance, with `#[serde(flatten)]`).
pub fn to_string<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    to_string_with_options(value, &Options::default())
}

/// Like [`to_string`], with [`Options`]. [`Options::max_depth`] and
/// [`Options::allow_duplicate_keys`] apply.
///
/// # Errors
/// See [`to_string`].
pub fn to_string_with_options<T: Serialize + ?Sized>(
    value: &T,
    options: &Options,
) -> Result<String> {
    let mut output = String::new();
    serialize(&mut output, value, options)?;
    Ok(output)
}

/// Serialize `value` as YAML into an [`io::Write`].
///
/// The YAML is written in many small pieces: wrap a file or a socket in a
/// [`BufWriter`](io::BufWriter).
///
/// The YAML is written as the value is serialized, and an error can come at any point: the writer
/// may be left with a part of a document. Use [`to_string`] if that is a problem.
///
/// # Errors
/// See [`to_string`]. Also fails if writing fails.
pub fn to_writer<W: io::Write, T: Serialize + ?Sized>(writer: W, value: &T) -> Result<()> {
    to_writer_with_options(writer, value, &Options::default())
}

/// Like [`to_writer`], with [`Options`]. [`Options::max_depth`] and
/// [`Options::allow_duplicate_keys`] apply.
///
/// # Errors
/// See [`to_writer`].
pub fn to_writer_with_options<W: io::Write, T: Serialize + ?Sized>(
    writer: W,
    value: &T,
    options: &Options,
) -> Result<()> {
    /// Gives an `io::Write` to what expects a `fmt::Write`, and keeps the error.
    struct Adapter<W> {
        writer: W,
        error: Option<io::Error>,
    }

    impl<W: io::Write> fmt::Write for Adapter<W> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.writer.write_all(s.as_bytes()).map_err(|e| {
                self.error = Some(e);
                fmt::Error
            })
        }
    }

    let mut adapter = Adapter {
        writer,
        error: None,
    };
    serialize(&mut adapter, value, options).map_err(|e| match adapter.error.take() {
        Some(io_error) => Error::new(ErrorKind::Serialize, io_error.to_string()),
        None => e,
    })
}

fn serialize<T: Serialize + ?Sized>(
    writer: &mut dyn fmt::Write,
    value: &T,
    options: &Options,
) -> Result<()> {
    let mut emitter = YamlEventEmitter::new(writer);
    emitter.multiline(true);
    // The emitter does not let the value nest more deeply. It stops the serialization of
    // recursive values, which would overflow the stack otherwise.
    emitter.max_depth(options.max_depth);
    let mut serializer = Serializer {
        emitter,
        check_duplicate_keys: !options.allow_duplicate_keys,
        capture: None,
    };
    value.serialize(&mut serializer)?;
    serializer.emitter.finish()?;
    Ok(())
}

struct Serializer<'w> {
    emitter: YamlEventEmitter<'w>,
    check_duplicate_keys: bool,
    /// The key being serialized, if we want to know it.
    capture: Option<Capture>,
}

/// What we know about a key of a mapping, to catch the keys that are there twice. As when
/// deserializing, keys are compared by their text, whatever their type.
#[derive(Default)]
struct Capture {
    /// The text of the key, if it is a scalar.
    scalar: Option<String>,
    /// The key is not a scalar. We do not compare these.
    complex: bool,
}

impl Serializer<'_> {
    /// Remember the scalar that is being serialized if it is (part of) a key.
    fn note_scalar(&mut self, identify: impl FnOnce() -> String) {
        if let Some(capture) = &mut self.capture {
            if capture.scalar.is_none() && !capture.complex {
                capture.scalar = Some(identify());
            } else {
                capture.complex = true;
            }
        }
    }

    /// Remember that a container is being serialized, which cannot be a scalar key.
    fn note_container(&mut self) {
        if let Some(capture) = &mut self.capture {
            capture.complex = true;
        }
    }

    fn string(&mut self, v: &str) -> Result<()> {
        self.note_scalar(|| v.into());
        Ok(self.emitter.scalar_str(v)?)
    }
}

impl<'a, 'w> ser::Serializer for &'a mut Serializer<'w> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Compound<'a, 'w>;
    type SerializeTuple = Compound<'a, 'w>;
    type SerializeTupleStruct = Compound<'a, 'w>;
    type SerializeTupleVariant = Compound<'a, 'w>;
    type SerializeMap = Compound<'a, 'w>;
    type SerializeStruct = Compound<'a, 'w>;
    type SerializeStructVariant = Compound<'a, 'w>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.note_scalar(|| v.to_string());
        Ok(self.emitter.scalar_bool(v)?)
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(v.into())
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(v.into())
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(v.into())
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.note_scalar(|| v.to_string());
        Ok(self.emitter.scalar_i64(v)?)
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        self.note_scalar(|| v.to_string());
        Ok(self.emitter.scalar_i128(v)?)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(v.into())
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(v.into())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(v.into())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.note_scalar(|| v.to_string());
        Ok(self.emitter.scalar_u64(v)?)
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        self.note_scalar(|| v.to_string());
        Ok(self.emitter.scalar_u128(v)?)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.note_scalar(|| f32_text(v));
        Ok(self.emitter.scalar_f32(v)?)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.note_scalar(|| f64_text(v));
        Ok(self.emitter.scalar_f64(v)?)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.string(v.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.string(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.note_container();
        self.emitter.sequence_start()?;
        for &byte in v {
            self.emitter.scalar_u64(byte.into())?;
        }
        Ok(self.emitter.sequence_end()?)
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.note_scalar(|| "~".into());
        Ok(self.emitter.scalar_null()?)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.string(variant)
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()> {
        self.note_container();
        self.emitter.mapping_start()?;
        self.emitter.scalar_str(variant)?;
        value.serialize(&mut *self)?;
        Ok(self.emitter.mapping_end()?)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Compound<'a, 'w>> {
        self.note_container();
        self.emitter.sequence_start()?;
        Ok(Compound::new(self, false))
    }

    fn serialize_tuple(self, len: usize) -> Result<Compound<'a, 'w>> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Compound<'a, 'w>> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Compound<'a, 'w>> {
        self.note_container();
        self.emitter.mapping_start()?;
        self.emitter.scalar_str(variant)?;
        self.emitter.sequence_start()?;
        Ok(Compound::new(self, true))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Compound<'a, 'w>> {
        self.note_container();
        self.emitter.mapping_start()?;
        Ok(Compound::new(self, false))
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Compound<'a, 'w>> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Compound<'a, 'w>> {
        self.note_container();
        self.emitter.mapping_start()?;
        self.emitter.scalar_str(variant)?;
        self.emitter.mapping_start()?;
        Ok(Compound::new(self, true))
    }
}

macro_rules! float_text {
    ($name:ident, $type:ty) => {
        /// The text of a float, as the emitter writes it.
        fn $name(v: $type) -> String {
            if v.is_nan() {
                ".nan".into()
            } else if v.is_infinite() {
                if v < 0.0 { "-.inf" } else { ".inf" }.into()
            } else if v.fract() == 0.0 {
                format!("{v}.0")
            } else {
                v.to_string()
            }
        }
    };
}
float_text!(f32_text, f32);
float_text!(f64_text, f64);

/// A sequence or a mapping being serialized.
struct Compound<'a, 'w> {
    serializer: &'a mut Serializer<'w>,
    /// The variant of an enum has a mapping of its own around the container.
    is_variant: bool,
    /// The keys of the mapping that are scalars, to catch duplicates. Only mappings with
    /// [`serialize_key`](ser::SerializeMap::serialize_key) have any: the fields of a struct
    /// cannot be the same.
    keys: BTreeSet<String>,
}

impl<'a, 'w> Compound<'a, 'w> {
    fn new(serializer: &'a mut Serializer<'w>, is_variant: bool) -> Self {
        Self {
            serializer,
            is_variant,
            keys: BTreeSet::new(),
        }
    }

    fn element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut *self.serializer)
    }

    fn field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> {
        self.serializer.emitter.scalar_str(key)?;
        value.serialize(&mut *self.serializer)
    }

    fn end_sequence(self) -> Result<()> {
        self.serializer.emitter.sequence_end()?;
        self.end_variant()
    }

    fn end_mapping(self) -> Result<()> {
        self.serializer.emitter.mapping_end()?;
        self.end_variant()
    }

    fn end_variant(self) -> Result<()> {
        if self.is_variant {
            self.serializer.emitter.mapping_end()?;
        }
        Ok(())
    }
}

impl ser::SerializeSeq for Compound<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.element(value)
    }

    fn end(self) -> Result<()> {
        self.end_sequence()
    }
}

impl ser::SerializeTuple for Compound<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.element(value)
    }

    fn end(self) -> Result<()> {
        self.end_sequence()
    }
}

impl ser::SerializeTupleStruct for Compound<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.element(value)
    }

    fn end(self) -> Result<()> {
        self.end_sequence()
    }
}

impl ser::SerializeTupleVariant for Compound<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.element(value)
    }

    fn end(self) -> Result<()> {
        self.end_sequence()
    }
}

impl ser::SerializeMap for Compound<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<()> {
        if !self.serializer.check_duplicate_keys {
            return key.serialize(&mut *self.serializer);
        }
        // The key can hold mappings of its own that capture keys: put ours back afterwards.
        let outer = self.serializer.capture.replace(Capture::default());
        let result = key.serialize(&mut *self.serializer);
        let capture = mem::replace(&mut self.serializer.capture, outer);
        result?;
        if let Some(Capture {
            scalar: Some(scalar),
            complex: false,
        }) = capture
        {
            // Losing a value silently would be data corruption, and the YAML would not be valid.
            if !self.keys.insert(scalar) {
                return Err(Error::new(
                    ErrorKind::Serialize,
                    "mapping would have the same key twice",
                ));
            }
        }
        Ok(())
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.element(value)
    }

    fn end(self) -> Result<()> {
        self.end_mapping()
    }
}

impl ser::SerializeStruct for Compound<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.field(key, value)
    }

    fn end(self) -> Result<()> {
        self.end_mapping()
    }
}

impl ser::SerializeStructVariant for Compound<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.field(key, value)
    }

    fn end(self) -> Result<()> {
        self.end_mapping()
    }
}
