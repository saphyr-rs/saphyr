use saphyr_parser::{Event, Parser};

/// Test case 4H7K in yaml_test_suite
#[test]
fn misplaced_closing_bracket() {
    let yaml = "---\n[ a, b, c ] ]\n";
    let mut parser = Parser::new_from_str(yaml);

    while let Some(next) = parser.next() {
        match next {
            Ok((Event::DocumentEnd, _)) => {
                assert!(false, "Document end before any error");
            }
            Err(err) => {
                assert_eq!(err.info(), "misplaced bracket",
                        "4H7K: misplaced bracket should result the error"
                );
                break; // fine
            }
            _ => {}
        }
    }
}
