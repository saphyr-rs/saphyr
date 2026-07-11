use saphyr_parser::{Event, Parser, ScalarStyle};

/// Comment intercepting the multiline text is invalid YAML (case BS4K)
#[test]
fn bs4k_comment_between_plain_scalar_lines_should_fail() {
    let yaml = r#"word1  # comment
word2
"#;

    let parser = Parser::new_from_str(yaml);
    for next in parser {
        match next {
            Ok((Event::DocumentEnd, _)) => {
                panic!("Document end before any error");
            }
            Err(err) => {
                assert_eq!(
                    err.info(),
                    "comment intercepting the multiline text",
                    "BS4K: comment intercepting the multiline text is invalid YAML"
                );
                break; // fine
            }
            _ => {}
        }
    }
}

#[test]
fn bs4k_comment_between_plain_scalar_lines_in_map_should_fail() {
    let yaml = r#"
key: word1  # comment
  word2
"#;

    let parser = Parser::new_from_str(yaml);
    let mut got_error = false;
    for next in parser {
        if let Err(err) = next {
            assert_eq!(
                err.info(),
                "comment intercepting the multiline text",
                "BS4K: comment intercepting the multiline text is invalid YAML in a map"
            );
            got_error = true;
            break;
        }
    }
    assert!(got_error, "Should have encountered BS4K error in map");
}

#[test]
fn multiline_plain_scalar_in_map_without_comment_is_valid() {
    let yaml = r#"
key: word1
  word2
"#;

    let parser = Parser::new_from_str(yaml);
    let mut events = Vec::new();
    for next in parser {
        events.push(next.unwrap().0);
    }

    assert!(events.contains(&Event::Scalar(
        "word1 word2".into(),
        ScalarStyle::Plain,
        0,
        None
    )));
}
