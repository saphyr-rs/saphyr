#![allow(dead_code, unused)]

use std::{fmt, ops::BitOrAssign};

use crate::EmitResult;

/// An event to feed to the [`YamlEventEmitter`].
///
/// To each of these events corresponds a method in [`YamlEventEmitter`] which can be directly
/// called to spare a `match`.
#[derive(Debug)]
pub enum EmitEvent<'a> {
    /// Start of a YAML document.
    ///
    /// The first `NewDocument` is omitted.
    NewDocument,
    /// The start of a sequence.
    SequenceStart,
    /// The end of a sequence.
    SequenceEnd,
    /// The start of a mapping.
    MappingStart,
    /// The end of a mapping.
    MappingEnd,

    /// A boolean to emit.
    Bool(bool),
    /// An `i64` to emit.
    I64(i64),
    /// An `f64` to emit.
    F64(f64),
    /// A string to emit.
    String(&'a str),
    /// A `null` value to emit.
    Null,
}

/// A lower-level YAML serializer that expects events rather than objects.
///
/// For a higher-level and more user-friendly emitter, see [`YamlEmitter`].
///
/// This emitter is mostly intended as a back-end for [`YamlEmitter`] and the serde `Serializer`
/// implementation. Error checking is left to the user. Namely, this emitter assumes:
///
/// - A [`SequenceStart`] will always be matched by a [`SequenceEnd`].
/// - A [`SequenceEnd`] always has a matching [`SequenceStart`].
/// - A [`MappingStart`] will always be matched by a [`MappingEnd`].
/// - A [`MappingEnd`] always has a matching [`MappingStart`].
/// - There is a balance between keys and values in a mapping.
///
/// Breaking these assumptions may result in invalid YAML output or logic errors in the emitter.
///
/// [`YamlEmitter`]: `crate::YamlEmitter`
/// [`SequenceStart`]: `EmitEvent::SequenceStart`
/// [`SequenceEnd`]: `EmitEvent::SequenceEnd`
/// [`MappingStart`]: `EmitEvent::MappingStart`
/// [`MappingEnd`]: `EmitEvent::MappingEnd`
pub struct YamlEventEmitter<'writer> {
    /// The output for serialized YAML.
    writer: &'writer mut dyn fmt::Write,
    /// The number of spaces to add for indentation.
    indent: usize,
    /// Whether "compact inline notation" is used.
    ///
    /// See [`Self::compact`].
    compact: bool,
    /// Whether we emit multiline strings.
    ///
    /// See [`Self::multiline_strings`].
    multiline_strings: bool,
    /// The indentation nesting level.
    ///
    /// This is set to -1 upon starting a document. If the document top-node is not a container,
    /// this will remain set to -1. If the document top-node is a container, then the level will be
    /// set to 0 for it (each line begins with a `-`).
    level: isize,
    /// Stack of states for nested container.
    container_info: ContainerInfoStack,
}

impl<'writer> YamlEventEmitter<'writer> {
    /// Create a new emitter serializing into `writer`.
    pub fn new(writer: &'writer mut dyn fmt::Write) -> Self {
        Self {
            writer,
            indent: 2,
            compact: true,
            multiline_strings: false,
            level: -1,
            container_info: ContainerInfoStack::new(),
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
    ///
    /// In this form, blocks cannot have any properties (such as anchors
    /// or tags).
    ///
    /// TODO(ethiraric, 2024/04/02): Support anchors, tags.
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
    /// baz: 42");
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

    /// Feed an event to the emitter.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    pub fn feed_event(&mut self, ev: &EmitEvent) -> EmitResult {
        match ev {
            EmitEvent::NewDocument => self.new_document(),
            EmitEvent::SequenceStart => {
                self.sequence_start();
                Ok(())
            }
            EmitEvent::SequenceEnd => self.sequence_end(),
            EmitEvent::MappingStart => {
                self.mapping_start();
                Ok(())
            }
            EmitEvent::MappingEnd => self.mapping_end(),
            EmitEvent::Bool(v) => self.scalar_bool(*v),
            EmitEvent::I64(v) => self.scalar_i64(*v),
            EmitEvent::F64(v) => self.scalar_f64(*v),
            EmitEvent::String(v) => self.scalar_str(v),
            EmitEvent::Null => self.scalar_null(),
        }
    }

    /// Mark the start of a new document.
    ///
    /// This MUST NOT be called to start the first document. The start of the first document is
    /// implicit at the creation of the [`YamlEventEmitter`] and calling it first will emit an
    /// empty document and start a second one.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    pub fn new_document(&mut self) -> EmitResult {
        writeln!(self.writer, "---")?;
        self.level = -1;
        Ok(())
    }

    /// Mark the start of a new sequence.
    pub fn sequence_start(&mut self) {
        self.node_preamble(true);
        self.level += 1;
        self.container_info.push_sequence();
    }

    /// Mark the end of a sequence.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    pub fn sequence_end(&mut self) -> EmitResult {
        if self.container_info.top_is_empty() {
            self.writer.write_str("[]")?;
        }
        self.level -= 1;
        self.container_info.pop();
        Ok(())
    }

    /// Mark the start of a new mapping.
    pub fn mapping_start(&mut self) {
        self.node_preamble(true);
        self.level += 1;
        self.container_info.push_mapping();
    }

    /// Mark the end of a mapping.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    pub fn mapping_end(&mut self) -> EmitResult {
        if self.container_info.top_is_empty() {
            self.writer.write_str("{}")?;
        }
        self.level -= 1;
        self.container_info.pop();
        Ok(())
    }

    /// Emit a boolean value.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    pub fn scalar_bool(&mut self, v: bool) -> EmitResult {
        self.node_preamble(false);
        Ok(self.writer.write_str(if v { "true" } else { "false" })?)
    }

    /// Emit an `i64` value.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    pub fn scalar_i64(&mut self, v: i64) -> EmitResult {
        self.node_preamble(false);
        Ok(write!(self.writer, "{v}")?)
    }

    /// Emit an `f64` value.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    pub fn scalar_f64(&mut self, v: f64) -> EmitResult {
        self.node_preamble(false);
        Ok(write!(self.writer, "{v}")?)
    }

    /// Emit a string value.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    pub fn scalar_str(&mut self, v: &str) -> EmitResult {
        self.node_preamble(false);
        Ok(write!(self.writer, "{v}")?)
    }

    /// Emit a null value.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    pub fn scalar_null(&mut self) -> EmitResult {
        self.node_preamble(false);
        Ok(self.writer.write_str("null")?)
    }

    /// Emit an empty sequence to the output.
    ///
    /// This is a shorthand for both a [`SequenceStart`] and [`SequenceEnd`] which emits `[]`
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    ///
    /// [`SequenceStart`]: `EmitEvent::SequenceStart`
    /// [`SequenceEnd`]: `EmitEvent::SequenceEnd`
    pub fn emit_empty_sequence(&mut self) -> EmitResult {
        self.node_preamble(true)?;
        Ok(self.writer.write_str("[]")?)
    }

    /// Operations to perform before emitting a node.
    ///
    /// This must be called prior to emitting a node, no matter whether it is a scalar or a
    /// container. This must be called prior to changing the `self.container_info` stack.
    fn node_preamble(&mut self, is_container: bool) -> EmitResult {
        // Our top-level node is a scalar. Nothing to do.
        if self.level == -1 {
            return Ok(());
        }

        // Print the `-` for sequences, or `:` for mapping values.
        // If the container isn't empty, we need to prepend a newline.
        if self.container_info.top_is_sequence() {
            // Need a newline after the first element.
            if !self.container_info.top_is_empty() {
                self.write_lnindent()?;
            }
            self.writer.write_char('-')?;
        } else {
            // This is a mapping, since `self.level != -1`.
            if self.container_info.top_expects_key() {
                // Need a newline after the first element.
                if !self.container_info.top_is_empty() {
                    self.write_lnindent()?;
                }
                // We have a complex key.
                if is_container {
                    self.writer.write_char('?')?;
                }
            } else {
                // We expect a value.
                self.writer.write_char(':')?;
                if is_container {
                    self.level += 1;
                    self.write_lnindent();
                    self.level -= 1;
                }
            }
        }

        if (self.compact && self.container_info.top_is_sequence())
            || (self.compact
                && self.container_info.top_is_mapping()
                && self.container_info.top_expects_value()
                && !is_container)
        {
            self.writer.write_char(' ')?;
        }

        self.container_info.set_top_nonempty();
        if self.container_info.top_is_mapping() {
            self.container_info.toggle_top_expects_key();
        }
        Ok(())
    }

    /// Write a newline and as many spaces as the current indentation level expects.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    fn write_lnindent(&mut self) -> EmitResult {
        writeln!(self.writer)?;
        self.write_indent()
    }

    /// Write as many spaces as the current indentation level expects.
    ///
    /// # Errors
    /// Returns an error if writing to the output fails.
    #[allow(clippy::cast_sign_loss)]
    fn write_indent(&mut self) -> EmitResult {
        // Buffer of 64 spaces.                                    There's another quote here v
        const SPACES: &str = "                                                                ";

        let mut spaces_left = self.indent * (self.level as usize);
        while spaces_left > 0 {
            // Write at most 64 or `n_spaces` spaces.
            let n_spaces = spaces_left.min(SPACES.len());
            let slice = &SPACES[0..n_spaces];
            self.writer.write_str(slice)?;
            spaces_left -= n_spaces;
        }
        Ok(())
    }
}

/// Stack of bit values to keep track of some container data in [`YamlEventEmitter`]
///
/// This keeps track of:
///  - container emptiness (to correctly emit empty containers  as `[]` and `{}`)
///  - whether a container is a mapping or a sequence
///  - whether we emit a key or value for a mapping
///
/// This implementation supports 255 nested containers (I do hope this is enough).
struct ContainerInfoStack {
    /// The bit values.
    ///
    /// We use a nibble for each container (that's a bit wasted for each container, but it
    /// simplifies the code _a lot_). In this nibble (from MSb to LSb):
    ///
    ///   - emptiness bit: 0 if container is empty, 1 if it has at least 1 element
    ///   - container type: 0 if container is a mapping, 1 if it is a sequence
    ///   - key-value (mapping only): 0 if the next item is a key, 1 if it is a value
    #[allow(clippy::doc_markdown)]
    bits: [u8; 128],
    /// The current nesting level of containers.
    size: u8,
}

impl ContainerInfoStack {
    /// Create a new stack with 0 nesting.
    fn new() -> Self {
        Self {
            bits: [0; 128],
            size: 0,
        }
    }

    /// Mark that we entered a new mapping.
    fn push_mapping(&mut self) {
        self.size += 1;
        // The bits we would have to set are all 0, no need for further processing.
    }

    /// Mark that we entered a new sequence.
    fn push_sequence(&mut self) {
        // (empty, sequence, _)
        const MASK: u8 = 0b010;
        self.size += 1;
        if self.size % 2 == 0 {
            self.bits[(self.size / 2) as usize] |= MASK;
        } else {
            self.bits[(self.size / 2) as usize] |= (MASK << 4);
        }
    }

    /// Mark the end of a container.
    fn pop(&mut self) {
        // Reset bits to 0.
        if self.size % 2 == 0 {
            self.bits[(self.size / 2) as usize] &= 0xF0;
        } else {
            self.bits[(self.size / 2) as usize] &= 0x0F;
        }
        self.size -= 1;
    }

    /// Check whether the top container is empty.
    fn top_is_empty(&self) -> bool {
        const MASK: u8 = 0b100;
        self.bitand_mask_to_top(MASK)
    }

    /// Set the bit of the top-most container for emptiness to non-empty.
    fn set_top_nonempty(&mut self) {
        const MASK: u8 = 0b100;
        if self.size % 2 == 0 {
            self.bits[(self.size / 2) as usize] |= MASK;
        } else {
            self.bits[(self.size / 2) as usize] |= (MASK << 4);
        }
    }

    /// Check whether the top container is a mapping.
    fn top_is_mapping(&self) -> bool {
        const MASK: u8 = 0b010;
        self.bitand_mask_to_top(MASK)
    }

    /// Check whether the top container is a sequence.
    fn top_is_sequence(&self) -> bool {
        !self.top_is_mapping()
    }

    /// Check whether the top container expects a key.
    ///
    /// # Return
    /// If the top container is a mapping, return `true` if it expects a key, `false` otherwise.
    /// If the top container is a sequence, return value is not defined.
    fn top_expects_key(&self) -> bool {
        const MASK: u8 = 0b001;
        self.bitand_mask_to_top(MASK)
    }

    /// Check whether the top container expects a value.
    ///
    /// # Return
    /// If the top container is a mapping, return `true` if it expects a key, `false` otherwise.
    /// If the top container is a sequence, return value is not defined.
    fn top_expects_value(&self) -> bool {
        !self.top_expects_key()
    }

    /// Toggle the bit for key-value for mappings.
    fn toggle_top_expects_key(&mut self) {
        const MASK: u8 = 0b001;
        if self.size % 2 == 0 {
            self.bits[(self.size / 2) as usize] ^= MASK;
        } else {
            self.bits[(self.size / 2) as usize] ^= (MASK << 4);
        }
    }

    /// Perform a bitand operation with the given mask on the top container data.
    ///
    /// # Return
    /// Returns whether the bitand result is 0.
    fn bitand_mask_to_top(&self, mask: u8) -> bool {
        if self.size % 2 == 0 {
            (self.bits[(self.size / 2) as usize] & mask) == 0
        } else {
            (self.bits[(self.size / 2) as usize] & (mask << 4)) == 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EmitEvent::{
            self, MappingEnd, MappingStart, SequenceEnd, SequenceStart, String as Str, I64,
        },
        YamlEventEmitter,
    };

    fn feed(emitter: &mut YamlEventEmitter<'_>, events: &[EmitEvent]) {
        for event in events {
            emitter.feed_event(event).unwrap();
        }
    }

    #[test]
    fn one_root_string() {
        let events = &[Str("foo")];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "foo");
    }

    #[test]
    fn one_root_integer() {
        let events = &[I64(32)];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "32");
    }

    #[test]
    fn one_root_sequence() {
        let events = &[SequenceStart, I64(32), SequenceEnd];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "- 32");
    }

    #[test]
    fn longer_root_sequence() {
        let events = &[SequenceStart, I64(32), Str("foo"), SequenceEnd];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "- 32\n- foo");
    }

    #[test]
    fn nested_sequences() {
        let events = &[
            SequenceStart,
            SequenceStart,
            I64(32),
            SequenceEnd,
            Str("foo"),
            SequenceEnd,
        ];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "- - 32\n- foo");
    }

    #[test]
    fn one_root_mapping() {
        let events = &[MappingStart, Str("foo"), I64(32), MappingEnd];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "foo: 32");
    }

    #[test]
    fn longer_root_mapping() {
        let events = &[
            MappingStart,
            Str("foo"),
            I64(32),
            Str("bar"),
            Str("baz"),
            MappingEnd,
        ];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "foo: 32\nbar: baz");
    }

    #[test]
    fn nested_mappings() {
        let events = &[
            MappingStart,
            Str("foo"),
            MappingStart,
            Str("bar"),
            Str("baz"),
            MappingEnd,
            MappingEnd,
        ];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "foo:\n  bar: baz");
    }

    fn empty_container_in_sequence() {
        let events = &[
            SequenceStart,
            Str("foo"),
            SequenceStart,
            SequenceEnd,
            Str("bar"),
            SequenceEnd,
        ];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "- foo\n- []\n- bar");
    }

    fn empty_container_in_mapping() {
        let events = &[
            MappingStart,
            Str("foo"),
            Str("bar"),
            Str("empty"),
            MappingStart,
            MappingEnd,
            Str("baz"),
            Str("quux"),
            MappingEnd,
        ];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "foo: bar\nempty: {}\nbaz: quux");
    }

    fn nested_empty_container_in_sequence() {
        let events = &[
            SequenceStart,
            Str("foo"),
            SequenceStart,
            SequenceStart,
            SequenceEnd,
            SequenceEnd,
            Str("bar"),
            SequenceEnd,
        ];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "- foo\n- - []\n- bar");
    }

    fn nested_empty_container_in_mapping() {
        let events = &[
            MappingStart,
            Str("foo"),
            Str("bar"),
            Str("empty"),
            MappingStart,
            Str("foo"),
            MappingStart,
            MappingEnd,
            MappingEnd,
            Str("baz"),
            Str("quux"),
            MappingEnd,
        ];
        let mut buffer = String::new();
        let mut emitter = YamlEventEmitter::new(&mut buffer);
        feed(&mut emitter, events);
        assert_eq!(buffer, "foo: bar\nempty:\n  foo: {}\nbaz: quux");
    }
}
