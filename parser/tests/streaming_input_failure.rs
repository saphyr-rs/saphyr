use std::{cell::RefCell, rc::Rc};

use saphyr_parser::{Event, EventReceiver, ParseError, Parser};

fn parse_events(
    input: impl Iterator<Item = char> + 'static,
) -> Vec<(Event<'static>, saphyr_parser::Span)> {
    Parser::new_from_iter(input)
        .collect::<Result<Vec<_>, _>>()
        .expect("the char-only API treats EOF as success")
}

#[test]
fn true_eof_still_ends_a_valid_prefix() {
    let events = parse_events("key: value\n".chars());
    assert!(
        events
            .iter()
            .any(|(event, _)| matches!(event, Event::StreamEnd))
    );
}

#[test]
fn complete_input_keeps_both_entries() {
    let events = parse_events("key: value\nnext: row\n".chars());
    let values = events
        .iter()
        .filter_map(|(event, _)| match event {
            Event::Scalar(value, _, _, _) => Some(value.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(values.contains(&"key"));
    assert!(values.contains(&"value"));
    assert!(values.contains(&"next"));
    assert!(values.contains(&"row"));
    assert!(
        events
            .iter()
            .any(|(event, _)| matches!(event, Event::StreamEnd))
    );
}

struct BorrowedError<'a> {
    message: &'a str,
    identity: Rc<()>,
}

#[test]
fn source_error_is_returned_once_without_later_reads_or_stream_end() {
    let message = String::from("read budget exhausted");
    let identity = Rc::new(());
    let error_identity = Rc::clone(&identity);
    let reads = Rc::new(RefCell::new(0));
    let source = FailingChars {
        chars: "key: value\n".chars(),
        error: Some(BorrowedError {
            message: &message,
            identity: error_identity,
        }),
        reads: Rc::clone(&reads),
    };
    let mut parser = Parser::new_from_fallible_iter(source);
    let mut emitted = Vec::new();

    loop {
        match parser.next() {
            Some(Ok((event, _))) => emitted.push(event),
            Some(Err(saphyr_parser::ParseError::Source(error))) => {
                assert_eq!(error.message, "read budget exhausted");
                assert!(Rc::ptr_eq(&identity, &error.identity));
                break;
            }
            _ => panic!("expected the original source error"),
        }
    }

    let reads_at_error = *reads.borrow();
    assert!(matches!(
        parser.peek(),
        Some(Err(saphyr_parser::ParseError::Failed))
    ));
    assert!(parser.next().is_none());
    assert_eq!(*reads.borrow(), reads_at_error);
    assert!(
        !emitted
            .iter()
            .any(|event| matches!(event, Event::StreamEnd))
    );
}

#[test]
fn source_error_before_the_first_character_is_not_eof() {
    let identity = Rc::new(());
    let reads = Rc::new(RefCell::new(0));
    let mut parser = Parser::new_from_fallible_iter(FailingChars {
        chars: "".chars(),
        error: Some(BorrowedError {
            message: "no input available",
            identity: Rc::clone(&identity),
        }),
        reads: Rc::clone(&reads),
    });

    let Some(Err(saphyr_parser::ParseError::Source(error))) = parser.next() else {
        panic!("expected the original source error");
    };
    assert_eq!(error.message, "no input available");
    assert!(Rc::ptr_eq(&identity, &error.identity));
    assert!(parser.next().is_none());
    assert_eq!(*reads.borrow(), 1);
}

#[test]
fn syntax_errors_remain_syntax_errors() {
    let mut parser = Parser::new_from_iter("{ unclosed: [".chars());
    let events = parser.by_ref().collect::<Result<Vec<_>, _>>();
    assert!(matches!(events, Err(saphyr_parser::ParseError::Syntax(_))));
    assert!(matches!(
        parser.peek(),
        Some(Err(saphyr_parser::ParseError::Syntax(_)))
    ));
}

#[test]
fn nul_and_replacement_character_remain_distinct_input_values() {
    let nul = Parser::new_from_iter("\0".chars()).collect::<Result<Vec<_>, _>>();
    assert!(matches!(nul, Err(ParseError::Syntax(_))));

    let replacement = Parser::new_from_iter("\u{fffd}".chars())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(replacement.iter().any(|(event, _)| matches!(
        event,
        Event::Scalar(value, _, _, _) if value.as_ref() == "\u{fffd}"
    )));
}

#[test]
fn single_document_load_does_not_read_the_source_error_after_terminator() {
    let source = "---\nfirst\n...\nthis suffix must remain unread\n";
    let reads = Rc::new(RefCell::new(0));
    let mut parser = Parser::new_from_fallible_iter(FailingChars {
        chars: source.chars(),
        error: Some(BorrowedError {
            message: "unread suffix",
            identity: Rc::new(()),
        }),
        reads: Rc::clone(&reads),
    });
    let mut events = RecordingEvents(Vec::new());
    parser
        .load(&mut events, false)
        .unwrap_or_else(|_| panic!("single-document load must stop before the suffix"));
    assert!(events.0.contains(&Event::DocumentEnd));
    assert!(!events.0.contains(&Event::StreamEnd));
    assert!(*reads.borrow() < source.chars().count());
}

struct RecordingEvents<'a>(Vec<Event<'a>>);

impl<'a> EventReceiver<'a> for RecordingEvents<'a> {
    fn on_event(&mut self, event: Event<'a>) {
        self.0.push(event);
    }
}

struct FailingChars<'a> {
    chars: std::str::Chars<'a>,
    error: Option<BorrowedError<'a>>,
    reads: Rc<RefCell<usize>>,
}

impl<'a> Iterator for FailingChars<'a> {
    type Item = Result<char, BorrowedError<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        *self.reads.borrow_mut() += 1;
        if let Some(ch) = self.chars.next() {
            return Some(Ok(ch));
        }
        let error = self
            .error
            .take()
            .expect("the parser read after the source error");
        Some(Err(error))
    }
}

#[test]
fn cloneable_character_iterators_keep_parser_clone_after_consumption() {
    let mut parser = Parser::new_from_iter("first: value\nsecond: value\n".chars());
    assert!(parser.next().unwrap().is_ok());
    assert!(parser.next().unwrap().is_ok());
    let clone = parser.clone();
    let remaining = parser.collect::<Result<Vec<_>, _>>().unwrap();
    let cloned_remaining = clone.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(remaining, cloned_remaining);
}

struct NonCloneChars {
    remaining: core::str::Chars<'static>,
}

impl Iterator for NonCloneChars {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.remaining.next()
    }
}

#[test]
fn non_clone_character_iterators_remain_supported() {
    let input: &'static str = "value";
    let parser = Parser::new_from_iter(NonCloneChars {
        remaining: input.chars(),
    });
    assert!(parser.collect::<Result<Vec<_>, _>>().is_ok());
}

#[derive(Clone)]
struct CloneableFallibleChars {
    remaining: core::str::Chars<'static>,
}

struct NonCloneError;

impl Iterator for CloneableFallibleChars {
    type Item = Result<char, NonCloneError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.remaining.next().map(Ok)
    }
}

#[test]
fn cloneable_fallible_iterator_does_not_require_cloneable_error() {
    let input: &'static str = "first: value\nsecond: value\n";
    let mut parser = Parser::new_from_fallible_iter(CloneableFallibleChars {
        remaining: input.chars(),
    });
    assert!(parser.next().is_some_and(|result| result.is_ok()));
    assert!(parser.next().is_some_and(|result| result.is_ok()));
    let clone = parser.clone();
    let remaining = parser
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|_| panic!("infallible iterator must not fail"));
    let cloned_remaining = clone
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|_| panic!("infallible iterator clone must not fail"));
    assert_eq!(remaining, cloned_remaining);
}

#[test]
fn owned_iterator_events_outlive_the_source_string() {
    fn parse(source: String) -> Vec<Event<'static>> {
        Parser::new_from_iter(source.chars())
            .collect::<Result<Vec<_>, _>>()
            .expect("infallible character input must parse")
            .into_iter()
            .map(|(event, _)| event)
            .collect()
    }

    let events = parse(String::from("value"));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Scalar(..)))
    );
}

#[test]
fn peek_reports_the_first_source_error_and_stops_reading() {
    let reads = Rc::new(RefCell::new(0));
    let identity = Rc::new(());
    let mut parser = Parser::new_from_fallible_iter(FailingChars {
        chars: "".chars(),
        error: Some(BorrowedError {
            message: "peek failure",
            identity: Rc::clone(&identity),
        }),
        reads: Rc::clone(&reads),
    });

    match parser.peek() {
        Some(Err(saphyr_parser::ParseError::Source(error))) => {
            assert_eq!(error.message, "peek failure");
            assert!(Rc::ptr_eq(&identity, &error.identity));
        }
        _ => panic!("expected source error from peek"),
    }
    assert!(parser.next().is_none());
    assert_eq!(*reads.borrow(), 1);
}

#[test]
fn every_first_observer_returns_the_source_error_once_then_terminates_iteration() {
    for observer in ["peek", "next_event", "load"] {
        let reads = Rc::new(RefCell::new(0));
        let identity = Rc::new(());
        let mut parser = Parser::new_from_fallible_iter(FailingChars {
            chars: "".chars(),
            error: Some(BorrowedError {
                message: "first failure",
                identity: Rc::clone(&identity),
            }),
            reads: Rc::clone(&reads),
        });

        let error = match observer {
            "peek" => match parser.peek() {
                Some(Err(error)) => error,
                _ => panic!("peek must report the source error"),
            },
            "next_event" => match parser
                .next_event()
                .expect("the parser must return an event")
            {
                Err(error) => error,
                Ok(_) => panic!("next_event must report the source error"),
            },
            "load" => parser
                .load(&mut RecordingEvents(Vec::new()), true)
                .expect_err("load must report the source error"),
            _ => unreachable!(),
        };

        match error {
            saphyr_parser::ParseError::Source(error) => {
                assert_eq!(error.message, "first failure");
                assert!(Rc::ptr_eq(&identity, &error.identity));
            }
            _ => panic!("the first observer must receive the original source error"),
        }
        assert!(parser.next().is_none());
        assert!(matches!(
            parser.peek(),
            Some(Err(saphyr_parser::ParseError::Failed))
        ));
        assert!(matches!(
            parser.next_event(),
            Some(Err(saphyr_parser::ParseError::Failed))
        ));
        assert!(matches!(
            parser.load(&mut RecordingEvents(Vec::new()), true),
            Err(saphyr_parser::ParseError::Failed)
        ));
        assert_eq!(*reads.borrow(), 1);
    }
}

#[test]
fn source_errors_propagate_at_every_boundary_in_scanner_helpers() {
    for input in [
        "key: value\n",
        "quoted: \"escaped \\\" value\\n\"\n",
        "key: |\n  block value\n",
        "# comment\nkey: value # trailing\n",
        "key: &item !tag value\ncopy: *item\n",
        "---\n...\n",
        "lookahead: 0123456789abcdef0123456789\n",
    ] {
        let events = parse_events(input.chars());
        assert!(
            events
                .iter()
                .any(|(event, _)| matches!(event, Event::StreamEnd))
        );

        for boundary in 0..=input.chars().count() {
            let prefix = input.chars().take(boundary).collect::<String>();
            let reads = Rc::new(RefCell::new(0));
            let identity = Rc::new(());
            let mut parser = Parser::new_from_fallible_iter(FailingChars {
                chars: prefix.chars(),
                error: Some(BorrowedError {
                    message: "boundary failure",
                    identity: Rc::clone(&identity),
                }),
                reads: Rc::clone(&reads),
            });
            let mut saw_source_error = false;
            let mut saw_stream_end = false;
            for item in parser.by_ref() {
                match item {
                    Ok((Event::StreamEnd, _)) => saw_stream_end = true,
                    Ok(_) => {}
                    Err(saphyr_parser::ParseError::Source(error)) => {
                        assert_eq!(error.message, "boundary failure");
                        assert!(Rc::ptr_eq(&identity, &error.identity));
                        saw_source_error = true;
                        break;
                    }
                    Err(_) => panic!("expected source error at character boundary {boundary}"),
                }
            }
            assert!(
                saw_source_error,
                "missing source error at boundary {boundary}"
            );
            assert!(!saw_stream_end, "stream end emitted at boundary {boundary}");
            assert_eq!(*reads.borrow(), boundary + 1);
            let reads_at_error = *reads.borrow();
            assert!(matches!(
                parser.peek(),
                Some(Err(saphyr_parser::ParseError::Failed))
            ));
            assert!(parser.next().is_none());
            assert_eq!(*reads.borrow(), reads_at_error);
        }
    }
}
