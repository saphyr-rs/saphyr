use std::collections::{BTreeMap, HashMap};

use saphyr_serde::{ErrorKind, from_str, to_string};
use serde::{Deserialize, Serialize};

fn roundtrip<T>(value: &T) -> String
where
    T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
{
    let yaml = to_string(value).unwrap();
    let back: T = from_str(&yaml).unwrap_or_else(|e| panic!("{e}\n{yaml}"));
    assert_eq!(&back, value, "{yaml}");
    yaml
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn output_format() {
    assert_eq!(to_string(&Point { x: 1, y: -2 }).unwrap(), "x: 1\ny: -2\n");
    assert_eq!(to_string(&1).unwrap(), "1\n");
    assert_eq!(to_string("a").unwrap(), "a\n");
    assert_eq!(to_string(&()).unwrap(), "~\n");
    assert_eq!(to_string(&None::<i32>).unwrap(), "~\n");
    assert_eq!(to_string(&vec![1, 2]).unwrap(), "- 1\n- 2\n");
    assert_eq!(to_string(&Vec::<i32>::new()).unwrap(), "[]\n");
    assert_eq!(to_string(&BTreeMap::<i32, i32>::new()).unwrap(), "{}\n");
    assert_eq!(
        to_string(&vec![Point { x: 1, y: 2 }]).unwrap(),
        "- x: 1\n  y: 2\n"
    );
    assert_eq!(to_string("a\nb\n").unwrap(), "|\na\nb\n");
    assert_eq!(to_string(&1.0).unwrap(), "1.0\n");
    assert_eq!(to_string(&f64::NAN).unwrap(), ".nan\n");
    assert_eq!(to_string(&f64::NEG_INFINITY).unwrap(), "-.inf\n");
    assert_eq!(to_string(&0.1f32).unwrap(), "0.1\n");
}

#[test]
fn ambiguous_strings_are_quoted() {
    for s in [
        "true", "True", "TRUE", "false", "null", "Null", "~", "", "1", "-1", "+1", "1.5", "1e3",
        ".inf", "-.inf", ".nan", ".5", "0x1F", "0o17", "yes", "no", "on", "off", "y", "n", "NO",
        "- a", "-", "? a", "a: b", "a #b", "#a", "&a", "*a", "!a", "%a", "@a", "`a", "|", ">",
        "{a}", "[a]", "a,b", " a", "a ", "a\tb", "\ta", "'a'", "\"a\"", "a\\b", "<<", "---", "...",
        "--- a", "=", "inf", "nan", "NaN", "Infinity", "1_000", "0b11", "١٢٣",
    ] {
        roundtrip(&s.to_string());
        roundtrip(&vec![s.to_string()]);
        // And as a key.
        let mut map = BTreeMap::new();
        map.insert(s.to_string(), 1);
        roundtrip(&map);
    }
    assert_eq!(to_string("true").unwrap(), "\"true\"\n");
    assert_eq!(to_string("123").unwrap(), "\"123\"\n");
    assert_eq!(to_string("NO").unwrap(), "\"NO\"\n");
}

#[test]
fn awkward_strings() {
    for s in [
        "a\nb",
        "a\nb\n",
        "\na",
        "\n",
        "\n\n",
        "a\n\n",
        "a\n\n\n",
        "  a\nb",
        "a\n  b",
        "a \nb",
        "a\nb ",
        "a\r\nb",
        "a\rb",
        "\r",
        "a\u{0}b",
        "\u{7f}",
        "\u{85}",
        "a\u{2028}b",
        "a\u{2029}b",
        "\u{feff}a",
        "a\u{feff}",
        "é",
        "日本語",
        "🦀",
        "a\n---\nb",
        "---\na",
        "a\n...\n",
        "a\n--- x",
        "\t\n",
        "a\n\tb",
        "a: b\nc: d",
        "- a\n- b",
        "#x\ny",
        "a\n#x",
        "\u{a0}a",
        "a\u{a0}",
        "a\u{b}b",
        "a\u{c}b",
        "a\u{1b}[0m",
    ] {
        roundtrip(&s.to_string());
        roundtrip(&vec![s.to_string(), s.to_string()]);
        let mut map = BTreeMap::new();
        map.insert(s.to_string(), s.to_string());
        roundtrip(&map);
        roundtrip(&Some(s.to_string()));
        roundtrip(&Nested {
            inner: vec![Some(map.clone())],
        });
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Nested {
    inner: Vec<Option<BTreeMap<String, String>>>,
}

#[test]
fn numbers() {
    roundtrip(&u64::MAX);
    roundtrip(&i64::MIN);
    roundtrip(&i64::MAX);
    roundtrip(&u128::MAX);
    roundtrip(&i128::MIN);
    roundtrip(&(i64::MAX as u64 + 1));
    roundtrip(&0u8);
    for f in [
        0.0,
        -0.0,
        1.0,
        0.1,
        1e300,
        1e-300,
        f64::MAX,
        f64::MIN_POSITIVE,
        5e-324,
        f64::INFINITY,
        f64::NEG_INFINITY,
        123_456_789.123_456_79,
    ] {
        let yaml = to_string(&f).unwrap();
        let back: f64 = from_str(&yaml).unwrap();
        assert_eq!(back.to_bits(), f.to_bits(), "{yaml}");
    }
    for f in [
        0.1f32,
        f32::MAX,
        f32::MIN_POSITIVE,
        1.0e-45,
        3.4028235e38,
        16777217.0,
    ] {
        let yaml = to_string(&f).unwrap();
        let back: f32 = from_str(&yaml).unwrap();
        assert_eq!(back.to_bits(), f.to_bits(), "{yaml}");
    }
    assert!(
        from_str::<f64>(&to_string(&f64::NAN).unwrap())
            .unwrap()
            .is_nan()
    );
}

#[test]
fn enums() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum E {
        Unit,
        Newtype(i32),
        Tuple(i32, String),
        Struct {
            a: i32,
            b: Option<Box<E>>,
        },
        #[serde(rename = "true")]
        True,
    }
    assert_eq!(roundtrip(&E::Unit), "Unit\n");
    assert_eq!(roundtrip(&E::Newtype(1)), "Newtype: 1\n");
    assert_eq!(
        roundtrip(&E::Tuple(1, "x".into())),
        "Tuple:\n  - 1\n  - x\n"
    );
    roundtrip(&E::Struct {
        a: 1,
        b: Some(Box::new(E::Newtype(3))),
    });
    roundtrip(&E::Struct {
        a: 1,
        b: Some(Box::new(E::Struct { a: 2, b: None })),
    });
    roundtrip(&E::True);
    roundtrip(&vec![E::Unit, E::Newtype(2), E::True]);
}

#[test]
fn collections() {
    roundtrip(&vec![vec![1, 2], vec![], vec![3]]);
    roundtrip(&vec![Some(1), None]);
    roundtrip(&(1, "a".to_string(), 2.5));
    roundtrip(&BTreeMap::from([
        ((1, 2), "x".to_string()),
        ((3, 4), "y".to_string()),
    ]));
    roundtrip(&BTreeMap::from([(1.5f64.to_bits(), vec![vec![1u8]])]));
    roundtrip(&BTreeMap::from([(true, 1), (false, 2)]));
    let mut hm = HashMap::new();
    hm.insert(
        "a".to_string(),
        vec![BTreeMap::from([("k".to_string(), Point { x: 1, y: 2 })])],
    );
    roundtrip(&hm);
    roundtrip(&Point { x: 1, y: 2 });
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Unit;
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct New(String);
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Tup(i32, i32);
    roundtrip(&Unit);
    roundtrip(&New("1".into()));
    roundtrip(&Tup(1, 2));
    roundtrip(&Some(Some(1)).map(|x: Option<i32>| x));
}

#[test]
fn bytes() {
    #[derive(Serialize)]
    struct B(#[serde(with = "as_bytes")] Vec<u8>);
    mod as_bytes {
        pub fn serialize<S: serde::Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
            s.serialize_bytes(v)
        }
    }
    assert_eq!(to_string(&B(vec![1, 2])).unwrap(), "- 1\n- 2\n");
}

#[test]
fn errors() {
    // Same key twice.
    #[derive(Serialize)]
    struct Inner {
        a: i32,
    }
    #[derive(Serialize)]
    struct Outer {
        a: i32,
        #[serde(flatten)]
        inner: Inner,
    }
    let err = to_string(&Outer {
        a: 1,
        inner: Inner { a: 2 },
    })
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Serialize);

    // Too deep.
    #[derive(Serialize)]
    struct Deep(Vec<Deep>);
    let mut deep = Deep(vec![]);
    for _ in 0..200 {
        deep = Deep(vec![deep]);
    }
    let err = to_string(&vec![deep]).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::LimitExceeded);
    // Custom errors.
    struct Fails;
    impl Serialize for Fails {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("nope"))
        }
    }
    assert_eq!(to_string(&Fails).unwrap_err().to_string(), "nope");
}

#[test]
fn writer() {
    let mut out = Vec::new();
    saphyr_serde::to_writer(&mut out, &Point { x: 1, y: 2 }).unwrap();
    assert_eq!(out, b"x: 1\ny: 2\n");

    // Writing errors are the ones of the writer.
    struct Failing;
    impl std::io::Write for Failing {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("disk is full"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let err = saphyr_serde::to_writer(Failing, &Point { x: 1, y: 2 }).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Serialize);
    assert!(err.to_string().contains("disk is full"), "{err}");
}

/// A sequence that is not in memory.
struct Counting(u32);

impl Serialize for Counting {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        for i in 0..self.0 {
            seq.serialize_element(&format!("item {i}"))?;
        }
        seq.end()
    }
}

#[test]
fn streams_the_output() {
    // Written as we go: a writer that has had enough ends the serialization.
    struct Limited(usize);
    impl std::io::Write for Limited {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_sub(buf.len());
            if self.0 == 0 {
                Err(std::io::Error::other("enough"))
            } else {
                Ok(buf.len())
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let err = saphyr_serde::to_writer(Limited(1000), &Counting(u32::MAX)).unwrap_err();
    assert!(err.to_string().contains("enough"), "{err}");

    let mut sink = std::io::sink();
    saphyr_serde::to_writer(&mut sink, &Counting(200_000)).unwrap();
    let yaml = to_string(&Counting(3)).unwrap();
    assert_eq!(yaml, "- item 0\n- item 1\n- item 2\n");
}

#[test]
fn duplicate_keys_are_compared_by_text() {
    use saphyr_serde::{Options, to_string_with_options};
    struct Pairs<K>(Vec<(K, i32)>);
    impl<K: Serialize> Serialize for Pairs<K> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeMap;
            let mut map = serializer.serialize_map(None)?;
            for (k, v) in &self.0 {
                map.serialize_entry(k, v)?;
            }
            map.end()
        }
    }
    let twice =
        |r: saphyr_serde::Result<String>| r.unwrap_err().to_string().contains("same key twice");
    assert!(twice(to_string(&Pairs(vec![("a", 1), ("b", 2), ("a", 3)]))));
    assert!(twice(to_string(&Pairs(vec![(1, 1), (1, 2)]))));
    assert!(twice(to_string(&Pairs(vec![(true, 1), (true, 2)]))));
    assert!(twice(to_string(&Pairs(vec![(1.5, 1), (1.5, 2)]))));
    assert!(twice(to_string(&Pairs(vec![(Some(1), 1), (Some(1), 2)]))));
    // `1` and "1" are the same key as far as a reader can tell.
    #[derive(Serialize)]
    #[serde(untagged)]
    enum Key {
        Int(i32),
        Str(&'static str),
        Seq(Vec<i32>),
        Map(BTreeMap<&'static str, i32>),
    }
    let map = || BTreeMap::from([("a", 1), ("b", 2)]);
    assert!(twice(to_string(&Pairs(vec![
        (Key::Int(1), 1),
        (Key::Str("1"), 2)
    ]))));
    // Other keys are fine.
    to_string(&Pairs(vec![
        (1.5, 1),
        (1.0, 2),
        (2.0, 3),
        (f64::NAN, 4),
        (-0.0, 5),
    ]))
    .unwrap();
    to_string(&Pairs(vec![(Key::Int(1), 1), (Key::Str("2"), 2)])).unwrap();
    // Keys that are not scalars are not compared, and do not confuse the others.
    to_string(&Pairs(vec![(Key::Seq(vec![1]), 1), (Key::Seq(vec![1]), 2)])).unwrap();
    to_string(&Pairs(vec![(Key::Map(map()), 1), (Key::Map(map()), 2)])).unwrap();
    let keys = vec![
        (Key::Map(map()), 1),
        (Key::Str("x"), 1),
        (Key::Seq(vec![1, 2]), 2),
        (Key::Int(7), 3),
        (Key::Map(map()), 4),
    ];
    to_string(&Pairs(keys)).unwrap();
    let keys = vec![
        (Key::Map(map()), 1),
        (Key::Str("x"), 1),
        (Key::Map(map()), 2),
        (Key::Str("x"), 2),
    ];
    assert!(twice(to_string(&Pairs(keys))));
    // The same key in two mappings.
    to_string(&vec![Pairs(vec![("a", 1)]), Pairs(vec![("a", 2)])]).unwrap();
    to_string(&BTreeMap::from([("a", BTreeMap::from([("a", 1)]))])).unwrap();

    let options = Options::default().allow_duplicate_keys(true);
    assert_eq!(
        to_string_with_options(&Pairs(vec![("a", 1), ("a", 2)]), &options).unwrap(),
        "a: 1\na: 2\n"
    );
}

#[test]
fn multiline_in_structure() {
    let map = BTreeMap::from([("k".to_string(), "a\nb\n".to_string())]);
    assert_eq!(to_string(&map).unwrap(), "k: |\n  a\n  b\n");
    assert_eq!(to_string(&vec!["a\nb"]).unwrap(), "- |-\n  a\n  b\n");
}
