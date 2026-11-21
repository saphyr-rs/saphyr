use saphyr_parser::{Event, Parser, ScalarStyle, ScanError};

fn plain(value: &'static str) -> Event<'static> {
    Event::Scalar(value.into(), ScalarStyle::Plain, 0, None)
}

fn expected(body: Vec<Event<'static>>) -> Vec<Event<'static>> {
    let mut events = vec![Event::StreamStart, Event::DocumentStart(false)];
    events.extend(body);
    events.extend([Event::DocumentEnd, Event::StreamEnd]);
    events
}

fn parse_both(input: &str) -> Result<Vec<Event<'_>>, ScanError> {
    let string = Parser::new_from_str(input).collect::<Result<Vec<_>, _>>();
    let iterator = Parser::new_from_iter(input.chars()).collect::<Result<Vec<_>, _>>();
    assert_eq!(
        string, iterator,
        "string/iterator events and spans for {input:?}"
    );
    string.map(|events| events.into_iter().map(|(event, _)| event).collect())
}

#[test]
fn tab_separates_explicit_key_in_flow_sequence() {
    let truth = expected(vec![
        Event::SequenceStart(0, None),
        Event::MappingStart(0, None),
        plain("f"),
        plain(""),
        Event::MappingEnd,
        Event::SequenceEnd,
    ]);
    for input in ["[?\tf]", "[? \tf]", "[?\t f]", "[?\n\tf]", "[?\t#c\n f]"] {
        assert_eq!(parse_both(input).unwrap(), truth, "{input:?}");
    }
}

#[test]
fn tab_separates_explicit_key_in_flow_mapping() {
    let truth = expected(vec![
        Event::MappingStart(0, None),
        plain("f"),
        plain("v"),
        Event::MappingEnd,
    ]);
    for input in ["{?\tf: v}", "{? \tf: v}", "{?\n\tf: v}"] {
        assert_eq!(parse_both(input).unwrap(), truth, "{input:?}");
    }
}

#[test]
fn tab_after_explicit_key_with_empty_content() {
    let truth = expected(vec![
        Event::MappingStart(0, None),
        plain(""),
        plain(""),
        Event::MappingEnd,
    ]);
    for input in ["{?\t}", "{?\n\t}"] {
        assert_eq!(parse_both(input).unwrap(), truth, "{input:?}");
    }
}

#[test]
fn tab_separates_explicit_key_in_flow_nested_in_block() {
    let truth = expected(vec![
        Event::SequenceStart(0, None),
        Event::SequenceStart(0, None),
        Event::MappingStart(0, None),
        plain("f"),
        plain(""),
        Event::MappingEnd,
        Event::SequenceEnd,
        Event::SequenceEnd,
    ]);
    assert_eq!(parse_both("- [?\tf]").unwrap(), truth);
}
