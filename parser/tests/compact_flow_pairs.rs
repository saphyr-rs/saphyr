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
fn compact_pair_keeps_every_entry_of_its_nested_mapping() {
    let truth = expected(vec![
        Event::SequenceStart(0, None),
        Event::MappingStart(0, None),
        plain("k"),
        Event::MappingStart(0, None),
        plain("a"),
        plain("1"),
        plain("b"),
        plain("2"),
        Event::MappingEnd,
        Event::MappingEnd,
        Event::SequenceEnd,
    ]);
    for input in [
        "[k: {a: 1, b: 2}]",
        "[{k: {a: 1, b: 2}}]",
        "[? k : {a: 1, b: 2}]",
    ] {
        assert_eq!(parse_both(input).unwrap(), truth, "{input}");
    }
}

#[test]
fn preceding_mapping_does_not_change_compact_pair_events() {
    let truth = expected(vec![
        Event::MappingStart(0, None),
        plain("before"),
        Event::MappingStart(0, None),
        plain("x"),
        plain("0"),
        Event::MappingEnd,
        plain("values"),
        Event::SequenceStart(0, None),
        Event::MappingStart(0, None),
        plain("k"),
        Event::MappingStart(0, None),
        plain("a"),
        plain("1"),
        plain("b"),
        plain("2"),
        Event::MappingEnd,
        Event::MappingEnd,
        Event::SequenceEnd,
        Event::MappingEnd,
    ]);
    for input in [
        "before: {x: 0}\nvalues: [k: {a: 1, b: 2}]",
        "before:\n  x: 0\nvalues: [k: {a: 1, b: 2}]",
    ] {
        assert_eq!(parse_both(input).unwrap(), truth, "{input}");
    }
}

#[test]
fn compact_pairs_survive_nested_sequences_and_mapping_values() {
    let truth = expected(vec![
        Event::SequenceStart(0, None),
        Event::SequenceStart(0, None),
        Event::MappingStart(0, None),
        plain("k"),
        Event::MappingStart(0, None),
        plain("a"),
        Event::SequenceStart(0, None),
        plain("1"),
        plain("2"),
        Event::SequenceEnd,
        plain("b"),
        Event::MappingStart(0, None),
        plain("c"),
        plain("3"),
        plain("d"),
        plain("4"),
        Event::MappingEnd,
        Event::MappingEnd,
        Event::MappingEnd,
        Event::SequenceEnd,
        Event::SequenceEnd,
    ]);
    assert_eq!(
        parse_both("[[k: {a: [1, 2], b: {c: 3, d: 4}}]]").unwrap(),
        truth
    );
}

#[test]
fn each_sequence_entry_has_its_own_pair_state() {
    let truth = expected(vec![
        Event::SequenceStart(0, None),
        Event::MappingStart(0, None),
        plain("k"),
        Event::MappingStart(0, None),
        plain("a"),
        plain("1"),
        plain("b"),
        plain("2"),
        Event::MappingEnd,
        Event::MappingEnd,
        Event::MappingStart(0, None),
        plain("j"),
        Event::MappingStart(0, None),
        plain("c"),
        plain("3"),
        plain("d"),
        plain("4"),
        Event::MappingEnd,
        Event::MappingEnd,
        Event::SequenceEnd,
    ]);
    for input in [
        "[k: {a: 1, b: 2}, j: {c: 3, d: 4}]",
        "[? k : {a: 1, b: 2}, j: {c: 3, d: 4}]",
        "[{k: {a: 1, b: 2}}, j: {c: 3, d: 4}]",
    ] {
        assert_eq!(parse_both(input).unwrap(), truth, "{input}");
    }
}

#[test]
fn mismatched_or_missing_closers_return_the_same_error_without_panicking() {
    for input in [
        "[k: {a: 1, b: 2]",
        "[k: {a: 1, b: 2]]",
        "[k: {a: 1, b: 2}}]",
        "[k: {a: 1, b: 2}, j: [1, 2}",
        "{x: [k: {a: 1, b: 2}], y: 3",
    ] {
        assert!(parse_both(input).is_err(), "{input}");
    }
}
