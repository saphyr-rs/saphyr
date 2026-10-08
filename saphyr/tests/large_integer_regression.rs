use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use saphyr::{
    LoadableYamlNode, Scalar, ScalarOwned, StringInteger, Tag, Yaml, YamlEmitter, YamlLoader,
};
use saphyr_parser::Parser;

fn int_tag() -> std::borrow::Cow<'static, Tag> {
    std::borrow::Cow::Owned(Tag {
        handle: "tag:yaml.org,2002:".into(),
        suffix: "int".into(),
    })
}

const RADIX_DECIMAL: &str = "1461501637330902918203684832716283019656055999765";
const RADIX_HEX: &str = "0x100000000000000000000000000000000075bcd15";
const RADIX_OCTAL: &str = "0o200000000000000000000000000000000000000000000726746425";

fn parse(input: &str) -> Yaml<'_> {
    Yaml::load_from_str(input).unwrap().remove(0)
}

fn hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn scalar(input: &str) -> Scalar<'_> {
    Scalar::parse_from_cow(input.into())
}

#[test]
fn large_decimal_and_radix_values_keep_exact_integer_identity() {
    let decimal = scalar(RADIX_DECIMAL);
    let hex = scalar(RADIX_HEX);
    let octal = scalar(RADIX_OCTAL);
    for value in [&decimal, &hex, &octal] {
        assert!(matches!(value, Scalar::StringInteger(_)));
        assert_eq!(value.as_large_integer_text(), Some(RADIX_DECIMAL));
        assert!(value.is_integer());
        assert_eq!(value.as_integer(), None);
    }
    assert_eq!(decimal, hex);
    assert_eq!(decimal, octal);
    assert_eq!(hash(&decimal), hash(&hex));
    assert_eq!(hash(&decimal), hash(&octal));
    assert_eq!(decimal.cmp(&hex), std::cmp::Ordering::Equal);

    let next = scalar("1461501637330902918203684832716283019656055999766");
    assert_ne!(decimal, next);
    assert_eq!(
        scalar("-1461501637330902918203684832716283019656055999765").as_large_integer_text(),
        Some("-1461501637330902918203684832716283019656055999765")
    );
}

#[test]
fn canonical_decimal_and_boundary_values_are_preserved() {
    for (input, expected) in [
        ("0", "0"),
        ("-0", "0"),
        ("+0009223372036854775808", "9223372036854775808"),
        ("9223372036854775808", "9223372036854775808"),
        ("9223372036854775809", "9223372036854775809"),
        ("-9223372036854775809", "-9223372036854775809"),
    ] {
        let value = scalar(input);
        if expected.parse::<i64>().is_ok() {
            assert!(matches!(value, Scalar::Integer(_)));
        } else {
            assert_eq!(value.as_large_integer_text(), Some(expected));
        }
    }
    assert_eq!(scalar(&i64::MIN.to_string()), Scalar::Integer(i64::MIN));
    assert_eq!(scalar(&i64::MAX.to_string()), Scalar::Integer(i64::MAX));

    let hundreds = format!("1{}", "0".repeat(300));
    assert_eq!(
        scalar(&hundreds).as_large_integer_text(),
        Some(hundreds.as_str())
    );
}

#[test]
fn malformed_integer_grammar_does_not_become_integer() {
    let long = "9".repeat(400);
    for input in [
        "++7", "+-7", "0x-1", "0o+1", "-0x1", "+0o1", "1_000", "0xg", "0o8", "12tail", "-é", "0xé",
        "0oé",
    ] {
        assert!(!scalar(input).is_integer(), "{input}");
        assert!(
            Scalar::parse_from_cow_and_metadata(
                input.into(),
                saphyr::ScalarStyle::Plain,
                Some(&int_tag())
            )
            .is_none()
        );
    }
    let malformed = format!("{long}tail");
    assert!(!scalar(&malformed).is_integer());
    assert!(
        Scalar::parse_from_cow_and_metadata(
            malformed.into(),
            saphyr::ScalarStyle::Plain,
            Some(&int_tag())
        )
        .is_none()
    );
}

#[test]
fn floats_strings_and_explicit_integer_tags_keep_their_types() {
    for input in ["1.0", "1e3", ".inf", ".nan"] {
        assert!(matches!(scalar(input), Scalar::FloatingPoint(_)), "{input}");
    }
    assert!(matches!(scalar("1e9999"), Scalar::FloatingPoint(_)));
    assert!(matches!(
        parse("!!float 1e3"),
        Yaml::Value(Scalar::FloatingPoint(_))
    ));
    assert!(matches!(
        Scalar::parse_from_cow_and_metadata(
            "9223372036854775808".into(),
            saphyr::ScalarStyle::SingleQuoted,
            Some(&int_tag())
        ),
        Some(Scalar::String(_))
    ));
    assert!(matches!(
        parse("!!str 9223372036854775808"),
        Yaml::Value(Scalar::String(_))
    ));
    let tagged = Scalar::parse_from_cow_and_metadata(
        RADIX_HEX.into(),
        saphyr::ScalarStyle::Plain,
        Some(&int_tag()),
    )
    .unwrap();
    assert_eq!(tagged, scalar(RADIX_DECIMAL));
    assert!(matches!(
        parse("!!int 9223372036854775808"),
        Yaml::Value(Scalar::StringInteger(_))
    ));
}

#[test]
fn all_scalar_ownership_families_expose_large_integers() {
    let borrowed = scalar("9223372036854775808");
    let owned = borrowed.clone().into_owned();
    assert_eq!(owned.as_large_integer_text(), Some("9223372036854775808"));
    assert!(owned.is_integer());
    assert_eq!(owned.as_integer(), None);
    assert_eq!(owned.clone().into_i64(), None);
    assert_eq!(owned.as_scalar(), borrowed);

    let parsed_owned = ScalarOwned::parse_from_cow("9223372036854775808".into());
    assert_eq!(
        parsed_owned.as_large_integer_text(),
        Some("9223372036854775808")
    );
    assert_eq!(
        StringInteger::new("9223372036854775808".into())
            .unwrap()
            .as_str(),
        "9223372036854775808"
    );
    assert!(StringInteger::new("123".into()).is_none());
    assert!(StringInteger::new("01".into()).is_none());

    let node = parse("9223372036854775808");
    assert!(node.is_integer());
    assert_eq!(node.as_large_integer_text(), Some("9223372036854775808"));
    assert_eq!(node.as_integer(), None);
    assert_eq!(node.clone().into_integer(), None);
    let owned_node = saphyr::YamlOwned::load_from_str("9223372036854775808")
        .unwrap()
        .remove(0);
    assert!(owned_node.is_integer());
    assert_eq!(
        owned_node.as_large_integer_text(),
        Some("9223372036854775808")
    );

    let marked = saphyr::MarkedYaml::value_from_str("9223372036854775808");
    assert_eq!(
        marked.data.as_large_integer_text(),
        Some("9223372036854775808")
    );
    let marked_owned = saphyr::MarkedYamlOwned::value_from_str("9223372036854775808");
    assert_eq!(
        marked_owned.data.as_large_integer_text(),
        Some("9223372036854775808")
    );
}

#[test]
fn deferred_representation_resolves_to_the_same_large_integer() {
    let source = "9223372036854775808";
    let mut loader = YamlLoader::<Yaml>::default();
    loader.early_parse(false);
    let mut parser = Parser::new_from_str(source);
    parser.load(&mut loader, true).unwrap();
    let mut deferred = loader.into_documents().remove(0);
    assert!(matches!(deferred, Yaml::Representation(_, _, _)));
    assert!(deferred.parse_representation());
    assert_eq!(deferred.as_large_integer_text(), Some(source));
}

#[test]
fn map_keys_values_and_emission_round_trip_exactly() {
    let map = parse(
        "{9223372036854775808: one, 9223372036854775809: two, 0x8000000000000000: equivalent}",
    );
    let mapping = map.as_mapping().unwrap();
    assert_eq!(mapping.len(), 2);
    assert!(mapping.keys().all(Yaml::is_integer));
    let sequence = parse("[9223372036854775808, 0x8000000000000000]");
    assert!(sequence.as_vec().unwrap().iter().all(Yaml::is_integer));
    let map_value = parse("{value: 9223372036854775808}");
    assert_eq!(
        map_value["value"].as_large_integer_text(),
        Some("9223372036854775808")
    );

    let mut emitted = String::new();
    YamlEmitter::new(&mut emitted).dump(&sequence).unwrap();
    assert_eq!(emitted, "---\n- 9223372036854775808\n- 9223372036854775808");
    let reloaded = parse(&emitted);
    assert_eq!(sequence, reloaded);

    for (value, expected_digits) in [
        (&map, &["9223372036854775808", "9223372036854775809"][..]),
        (&map_value, &["9223372036854775808"][..]),
    ] {
        let mut emitted = String::new();
        YamlEmitter::new(&mut emitted).dump(value).unwrap();
        for digits in expected_digits {
            assert!(emitted.contains(digits));
        }
        assert_eq!(parse(&emitted), *value);
    }

    let large_scalar = parse("9223372036854775808");
    let mut emitted = String::new();
    YamlEmitter::new(&mut emitted).dump(&large_scalar).unwrap();
    assert_eq!(emitted, "---\n9223372036854775808");
    assert_eq!(parse(&emitted), large_scalar);

    let quoted = parse("'9223372036854775808'");
    let mut quoted_output = String::new();
    YamlEmitter::new(&mut quoted_output).dump(&quoted).unwrap();
    assert!(matches!(
        parse(&quoted_output),
        Yaml::Value(Scalar::String(_))
    ));
}
