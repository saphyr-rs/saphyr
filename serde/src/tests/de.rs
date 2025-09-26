use std::f32;

use serde::Deserialize;
use serde_json::json;

use crate::{de::from_str, error::DeserializeError};

const ADDRESS_YAML_STR: &str = r###"
street: Kerkstraat
state: Noord Holland
"###;

#[derive(Deserialize, PartialEq, Eq, Debug)]
struct Address {
    street: String,
    state: String,
}

#[test]
fn it_deserializes_empty_string() {
    #[derive(Deserialize, PartialEq, Eq, Debug, Default)]
    #[serde(default)]
    struct Point {
        x: i32,
        y: i32,
    }

    let result: Option<Point> = from_str("").expect("Should deserialize");

    assert_eq!(result, Some(Point::default()));
}

#[test]
fn it_deserializes_empty_with_default() {
    #[derive(Deserialize, PartialEq, Eq, Debug, Default)]
    #[serde(default)]
    struct Point {
        x: i32,
        y: i32,
    }

    let result: Point = from_str("x: 27\nz: 0").expect("Should deserialize");

    assert_eq!(result, Point { x: 27, y: 0 });
}

#[test]
fn it_deserializes_mappings() {
    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Point {
        x: i32,
        y: i32,
    }

    const POINT_YAML_STR: &str = r###"
x: 10
y: 45
"###;
    let result: Point = from_str(POINT_YAML_STR).expect("Should deserialize");

    assert_eq!(result, Point { x: 10, y: 45 });

    let _err = from_str::<Point>("x: 10\nz: 20").expect_err("Should not deserialize");

    assert_eq!(
        _err,
        DeserializeError::SerdeError(String::from("missing field `y`"))
    );
}

#[test]
fn it_deserializes_strings() {
    let result: Address = from_str(ADDRESS_YAML_STR).expect("Should deserialize");

    assert_eq!(
        result,
        Address {
            street: String::from("Kerkstraat"),
            state: String::from("Noord Holland")
        }
    );
}

#[test]
fn it_reads_json_values() {
    let result: serde_json::Value = from_str(ADDRESS_YAML_STR).expect("Should deserialize");

    assert_eq!(
        result,
        json!({"street": "Kerkstraat", "state": "Noord Holland"})
    );
}

#[test]
fn it_reads_nested_values() {
    #[derive(Deserialize, Debug, PartialEq, Eq)]
    struct NestedAddress {
        address: Address,
    }

    const NESTED_ADDRESS_YAML_STR: &str = r###"
address:
    street: Kerkstraat
    state: Noord Holland
"###;

    let result: serde_json::Value = from_str(NESTED_ADDRESS_YAML_STR).expect("Should deserialize");

    assert_eq!(
        result,
        json!({"address": {"street": "Kerkstraat", "state": "Noord Holland"}})
    );

    let address: NestedAddress = from_str(NESTED_ADDRESS_YAML_STR).expect("Should deserialize");

    assert_eq!(
        address,
        NestedAddress {
            address: Address {
                street: String::from("Kerkstraat"),
                state: String::from("Noord Holland")
            }
        }
    );
}

#[test]
fn it_reads_sequences() {
    const SEQUENCE_ADDRESS_YAML_STR: &str = r###"
- street: Kerkstraat
  state: Noord Holland
- street: Main Street
  state: New York
"###;

    let result: serde_json::Value =
        from_str(SEQUENCE_ADDRESS_YAML_STR).expect("Should deserialize");

    assert_eq!(
        result,
        json!([
            {"street": "Kerkstraat", "state": "Noord Holland"},
            {"street": "Main Street", "state": "New York"},
        ])
    );

    let address: Vec<Address> = from_str(SEQUENCE_ADDRESS_YAML_STR).expect("Should deserialize");

    assert_eq!(
        address,
        vec![
            Address {
                street: String::from("Kerkstraat"),
                state: String::from("Noord Holland")
            },
            Address {
                street: String::from("Main Street"),
                state: String::from("New York")
            },
        ]
    );
}

#[test]
fn it_reads_enums() {
    #[derive(Deserialize, PartialEq, Eq, Debug)]
    enum TestEnum {
        ValueA,
        ValueB,
    }

    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct StructWithEnum {
        value: TestEnum,
    }

    const STRUCT_WITH_ENUM_YAML_STR: &str = r###"
value: ValueA
"###;

    let result: StructWithEnum = from_str(STRUCT_WITH_ENUM_YAML_STR).expect("Should deserialize");

    assert_eq!(
        result,
        StructWithEnum {
            value: TestEnum::ValueA
        }
    );
}

#[test]
fn it_reads_externally_tagged_enums() {
    #[derive(Deserialize, PartialEq, Eq, Debug)]
    enum TestExternallyTaggedEnum {
        ValueA { id: String, method: String },
        ValueB { id: String, result: String },
    }

    const EXTERNALLY_TAGGED_ENUM_YAML_STR: &str = r###"
- ValueA:
    id: foo
    method: bar
- ValueB:
    id: baz
    result: passed
"###;
    let result: Vec<TestExternallyTaggedEnum> =
        from_str(EXTERNALLY_TAGGED_ENUM_YAML_STR).expect("Should deserialize");

    assert_eq!(
        result,
        vec![
            TestExternallyTaggedEnum::ValueA {
                id: String::from("foo"),
                method: String::from("bar")
            },
            TestExternallyTaggedEnum::ValueB {
                id: String::from("baz"),
                result: String::from("passed")
            }
        ],
    );
}

#[test]
fn it_reads_other_enum_types() {
    #[derive(Deserialize)]
    enum Test {
        ValueA,
        ValueB,
    }

    let _value: Test = from_str("ValueA").expect("Should deserialize");

    #[derive(Deserialize, PartialEq, Debug)]
    enum TupleVariant {
        T(u8, u8),
    }

    let _value: TupleVariant = from_str("T:\n  - 27\n  - 32\n").expect("Should deserialize");
    assert_eq!(_value, TupleVariant::T(27, 32));
}

#[test]
fn it_reads_all_the_int_formats() {
    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct TestInts {
        sbyte: i8,
        ubyte: u8,
        sshort: i16,
        ushort: u16,
        slong: i32,
        ulong: u32,
        slonglong: i64,
        ulonglong: u64,
    }

    const TEST_INTS_YAML: &str = r###"
sbyte: -1
ubyte: 2
sshort: -3
ushort: 4
slong: -5
ulong: 6
slonglong: -7
ulonglong: 8
"###;
    let result: TestInts = from_str(TEST_INTS_YAML).expect("Should deserialize");

    assert_eq!(
        result,
        TestInts {
            sbyte: -1,
            ubyte: 2,
            sshort: -3,
            ushort: 4,
            slong: -5,
            ulong: 6,
            slonglong: -7,
            ulonglong: 8,
        }
    );
}

#[test]
fn it_reads_all_both_floats() {
    #[derive(Deserialize, Debug)]
    struct TestFloats {
        single: f32,
        double: f64,
    }

    const TEST_YAML: &str = r###"
single: 0.123
double: 0.123
"###;
    let result: TestFloats = from_str(TEST_YAML).expect("Should deserialize");

    fn are_nearly_equal<T: Into<f64>>(a: T, b: T, epsilon: T) -> bool {
        let a = a.into();
        let b = b.into();
        let epsilon = epsilon.into();

        (a - b).abs() < epsilon
    }

    assert!(are_nearly_equal(result.single, 0.123, f32::EPSILON));
    assert!(are_nearly_equal(result.double, 0.123, f64::EPSILON));
}

#[test]
fn it_reads_chars() {
    #[derive(Deserialize, Debug)]
    struct Test {
        c: char,
    }

    from_str::<Test>(
        r###"
c: ab
"###,
    )
    .expect_err("Should not deserialize");

    let result: Test = from_str(
        r###"
c: a
"###,
    )
    .expect("Should deserialize");
    assert_eq!(result.c, 'a');
}

#[test]
fn it_reads_bools() {
    #[derive(Deserialize, Debug)]
    struct Test {
        b: bool,
    }

    from_str::<Test>("b: not_a_boolean").expect_err("Should not deserialize");

    let result: Test = from_str("b: True").expect("Should deserialize");
    assert!(result.b);

    from_str::<Test>("b: tRUE").expect_err("Should not deserialize");
}

#[test]
fn it_reads_options() {
    #[derive(Deserialize, Debug)]
    struct Test {
        opt: Option<String>,
    }

    let result: Test = from_str("opt: foo").expect("Should deserialize");
    assert_eq!(result.opt, Some(String::from("foo")));

    let result: Test = from_str("opt: null").expect("Should deserialize");
    assert_eq!(result.opt, None);

    // saphyr uses ~ for null values too
    let result: Test = from_str("opt: ").expect("Should deserialize");
    assert_eq!(result.opt, None);
}

#[test]
fn it_reads_unit() {
    // no idea when this would be useful...
    let _value: () = from_str("~").expect("Should deserialize");
    let _value: () = from_str("null").expect("Should deserialize");
    let _value: () = from_str("---\n").expect("Should deserialize");
}

#[test]
fn it_reads_unit_structs() {
    #[derive(Debug, PartialEq, Deserialize)]
    struct Unit;

    let _value: Unit = from_str("~").expect("Should deserialize");
    let _value: Unit = from_str("null").expect("Should deserialize");
    let _value: Unit = from_str("---\n").expect("Should deserialize");

    assert_eq!(_value, Unit);
}

#[test]
fn it_reads_newtype_structs() {
    #[derive(Debug, PartialEq, Deserialize)]
    pub struct Test(u32);

    let _value: Test = from_str("5").expect("Should deserialize");

    assert_eq!(_value, Test(5));
}

#[test]
fn it_reads_tuples() {
    let _value: (String, i32) = from_str("- abc\n- 27\n").expect("Should deserialize");

    assert_eq!(_value.0, "abc");
    assert_eq!(_value.1, 27);

    from_str::<(String, i32)>("- abc\n- 27\n- too many values\n")
        .expect_err("Should not deserialize");
}

#[test]
fn it_reads_tuple_structs() {
    #[derive(Debug, PartialEq, Deserialize)]
    pub struct Point(i32, i32);

    let _value: Point = from_str("- 27\n- 32\n").expect("Should deserialize");

    assert_eq!(_value, Point(27, 32));

    from_str::<Point>("- 32\n- 27\n- 47\n").expect_err("Should not deserialize");
    from_str::<Point>("- not a i32\n- 27\n").expect_err("Should not deserialize");
}

#[test]
fn it_reads_internally_tagged_enums() {
    #[derive(Deserialize, PartialEq, Debug)]
    #[serde(tag = "type")]
    enum Message {
        Request { id: String, method: String },
        Response { id: String, result: String },
    }

    let _value: Message =
        from_str("type: Request\nid: foo\nmethod: PUT").expect("Should deserialize");

    assert_eq!(
        _value,
        Message::Request {
            id: String::from("foo"),
            method: String::from("PUT")
        }
    );

    let err: DeserializeError = from_str::<Message>("type: UnknownVariant\nid: foo\nmethod: PUT")
        .expect_err("Should not deserialize");

    assert_eq!(
        err,
        // ("unknown variant `UnknownVariant`, expected `Request` or `Response`")
        DeserializeError::SerdeError(String::from(
            "unknown variant `UnknownVariant`, expected `Request` or `Response`"
        ))
    );
}
