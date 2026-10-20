use std::borrow::Cow;

use saphyr::{LoadableYamlNode, ScalarStyle, Yaml, YamlEmitter};

#[allow(clippy::similar_names)]
#[test]
fn test_emit_simple() {
    let s = "
# comment
a0 bb: val
a1:
    b1: 4
    b2: d
a2: 4 # i'm comment
a3: [1, 2, 3]
a4:
    - [a1, a2]
    - 2
";

    let docs = Yaml::load_from_str(s).unwrap();
    let doc = &docs[0];
    let mut writer = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.dump(doc).unwrap();
    }
    println!("original:\n{s}");
    println!("emitted:\n{writer}");
    let docs_new = match Yaml::load_from_str(&writer) {
        Ok(y) => y,
        Err(e) => panic!("{}", e),
    };
    let doc_new = &docs_new[0];

    assert_eq!(doc, doc_new);
}

#[test]
fn test_emit_complex() {
    let s = r"
catalogue:
  product1: &coffee   { name: Coffee,    price: 2.5  ,  unit: 1l  }
  product2: &cookies  { name: Cookies!,  price: 3.40 ,  unit: 400g}

products:
  *coffee :
    amount: 4
  *cookies :
    amount: 4
  [1,2,3,4]:
    array key
  2.4:
    real key
  true:
    bool key
  {}:
    empty hash key
            ";
    let docs = Yaml::load_from_str(s).unwrap();
    let doc = &docs[0];
    let mut writer = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.dump(doc).unwrap();
    }
    let docs_new = match Yaml::load_from_str(&writer) {
        Ok(y) => y,
        Err(e) => panic!("{}", e),
    };
    let new_doc = &docs_new[0];
    assert_eq!(doc, new_doc);
}

#[test]
fn test_emit_avoid_quotes() {
    let s = r#"---
a7: 你好
boolean: "true"
boolean2: "false"
date: 2014-12-31
empty_string: ""
empty_string1: " "
empty_string2: "    a"
empty_string3: "    a "
exp: "12e7"
field: ":"
field2: "{"
field3: "\\"
field4: "\n"
field5: "can't avoid quote"
float: "2.6"
int: "4"
nullable: "null"
nullable2: "~"
products:
  "*coffee":
    amount: 4
  "*cookies":
    amount: 4
  ".milk":
    amount: 1
  "2.4": real key
  "[1,2,3,4]": array key
  "true": bool key
  "{}": empty hash key
x: test
y: avoid quoting here
z: string with spaces"#;

    let docs = Yaml::load_from_str(s).unwrap();
    let doc = &docs[0];
    let mut writer = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.dump(doc).unwrap();
    }

    assert_eq!(s, writer, "actual:\n\n{writer}\n");
}

#[test]
fn emit_quoted_bools() {
    let input = r#"---
string0: yes
string1: no
string2: "true"
string3: "false"
string4: "~"
null0: ~
[true, True, TRUE, false, False, FALSE]: real_bools
[y,Y,yes,Yes,YES,n,N,no,No,NO,on,On,ON,off,Off,OFF]: false_bools
bool0: true
bool1: false"#;
    let expected = r#"---
string0: "yes"
string1: "no"
string2: "true"
string3: "false"
string4: "~"
null0: ~
? - true
  - true
  - true
  - false
  - false
  - false
: real_bools
? - y
  - Y
  - "yes"
  - "Yes"
  - "YES"
  - n
  - N
  - "no"
  - "No"
  - "NO"
  - "on"
  - "On"
  - "ON"
  - "off"
  - "Off"
  - "OFF"
: false_bools
bool0: true
bool1: false"#;

    let docs = Yaml::load_from_str(input).unwrap();
    let doc = &docs[0];
    let mut writer = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.dump(doc).unwrap();
    }

    assert_eq!(
        expected, writer,
        "expected:\n{expected}\nactual:\n{writer}\n",
    );
}

/// Ensure that whole floats such as 100.0 emit as a float (not int).
#[test]
fn test_emit_whole_floats() {
    let input = r#"---
value: 100.0"#;
    let expected = r#"---
value: 100.0"#;
    let docs = Yaml::load_from_str(input).unwrap();
    let doc = &docs[0];
    let mut writer = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.dump(doc).unwrap();
    }
    assert_eq!(
        expected, writer,
        "expected:\n{expected}\nactual:\n{writer}\n",
    );
}

#[test]
fn test_empty_and_nested() {
    test_empty_and_nested_flag(false);
}

#[test]
fn test_empty_and_nested_compact() {
    test_empty_and_nested_flag(true);
}

fn test_empty_and_nested_flag(compact: bool) {
    let s = if compact {
        r"---
a:
  b:
    c: hello
  d: {}
e:
  - f
  - g
  - h: []"
    } else {
        r"---
a:
  b:
    c: hello
  d: {}
e:
  - f
  - g
  -
    h: []"
    };

    let docs = Yaml::load_from_str(s).unwrap();
    let doc = &docs[0];
    let mut writer = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.compact(compact);
        emitter.dump(doc).unwrap();
    }

    assert_eq!(s, writer);
}

#[test]
fn test_nested_sequences() {
    let s = r"---
a:
  - b
  - - c
    - d
    - - e
      - f";

    let docs = Yaml::load_from_str(s).unwrap();
    let doc = &docs[0];
    let mut writer = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.dump(doc).unwrap();
    }
    println!("original:\n{s}");
    println!("emitted:\n{writer}");

    assert_eq!(s, writer);
}

#[test]
fn test_deeply_nested_sequences() {
    let s = r"---
a:
  - b
  - - c
    - d
    - - e
      - - f
      - - e";

    let docs = Yaml::load_from_str(s).unwrap();
    let doc = &docs[0];
    let mut writer = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.dump(doc).unwrap();
    }
    println!("original:\n{s}");
    println!("emitted:\n{writer}");

    assert_eq!(s, writer);
}

#[test]
fn test_nested_mappings() {
    let s = r"---
a:
  b:
    c:
      d:
        e: f";

    let docs = Yaml::load_from_str(s).unwrap();
    let doc = &docs[0];
    let mut writer = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.dump(doc).unwrap();
    }
    println!("original:\n{s}");
    println!("emitted:\n{writer}");

    assert_eq!(s, writer);
}

#[test]
fn test_string_values_default_to_existing_output() {
    let mapping = Yaml::load_from_str("---\nanswer: \"value\"").unwrap();
    let mut mapping_output = String::new();
    YamlEmitter::new(&mut mapping_output)
        .dump(&mapping[0])
        .unwrap();
    println!("quoted mapping value baseline: {mapping_output:?}");
    assert_eq!(mapping_output, "---\nanswer: value");
    assert_eq!(Yaml::load_from_str(&mapping_output).unwrap()[0], mapping[0]);

    let sequence = Yaml::load_from_str("---\n- \"first\"\n- \"second\"").unwrap();
    let mut sequence_output = String::new();
    YamlEmitter::new(&mut sequence_output)
        .dump(&sequence[0])
        .unwrap();
    println!("quoted sequence values baseline: {sequence_output:?}");
    assert_eq!(sequence_output, "---\n- first\n- second");
    assert_eq!(
        Yaml::load_from_str(&sequence_output).unwrap()[0],
        sequence[0]
    );
}

fn emit_with_options(doc: &Yaml, quote_string_values: bool, multiline_strings: bool) -> String {
    let mut output = String::new();
    let mut emitter = YamlEmitter::new(&mut output);
    emitter.quote_string_values(quote_string_values);
    emitter.multiline_strings(multiline_strings);
    emitter.dump(doc).unwrap();
    output
}

#[test]
fn quote_string_values_applies_to_values_but_not_mapping_keys() {
    let source = r#"---
root: "plain value"
items:
  - "first"
  - "second"
nested:
  child: "inner"
? ["complex", {inside: "key value"}]
: "complex value""#;
    let docs = Yaml::load_from_str(source).unwrap();
    let output = emit_with_options(&docs[0], true, false);
    println!("quoted semantic values: {output:?}");

    let reparsed = Yaml::load_from_str(&output).unwrap();
    assert_eq!(reparsed[0], docs[0]);
    assert!(output.contains("root: \"plain value\""));
    assert!(output.contains("\"first\""));
    assert!(output.contains("\"inner\""));
    assert!(output.contains("inside: key value"));
    assert!(output.contains("\"complex value\""));
}

#[test]
fn quote_string_values_handles_roots_tags_and_explicit_representations() {
    let root = Yaml::value_from_str("root string");
    assert_eq!(
        emit_with_options(&root, true, false),
        "---\n\"root string\""
    );

    let owned = Yaml::value_from_cow(Cow::Owned(String::from("owned string")));
    assert_eq!(
        emit_with_options(&owned, true, false),
        "---\n\"owned string\""
    );

    let tagged = Yaml::load_from_str("---\n!example tagged string").unwrap();
    let tagged_output = emit_with_options(&tagged[0], true, false);
    println!("quoted tagged string: {tagged_output:?}");
    assert!(tagged_output.contains("\"tagged string\""));
    assert_eq!(Yaml::load_from_str(&tagged_output).unwrap()[0], tagged[0]);

    let representation = Yaml::Representation(
        Cow::Borrowed("explicit style"),
        ScalarStyle::SingleQuoted,
        None,
    );
    assert_eq!(
        emit_with_options(&representation, true, false),
        "---\n'explicit style'"
    );
}

#[test]
fn quote_string_values_overrides_multiline_style_and_preserves_other_scalars() {
    let multiline = Yaml::load_from_str("---\ntext: \"first\\nsecond\"").unwrap();
    let output = emit_with_options(&multiline[0], true, true);
    assert_eq!(output, "---\ntext: \"first\\nsecond\"");
    assert_eq!(Yaml::load_from_str(&output).unwrap()[0], multiline[0]);

    let scalars =
        Yaml::load_from_str("---\nboolean: true\nfloat: 2.5\ninteger: 7\nnull: ~").unwrap();
    assert_eq!(
        emit_with_options(&scalars[0], true, false),
        emit_with_options(&scalars[0], false, false)
    );
    let alias = Yaml::Alias(1);
    assert_eq!(
        emit_with_options(&alias, true, false),
        emit_with_options(&alias, false, false)
    );
}

#[test]
fn quote_string_values_can_be_toggled_and_reports_its_state() {
    let document = Yaml::value_from_str("plain value");
    let mut output = String::new();
    let default_end;
    let quoted_end;
    {
        let mut emitter = YamlEmitter::new(&mut output);

        assert!(!emitter.is_quote_string_values());
        emitter.dump(&document).unwrap();
        default_end = "---\nplain value".len();

        emitter.quote_string_values(true);
        assert!(emitter.is_quote_string_values());
        emitter.dump(&document).unwrap();
        quoted_end = default_end + "---\n\"plain value\"".len();

        emitter.quote_string_values(false);
        assert!(!emitter.is_quote_string_values());
        emitter.dump(&document).unwrap();
    }

    assert_eq!(&output[..default_end], "---\nplain value");
    assert_eq!(&output[default_end..quoted_end], "---\n\"plain value\"");
    assert_eq!(&output[quoted_end..], "---\nplain value");
}
