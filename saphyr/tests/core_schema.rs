//! Resolution of scalars against the [YAML 1.2 Core Schema].
//!
//! [YAML 1.2 Core Schema]: https://yaml.org/spec/1.2.2/#103-core-schema

use std::borrow::Cow;

use saphyr::{LoadableYamlNode, Scalar, ScalarStyle, Tag, Yaml};

fn implicit(v: &str) -> Scalar<'_> {
    Scalar::parse_from_cow(v.into())
}

fn tagged<'a>(v: &'a str, suffix: &str) -> Option<Scalar<'a>> {
    let tag = Cow::Owned(Tag {
        handle: "tag:yaml.org,2002:".into(),
        suffix: suffix.into(),
    });
    Scalar::parse_from_cow_and_metadata(v.into(), ScalarStyle::Plain, Some(&tag))
}

fn f(v: f64) -> Scalar<'static> {
    Scalar::FloatingPoint(v.into())
}

fn s(v: &str) -> Scalar<'_> {
    Scalar::String(v.into())
}

/// Every representation of the [table] in <https://yaml.org/spec/1.2.2/#103-core-schema>, with the
/// near-misses that must *not* resolve to the same tag.
fn corpus() -> Vec<(&'static str, Scalar<'static>)> {
    vec![
        // null: `null | Null | NULL | ~` and the empty representation.
        ("", Scalar::Null),
        ("~", Scalar::Null),
        ("null", Scalar::Null),
        ("Null", Scalar::Null),
        ("NULL", Scalar::Null),
        ("nULL", s("nULL")),
        ("None", s("None")),
        ("nil", s("nil")),
        // bool: `true | True | TRUE | false | False | FALSE`.
        ("true", Scalar::Boolean(true)),
        ("True", Scalar::Boolean(true)),
        ("TRUE", Scalar::Boolean(true)),
        ("false", Scalar::Boolean(false)),
        ("False", Scalar::Boolean(false)),
        ("FALSE", Scalar::Boolean(false)),
        ("tRUE", s("tRUE")),
        // YAML 1.1 booleans, dropped by the 1.2 core schema.
        ("yes", s("yes")),
        ("No", s("No")),
        ("on", s("on")),
        // int, base 10: `[-+]?[0-9]+`.
        ("0", Scalar::Integer(0)),
        ("7", Scalar::Integer(7)),
        ("007", Scalar::Integer(7)),
        ("-7", Scalar::Integer(-7)),
        ("+7", Scalar::Integer(7)),
        ("+0", Scalar::Integer(0)),
        ("-0", Scalar::Integer(0)),
        ("++7", s("++7")),
        ("+-7", s("+-7")),
        ("1_000", s("1_000")),
        ("+", s("+")),
        // int, base 8: `0o[0-7]+`.
        ("0o17", Scalar::Integer(15)),
        ("0o0", Scalar::Integer(0)),
        ("0o", s("0o")),
        ("0o8", s("0o8")),
        ("0O17", s("0O17")),
        ("-0o17", s("-0o17")),
        ("0o+17", s("0o+17")),
        ("0o-17", s("0o-17")),
        // int, base 16: `0x[0-9a-fA-F]+`.
        ("0x1F", Scalar::Integer(31)),
        ("0x00FF", Scalar::Integer(255)),
        ("0x", s("0x")),
        ("0xG", s("0xG")),
        ("0X1F", s("0X1F")),
        ("-0x1F", s("-0x1F")),
        ("+0x1F", s("+0x1F")),
        ("0x+1F", s("0x+1F")),
        ("0x-1F", s("0x-1F")),
        // No base 2 in the core schema.
        ("0b101", s("0b101")),
        // float.
        ("1.5", f(1.5)),
        ("-1.5", f(-1.5)),
        ("+.5", f(0.5)),
        ("1.", f(1.0)),
        ("1e3", f(1000.0)),
        ("1.5e-3", f(0.0015)),
        (".inf", f(f64::INFINITY)),
        ("+.Inf", f(f64::INFINITY)),
        ("-.INF", f(f64::NEG_INFINITY)),
        (".nan", f(f64::NAN)),
        ("inf", s("inf")),
        ("nan", s("nan")),
        // str, the default.
        ("abc", s("abc")),
        ("0x1F ", s("0x1F ")),
    ]
}

/// The implicit resolver must agree with the core schema table.
#[test]
fn implicit_resolution_follows_the_core_schema() {
    for (repr, expected) in corpus() {
        assert_eq!(implicit(repr), expected, "implicit resolution of {repr:?}");
    }
}

/// A `!!null`/`!!bool`/`!!int` tag must accept exactly what the implicit resolver resolves to that
/// type, and yield the same value. Without this, writing the tag that asserts a scalar's type is
/// what stops saphyr from parsing it.
#[test]
fn explicit_tag_agrees_with_implicit_resolution() {
    for (repr, expected) in corpus() {
        for (suffix, matches) in [
            ("null", matches!(expected, Scalar::Null)),
            ("bool", matches!(expected, Scalar::Boolean(_))),
            ("int", matches!(expected, Scalar::Integer(_))),
        ] {
            let got = tagged(repr, suffix);
            let want = matches.then(|| expected.clone());
            assert_eq!(got, want, "!!{suffix} {repr:?} vs untagged {expected:?}");
        }
    }
}

/// `!!float` additionally accepts base 10 integers (the float regexp subsumes `[-+]?[0-9]+`, but
/// not the `0o`/`0x` forms), and `!!str` accepts anything.
#[test]
fn float_and_str_tags_accept_their_supersets() {
    for (repr, expected) in corpus() {
        assert_eq!(tagged(repr, "str"), Some(s(repr)), "!!str {repr:?}");
        let want = match expected {
            Scalar::Integer(i) if !repr.starts_with("0o") && !repr.starts_with("0x") => {
                Some(f(i as f64))
            }
            Scalar::FloatingPoint(_) => Some(expected),
            _ => None,
        };
        assert_eq!(tagged(repr, "float"), want, "!!float {repr:?}");
    }
}

#[test]
fn unknown_core_schema_suffix_is_rejected() {
    assert_eq!(tagged("123", "unknown"), None);
}

/// A quoted scalar is a string whatever the tag says.
#[test]
fn quoting_wins_over_the_tag() {
    let tag = Cow::Owned(Tag {
        handle: "tag:yaml.org,2002:".into(),
        suffix: "int".into(),
    });
    for style in [ScalarStyle::SingleQuoted, ScalarStyle::DoubleQuoted] {
        let got = Scalar::parse_from_cow_and_metadata("12".into(), style, Some(&tag));
        assert_eq!(got, Some(s("12")));
    }
}

#[test]
fn tagged_documents_resolve_like_untagged_ones() {
    let load = |s: &str| Yaml::load_from_str(s).unwrap().pop().unwrap();
    for (repr, expected) in [
        ("0x1F", Yaml::Value(Scalar::Integer(31))),
        ("0o17", Yaml::Value(Scalar::Integer(15))),
        ("+7", Yaml::Value(Scalar::Integer(7))),
        ("TRUE", Yaml::Value(Scalar::Boolean(true))),
        ("False", Yaml::Value(Scalar::Boolean(false))),
        ("NULL", Yaml::Value(Scalar::Null)),
        ("~", Yaml::Value(Scalar::Null)),
    ] {
        let suffix = match expected {
            Yaml::Value(Scalar::Integer(_)) => "int",
            Yaml::Value(Scalar::Boolean(_)) => "bool",
            _ => "null",
        };
        assert_eq!(load(repr), expected, "untagged {repr}");
        assert_eq!(
            load(&format!("!!{suffix} {repr}")),
            expected,
            "!!{suffix} {repr}"
        );
    }
    assert_eq!(load("!!null"), Yaml::Value(Scalar::Null));
}

/// An empty node resolves to null, including when it carries an anchor or an alias. The parser
/// only substitutes a `~` for the plain case, so the anchored ones reach the resolver as `""`.
///
/// yaml-test-suite `6KGN`.
#[test]
fn empty_nodes_are_null() {
    let load = |s: &str| Yaml::load_from_str(s).unwrap().pop().unwrap();

    let doc = load("---\na: &anchor\nb: *anchor\n");
    assert!(doc["a"].is_null(), "a: {:?}", doc["a"]);
    assert!(doc["b"].is_null(), "b: {:?}", doc["b"]);

    assert!(load("&anchor\n").is_null());
    for item in load("- &a\n- *a\n").as_sequence().unwrap() {
        assert!(item.is_null(), "{item:?}");
    }

    // Quoting still forces a string.
    assert_eq!(load("a: ''\n")["a"], Yaml::Value(s("")));
}
