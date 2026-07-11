use saphyr_parser::{Event, Parser};

/// Test case 4H7K in yaml_test_suite
#[test]
fn misplaced_closing_bracket() {
    let yaml = "---\n[ a, b, c ] ]\n";
    let parser = Parser::new_from_str(yaml);

    for next in parser {
        match next {
            Ok((Event::DocumentEnd, _)) => {
                panic!("Document end before any error");
            }
            Err(err) => {
                assert_eq!(
                    err.info(),
                    "misplaced bracket",
                    "4H7K: misplaced bracket should result the error"
                );
                break; // fine
            }
            _ => {}
        }
    }
}
