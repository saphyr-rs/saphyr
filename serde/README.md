# Serde Implementation using Saphyr Yaml Parser

Implements serialization and deserialization of YAML using the
[saphyr-parser](https://github.com/saphyr-rs/saphyr/tree/master/parser)
library.

Example:

```rust
#[derive(Deserialize, PartialEq, Eq, Debug)]
struct Address {
    street: String,
    state: String,
}

const ADDRESS_YAML_STR: &str = r###"
street: Main Street
state: New Jersey
"###;

// deserialize
let result: Address = from_str(ADDRESS_YAML_STR).expect("Should deserialize");

assert_eq!(
    result,
    Address {
        street: String::from("Main Street"),
        state: String::from("New Jersey")
    }
);

// serialize
let output = to_string(&result).unwrap();
assert_eq!(output, String::from(ADDRESS_YAML_STR).trim());
```

## TODO

- Input from / Output to files
- Better Error Handling
