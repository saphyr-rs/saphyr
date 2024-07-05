use crate::{char_traits::is_blank_or_breakz, input::Input};

#[allow(clippy::module_name_repetitions)]
pub struct StrInput<'a> {
    /// The input str buffer.
    buffer: &'a str,
    /// The number of characters (**not** bytes) in the buffer.
    n_chars: usize,
    /// The number of characters we have looked ahead.
    ///
    /// We must however keep track of how many characters the parser asked us to look ahead for so
    /// that we can return the correct value in [`Self::buflen`].
    lookahead: usize,
}

impl<'a> StrInput<'a> {
    /// Create a new [`StrInput`] with the given str.
    pub fn new(input: &'a str) -> Self {
        Self {
            buffer: input,
            n_chars: input.chars().count(),
            lookahead: 0,
        }
    }
}

impl<'a> Input for StrInput<'a> {
    #[inline]
    fn lookahead(&mut self, x: usize) {
        // We already have all characters that we need.
        // We cannot add '\0's to the buffer should we prematurely reach EOF.
        // Returning '\0's befalls the character-retrieving functions.
        self.lookahead = self.lookahead.max(x);
    }

    #[inline]
    fn buflen(&self) -> usize {
        self.lookahead
    }

    #[inline]
    fn bufmaxlen(&self) -> usize {
        BUFFER_LEN
    }

    fn buf_is_empty(&self) -> bool {
        self.buflen() == 0
    }

    #[inline]
    fn raw_read_ch(&mut self) -> char {
        let mut chars = self.buffer.chars();
        if let Some(c) = chars.next() {
            self.buffer = chars.as_str();
            self.n_chars -= 1;
            c
        } else {
            '\0'
        }
    }

    #[inline]
    fn push_back(&mut self, c: char) {
        let n_bytes = c.len_utf8();

        // SAFETY: The character that gets pushed back is guaranteed to be the one that is
        // immediately preceding our buffer. We can compute the length of the character and move
        // our buffer back that many bytes.
        unsafe {
            let buffer_byte_len = self.buffer.len();
            let mut now_ptr = self.buffer.as_ptr();
            now_ptr = now_ptr.wrapping_sub(n_bytes);
            self.buffer = std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                now_ptr,
                buffer_byte_len + n_bytes,
            ));
        }
    }

    #[inline]
    fn skip(&mut self) {
        let mut chars = self.buffer.chars();
        if chars.next().is_some() {
            self.buffer = chars.as_str();
            self.n_chars -= 1;
        }
    }

    #[inline]
    fn skip_n(&mut self, count: usize) {
        let mut chars = self.buffer.chars();
        for _ in 0..count {
            if chars.next().is_none() {
                break;
            }
        }
        self.buffer = chars.as_str();
        self.n_chars = self.n_chars.saturating_sub(count);
    }

    #[inline]
    fn peek(&self) -> char {
        self.buffer.chars().next().unwrap_or('\0')
    }

    #[inline]
    fn peek_nth(&self, n: usize) -> char {
        let mut chars = self.buffer.chars();
        for _ in 0..n {
            if chars.next().is_none() {
                return '\0';
            }
        }
        chars.next().unwrap_or('\0')
    }

    #[inline]
    fn look_ch(&mut self) -> char {
        self.lookahead(1);
        self.peek()
    }

    #[inline]
    fn next_char_is(&self, c: char) -> bool {
        self.peek() == c
    }

    #[inline]
    fn nth_char_is(&self, n: usize, c: char) -> bool {
        self.peek_nth(n) == c
    }

    #[inline]
    fn next_2_are(&self, c1: char, c2: char) -> bool {
        let mut chars = self.buffer.chars();
        chars.next().is_some_and(|c| c == c1) && chars.next().is_some_and(|c| c == c2)
    }

    #[inline]
    fn next_3_are(&self, c1: char, c2: char, c3: char) -> bool {
        let mut chars = self.buffer.chars();
        chars.next().is_some_and(|c| c == c1)
            && chars.next().is_some_and(|c| c == c2)
            && chars.next().is_some_and(|c| c == c3)
    }

    #[inline]
    fn next_is_document_indicator(&self) -> bool {
        if self.buffer.len() < 3 {
            false
        } else {
            // Since all characters we look for are ascii, we can directly use the byte API of str.
            (if self.buffer.len() == 3 {
                true
            } else {
                is_blank_or_breakz(self.buffer.as_bytes()[3] as char)
            }) && (self.buffer.starts_with("...") || self.buffer.starts_with("---"))
        }
    }

    #[inline]
    fn next_is_document_start(&self) -> bool {
        if self.buffer.len() < 3 {
            false
        } else {
            // Since all characters we look for are ascii, we can directly use the byte API of str.
            (if self.buffer.len() == 3 {
                true
            } else {
                is_blank_or_breakz(self.buffer.as_bytes()[3] as char)
            }) && self.buffer.starts_with("---")
        }
    }

    #[inline]
    fn next_is_document_end(&self) -> bool {
        if self.buffer.len() < 3 {
            false
        } else {
            // Since all characters we look for are ascii, we can directly use the byte API of str.
            (if self.buffer.len() == 3 {
                true
            } else {
                is_blank_or_breakz(self.buffer.as_bytes()[3] as char)
            }) && self.buffer.starts_with("...")
        }
    }
}

/// The buffer size we return to the scanner.
///
/// This does not correspond to any allocated buffer size. In practice, the scanner can withdraw
/// any character they want. If it's within the input buffer, the given character is returned,
/// otherwise `\0` is returned.
///
/// The number of characters we are asked to retrieve in [`lookahead`] depends on the buffer size
/// of the input. Our buffer here is virtually unlimited, but the scanner cannot work with that. It
/// may allocate buffers of its own of the size we return in [`bufmaxlen`] (so we can't return
/// [`usize::MAX`]). We can't always return the number of characters left either, as the scanner
/// expects [`buflen`] to return the same value that was given to [`lookahead`] right after its
/// call.
///
/// This create a complex situation where [`bufmaxlen`] influences what value [`lookahead`] is
/// called with, which in turns dictates what [`buflen`] returns. In order to avoid breaking any
/// function, we return this constant in [`bufmaxlen`] which, since the input is processed one line
/// at a time, should fit what we expect to be a good balance between memory consumption and what
/// we expect the maximum line length to be.
///
/// [`lookahead`]: `StrInput::lookahead`
/// [`bufmaxlen`]: `StrInput::bufmaxlen`
/// [`buflen`]: `StrInput::buflen`
const BUFFER_LEN: usize = 128;

#[cfg(test)]
mod test {
    use crate::input::Input;

    use super::StrInput;

    #[test]
    pub fn is_document_start() {
        let input = StrInput::new("---\n");
        assert!(input.next_is_document_start());
        assert!(input.next_is_document_indicator());
        let input = StrInput::new("---");
        assert!(input.next_is_document_start());
        assert!(input.next_is_document_indicator());
        let input = StrInput::new("...\n");
        assert!(!input.next_is_document_start());
        assert!(input.next_is_document_indicator());
        let input = StrInput::new("--- ");
        assert!(input.next_is_document_start());
        assert!(input.next_is_document_indicator());
    }

    #[test]
    pub fn is_document_end() {
        let input = StrInput::new("...\n");
        assert!(input.next_is_document_end());
        assert!(input.next_is_document_indicator());
        let input = StrInput::new("...");
        assert!(input.next_is_document_end());
        assert!(input.next_is_document_indicator());
        let input = StrInput::new("---\n");
        assert!(!input.next_is_document_end());
        assert!(input.next_is_document_indicator());
        let input = StrInput::new("... ");
        assert!(input.next_is_document_end());
        assert!(input.next_is_document_indicator());
    }
}
