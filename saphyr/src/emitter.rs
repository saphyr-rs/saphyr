//! YAML serialization helpers.

use crate::char_traits;
use crate::MarkedYaml;
use crate::MarkedYamlOwned;
use crate::{
    yaml::{Mapping, Yaml},
    Scalar,
};
use std::convert::From;
use std::error::Error;
use std::fmt::{self, Display};

/// An error when emitting YAML.
#[derive(Copy, Clone, Debug)]
pub enum EmitError {
    /// A formatting error.
    FmtError(fmt::Error),
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
        }
    }
}

impl From<fmt::Error> for EmitError {
    fn from(f: fmt::Error) -> Self {
        EmitError::FmtError(f)
    }
}

/// The YAML serializer.
///
/// ```
/// # use saphyr::{LoadableYamlNode, Yaml, YamlEmitter};
/// let input_string = "a: b\nc: d";
/// let yaml = Yaml::load_from_str(input_string).unwrap();
///
/// let mut output = String::new();
/// YamlEmitter::new(&mut output).dump(&yaml[0]).unwrap();
///
/// assert_eq!(output, r#"---
/// a: b
/// c: d
/// "#);
/// ```
#[allow(clippy::module_name_repetitions)]
pub struct YamlEmitter<'a> {
    writer: &'a mut dyn fmt::Write,
    best_indent: usize,
    compact: bool,
    level: isize,
    multiline_strings: bool,
}

/// A convenience alias for emitter functions that may fail without returning a value.
pub type EmitResult = Result<(), EmitError>;

// from serialize::json
fn escape_str(wr: &mut dyn fmt::Write, v: &str) -> Result<(), fmt::Error> {
    wr.write_str("\"")?;

    let mut start = 0;

    for (i, byte) in v.bytes().enumerate() {
        let escaped = match byte {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            b'\x00' => "\\u0000",
            b'\x01' => "\\u0001",
            b'\x02' => "\\u0002",
            b'\x03' => "\\u0003",
            b'\x04' => "\\u0004",
            b'\x05' => "\\u0005",
            b'\x06' => "\\u0006",
            b'\x07' => "\\u0007",
            b'\x08' => "\\b",
            b'\t' => "\\t",
            b'\n' => "\\n",
            b'\x0b' => "\\u000b",
            b'\x0c' => "\\f",
            b'\r' => "\\r",
            b'\x0e' => "\\u000e",
            b'\x0f' => "\\u000f",
            b'\x10' => "\\u0010",
            b'\x11' => "\\u0011",
            b'\x12' => "\\u0012",
            b'\x13' => "\\u0013",
            b'\x14' => "\\u0014",
            b'\x15' => "\\u0015",
            b'\x16' => "\\u0016",
            b'\x17' => "\\u0017",
            b'\x18' => "\\u0018",
            b'\x19' => "\\u0019",
            b'\x1a' => "\\u001a",
            b'\x1b' => "\\u001b",
            b'\x1c' => "\\u001c",
            b'\x1d' => "\\u001d",
            b'\x1e' => "\\u001e",
            b'\x1f' => "\\u001f",
            b'\x7f' => "\\u007f",
            _ => continue,
        };

        if start < i {
            wr.write_str(&v[start..i])?;
        }

        wr.write_str(escaped)?;

        start = i + 1;
    }

    if start != v.len() {
        wr.write_str(&v[start..])?;
    }

    wr.write_str("\"")?;
    Ok(())
}

pub trait Emittable {
    fn node(&self) -> Yaml<'_>;
}

impl<E> Emittable for &E
where
    E: ?Sized + Emittable,
{
    fn node(&self) -> Yaml<'_> {
        (**self).node()
    }
}

impl Emittable for Yaml<'_> {
    fn node(&self) -> Yaml<'_> {
        Yaml::clone(self)
    }
}

impl Emittable for MarkedYaml<'_> {
    fn node(&self) -> Yaml<'_> {
        from_marked_yaml(self)
    }
}

impl Emittable for MarkedYamlOwned {
    fn node(&self) -> Yaml<'_> {
        from_marked_yaml_owned(self)
    }
}

pub fn from_marked_yaml<'a>(input: &'a MarkedYaml) -> Yaml<'a> {
    match &input.data {
        crate::YamlData::Tagged(tag, node) => {
            Yaml::Tagged(tag.clone(), Box::new(from_marked_yaml(node)))
        }
        crate::YamlData::Representation(content, style, tag) => {
            Yaml::Representation(content.clone(), *style, tag.clone())
        }
        crate::YamlData::Value(v) => Yaml::Value(v.clone()),
        crate::YamlData::Sequence(vec) => {
            Yaml::Sequence(vec.iter().map(from_marked_yaml).collect())
        }
        crate::YamlData::Mapping(linked_hash_map) => Yaml::Mapping(
            linked_hash_map
                .iter()
                .map(|(k, v)| (from_marked_yaml(k), from_marked_yaml(v)))
                .collect(),
        ),
        crate::YamlData::Alias(a) => Yaml::Alias(*a),
        crate::YamlData::BadValue => Yaml::BadValue,
    }
}

pub fn from_marked_yaml_owned(input: &MarkedYamlOwned) -> Yaml<'_> {
    match &input.data {
        crate::YamlDataOwned::Tagged(tag, node) => Yaml::Tagged(
            std::borrow::Cow::Borrowed(tag),
            Box::new(from_marked_yaml_owned(node)),
        ),
        crate::YamlDataOwned::Representation(content, style, tag) => Yaml::Representation(
            std::borrow::Cow::Borrowed(content.as_str()),
            *style,
            tag.as_ref().map(std::borrow::Cow::Borrowed),
        ),
        crate::YamlDataOwned::Value(v) => Yaml::Value(v.as_scalar()),
        crate::YamlDataOwned::Sequence(vec) => {
            let items: Vec<Yaml<'_>> = vec.iter().map(from_marked_yaml_owned).collect();
            Yaml::Sequence(items)
        }
        crate::YamlDataOwned::Mapping(linked_hash_map) => Yaml::Mapping(
            linked_hash_map
                .iter()
                .map(|(k, v)| (from_marked_yaml_owned(k), from_marked_yaml_owned(v)))
                .collect(),
        ),
        crate::YamlDataOwned::Alias(a) => Yaml::Alias(*a),
        crate::YamlDataOwned::BadValue => Yaml::BadValue,
    }
}

impl<'a> YamlEmitter<'a> {
    /// Create a new emitter serializing into `writer`.
    pub fn new(writer: &'a mut dyn fmt::Write) -> Self {
        YamlEmitter {
            writer,
            best_indent: 2,
            compact: true,
            level: -1,
            multiline_strings: false,
        }
    }

    /// Set 'compact inline notation' on or off, as described for block
    /// [sequences](http://www.yaml.org/spec/1.2/spec.html#id2797382)
    /// and
    /// [mappings](http://www.yaml.org/spec/1.2/spec.html#id2798057).
    ///
    /// In this form, blocks cannot have any properties (such as anchors
    /// or tags), which should be OK, because this emitter doesn't
    /// (currently) emit those anyways.
    ///
    /// TODO(ethiraric, 2024/04/02): We can support those now.
    pub fn compact(&mut self, compact: bool) {
        self.compact = compact;
    }

    /// Determine if this emitter is using 'compact inline notation'.
    #[must_use]
    pub fn is_compact(&self) -> bool {
        self.compact
    }

    /// Render strings containing multiple lines in [literal style].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saphyr::{LoadableYamlNode, Yaml, YamlEmitter};
    ///
    /// let input = r#"{foo: "bar!\nbar!", baz: 42}"#;
    /// let parsed = Yaml::load_from_str(input).unwrap();
    /// eprintln!("{:?}", parsed);
    ///
    /// let mut output = String::new();
    /// let mut emitter = YamlEmitter::new(&mut output);
    /// emitter.multiline_strings(true);
    /// emitter.dump(&parsed[0]).unwrap();
    /// assert_eq!(output.as_str(), "\
    /// ---
    /// foo: |-
    ///   bar!
    ///   bar!
    /// baz: 42
    /// ");
    /// ```
    ///
    /// [literal style]: https://yaml.org/spec/1.2/spec.html#id2795688
    pub fn multiline_strings(&mut self, multiline_strings: bool) {
        self.multiline_strings = multiline_strings;
    }

    /// Determine if this emitter will emit multiline strings when appropriate.
    #[must_use]
    pub fn is_multiline_strings(&self) -> bool {
        self.multiline_strings
    }

    /// Dump Yaml to an output stream.
    /// # Errors
    /// Returns `EmitError` when an error occurs.
    pub fn dump<Y: Emittable>(&mut self, doc: &Y) -> EmitResult {
        // write DocumentStart
        writeln!(self.writer, "---")?;
        self.level = -1;
        let node = doc.node();
        self.emit_node(&node)?;
        writeln!(self.writer).map_err(EmitError::FmtError)
    }

    fn write_indent(&mut self) -> EmitResult {
        if self.level <= 0 {
            return Ok(());
        }
        for _ in 0..self.level {
            for _ in 0..self.best_indent {
                write!(self.writer, " ")?;
            }
        }
        Ok(())
    }

    fn emit_node(&mut self, node: &Yaml) -> EmitResult {
        match *node {
            Yaml::Sequence(ref v) => self.emit_sequence(v),
            Yaml::Mapping(ref h) => self.emit_mapping(h),
            Yaml::Value(Scalar::String(ref v)) => {
                if self.should_emit_string_as_block(v) {
                    self.emit_literal_block(v)?;
                } else if need_quotes(v) {
                    escape_str(self.writer, v)?;
                } else {
                    write!(self.writer, "{v}")?;
                }
                Ok(())
            }
            Yaml::Value(Scalar::Boolean(v)) => {
                if v {
                    self.writer.write_str("true")?;
                } else {
                    self.writer.write_str("false")?;
                }
                Ok(())
            }
            Yaml::Value(Scalar::Integer(v)) => Ok(write!(self.writer, "{v}")?),
            Yaml::Value(Scalar::FloatingPoint(ref v)) => Ok(write!(self.writer, "{v}")?),
            Yaml::Value(Scalar::Null) | Yaml::BadValue => Ok(write!(self.writer, "~")?),
            Yaml::Representation(ref v, style, ref tag) => {
                if let Some(tag) = tag {
                    write!(self.writer, "{} ", tag.as_ref())?;
                }
                match style {
                    saphyr_parser::ScalarStyle::Plain => write!(self.writer, "{v}")?,
                    saphyr_parser::ScalarStyle::SingleQuoted => write!(self.writer, "'{v}'")?,
                    saphyr_parser::ScalarStyle::DoubleQuoted => write!(self.writer, "\"{v}\"")?,
                    saphyr_parser::ScalarStyle::Literal => todo!(),
                    saphyr_parser::ScalarStyle::Folded => todo!(),
                }
                Ok(())
            }
            Yaml::Tagged(ref tag, ref node) => {
                write!(self.writer, "{} ", tag.as_ref())?;
                // We need to insert a newline after the tag in the following cases:
                //   - We have a non-empty sequence or mapping. `emit_sequence` and `emit_mapping`
                //     do not add that extra newline at the beginning.
                //       foo: !tag {} // OK
                //       ---
                //       foo: !tag [] // OK
                //       ---
                //       foo: !tag bar: baz // KO
                //       ---
                //       foo: !tag // OK
                //         bar: baz
                //       ---
                //       foo: !tag - a // OK
                //       ---
                //       foo: !tag - a // KO
                //         - b
                //       ---
                //       foo: !tag // OK
                //         - a
                //         - b
                if node.is_non_empty_collection() {
                    self.level += 1;
                    writeln!(self.writer)?;
                    self.write_indent()?;
                    self.level -= 1;
                }
                self.emit_node(node.as_ref())
            }
            // XXX(chenyh) Alias
            Yaml::Alias(_) => Ok(()),
        }
    }

    fn emit_literal_block(&mut self, v: &str) -> EmitResult {
        let ends_with_newline = v.ends_with('\n');
        if ends_with_newline {
            self.writer.write_str("|")?;
        } else {
            self.writer.write_str("|-")?;
        }

        self.level += 1;
        // lines() will omit the last line if it is empty.
        for line in v.lines() {
            writeln!(self.writer)?;
            self.write_indent()?;
            // It's literal text, so don't escape special chars.
            self.writer.write_str(line)?;
        }
        self.level -= 1;
        Ok(())
    }

    fn emit_sequence(&mut self, v: &[Yaml]) -> EmitResult {
        if v.is_empty() {
            write!(self.writer, "[]")?;
        } else {
            self.level += 1;
            for (cnt, x) in v.iter().enumerate() {
                if cnt > 0 {
                    writeln!(self.writer)?;
                    self.write_indent()?;
                }
                write!(self.writer, "-")?;
                self.emit_val(true, x)?;
            }
            self.level -= 1;
        }
        Ok(())
    }

    fn emit_mapping(&mut self, h: &Mapping) -> EmitResult {
        if h.is_empty() {
            self.writer.write_str("{}")?;
        } else {
            self.level += 1;
            for (cnt, (k, v)) in h.iter().enumerate() {
                let complex_key = matches!(k, Yaml::Mapping(_) | Yaml::Sequence(_));
                if cnt > 0 {
                    writeln!(self.writer)?;
                    self.write_indent()?;
                }
                if complex_key {
                    write!(self.writer, "?")?;
                    self.emit_val(true, k)?;
                    writeln!(self.writer)?;
                    self.write_indent()?;
                    write!(self.writer, ":")?;
                    self.emit_val(true, v)?;
                } else {
                    self.emit_node(k)?;
                    write!(self.writer, ":")?;
                    self.emit_val(false, v)?;
                }
            }
            self.level -= 1;
        }
        Ok(())
    }

    /// Emit a yaml as a hash or array value: i.e., which should appear
    /// following a ":" or "-", either after a space, or on a new line.
    /// If `inline` is true, then the preceding characters are distinct
    /// and short enough to respect the compact flag.
    fn emit_val(&mut self, inline: bool, val: &Yaml) -> EmitResult {
        match *val {
            Yaml::Sequence(ref v) => {
                if (inline && self.compact) || v.is_empty() {
                    write!(self.writer, " ")?;
                } else {
                    writeln!(self.writer)?;
                    self.level += 1;
                    self.write_indent()?;
                    self.level -= 1;
                }
                self.emit_sequence(v)
            }
            Yaml::Mapping(ref h) => {
                if (inline && self.compact) || h.is_empty() {
                    write!(self.writer, " ")?;
                } else {
                    writeln!(self.writer)?;
                    self.level += 1;
                    self.write_indent()?;
                    self.level -= 1;
                }
                self.emit_mapping(h)
            }
            _ => {
                write!(self.writer, " ")?;
                self.emit_node(val)
            }
        }
    }

    /// Check whether the string should be emitted as a literal block.
    ///
    /// This takes into account the [`multiline_strings`] option and whether the string contains
    /// newlines.
    ///
    /// [`multiline_strings`]: Self::multiline_strings.
    #[must_use]
    fn should_emit_string_as_block(&self, s: &str) -> bool {
        self.multiline_strings && s.contains('\n') && char_traits::is_valid_literal_block_scalar(s)
    }
}

/// Check if the string requires quoting.
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
fn need_quotes(string: &str) -> bool {
    fn need_quotes_spaces(string: &str) -> bool {
        string.starts_with(' ') || string.ends_with(' ')
    }

    string.is_empty()
        || need_quotes_spaces(string)
        || string.starts_with(|character: char| {
            matches!(
                character,
                '&' | '*' | '?' | '|' | '-' | '<' | '>' | '=' | '!' | '%' | '@'
            )
        })
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

#[cfg(test)]
mod test {
    use crate::{LoadableYamlNode, Yaml, YamlEmitter};

    #[test]
    fn test_multiline_string() {
        let input = r#"{foo: "bar!\nbar!", baz: 42}"#;
        let parsed = Yaml::load_from_str(input).unwrap();
        let mut output = String::new();
        let mut emitter = YamlEmitter::new(&mut output);
        emitter.multiline_strings(true);
        emitter.dump(&parsed[0]).unwrap();
    }
}
