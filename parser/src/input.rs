//! Utilities to create a source of input to the parser.
//!
//! [`Input`] must be implemented for the parser to fetch input. Make sure your needs aren't
//! covered by the [`BufferedInput`].

use alloc::string::String;
use core::{error::Error, fmt};

pub(crate) mod buffered;
pub(crate) mod str;

#[allow(clippy::module_name_repetitions)]
pub use buffered::{BufferedInput, InfallibleChars};

pub use crate::char_traits::{
    is_alpha, is_blank, is_blank_or_breakz, is_break, is_breakz, is_digit, is_flow, is_z,
};

/// An input source failure or a call made after that failure.
///
/// The source error is returned once. Later input operations return [`InputError::Failed`].
#[derive(Debug)]
pub enum InputError<E> {
    /// The original error returned by the input source.
    Source(E),
    /// The input source has already returned its error.
    Failed,
}

impl<E: fmt::Display> fmt::Display for InputError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(f, "input source error: {error}"),
            Self::Failed => f.write_str("input has failed"),
        }
    }
}

impl<E: Error + 'static> Error for InputError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Failed => None,
        }
    }
}

/// A fallible result from an [`Input`] operation.
pub type InputResult<T, E> = Result<T, InputError<E>>;

/// Interface for a source of characters.
///
/// Hiding the input's implementation behind this trait allows mostly:
///  * For input-specific optimizations (for instance, using `str` methods instead of manually
///    transferring one `char` at a time to a buffer).
///  * To return `&str`s referencing the input string, thus avoiding potentially costly
///    allocations. Should users need an owned version of the data, they can always `.to_owned()`
///    their YAML object.
pub trait Input {
    /// Error type returned by the underlying input source.
    type Error;
    /// A hint to the input source that we will need to read `count` characters.
    ///
    /// If the input is exhausted, `\0` can be used to pad the last characters and later returned.
    /// The characters must not be consumed, but may be placed in an internal buffer.
    ///
    /// This method may be a no-op if buffering yields no performance improvement.
    ///
    /// Implementers of [`Input`] must _not_ load more than `count` characters into the buffer. The
    /// parser tracks how many characters are loaded in the buffer and acts accordingly.
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn lookahead(&mut self, count: usize) -> InputResult<(), Self::Error>;

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
    /// The internal buffer (if any) is bypassed.
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn raw_read_ch(&mut self) -> InputResult<char, Self::Error>;

    /// Read a non-breakz a character from the input stream and return it directly.
    ///
    /// The internal buffer (if any) is bypassed.
    ///
    /// If the next character is a breakz, it is either not consumed or placed into the buffer (if
    /// any).
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn raw_read_non_breakz_ch(&mut self) -> InputResult<Option<char>, Self::Error>;

    /// Consume the next character.
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn skip(&mut self) -> InputResult<(), Self::Error>;

    /// Consume the next `count` character.
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn skip_n(&mut self, count: usize) -> InputResult<(), Self::Error>;

    /// Return the next character, without consuming it.
    ///
    /// Users of the [`Input`] must make sure that the character has been loaded through a prior
    /// call to [`Input::lookahead`]. Implementors of [`Input`] may assume that a valid call to
    /// [`Input::lookahead`] has been made beforehand.
    ///
    /// # Return
    /// If the input source is not exhausted, returns the next character to be fed into the
    /// scanner. Otherwise, returns `\0`.
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn peek(&mut self) -> InputResult<char, Self::Error>;

    /// Return the `n`-th character in the buffer, without consuming it.
    ///
    /// This function assumes that the n-th character in the input has already been fetched through
    /// [`Input::lookahead`].
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn peek_nth(&mut self, n: usize) -> InputResult<char, Self::Error>;

    /// Return the next character in the input source, without consuming it, unless the input
    /// source is exhausted.
    ///
    /// Unlike [`Input::peek`], this function does not conflate a NUL character with the
    /// end-of-stream sentinel: a raw NUL character in the stream is returned as `Some('\0')`,
    /// while an exhausted input source returns `None`. Padding characters introduced by
    /// [`Input::lookahead`] are never returned.
    ///
    /// This function assumes that the next character in the input has already been fetched through
    /// [`Input::lookahead`].
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn peek_opt(&mut self) -> InputResult<Option<char>, Self::Error>;

    /// Look for the next character and return it.
    ///
    /// The character is not consumed.
    /// Equivalent to calling [`Input::lookahead`] and [`Input::peek`].
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn look_ch(&mut self) -> InputResult<char, Self::Error> {
        self.lookahead(1)?;
        self.peek()
    }

    /// Return whether the next character in the input source is equal to `c`.
    ///
    /// This function assumes that the next character in the input has already been fetched through
    /// [`Input::lookahead`].
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_char_is(&mut self, c: char) -> InputResult<bool, Self::Error> {
        Ok(self.peek()? == c)
    }

    /// Return whether the `n`-th character in the input source is equal to `c`.
    ///
    /// This function assumes that the n-th character in the input has already been fetched through
    /// [`Input::lookahead`].
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn nth_char_is(&mut self, n: usize, c: char) -> InputResult<bool, Self::Error> {
        Ok(self.peek_nth(n)? == c)
    }

    /// Return whether the next 2 characters in the input source match the given characters.
    ///
    /// This function assumes that the next 2 characters in the input have already been fetched
    /// through [`Input::lookahead`].
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_2_are(&mut self, c1: char, c2: char) -> InputResult<bool, Self::Error> {
        assert!(self.buflen() >= 2);
        Ok(self.peek()? == c1 && self.peek_nth(1)? == c2)
    }

    /// Return whether the next 3 characters in the input source match the given characters.
    ///
    /// This function assumes that the next 3 characters in the input have already been fetched
    /// through [`Input::lookahead`].
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_3_are(&mut self, c1: char, c2: char, c3: char) -> InputResult<bool, Self::Error> {
        assert!(self.buflen() >= 3);
        Ok(self.peek()? == c1 && self.peek_nth(1)? == c2 && self.peek_nth(2)? == c3)
    }

    /// Check whether the next characters correspond to a document indicator.
    ///
    /// This function assumes that the next 4 characters in the input has already been fetched
    /// through [`Input::lookahead`].
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_document_indicator(&mut self) -> InputResult<bool, Self::Error> {
        assert!(self.buflen() >= 4);
        Ok(is_blank_or_breakz(self.peek_nth(3)?)
            && (self.next_3_are('.', '.', '.')? || self.next_3_are('-', '-', '-')?))
    }

    /// Check whether the next characters correspond to a start of document.
    ///
    /// This function assumes that the next 4 characters in the input has already been fetched
    /// through [`Input::lookahead`].
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_document_start(&mut self) -> InputResult<bool, Self::Error> {
        assert!(self.buflen() >= 4);
        Ok(self.next_3_are('-', '-', '-')? && is_blank_or_breakz(self.peek_nth(3)?))
    }

    /// Check whether the next characters correspond to an end of document.
    ///
    /// This function assumes that the next 4 characters in the input has already been fetched
    /// through [`Input::lookahead`].
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_document_end(&mut self) -> InputResult<bool, Self::Error> {
        assert!(self.buflen() >= 4);
        Ok(self.next_3_are('.', '.', '.')? && is_blank_or_breakz(self.peek_nth(3)?))
    }

    /// Skip yaml whitespace at most up to eol. Also skips comments. Advances the input.
    ///
    /// # Return
    /// Return a tuple with the number of characters that were consumed and the result of skipping
    /// whitespace. The number of characters returned can be used to advance the index and column,
    /// since no end-of-line character will be consumed.
    /// See [`SkipTabs`] For more details on the success variant.
    ///
    /// # Errors
    /// Errors if a comment is encountered but it was not preceded by a whitespace. In that event,
    /// the first tuple element will contain the number of characters consumed prior to reaching
    /// the `#`.
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn skip_ws_to_eol(
        &mut self,
        skip_tabs: SkipTabs,
    ) -> InputResult<(usize, Result<SkipTabs, &'static str>), Self::Error> {
        let mut encountered_tab = false;
        let mut has_yaml_ws = false;
        let mut chars_consumed = 0;
        loop {
            match self.look_ch()? {
                ' ' => {
                    has_yaml_ws = true;
                    self.skip()?;
                }
                '\t' if skip_tabs != SkipTabs::No => {
                    encountered_tab = true;
                    self.skip()?;
                }
                // YAML comments must be preceded by whitespace.
                '#' if !encountered_tab && !has_yaml_ws => {
                    return Ok((
                        chars_consumed,
                        Err("comments must be separated from other tokens by whitespace"),
                    ));
                }
                '#' => {
                    self.skip()?; // Skip over '#'
                    while !is_breakz(self.look_ch()?) {
                        self.skip()?;
                        chars_consumed += 1;
                    }
                }
                _ => break,
            }
            chars_consumed += 1;
        }

        Ok((
            chars_consumed,
            Ok(SkipTabs::Result(encountered_tab, has_yaml_ws)),
        ))
    }

    /// Check whether the next characters may be part of a plain scalar.
    ///
    /// This function assumes we are not given a blankz character.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_can_be_plain_scalar(&mut self, in_flow: bool) -> InputResult<bool, Self::Error> {
        let nc = self.peek_nth(1)?;
        Ok(match self.peek()? {
            // indicators can end a plain scalar, see 7.3.3. Plain Style
            ':' if is_blank_or_breakz(nc) || (in_flow && is_flow(nc)) => false,
            c if in_flow && is_flow(c) => false,
            _ => true,
        })
    }

    /// Check whether the next character is [a blank] or [a break].
    ///
    /// The character must have previously been fetched through [`lookahead`]
    ///
    /// # Return
    /// Returns true if the character is [a blank] or [a break], false otherwise.
    ///
    /// [`lookahead`]: Input::lookahead
    /// [a blank]: is_blank
    /// [a break]: is_break
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_blank_or_break(&mut self) -> InputResult<bool, Self::Error> {
        Ok(is_blank(self.peek()?) || is_break(self.peek()?))
    }

    /// Check whether the next character is [a blank] or [a breakz].
    ///
    /// The character must have previously been fetched through [`lookahead`]
    ///
    /// # Return
    /// Returns true if the character is [a blank] or [a break], false otherwise.
    ///
    /// [`lookahead`]: Input::lookahead
    /// [a blank]: is_blank
    /// [a breakz]: is_breakz
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_blank_or_breakz(&mut self) -> InputResult<bool, Self::Error> {
        Ok(is_blank(self.peek()?) || is_breakz(self.peek()?))
    }

    /// Check whether the next character is [a blank].
    ///
    /// The character must have previously been fetched through [`lookahead`]
    ///
    /// # Return
    /// Returns true if the character is [a blank], false otherwise.
    ///
    /// [`lookahead`]: Input::lookahead
    /// [a blank]: is_blank
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_blank(&mut self) -> InputResult<bool, Self::Error> {
        Ok(is_blank(self.peek()?))
    }

    /// Check whether the next character is [a break].
    ///
    /// The character must have previously been fetched through [`lookahead`]
    ///
    /// # Return
    /// Returns true if the character is [a break], false otherwise.
    ///
    /// [`lookahead`]: Input::lookahead
    /// [a break]: is_break
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_break(&mut self) -> InputResult<bool, Self::Error> {
        Ok(is_break(self.peek()?))
    }

    /// Check whether the next character is [a breakz].
    ///
    /// The character must have previously been fetched through [`lookahead`]
    ///
    /// # Return
    /// Returns true if the character is [a breakz], false otherwise.
    ///
    /// [`lookahead`]: Input::lookahead
    /// [a breakz]: is_breakz
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_breakz(&mut self) -> InputResult<bool, Self::Error> {
        Ok(is_breakz(self.peek()?))
    }

    /// Check whether the next character is [a z].
    ///
    /// The character must have previously been fetched through [`lookahead`]
    ///
    /// # Return
    /// Returns true if the character is [a z], false otherwise.
    ///
    /// [`lookahead`]: Input::lookahead
    /// [a z]: is_z
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_z(&mut self) -> InputResult<bool, Self::Error> {
        Ok(is_z(self.peek()?))
    }

    /// Check whether the next character is [a flow].
    ///
    /// The character must have previously been fetched through [`lookahead`]
    ///
    /// # Return
    /// Returns true if the character is [a flow], false otherwise.
    ///
    /// [`lookahead`]: Input::lookahead
    /// [a flow]: is_flow
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_flow(&mut self) -> InputResult<bool, Self::Error> {
        Ok(is_flow(self.peek()?))
    }

    /// Check whether the next character is [a digit].
    ///
    /// The character must have previously been fetched through [`lookahead`]
    ///
    /// # Return
    /// Returns true if the character is [a digit], false otherwise.
    ///
    /// [`lookahead`]: Input::lookahead
    /// [a digit]: is_digit
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_digit(&mut self) -> InputResult<bool, Self::Error> {
        Ok(is_digit(self.peek()?))
    }

    /// Check whether the next character is [a letter].
    ///
    /// The character must have previously been fetched through [`lookahead`]
    ///
    /// # Return
    /// Returns true if the character is [a letter], false otherwise.
    ///
    /// [`lookahead`]: Input::lookahead
    /// [a letter]: is_alpha
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn next_is_alpha(&mut self) -> InputResult<bool, Self::Error> {
        Ok(is_alpha(self.peek()?))
    }

    /// Skip characters from the input until a [breakz] is found.
    ///
    /// The characters are consumed from the input.
    ///
    /// # Return
    /// Return the number of characters that were consumed. The number of characters returned can
    /// be used to advance the index and column, since no end-of-line character will be consumed.
    ///
    /// [breakz]: is_breakz
    #[inline]
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn skip_while_non_breakz(&mut self) -> InputResult<usize, Self::Error> {
        let mut count = 0;
        while !is_breakz(self.look_ch()?) {
            count += 1;
            self.skip()?;
        }
        Ok(count)
    }

    /// Skip characters from the input while [blanks] are found.
    ///
    /// The characters are consumed from the input.
    ///
    /// # Return
    /// Return the number of characters that were consumed. The number of characters returned can
    /// be used to advance the index and column, since no end-of-line character will be consumed.
    ///
    /// [blanks]: is_blank
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn skip_while_blank(&mut self) -> InputResult<usize, Self::Error> {
        let mut n_chars = 0;
        while is_blank(self.look_ch()?) {
            n_chars += 1;
            self.skip()?;
        }
        Ok(n_chars)
    }

    /// Fetch characters from the input while we encounter letters and store them in `out`.
    ///
    /// The characters are consumed from the input.
    ///
    /// # Return
    /// Return the number of characters that were consumed. The number of characters returned can
    /// be used to advance the index and column, since no end-of-line character will be consumed.
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn fetch_while_is_alpha(&mut self, out: &mut String) -> InputResult<usize, Self::Error> {
        let mut n_chars = 0;
        while is_alpha(self.look_ch()?) {
            n_chars += 1;
            out.push(self.peek()?);
            self.skip()?;
        }
        Ok(n_chars)
    }

    /// Fetch characters as long as they satisfy `is_yaml_non_space(c)`.
    ///
    /// The characters are consumed from the input.
    ///
    /// # Return
    /// Return the number of characters that were consumed. The number of characters returned can
    /// be used to advance the index and column, since no end-of-line character will be consumed.
    /// # Errors
    /// Returns an input error when the source fails or has already failed.
    fn fetch_while_is_yaml_non_space(
        &mut self,
        out: &mut String,
    ) -> InputResult<usize, Self::Error> {
        let mut n_chars = 0;
        while crate::char_traits::is_yaml_non_space(self.look_ch()?) {
            n_chars += 1;
            out.push(self.peek()?);
            self.skip()?;
        }
        Ok(n_chars)
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
    #[must_use]
    pub fn found_tabs(self) -> bool {
        matches!(self, SkipTabs::Result(true, _))
    }

    /// Whether a valid YAML whitespace has been found in skipped-over content.
    ///
    /// This function must be called after a call to `skip_ws_to_eol`.
    #[must_use]
    pub fn has_valid_yaml_ws(self) -> bool {
        matches!(self, SkipTabs::Result(_, true))
    }
}
