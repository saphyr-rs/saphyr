# Saphyr libraries

This repository is home to `saphyr-parser`, `saphyr` and soon-to-be
`saphyr-serde`. These crates provide fully YAML 1.2 compliant parsing and
manipulation, with a focus on correctness, performance and API friendliness (in that order).

[`saphyr`](https://docs.rs/saphyr/latest/saphyr/) is the most user-friendly and
high-level crate, providing quick-and-easy YAML importing, exporting and object
manipulation.

```rs
use saphyr::{YamlLoader, YamlEmitter};

let docs = YamlLoader::load_from_str("[1, 2, 3]").unwrap();
let doc = &docs[0]; // select the first YAML document
assert_eq!(doc[0].as_i64().unwrap(), 1); // access elements by index

let mut out_str = String::new();
let mut emitter = YamlEmitter::new(&mut out_str);
emitter.dump(doc).unwrap(); // dump the YAML object to a String
```

---

[`saphyr-parser`](https://docs.rs/saphyr-parser/latest/saphyr_parser/) is the
parser behind `saphyr`. It provides direct access to the parsing process by
emitting [YAML
events](https://docs.rs/saphyr-parser/latest/saphyr_parser/parser/enum.Event.html).
It does not include YAML to object mapping, but is a lightweight alternative to
`saphyr` for those interested in building directly atop the parser, without
having an intermediate conversion to a Rust object. More details on where to
start are available [on
doc.rs](https://docs.rs/saphyr-parser/latest/saphyr_parser/parser/trait.EventReceiver.html).

```rs
/// Sink of events. Collects them into an array.
struct EventSink {
    events: Vec<Event>,
}

/// Implement `on_event`, pushing into `self.events`.
impl EventReceiver for EventSink {
    fn on_event(&mut self, ev: Event) {
        self.events.push(ev);
    }
}

/// Load events from a yaml string.
fn str_to_events(yaml: &str) -> Vec<Event> {
    let mut sink = EventSink { events: Vec::new() };
    let mut parser = Parser::new_from_str(yaml);
    // Load events using our sink as the receiver.
    parser.load(&mut sink, true).unwrap();
    sink.events
}
```

## Specification Compliance

This implementation is fully compatible with the YAML 1.2 specification.
`saphyr-parser`) tests against (and passes) the [YAML test
suite](https://github.com/yaml/yaml-test-suite/).

## License

Sets of licences are available for each of the crates. Due to this project
being based on a fork of [chyh1990's
`yaml-rust`](https://github.com/chyh1990/yaml-rust), there are 2 licenses to be
included if using `saphyr` or `saphyr-parser`. Refer to the projects' READMEs
for details.

## Contribution

[Fork this repository](https://github.com/saphyr-rs/saphyr/fork) and
[Create a Pull Request on Github](https://github.com/saphyr-rs/saphyr/compare/master...saphyr-rs:saphyr:master).
You may need to click on "compare across forks" and select your fork's branch.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

## Links
### `saphyr`
* [saphyr source code repository](https://github.com/saphyr-rs/saphyr/tree/master/saphyr)
* [saphyr releases on crates.io](https://crates.io/crates/saphyr)
* [saphyr documentation on docs.rs](https://docs.rs/saphyr/latest/saphyr/)

### `saphyr-parser`
* [saphyr-parser source code repository](https://github.com/saphyr-rs/saphyr/tree/master/parser)
* [saphyr-parser releases on crates.io](https://crates.io/crates/saphyr-parser)
* [saphyr-parser documentation on docs.rs](https://docs.rs/saphyr-parser/latest/saphyr-parser/)

### Other links
* [yaml-test-suite](https://github.com/yaml/yaml-test-suite)
* [YAML 1.2 specification](https://yaml.org/spec/1.2.2/)
