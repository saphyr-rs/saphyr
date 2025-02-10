use saphyr::{MarkedYaml, Scalar, Yaml, YamlData};

fn get_yaml_mapping() -> Yaml<'static> {
    let s = "
foo: 1
bar: 2
baz: 3";
    let mut docs = Yaml::load_from_str(s).unwrap();
    docs.remove(0)
}

fn get_marked_yaml_mapping() -> MarkedYaml<'static> {
    let s = "
foo: 1
bar: 2
baz: 3";
    let mut docs = MarkedYaml::load_from_str(s).unwrap();
    docs.remove(0)
}

fn get_yaml_sequence() -> Yaml<'static> {
    let s = "
- foo
- bar
- baz";
    let mut docs = Yaml::load_from_str(s).unwrap();
    docs.remove(0)
}

fn get_marked_yaml_sequence() -> MarkedYaml<'static> {
    let s = "
- foo
- bar
- baz";
    let mut docs = MarkedYaml::load_from_str(s).unwrap();
    docs.remove(0)
}

#[test]
fn yaml_index_str() {
    let doc = get_yaml_mapping();
    assert_eq!(doc["foo"], Yaml::Value(Scalar::Integer(1)));
    assert_eq!(doc["bar"], Yaml::Value(Scalar::Integer(2)));
    assert_eq!(doc["baz"], Yaml::Value(Scalar::Integer(3)));
}

#[test]
#[should_panic(expected = "Key 'oob' not found in YAML mapping")]
fn yaml_index_str_oob() {
    let doc = get_yaml_mapping();
    assert_eq!(doc["oob"], Yaml::BadValue);
}

#[test]
fn yaml_index_str_mut() {
    let mut doc = get_yaml_mapping();
    doc["foo"] = Yaml::Value(Scalar::Integer(4));
    assert_eq!(doc["foo"], Yaml::Value(Scalar::Integer(4)));
}

#[test]
#[should_panic(expected = "Key 'oob' not found in YAML mapping")]
fn yaml_index_str_oob_mut() {
    let mut doc = get_yaml_mapping();
    doc["oob"] = Yaml::Value(Scalar::Integer(4));
}

#[test]
fn marked_yaml_index_str() {
    let doc = get_marked_yaml_mapping();
    assert_eq!(doc.data["foo"].data, YamlData::Value(Scalar::Integer(1)));
    assert_eq!(doc.data["bar"].data, YamlData::Value(Scalar::Integer(2)));
    assert_eq!(doc.data["baz"].data, YamlData::Value(Scalar::Integer(3)));
}

#[test]
#[should_panic(expected = "Key 'oob' not found in YamlData mapping")]
fn marked_yaml_index_str_oob() {
    let doc = get_marked_yaml_mapping();
    let _ = doc.data["oob"];
}

#[test]
fn marked_yaml_index_str_mut() {
    let mut doc = get_marked_yaml_mapping();
    doc.data["foo"] = YamlData::Value(Scalar::Integer(4)).into();
    assert_eq!(doc.data["foo"].data, YamlData::Value(Scalar::Integer(4)));
}

#[test]
#[should_panic(expected = "Key 'oob' not found in YamlData mapping")]
fn marked_yaml_index_str_oob_mut() {
    let mut doc = get_marked_yaml_mapping();
    doc.data["oob"].data = YamlData::Value(Scalar::Integer(4));
}

#[test]
fn yaml_index_integer() {
    let doc = get_yaml_sequence();
    assert_eq!(doc[0], Yaml::Value(Scalar::String("foo".into())));
    assert_eq!(doc[1], Yaml::Value(Scalar::String("bar".into())));
    assert_eq!(doc[2], Yaml::Value(Scalar::String("baz".into())));
}

#[test]
#[should_panic(expected = "Index 12 out of bounds in YAML sequence")]
fn yaml_index_integer_oob() {
    let doc = get_yaml_sequence();
    assert_eq!(doc[12], Yaml::BadValue);
}

#[test]
fn yaml_index_integer_mut() {
    let mut doc = get_yaml_sequence();
    doc[0] = Yaml::Value(Scalar::Integer(4));
    assert_eq!(doc[0], Yaml::Value(Scalar::Integer(4)));
}

#[test]
#[should_panic(expected = "Index 12 out of bounds in YAML sequence")]
fn yaml_index_integer_oob_mut() {
    let mut doc = get_yaml_sequence();
    doc[12] = Yaml::Value(Scalar::Integer(4));
}

#[test]
fn marked_yaml_index_integer() {
    let doc = get_marked_yaml_sequence();
    assert_eq!(
        doc.data[0].data,
        YamlData::Value(Scalar::String("foo".into()))
    );
    assert_eq!(
        doc.data[1].data,
        YamlData::Value(Scalar::String("bar".into()))
    );
    assert_eq!(
        doc.data[2].data,
        YamlData::Value(Scalar::String("baz".into()))
    );
}

#[test]
#[should_panic(expected = "Index 12 out of bounds in YamlData sequence")]
fn marked_yaml_index_integer_oob() {
    let doc = get_marked_yaml_sequence();
    let _ = doc.data[12];
}

#[test]
fn marked_yaml_index_integer_mut() {
    let mut doc = get_marked_yaml_sequence();
    doc.data[0] = YamlData::Value(Scalar::Integer(4)).into();
    assert_eq!(doc.data[0].data, YamlData::Value(Scalar::Integer(4)));
}

#[test]
#[should_panic(expected = "Index 12 out of bounds in YamlData sequence")]
fn marked_yaml_index_integer_oob_mut() {
    let mut doc = get_marked_yaml_sequence();
    doc.data[12].data = YamlData::Value(Scalar::Integer(4));
}

#[test]
#[should_panic(expected = "Attempt to index YAML with 'oob' but it's not a mapping")]
fn yaml_index_str_wrong_variant() {
    let _ = Yaml::Value(Scalar::Integer(3))["oob"];
}

#[test]
#[should_panic(expected = "Attempt to index YAML with 12 but it's not a mapping nor a sequence")]
fn yaml_index_integer_wrong_variant() {
    let _ = Yaml::Value(Scalar::Integer(3))[12];
}

#[test]
#[should_panic(expected = "Attempt to index YamlData with 'oob' but it's not a mapping")]
fn marked_yaml_index_str_wrong_variant() {
    let node: MarkedYaml<'_> = YamlData::Value(Scalar::Integer(3)).into();
    let _ = node.data["oob"];
}

#[test]
#[should_panic(
    expected = "Attempt to index YamlData with 12 but it's not a mapping nor a sequence"
)]
fn marked_yaml_index_integer_wrong_variant() {
    let node: MarkedYaml<'_> = YamlData::Value(Scalar::Integer(3)).into();
    let _ = node.data[12];
}
