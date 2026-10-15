use saphyr_parser::{Event, Parser, ScanError};

/// Returns the scalars in `input` and the columns where they start.
fn scalars(input: &str) -> Result<Vec<(String, usize)>, ScanError> {
    let mut scalars = vec![];
    for event in Parser::new_from_str(input) {
        if let (Event::Scalar(s, ..), span) = event? {
            scalars.push((s.into(), span.start.col()));
        }
    }
    Ok(scalars)
}

#[test]
fn bom_before_document() {
    let expected = [("a".to_owned(), 0), ("1".to_owned(), 3)];
    assert_eq!(scalars("\u{FEFF}a: 1").unwrap(), expected);
    assert_eq!(scalars("\u{FEFF}\u{FEFF}a: 1").unwrap(), expected);
    assert_eq!(scalars("\u{FEFF}# comment\na: 1").unwrap(), expected);
    assert_eq!(scalars("\u{FEFF}%YAML 1.2\n---\na: 1").unwrap(), expected);
    assert!(scalars("\u{FEFF}").unwrap().is_empty());

    // Example 9.2, each document may start with a byte order mark.
    assert_eq!(
        scalars("a\n...\n\u{FEFF}b").unwrap(),
        [("a".into(), 0), ("b".into(), 0)]
    );
    assert_eq!(
        scalars("a: 1\n\u{FEFF}--- b").unwrap().last(),
        Some(&("b".into(), 4))
    );
    assert_eq!(scalars("- a\n\u{FEFF}... \n").unwrap(), [("a".into(), 2)]);
}

#[test]
fn bom_inside_document() {
    // Example 5.2, a byte order mark must not appear inside a document.
    for yaml in [
        "- Invalid use of BOM\n\u{FEFF}\n- Inside a document.",
        "a: 1\n\u{FEFF}b: 2",
        "a: \u{FEFF}b",
        "a: b\u{FEFF}c",
        "[a, \u{FEFF}b]",
    ] {
        let error = scalars(yaml).unwrap_err();
        assert_eq!(
            error.info(),
            "a byte order mark must not appear inside a document",
            "{yaml:?}"
        );
    }

    // Quoted scalars may contain one.
    assert_eq!(scalars("a: \"\u{FEFF}b\"").unwrap()[1].0, "\u{FEFF}b");
    assert_eq!(scalars("a: '\u{FEFF}b'").unwrap()[1].0, "\u{FEFF}b");
}
