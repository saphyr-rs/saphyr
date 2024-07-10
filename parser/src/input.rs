pub mod buffered;
pub mod str;

#[allow(clippy::module_name_repetitions)]
pub use buffered::BufferedInput;

use crate::char_traits::{is_blank_or_breakz, is_breakz, is_flow};

/// Interface for a source of characters.
///
/// Hiding the input's implementation behind this trait allows mostly:
///  * For input-specific optimizations (for instance, using `str` methods instead of manually
///    transferring one `char` at a time to a buffer).
///  * To return `&str`s referencing the input string, thus avoiding potentially costly
///    allocations. Should users need an owned version of the data, they can always `.to_owned()`
///    their YAML object.
pub trait Input {
    /// A hint to the input source that we will need to read `count` characters.
    ///
    /// If the input is exhausted, `\0` can be used to pad the last characters and later returned.
    /// The characters must not be consumed, but may be placed in an internal buffer.
    ///
    /// This method may be a no-op if buffering yields no performance improvement.
    ///
    /// Implementers of [`Input`] must _not_ load more than `count` characters into the buffer. The
    /// parser tracks how many characters are loaded in the buffer and acts accordingly.
    fn lookahead(&mut self, count: usize);

    /// Return the number of buffered characters in `self`.
    #[must_use]
    fn buflen(&self) -> usize;

    /// Return the capacity of the buffer in `self`.
    #[must_use]
    fn bufmaxlen(&self) -> usize;

    /// Return whether the buffer (!= stream) is empty.
    #[inline]
    #[must_use]
    fn buf_is_empty(&self) -> bool {
        self.buflen() == 0
    }

    /// Read a character from the input stream and return it directly.
    ///
    /// The internal buffer (is any) is bypassed.
    #[must_use]
    fn raw_read_ch(&mut self) -> char;

    /// Put a character back in the buffer.
    ///
    /// This function is only called when we read one too many characters and the pushed back
    /// character is exactly the last character that was read. This function will not be called
    /// multiple times consecutively.
    fn push_back(&mut self, c: char);

    /// Consume the next character.
    fn skip(&mut self);

    /// Consume the next `count` character.
    fn skip_n(&mut self, count: usize);

    /// Return the next character, without consuming it.
    ///
    /// Users of the [`Input`] must make sure that the character has been loaded through a prior
    /// call to [`Input::lookahead`]. Implementors of [`Input`] may assume that a valid call to
    /// [`Input::lookahead`] has been made beforehand.
    ///
    /// # Return
    /// If the input source is not exhausted, returns the next character to be fed into the
    /// scanner. Otherwise, returns `\0`.
    #[must_use]
    fn peek(&self) -> char;

    /// Return the `n`-th character in the buffer, without consuming it.
    ///
    /// This function assumes that the n-th character in the input has already been fetched through
    /// [`Input::lookahead`].
    #[must_use]
    fn peek_nth(&self, n: usize) -> char;

    /// Look for the next character and return it.
    ///
    /// The character is not consumed.
    /// Equivalent to calling [`Input::lookahead`] and [`Input::peek`].
    #[inline]
    #[must_use]
    fn look_ch(&mut self) -> char {
        self.lookahead(1);
        self.peek()
    }

    /// Return whether the next character in the input source is equal to `c`.
    ///
    /// This function assumes that the next character in the input has already been fetched through
    /// [`Input::lookahead`].
    #[inline]
    #[must_use]
    fn next_char_is(&self, c: char) -> bool {
        self.peek() == c
    }

    /// Return whether the `n`-th character in the input source is equal to `c`.
    ///
    /// This function assumes that the n-th character in the input has already been fetched through
    /// [`Input::lookahead`].
    #[inline]
    #[must_use]
    fn nth_char_is(&self, n: usize, c: char) -> bool {
        self.peek_nth(n) == c
    }

    /// Return whether the next 2 characters in the input source match the given characters.
    ///
    /// This function assumes that the next 2 characters in the input have already been fetched
    /// through [`Input::lookahead`].
    #[inline]
    #[must_use]
    fn next_2_are(&self, c1: char, c2: char) -> bool {
        assert!(self.buflen() >= 2);
        self.peek() == c1 && self.peek_nth(1) == c2
    }

    /// Return whether the next 3 characters in the input source match the given characters.
    ///
    /// This function assumes that the next 3 characters in the input have already been fetched
    /// through [`Input::lookahead`].
    #[inline]
    #[must_use]
    fn next_3_are(&self, c1: char, c2: char, c3: char) -> bool {
        assert!(self.buflen() >= 3);
        self.peek() == c1 && self.peek_nth(1) == c2 && self.peek_nth(2) == c3
    }

    /// Check whether the next characters correspond to a document indicator.
    ///
    /// This function assumes that the next 4 characters in the input has already been fetched
    /// through [`Input::lookahead`].
    #[inline]
    #[must_use]
    fn next_is_document_indicator(&self) -> bool {
        assert!(self.buflen() >= 4);
        is_blank_or_breakz(self.peek_nth(3))
            && (self.next_3_are('.', '.', '.') || self.next_3_are('-', '-', '-'))
    }

    /// Check whether the next characters correspond to a start of document.
    ///
    /// This function assumes that the next 4 characters in the input has already been fetched
    /// through [`Input::lookahead`].
    #[inline]
    #[must_use]
    fn next_is_document_start(&self) -> bool {
        assert!(self.buflen() >= 4);
        self.next_3_are('-', '-', '-') && is_blank_or_breakz(self.peek_nth(3))
    }

    /// Check whether the next characters correspond to an end of document.
    ///
    /// This function assumes that the next 4 characters in the input has already been fetched
    /// through [`Input::lookahead`].
    #[inline]
    #[must_use]
    fn next_is_document_end(&self) -> bool {
        assert!(self.buflen() >= 4);
        self.next_3_are('.', '.', '.') && is_blank_or_breakz(self.peek_nth(3))
    }

    /// Skip yaml whitespace at most up to eol. Also skips comments. Advances the input.
    ///
    /// # Return
    /// Return a tuple with the number of characters that were consumed and the result of skipping
    /// whitespace. The number of characters returned can be used to advance the index and columns,
    /// since no end-of-line character will be consumed.
    /// See [`SkipTabs`] For more details on the success variant.
    ///
    /// # Errors
    /// Errors if a comment is encountered but it was not preceded by a whitespace. In that event,
    /// the first tuple element will contain the number of characters consumed prior to reaching
    /// the `#`.
    fn skip_ws_to_eol(&mut self, skip_tabs: SkipTabs) -> (usize, Result<SkipTabs, &'static str>) {
        let mut encountered_tab = false;
        let mut has_yaml_ws = false;
        let mut chars_consumed = 0;
        loop {
            match self.look_ch() {
                ' ' => {
                    has_yaml_ws = true;
                    self.skip();
                }
                '\t' if skip_tabs != SkipTabs::No => {
                    encountered_tab = true;
                    self.skip();
                }
                // YAML comments must be preceded by whitespace.
                '#' if !encountered_tab && !has_yaml_ws => {
                    return (
                        chars_consumed,
                        Err("comments must be separated from other tokens by whitespace"),
                    );
                }
                '#' => {
                    while !is_breakz(self.look_ch()) {
                        self.skip();
                        chars_consumed += 1;
                    }
                }
                _ => break,
            }
            chars_consumed += 1;
        }

        (
            chars_consumed,
            Ok(SkipTabs::Result(encountered_tab, has_yaml_ws)),
        )
    }

    /// Check whether the next characters may be part of a plain scalar.
    ///
    /// This function assumes we are not given a blankz character.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn next_can_be_plain_scalar(&self, in_flow: bool) -> bool {
        let nc = self.peek_nth(1);
        match self.peek() {
            // indicators can end a plain scalar, see 7.3.3. Plain Style
            ':' if is_blank_or_breakz(nc) || (in_flow && is_flow(nc)) => false,
            c if in_flow && is_flow(c) => false,
            _ => true,
        }
    }
}

/// Behavior to adopt regarding treating tabs as whitespace.
///
/// Although tab is a valid yaml whitespace, it doesn't always behave the same as a space.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SkipTabs {
    /// Skip all tabs as whitespace.
    Yes,
    /// Don't skip any tab. Return from the function when encountering one.
    No,
    /// Return value from the function.
    Result(
        /// Whether tabs were encountered.
        bool,
        /// Whether at least 1 valid yaml whitespace has been encountered.
        bool,
    ),
}

impl SkipTabs {
    /// Whether tabs were found while skipping whitespace.
    ///
    /// This function must be called after a call to `skip_ws_to_eol`.
    pub fn found_tabs(self) -> bool {
        matches!(self, SkipTabs::Result(true, _))
    }

    /// Whether a valid YAML whitespace has been found in skipped-over content.
    ///
    /// This function must be called after a call to `skip_ws_to_eol`.
    pub fn has_valid_yaml_ws(self) -> bool {
        matches!(self, SkipTabs::Result(_, true))
    }
}
