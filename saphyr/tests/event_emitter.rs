use saphyr::{
    EmitEvent, EventEmitError, LoadableYamlNode, Mapping, Scalar, Yaml, YamlEmitter,
    YamlEventEmitter,
};

fn emit_with(
    events: &[EmitEvent<'_>],
    configure: impl FnOnce(&mut YamlEventEmitter<'_>),
) -> Result<String, EventEmitError> {
    let mut output = String::new();
    let mut emitter = YamlEventEmitter::new(&mut output);
    configure(&mut emitter);
    for event in events {
        emitter.feed_event(event)?;
    }
    emitter.finish()?;
    Ok(output)
}

fn emit(events: &[EmitEvent<'_>]) -> String {
    emit_with(events, |_| {}).unwrap()
}

use EmitEvent::{
    Bool, F32, F64, I64, I128, MappingEnd as ME, MappingStart as MS, Null, SequenceEnd as SE,
    SequenceStart as SS, String as Str, U64, U128,
};

#[test]
fn scalars() {
    assert_eq!(emit(&[Str("foo")]), "foo\n");
    assert_eq!(emit(&[I64(-32)]), "-32\n");
    assert_eq!(emit(&[U64(u64::MAX)]), "18446744073709551615\n");
    assert_eq!(
        emit(&[I128(i128::MIN)]),
        "-170141183460469231731687303715884105728\n"
    );
    assert_eq!(
        emit(&[U128(u128::MAX)]),
        "340282366920938463463374607431768211455\n"
    );
    assert_eq!(emit(&[Bool(true)]), "true\n");
    assert_eq!(emit(&[Null]), "~\n");
    assert_eq!(emit(&[F64(1.0)]), "1.0\n");
    assert_eq!(emit(&[F64(0.5)]), "0.5\n");
    assert_eq!(emit(&[F64(-0.0)]), "-0.0\n");
    assert_eq!(emit(&[F64(f64::NAN)]), ".nan\n");
    assert_eq!(emit(&[F64(f64::INFINITY)]), ".inf\n");
    assert_eq!(emit(&[F64(f64::NEG_INFINITY)]), "-.inf\n");
    assert_eq!(emit(&[F32(0.1)]), "0.1\n");
    assert_eq!(emit(&[F32(3.0)]), "3.0\n");
}

#[test]
fn strings_are_quoted_when_needed() {
    for (input, expected) in [
        ("true", "\"true\""),
        ("null", "\"null\""),
        ("~", "\"~\""),
        ("", "\"\""),
        ("12", "\"12\""),
        ("1.5", "\"1.5\""),
        (".inf", "\".inf\""),
        ("a: b", "\"a: b\""),
        ("a #b", "\"a #b\""),
        ("- a", "\"- a\""),
        (" a", "\" a\""),
        ("a\nb", "\"a\\nb\""),
        ("a\"b", "\"a\\\"b\""),
        ("a\\b", "\"a\\\\b\""),
        ("a\tb", "\"a\\tb\""),
        ("a\u{7}b", "\"a\\u0007b\""),
        ("plain text", "plain text"),
        ("é日🦀", "é日🦀"),
    ] {
        assert_eq!(emit(&[Str(input)]), format!("{expected}\n"), "{input:?}");
    }
}

#[test]
fn sequences() {
    assert_eq!(emit(&[SS, I64(32), SE]), "- 32\n");
    assert_eq!(emit(&[SS, I64(32), Str("foo"), SE]), "- 32\n- foo\n");
    assert_eq!(
        emit(&[SS, SS, I64(32), SE, Str("foo"), SE]),
        "- - 32\n- foo\n"
    );
    assert_eq!(
        emit(&[SS, SS, I64(1), I64(2), SE, SS, I64(3), SE, SE]),
        "- - 1\n  - 2\n- - 3\n"
    );
}

#[test]
fn mappings() {
    assert_eq!(emit(&[MS, Str("foo"), I64(32), ME]), "foo: 32\n");
    assert_eq!(
        emit(&[MS, Str("foo"), I64(32), Str("bar"), Str("baz"), ME]),
        "foo: 32\nbar: baz\n"
    );
    assert_eq!(
        emit(&[MS, Str("foo"), MS, Str("bar"), Str("baz"), ME, ME]),
        "foo:\n  bar: baz\n"
    );
    assert_eq!(
        emit(&[MS, Str("a"), SS, I64(1), I64(2), SE, Str("b"), I64(3), ME]),
        "a:\n  - 1\n  - 2\nb: 3\n"
    );
    assert_eq!(
        emit(&[SS, MS, Str("a"), I64(1), Str("b"), I64(2), ME, SE]),
        "- a: 1\n  b: 2\n"
    );
    assert_eq!(
        emit(&[SS, MS, Str("a"), SS, I64(1), SE, ME, SE]),
        "- a:\n    - 1\n"
    );
    // Keys that are not strings.
    assert_eq!(
        emit(&[MS, I64(1), Null, Null, Bool(false), ME]),
        "1: ~\n~: false\n"
    );
    assert_eq!(emit(&[MS, Str("a\nb"), I64(1), ME]), "\"a\\nb\": 1\n");
}

#[test]
fn empty_containers() {
    assert_eq!(emit(&[SS, SE]), "[]\n");
    assert_eq!(emit(&[MS, ME]), "{}\n");
    assert_eq!(
        emit(&[SS, Str("foo"), SS, SE, Str("bar"), SE]),
        "- foo\n- []\n- bar\n"
    );
    assert_eq!(
        emit(&[
            MS,
            Str("foo"),
            Str("bar"),
            Str("empty"),
            MS,
            ME,
            Str("baz"),
            Str("quux"),
            ME
        ]),
        "foo: bar\nempty: {}\nbaz: quux\n"
    );
    assert_eq!(
        emit(&[
            MS,
            Str("foo"),
            Str("bar"),
            Str("empty"),
            MS,
            Str("foo"),
            MS,
            ME,
            ME,
            Str("baz"),
            Str("quux"),
            ME
        ]),
        "foo: bar\nempty:\n  foo: {}\nbaz: quux\n"
    );
    assert_eq!(
        emit(&[SS, Str("foo"), SS, SS, SE, SE, Str("bar"), SE]),
        "- foo\n- - []\n- bar\n"
    );
    assert_eq!(emit(&[MS, Str("a"), SS, SE, ME]), "a: []\n");
}

#[test]
fn complex_keys() {
    assert_eq!(
        emit(&[MS, SS, I64(1), I64(2), SE, Str("v"), ME]),
        "? - 1\n  - 2\n: v\n"
    );
    assert_eq!(
        emit(&[
            MS,
            MS,
            Str("a"),
            I64(1),
            ME,
            SS,
            I64(2),
            SE,
            Str("b"),
            I64(3),
            ME
        ]),
        "? a: 1\n: - 2\nb: 3\n"
    );
    assert_eq!(emit(&[MS, SS, SE, MS, ME, ME]), "? []\n: {}\n");
}

/// Implicit keys are at most 1024 characters long, quotes included. Longer ones are written as
/// complex keys, and read back as the same key.
#[test]
fn long_keys() {
    let key = |c: &str, n: usize| c.repeat(n);
    for (unit, per_char) in [
        ("k", 1),
        ("é", 1),
        (":", 1),
        ("\n", 2),
        ("\u{1}", 6),
        ("\"", 2),
    ] {
        // Plain keys take one character; the others are quoted, which adds 2.
        let quote_overhead = if unit == "k" || unit == "é" { 0 } else { 2 };
        let fits = (1024 - quote_overhead) / per_char;
        for n in [1, fits - 1, fits, fits + 1, fits + 2, 3 * fits] {
            let k = key(unit, n);
            let events = [MS, Str(&k), I64(1), Str("b"), I64(2), ME];
            let yaml = emit(&events);
            let explicit = n > fits;
            assert_eq!(yaml.starts_with("? "), explicit, "{unit:?} x {n}");
            let docs = Yaml::load_from_str(&yaml)
                .unwrap_or_else(|e| panic!("{unit:?} x {n} is not valid: {e}"));
            let map = docs[0].as_mapping().unwrap();
            assert_eq!(map.len(), 2);
            let (read_key, value) = map.iter().next().unwrap();
            assert_eq!(read_key.as_str(), Some(k.as_str()), "{unit:?} x {n}");
            assert_eq!(value.as_integer(), Some(1));
        }
    }
    // A long key, and a long key with a literal block, in a nested mapping.
    let k = key("k", 2000);
    let yaml = emit(&[MS, Str("a"), MS, Str(&k), Str("x\ny\n"), ME, ME]);
    assert!(Yaml::load_from_str(&yaml).is_ok(), "{yaml}");
}

#[test]
fn not_compact() {
    let events = [SS, SS, I64(1), SE, MS, Str("a"), SS, I64(2), SE, ME, SE];
    assert_eq!(
        emit_with(&events, |e| e.compact(false)).unwrap(),
        "-\n  - 1\n-\n  a:\n    - 2\n"
    );
    let events = [MS, SS, I64(1), SE, SS, I64(2), SE, ME];
    assert_eq!(
        emit_with(&events, |e| e.compact(false)).unwrap(),
        "?\n  - 1\n:\n  - 2\n"
    );
}

#[test]
fn multiline_strings() {
    let on = |e: &mut YamlEventEmitter<'_>| e.multiline(true);
    assert_eq!(emit_with(&[Str("a\nb")], on).unwrap(), "|-\na\nb\n");
    assert_eq!(emit_with(&[Str("a\nb\n")], on).unwrap(), "|\na\nb\n");
    assert_eq!(
        emit_with(&[Str("a\nb\n\n")], on).unwrap(),
        "|+\na\nb\n\n...\n"
    );
    assert_eq!(
        emit_with(
            &[MS, Str("k"), Str("a\nb\n"), Str("l"), Str("c\nd"), ME],
            on
        )
        .unwrap(),
        "k: |\n  a\n  b\nl: |-\n  c\n  d\n"
    );
    assert_eq!(
        emit_with(
            &[SS, Str("a\nb"), Str("c"), Str(" d\ne\n\n"), Str("f"), SE],
            on
        )
        .unwrap(),
        "- |-\n  a\n  b\n- c\n- |2+\n   d\n  e\n  \n- f\n"
    );
    // Not as keys.
    assert_eq!(
        emit_with(&[MS, Str("a\nb"), I64(1), ME], on).unwrap(),
        "\"a\\nb\": 1\n"
    );
    // Not when they cannot be one.
    assert_eq!(emit_with(&[Str("\n\n")], on).unwrap(), "\"\\n\\n\"\n");
    assert_eq!(emit_with(&[Str("a\r\nb")], on).unwrap(), "\"a\\r\\nb\"\n");
    // Off by default.
    assert_eq!(emit(&[Str("a\nb")]), "\"a\\nb\"\n");
}

#[test]
fn documents() {
    assert_eq!(emit(&[I64(1), I64(2)]), "1\n---\n2\n");
    assert_eq!(
        emit(&[SS, I64(1), SE, MS, Str("a"), I64(2), ME, Null]),
        "- 1\n---\na: 2\n---\n~\n"
    );
    let on = |e: &mut YamlEventEmitter<'_>| e.multiline(true);
    assert_eq!(
        emit_with(&[Str("a\nb\n"), Str("c\nd\n\n"), Str("e")], on).unwrap(),
        "|\na\nb\n---\n|+\nc\nd\n\n...\n---\ne\n"
    );
    assert_eq!(emit(&[]), "");
}

#[test]
fn invalid_events() {
    for events in [
        &[SE][..],
        &[ME],
        &[SS, ME],
        &[MS, SE],
        &[MS, Str("a"), ME],
        &[MS, SS, SE, ME],
        &[SS, SS, SE, SE, SE],
        &[SS, SE, ME],
        &[SS],
        &[MS, Str("a"), I64(1)],
        &[SS, I64(1), SS],
    ] {
        assert!(
            matches!(
                emit_with(events, |_| {}),
                Err(EventEmitError::InvalidEvent(_))
            ),
            "{events:?}"
        );
    }
}

#[test]
fn depth_limit() {
    // The emitter does not recurse: it can do this, and stops when it is told to.
    let deep = vec![SS; 100_000];
    assert!(matches!(
        emit_with(&deep, |_| {}),
        Err(EventEmitError::TooDeep(1024))
    ));
    let events: Vec<_> = (0..10).map(|_| SS).chain((0..10).map(|_| SE)).collect();
    assert!(emit_with(&events, |e| e.max_depth(10)).is_ok());
    assert!(matches!(
        emit_with(&events, |e| e.max_depth(9)),
        Err(EventEmitError::TooDeep(9))
    ));
    assert!(matches!(
        emit_with(&[MS, Str("a"), MS], |e| e.max_depth(1)),
        Err(EventEmitError::TooDeep(1))
    ));
    // Nothing is written for a container that is refused.
    let mut output = String::new();
    let mut emitter = YamlEventEmitter::new(&mut output);
    emitter.max_depth(1);
    emitter.sequence_start().unwrap();
    emitter.scalar_i64(1).unwrap();
    assert!(emitter.sequence_start().is_err());
    assert_eq!(output, "- 1");
}

#[test]
fn write_errors_are_reported() {
    struct Failing(usize);
    impl std::fmt::Write for Failing {
        fn write_str(&mut self, _: &str) -> std::fmt::Result {
            if self.0 == 0 {
                return Err(std::fmt::Error);
            }
            self.0 -= 1;
            Ok(())
        }
    }
    for allowed in 0..8 {
        let mut writer = Failing(allowed);
        let mut emitter = YamlEventEmitter::new(&mut writer);
        let result = [MS, Str("a"), SS, I64(1), SE, ME]
            .iter()
            .try_for_each(|e| emitter.feed_event(e));
        assert!(
            matches!(result, Err(EventEmitError::FmtError(_))),
            "{allowed}"
        );
    }
}

// Random trees, to compare with `YamlEmitter` and to read back.

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn string(&mut self) -> String {
        const PIECES: &[&str] = &[
            "a", "b", "z", "0", "1", "9", " ", "  ", "\n", "\n\n", "\t", "\r", ":", ": ", "-",
            "- ", "?", "? ", "#", " #", "'", "\"", "\\", "[", "]", "{", "}", ",", "&", "*", "!",
            "|", ">", "%", "@", "`", ".", "...", "---", "~", "null", "true", "false", "yes", "no",
            "<<", "0x", "0o", "e", "E", "+", "inf", "nan", "é", "日", "🦀", "\u{0}", "\u{7f}",
            "\u{85}", "\u{a0}", "\u{2028}", "\u{2029}", "\u{feff}", "\u{1b}", "=", "%YAML",
            "!!str",
        ];
        (0..self.below(8))
            .map(|_| PIECES[self.below(PIECES.len())])
            .collect()
    }

    fn yaml(&mut self, depth: usize) -> Yaml<'static> {
        match self.below(if depth > 3 { 6 } else { 8 }) {
            0 => Yaml::Value(Scalar::Null),
            1 => Yaml::Value(Scalar::Boolean(self.next() % 2 == 0)),
            2 => Yaml::Value(Scalar::Integer(self.next() as i64)),
            3 => Yaml::Value(Scalar::FloatingPoint(
                match self.below(8) {
                    0 => f64::NAN,
                    1 => f64::INFINITY,
                    2 => f64::NEG_INFINITY,
                    3 => -0.0,
                    _ => f64::from_bits(self.next()).clamp(-1e300, 1e300),
                }
                .into(),
            )),
            4 | 5 => Yaml::Value(Scalar::String(self.string().into())),
            6 => Yaml::Sequence((0..self.below(4)).map(|_| self.yaml(depth + 1)).collect()),
            _ => {
                let mut mapping = Mapping::new();
                for _ in 0..self.below(4) {
                    // Mostly simple keys.
                    let key = if self.below(5) == 0 {
                        self.yaml(depth + 1)
                    } else {
                        Yaml::Value(Scalar::String(self.string().into()))
                    };
                    mapping.insert(key, self.yaml(depth + 1));
                }
                Yaml::Mapping(mapping)
            }
        }
    }
}

fn events_of<'a>(yaml: &'a Yaml<'_>, out: &mut Vec<EmitEvent<'a>>) {
    match yaml {
        Yaml::Value(Scalar::Null) => out.push(Null),
        Yaml::Value(Scalar::Boolean(v)) => out.push(Bool(*v)),
        Yaml::Value(Scalar::Integer(v)) => out.push(I64(*v)),
        Yaml::Value(Scalar::FloatingPoint(v)) => out.push(F64(v.0)),
        Yaml::Value(Scalar::String(v)) => out.push(Str(v)),
        Yaml::Sequence(items) => {
            out.push(SS);
            for item in items {
                events_of(item, out);
            }
            out.push(SE);
        }
        Yaml::Mapping(mapping) => {
            out.push(MS);
            for (key, value) in mapping {
                events_of(key, out);
                events_of(value, out);
            }
            out.push(ME);
        }
        _ => unreachable!(),
    }
}

fn from_events(yaml: &Yaml<'_>, configure: impl FnOnce(&mut YamlEventEmitter<'_>)) -> String {
    let mut events = Vec::new();
    events_of(yaml, &mut events);
    emit_with(&events, configure).unwrap()
}

/// What `YamlEmitter` writes, as the document of the event emitter.
fn from_tree(yaml: &Yaml<'_>, multiline: bool, compact: bool) -> String {
    let mut output = String::new();
    let mut emitter = YamlEmitter::new(&mut output);
    emitter.multiline(multiline);
    emitter.compact(compact);
    emitter.dump(yaml).unwrap();
    let mut output = output.strip_prefix("---\n").unwrap().to_string();
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

#[test]
fn same_output_as_yaml_emitter() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..20_000 {
        let yaml = rng.yaml(0);
        for (multiline, compact) in [(false, true), (true, true), (false, false), (true, false)] {
            let expected = from_tree(&yaml, multiline, compact);
            let actual = from_events(&yaml, |e| {
                e.multiline(multiline);
                e.compact(compact);
            });
            assert_eq!(
                actual, expected,
                "{yaml:?} multiline={multiline} compact={compact}"
            );
        }
    }
}

#[test]
fn reads_back() {
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    for _ in 0..20_000 {
        let documents: Vec<_> = (0..1 + rng.below(3)).map(|_| rng.yaml(0)).collect();
        for multiline in [false, true] {
            let mut events = Vec::new();
            for document in &documents {
                events_of(document, &mut events);
            }
            let output = emit_with(&events, |e| e.multiline(multiline)).unwrap();
            let loaded = Yaml::load_from_str(&output)
                .unwrap_or_else(|e| panic!("{e}\n{documents:?}\n{output}"));
            assert_eq!(loaded, documents, "{output}");
        }
    }
}
