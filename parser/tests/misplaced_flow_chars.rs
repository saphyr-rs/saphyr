use saphyr_parser::{Event, Parser, ScalarStyle};

#[test]
fn misplaced_closing_bracket_at_col_0() {
    let yaml = "key: [\n]\n";
    let mut parser = Parser::new_from_str(yaml);

    let mut events = Vec::new();
    while let Some(next) = parser.next() {
        match next {
            Ok((ev, _)) => events.push(ev),
            Err(err) => {
                panic!("Error: {}", err.info());
            }
        }
    }

    assert_eq!(events.len(), 9);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(events[1], Event::DocumentStart(false)));
    assert!(matches!(events[2], Event::MappingStart(0, None)));
    if let Event::Scalar(ref v, ScalarStyle::Plain, 0, None) = events[3] {
        assert_eq!(&**v, "key");
    } else { panic!("Expected scalar 'key'"); }
    assert!(matches!(events[4], Event::SequenceStart(0, None)));
    assert!(matches!(events[5], Event::SequenceEnd));
    assert!(matches!(events[6], Event::MappingEnd));
    assert!(matches!(events[7], Event::DocumentEnd));
    assert!(matches!(events[8], Event::StreamEnd));
}

#[test]
fn misplaced_closing_brace_at_col_0() {
    let yaml = "key: {\n}\n";
    let mut parser = Parser::new_from_str(yaml);

    let mut events = Vec::new();
    while let Some(next) = parser.next() {
        match next {
            Ok((ev, _)) => events.push(ev),
            Err(err) => {
                panic!("Error: {}", err.info());
            }
        }
    }

    assert_eq!(events.len(), 9);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(events[1], Event::DocumentStart(false)));
    assert!(matches!(events[2], Event::MappingStart(0, None)));
    if let Event::Scalar(ref v, ScalarStyle::Plain, 0, None) = events[3] {
        assert_eq!(&**v, "key");
    } else { panic!("Expected scalar 'key'"); }
    assert!(matches!(events[4], Event::MappingStart(0, None)));
    assert!(matches!(events[5], Event::MappingEnd));
    assert!(matches!(events[6], Event::MappingEnd));
    assert!(matches!(events[7], Event::DocumentEnd));
    assert!(matches!(events[8], Event::StreamEnd));
}

#[test]
fn misplaced_comma_at_col_0() {
    let yaml = "key: [\n a\n, b\n]\n";
    let mut parser = Parser::new_from_str(yaml);

    let mut events = Vec::new();
    while let Some(next) = parser.next() {
        match next {
            Ok((ev, _)) => events.push(ev),
            Err(err) => {
                panic!("Error: {}", err.info());
            }
        }
    }

    assert_eq!(events.len(), 11);
    assert!(matches!(events[0], Event::StreamStart));
    assert!(matches!(events[1], Event::DocumentStart(false)));
    assert!(matches!(events[2], Event::MappingStart(0, None)));
    if let Event::Scalar(ref v, ScalarStyle::Plain, 0, None) = events[3] {
        assert_eq!(&**v, "key");
    } else { panic!("Expected scalar 'key'"); }
    assert!(matches!(events[4], Event::SequenceStart(0, None)));
    if let Event::Scalar(ref v, ScalarStyle::Plain, 0, None) = events[5] {
        assert_eq!(&**v, "a");
    } else { panic!("Expected scalar 'a'"); }
    if let Event::Scalar(ref v, ScalarStyle::Plain, 0, None) = events[6] {
        assert_eq!(&**v, "b");
    } else { panic!("Expected scalar 'b'"); }
    assert!(matches!(events[7], Event::SequenceEnd));
    assert!(matches!(events[8], Event::MappingEnd));
    assert!(matches!(events[9], Event::DocumentEnd));
    assert!(matches!(events[10], Event::StreamEnd));
}
