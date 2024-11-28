//! YAML serialization helpers.

use std::{
    convert::From,
    error::Error,
    fmt::{self, Display},
};

use saphyr_parser::TScalarStyle;

use crate::{
    char_traits,
    emitter::event::{EmitterEvent, EventYamlEmitter},
    yaml::{Hash, Yaml},
};

pub(crate) mod event;

/// The YAML serializer.
///
/// ```
/// # use saphyr::{Yaml, YamlEmitter};
/// let input_string = "a: b\nc: d";
/// let yaml = Yaml::load_from_str(input_string).unwrap();
///
/// let mut output = String::new();
/// YamlEmitter::new(&mut output).dump(&yaml[0]).unwrap();
///
/// assert_eq!(output, r#"---
/// a: b
/// c: d"#);
/// ```
#[allow(clippy::module_name_repetitions)]
pub struct YamlEmitter<'a> {
    /// The inner emitter, using the lower-level event API.
    event_emitter: EventYamlEmitter<'a>,
}

impl<'a> YamlEmitter<'a> {
    /// Create a new emitter serializing into `writer`.
    pub fn new(writer: &'a mut dyn fmt::Write) -> Self {
        YamlEmitter {
            event_emitter: EventYamlEmitter::new(writer),
        }
        // While we could emit the `StreamStart` event, the `EventYamlEmitter` ignores it.
    }

    /// Set 'compact in-line notation' on or off, as described for block
    /// [sequences](http://www.yaml.org/spec/1.2/spec.html#id2797382)
    /// and
    /// [mappings](http://www.yaml.org/spec/1.2/spec.html#id2798057).
    ///
    /// See [`EventYamlEmitter::compact`].
    pub fn compact(&mut self, compact: bool) {
        self.event_emitter.compact(compact);
    }

    /// Determine if this emitter is using 'compact in-line notation'.
    ///
    /// See [`EventYamlEmitter::compact`].
    #[must_use]
    pub fn is_compact(&self) -> bool {
        self.event_emitter.is_compact()
    }

    /// Render strings containing multiple lines in [literal style].
    ///
    /// See [`EventYamlEmitter::multiline_strings`].
    pub fn multiline_strings(&mut self, multiline_strings: bool) {
        self.event_emitter.multiline_strings(multiline_strings);
    }

    /// Determine if this emitter will emit multiline strings when appropriate.
    ///
    /// See [`EventYamlEmitter::multiline_strings`].
    #[must_use]
    pub fn is_multiline_strings(&self) -> bool {
        self.event_emitter.is_multiline_strings()
    }

    /// Dump the given YAML node as a single document to the inner output stream.
    ///
    /// # Errors
    /// Returns [`EmitError`] when an error occurs.
    pub fn dump(&mut self, doc: &Yaml) -> EmitResult {
        self.event_emitter.on_document_start(true)?;
        self.emit_node(doc)?;
        self.event_emitter.on_document_end(false)
    }

    /// Emit a YAML node.
    fn emit_node(&mut self, node: &Yaml) -> EmitResult {
        match *node {
            Yaml::Array(ref v) => self.emit_array(v),
            Yaml::Hash(ref h) => self.emit_hash(h),
            Yaml::String(ref v) => {
                let style = if self.event_emitter.is_multiline_strings()
                    && v.contains('\n')
                    && char_traits::is_valid_literal_block_scalar(v)
                {
                    TScalarStyle::Literal
                } else if needs_quotes(v) {
                    TScalarStyle::DoubleQuoted
                } else {
                    TScalarStyle::Plain
                };
                self.event_emitter.on_scalar(v, style)
            }
            Yaml::Boolean(v) => {
                let repr = if v { "true" } else { "false" };
                self.event_emitter.on_scalar(repr, TScalarStyle::Plain)
            }
            Yaml::Integer(v) => {
                let repr = v.to_string();
                self.event_emitter.on_scalar(&repr, TScalarStyle::Plain)
            }
            Yaml::Real(ref v) => {
                let repr = v.to_string();
                self.event_emitter.on_scalar(&repr, TScalarStyle::Plain)
            }
            Yaml::Null | Yaml::BadValue => self.event_emitter.on_scalar("~", TScalarStyle::Plain),
            // XXX(chenyh) Alias
            Yaml::Alias(_) => Ok(()),
        }
    }

    /// Emit a YAML sequence.
    fn emit_array(&mut self, sequence: &[Yaml]) -> EmitResult {
        self.event_emitter
            .on_event(EmitterEvent::SequenceStart(None))?;
        for node in sequence {
            self.emit_node(node)?;
        }
        self.event_emitter.on_event(EmitterEvent::SequenceEnd)?;
        Ok(())
    }

    /// Emit a YAML mapping.
    fn emit_hash(&mut self, mapping: &Hash) -> EmitResult {
        self.event_emitter
            .on_event(EmitterEvent::MappingStart(None))?;
        for (key, value) in mapping {
            self.emit_node(key)?;
            self.emit_node(value)?;
        }
        self.event_emitter.on_event(EmitterEvent::MappingEnd)?;
        Ok(())
    }
}

/// A convenience alias for emitter functions that may fail without returning a value.
pub type EmitResult = Result<(), EmitError>;

/// An error when emitting YAML.
#[derive(Copy, Clone, Debug)]
pub enum EmitError {
    /// A formatting error.
    FmtError(fmt::Error),
    /// An error in the sequence of event the emitter received.
    EventError(&'static str),
}

impl Error for EmitError {
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl Display for EmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EmitError::FmtError(ref err) => Display::fmt(err, formatter),
            EmitError::EventError(msg) => Display::fmt(msg, formatter),
        }
    }
}

impl From<fmt::Error> for EmitError {
    fn from(f: fmt::Error) -> Self {
        EmitError::FmtError(f)
    }
}

/// Check if the string requires quoting.
///
/// Strings starting with any of the following characters must be quoted.
/// :, &, *, ?, |, -, <, >, =, !, %, @
/// Strings containing any of the following characters must be quoted.
/// {, }, \[, t \], ,, #, `
///
/// If the string contains any of the following control characters, it must be escaped with double quotes:
/// \0, \x01, \x02, \x03, \x04, \x05, \x06, \a, \b, \t, \n, \v, \f, \r, \x0e, \x0f, \x10, \x11, \x12, \x13, \x14, \x15, \x16, \x17, \x18, \x19, \x1a, \e, \x1c, \x1d, \x1e, \x1f, \N, \_, \L, \P
///
/// Finally, there are other cases when the strings must be quoted, no matter if you're using single or double quotes:
/// * When the string is true or false (otherwise, it would be treated as a boolean value);
/// * When the string is null or ~ (otherwise, it would be considered as a null value);
/// * When the string looks like a number, such as integers (e.g. 2, 14, etc.), floats (e.g. 2.6, 14.9) and exponential numbers (e.g. 12e7, etc.) (otherwise, it would be treated as a numeric value);
/// * When the string looks like a date (e.g. 2014-12-31) (otherwise it would be automatically converted into a Unix timestamp).
#[allow(clippy::doc_markdown)]
fn needs_quotes(string: &str) -> bool {
    string.is_empty()
        || string.starts_with(|character: char| {
            matches!(
                character,
                ' ' | '&' | '*' | '?' | '|' | '-' | '<' | '>' | '=' | '!' | '%' | '@'
            )
        })
        || string.ends_with(' ') // `starts_with(' ')`tested above
        || string.contains(|character: char| {
            matches!(character, ':'
            | '{'
            | '}'
            | '['
            | ']'
            | ','
            | '#'
            | '`'
            | '\"'
            | '\''
            | '\\'
            | '\0'..='\x06'
            | '\t'
            | '\n'
            | '\r'
            | '\x0e'..='\x1a'
            | '\x1c'..='\x1f')
        })
        || [
            // http://yaml.org/type/bool.html
            // Note: 'y', 'Y', 'n', 'N', is not quoted deliberately, as in libyaml. PyYAML also parse
            // them as string, not booleans, although it is violating the YAML 1.1 specification.
            // See https://github.com/dtolnay/serde-yaml/pull/83#discussion_r152628088.
            "yes", "Yes", "YES", "no", "No", "NO", "True", "TRUE", "true", "False", "FALSE",
            "false", "on", "On", "ON", "off", "Off", "OFF",
            // http://yaml.org/type/null.html
            "null", "Null", "NULL", "~",
        ]
        .contains(&string)
        || string.starts_with('.')
        || string.starts_with("0x")
        || string.parse::<i64>().is_ok()
        || string.parse::<f64>().is_ok()
}
