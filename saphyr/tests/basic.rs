#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::float_cmp)]

use saphyr::{LoadableYamlNode, Scalar, Yaml, YamlEmitter};

#[test]
fn test_api() {
    let s = "
# from yaml-cpp example
- name: Ogre
  position: [0, 5, 0]
  powers:
    - name: Club
      damage: 10
    - name: Fist
      damage: 8
- name: Dragon
  position: [1, 0, 10]
  powers:
    - name: Fire Breath
      damage: 25
    - name: Claws
      damage: 15
- name: Wizard
  position: [5, -3, 0]
  powers:
    - name: Acid Rain
      damage: 50
    - name: Staff
      damage: 3
";
    let docs = Yaml::load_from_str(s).unwrap();
    let doc = &docs[0];

    assert_eq!(doc[0]["name"].as_str().unwrap(), "Ogre");

    let mut writer = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.dump(doc).unwrap();
    }

    assert!(!writer.is_empty());
}

#[test]
fn test_coerce() {
    let s = "---
a: 1
b: 2.2
c: [1, 2]
";
    let out = Yaml::load_from_str(s).unwrap();
    let doc = &out[0];
    assert_eq!(doc["a"].as_integer().unwrap(), 1i64);
    assert_eq!(doc["b"].as_floating_point().unwrap(), 2.2f64);
    assert_eq!(doc["c"][1].as_integer().unwrap(), 2i64);
    assert!(!doc.contains_mapping_key("d"));
}

#[test]
fn test_anchor() {
    let s = "
a1: &DEFAULT
    b1: 4
    b2: d
a2: *DEFAULT
";
    let out = Yaml::load_from_str(s).unwrap();
    let doc = &out[0];
    assert_eq!(doc["a2"]["b1"].as_integer().unwrap(), 4);
}

#[test]
fn test_bad_anchor() {
    let s = "
a1: &DEFAULT
    b1: 4
    b2: *DEFAULT
";
    let out = Yaml::load_from_str(s).unwrap();
    let doc = &out[0];
    assert_eq!(doc["a1"]["b2"], Yaml::BadValue);
}

#[test]
fn test_plain_datatype() {
    let s = "
- 'string'
- \"string\"
- string
- 123
- -321
- 1.23
- -1e4
- ~
- null
- true
- false
- !!str 0
- !!int 100
- !!float 2
- !!null ~
- !!bool true
- !!bool false
- 0xFF
# bad values
- !!int string
- !!float string
- !!bool null
- !!null val
- 0o77
- [ 0xF, 0xF ]
- +12345
- [ true, false ]
";
    let out = Yaml::load_from_str(s).unwrap();
    let doc = &out[0];

    assert_eq!(doc[0].as_str().unwrap(), "string");
    assert_eq!(doc[1].as_str().unwrap(), "string");
    assert_eq!(doc[2].as_str().unwrap(), "string");
    assert_eq!(doc[3].as_integer().unwrap(), 123);
    assert_eq!(doc[4].as_integer().unwrap(), -321);
    assert_eq!(doc[5].as_floating_point().unwrap(), 1.23);
    assert_eq!(doc[6].as_floating_point().unwrap(), -1e4);
    assert!(doc[7].is_null());
    assert!(doc[8].is_null());
    assert_eq!(doc[9].as_bool().unwrap(), true);
    assert_eq!(doc[10].as_bool().unwrap(), false);
    assert_eq!(doc[11].as_str().unwrap(), "0");
    assert_eq!(doc[12].as_integer().unwrap(), 100);
    assert_eq!(doc[13].as_floating_point().unwrap(), 2.0);
    assert!(doc[14].is_null());
    assert_eq!(doc[15].as_bool().unwrap(), true);
    assert_eq!(doc[16].as_bool().unwrap(), false);
    assert_eq!(doc[17].as_integer().unwrap(), 255);
    assert!(doc[18].is_badvalue());
    assert!(doc[19].is_badvalue());
    assert!(doc[20].is_badvalue());
    assert!(doc[21].is_badvalue());
    assert_eq!(doc[22].as_integer().unwrap(), 63);
    assert_eq!(doc[23][0].as_integer().unwrap(), 15);
    assert_eq!(doc[23][1].as_integer().unwrap(), 15);
    assert_eq!(doc[24].as_integer().unwrap(), 12345);
    assert!(doc[25][0].as_bool().unwrap());
    assert!(!doc[25][1].as_bool().unwrap());
}

#[test]
fn test_plain_datatype_with_into_methods() {
    let s = "
- 'string'
- \"string\"
- string
- 123
- -321
- 1.23
- -1e4
- true
- false
- !!str 0
- !!int 100
- !!float 2
- !!bool true
- !!bool false
- 0xFF
- 0o77
- +12345
- -.INF
- .NAN
- !!float .INF
";
    let mut out = Yaml::load_from_str(s).unwrap().into_iter();
    let mut doc = out.next().unwrap().into_iter();

    assert_eq!(doc.next().unwrap().into_string().unwrap(), "string");
    assert_eq!(doc.next().unwrap().into_string().unwrap(), "string");
    assert_eq!(doc.next().unwrap().into_string().unwrap(), "string");
    assert_eq!(doc.next().unwrap().into_integer().unwrap(), 123);
    assert_eq!(doc.next().unwrap().into_integer().unwrap(), -321);
    assert_eq!(doc.next().unwrap().into_floating_point().unwrap(), 1.23);
    assert_eq!(doc.next().unwrap().into_floating_point().unwrap(), -1e4);
    assert_eq!(doc.next().unwrap().into_bool().unwrap(), true);
    assert_eq!(doc.next().unwrap().into_bool().unwrap(), false);
    assert_eq!(doc.next().unwrap().into_string().unwrap(), "0");
    assert_eq!(doc.next().unwrap().into_integer().unwrap(), 100);
    assert_eq!(doc.next().unwrap().into_floating_point().unwrap(), 2.0);
    assert_eq!(doc.next().unwrap().into_bool().unwrap(), true);
    assert_eq!(doc.next().unwrap().into_bool().unwrap(), false);
    assert_eq!(doc.next().unwrap().into_integer().unwrap(), 255);
    assert_eq!(doc.next().unwrap().into_integer().unwrap(), 63);
    assert_eq!(doc.next().unwrap().into_integer().unwrap(), 12345);
    assert_eq!(
        doc.next().unwrap().into_floating_point().unwrap(),
        f64::NEG_INFINITY
    );
    assert!(doc.next().unwrap().into_floating_point().is_some());
    assert_eq!(
        doc.next().unwrap().into_floating_point().unwrap(),
        f64::INFINITY
    );
}

#[test]
fn test_mapping_order() {
    let s = "---
b: ~
a: ~
c: ~
";
    let out = Yaml::load_from_str(s).unwrap();
    let first = out.into_iter().next().unwrap();
    let mut iter = first.into_mapping().unwrap().into_iter();
    assert_eq!(
        iter.next(),
        Some((
            Yaml::Value(Scalar::String("b".into())),
            Yaml::Value(Scalar::Null)
        ))
    );
    assert_eq!(
        iter.next(),
        Some((
            Yaml::Value(Scalar::String("a".into())),
            Yaml::Value(Scalar::Null)
        ))
    );
    assert_eq!(
        iter.next(),
        Some((
            Yaml::Value(Scalar::String("c".into())),
            Yaml::Value(Scalar::Null)
        ))
    );
    assert_eq!(None, iter.next());
}

#[test]
fn test_integer_key() {
    let s = "
0:
    important: true
1:
    important: false
";
    let out = Yaml::load_from_str(s).unwrap();
    let first = out.into_iter().next().unwrap();
    assert_eq!(first[0]["important"].as_bool().unwrap(), true);
}

#[test]
fn test_nominal_float_parse() {
    use std::fmt::Write;

    // back test for
    // https://github.com/Ethiraric/yaml-rust2/issues/50

    // Test cases derived from https://doc.rust-lang.org/std/primitive.f64.html#impl-FromStr-for-f64

    // Generates a document like so:
    // ```yaml
    // - +nan
    // - -nan
    // - nan
    // - +NAN
    // ...
    // ```
    // Every single one of these values should be parsed as a string in yaml,
    // but would be parsed as a float according to rust. This test verifies they
    // all end up parsed as strings.
    let raw = &["nan", "NAN", "NaN", "inf", "infinity", "Infinity"]
        .iter()
        .fold(String::new(), |mut output, base| {
            write!(output, "- +{base}\n- -{base}\n- {base}\n").unwrap();
            output
        });

    println!("parsing {raw}");

    let doc = Yaml::load_from_str(raw).expect("could not parse document");
    let doc = doc.into_iter().next().unwrap();

    assert!(doc.is_sequence());

    for it in doc {
        assert!(it.as_str().is_some());
    }
}
