use saphyr_parser::{Event, Parser};
#[test]
fn misplaced_closing_bracket() {
    let yaml = "---\n[ a, b, c ] ]\n";

    let mut parser = Parser::new_from_str(yaml);
    let mut i: usize = 0;
    let mut first_err_idx: Option<usize> = None;
    let mut first_doc_end_idx: Option<usize> = None;

    while let Some(next) = parser.next() {
        println!("{:?}", next);
        match next {
            Ok((Event::DocumentEnd, _)) => {
                if first_doc_end_idx.is_none() {
                    first_doc_end_idx = Some(i);
                }
            }
            Err(_) => {
                first_err_idx = Some(i);
                break;
            }
            _ => {}
        }
        i += 1;
    }

    // Assert that a ScanError was emitted and that it happened before DocumentEnd.
    let err_idx = first_err_idx.expect("Expected a ScanError to be emitted");
    let doc_end_idx = first_doc_end_idx.expect("Expected a DocumentEnd event to be emitted");
    assert!(
        err_idx < doc_end_idx,
        "ScanError should be emitted before DocumentEnd (err at {}, doc_end at {})",
        err_idx,
        doc_end_idx
    );
}
