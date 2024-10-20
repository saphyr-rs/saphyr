#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut str_events = vec![];
        let mut str_error = None;
        let mut iter_events = vec![];
        let mut iter_error = None;

        for x in saphyr_parser::Parser::new_from_str(s) {
            match x {
                Ok(event) => str_events.push(event),
                Err(e) => {
                    str_error = Some(e);
                    break;
                }
            }
        }
        for x in saphyr_parser::Parser::new_from_iter(s.chars()) {
            match x {
                Ok(event) => iter_events.push(event),
                Err(e) => {
                    iter_error = Some(e);
                    break;
                }
            }
        }

        assert_eq!(str_events, iter_events);
        assert_eq!(str_error, iter_error);
    }
});
