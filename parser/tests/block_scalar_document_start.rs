use saphyr_parser::{Event, Parser};

fn documents(input: &str) -> Vec<String> {
    let mut documents = Vec::new();
    let mut scalars = Vec::new();
    for item in Parser::new_from_str(input) {
        match item.expect("input is valid").0 {
            Event::Scalar(s, ..) => scalars.push(s.to_string()),
            Event::DocumentEnd => documents.push(core::mem::take(&mut scalars).join("|")),
            _ => {}
        }
    }
    documents
}

/// A block scalar that is not indented at the root ends at a line with `---`, as it does at
/// `...`.
#[test]
fn unindented_block_scalar_ends_at_document_start() {
    assert_eq!(documents("|-\nz\n---\na\n"), ["z", "a"]);
    assert_eq!(documents("|\nz\n\n---\na\n"), ["z\n", "a"]);
    assert_eq!(documents(">-\nz\ny\n--- b\n"), ["z y", "b"]);
    assert_eq!(documents("|-\nz\n...\n---\na\n"), ["z", "a"]);
    // Not markers.
    assert_eq!(documents("|-\nz\n---a\n"), ["z\n---a"]);
    assert_eq!(documents("|-\nz\n --- \n"), ["z\n --- "]);
}
