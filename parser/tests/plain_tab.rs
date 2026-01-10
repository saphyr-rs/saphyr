use saphyr_parser::{Event, Parser, ScanError};

fn collect_scalars(input: &str) -> Result<Vec<String>, ScanError> {
    let mut out = Vec::new();
    for item in Parser::new_from_str(input) {
        let ev = item?;
        if let Event::Scalar(s, ..) = ev.0 {
            out.push(s.into());
        }
    }
    Ok(out)
}

#[test]
fn tab_in_block_literal_body_is_allowed() {
    // A tab in the body of a literal block scalar (|) should be accepted.
    let yaml = "key: |\n  a\tb"; // 'a\tb' inside the block content
    let scalars = collect_scalars(yaml).expect("parser should accept tab inside block scalar body");
    // Literal style preserves newlines; a single content line ends with a trailing \n
    assert_eq!(scalars, vec!["key".to_string(), "a\tb\n".to_string()]);
}

#[test]
fn tab_in_block_folded_body_is_allowed() {
    // A tab in the body of a folded block scalar (>) should be accepted as content.
    let yaml = "key: >\n  a\tb";
    let scalars = collect_scalars(yaml).expect("parser should accept tab inside folded block scalar body");
    // For a single content line, folded and literal both end with a trailing \n
    assert_eq!(scalars, vec!["key".to_string(), "a\tb\n".to_string()]);
}

#[test]
fn tab_at_start_of_block_scalar_is_rejected() {
    // If the first content character of the block scalar is a tab, it must be rejected.
    // This means the content line starts with a tab instead of spaces for indentation.
    let yaml = "key: |\n\tvalue";

    let mut got_err: Option<ScanError> = None;
    for item in Parser::new_from_str(yaml) {
        match item {
            Ok(_) => continue,
            Err(e) => {
                got_err = Some(e);
                break;
            }
        }
    }

    let err = got_err.expect("expected a ScanError due to leading tab at start of block scalar content");
    // The scanner has a specific error for this case.
    assert!(
        err.info().contains("a block scalar content cannot start with a tab"),
        "unexpected error message: {}",
        err.info()
    );
}
