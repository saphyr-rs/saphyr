#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::float_cmp)]

use saphyr_parser::{Event, Parser, ScalarStyle, ScanError};

/// Run the parser through the string.
///
/// # Returns
/// This functions returns the events if parsing succeeds, the error the parser returned otherwise.
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
        Ok(str_events.into_iter().map(|x| x.0).collect())
    }
}

#[test]
fn test_fail() {
    let s = "
# syntax error
scalar
key: [1, 2]]
key1:a2
";
    let Err(error) = run_parser(s) else { panic!() };
    assert_eq!(
        error.info(),
        "mapping values are not allowed in this context"
    );
    assert_eq!(
        error.to_string(),
        "mapping values are not allowed in this context at byte 26 line 4 column 4"
    );
}

#[test]
fn test_empty_doc() {
    assert_eq!(
        run_parser("").unwrap(),
        [Event::StreamStart, Event::StreamEnd]
    );

    assert_eq!(
        run_parser("---").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(true),
            Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

#[test]
fn test_utf() {
    assert_eq!(
        run_parser("a: 你好").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::MappingStart(0, None),
            Event::Scalar("a".into(), ScalarStyle::Plain, 0, None),
            Event::Scalar("你好".into(), ScalarStyle::Plain, 0, None),
            Event::MappingEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

#[test]
fn test_comments() {
    let s = "
# This is a comment
a: b # This is another comment
##
  #
";

    assert_eq!(
        run_parser(s).unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::MappingStart(0, None),
            Event::Scalar("a".into(), ScalarStyle::Plain, 0, None),
            Event::Scalar("b".into(), ScalarStyle::Plain, 0, None),
            Event::MappingEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

#[test]
fn test_quoting() {
    let s = "
- plain
- 'squote'
- \"dquote\"
";

    assert_eq!(
        run_parser(s).unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::SequenceStart(0, None),
            Event::Scalar("plain".into(), ScalarStyle::Plain, 0, None),
            Event::Scalar("squote".into(), ScalarStyle::SingleQuoted, 0, None),
            Event::Scalar("dquote".into(), ScalarStyle::DoubleQuoted, 0, None),
            Event::SequenceEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

#[test]
fn test_multi_doc() {
    let s = "
a scalar
---
a scalar
---
a scalar
";
    assert_eq!(
        run_parser(s).unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::Scalar("a scalar".into(), ScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::DocumentStart(true),
            Event::Scalar("a scalar".into(), ScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::DocumentStart(true),
            Event::Scalar("a scalar".into(), ScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

#[test]
fn test_github_27() {
    // https://github.com/chyh1990/yaml-rust/issues/27
    assert_eq!(
        run_parser("&a").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::Scalar("".into(), ScalarStyle::Plain, 1, None),
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

#[test]
fn test_bad_hyphen() {
    // See: https://github.com/chyh1990/yaml-rust/issues/23
    assert!(run_parser("{-").is_err());
}

#[test]
fn test_issue_65() {
    // See: https://github.com/chyh1990/yaml-rust/issues/65
    let b = "\n\"ll\\\"ll\\\r\n\"ll\\\"ll\\\r\r\r\rU\r\r\rU";
    assert!(run_parser(b).is_err());
}

#[test]
fn test_issue_65_mwe() {
    // A MWE for `test_issue_65`. The error over there is that there is invalid trailing content
    // after a double quoted string.
    let b = r#""foo" l"#;
    assert!(run_parser(b).is_err());
}

#[test]
fn test_comment_after_tag() {
    // https://github.com/Ethiraric/yaml-rust2/issues/21#issuecomment-2053513507
    let s = "
%YAML 1.2
# This is a comment
--- #-------
foobar";

    assert_eq!(
        run_parser(s).unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(true),
            Event::Scalar("foobar".into(), ScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}
#[test]
fn test_large_block_scalar_indent() {
    // https://github.com/Ethiraric/yaml-rust2/issues/29
    // https://github.com/saphyr-rs/saphyr-parser/issues/2
    // Tests the `loop` fallback of `skip_block_scalar_indent`. The indent in the YAML string must
    // be greater than `BUFFER_LEN - 2`. The second line is further indented with spaces, and the
    // resulting string should be "a\n    b".
    let s = "
a: |-
                  a
                      b
";

    assert_eq!(
        run_parser(s).unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::MappingStart(0, None),
            Event::Scalar("a".into(), ScalarStyle::Plain, 0, None),
            Event::Scalar("a\n    b".into(), ScalarStyle::Literal, 0, None),
            Event::MappingEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

#[test]
fn test_bad_docstart() {
    run_parser("---This used to cause an infinite loop").unwrap();
    assert_eq!(
        run_parser("----").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::Scalar("----".into(), ScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );

    assert_eq!(
        run_parser("--- #comment").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(true),
            Event::Scalar("~".into(), ScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );

    assert_eq!(
        run_parser("---- #comment").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::Scalar("----".into(), ScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

#[test]
fn test_indentation_equality() {
    let four_spaces = run_parser(
        r"
hash:
    with:
        indentations
",
    )
    .unwrap();

    let two_spaces = run_parser(
        r"
hash:
  with:
    indentations
",
    )
    .unwrap();

    let one_space = run_parser(
        r"
hash:
 with:
  indentations
",
    )
    .unwrap();

    let mixed_spaces = run_parser(
        r"
hash:
     with:
               indentations
",
    )
    .unwrap();

    for (((a, b), c), d) in four_spaces
        .iter()
        .zip(two_spaces.iter())
        .zip(one_space.iter())
        .zip(mixed_spaces.iter())
    {
        assert!(a == b);
        assert!(a == c);
        assert!(a == d);
    }
}

#[test]
fn test_recursion_depth_check_objects() {
    let s = "{a:".repeat(10_000) + &"}".repeat(10_000);
    assert!(run_parser(&s).is_err());
}

#[test]
fn test_recursion_depth_check_arrays() {
    let s = "[".repeat(10_000) + &"]".repeat(10_000);
    assert!(run_parser(&s).is_err());
}
