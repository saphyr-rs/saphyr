#![allow(dead_code)]
#![allow(non_upper_case_globals)]
extern crate saphyr_parser;

use saphyr_parser::{Event, EventReceiver, Parser, ScalarStyle};

// These names match the names used in the C++ test suite.
#[allow(clippy::enum_variant_names)]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
enum TestEvent {
    OnDocumentStart,
    OnDocumentEnd,
    OnSequenceStart,
    OnSequenceEnd,
    OnMapStart,
    OnMapEnd,
    OnScalar,
    OnAlias,
    OnNull,
}

struct YamlChecker {
    pub evs: Vec<TestEvent>,
}

impl EventReceiver<'_> for YamlChecker {
    fn on_event(&mut self, ev: Event) {
        let tev = match ev {
            Event::DocumentStart(_) => TestEvent::OnDocumentStart,
            Event::DocumentEnd => TestEvent::OnDocumentEnd,
            Event::SequenceStart(..) => TestEvent::OnSequenceStart,
            Event::SequenceEnd => TestEvent::OnSequenceEnd,
            Event::MappingStart(..) => TestEvent::OnMapStart,
            Event::MappingEnd => TestEvent::OnMapEnd,
            Event::Scalar(ref v, style, _, _) => {
                if v == "~" && style == ScalarStyle::Plain {
                    TestEvent::OnNull
                } else {
                    TestEvent::OnScalar
                }
            }
            Event::Alias(_) => TestEvent::OnAlias,
            _ => return, // ignore other events
        };
        self.evs.push(tev);
    }
}

fn str_to_test_events(docs: &str) -> Vec<TestEvent> {
    let mut str_events = vec![];
    let mut str_error = None;
    let mut iter_events = vec![];
    let mut iter_error = None;

    for x in Parser::new_from_str(docs) {
        match x {
            Ok(event) => str_events.push(event),
            Err(e) => {
                str_error = Some(e);
                break;
            }
        }
    }
    for x in Parser::new_from_iter(docs.chars()) {
        match x {
            Ok(event) => iter_events.push(event),
            Err(e) => {
                iter_error = Some(e);
                break;
            }
        }
    }

    // eprintln!("str_events");
    // for x in &str_events {
    //     eprintln!("\t{x:?}");
    // }
    // eprintln!("iter_events");
    // for x in &iter_events {
    //     eprintln!("\t{x:?}");
    // }

    assert_eq!(str_events, iter_events);
    assert_eq!(str_error, None);
    assert_eq!(iter_error, None);

    let mut p = YamlChecker { evs: Vec::new() };
    for event in str_events.into_iter().map(|x| x.0) {
        p.on_event(event);
    }
    p.evs
}

macro_rules! assert_next {
    ($v:expr, $p:pat) => {
        match $v.next().unwrap() {
            $p => {}
            e => {
                panic!("unexpected event: {:?} (expected {:?})", e, stringify!($p));
            }
        }
    };
}

// auto generated from handler_spec_test.cpp
include!("specexamples.rs.inc");
include!("spec_test.rs.inc");
