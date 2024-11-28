use saphyr::{Yaml, YamlEmitter};

/// Test in sequence the parser, emitter and parser with the given input.
///
/// 1. Pass the input through the loader and build a YAML object from it.
/// 2. Pass the newly created YAML object through the emitter.
/// 3. Pass the emitted string through the loader and build another YAML object from it.
/// 4. Assert that the YAML objects from 1. and 3. are the same.
/// 5. Return the string from 3. so the caller can ensure its formatting.
///
/// The assertion done in this function is purely on the contents of the YAML objects and not on
/// its presentation.
///
/// This function additionally prints to stdout the input string and the resulting string from 2..
///
/// The configuration function `config` allows the caller to potentially change some settings in
/// the emitter prior to emitting.
fn raw_roundtrip<Config: FnOnce(&mut YamlEmitter)>(input: &str, config: Config) -> String {
    let original_docs = Yaml::load_from_str(input).unwrap();
    let original_doc = &original_docs[0];
    let mut emitted_string = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut emitted_string);
        config(&mut emitter);
        emitter.dump(original_doc).unwrap();
    }
    println!("original:\n{input}");
    println!("emitted:\n{emitted_string}");

    let emitted_docs = Yaml::load_from_str(&emitted_string).unwrap();
    assert_eq!(original_docs, emitted_docs);

    emitted_string
}

/// [`raw_roundtrip`] with default configuration
fn roundtrip(input: &str) -> String {
    raw_roundtrip(input, |_| {})
}

/// Like [`roundtrip`] but with the [compact flag] disabled.
///
/// [compact flag]: `YamlEmitter::compact`
fn roundtrip_not_compact(input: &str) -> String {
    raw_roundtrip(input, |emitter| emitter.compact(false))
}

/// Like [`roundtrip`] but with the [multiline strings flag] enabled.
///
/// [multiline strings flag]: `YamlEmitter::multiline_strings`
fn roundtrip_multiline(input: &str) -> String {
    raw_roundtrip(input, |emitter| emitter.multiline_strings(true))
}

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

    roundtrip(s);
}

#[test]
fn test_emit_complex() {
    let s = r"
catalogue:
  product: &coffee   { name: Coffee,    price: 2.5  ,  unit: 1l  }
  product: &cookies  { name: Cookies!,  price: 3.40 ,  unit: 400g}

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

    roundtrip(s);
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

    assert_eq!(roundtrip(s), s);
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
[true, false]: real_bools
[True, TRUE, False, FALSE, y,Y,yes,Yes,YES,n,N,no,No,NO,on,On,ON,off,Off,OFF]: false_bools
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
  - false
: real_bools
? - "True"
  - "TRUE"
  - "False"
  - "FALSE"
  - y
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

    assert_eq!(roundtrip(input), expected);
}

#[test]
fn test_empty_and_nested_not_compact() {
    let s = r"---
a:
  b:
    c: hello
  d: {}
e:
  - f
  - g
  -
    h: []";
    assert_eq!(roundtrip_not_compact(s), s);
}

#[test]
fn test_empty_and_nested_compact() {
    let s = r"---
a:
  b:
    c: hello
  d: {}
e:
  - f
  - g
  - h: []";
    assert_eq!(roundtrip(s), s);
}

#[test]
fn test_interleaved_mappings_and_sequences() {
    let input = r"---
a:
  - b:
      - c: d";
    assert_eq!(roundtrip(input), input);
}

#[test]
fn test_nested_arrays() {
    let s = r"---
a:
  - b
  - - c
    - d
    - - e
      - f";
    assert_eq!(roundtrip(s), s);
}

#[test]
fn test_deeply_nested_arrays() {
    let s = r"---
a:
  - b
  - - c
    - d
    - - e
      - - f
      - - e";
    assert_eq!(roundtrip(s), s);
}

#[test]
fn test_nested_hashes() {
    let s = r"---
a:
  b:
    c:
      d:
        e: f";
    assert_eq!(roundtrip(s), s);
}

#[test]
fn test_empty_sequence() {
    let s = r"---
[]";
    assert_eq!(roundtrip(s), s);
}

#[test]
fn test_empty_mapping() {
    let s = r"---
{}";
    assert_eq!(roundtrip(s), s);
}

#[test]
fn test_root_sequence() {
    let s = r"---
- a";
    assert_eq!(roundtrip(s), s);
}

#[test]
fn test_root_mapping() {
    let s = r"---
a: b";
    assert_eq!(roundtrip(s), s);
}

#[test]
fn test_multiline_string() {
    let input = r#"{foo: "bar!\nbar!", baz: 42}"#;
    let expected = r"---
foo: |-
  bar!
  bar!
baz: 42";
    assert_eq!(roundtrip_multiline(input), expected);
}
