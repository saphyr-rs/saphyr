use saphyr::{LoadableYamlNode, Mapping, Scalar, Yaml, YamlEmitter};

fn roundtrip(original: &Yaml) {
    let mut emitted = String::new();
    YamlEmitter::new(&mut emitted).dump(original).unwrap();

    let documents = Yaml::load_from_str(&emitted).unwrap();
    println!("emitted {emitted}");

    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0], *original);
}

#[allow(clippy::similar_names)]
fn roundtrip_multiline(original: &Yaml) {
    let mut emitted = String::new();
    let mut emitter = YamlEmitter::new(&mut emitted);
    emitter.multiline(true);
    emitter.dump(original).unwrap();

    println!("emitted {emitted}");
    let documents = Yaml::load_from_str(&emitted).unwrap();

    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0], *original);
}

fn assert_emits_yaml(input: &str, expected: &str) {
    let original = Yaml::load_from_str(input).unwrap();
    assert_eq!(original.len(), 1);

    let mut emitted = String::new();
    let mut emitter = YamlEmitter::new(&mut emitted);
    emitter.multiline(true);
    emitter.dump(&original[0]).unwrap();

    assert_eq!(emitted, expected);
    let reparsed = Yaml::load_from_str(&emitted).unwrap();
    assert_eq!(reparsed, original);
}

fn double_roundtrip(original: &str) {
    let parsed = Yaml::load_from_str(original).unwrap();

    let mut serialized = String::new();
    YamlEmitter::new(&mut serialized).dump(&parsed[0]).unwrap();

    let reparsed = Yaml::load_from_str(&serialized).unwrap();

    assert_eq!(parsed, reparsed);
}

#[test]
fn test_escape_character() {
    let y = Yaml::Value(Scalar::String("\x1b".into()));
    roundtrip(&y);
}

#[test]
fn test_colon_in_string() {
    let y = Yaml::Value(Scalar::String("x: %".into()));
    roundtrip(&y);
}

#[test]
fn test_numberlike_strings() {
    let docs = [
        r#"x: "1234""#,
        r#"x: "01234""#,
        r#""1234""#,
        r#""01234""#,
        r#"" 01234""#,
        r#""0x1234""#,
        r#"" 0x1234""#,
        r#""0o1234""#,
        r#""+.inf""#,
    ];

    for doc in &docs {
        roundtrip(&Yaml::Value(Scalar::String((*doc).into())));
        double_roundtrip(doc);
    }
}

/// Example from <https://github.com/chyh1990/yaml-rust/issues/133>
#[test]
fn test_issue133() {
    let doc = Yaml::load_from_str("\"0x123\"").unwrap().pop().unwrap();
    assert_eq!(doc, Yaml::Value(Scalar::String("0x123".into())));

    let mut out_str = String::new();
    YamlEmitter::new(&mut out_str).dump(&doc).unwrap();
    let doc2 = Yaml::load_from_str(&out_str).unwrap().pop().unwrap();
    assert_eq!(doc, doc2); // This failed because the type has changed to a number now
}

#[test]
fn test_newline() {
    let y = Yaml::Sequence(vec![Yaml::Value(Scalar::String("\n".into()))]);
    roundtrip(&y);
}

#[test]
fn test_crlf() {
    let y = Yaml::Sequence(vec![Yaml::Value(Scalar::String("\r\n".into()))]);
    roundtrip(&y);
}

#[test]
fn test_multiline_noline() {
    let y = Yaml::Sequence(vec![Yaml::Value(Scalar::String("a".into()))]);
    roundtrip_multiline(&y);
}

#[test]
fn test_multiline_inner_newline() {
    let y = Yaml::Sequence(vec![Yaml::Value(Scalar::String("a\nb".into()))]);
    roundtrip_multiline(&y);
}

#[test]
fn test_multiline_trailing_newline() {
    let y = Yaml::Sequence(vec![Yaml::Value(Scalar::String("a\n".into()))]);
    roundtrip_multiline(&y);
}

#[test]
fn test_multiline_leading_newline() {
    let y = Yaml::Sequence(vec![Yaml::Value(Scalar::String("\na".into()))]);
    roundtrip_multiline(&y);
}

#[test]
fn test_multiline_document_markers_root() {
    // At the root, lines starting with `---` or `...` must be indented, or they would be read as
    // document markers.
    for (value, expected) in [
        ("a\n...\nb", "---\n|-\n  a\n  ...\n  b"),
        ("a\n---\nb\n", "---\n|\n  a\n  ---\n  b"),
        ("a\n--- x\nb", "---\n|-\n  a\n  --- x\n  b"),
        ("a\n...\tx", "---\n|-\n  a\n  ...\tx"),
        ("a\n---x\n....", "---\n|-\na\n---x\n...."),
    ] {
        let scalar = Yaml::Value(Scalar::String(value.into()));
        let mut emitted = String::new();
        let mut emitter = YamlEmitter::new(&mut emitted);
        emitter.multiline(true);
        emitter.dump(&scalar).unwrap();
        assert_eq!(emitted, expected);
        roundtrip_multiline(&scalar);
    }
}

#[test]
fn test_multiline_key() {
    let mut mapping = Mapping::new();
    mapping.insert(
        Yaml::Value(Scalar::String("a\nb".into())),
        Yaml::Value(Scalar::String("c".into())),
    );
    mapping.insert(
        Yaml::Value(Scalar::String("d\n".into())),
        Yaml::Value(Scalar::String("e\nf".into())),
    );
    roundtrip_multiline(&Yaml::Mapping(mapping));
}

const MULTILINE_LEADING_WHITESPACE: [&str; 5] = [
    " First line with a leading space.\nSecond line at margin.",
    " First line with a leading space.\nSecond line at margin.\n",
    "\n First line\nSecond line",
    "  \nFirst line\nSecond line",
    "\tFirst line\nSecond line",
];
const EXPECTED_LEADING_WHITESPACE_VALUE: &str =
    " First line with a leading space.\nSecond line at margin.";

#[test]
fn test_multiline_leading_whitespace_root() {
    for value in MULTILINE_LEADING_WHITESPACE {
        let scalar = Yaml::Value(Scalar::String(value.into()));
        roundtrip_multiline(&scalar);
    }
}

#[test]
fn test_multiline_leading_whitespace_sequence() {
    for value in MULTILINE_LEADING_WHITESPACE {
        let scalar = Yaml::Value(Scalar::String(value.into()));
        roundtrip_multiline(&Yaml::Sequence(vec![scalar]));
    }
}

#[test]
fn test_multiline_leading_whitespace_mapping() {
    for value in MULTILINE_LEADING_WHITESPACE {
        let scalar = Yaml::Value(Scalar::String(value.into()));
        let mapping: Mapping = [(Yaml::Value(Scalar::String("key".into())), scalar)]
            .into_iter()
            .collect();
        roundtrip(&Yaml::Mapping(mapping.clone()));
        roundtrip_multiline(&Yaml::Mapping(mapping));
    }
}

#[test]
fn test_multiline_leading_whitespace_yaml_root() {
    let input = r#"---
|2-
   First line with a leading space.
  Second line at margin."#;
    let documents = Yaml::load_from_str(input).unwrap();
    assert_eq!(
        documents[0].as_str().unwrap(),
        EXPECTED_LEADING_WHITESPACE_VALUE
    );
    assert_emits_yaml(
        input,
        r#"---
|2-
   First line with a leading space.
  Second line at margin."#,
    );
}

#[test]
fn test_multiline_leading_whitespace_yaml_sequence() {
    let input = r#"---
- |2-
   First line with a leading space.
  Second line at margin."#;
    let documents = Yaml::load_from_str(input).unwrap();
    assert_eq!(
        documents[0][0].as_str().unwrap(),
        EXPECTED_LEADING_WHITESPACE_VALUE
    );
    assert_emits_yaml(
        input,
        r#"---
- |2-
   First line with a leading space.
  Second line at margin."#,
    );
}

#[test]
fn test_multiline_leading_whitespace_yaml_mapping() {
    let input = r#"---
key: |2-
   First line with a leading space.
  Second line at margin."#;
    let documents = Yaml::load_from_str(input).unwrap();
    assert_eq!(
        documents[0]["key"].as_str().unwrap(),
        EXPECTED_LEADING_WHITESPACE_VALUE
    );
    assert_emits_yaml(
        input,
        r#"---
key: |2-
   First line with a leading space.
  Second line at margin."#,
    );
}

#[test]
fn test_multiline_leading_whitespace_yaml_nested_mapping() {
    let input = r#"---
outer:
  key: |2-
     First line with a leading space.
    Second line at margin."#;
    let documents = Yaml::load_from_str(input).unwrap();
    assert_eq!(
        documents[0]["outer"]["key"].as_str().unwrap(),
        EXPECTED_LEADING_WHITESPACE_VALUE
    );
    assert_emits_yaml(
        input,
        r#"---
outer:
  key: |2-
     First line with a leading space.
    Second line at margin."#,
    );
}
