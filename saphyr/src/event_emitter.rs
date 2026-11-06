//! A YAML emitter driven by events, which writes the output as it receives them.

use alloc::vec::Vec;
use core::fmt;

use thiserror::Error;

use crate::emitter::{
    Chomping, can_be_literal_block, escape_str, need_quotes, write_f32, write_f64, write_indent,
    write_literal_block,
};

/// An error when emitting YAML from events.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum EventEmitError {
    /// A formatting error.
    #[error("{0}")]
    FmtError(#[from] fmt::Error),
    /// The event cannot follow the events before it, for instance a [`MappingEnd`] when a
    /// [`SequenceStart`] is the last container left open.
    ///
    /// [`MappingEnd`]: EmitEvent::MappingEnd
    /// [`SequenceStart`]: EmitEvent::SequenceStart
    #[error("invalid event: {0}")]
    InvalidEvent(&'static str),
    /// The event would nest containers more deeply than the maximum depth of the emitter.
    ///
    /// See [`YamlEventEmitter::max_depth`].
    #[error("nesting exceeds the maximum depth of {0}")]
    TooDeep(usize),
}

/// A convenience alias for the functions of [`YamlEventEmitter`] that may fail without returning a
/// value.
pub type EventEmitResult = Result<(), EventEmitError>;

/// An event to feed to the [`YamlEventEmitter`].
///
/// To each of these events corresponds a method of the emitter, which can be called directly to
/// spare a `match`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum EmitEvent<'a> {
    /// The start of a sequence.
    SequenceStart,
    /// The end of a sequence.
    SequenceEnd,
    /// The start of a mapping. Its keys and values alternate, starting with a key.
    MappingStart,
    /// The end of a mapping.
    MappingEnd,
    /// A `null` value.
    Null,
    /// A boolean.
    Bool(bool),
    /// An integer.
    I64(i64),
    /// An integer.
    U64(u64),
    /// An integer.
    I128(i128),
    /// An integer.
    U128(u128),
    /// A floating-point number.
    F32(f32),
    /// A floating-point number.
    F64(f64),
    /// A string. It is quoted if it would be read as something else than a string.
    String(&'a str),
}

/// A YAML serializer that expects events rather than a tree of nodes, and writes each of them as
/// soon as it gets it.
///
/// Where [`YamlEmitter`] dumps a [`Yaml`] that has to be in memory, this keeps only the state of
/// the containers that are open. Its memory use depends on the depth of the document, not on its
/// size. It is the back-end of the `Serializer` of `saphyr-serde`.
///
/// The events make a stream of nodes: a scalar, or a container (see [`SequenceStart`] and
/// [`MappingStart`]) with the events of its content up to the matching end. Each node at the root
/// is a document of its own. The first one is written as is, and the next ones are written after
/// a `---` line. The events of a mapping alternate between keys and values. A key can be any
/// node, but is written on one line if it is a scalar, and with `?` otherwise.
///
/// The output is the same as the one of [`YamlEmitter`] (with no `---` line for the first
/// document), and always ends with a line break.
///
/// The emitter checks the order of events. Events that are not valid (an end that closes nothing,
/// a key with no value...) and containers nested too deeply are errors, and never panic or write
/// invalid YAML. After an error, the output so far is incomplete, and the emitter should not be
/// used again.
///
/// ```
/// use saphyr::{EmitEvent, YamlEventEmitter};
///
/// let mut output = String::new();
/// let mut emitter = YamlEventEmitter::new(&mut output);
/// for event in [
///     EmitEvent::MappingStart,
///     EmitEvent::String("name"),
///     EmitEvent::String("saphyr"),
///     EmitEvent::String("versions"),
///     EmitEvent::SequenceStart,
///     EmitEvent::I64(1),
///     EmitEvent::String("2"),
///     EmitEvent::SequenceEnd,
///     EmitEvent::MappingEnd,
/// ] {
///     emitter.feed_event(&event).unwrap();
/// }
/// emitter.finish().unwrap();
/// assert_eq!(output, "name: saphyr\nversions:\n  - 1\n  - \"2\"\n");
/// ```
///
/// [`YamlEmitter`]: crate::YamlEmitter
/// [`Yaml`]: crate::Yaml
/// [`SequenceStart`]: EmitEvent::SequenceStart
/// [`MappingStart`]: EmitEvent::MappingStart
pub struct YamlEventEmitter<'w> {
    writer: &'w mut dyn fmt::Write,
    compact: bool,
    multiline: bool,
    max_depth: usize,
    /// The containers that are open, innermost last.
    stack: Vec<Frame>,
    /// The number of documents that are complete.
    documents: usize,
    /// Set when a literal block has just written the line break after its last line, which the
    /// next node must not write again.
    final_line_break: Option<Chomping>,
}

/// The number of spaces by which each level is indented.
const INDENT: usize = 2;

/// The default of [`YamlEventEmitter::max_depth`].
const DEFAULT_MAX_DEPTH: usize = 1024;

/// An open container.
#[derive(Clone, Copy)]
struct Frame {
    is_mapping: bool,
    /// How the container is introduced.
    lead: Lead,
    /// Whether it has any content yet. Until then, we cannot know how to write the container
    /// (`[]` or a block).
    nonempty: bool,
    /// What the next event must be.
    expect: Expect,
}

/// How a container is introduced.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lead {
    /// It is the root.
    Root,
    /// It follows a `-`, a `?`, or the `:` of a complex key. It can start on the same line.
    Inline,
    /// It is the value of a key written on the same line (`key:`). It starts on the next line.
    Block,
}

/// What a container expects next.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// The item of a sequence.
    Item,
    /// The key of a mapping.
    Key,
    /// The value of a mapping. The key was written as a node that needs `?` and `:` if it is
    /// `complex`.
    Value { complex: bool },
}

/// Where a node is written.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// A document.
    Root,
    /// A key of a mapping that is written as `key:`.
    Key,
    /// After a `-`, a `?` or a `:`.
    Inline,
    /// After a `key:`.
    Value,
}

/// A scalar to write.
enum Scalar<'a> {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    I128(i128),
    U128(u128),
    F32(f32),
    F64(f64),
    Str(&'a str),
}

impl<'w> YamlEventEmitter<'w> {
    /// Create a new emitter writing into `writer`.
    pub fn new(writer: &'w mut dyn fmt::Write) -> Self {
        YamlEventEmitter {
            writer,
            compact: true,
            multiline: false,
            max_depth: DEFAULT_MAX_DEPTH,
            stack: Vec::new(),
            documents: 0,
            final_line_break: None,
        }
    }

    /// Set 'compact inline notation' on or off, as described for block
    /// [sequences](http://www.yaml.org/spec/1.2/spec.html#id2797382)
    /// and
    /// [mappings](http://www.yaml.org/spec/1.2/spec.html#id2798057).
    ///
    /// ```yaml
    /// - foo
    /// -
    ///   - Not compact
    /// - - Compact
    /// ```
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
    /// [literal style]: https://yaml.org/spec/1.2/spec.html#id2795688
    pub fn multiline(&mut self, multiline: bool) {
        self.multiline = multiline;
    }

    /// Determine if this emitter will emit multiline strings when appropriate.
    #[must_use]
    pub fn is_multiline(&self) -> bool {
        self.multiline
    }

    /// Set how deep containers can be nested (1024 by default).
    ///
    /// A container that would be nested more deeply is an [`EventEmitError::TooDeep`] error. The
    /// emitter itself does not recurse, so this is not about the stack: it bounds the memory the
    /// emitter uses, and the width of the indentation it writes. The code that feeds the
    /// emitter from a recursive structure can rely on it to stop recursing.
    pub fn max_depth(&mut self, max_depth: usize) {
        self.max_depth = max_depth;
    }

    /// The maximum depth at which containers can be nested. See [`Self::max_depth`].
    #[must_use]
    pub fn get_max_depth(&self) -> usize {
        self.max_depth
    }

    /// Feed an event to the emitter.
    ///
    /// # Errors
    /// Returns an error if the event is not valid at this point, or if writing to the output
    /// fails.
    pub fn feed_event(&mut self, event: &EmitEvent<'_>) -> EventEmitResult {
        match *event {
            EmitEvent::SequenceStart => self.sequence_start(),
            EmitEvent::SequenceEnd => self.sequence_end(),
            EmitEvent::MappingStart => self.mapping_start(),
            EmitEvent::MappingEnd => self.mapping_end(),
            EmitEvent::Null => self.scalar_null(),
            EmitEvent::Bool(v) => self.scalar_bool(v),
            EmitEvent::I64(v) => self.scalar_i64(v),
            EmitEvent::U64(v) => self.scalar_u64(v),
            EmitEvent::I128(v) => self.scalar_i128(v),
            EmitEvent::U128(v) => self.scalar_u128(v),
            EmitEvent::F32(v) => self.scalar_f32(v),
            EmitEvent::F64(v) => self.scalar_f64(v),
            EmitEvent::String(v) => self.scalar_str(v),
        }
    }

    /// Check that the document is complete.
    ///
    /// The output of the documents that are complete is already written, so calling this is not
    /// needed to get it. It is a way to catch containers that were left open.
    ///
    /// # Errors
    /// Returns an error if a container is still open.
    pub fn finish(&mut self) -> EventEmitResult {
        if self.stack.is_empty() {
            Ok(())
        } else {
            Err(EventEmitError::InvalidEvent("a container is not closed"))
        }
    }

    /// Mark the start of a sequence.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn sequence_start(&mut self) -> EventEmitResult {
        self.container_start(false)
    }

    /// Mark the end of a sequence.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn sequence_end(&mut self) -> EventEmitResult {
        self.container_end(false)
    }

    /// Mark the start of a mapping.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn mapping_start(&mut self) -> EventEmitResult {
        self.container_start(true)
    }

    /// Mark the end of a mapping.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn mapping_end(&mut self) -> EventEmitResult {
        self.container_end(true)
    }

    /// Emit a `null` value.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn scalar_null(&mut self) -> EventEmitResult {
        self.scalar(&Scalar::Null)
    }

    /// Emit a boolean.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn scalar_bool(&mut self, v: bool) -> EventEmitResult {
        self.scalar(&Scalar::Bool(v))
    }

    /// Emit an integer.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn scalar_i64(&mut self, v: i64) -> EventEmitResult {
        self.scalar(&Scalar::I64(v))
    }

    /// Emit an integer.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn scalar_u64(&mut self, v: u64) -> EventEmitResult {
        self.scalar(&Scalar::U64(v))
    }

    /// Emit an integer.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn scalar_i128(&mut self, v: i128) -> EventEmitResult {
        self.scalar(&Scalar::I128(v))
    }

    /// Emit an integer.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn scalar_u128(&mut self, v: u128) -> EventEmitResult {
        self.scalar(&Scalar::U128(v))
    }

    /// Emit a floating-point number.
    ///
    /// It is written so that it is read back as a float, and as the same one: `1.0` and not `1`,
    /// and `.nan`, `.inf` and `-.inf` for the special values.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn scalar_f32(&mut self, v: f32) -> EventEmitResult {
        self.scalar(&Scalar::F32(v))
    }

    /// Emit a floating-point number. See [`Self::scalar_f32`].
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn scalar_f64(&mut self, v: f64) -> EventEmitResult {
        self.scalar(&Scalar::F64(v))
    }

    /// Emit a string.
    ///
    /// It is quoted, with escape sequences if needed, when it would not be read back as the same
    /// string otherwise (`"true"`, `"1"`, `"a: b"`...). With [`Self::multiline`], strings of
    /// several lines are written as literal blocks, unless they are keys.
    ///
    /// # Errors
    /// See [`Self::feed_event`].
    pub fn scalar_str(&mut self, v: &str) -> EventEmitResult {
        self.scalar(&Scalar::Str(v))
    }

    /// The level of the innermost container, `-1` outside of any.
    #[allow(clippy::cast_possible_wrap)]
    fn level(&self) -> isize {
        // `max_depth` bounds the length: it cannot wrap around.
        self.stack.len() as isize - 1
    }

    fn write_indent(&mut self) -> EventEmitResult {
        let level = self.level();
        Ok(write_indent(self.writer, level, INDENT)?)
    }

    /// Start a new line, unless a literal block has just written its final line break.
    fn write_line_break(&mut self) -> EventEmitResult {
        if self.final_line_break.take().is_none() {
            self.writer.write_char('\n')?;
        }
        Ok(())
    }

    /// Write what comes before a node, and update what the container it is in expects.
    fn begin_node(
        &mut self,
        is_container: bool,
        explicit_key: bool,
    ) -> Result<Slot, EventEmitError> {
        let Some(&Frame {
            lead,
            nonempty,
            expect,
            ..
        }) = self.stack.last()
        else {
            if self.documents > 0 {
                self.writer.write_str("---\n")?;
            }
            return Ok(Slot::Root);
        };

        if !nonempty {
            // The container was waiting for its first node to know how to start.
            match lead {
                Lead::Root => {}
                Lead::Inline if self.compact => self.writer.write_char(' ')?,
                Lead::Inline | Lead::Block => {
                    self.writer.write_char('\n')?;
                    self.write_indent()?;
                }
            }
        }

        let (slot, next) = match expect {
            Expect::Item => {
                if nonempty {
                    self.write_line_break()?;
                    self.write_indent()?;
                }
                self.writer.write_char('-')?;
                (Slot::Inline, Expect::Item)
            }
            Expect::Key => {
                if nonempty {
                    self.write_line_break()?;
                    self.write_indent()?;
                }
                if is_container || explicit_key {
                    self.writer.write_char('?')?;
                    (Slot::Inline, Expect::Value { complex: true })
                } else {
                    (Slot::Key, Expect::Value { complex: false })
                }
            }
            Expect::Value { complex: true } => {
                self.write_line_break()?;
                self.write_indent()?;
                self.writer.write_char(':')?;
                (Slot::Inline, Expect::Key)
            }
            Expect::Value { complex: false } => (Slot::Value, Expect::Key),
        };

        if let Some(top) = self.stack.last_mut() {
            top.nonempty = true;
            top.expect = next;
        }
        Ok(slot)
    }

    fn container_start(&mut self, is_mapping: bool) -> EventEmitResult {
        if self.stack.len() >= self.max_depth {
            return Err(EventEmitError::TooDeep(self.max_depth));
        }
        let lead = match self.begin_node(true, false)? {
            Slot::Root => Lead::Root,
            Slot::Inline | Slot::Key => Lead::Inline,
            Slot::Value => Lead::Block,
        };
        self.stack.push(Frame {
            is_mapping,
            lead,
            nonempty: false,
            expect: if is_mapping {
                Expect::Key
            } else {
                Expect::Item
            },
        });
        Ok(())
    }

    fn container_end(&mut self, is_mapping: bool) -> EventEmitResult {
        let Some(&Frame {
            is_mapping: open_is_mapping,
            lead,
            nonempty,
            expect,
        }) = self.stack.last()
        else {
            return Err(EventEmitError::InvalidEvent("no container is open"));
        };
        if open_is_mapping != is_mapping {
            return Err(EventEmitError::InvalidEvent(if is_mapping {
                "the end of a mapping closes a sequence"
            } else {
                "the end of a sequence closes a mapping"
            }));
        }
        if matches!(expect, Expect::Value { .. }) {
            return Err(EventEmitError::InvalidEvent("a key has no value"));
        }

        if !nonempty {
            if lead != Lead::Root {
                self.writer.write_char(' ')?;
            }
            self.writer
                .write_str(if is_mapping { "{}" } else { "[]" })?;
        }
        self.stack.pop();
        self.end_node()
    }

    /// Called when a node is complete, to end the document if it was the root.
    fn end_node(&mut self) -> EventEmitResult {
        if self.stack.is_empty() {
            match self.final_line_break.take() {
                None => self.writer.write_char('\n')?,
                Some(Chomping::Clip) => {}
                // The kept line breaks would take whatever follows for text of the block.
                Some(Chomping::Keep) => self.writer.write_str("...\n")?,
            }
            self.documents += 1;
        }
        Ok(())
    }

    fn scalar(&mut self, scalar: &Scalar<'_>) -> EventEmitResult {
        // An implicit key cannot be too long, however it is quoted.
        let explicit_key = matches!(*scalar, Scalar::Str(v) if !fits_implicit_key(v));
        let slot = self.begin_node(false, explicit_key)?;
        if slot == Slot::Inline || slot == Slot::Value {
            self.writer.write_char(' ')?;
        }
        match *scalar {
            Scalar::Null => self.writer.write_char('~')?,
            Scalar::Bool(v) => self.writer.write_str(if v { "true" } else { "false" })?,
            Scalar::I64(v) => write!(self.writer, "{v}")?,
            Scalar::U64(v) => write!(self.writer, "{v}")?,
            Scalar::I128(v) => write!(self.writer, "{v}")?,
            Scalar::U128(v) => write!(self.writer, "{v}")?,
            Scalar::F32(v) => write_f32(self.writer, v)?,
            Scalar::F64(v) => write_f64(self.writer, v)?,
            Scalar::Str(v) => {
                // A literal block cannot be used as an implicit key.
                if self.multiline && slot != Slot::Key && can_be_literal_block(v) {
                    let level = self.level();
                    self.final_line_break = write_literal_block(self.writer, v, INDENT, level)?;
                } else if need_quotes(v) {
                    escape_str(self.writer, v)?;
                } else {
                    self.writer.write_str(v)?;
                }
            }
        }
        if slot == Slot::Key {
            self.writer.write_char(':')?;
        }
        self.end_node()
    }
}

/// The longest implicit key, in characters, quotes included (YAML 1.2, 7.4.2).
const MAX_IMPLICIT_KEY: usize = 1024;

/// Whether the string, as it is written, is short enough to be an implicit key.
fn fits_implicit_key(v: &str) -> bool {
    // Quoting writes at most 6 characters for a byte (`\u001f`), and 2 for the quotes.
    if v.len() <= MAX_IMPLICIT_KEY / 6 - 2 {
        return true;
    }
    let quoted = need_quotes(v);
    let mut len = if quoted { 2 } else { 0 };
    for c in v.chars() {
        len += match c {
            _ if !quoted => 1,
            '"' | '\\' | '\t' | '\n' | '\x08' | '\x0c' | '\r' => 2,
            '\0'..='\x1f' | '\x7f' => 6,
            _ => 1,
        };
        if len > MAX_IMPLICIT_KEY {
            return false;
        }
    }
    true
}
