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
    let mut iter_events = vec![];

    for x in Parser::new_from_str(input) {
        str_events.push(x?.0);
    }
    for x in Parser::new_from_iter(input.chars()) {
        iter_events.push(x?.0);
    }

    assert_eq!(str_events, iter_events);

    Ok(str_events)
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
