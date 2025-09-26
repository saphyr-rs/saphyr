use saphyr_serde::{de::from_str, ser::to_string};
use serde::{Deserialize, Serialize};

fn main() {
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct Address {
        street: String,
        state: String,
    }

    const ADDRESS_YAML_STR: &str = r#"street: Main Street
state: New Jersey
"#;
    // deserialize
    let result: Address = from_str(ADDRESS_YAML_STR).expect("Should deserialize");

    println!("{:?}", result);

    assert_eq!(
        result,
        Address {
            street: String::from("Main Street"),
            state: String::from("New Jersey")
        }
    );

    // serialize
    let output = to_string(&result).unwrap();
    assert_eq!(output, ADDRESS_YAML_STR);
}
