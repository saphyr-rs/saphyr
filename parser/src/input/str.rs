use crate::{
    char_traits::{is_blank_or_breakz, is_breakz, is_flow},
    input::{Input, SkipTabs},
};

#[allow(clippy::module_name_repetitions)]
pub struct StrInput<'a> {
    /// The input str buffer.
    buffer: &'a str,
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
            c
        } else {
            '\0'
        }
    }

    #[inline]
    fn push_back(&mut self, c: char) {
        self.buffer = put_back_in_str(self.buffer, c);
    }

    #[inline]
    fn skip(&mut self) {
        let mut chars = self.buffer.chars();
        if chars.next().is_some() {
            self.buffer = chars.as_str();
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
            let bytes = self.buffer.as_bytes();
            (bytes.len() == 3 || is_blank_or_breakz(bytes[3] as char))
                && (bytes[0] == b'.' || bytes[0] == b'-')
                && bytes[0] == bytes[1]
                && bytes[1] == bytes[2]
        }
    }

    #[inline]
    fn next_is_document_start(&self) -> bool {
        if self.buffer.len() < 3 {
            false
        } else {
            // Since all characters we look for are ascii, we can directly use the byte API of str.
            let bytes = self.buffer.as_bytes();
            (bytes.len() == 3 || is_blank_or_breakz(bytes[3] as char))
                && bytes[0] == b'-'
                && bytes[1] == b'-'
                && bytes[2] == b'-'
        }
    }

    #[inline]
    fn next_is_document_end(&self) -> bool {
        if self.buffer.len() < 3 {
            false
        } else {
            // Since all characters we look for are ascii, we can directly use the byte API of str.
            let bytes = self.buffer.as_bytes();
            (bytes.len() == 3 || is_blank_or_breakz(bytes[3] as char))
                && bytes[0] == b'.'
                && bytes[1] == b'.'
                && bytes[2] == b'.'
        }
    }

    fn skip_ws_to_eol(&mut self, skip_tabs: SkipTabs) -> (usize, Result<SkipTabs, &'static str>) {
        assert!(!matches!(skip_tabs, SkipTabs::Result(..)));

        let mut new_str = self.buffer.as_bytes();
        let mut has_yaml_ws = false;
        let mut encountered_tab = false;

        // This ugly pair of loops is the fastest way of trimming spaces (and maybe tabs) I found
        // while keeping track of whether we encountered spaces and/or tabs.
        if skip_tabs == SkipTabs::Yes {
            let mut i = 0;
            while i < new_str.len() {
                if new_str[i] == b' ' {
                    has_yaml_ws = true;
                } else if new_str[i] == b'\t' {
                    encountered_tab = true;
                } else {
                    break;
                }
                i += 1;
            }
            new_str = &new_str[i..];
        } else {
            let mut i = 0;
            while i < new_str.len() {
                if new_str[i] != b' ' {
                    break;
                }
                i += 1;
            }
            has_yaml_ws = i != 0;
            new_str = &new_str[i..];
        }

        // All characters consumed were ascii. We can use the byte length difference to count the
        // number of whitespace ignored.
        let mut chars_consumed = self.buffer.len() - new_str.len();
        // SAFETY: We only trimmed spaces and tabs, both of which are bytes. This means we won't
        // start the string outside of a valid UTF-8 boundary.
        // It is assumed the input string is valid UTF-8, so the rest of the string is assumed to
        // be valid UTF-8 as well.
        let mut new_str = unsafe { std::str::from_utf8_unchecked(new_str) };

        if !new_str.is_empty() && new_str.as_bytes()[0] == b'#' {
            if !encountered_tab && !has_yaml_ws {
                return (
                    chars_consumed,
                    Err("comments must be separated from other tokens by whitespace"),
                );
            }

            let mut chars = new_str.chars();
            let mut found_breakz = false;
            // Iterate over all remaining chars until we hit a breakz.
            for c in chars.by_ref() {
                if is_breakz(c) {
                    found_breakz = true;
                    break;
                }
                chars_consumed += 1;
            }

            new_str = if found_breakz {
                // SAFETY: The last character we pulled out of the `chars()` is a breakz, one of
                // '\0', '\r', '\n'. All 3 of them are 1-byte long.
                unsafe { extend_left(chars.as_str(), 1) }
            } else {
                chars.as_str()
            };
        }

        self.buffer = new_str;

        (
            chars_consumed,
            Ok(SkipTabs::Result(encountered_tab, has_yaml_ws)),
        )
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn next_can_be_plain_scalar(&self, in_flow: bool) -> bool {
        let c = self.buffer.as_bytes()[0];
        if self.buffer.len() > 1 {
            let nc = self.buffer.as_bytes()[1];
            match c {
                // indicators can end a plain scalar, see 7.3.3. Plain Style
                b':' if is_blank_or_breakz(nc as char) || (in_flow && is_flow(nc as char)) => false,
                c if in_flow && is_flow(c as char) => false,
                _ => true,
            }
        } else {
            match c {
                // indicators can end a plain scalar, see 7.3.3. Plain Style
                b':' => false,
                c if in_flow && is_flow(c as char) => false,
                _ => true,
            }
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

/// Fake prepending a character to the given string.
///
/// The character given as parameter MUST be the one that precedes the given string.
///
/// # Exmaple
/// ```ignore
/// let s1 = "foo";
/// let s2 = &s1[1..];
/// let s3 = put_back_in_str(s2, 'f'); // OK, 'f' is the character immediately preceding
/// // let s3 = put_back_in_str('g'); // Not allowed
/// assert_eq!(s1, s3);
/// assert_eq!(s1.as_ptr(), s3.as_ptr());
/// ```
fn put_back_in_str(s: &str, c: char) -> &str {
    let n_bytes = c.len_utf8();

    // SAFETY: The character that gets pushed back is guaranteed to be the one that is
    // immediately preceding our buffer. We can compute the length of the character and move
    // our buffer back that many bytes.
    unsafe { extend_left(s, n_bytes) }
}

/// Extend the string by moving the start pointer to the left by `n` bytes.
#[inline]
unsafe fn extend_left(s: &str, n: usize) -> &str {
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(
        s.as_ptr().wrapping_sub(n),
        s.len() + n,
    ))
}

#[cfg(test)]
mod test {
    use crate::input::{str::put_back_in_str, Input};

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

    #[test]
    pub fn put_back_in_str_example() {
        let s1 = "foo";
        let s2 = &s1[1..];
        let s3 = put_back_in_str(s2, 'f'); // OK, 'f' is the character immediately preceding
        assert_eq!(s1, s3);
        assert_eq!(s1.as_ptr(), s3.as_ptr());
    }
}
