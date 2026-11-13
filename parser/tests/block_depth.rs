use saphyr_parser::{Event, Parser};

fn nested_map(levels: usize) -> String {
    let mut text = String::from("a:\n");
    for level in 0..levels {
        text.push_str(&" ".repeat(level + 1));
        text.push_str("k:\n");
    }
    text.push_str(&" ".repeat(levels + 1));
    text.push_str("v: 1\n");
    text
}

fn nested_seq(levels: usize) -> String {
    let mut text = String::new();
    for level in 0..levels {
        text.push_str(&" ".repeat(level * 2));
        text.push_str("- k:\n");
    }
    text
}

fn scan(text: String) -> Result<usize, String> {
    let handle = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            Parser::new_from_str(&text)
                .map(|e| e.map(|_| ()))
                .collect::<Result<Vec<_>, _>>()
                .map(|v| v.len())
                .map_err(|e| e.to_string())
        })
        .unwrap();
    handle.join().unwrap()
}

#[test]
fn deep_block_mapping_is_rejected() {
    let err = scan(nested_map(12_000)).unwrap_err();
    assert!(err.contains("recursion limit exceeded"), "{err}");
}

#[test]
fn deep_block_sequence_is_rejected() {
    let err = scan(nested_seq(700)).unwrap_err();
    assert!(err.contains("recursion limit exceeded"), "{err}");
}

#[test]
fn moderate_block_nesting_is_accepted() {
    assert!(scan(nested_map(100)).is_ok());
    assert!(scan(nested_seq(100)).is_ok());
}

#[test]
fn deep_block_mapping_loads_without_hanging() {
    use saphyr_parser::{Parser as P, SpannedEventReceiver};
    struct R;
    impl<'a> SpannedEventReceiver<'a> for R {
        fn on_event(&mut self, _: Event<'a>, _: saphyr_parser::Span) {}
    }
    let text = nested_map(12_000);
    let res = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || P::new_from_str(&text).load(&mut R, true).map_err(|e| e.to_string()))
        .unwrap()
        .join()
        .unwrap();
    assert!(res.is_err());
}
