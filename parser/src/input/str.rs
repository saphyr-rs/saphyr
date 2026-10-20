use crate::{
    char_traits::{
        is_alpha, is_blank, is_blank_or_breakz, is_break, is_breakz, is_digit, is_flow, is_z,
    },
    input::{Input, InputResult, SkipTabs},
};
use alloc::string::String;
use core::convert::Infallible;

/// A parser input that uses a `&str` as source.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Copy)]
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
    type Error = Infallible;

    #[inline]
    fn lookahead(&mut self, x: usize) -> InputResult<(), Self::Error> {
        self.lookahead = self.lookahead.max(x);
        Ok(())
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
    fn raw_read_ch(&mut self) -> InputResult<char, Self::Error> {
        let mut chars = self.buffer.chars();
        let c = chars.next().unwrap_or('\0');
        if chars.as_str().len() < self.buffer.len() {
            self.buffer = chars.as_str();
        }
        Ok(c)
    }

    #[inline]
    fn raw_read_non_breakz_ch(&mut self) -> InputResult<Option<char>, Self::Error> {
        Ok(match split_first_char(self.buffer) {
            Some((c, rest)) if !is_breakz(c) => {
                self.buffer = rest;
                Some(c)
            }
            _ => None,
        })
    }

    #[inline]
    fn skip(&mut self) -> InputResult<(), Self::Error> {
        let mut chars = self.buffer.chars();
        if chars.next().is_some() {
            self.buffer = chars.as_str();
        }
        Ok(())
    }

    #[inline]
    fn skip_n(&mut self, count: usize) -> InputResult<(), Self::Error> {
        let mut chars = self.buffer.chars();
        for _ in 0..count {
            if chars.next().is_none() {
                break;
            }
        }
        self.buffer = chars.as_str();
        Ok(())
    }

    #[inline]
    fn peek(&mut self) -> InputResult<char, Self::Error> {
        Ok(self.buffer.chars().next().unwrap_or('\0'))
    }

    #[inline]
    fn peek_nth(&mut self, n: usize) -> InputResult<char, Self::Error> {
        Ok(self.buffer.chars().nth(n).unwrap_or('\0'))
    }

    #[inline]
    fn peek_opt(&mut self) -> InputResult<Option<char>, Self::Error> {
        Ok(self.buffer.chars().next())
    }

    #[inline]
    fn next_2_are(&mut self, c1: char, c2: char) -> InputResult<bool, Self::Error> {
        let mut chars = self.buffer.chars();
        Ok(chars.next() == Some(c1) && chars.next() == Some(c2))
    }

    #[inline]
    fn next_3_are(&mut self, c1: char, c2: char, c3: char) -> InputResult<bool, Self::Error> {
        let mut chars = self.buffer.chars();
        Ok(chars.next() == Some(c1) && chars.next() == Some(c2) && chars.next() == Some(c3))
    }

    #[inline]
    fn next_is_document_indicator(&mut self) -> InputResult<bool, Self::Error> {
        if self.buffer.len() < 3 {
            return Ok(false);
        }

        let bytes = self.buffer.as_bytes();
        Ok((bytes.len() == 3 || is_blank_or_breakz(bytes[3] as char))
            && (bytes[0] == b'.' || bytes[0] == b'-')
            && bytes[0] == bytes[1]
            && bytes[1] == bytes[2])
    }

    #[inline]
    fn next_is_document_start(&mut self) -> InputResult<bool, Self::Error> {
        if self.buffer.len() < 3 {
            return Ok(false);
        }

        let bytes = self.buffer.as_bytes();
        Ok((bytes.len() == 3 || is_blank_or_breakz(bytes[3] as char))
            && bytes[0] == b'-'
            && bytes[1] == b'-'
            && bytes[2] == b'-')
    }

    #[inline]
    fn next_is_document_end(&mut self) -> InputResult<bool, Self::Error> {
        if self.buffer.len() < 3 {
            return Ok(false);
        }

        let bytes = self.buffer.as_bytes();
        Ok((bytes.len() == 3 || is_blank_or_breakz(bytes[3] as char))
            && bytes[0] == b'.'
            && bytes[1] == b'.'
            && bytes[2] == b'.')
    }

    fn skip_ws_to_eol(
        &mut self,
        skip_tabs: SkipTabs,
    ) -> InputResult<(usize, Result<SkipTabs, &'static str>), Self::Error> {
        assert!(!matches!(skip_tabs, SkipTabs::Result(..)));

        let mut remaining = self.buffer;
        let mut has_yaml_ws = false;
        let mut encountered_tab = false;
        if skip_tabs == SkipTabs::Yes {
            loop {
                if let Some(rest) = remaining.strip_prefix(' ') {
                    has_yaml_ws = true;
                    remaining = rest;
                } else if let Some(rest) = remaining.strip_prefix('\t') {
                    encountered_tab = true;
                    remaining = rest;
                } else {
                    break;
                }
            }
        } else {
            while let Some(rest) = remaining.strip_prefix(' ') {
                has_yaml_ws = true;
                remaining = rest;
            }
        }

        let mut consumed = self.buffer.len() - remaining.len();
        if remaining.as_bytes().first() == Some(&b'#') {
            if !encountered_tab && !has_yaml_ws {
                return Ok((
                    consumed,
                    Err("comments must be separated from other tokens by whitespace"),
                ));
            }

            while let Some((c, rest)) = split_first_char(remaining) {
                if is_breakz(c) {
                    break;
                }
                remaining = rest;
                consumed += 1;
            }
        }

        self.buffer = remaining;
        Ok((consumed, Ok(SkipTabs::Result(encountered_tab, has_yaml_ws))))
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn next_can_be_plain_scalar(&mut self, in_flow: bool) -> InputResult<bool, Self::Error> {
        let next = self.peek_nth(1)?;
        Ok(match self.peek()? {
            ':' if is_blank_or_breakz(next) || (in_flow && is_flow(next)) => false,
            c if in_flow && is_flow(c) => false,
            _ => true,
        })
    }

    #[inline]
    fn next_is_blank_or_break(&mut self) -> InputResult<bool, Self::Error> {
        Ok(self
            .buffer
            .as_bytes()
            .first()
            .is_some_and(|byte| is_blank(*byte as char) || is_break(*byte as char)))
    }

    #[inline]
    fn next_is_blank_or_breakz(&mut self) -> InputResult<bool, Self::Error> {
        Ok(self
            .buffer
            .as_bytes()
            .first()
            .is_none_or(|byte| is_blank(*byte as char) || is_breakz(*byte as char)))
    }

    #[inline]
    fn next_is_blank(&mut self) -> InputResult<bool, Self::Error> {
        Ok(self
            .buffer
            .as_bytes()
            .first()
            .is_some_and(|byte| is_blank(*byte as char)))
    }

    #[inline]
    fn next_is_break(&mut self) -> InputResult<bool, Self::Error> {
        Ok(self
            .buffer
            .as_bytes()
            .first()
            .is_some_and(|byte| is_break(*byte as char)))
    }

    #[inline]
    fn next_is_breakz(&mut self) -> InputResult<bool, Self::Error> {
        Ok(self
            .buffer
            .as_bytes()
            .first()
            .is_none_or(|byte| is_breakz(*byte as char)))
    }

    #[inline]
    fn next_is_z(&mut self) -> InputResult<bool, Self::Error> {
        Ok(self
            .buffer
            .as_bytes()
            .first()
            .is_none_or(|byte| is_z(*byte as char)))
    }

    #[inline]
    fn next_is_flow(&mut self) -> InputResult<bool, Self::Error> {
        Ok(self
            .buffer
            .as_bytes()
            .first()
            .is_some_and(|byte| is_flow(*byte as char)))
    }

    #[inline]
    fn next_is_digit(&mut self) -> InputResult<bool, Self::Error> {
        Ok(self
            .buffer
            .as_bytes()
            .first()
            .is_some_and(|byte| is_digit(*byte as char)))
    }

    #[inline]
    fn next_is_alpha(&mut self) -> InputResult<bool, Self::Error> {
        Ok(self
            .buffer
            .as_bytes()
            .first()
            .is_some_and(|byte| is_alpha(*byte as char)))
    }

    fn skip_while_non_breakz(&mut self) -> InputResult<usize, Self::Error> {
        let mut remaining = self.buffer;
        let mut count = 0;
        while let Some((c, rest)) = split_first_char(remaining) {
            if is_breakz(c) {
                break;
            }
            remaining = rest;
            count += 1;
        }
        self.buffer = remaining;
        Ok(count)
    }

    fn skip_while_blank(&mut self) -> InputResult<usize, Self::Error> {
        let mut bytes = 0;
        while bytes < self.buffer.len() && is_blank(self.buffer.as_bytes()[bytes] as char) {
            bytes += 1;
        }
        self.buffer = &self.buffer[bytes..];
        Ok(bytes)
    }

    fn fetch_while_is_alpha(&mut self, out: &mut String) -> InputResult<usize, Self::Error> {
        let mut chars = self.buffer.chars();
        for c in chars.by_ref() {
            if !is_alpha(c) {
                let bytes = self.buffer.len() - chars.as_str().len() - c.len_utf8();
                out.reserve(bytes);
                out.push_str(&self.buffer[..bytes]);
                self.buffer = &self.buffer[bytes..];
                return Ok(bytes);
            }
        }

        let bytes = self.buffer.len() - chars.as_str().len();
        out.reserve(bytes);
        out.push_str(&self.buffer[..bytes]);
        self.buffer = chars.as_str();
        Ok(bytes)
    }

    fn fetch_while_is_yaml_non_space(
        &mut self,
        out: &mut String,
    ) -> InputResult<usize, Self::Error> {
        let mut chars = self.buffer.chars();
        for c in chars.by_ref() {
            if !crate::char_traits::is_yaml_non_space(c) {
                let bytes = self.buffer.len() - chars.as_str().len() - c.len_utf8();
                out.reserve(bytes);
                out.push_str(&self.buffer[..bytes]);
                self.buffer = &self.buffer[bytes..];
                return Ok(bytes);
            }
        }

        let bytes = self.buffer.len() - chars.as_str().len();
        out.reserve(bytes);
        out.push_str(&self.buffer[..bytes]);
        self.buffer = chars.as_str();
        Ok(bytes)
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
        let mut input = StrInput::new("---\n");
        input.lookahead(4).unwrap();
        assert!(input.next_is_document_start().unwrap());
        assert!(input.next_is_document_indicator().unwrap());
        let mut input = StrInput::new("---");
        input.lookahead(4).unwrap();
        assert!(input.next_is_document_start().unwrap());
        assert!(input.next_is_document_indicator().unwrap());
        let mut input = StrInput::new("...\n");
        input.lookahead(4).unwrap();
        assert!(!input.next_is_document_start().unwrap());
        assert!(input.next_is_document_indicator().unwrap());
        let mut input = StrInput::new("--- ");
        input.lookahead(4).unwrap();
        assert!(input.next_is_document_start().unwrap());
        assert!(input.next_is_document_indicator().unwrap());
    }

    #[test]
    pub fn is_document_end() {
        let mut input = StrInput::new("...\n");
        input.lookahead(4).unwrap();
        assert!(input.next_is_document_end().unwrap());
        assert!(input.next_is_document_indicator().unwrap());
        let mut input = StrInput::new("...");
        input.lookahead(4).unwrap();
        assert!(input.next_is_document_end().unwrap());
        assert!(input.next_is_document_indicator().unwrap());
        let mut input = StrInput::new("---\n");
        input.lookahead(4).unwrap();
        assert!(!input.next_is_document_end().unwrap());
        assert!(input.next_is_document_indicator().unwrap());
        let mut input = StrInput::new("... ");
        input.lookahead(4).unwrap();
        assert!(input.next_is_document_end().unwrap());
        assert!(input.next_is_document_indicator().unwrap());
    }
}
