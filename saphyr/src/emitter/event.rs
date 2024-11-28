//! Home to the [`EventYamlEmitter`] and its associated types.

use std::fmt;

use saphyr_parser::TScalarStyle;

use crate::emitter::{EmitError, EmitResult};

/// A lower-level YAML serializer that is fed events instead of a fully constructed object.
///
/// This serializer is a building block for [`YamlEmitter`]. It takes [`EmitterEvent`]s and builds
/// the output on the go. If the destination is not an in-memory buffer, then this emitter is a
/// more lightweight alternative (in terms of memory footprint) as it does not need to work with a
/// [`Yaml`] instance.
///
/// Events are expected to be coherent. The emitter won't panic, but may behave unexpectedely
/// namely if:
/// - Documents aren't started properly ([`DocumentStart`])
/// - There is an imbalance in collection starting and ending events
///
/// # Example
/// ```
/// use saphyr::{EmitterEvent, EventYamlEmitter, TScalarStyle};
///
/// let mut output = String::new();
/// let mut emitter = EventYamlEmitter::new(&mut output);
/// emitter.on_event(EmitterEvent::DocumentStart(true));
/// emitter.on_event(EmitterEvent::MappingStart(None));
/// emitter.on_scalar("a", TScalarStyle::Plain);
/// emitter.on_event(EmitterEvent::SequenceStart(None));
/// emitter.on_scalar("b", TScalarStyle::Plain);
/// emitter.on_scalar("c", TScalarStyle::Plain);
/// emitter.on_event(EmitterEvent::SequenceEnd);
/// emitter.on_event(EmitterEvent::MappingEnd);
/// emitter.on_event(EmitterEvent::DocumentEnd(false));
/// assert_eq!(output, r#"---
/// a:
///   - b
///   - c"#);
/// ```
///
/// [`DocumentStart`]: EmitterEvent::DocumentStart
/// [`YamlEmitter`]: crate::emitter::YamlEmitter
/// [`Yaml`]: crate::Yaml
#[allow(clippy::module_name_repetitions)]
pub struct EventYamlEmitter<'a> {
    /// The output stream in which we output YAML.
    writer: &'a mut dyn fmt::Write,
    /// Whether compact in-line notation is on or off.
    ///
    /// See [`Self::compact`].
    compact: bool,
    /// Whether we render multiline strings in literal style.
    ///
    /// See [`Self::multiline_strings`].
    multiline_strings: bool,
    /// How many spaces are added to a nested indentation level.
    indent_step: u32,
    /// The nesting of non-flow collections we are in.
    ///
    /// We can derive the indentation level from the number of elements that this vec holds.
    collections: Vec<CollectionKind>,
    /// The current state of the emitter.
    state: EmitterState,
}

impl<'a> EventYamlEmitter<'a> {
    /// Create a new emitter serializing into `writer`.
    pub fn new(writer: &'a mut dyn fmt::Write) -> Self {
        Self {
            writer,
            compact: true,
            multiline_strings: false,
            indent_step: 2,
            collections: vec![],
            state: EmitterState::Init,
        }
    }

    /// Set 'compact in-line notation' on or off, as described for block
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

    /// Determine if this emitter is using 'compact in-line notation'.
    ///
    /// See [`Self::compact`].
    #[must_use]
    pub fn is_compact(&self) -> bool {
        self.compact
    }

    /// Render strings containing multiple lines in [literal style].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use saphyr::{Yaml, YamlEmitter};
    /// #
    /// let input = r#"{foo: "bar\nbar", baz: 42}"#;
    /// let parsed = Yaml::load_from_str(input).unwrap();
    ///
    /// let mut output = String::new();
    /// let mut emitter = YamlEmitter::new(&mut output);
    /// emitter.multiline_strings(true);
    /// emitter.dump(&parsed[0]).unwrap();
    /// assert_eq!(output.as_str(), "\
    /// ---
    /// foo: |-
    ///   bar
    ///   bar
    /// baz: 42");
    /// ```
    ///
    /// [literal style]: https://yaml.org/spec/1.2/spec.html#id2795688
    pub fn multiline_strings(&mut self, multiline_strings: bool) {
        self.multiline_strings = multiline_strings;
    }

    /// Determine if this emitter will emit multiline strings when appropriate.
    ///
    /// See [`Self::multiline_strings`].
    #[must_use]
    pub fn is_multiline_strings(&self) -> bool {
        self.multiline_strings
    }

    /// Set how many spaces are added to a nested indentation level.
    pub fn indent_step(&mut self, indent_step: u32) {
        self.indent_step = indent_step;
    }

    /// Get how many spaces are added to a nested indentation level.
    #[must_use]
    pub fn get_indent_step(&self) -> u32 {
        self.indent_step
    }

    /// A convenience function for [`on_event`] with a [`Scalar`] event.
    ///
    /// # Errors
    /// Returns an error if outputting to the writer fails.
    ///
    /// [`on_event`]: Self::on_event
    /// [`Scalar`]: EmitterEvent::Scalar
    pub fn on_scalar(&mut self, value: &str, style: TScalarStyle) -> EmitResult {
        self.on_scalar_impl(&Scalar {
            tag: None,
            value,
            style,
        })
    }

    /// Feed a new event into the emitter.
    ///
    /// # Errors
    /// Returns an error if the given event is incoherent with the preceding sequence of events or
    /// if writing to the output writer failed.
    pub fn on_event(&mut self, event: EmitterEvent) -> EmitResult {
        match event {
            EmitterEvent::StreamStart | EmitterEvent::StreamEnd => {}
            EmitterEvent::DocumentStart(explicit) => self.on_document_start(explicit)?,
            EmitterEvent::DocumentEnd(explicit) => self.on_document_end(explicit)?,
            EmitterEvent::Scalar(scalar) => self.on_scalar_impl(&scalar)?,
            EmitterEvent::SequenceStart(tag) => {
                self.on_collection_start(CollectionKind::Sequence(SequenceState::Empty), &tag)?;
            }
            EmitterEvent::MappingStart(tag) => {
                self.on_collection_start(CollectionKind::Mapping(MappingState::Empty), &tag)?;
            }
            EmitterEvent::SequenceEnd => {
                // The value to `Sequence` here does not matter. We won't match against it.
                self.on_collection_end(CollectionKind::Sequence(SequenceState::Empty))?;
            }
            EmitterEvent::MappingEnd => {
                // The value to `Mapping` here does not matter. We won't match against it.
                self.on_collection_end(CollectionKind::Mapping(MappingState::ExpectsKey))?;
            }
        }
        Ok(())
    }

    /// Check the state allows starting a document and emit `---` if asked.
    ///
    /// # Errors
    /// Returns an error if outputting to the writer fails.
    pub fn on_document_start(&mut self, explicit: bool) -> EmitResult {
        // If the document was implicily ended, we still need to emit a document start.
        if explicit || self.state == EmitterState::DocumentEnded(Implicit) {
            writeln!(self.writer, "---")?;
        }
        self.state = EmitterState::DocumentStarted;
        Ok(())
    }

    /// Check the state allows ending a document and emit `...` if asked.
    ///
    /// # Errors
    /// Returns an error if outputting to the writer fails.
    pub fn on_document_end(&mut self, explicit: bool) -> EmitResult {
        if explicit {
            write!(self.writer, "...")?;
        }
        self.state = EmitterState::DocumentEnded(if explicit { Explicit } else { Implicit });
        Ok(())
    }

    /// Start a new collection.
    fn on_collection_start(&mut self, kind: CollectionKind, _tag: &Option<String>) -> EmitResult {
        // Emit newline and indent only if needed. We don't emit it:
        // - If we just started the document; this would make every emitted string with a root
        //   collection start with a newline.
        // - If our collection is a value in a mapping. Otherwise, our collections would look like:
        //   a
        //   :
        //     - b
        if !matches!(
            self.state,
            EmitterState::MappingExpectingValue | EmitterState::DocumentStarted
        ) {
            self.emit_lnindent()?;
        }

        match self.state {
            EmitterState::InSequence => {
                // Do not emit a space if we are not in compact mode. Otherwise, there would be a
                // trailing space ($ marks eol):
                // a:$
                //   - $
                //     foo: bar$
                if self.compact {
                    write!(self.writer, "- ")?;
                } else {
                    write!(self.writer, "-")?;
                }
            }
            EmitterState::MappingExpectingKey => {
                write!(self.writer, "? ")?;
            }
            EmitterState::MappingExpectingValue => {
                write!(self.writer, ":")?;
            }
            _ => {}
        };

        self.collections.push(kind);
        self.state = match kind {
            CollectionKind::Mapping(_) => EmitterState::MappingExpectingKey,
            CollectionKind::Sequence(_) => EmitterState::InSequence,
        };
        Ok(())
    }

    /// Check the collection end matches an associated collection start.
    ///
    /// # Errors
    /// This function returns an error if there is a mismatch or imbalance in the collection start
    /// and the collection end.
    fn on_collection_end(&mut self, ev: CollectionKind) -> EmitResult {
        use CollectionKind as Kind; // Shorthand to avoid awkward newlines in matches.

        if let Some(kind) = self.collections.pop() {
            match (kind, ev) {
                (Kind::Mapping(_), Kind::Sequence(_)) | (Kind::Sequence(_), Kind::Mapping(_)) => {
                    // We have either started a sequence and closed a mapping, or opened a mapping and
                    // closed a sequence.
                    return Err(EmitError::EventError("mismatch in collection start/end"));
                }
                (Kind::Mapping(MappingState::ExpectsValue), _) => {
                    return Err(EmitError::EventError(
                        "last mapping pair is missing its value",
                    ))
                }
                (Kind::Sequence(SequenceState::Empty), Kind::Sequence(_)) => {
                    // If the sequence is empty, we still need to emit it.
                    if self.at_mapping_value() {
                        // This prints the following space:
                        //   v
                        // a: []
                        write!(self.writer, " []")?;
                    } else {
                        write!(self.writer, "[]")?;
                    }
                }
                (Kind::Mapping(MappingState::Empty), Kind::Mapping(_)) => {
                    // If the mapping is empty, we still need to emit it.
                    if self.at_mapping_value() {
                        // This prints the following space:
                        //   v
                        // a: {}
                        write!(self.writer, " {{}}")?;
                    } else {
                        write!(self.writer, "{{}}")?;
                    }
                }
                (Kind::Sequence(_), Kind::Sequence(_))
                | (Kind::Mapping(MappingState::ExpectsKey), Kind::Mapping(_)) => {}
            }
            self.advance_state_with_new_item();

            // If we are now expecting a mapping value, this means that our collection was a
            // complex mapping key. This newline corresponds to that at the `#` below:
            // ? - foo
            //   - bar#
            // : baz
            if self.state == EmitterState::MappingExpectingValue {
                self.emit_lnindent()?;
            }

            Ok(())
        } else {
            // Can't end a collection if we haven't started any.
            Err(EmitError::EventError(
                "collection end with no matching collection start",
            ))
        }
    }

    /// Display the given scalar.
    ///
    /// # Errors
    /// Returns an error if outputting to the writer fails.
    fn on_scalar_impl(&mut self, scalar: &Scalar) -> EmitResult {
        // Don't emit the newline if we are ...
        if !(
            // At the beginning of the document or just after a `:` in a mapping.
            matches!(
                self.state,
                EmitterState::MappingExpectingValue | EmitterState::DocumentStarted
            )
            // Or at the first value in a sequence.
            || self.at_sequence_start()
            // Or at the first value of a mapping in the root document.
            || (self.at_mapping_start() && self.collections.len() == 1)
            // Or in compact mode where we could omit a newline (see
            // `at_mapping_start_in_sequence`).
                || (self.compact && self.at_mapping_start_in_sequence())
        ) {
            self.emit_lnindent()?;
        }

        // Write preceding tokens for collections.
        match self.state {
            EmitterState::InSequence => {
                if self.at_sequence_start() && self.in_sequence_a_mapping_value() {
                    // This is the newline that is inserted where the hash is in the example below:
                    //
                    // a:#
                    //   - b
                    self.emit_lnindent()?;
                }
                write!(self.writer, "- ")?;
            }
            EmitterState::MappingExpectingValue => {
                write!(self.writer, ": ")?;
            }
            _ => {}
        }

        match scalar.style {
            TScalarStyle::Plain => write!(self.writer, "{}", scalar.value)?,
            TScalarStyle::SingleQuoted => todo!(), // TODO(ethiraric, 24/11/2024)
            TScalarStyle::DoubleQuoted => emit_double_quoted_string(self.writer, scalar.value)?,
            TScalarStyle::Literal | TScalarStyle::Folded => self.emit_literal_block(scalar)?,
        }

        self.advance_state_with_new_item();
        Ok(())
    }

    /// Update the internal state when we have fully constructed a item.
    ///
    /// This must be called when we receive a scalar (which is an item) and when we receive a
    /// collection end event (the collection is an item, which can be a key, a value or an item in
    /// a sequence). In the latter case, it must be called _after_ the ending collection has been
    /// removed from `self.indent`.
    fn advance_state_with_new_item(&mut self) {
        if let Some(last_indent) = self.collections.last_mut() {
            // If we are in a collection, update its state.
            match last_indent {
                // If we had a value in a mapping, expect a key, and vice-versa.
                CollectionKind::Mapping(MappingState::ExpectsValue) => {
                    *last_indent = CollectionKind::Mapping(MappingState::ExpectsKey);
                    self.state = EmitterState::MappingExpectingKey;
                }
                CollectionKind::Mapping(MappingState::ExpectsKey | MappingState::Empty) => {
                    *last_indent = CollectionKind::Mapping(MappingState::ExpectsValue);
                    self.state = EmitterState::MappingExpectingValue;
                }
                // If we had a sequence, then it no longer is empty.
                CollectionKind::Sequence(_) => {
                    *last_indent = CollectionKind::Sequence(SequenceState::NonEmpty);
                    // If we were in a mapping inside a sequence, `self.state` would be
                    // `MappingExpectingKey`. We need to reset it to a
                    self.state = EmitterState::InSequence;
                }
            }
        } else {
            // If we no longer have any open collection, this means we have reached the top-level
            // scope. Our document is fully emitted.
            self.state = EmitterState::DocumentEmitted;
        }
    }

    /// Emit the given value as a literal block.
    ///
    /// The emitter must be positioned prior the `|` or `|-`.
    fn emit_literal_block(&mut self, scalar: &Scalar) -> EmitResult {
        let ends_with_newline = scalar.value.ends_with('\n');
        if ends_with_newline {
            self.writer.write_str("|")?;
        } else {
            self.writer.write_str("|-")?;
        }

        // lines() will omit the last line if it is empty.
        for line in scalar.value.lines() {
            // TODO(ethiraric, 24/11/2024): Handle folded scalars.
            self.emit_lnindent()?;
            // Indent the block further than its parent node.
            write!(self.writer, "  ")?;
            // It's literal text, so don't escape special chars.
            self.writer.write_str(line)?;
        }
        Ok(())
    }

    /// Emit a new line and indentation for it.
    fn emit_lnindent(&mut self) -> EmitResult {
        writeln!(self.writer)?;
        self.emit_indent()
    }

    /// Emit an amount of spaces equal to the current indentation.
    fn emit_indent(&mut self) -> EmitResult {
        for _ in 0..self.collections.len().saturating_sub(1) {
            write!(self.writer, "  ",)?;
        }
        Ok(())
    }

    /// Return true if we are outputting a sequence as a value in a mapping.
    ///
    /// Checks that the inner-most collection is a sequence whose immediate parent is a mapping.
    /// Also check that this sequence is a value in the parent mapping (i.e.: not a complex key).
    fn in_sequence_a_mapping_value(&self) -> bool {
        let len = self.collections.len();
        len >= 2
            && matches!(self.collections[len - 1], CollectionKind::Sequence(_))
            && matches!(
                self.collections[len - 2],
                CollectionKind::Mapping(MappingState::ExpectsValue)
            )
    }

    /// Return true if the inner-most collection is a mapping expecting a value.
    fn at_mapping_value(&self) -> bool {
        matches!(
            self.collections.last(),
            Some(CollectionKind::Mapping(MappingState::ExpectsValue))
        )
    }

    /// Return true if the inner-most collection is a yet-empty sequence.
    fn at_sequence_start(&self) -> bool {
        matches!(
            self.collections.last(),
            Some(CollectionKind::Sequence(SequenceState::Empty))
        )
    }

    /// Return true if the inner-most collection is a yet-empty mapping.
    fn at_mapping_start(&self) -> bool {
        matches!(
            self.collections.last(),
            Some(CollectionKind::Mapping(MappingState::Empty))
        )
    }

    /// Return true if we are at the first key in a mapping whose immediate parent is a sequence.
    ///
    /// Checks that the inner-most collection is a yet-empty mapping whose immediate parent is a
    /// sequence.
    ///
    /// Example:
    /// ```yaml
    /// - a: b
    /// ```
    /// Prior to emitting `a`, this function would return true.
    fn at_mapping_start_in_sequence(&self) -> bool {
        let len = self.collections.len();
        len >= 2
            && self.at_mapping_start()
            && matches!(
                self.collections.get(self.collections.len() - 2),
                Some(CollectionKind::Sequence(_))
            )
    }
}

/// The state of the emitter.
#[derive(PartialEq, Eq, Copy, Clone)]
enum EmitterState {
    /// We have just built an emitter.
    Init,
    /// We have started a new document (explicitly or implicitly) and are waiting for its contents.
    DocumentStarted,
    /// We have ended a document (explicitly or implicitly).
    DocumentEnded(Explicity),
    /// We have finished emitting the document, but have not yet received a [`DocumentEnd`].
    ///
    /// A YAML document is always a single item, whether it be a mapping, a sequence or a scalar.
    /// When we reach the end of that item, we enter the [`DocumentEmitted`] state.
    ///
    /// [`DocumentEnd`]: EmitterEvent::DocumentEnd
    /// [`DocumentEmitted`]: EmitterState::DocumentEmitted
    DocumentEmitted,
    /// Our inner-most collection is a sequence.
    InSequence,
    /// Our inner-most collection is a mapping. It expects a key (or mapping end) next.
    MappingExpectingKey,
    /// Our inner-most collection is a mapping. It expects a value next.
    MappingExpectingValue,
}

/// The kind of collection we opened in the emitter.
///
/// This serves for tracking whether the events we receive are correct.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum CollectionKind {
    /// We opened a mapping.
    Mapping(MappingState),
    /// We opened a sequence.
    Sequence(SequenceState),
}

/// The state of an opened mapping in the emitter.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum MappingState {
    /// The mapping has not yet gotten a key-value pair.
    ///
    /// In this state, the mapping expects a key. It is different from [`ExpectsKey`] in that it is
    /// used to know when to emit empty mappings (`{}`).
    ///
    /// [`ExpectsKey`]: MappingState::ExpectsKey
    Empty,
    /// The mapping was just opened or has successfully received pairs.
    ///
    /// If the next event is a scalar, it will be a key.
    ExpectsKey,
    /// The mapping has received a key but not its associated value yet.
    ///
    /// If the next event is a scalar, it will be a value.
    ExpectsValue,
}

/// The state of an opened sequence in the emitter.
///
/// We need to track this in case we need to emit an empty sequence. If we don't emit it, we would
/// read it back as a null value.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum SequenceState {
    /// The sequence is empty.
    Empty,
    /// At least one item has been added to the sequence.
    NonEmpty,
}

/// Fancy boolean value for whether something is implicit or explicit.
#[derive(PartialEq, Eq, Copy, Clone)]
enum Explicity {
    /// Explicit.
    Explicit,
    /// Implicit.
    Implicit,
}
use Explicity::{Explicit, Implicit};

/// YAML events to send the emitter.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum EmitterEvent<'a> {
    /// The stream started. This must be the first event sent.
    StreamStart,
    /// The stream has ended. The emitter performs final routines.
    StreamEnd,
    /// A document has started.
    DocumentStart(
        /// Whether the document is explicitly or implicitly started.
        bool,
    ),
    /// The current document has ended.
    DocumentEnd(
        /// Whether the document is explicitly or implicitly ended.
        bool,
    ),
    /// Emit a scalar.
    Scalar(Scalar<'a>),
    /// Start a sequence.
    SequenceStart(
        /// An optional YAML tag to the sequence.
        Option<String>,
    ),
    /// End a sequence.
    SequenceEnd,
    /// Start a mapping.
    MappingStart(
        /// An optional YAML tag to the mapping.
        Option<String>,
    ),
    /// End a mapping.
    MappingEnd,
}

/// A scalar to emit.
// TODO(ethiraric, 2024/11/11): Use it in `saphyr-parser` to replace `Boolean`, `Real`, ...
#[derive(Debug)]
pub struct Scalar<'a> {
    /// An optional YAML tag to the scalar.
    pub tag: Option<String>,
    /// The literal value of the scalar.
    ///
    /// If the scalar is not a string (number, boolean, ...) it must be strigified.
    pub value: &'a str,
    /// The style in which to emit the scalar.
    pub style: TScalarStyle,
}

/// Write the escaped double-quoted string into the given writer.
// from serialize::json
fn emit_double_quoted_string(wr: &mut dyn fmt::Write, v: &str) -> Result<(), fmt::Error> {
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
