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

/// Anything left after the root node is an error, and it must be reported before `DocumentEnd`.
/// A caller that only reads one document stops at that event and would never see the error.
#[test]
fn trailing_content_is_an_error_before_document_end() {
    for yaml in [
        "---\n[a] trailing\n",
        "---\n[a],\n",
        "---\n{a: b} trailing\n",
        "--- \"a\" trailing\n",
    ] {
        let mut saw_error = false;
        for next in Parser::new_from_str(yaml) {
            match next {
                Ok((Event::DocumentEnd, _)) => {
                    panic!("{yaml:?}: document end before any error");
                }
                Err(_) => {
                    saw_error = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(saw_error, "{yaml:?}: expected an error");
    }
}

/// Input that may legally follow an implicit document end still parses.
#[test]
fn document_markers_and_bom_may_follow_implicit_document_end() {
    for yaml in [
        "---\n[a]\n",
        "---\n[a]\n---\n[b]\n",
        "---\n[a]\n...\n",
        "---\n[a]\n\u{feff}---\n[b]\n",
        "[a]\n# comment\n",
    ] {
        for next in Parser::new_from_str(yaml) {
            assert!(next.is_ok(), "{yaml:?}: {next:?}");
        }
    }
}
