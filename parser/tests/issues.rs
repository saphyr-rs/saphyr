use saphyr::Marker;
use saphyr_parser::{Event, Parser, ScalarStyle, ScanError, Span};

/// Run the parser through the string.
///
/// The parser is run through both the `StrInput` and `BufferedInput` variants. The resulting
/// events are then compared and must match.
///
/// # Returns
/// This function returns the events and associated spans if parsing succeeds, the error the parser returned otherwise.
///
/// # Panics
/// This function panics if there is a mismatch between the 2 parser invocations with the different
/// input traits.
fn run_parser_with_span(input: &str) -> Result<Vec<(Event<'_>, Span)>, ScanError> {
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

    // eprintln!("str_events");
    // for x in &str_events {
    //     eprintln!("\t{x:?}");
    // }
    // eprintln!("iter_events");
    // for x in &iter_events {
    //     eprintln!("\t{x:?}");
    // }

    assert_eq!(str_events, iter_events);
    assert_eq!(str_error, iter_error);

    if let Some(err) = str_error {
        Err(err)
    } else {
        Ok(str_events)
    }
}

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
fn run_parser(input: &str) -> Result<Vec<Event<'_>>, ScanError> {
    Ok(run_parser_with_span(input)?
        .into_iter()
        .map(|x| x.0)
        .collect())
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_issue1() {
    // https://github.com/saphyr-rs/saphyr-parser/issues/1

    // Implicit mappings with a nested flow sequence prematurely end the mapping.
    //
    // [a: [42]]
    //        ^ This closing sequence character
    // ^ is interpreted as closing this sequence, where the implicit mapping starts.
    //
    // The current implementation does not allow for nested implicit mapping:
    //
    // [a: [b: c]]
    //      ^ this implicit mapping would be ignored
    let reference = r"
- a:
  - 42
";

    let expected = [
        Event::StreamStart,
        Event::DocumentStart(false),
        Event::SequenceStart(0, None),
        Event::MappingStart(0, None),
        Event::Scalar("a".into(), ScalarStyle::Plain, 0, None),
        Event::SequenceStart(0, None),
        Event::Scalar("42".into(), ScalarStyle::Plain, 0, None),
        Event::SequenceEnd,
        Event::MappingEnd,
        Event::SequenceEnd,
        Event::DocumentEnd,
        Event::StreamEnd,
    ];
    assert_eq!(run_parser(reference).unwrap(), expected);
    assert_eq!(run_parser("[{a: [42]}]").unwrap(), expected);
    assert_eq!(run_parser("[a: [42]]").unwrap(), expected);

    // Other test cases derived from the bug

    // Implicit mapping in a complex key.
    assert_eq!(
        run_parser("[foo: [bar]]: baz").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::MappingStart(0, None),
            Event::SequenceStart(0, None),
            Event::MappingStart(0, None),
            Event::Scalar("foo".into(), ScalarStyle::Plain, 0, None),
            Event::SequenceStart(0, None),
            Event::Scalar("bar".into(), ScalarStyle::Plain, 0, None),
            Event::SequenceEnd,
            Event::MappingEnd,
            Event::SequenceEnd,
            Event::Scalar("baz".into(), ScalarStyle::Plain, 0, None),
            Event::MappingEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );

    // Implicit mappings with implicit null keys
    assert_eq!(
        run_parser("[:]").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::SequenceStart(0, None),
            Event::MappingStart(0, None),
            Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),
            Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),
            Event::MappingEnd,
            Event::SequenceEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );

    // Nested implicit mappings with implicit null keys
    assert_eq!(
        run_parser("[: [:]]").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::SequenceStart(0, None),
            Event::MappingStart(0, None),
            Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),
            Event::SequenceStart(0, None),
            Event::MappingStart(0, None),
            Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),
            Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),
            Event::MappingEnd,
            Event::SequenceEnd,
            Event::MappingEnd,
            Event::SequenceEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );

    // Interleaved nested implicit and non-implicit mappings.
    assert_eq!(
        run_parser("[a: [  [b:]]]").unwrap(),
        //          ^   ^  ^ has an implicit mapping
        //          |   ` has no implicit mapping
        //          ` has an implicit mapping
        // We must make sure that the `MappingEnd` events are correctly issued for the first and
        // third nested sequences, but not the second.
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::SequenceStart(0, None),
            Event::MappingStart(0, None),
            Event::Scalar("a".into(), ScalarStyle::Plain, 0, None),
            Event::SequenceStart(0, None),
            // No `MappingStart` here.
            Event::SequenceStart(0, None),
            Event::MappingStart(0, None),
            Event::Scalar("b".into(), ScalarStyle::Plain, 0, None),
            Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),
            Event::MappingEnd,
            Event::SequenceEnd,
            // No `MappingEnd` here.
            Event::SequenceEnd,
            Event::MappingEnd,
            Event::SequenceEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );

    // There needs to be a space between a `:` in a flow sequence and the value.
    assert!(run_parser("[:[:]]").is_err());
    assert!(run_parser("[a:[42]]").is_err());

    // Double-quoted keys may have a value without a space for JSON-compatibility.
    assert_eq!(
        run_parser(r#"["a":[]]"#).unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::SequenceStart(0, None),
            Event::MappingStart(0, None),
            Event::Scalar("a".into(), ScalarStyle::DoubleQuoted, 0, None),
            Event::SequenceStart(0, None),
            Event::SequenceEnd,
            Event::MappingEnd,
            Event::SequenceEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

#[test]
fn test_pr12() {
    assert_eq!(
        run_parser("---\n- |\n  a").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(true),
            Event::SequenceStart(0, None),
            Event::Scalar("a\n".into(), ScalarStyle::Literal, 0, None),
            Event::SequenceEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

#[test]
fn test_issue14() {
    // The following input creates an infinite loop.
    // https://github.com/saphyr-rs/saphyr/issues/14
    let s = "{---";
    let Err(error) = run_parser(s) else { panic!() };
    assert_eq!(
        error.info(),
        "while parsing a flow mapping, did not find expected ',' or '}'"
    );
    assert_eq!(
        error.to_string(),
        "while parsing a flow mapping, did not find expected ',' or '}' at byte 4 line 2 column 1"
    );
}

#[test]
fn test_issue14_v2() {
    let s = "{...";
    let Err(error) = run_parser(s) else { panic!() };
    assert_eq!(
        error.info(),
        "while parsing a flow mapping, did not find expected ',' or '}'"
    );
    assert!(error.to_string().ends_with("at byte 4 line 2 column 1"));
}

#[test]
fn test_issue13() {
    // The following input creates an infinite loop.
    // https://github.com/saphyr-rs/saphyr/issues/13
    let s = r"---
array:
  - object:
      array:
        - object:
            array:
              - text: >-
                  Line 1
                  Line 2
...";

    assert_eq!(
        run_parser(s).unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(true),
            Event::MappingStart(0, None),
            Event::Scalar("array".into(), ScalarStyle::Plain, 0, None),
            Event::SequenceStart(0, None),
            Event::MappingStart(0, None),
            Event::Scalar("object".into(), ScalarStyle::Plain, 0, None),
            Event::MappingStart(0, None),
            Event::Scalar("array".into(), ScalarStyle::Plain, 0, None),
            Event::SequenceStart(0, None),
            Event::MappingStart(0, None),
            Event::Scalar("object".into(), ScalarStyle::Plain, 0, None),
            Event::MappingStart(0, None),
            Event::Scalar("array".into(), ScalarStyle::Plain, 0, None),
            Event::SequenceStart(0, None),
            Event::MappingStart(0, None),
            Event::Scalar("text".into(), ScalarStyle::Plain, 0, None),
            Event::Scalar("Line 1 Line 2".into(), ScalarStyle::Folded, 0, None),
            Event::MappingEnd,
            Event::SequenceEnd,
            Event::MappingEnd,
            Event::MappingEnd,
            Event::SequenceEnd,
            Event::MappingEnd,
            Event::MappingEnd,
            Event::SequenceEnd,
            Event::MappingEnd,
            Event::DocumentEnd,
            Event::StreamEnd
        ]
    );
}

#[test]
fn test_issue22() {
    // The ellipsis is parsed as a document end.
    // https://github.com/saphyr-rs/saphyr/issues/22
    let s = "comment: hello ... world";
    assert_eq!(
        run_parser(s).unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::MappingStart(0, None),
            Event::Scalar("comment".into(), ScalarStyle::Plain, 0, None),
            Event::Scalar("hello ... world".into(), ScalarStyle::Plain, 0, None),
            Event::MappingEnd,
            Event::DocumentEnd,
            Event::StreamEnd
        ]
    );
}

#[test]
#[rustfmt::skip]
fn test_issue37() {
    // MarkedYaml: Empty scalars get the span of the next value.
    // With the fix they get the position of the ":" for block maps and the "-" for block arrays.
    let s = r"---
    hash_block_null_value:
    hash_flow: {hash_flow_null_value: null}
    array_block_null_value:
      -
      - ~
      - null
    array_flow_null_value: [~, null]
    indentless_array_block_null_value:
    -
    - ~
    - null
    ";

    assert_eq!(
        run_parser_with_span(s).unwrap(),
        [
            (Event::StreamStart,                                                          Span::new(Marker::new(0, 1, 0), Marker::new(0, 1, 0))),
            (Event::DocumentStart(true),                                                  Span::new(Marker::new(0, 1, 0), Marker::new(3, 1, 3))),
            (Event::MappingStart(0, None),                                                Span::new(Marker::new(8, 2, 4), Marker::new(8, 2, 4))),
            (Event::Scalar("hash_block_null_value".into(), ScalarStyle::Plain, 0, None),  Span::new(Marker::new(8, 2, 4), Marker::new(29, 2, 25))),

            (Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),                      Span::new(Marker::new(29, 2, 25), Marker::new(29, 2, 25))),

            (Event::Scalar("hash_flow".into(), ScalarStyle::Plain, 0, None),              Span::new(Marker::new(35, 3, 4), Marker::new(44, 3, 13))),
            (Event::MappingStart(0, None),                                                Span::new(Marker::new(46, 3, 15), Marker::new(47, 3, 16))),
            (Event::Scalar("hash_flow_null_value".into(), ScalarStyle::Plain, 0, None),   Span::new(Marker::new(47, 3, 16), Marker::new(67, 3, 36))),
            (Event::Scalar("null".into(), ScalarStyle::Plain, 0, None),                   Span::new(Marker::new(69, 3, 38), Marker::new(73, 3, 42))),
            (Event::MappingEnd,                                                           Span::new(Marker::new(73, 3, 42), Marker::new(74, 3, 43))),
            (Event::Scalar("array_block_null_value".into(), ScalarStyle::Plain, 0, None), Span::new(Marker::new(79, 4, 4), Marker::new(101, 4, 26))),
            (Event::SequenceStart(0, None),                                               Span::new(Marker::new(109, 5, 6), Marker::new(109, 5, 6))),

            (Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),                      Span::new(Marker::new(110, 5, 7), Marker::new(110, 5, 7))),

            (Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),                                 Span::new(Marker::new(119, 6, 8), Marker::new(120, 6, 9))),
            (Event::Scalar("null".into(), ScalarStyle::Plain, 0, None),                              Span::new(Marker::new(129, 7, 8), Marker::new(133, 7, 12))),
            (Event::SequenceEnd,                                                                     Span::new(Marker::new(138, 8, 4), Marker::new(138, 8, 4))),
            (Event::Scalar("array_flow_null_value".into(), ScalarStyle::Plain, 0, None),             Span::new(Marker::new(138, 8, 4), Marker::new(159, 8, 25))),
            (Event::SequenceStart(0, None),                                                          Span::new(Marker::new(161, 8, 27), Marker::new(162, 8, 28))),
            (Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),                                 Span::new(Marker::new(162, 8, 28), Marker::new(163, 8, 29))),
            (Event::Scalar("null".into(), ScalarStyle::Plain, 0, None),                              Span::new(Marker::new(165, 8, 31), Marker::new(169, 8, 35))),
            (Event::SequenceEnd,                                                                     Span::new(Marker::new(169, 8, 35), Marker::new(170, 8, 36))),
            (Event::Scalar("indentless_array_block_null_value".into(), ScalarStyle::Plain, 0, None), Span::new(Marker::new(175, 9, 4), Marker::new(208, 9, 37))),
            (Event::SequenceStart(0, None),                                                          Span::new(Marker::new(215, 10, 5), Marker::new(215, 10, 5))),

            (Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),    Span::new(Marker::new(215, 10, 5), Marker::new(215, 10, 5))),

            (Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),    Span::new(Marker::new(222, 11, 6), Marker::new(223, 11, 7))),
            (Event::Scalar("null".into(), ScalarStyle::Plain, 0, None), Span::new(Marker::new(230, 12, 6), Marker::new(234, 12, 10))),
            (Event::SequenceEnd,                                        Span::new(Marker::new(239, 14, 0), Marker::new(239, 14, 0))),
            (Event::MappingEnd,                                         Span::new(Marker::new(239, 14, 0), Marker::new(239, 14, 0))),
            (Event::DocumentEnd,                                        Span::new(Marker::new(239, 14, 0), Marker::new(239, 14, 0))),
            (Event::StreamEnd,                                          Span::new(Marker::new(239, 14, 0), Marker::new(239, 14, 0)))
        ]
    );
}
