use std::collections::HashMap;

use serde::Serialize;
use serde_json::json;

use crate::{de::from_str, ser::to_string};

#[test]
fn test_struct() {
    #[derive(Serialize)]
    struct Test {
        int: u32,
        seq: Vec<&'static str>,
    }

    let test = Test {
        int: 1,
        seq: vec!["a", "b"],
    };
    let expected = r###"int: 1
seq:
  - a
  - b
"###;
    assert_eq!(to_string(&test).unwrap(), expected);
}

#[test]
fn test_enum() {
    #[derive(Serialize)]
    enum E {
        Unit,
        Newtype(u32),
        Tuple(u32, u32),
        Struct { a: u32 },
    }

    let u = E::Unit;
    let expected = "Unit\n";
    assert_eq!(to_string(&u).unwrap(), expected);

    let n = E::Newtype(1);
    let expected = "Newtype: 1\n";
    assert_eq!(to_string(&n).unwrap(), expected);

    let t = E::Tuple(1, 2);
    let expected = r###"Tuple:
  - 1
  - 2
"###;
    assert_eq!(to_string(&t).unwrap(), expected);

    let s = E::Struct { a: 1 };
    let expected = r#"Struct:
  a: 1
"#;
    assert_eq!(to_string(&s).unwrap(), expected);
}

#[test]
fn test_outdenting() {
    #[derive(Serialize)]
    enum E {
        Unit,
        Newtype(u32),
        Tuple(u32, u32),
        Struct { a: u32 },
    }

    let u = E::Unit;
    let n = E::Newtype(1);
    let t = E::Tuple(1, 2);
    let s = E::Struct { a: 1 };

    let v = vec![
        u,
        n,
        t,
        s,
        E::Newtype(2),
        E::Tuple(3, 4),
        E::Struct { a: 2 },
    ];

    let expected = r#"- Unit
- Newtype: 1
- Tuple:
    - 1
    - 2
- Struct:
    a: 1
- Newtype: 2
- Tuple:
    - 3
    - 4
- Struct:
    a: 2
"#;
    assert_eq!(to_string(&v).unwrap(), expected);
}

#[test]
fn it_writes_multiline_strings() {
    #[derive(Serialize)]
    struct S {
        value: String,
    }

    let s = S {
        value: String::from("foo\nbar\nbaz"),
    };

    let expected = r#"value: |
  foo
  bar
  baz
"#;
    assert_eq!(to_string(&s).unwrap(), expected);
}

#[test]
fn it_quotes_strings_with_json_chars() {
    #[derive(Serialize)]
    struct S {
        value: String,
    }

    let s = S {
        value: String::from("['foo', 'bar', 'baz']"), // looks like json, but we want the string
    };

    let expected = r#"value: "['foo', 'bar', 'baz']"
"#;
    assert_eq!(to_string(&s).unwrap(), expected);
}

#[test]
fn it_serializes_other_types() {
    #[derive(Debug, PartialEq, Serialize)]
    pub struct NewTypeStruct(u32);

    #[derive(Serialize, PartialEq, Eq, Debug)]
    struct Address {
        street: String,
        state: String,
    }

    #[derive(Serialize, Debug)]
    struct S {
        b: bool,
        o_none: Option<String>,
        o_some: Option<String>,
        nested: Address,
        sbyte: i8,
        ubyte: u8,
        sshort: i16,
        ushort: u16,
        slong: i32,
        ulong: u32,
        slonglong: i64,
        ulonglong: u64,
        tuple: (i32, String),
        newtype: NewTypeStruct,
    }

    let s = S {
        b: true,
        o_none: None,
        o_some: Some(String::from("Some string")),
        nested: Address {
            street: String::from("Main Street"),
            state: String::from("New Jersey"),
        },
        sbyte: -1,
        ubyte: 2,
        sshort: -3,
        ushort: 4,
        slong: -5,
        ulong: 6,
        slonglong: -7,
        ulonglong: 8,
        tuple: (9, String::from("that's a tuple")),
        newtype: NewTypeStruct(10),
    };

    let expected = r#"b: true
o_none: null
o_some: Some string
nested:
  street: Main Street
  state: New Jersey
sbyte: -1
ubyte: 2
sshort: -3
ushort: 4
slong: -5
ulong: 6
slonglong: -7
ulonglong: 8
tuple:
  - 9
  - "that's a tuple"
newtype: 10
"#;

    assert_eq!(to_string(&s).unwrap(), expected);
}

#[test]
fn it_serializes_maps() {
    #[derive(Serialize, Debug)]
    struct S {
        map: HashMap<String, String>,
    }

    let mut map = HashMap::new();
    map.insert(String::from("foo"), String::from("bar"));
    map.insert(String::from("baz"), String::from("duh"));

    let s = S { map };

    // Maps have non deterministic ordering, so let's use the serde_json::Value to check via our de
    // module.
    let actual = to_string(&s).unwrap();
    let value: serde_json::Value = from_str(&actual).expect("Should deserialize");

    assert_eq!(value, json!({ "map": {"foo": "bar", "baz": "duh"}}));
}

#[test]
fn it_serializes_unit_types() {
    #[derive(Debug, PartialEq, Serialize)]
    struct Unit;

    #[derive(Debug, PartialEq, Serialize)]
    struct S {
        unit_member: (),
        unit_struct: Unit,
    }

    let s = S {
        unit_member: (),
        unit_struct: Unit,
    };

    let expected = r#"unit_member: null
unit_struct: null
"#;

    let actual = to_string(&s).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn it_serializes_tuple_structs() {
    #[derive(Debug, PartialEq, Serialize)]
    struct Point(f32, f32);
    let p = Point(1.0, 2.2);
    // TODO: should we enforce some precision?
    let expected = r#"- 1
- 2.200000047683716
"#;

    let actual = to_string(&p).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn it_serializes_key_strings_ending_with_colon() {
    #[derive(Debug, PartialEq, Serialize)]
    struct S {
        key: String,
    }
    let s = S {
        key: String::from("a string with a colon:"),
    };
    // TODO: should we enforce some precision?
    let expected = r#"key: "a string with a colon:"
"#;

    let actual = to_string(&s).unwrap();
    assert_eq!(expected, actual);
}
