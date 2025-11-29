use saphyr_parser::{Event, Parser};

/// Comment intercepting the multiline text is invalid YAML (case BS4K)
#[test]
fn bs4k_comment_between_plain_scalar_lines_should_fail() {
    let yaml = "word1  # comment\nword2\n";

    let mut parser = Parser::new_from_str(yaml);
    while let Some(next) = parser.next() {
        match next {
            Ok((Event::DocumentEnd, _)) => {
                assert!(false, "Document end before any error");
            }
            Err(err) => {
                assert_eq!(err.info(), "comment intercepting the multiline text",
                           "BS4K: comment intercepting the multiline text is invalid YAML"
                );
                break; // fine
            }
            _ => {}
        }
    }
}