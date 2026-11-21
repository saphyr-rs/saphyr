use std::collections::{BTreeMap, HashMap};

use saphyr_serde::{ErrorKind, Options, from_str, from_str_with_options};
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn struct_and_nesting() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Outer {
        name: String,
        points: Vec<Point>,
        extra: Option<Point>,
    }
    let input = "
name: foo
points:
  - {x: 1, y: 2}
  - x: -3
    y: 4
extra:
";
    assert_eq!(
        from_str::<Outer>(input).unwrap(),
        Outer {
            name: "foo".into(),
            points: vec![Point { x: 1, y: 2 }, Point { x: -3, y: 4 }],
            extra: None,
        }
    );
}

#[test]
fn scalars_are_type_driven() {
    // The "Norway problem": strings stay strings.
    assert_eq!(from_str::<String>("NO").unwrap(), "NO");
    assert_eq!(from_str::<String>("yes").unwrap(), "yes");
    assert_eq!(from_str::<String>("123").unwrap(), "123");
    assert_eq!(from_str::<String>("1.50").unwrap(), "1.50");
    assert_eq!(from_str::<String>("null").unwrap(), "null");
    assert_eq!(from_str::<String>("0x1F").unwrap(), "0x1F");
    assert_eq!(from_str::<String>("'quoted'").unwrap(), "quoted");
    assert_eq!(
        from_str::<Vec<String>>("[1, true, ~, a b]").unwrap(),
        ["1", "true", "~", "a b"]
    );
    // And so do they in other types.
    assert!(from_str::<bool>("NO").is_err());
    assert!(from_str::<bool>("yes").is_err());
    assert!(from_str::<bool>("'true'").is_err());
    assert!(from_str::<bool>("True").unwrap());
    assert_eq!(from_str::<i64>("0x1F").unwrap(), 31);
    assert_eq!(from_str::<i64>("0o17").unwrap(), 15);
    assert_eq!(from_str::<i64>("+5").unwrap(), 5);
    assert!(from_str::<i64>("'5'").is_err());
    assert!(from_str::<i64>("5.0").is_err());
    assert_eq!(from_str::<f64>("5").unwrap(), 5.0);
    assert_eq!(from_str::<f64>("-1.5e3").unwrap(), -1500.0);
    assert_eq!(from_str::<f64>(".inf").unwrap(), f64::INFINITY);
    assert!(from_str::<f64>(".nan").unwrap().is_nan());
    assert_eq!(from_str::<char>("a").unwrap(), 'a');
    assert!(from_str::<char>("ab").is_err());
}

#[test]
fn integer_ranges() {
    assert_eq!(from_str::<u64>("18446744073709551615").unwrap(), u64::MAX);
    assert_eq!(from_str::<i64>("-9223372036854775808").unwrap(), i64::MIN);
    assert_eq!(
        from_str::<u128>("340282366920938463463374607431768211455").unwrap(),
        u128::MAX
    );
    assert_eq!(
        from_str::<i128>("-170141183460469231731687303715884105728").unwrap(),
        i128::MIN
    );
    assert_eq!(
        from_str::<i128>("170141183460469231731687303715884105727").unwrap(),
        i128::MAX
    );
    assert!(from_str::<u8>("256").is_err());
    assert!(from_str::<u8>("-1").is_err());
    assert!(from_str::<i8>("-129").is_err());
    assert!(from_str::<i64>("9223372036854775808").is_err());
    assert_eq!(from_str::<u8>("0xff").unwrap(), 255);
}

#[test]
fn options_and_units() {
    assert_eq!(from_str::<Option<i32>>("~").unwrap(), None);
    assert_eq!(from_str::<Option<i32>>("").unwrap(), None);
    assert_eq!(from_str::<Option<i32>>("3").unwrap(), Some(3));
    assert_eq!(from_str::<Option<String>>("null").unwrap(), None);
    assert_eq!(
        from_str::<Option<String>>("'null'").unwrap().as_deref(),
        Some("null")
    );
    from_str::<()>("~").unwrap();
    from_str::<()>("").unwrap();
    assert!(from_str::<()>("1").is_err());
}

#[test]
fn maps_with_any_keys() {
    let map: BTreeMap<i32, String> = from_str("1: a\n2: b").unwrap();
    assert_eq!(
        map,
        BTreeMap::from([(1, "a".to_string()), (2, "b".to_string())])
    );
    let map: HashMap<String, Vec<u8>> = from_str("a: [1]\n'b': []").unwrap();
    assert_eq!(map["a"], [1]);
    let map: BTreeMap<(i32, i32), i32> = from_str("? [1, 2]\n: 3").unwrap();
    assert_eq!(map[&(1, 2)], 3);
}

#[test]
fn enums() {
    #[derive(Deserialize, Debug, PartialEq)]
    enum E {
        Unit,
        Newtype(i32),
        Tuple(i32, String),
        Struct { a: i32 },
    }
    assert_eq!(from_str::<E>("Unit").unwrap(), E::Unit);
    assert_eq!(from_str::<E>("'Unit'").unwrap(), E::Unit);
    assert_eq!(from_str::<E>("Newtype: 1").unwrap(), E::Newtype(1));
    assert_eq!(
        from_str::<E>("Tuple: [1, x]").unwrap(),
        E::Tuple(1, "x".into())
    );
    assert_eq!(from_str::<E>("Struct: {a: 1}").unwrap(), E::Struct { a: 1 });
    assert_eq!(from_str::<E>("Unit: ~").unwrap(), E::Unit);
    // Tags.
    assert_eq!(from_str::<E>("!Unit").unwrap(), E::Unit);
    assert_eq!(from_str::<E>("!Newtype 1").unwrap(), E::Newtype(1));
    assert_eq!(
        from_str::<E>("!Tuple [1, x]").unwrap(),
        E::Tuple(1, "x".into())
    );
    assert_eq!(from_str::<E>("!Struct {a: 1}").unwrap(), E::Struct { a: 1 });
    assert_eq!(
        from_str::<Vec<E>>("[!Unit, Unit, {Newtype: 2}]").unwrap(),
        [E::Unit, E::Unit, E::Newtype(2)]
    );
    // Errors.
    assert!(from_str::<E>("Nope").is_err());
    assert!(from_str::<E>("Newtype").is_err());
    assert!(from_str::<E>("Unit: 1").is_err());
    assert!(from_str::<E>("{Unit: ~, Newtype: 1}").is_err());
    assert!(from_str::<E>("[Unit]").is_err());
}

#[test]
fn untagged_and_flatten() {
    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(untagged)]
    enum U {
        Int(i64),
        Str(String),
        List(Vec<U>),
    }
    assert_eq!(
        from_str::<U>("[1, a, [2]]").unwrap(),
        U::List(vec![
            U::Int(1),
            U::Str("a".into()),
            U::List(vec![U::Int(2)])
        ])
    );

    #[derive(Deserialize, Debug, PartialEq)]
    struct F {
        a: i32,
        #[serde(flatten)]
        rest: BTreeMap<String, i32>,
    }
    let f: F = from_str("a: 1\nb: 2\nc: 3").unwrap();
    assert_eq!(f.rest.len(), 2);
}

#[test]
fn cow_strings() {
    use std::borrow::Cow;
    // Strings are borrowed from the input when the parser lets us, and owned otherwise.
    #[derive(Deserialize)]
    struct B<'a> {
        s: Cow<'a, str>,
    }
    let b: B = from_str("s: hello world").unwrap();
    assert_eq!(b.s, "hello world");
    let b: B = from_str("s: \"a\\nb\"").unwrap();
    assert_eq!(b.s, "a\nb");
}

#[test]
fn strings_styles() {
    assert_eq!(from_str::<String>("\"a\\tb\\u00e9\"").unwrap(), "a\tbé");
    assert_eq!(from_str::<String>("|\n  a\n  b\n").unwrap(), "a\nb\n");
    assert_eq!(from_str::<String>(">\n  a\n  b\n").unwrap(), "a b\n");
    assert_eq!(from_str::<String>("a\n b").unwrap(), "a b");
}

#[test]
fn documents() {
    assert!(from_str::<Option<i32>>("").unwrap().is_none());
    assert!(from_str::<Option<i32>>("# comment").unwrap().is_none());
    assert_eq!(from_str::<i32>("---\n1\n...\n").unwrap(), 1);
    assert!(from_str::<i32>("1\n---\n2").is_err());
    assert!(from_str::<i32>("1\n--- [\n").is_err());

    let docs: Vec<i32> = saphyr_serde::Deserializer::from_str("1\n---\n2\n---\n3")
        .map(|d| i32::deserialize(d.unwrap()).unwrap())
        .collect();
    assert_eq!(docs, [1, 2, 3]);
    assert_eq!(saphyr_serde::Deserializer::from_str("").count(), 0);

    // An error ends the stream.
    let mut stream = saphyr_serde::Deserializer::from_str("1\n---\n[\n---\n3");
    assert!(stream.next().unwrap().is_ok());
    assert!(stream.next().unwrap().is_err());
    assert!(stream.next().is_none());
}

#[test]
fn from_slice() {
    assert_eq!(saphyr_serde::from_slice::<i32>(b"5").unwrap(), 5);
    assert_eq!(
        saphyr_serde::from_slice::<i32>(b"\xef\xbb\xbf5").unwrap(),
        5
    );
    assert!(saphyr_serde::from_slice::<i32>(b"\xff").is_err());
}

#[test]
fn error_locations() {
    let err = from_str::<Point>("x: 1\ny: oops").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Data);
    let loc = err.location().unwrap();
    assert_eq!((loc.line(), loc.column()), (2, 4));
    assert!(err.to_string().contains("line 2 column 4"), "{err}");

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    struct Strict {
        #[allow(dead_code)]
        x: i32,
    }
    let err = from_str::<Strict>("x: 1\nz: 1").unwrap_err();
    assert!(err.to_string().contains("unknown field `z`"), "{err}");
    assert!(err.to_string().contains("line 2"), "{err}");

    let err = from_str::<Point>("x: 1\ny: [").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Syntax);
    assert!(err.location().is_some());

    assert!(from_str::<Vec<i32>>("{}").is_err());
    assert!(from_str::<Point>("[]").is_err());
    assert!(from_str::<Vec<i32>>("[1, 2]").is_ok());
    assert!(from_str::<(i32, i32)>("[1, 2, 3]").is_err());
    assert!(from_str::<(i32, i32)>("[1]").is_err());
}

#[test]
fn tags() {
    assert_eq!(from_str::<String>("!!str 123").unwrap(), "123");
    assert_eq!(from_str::<f64>("!!float 1").unwrap(), 1.0);
    assert_eq!(from_str::<i32>("!!int 0x10").unwrap(), 16);
    assert!(from_str::<i32>("!!int abc").is_err());
    // Data we do not support must not silently be something else.
    assert!(from_str::<String>("!!binary aGVsbG8=").is_err());
    assert!(from_str::<serde::de::IgnoredAny>("!!binary aGVsbG8=").is_ok());
    // Custom tags are ignored.
    assert_eq!(from_str::<i32>("!degrees 50").unwrap(), 50);
    assert_eq!(from_str::<Vec<i32>>("!things [1]").unwrap(), [1]);
}

#[test]
fn aliases() {
    let input = "
base: &b {x: 1, y: 2}
a: *b
list: [&n 5, *n, *n]
";
    #[derive(Deserialize, Debug)]
    struct D {
        base: Point,
        a: Point,
        list: Vec<i32>,
    }
    let d: D = from_str(input).unwrap();
    assert_eq!(d.base, d.a);
    assert_eq!(d.list, [5, 5, 5]);

    // An alias as a key.
    let m: HashMap<String, String> = from_str("&k a: 1\nb: *k").unwrap();
    assert_eq!(m["a"], "1");
    assert_eq!(m["b"], "a");
}

#[test]
fn alias_keys() {
    let m: HashMap<String, String> = from_str("&k a: &v 1\n*v : *k").unwrap();
    assert_eq!(m["a"], "1");
    assert_eq!(m["1"], "a");
}

#[test]
fn merge_keys_are_not_special() {
    // `<<` is YAML 1.1. It is a key like any other.
    use serde::de::IgnoredAny;
    let m: BTreeMap<String, BTreeMap<String, IgnoredAny>> =
        from_str("base: &b {x: 1}\nuse: {<<: *b, y: 2}").unwrap();
    assert!(m["use"].keys().eq(["<<", "y"]));
    let m: BTreeMap<String, BTreeMap<String, BTreeMap<String, i32>>> =
        from_str("a: {<<: {x: 1}}").unwrap();
    assert_eq!(m["a"]["<<"]["x"], 1);
    let m: BTreeMap<String, BTreeMap<String, i32>> = from_str("a: {<<: 1}").unwrap();
    assert_eq!(m["a"]["<<"], 1);
    assert!(from_str::<BTreeMap<String, i32>>("<<: 1\n'<<': 2").is_err());
}

#[test]
fn duplicate_keys() {
    let err = from_str::<BTreeMap<String, i32>>("a: 1\nb: 2\na: 3").unwrap_err();
    assert!(err.to_string().contains("duplicate"), "{err}");
    assert!(from_str::<BTreeMap<String, i32>>("{a: 1, 'a': 3}").is_err());
    assert!(from_str::<BTreeMap<String, i32>>("&k a: 1\n*k : 3").is_err());
    assert!(from_str::<Vec<BTreeMap<String, i32>>>("[{a: 1}, {a: 2}]").is_ok());
    let options = Options::default().allow_duplicate_keys(true);
    let m: BTreeMap<String, i32> = from_str_with_options("a: 1\na: 3", &options).unwrap();
    assert_eq!(m["a"], 3);
}

fn nested(depth: usize) -> String {
    format!("{}{}", "[".repeat(depth), "]".repeat(depth))
}

#[test]
fn depth_limit() {
    use serde::de::IgnoredAny;
    let options = Options::default();
    // Deeper than the stack could take, if we did not limit it.
    for input in [
        nested(1_000_000),
        "- ".repeat(100_000),
        "{a: ".repeat(100_000),
    ] {
        // The parser has a limit of its own for flow collections.
        let err = from_str::<IgnoredAny>(&input).unwrap_err();
        assert!(
            matches!(err.kind(), ErrorKind::LimitExceeded | ErrorKind::Syntax),
            "{err}"
        );
    }
    for input in ["- ".repeat(100_000), "{a: ".repeat(200), nested(200)] {
        let err = from_str_with_options::<IgnoredAny>(&input, &Options::default().max_depth(100))
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err}");
    }
    from_str_with_options::<IgnoredAny>(&nested(128), &options).unwrap();
    let err = from_str_with_options::<IgnoredAny>(&nested(129), &options).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::LimitExceeded);

    // Deserializing into recursive types is safe too.
    #[derive(Deserialize, Debug)]
    enum Tree {
        Leaf,
        Node(#[allow(dead_code)] Vec<Tree>),
    }
    let err = from_str::<Tree>(&format!(
        "{}Leaf{}",
        "{Node: [".repeat(100),
        "]}".repeat(100)
    ))
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::LimitExceeded);
    from_str::<Tree>(&format!("{}Leaf{}", "{Node: [".repeat(30), "]}".repeat(30))).unwrap();
}

#[test]
fn big_integers() {
    // The magnitude of a negative number is at most 2^127: it does not wrap around.
    for text in [
        "-340282366920938463463374607431768211455",
        "-170141183460469231731687303715884105729",
        "-0xff",
    ] {
        assert!(from_str::<i128>(text).is_err(), "{text}");
    }
    assert_eq!(from_str::<i128>("-1").unwrap(), -1);
    assert_eq!(
        from_str::<i128>("-170141183460469231731687303715884105727").unwrap(),
        i128::MIN + 1
    );
    // Too big for 64 bits, or even 128 bits: still a number when a float is expected.
    assert_eq!(
        from_str::<f64>("18446744073709551616").unwrap(),
        18446744073709551616.0
    );
    assert_eq!(
        from_str::<f32>("-18446744073709551616").unwrap(),
        -18446744073709551616.0
    );
    assert_eq!(
        from_str::<f64>("-340282366920938463463374607431768211455").unwrap(),
        -3.402823669209385e38
    );
    assert_eq!(from_str::<f64>("12").unwrap(), 12.0);
    assert!(from_str::<f64>("abc").is_err());
    assert!(from_str::<f64>("[1]").is_err());
}

#[test]
fn tags_on_quoted_scalars() {
    assert_eq!(from_str::<i32>("!!int '5'").unwrap(), 5);
    assert_eq!(from_str::<i32>("!!int \"5\"").unwrap(), 5);
    assert!(from_str::<i32>("!!int 'five'").is_err());
    assert!(from_str::<bool>("!!bool 'true'").unwrap());
    assert_eq!(from_str::<String>("!!str 5").unwrap(), "5");
    assert_eq!(from_str::<String>("'5'").unwrap(), "5");
    assert!(from_str::<i32>("'5'").is_err());
}

#[test]
fn aliases_to_long_strings_are_limited() {
    // One byte of input does not make a thousand bytes of output: the text counts.
    let long = "x".repeat(8000);
    let mut input = format!("- &a {long}\n");
    input.push_str(&"- *a\n".repeat(2000));
    let err = from_str::<Vec<String>>(&input).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err}");
    // A few are fine.
    let mut input = format!("- &a {long}\n");
    input.push_str(&"- *a\n".repeat(20));
    assert_eq!(from_str::<Vec<String>>(&input).unwrap().len(), 21);
}

#[test]
fn billion_laughs() {
    let mut input = String::from("a: &a0 [x, x, x, x, x, x, x, x, x, x]\n");
    for i in 1..20 {
        let prev = format!("*a{}", i - 1);
        let items = vec![prev; 10].join(", ");
        input.push_str(&format!("a{i}: &a{i} [{items}]\n"));
    }
    let start = std::time::Instant::now();
    let err = from_str::<serde::de::IgnoredAny>(&input).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err}");
    assert!(start.elapsed().as_secs() < 5);

    // A reasonable use of aliases is fine.
    #[derive(Deserialize, Debug, PartialEq)]
    struct Shared {
        a: Vec<String>,
        b: Vec<Vec<String>>,
    }
    let shared: Shared = from_str("a: &a [x, y]\nb: [*a, *a]\n").unwrap();
    assert_eq!(shared.a, ["x", "y"]);
    assert_eq!(shared.b, [["x", "y"], ["x", "y"]]);
    // Each node, and each byte of text, counts: `[x, y]` is 5.
    let options = Options::default().max_alias_expansion(5);
    assert!(
        from_str_with_options::<serde::de::IgnoredAny>("a: &a [x, y]\nb: [*a, *a]", &options)
            .is_err()
    );
    assert!(
        from_str_with_options::<serde::de::IgnoredAny>("a: &a [x, y]\nb: [*a]", &options).is_ok()
    );
}

#[test]
fn alias_cycles_and_depth() {
    use serde::de::IgnoredAny;
    // An alias to an anchor that is not complete (or does not exist).
    assert!(from_str::<IgnoredAny>("&a [*a]").is_err());
    assert!(from_str::<IgnoredAny>("&a {x: *a}").is_err());
    assert!(from_str::<IgnoredAny>("[*a]").is_err());
    assert!(from_str::<IgnoredAny>("*a").is_err());
    // The depth through aliases counts.
    let mut input = String::from("a0: &a0 [[[[[[[[[[x]]]]]]]]]]\n");
    for i in 1..30 {
        input.push_str(&format!("a{i}: &a{i} [*a{}]\n", i - 1));
    }
    let options = Options::default().max_depth(30);
    let err = from_str_with_options::<IgnoredAny>(&input, &options).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::LimitExceeded, "{err}");
    let options = Options::default().max_depth(45);
    from_str_with_options::<IgnoredAny>(&input, &options).unwrap();
}

#[test]
fn node_limit() {
    let options = Options::default().max_nodes(100);
    let input = format!("[{}]", "x, ".repeat(200));
    let err = from_str_with_options::<serde::de::IgnoredAny>(&input, &options).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::LimitExceeded);
}
