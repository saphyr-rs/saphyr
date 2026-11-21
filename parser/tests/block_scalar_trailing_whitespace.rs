use saphyr_parser::{Event, Parser};

fn scalars(input: &str) -> Vec<String> {
    Parser::new_from_str(input)
        .filter_map(|item| match item.expect("input is valid").0 {
            Event::Scalar(value, ..) => Some(value.into_owned()),
            _ => None,
        })
        .collect()
}

/// A last line holding only indentation, with no line break after it, is an empty line, which
/// clip chomping drops.
#[test]
fn clip_drops_unterminated_whitespace_line() {
    assert_eq!(scalars("|\n a\n "), ["a\n"]);
    assert_eq!(scalars(">\n a\n "), ["a\n"]);
    assert_eq!(scalars("|\n |\n "), ["|\n"]);
    assert_eq!(scalars(">\n\n folded>-\n\n "), ["\nfolded>-\n"]);
    assert_eq!(scalars("|2\n  a\n  "), ["a\n"]);
    assert_eq!(scalars("k: |\n  a\n  "), ["k", "a\n"]);
    assert_eq!(scalars("|-\n a\n "), ["a"]);
}

/// Keep chomping keeps that empty line, as yaml-test-suite JEF9 does for `- |+\n   `.
#[test]
fn keep_keeps_unterminated_whitespace_line() {
    assert_eq!(scalars("|+\n a\n "), ["a\n\n"]);
    assert_eq!(scalars("--- |+\n ab\n \n "), ["ab\n\n\n"]);
    assert_eq!(scalars("|+\n a"), ["a\n"]);
}
