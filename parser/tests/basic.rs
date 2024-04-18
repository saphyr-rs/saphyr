#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::float_cmp)]

use saphyr_parser::{Event, Parser, ScanError, TScalarStyle};

/// Run the parser through the string.
///
/// # Returns
/// This functions returns the events if parsing succeeds, the error the parser returned otherwise.
fn run_parser(input: &str) -> Result<Vec<Event>, ScanError> {
    let mut events = vec![];
    for x in Parser::new_from_str(input) {
        events.push(x?.0);
    }
    Ok(events)
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
            Event::DocumentStart,
            Event::Scalar("~".to_string(), TScalarStyle::Plain, 0, None),
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
            Event::DocumentStart,
            Event::MappingStart(0, None),
            Event::Scalar("a".to_string(), TScalarStyle::Plain, 0, None),
            Event::Scalar("你好".to_string(), TScalarStyle::Plain, 0, None),
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
            Event::DocumentStart,
            Event::MappingStart(0, None),
            Event::Scalar("a".to_string(), TScalarStyle::Plain, 0, None),
            Event::Scalar("b".to_string(), TScalarStyle::Plain, 0, None),
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
            Event::DocumentStart,
            Event::SequenceStart(0, None),
            Event::Scalar("plain".to_string(), TScalarStyle::Plain, 0, None),
            Event::Scalar("squote".to_string(), TScalarStyle::SingleQuoted, 0, None),
            Event::Scalar("dquote".to_string(), TScalarStyle::DoubleQuoted, 0, None),
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
            Event::DocumentStart,
            Event::Scalar("a scalar".to_string(), TScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::DocumentStart,
            Event::Scalar("a scalar".to_string(), TScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::DocumentStart,
            Event::Scalar("a scalar".to_string(), TScalarStyle::Plain, 0, None),
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
            Event::DocumentStart,
            Event::Scalar(String::new(), TScalarStyle::Plain, 1, None),
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
            Event::DocumentStart,
            Event::Scalar("foobar".to_string(), TScalarStyle::Plain, 0, None),
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
            Event::DocumentStart,
            Event::MappingStart(0, None),
            Event::Scalar("a".to_string(), TScalarStyle::Plain, 0, None),
            Event::Scalar("a\n    b".to_string(), TScalarStyle::Literal, 0, None),
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
            Event::DocumentStart,
            Event::Scalar("----".to_string(), TScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );

    assert_eq!(
        run_parser("--- #comment").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart,
            Event::Scalar("~".to_string(), TScalarStyle::Plain, 0, None),
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );

    assert_eq!(
        run_parser("---- #comment").unwrap(),
        [
            Event::StreamStart,
            Event::DocumentStart,
            Event::Scalar("----".to_string(), TScalarStyle::Plain, 0, None),
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
