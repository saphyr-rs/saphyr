`saphyr-serde`
==============

Rust library for using the [`serde`] serialization framework with data in [YAML]
file format.

This project is a continuation of [dtolnay]'s [`serde-yaml`]. The backend has
been changed from [`unsafe-libyaml`] to [`saphyr-parser`].

## Dependency

```sh
cargo add serde
cargo add saphyr-serde
```

Release notes are available under [GitHub releases].


## Using `saphyr-serde`

[API documentation is available in rustdoc form][docs.rs] but the general idea
is:

```rust
use std::collections::BTreeMap;

fn main() -> Result<(), saphyr_serde::Error> {
    // You have some type.
    let mut map = BTreeMap::new();
    map.insert("x".to_string(), 1.0);
    map.insert("y".to_string(), 2.0);

    // Serialize it to a YAML string.
    let yaml = saphyr_serde::to_string(&map)?;
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");

    // Deserialize it back to a Rust type.
    let deserialized_map: BTreeMap<String, f64> = saphyr_serde::from_str(&yaml)?;
    assert_eq!(map, deserialized_map);
    Ok(())
}
```

It can also be used with Serde's derive macros to handle structs and enums
defined in your program.

```sh
cargo add serde --features derive
cargo add saphyr-serde
```

Structs serialize in the obvious way:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Point {
    x: f64,
    y: f64,
}

fn main() -> Result<(), saphyr_serde::Error> {
    let point = Point { x: 1.0, y: 2.0 };

    let yaml = saphyr_serde::to_string(&point)?;
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");

    let deserialized_point: Point = saphyr_serde::from_str(&yaml)?;
    assert_eq!(point, deserialized_point);
    Ok(())
}
```

Enums serialize using YAML's `!tag` syntax to identify the variant name.

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Enum {
    Unit,
    Newtype(usize),
    Tuple(usize, usize, usize),
    Struct { x: f64, y: f64 },
}

fn main() -> Result<(), saphyr_serde::Error> {
    let yaml = "
        - !Newtype 1
        - !Tuple [0, 0, 0]
        - !Struct {x: 1.0, y: 2.0}
    ";
    let values: Vec<Enum> = saphyr_serde::from_str(yaml).unwrap();
    assert_eq!(values[0], Enum::Newtype(1));
    assert_eq!(values[1], Enum::Tuple(0, 0, 0));
    assert_eq!(values[2], Enum::Struct { x: 1.0, y: 2.0 });

    // The last two in YAML's block style instead:
    let yaml = "
        - !Tuple
          - 0
          - 0
          - 0
        - !Struct
          x: 1.0
          y: 2.0
    ";
    let values: Vec<Enum> = saphyr_serde::from_str(yaml).unwrap();
    assert_eq!(values[0], Enum::Tuple(0, 0, 0));
    assert_eq!(values[1], Enum::Struct { x: 1.0, y: 2.0 });

    // Variants with no data can be written using !Tag or just the string name.
    let yaml = "
        - Unit  # serialization produces this one
        - !Unit
    ";
    let values: Vec<Enum> = saphyr_serde::from_str(yaml).unwrap();
    assert_eq!(values[0], Enum::Unit);
    assert_eq!(values[1], Enum::Unit);

    Ok(())
}
```

<br>

#### License

<sup>
Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>

[`serde`]: https://github.com/serde-rs/serde
[YAML]: https://yaml.org/
[dtolnay]: https://github.com/dtolnay
[`serde-yaml`]: https://github.com/dtolnay/serde-yaml
[`unsafe-libyaml`]: https://github.com/dtolnay/unsafe-libyaml
[`saphyr-parser`]: https://github.com/saphyr-rs/saphyr-parser
[GitHub releases]: https://github.com/saphyr-rs/saphyr-serde/releases
[docs.rs]: https://docs.rs/saphyr-serde
