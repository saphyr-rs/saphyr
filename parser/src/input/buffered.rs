use crate::char_traits::is_breakz;
use crate::input::{Input, InputError, InputResult};

use arraydeque::ArrayDeque;

const BUFFER_LEN: usize = 16;

/// An infallible character iterator adapter.
#[derive(Clone)]
pub struct InfallibleChars<T>(T);

impl<T: Iterator<Item = char>> InfallibleChars<T> {
    pub(crate) fn new(input: T) -> Self {
        Self(input)
    }
}

impl<T: Iterator<Item = char>> Iterator for InfallibleChars<T> {
    type Item = Result<char, core::convert::Infallible>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(Ok)
    }
}

/// A wrapper around an iterator with a bounded lookahead buffer.
#[allow(clippy::module_name_repetitions)]
pub struct BufferedInput<T: Iterator<Item = Result<char, E>>, E> {
    input: T,
    buffer: ArrayDeque<char, BUFFER_LEN>,
    padding: usize,
    failed: bool,
    finished: bool,
}

impl<T, E> Clone for BufferedInput<T, E>
where
    T: Iterator<Item = Result<char, E>> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            input: self.input.clone(),
            buffer: self.buffer.clone(),
            padding: self.padding,
            failed: self.failed,
            finished: self.finished,
        }
    }
}

impl<T: Iterator<Item = Result<char, E>>, E> BufferedInput<T, E> {
    /// Create a buffered input from a fallible character iterator.
    pub fn new(input: T) -> Self {
        Self {
            input,
            buffer: ArrayDeque::default(),
            padding: 0,
            failed: false,
            finished: false,
        }
    }

    fn read_next(&mut self) -> InputResult<Option<char>, E> {
        if self.failed {
            return Err(InputError::Failed);
        }
        if self.finished {
            return Ok(None);
        }
        if let Some(item) = self.input.next() {
            match item {
                Ok(c) => Ok(Some(c)),
                Err(error) => {
                    self.failed = true;
                    Err(InputError::Source(error))
                }
            }
        } else {
            self.finished = true;
            Ok(None)
        }
    }
}

impl<T: Iterator<Item = Result<char, E>>, E> Input for BufferedInput<T, E> {
    type Error = E;

    #[inline]
    fn lookahead(&mut self, count: usize) -> InputResult<(), Self::Error> {
        if self.failed {
            return Err(InputError::Failed);
        }
        if self.buffer.len() >= count {
            return Ok(());
        }
        for _ in 0..(count - self.buffer.len()) {
            if let Some(c) = self.read_next()? {
                self.buffer.push_back(c).unwrap();
            } else {
                self.padding += 1;
                self.buffer.push_back('\0').unwrap();
            }
        }
        Ok(())
    }

    #[inline]
    fn buflen(&self) -> usize {
        self.buffer.len()
    }

    #[inline]
    fn bufmaxlen(&self) -> usize {
        BUFFER_LEN
    }

    #[inline]
    fn raw_read_ch(&mut self) -> InputResult<char, Self::Error> {
        Ok(self.read_next()?.unwrap_or('\0'))
    }

    #[inline]
    fn raw_read_non_breakz_ch(&mut self) -> InputResult<Option<char>, Self::Error> {
        if let Some(c) = self.read_next()? {
            if is_breakz(c) {
                self.buffer.push_back(c).unwrap();
                Ok(None)
            } else {
                Ok(Some(c))
            }
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn skip(&mut self) -> InputResult<(), Self::Error> {
        if self.failed {
            return Err(InputError::Failed);
        }
        if self.buffer.len() == self.padding {
            self.padding = self.padding.saturating_sub(1);
        }
        self.buffer.pop_front();
        Ok(())
    }

    #[inline]
    fn skip_n(&mut self, count: usize) -> InputResult<(), Self::Error> {
        if self.failed {
            return Err(InputError::Failed);
        }
        self.buffer.drain(0..count);
        if self.buffer.len() < self.padding {
            self.padding = self.buffer.len();
        }
        Ok(())
    }

    #[inline]
    fn peek(&mut self) -> InputResult<char, Self::Error> {
        if self.failed {
            Err(InputError::Failed)
        } else {
            Ok(self.buffer[0])
        }
    }

    #[inline]
    fn peek_nth(&mut self, n: usize) -> InputResult<char, Self::Error> {
        if self.failed {
            Err(InputError::Failed)
        } else {
            Ok(self.buffer[n])
        }
    }

    #[inline]
    fn peek_opt(&mut self) -> InputResult<Option<char>, Self::Error> {
        if self.failed {
            Err(InputError::Failed)
        } else if self.buffer.len() > self.padding {
            Ok(Some(self.buffer[0]))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BufferedInput;
    use crate::input::{Input, InputError};
    use alloc::rc::Rc;
    use core::cell::Cell;

    struct StopsAfterEof {
        state: u8,
        polls: Rc<Cell<usize>>,
    }

    impl Iterator for StopsAfterEof {
        type Item = Result<char, &'static str>;

        fn next(&mut self) -> Option<Self::Item> {
            self.polls.set(self.polls.get() + 1);
            match self.state {
                0 => {
                    self.state = 1;
                    Some(Ok('x'))
                }
                1 => {
                    self.state = 2;
                    None
                }
                _ => Some(Err("polled after EOF")),
            }
        }
    }

    #[test]
    fn eof_is_terminal_for_buffered_input() {
        let polls = Rc::new(Cell::new(0));
        let source = StopsAfterEof {
            state: 0,
            polls: Rc::clone(&polls),
        };
        let mut input = BufferedInput::new(source);

        input.lookahead(4).unwrap();
        assert_eq!(input.peek_opt().unwrap(), Some('x'));
        input.skip().unwrap();
        assert_eq!(input.peek_opt().unwrap(), None);
        input.lookahead(1).unwrap();
        assert_eq!(input.peek_opt().unwrap(), None);
        assert_eq!(polls.get(), 2);
        assert!(matches!(input.raw_read_ch(), Ok('\0')));
        assert_eq!(polls.get(), 2);
        assert!(matches!(input.peek(), Ok('\0')));
    }

    #[test]
    fn source_error_is_reported_once() {
        let mut input = BufferedInput::new([Ok('x'), Err("source")].into_iter());
        input.lookahead(1).unwrap();
        assert!(matches!(
            input.lookahead(2),
            Err(InputError::Source("source"))
        ));
        assert!(matches!(input.lookahead(2), Err(InputError::Failed)));
    }

    #[test]
    fn nul_and_replacement_character_are_source_data() {
        let mut input = BufferedInput::new(
            [Ok::<char, core::convert::Infallible>('\0'), Ok('\u{fffd}')].into_iter(),
        );
        input.lookahead(2).unwrap();
        assert_eq!(input.peek_opt().unwrap(), Some('\0'));
        input.skip().unwrap();
        assert_eq!(input.peek_opt().unwrap(), Some('\u{fffd}'));
        input.skip().unwrap();
        input.lookahead(1).unwrap();
        assert_eq!(input.peek_opt().unwrap(), None);
    }

    #[test]
    fn raw_read_propagates_the_original_source_error() {
        let mut input = BufferedInput::new([Err("raw-read")].into_iter());
        assert!(matches!(
            input.raw_read_ch(),
            Err(InputError::Source("raw-read"))
        ));
        assert!(matches!(input.raw_read_ch(), Err(InputError::Failed)));
    }
}
