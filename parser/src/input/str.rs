use crate::{
    char_traits::{
        is_alpha, is_blank, is_blank_or_breakz, is_break, is_breakz, is_digit, is_flow, is_z,
    },
    input::{Input, SkipTabs},
};

/// A parser input that uses a `&str` as source.
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
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            buffer: input,
            lookahead: 0,
        }
    }
}

impl Input for StrInput<'_> {
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
    fn raw_read_non_breakz_ch(&mut self) -> Option<char> {
        if let Some((c, sub_str)) = split_first_char(self.buffer) {
            if is_breakz(c) {
                None
            } else {
                self.buffer = sub_str;
                Some(c)
            }
        } else {
            None
        }
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
        chars.next() == Some(c1) && chars.next() == Some(c2)
    }

    #[inline]
    fn next_3_are(&self, c1: char, c2: char, c3: char) -> bool {
        let mut chars = self.buffer.chars();
        chars.next() == Some(c1) && chars.next() == Some(c2) && chars.next() == Some(c3)
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

        let mut new_str = self.buffer;
        let mut has_yaml_ws = false;
        let mut encountered_tab = false;

        // This ugly pair of loops is the fastest way of trimming spaces (and maybe tabs) I found
        // while keeping track of whether we encountered spaces and/or tabs.
        if skip_tabs == SkipTabs::Yes {
            loop {
                if let Some(sub_str) = new_str.strip_prefix(' ') {
                    has_yaml_ws = true;
                    new_str = sub_str;
                } else if let Some(sub_str) = new_str.strip_prefix('\t') {
                    encountered_tab = true;
                    new_str = sub_str;
                } else {
                    break;
                }
            }
        } else {
            while let Some(sub_str) = new_str.strip_prefix(' ') {
                has_yaml_ws = true;
                new_str = sub_str;
            }
        }

        // All characters consumed were ascii. We can use the byte length difference to count the
        // number of whitespace ignored.
        let mut chars_consumed = self.buffer.len() - new_str.len();

        if !new_str.is_empty() && new_str.as_bytes()[0] == b'#' {
            if !encountered_tab && !has_yaml_ws {
                return (
                    chars_consumed,
                    Err("comments must be separated from other tokens by whitespace"),
                );
            }

            // Skip remaining characters until we hit a breakz.
            while let Some((c, sub_str)) = split_first_char(new_str) {
                if is_breakz(c) {
                    break;
                }
                new_str = sub_str;
                chars_consumed += 1;
            }
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

    #[inline]
    fn next_is_blank_or_break(&self) -> bool {
        !self.buffer.is_empty()
            && (is_blank(self.buffer.as_bytes()[0] as char)
                || is_break(self.buffer.as_bytes()[0] as char))
    }

    #[inline]
    fn next_is_blank_or_breakz(&self) -> bool {
        self.buffer.is_empty()
            || (is_blank(self.buffer.as_bytes()[0] as char)
                || is_breakz(self.buffer.as_bytes()[0] as char))
    }

    #[inline]
    fn next_is_blank(&self) -> bool {
        !self.buffer.is_empty() && is_blank(self.buffer.as_bytes()[0] as char)
    }

    #[inline]
    fn next_is_break(&self) -> bool {
        !self.buffer.is_empty() && is_break(self.buffer.as_bytes()[0] as char)
    }

    #[inline]
    fn next_is_breakz(&self) -> bool {
        self.buffer.is_empty() || is_breakz(self.buffer.as_bytes()[0] as char)
    }

    #[inline]
    fn next_is_z(&self) -> bool {
        self.buffer.is_empty() || is_z(self.buffer.as_bytes()[0] as char)
    }

    #[inline]
    fn next_is_flow(&self) -> bool {
        !self.buffer.is_empty() && is_flow(self.buffer.as_bytes()[0] as char)
    }

    #[inline]
    fn next_is_digit(&self) -> bool {
        !self.buffer.is_empty() && is_digit(self.buffer.as_bytes()[0] as char)
    }

    #[inline]
    fn next_is_alpha(&self) -> bool {
        !self.buffer.is_empty() && is_alpha(self.buffer.as_bytes()[0] as char)
    }

    fn skip_while_non_breakz(&mut self) -> usize {
        let mut new_str = self.buffer;
        let mut count = 0;

        // Skip over all non-breaks.
        while let Some((c, sub_str)) = split_first_char(new_str) {
            if is_breakz(c) {
                break;
            }
            new_str = sub_str;
            count += 1;
        }

        self.buffer = new_str;

        count
    }

    fn skip_while_blank(&mut self) -> usize {
        // Since all characters we look for are ascii, we can directly use the byte API of str.
        let mut i = 0;
        while i < self.buffer.len() {
            if !is_blank(self.buffer.as_bytes()[i] as char) {
                break;
            }
            i += 1;
        }
        self.buffer = &self.buffer[i..];
        i
    }

    fn fetch_while_is_alpha(&mut self, out: &mut String) -> usize {
        let mut not_alpha = None;

        // Skip while we have alpha characters.
        let mut chars = self.buffer.chars();
        for c in chars.by_ref() {
            if !is_alpha(c) {
                not_alpha = Some(c);
                break;
            }
        }

        let remaining_string = if let Some(c) = not_alpha {
            let n_bytes_read = chars.as_str().as_ptr() as usize - self.buffer.as_ptr() as usize;
            let last_char_bytes = c.len_utf8();
            &self.buffer[n_bytes_read - last_char_bytes..]
        } else {
            chars.as_str()
        };

        let n_bytes_to_append = remaining_string.as_ptr() as usize - self.buffer.as_ptr() as usize;
        out.reserve(n_bytes_to_append);
        out.push_str(&self.buffer[..n_bytes_to_append]);
        self.buffer = remaining_string;

        n_bytes_to_append
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

/// Splits the first character of the given string and returns it along with the rest of the
/// string.
#[inline]
fn split_first_char(s: &str) -> Option<(char, &str)> {
    let mut chars = s.chars();
    let c = chars.next()?;
    Some((c, chars.as_str()))
}

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
