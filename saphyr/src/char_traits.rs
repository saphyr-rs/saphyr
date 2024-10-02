//! Holds functions to determine if a character belongs to a specific character set.

/// Check if the string can be expressed a valid literal block scalar.
/// The YAML spec supports all of the following in block literals except `#xFEFF`:
/// ```no_compile
///     #x9 | #xA | [#x20-#x7E]                /* 8 bit */
///   | #x85 | [#xA0-#xD7FF] | [#xE000-#xFFFD] /* 16 bit */
///   | [#x10000-#x10FFFF]                     /* 32 bit */
/// ```
#[inline]
pub(crate) fn is_valid_literal_block_scalar(string: &str) -> bool {
    string.chars().all(|character: char|
        matches!(character, '\t' | '\n' | '\x20'..='\x7e' | '\u{0085}' | '\u{00a0}'..='\u{d7fff}'))
}
