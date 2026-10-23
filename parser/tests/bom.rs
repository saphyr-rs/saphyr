use saphyr_parser::{Event, Parser, ScanError};

/// Returns the scalars in `input` and the columns where they start. Parsing from a `str` and from
/// an iterator must agree.
fn scalars(input: &str) -> Result<Vec<(String, usize)>, ScanError> {
    let events: Result<Vec<_>, _> = Parser::new_from_str(input).collect();
    assert_eq!(
        events,
        Parser::new_from_iter(input.chars()).collect(),
        "{input:?}"
    );
    let mut scalars = vec![];
    for (event, span) in events? {
        if let Event::Scalar(s, ..) = event {
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

    // 9.1.1. Document Prefix, each document may start with a byte order mark.
    assert_eq!(
        scalars("a\n...\n\u{FEFF}b").unwrap(),
        [("a".into(), 0), ("b".into(), 0)]
    );
}

#[test]
fn bom_after_document() {
    // 9.2. Streams, a byte order mark ends a document like `...` if only byte order marks,
    // comments, and document markers follow it.
    for yaml in [
        "- a\n\u{FEFF}\u{FEFF}... \n",
        "--- >\nx\n\u{FEFF}",
        "---\n\u{FEFF}--- b",
        "a: 1\n\u{FEFF}# comment\n--- b",
    ] {
        let explicit = yaml.replace('\u{FEFF}', "...\n");
        assert_eq!(
            scalars(yaml).unwrap(),
            scalars(&explicit).unwrap(),
            "{yaml:?}"
        );
    }
}

#[test]
fn bom_inside_document() {
    // Example 5.2, a byte order mark must not appear inside a document.
    for yaml in [
        "- Invalid use of BOM\n\u{FEFF}\n- Inside a document.",
        "a: \u{FEFF}b",
        "a: b\u{FEFF}c",
        "[a,\n\u{FEFF}b]",
        "---\n\u{FEFF}a: 1",
        "%YAML 1.2\n\u{FEFF}---\na",
        "a\n\u{FEFF}%YAML 1.2\n--- b",
        // 5.4.
        "# x\u{FEFF}y\na: 1",
        "a\n... # x\u{FEFF}y",
        "a: |\n  a line longer than the input buffer\u{FEFF}",
    ] {
        let error = scalars(yaml).unwrap_err();
        let bom = yaml.chars().position(|c| c == '\u{FEFF}').unwrap();
        assert_eq!(
            (error.info(), error.marker().index()),
            ("a byte order mark must not appear inside a document", bom),
            "{yaml:?}"
        );
    }

    // BOMs can appear in quoted scalars.
    assert_eq!(scalars("a: \"\u{FEFF}b\"").unwrap()[1].0, "\u{FEFF}b");
    assert_eq!(scalars("a: '\u{FEFF}b'").unwrap()[1].0, "\u{FEFF}b");
}
