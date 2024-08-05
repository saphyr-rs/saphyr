#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::float_cmp)]

use saphyr_parser::{Event, Parser, ScanError};

/// Run the parser through the string, returning all the scalars, and collecting their spans to strings.
fn run_parser_and_deref_scalar_spans(input: &str) -> Result<Vec<(String, String)>, ScanError> {
    let mut events = vec![];
    for x in Parser::new_from_str(input) {
        let x = x?;
        if let Event::Scalar(s, ..) = x.0 {
            let start = x.1.start.index();
            let end = x.1.end.index();
            let input_s = input.chars().skip(start).take(end - start).collect();
            events.push((s, input_s));
        }
    }
    Ok(events)
}

/// Run the parser through the string, returning all the scalars, and collecting their spans to strings.
fn run_parser_and_deref_seq_spans(input: &str) -> Result<Vec<String>, ScanError> {
    let mut events = vec![];
    let mut start_stack = vec![];
    for x in Parser::new_from_str(input) {
        let x = x?;
        match x.0 {
            Event::SequenceStart(_, _) => start_stack.push(x.1.start.index()),
            Event::SequenceEnd => {
                let start = start_stack.pop().unwrap();
                let end = x.1.end.index();
                let input_s = input.chars().skip(start).take(end - start).collect();
                events.push(input_s);
            }
            _ => {}
        }
    }
    Ok(events)
}

fn deref_pairs(pairs: &[(String, String)]) -> Vec<(&str, &str)> {
    pairs
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect()
}

#[test]
fn test_plain() {
    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans("foo: bar").unwrap()),
        [("foo", "foo"), ("bar", "bar"),]
    );
    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans("foo: bar ").unwrap()),
        [("foo", "foo"), ("bar", "bar"),]
    );
    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans("foo :  \t  bar\t ").unwrap()),
        [("foo", "foo"), ("bar", "bar"),]
    );

    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans("foo :  \n  - bar\n  - baz\n ").unwrap()),
        [("foo", "foo"), ("bar", "bar"), ("baz", "baz")]
    );
}

#[test]
fn test_plain_utf8() {
    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans("a: 你好").unwrap()),
        [("a", "a"), ("你好", "你好")]
    );
}

#[test]
fn test_quoted() {
    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans(r#"foo: "bar""#).unwrap()),
        [("foo", "foo"), ("bar", r#""bar""#),]
    );
    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans(r#"foo: 'bar'"#).unwrap()),
        [("foo", "foo"), ("bar", r#"'bar'"#),]
    );

    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans(r#"foo: "bar ""#).unwrap()),
        [("foo", "foo"), ("bar ", r#""bar ""#),]
    );
}

#[test]
fn test_literal() {
    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans("foo: |\n  bar").unwrap()),
        [("foo", "foo"), ("bar\n", "bar"),]
    );
    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans("foo: |\n  bar\n  more").unwrap()),
        [("foo", "foo"), ("bar\nmore\n", "bar\n  more"),]
    );
}

#[test]
fn test_block() {
    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans("foo: >\n  bar").unwrap()),
        [("foo", "foo"), ("bar\n", "bar"),]
    );
    assert_eq!(
        deref_pairs(&run_parser_and_deref_scalar_spans("foo: >\n  bar\n  more").unwrap()),
        [("foo", "foo"), ("bar more\n", "bar\n  more"),]
    );
}

#[test]
fn test_seq() {
    assert_eq!(
        run_parser_and_deref_seq_spans("[a, b]").unwrap(),
        ["[a, b]"]
    );
    assert_eq!(
        run_parser_and_deref_seq_spans("- a\n- b").unwrap(),
        ["- a\n- b"]
    );
    assert_eq!(
        run_parser_and_deref_seq_spans("foo:\n  - a\n  - b").unwrap(),
        ["- a\n  - b"]
    );
    assert_eq!(
        run_parser_and_deref_seq_spans("foo:\n  - a\n  - bar:\n    - b\n    - c").unwrap(),
        ["b\n    - c", "- a\n  - bar:\n    - b\n    - c"]
    );
}
