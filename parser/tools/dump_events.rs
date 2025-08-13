use std::env;
use std::fs::File;
use std::io::prelude::*;

use saphyr_parser::{Event, Parser, Span, SpannedEventReceiver};

#[derive(Debug)]
struct EventSink<'a> {
    events: Vec<(Event<'a>, Span)>,
}

impl<'a> SpannedEventReceiver<'a> for EventSink<'a> {
    fn on_event(&mut self, ev: Event<'a>, span: Span) {
        eprintln!("      \x1B[;34m\u{21B3} {:?}\x1B[;m", &ev);
        self.events.push((ev, span));
    }
}

fn str_to_events(yaml: &str) -> Vec<(Event<'_>, Span)> {
    let mut sink = EventSink { events: Vec::new() };
    let mut parser = Parser::new_from_str(yaml);
    // Load events using our sink as the receiver.
    parser.load(&mut sink, true).unwrap();
    sink.events
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let mut f = File::open(&args[1]).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    // dbg!(str_to_events(&s));
    str_to_events(&s);
}
