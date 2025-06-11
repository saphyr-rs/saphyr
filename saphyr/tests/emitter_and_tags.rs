use saphyr::{LoadableYamlNode, Yaml, YamlEmitter};

fn roundtrip_from_string(input: &str) {
    let yaml = Yaml::load_from_str(input).unwrap();
    let mut emitted = String::new();
    YamlEmitter::new(&mut emitted).dump(&yaml[0]).unwrap();
    println!("{emitted}");
    let roundtripped = Yaml::load_from_str(&emitted).unwrap();
    assert_eq!(yaml, roundtripped);
}

#[test]
fn tagged_mapping_in_sequence() {
    let s = r"
foo:
  - !tag
    name: Alice
    age: 5
    ";

    roundtrip_from_string(s);
}

#[test]
fn tagged_key_in_mapping_in_sequence() {
    let s = r"
foo:
  - !tag name: Alice
    age: 5
    ";

    roundtrip_from_string(s);

    let s = r"
foo:
  - !!str name: Alice
    age: 5
    ";
    roundtrip_from_string(s);
}

#[test]
fn tagged_sequence() {
    let s = r"
foo: !tag
  - name: Alice
    age: 5
    ";

    roundtrip_from_string(s);
}

#[test]
fn tagged_global_mapping() {
    let s = r"
!tag
foo:
  - name: Alice
    age: 5
    ";

    roundtrip_from_string(s);
}

#[test]
fn tagged_key_in_global_mapping() {
    let s = r"
!tag foo:
  - name: Alice
    age: 5
    ";

    roundtrip_from_string(s);
}

#[test]
fn tagged_empty_sequence() {
    let s = r"
foo: !tag []
    ";

    roundtrip_from_string(s);
}

#[test]
fn tagged_empty_mapping() {
    let s = r"
foo: !tag {}
    ";

    roundtrip_from_string(s);
}
