# saphyr-serde

[Serde](https://serde.rs) support for [`saphyr`](https://crates.io/crates/saphyr), a YAML 1.2
library.

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Config {
    name: String,
    ports: Vec<u16>,
}

let config: Config = saphyr_serde::from_str("name: web\nports: [80, 443]").unwrap();
let yaml = saphyr_serde::to_string(&config).unwrap();
```

## Design

- **Type-driven scalars.** `NO`, `1.0` and `null` are strings when a `String` is expected, and
  `yes` is not a boolean. Scalars are resolved by the YAML 1.2 Core Schema only when the target
  type needs it, and quoted scalars are always strings. Serialization quotes the strings that
  would be read as something else.
- **Safe on untrusted input.** Depth of nesting, number of nodes and the expansion of aliases
  ("billion laughs") are limited (see `Options`), and are checked on the whole document before it
  is deserialized. Duplicate keys are rejected. Nothing in a document can construct anything but
  the types you ask for.
- **Aliases** are supported. Merge keys (`<<`) are YAML 1.1, not 1.2, and are not: `<<` is a
  key like any other.
- **Enums** are externally tagged (`Variant` or `Variant: content`); `!Variant content` is
  accepted when deserializing.
- Serialization is streamed: values are written as they are serialized, with
  `saphyr::YamlEventEmitter`, and memory use does not depend on their size.
- Multi-document streams are read through `Deserializer`, an iterator of documents.

## Limitations

- Every document is loaded in memory before it is deserialized.
- `!!binary`, `!!set`, `!!omap` and `!!timestamp` are not supported, and are rejected rather than
  read as something else.
- Only scalar keys are checked for duplicates.
- Catching duplicate keys when serializing needs to remember the keys of each mapping that is
  open (`Options::allow_duplicate_keys` turns it off).
- The emitter of `saphyr` writes U+0085, U+2028 and U+2029 unescaped, as YAML 1.2 allows. YAML 1.1
  parsers read them as line breaks.
