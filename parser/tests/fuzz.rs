use core::str;

use saphyr_parser::{Event, Parser, ScanError};

/// Run the parser through the string.
///
/// The parser is run through both the `StrInput` and `BufferedInput` variants. The resulting
/// events are then compared and must match.
///
/// # Returns
/// This function returns the events if parsing succeeds, the error the parser returned otherwise.
///
/// # Panics
/// This function panics if there is a mismatch between the 2 parser invocations with the different
/// input traits.
fn run_parser(input: &str) -> Result<Vec<Event>, ScanError> {
    let mut str_events = vec![];
    let mut str_error = None;
    let mut iter_events = vec![];
    let mut iter_error = None;

    for x in Parser::new_from_str(input) {
        match x {
            Ok(event) => str_events.push(event),
            Err(e) => {
                str_error = Some(e);
                break;
            }
        }
    }
    for x in Parser::new_from_iter(input.chars()) {
        match x {
            Ok(event) => iter_events.push(event),
            Err(e) => {
                iter_error = Some(e);
                break;
            }
        }
    }

    assert_eq!(str_events, iter_events);
    assert_eq!(str_error, iter_error);

    if let Some(err) = str_error {
        Err(err)
    } else {
        Ok(str_events.into_iter().map(|x| x.0).collect())
    }
}

#[test]
fn fuzz_1() {
    // Crashing with an index out-of-bounds error.
    // In `scan_plain_scalar`, we would lookahead 1 and call `skip_break`, which requires a
    // lookahead of 2.
    let raw_input: &[u8] = &[
        1, 39, 110, 117, 108, 108, 34, 13, 13, 13, 13, 13, 10, 13, 13, 13, 13,
    ];
    let s = str::from_utf8(raw_input).unwrap();
    let _ = run_parser(s);
}

#[test]
fn fuzz_2() {
    // Crashing with an unwrap of a None value.
    // There is an imbalance of implicit flow mapping contexts here between the opening `[`/`{` and
    // closing `]`/`}`. We would test against flow-level when only `[` can create implicit flow
    // mappings.
    let raw_input: &[u8] = &[
        91, 91, 32, 101, 58, 9, 123, 63, 32, 45, 106, 101, 58, 9, 123, 63, 32, 44, 117, 101, 58, 9,
        123, 63, 32, 44, 9, 26, 58, 32, 126, 93, 8, 58, 32, 58, 10, 29, 58, 58, 58, 32, 58, 29, 63,
        32, 44, 9, 26, 58, 32, 126, 93, 8, 58, 32, 58, 10, 78, 32,
    ];
    let s = str::from_utf8(raw_input).unwrap();
    let _ = run_parser(s);
}

#[test]
fn fuzz_3() {
    // Span mismatch when parsing with `StrInput` and `BufferedInput`.
    // In block scalars, there was a section in which we took the byte count rather than the char
    // count to update the index. The issue didn't happen with `StrInput` as the buffer was always
    // full and the offending code was never executed.
    let raw_input: &[u8] = &[124, 13, 32, 210, 180, 65];
    let s = str::from_utf8(raw_input).unwrap();
    let _ = run_parser(s);
}
