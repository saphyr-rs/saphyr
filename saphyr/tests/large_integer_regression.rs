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
        assert_eq!(value.as_string_integer(), Some(RADIX_DECIMAL));
        assert!(!value.is_integer());
        assert!(value.is_string_integer());
        assert_eq!(value.as_str(), None);
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
        scalar("-1461501637330902918203684832716283019656055999765").as_string_integer(),
        Some("-1461501637330902918203684832716283019656055999765")
    );
}

#[test]
fn radix_overflow_matches_u128_formatting_at_boundaries() {
    for exponent in 63..128 {
        for value in [
            (1u128 << exponent) - 1,
            1u128 << exponent,
            (1u128 << exponent) + 1,
        ] {
            if value <= i64::MAX as u128 {
                continue;
            }
            let decimal = value.to_string();
            let hexadecimal = format!("0x{:x}", value);
            let octal = format!("0o{:o}", value);
            let leading_zero_hexadecimal = format!("0x0{:x}", value);
            let leading_zero_octal = format!("0o0{:o}", value);
            let values = [
                scalar(&decimal),
                scalar(&hexadecimal),
                scalar(&octal),
                scalar(&leading_zero_hexadecimal),
                scalar(&leading_zero_octal),
            ];
            for parsed in &values {
                assert_eq!(parsed.as_string_integer(), Some(decimal.as_str()));
                assert_eq!(parsed, &values[0]);
                assert_eq!(hash(parsed), hash(&values[0]));
            }
        }
    }
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
            assert_eq!(value.as_string_integer(), Some(expected));
        }
    }
    assert_eq!(scalar(&i64::MIN.to_string()), Scalar::Integer(i64::MIN));
    assert_eq!(scalar(&i64::MAX.to_string()), Scalar::Integer(i64::MAX));

    let hundreds = format!("1{}", "0".repeat(300));
    assert_eq!(
        scalar(&hundreds).as_string_integer(),
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
fn malformed_overflow_suffixes_remain_explicit_errors() {
    let cases = [
        format!("{}x", "9".repeat(300)),
        format!("{}_0", "9".repeat(300)),
        format!("{}é", "9".repeat(300)),
        format!("0x{}g", "f".repeat(200)),
        format!("0x{}_0", "f".repeat(200)),
        format!("0x{}é", "f".repeat(200)),
        format!("0o{}8", "7".repeat(300)),
        format!("0o{}_0", "7".repeat(300)),
        format!("0o{}é", "7".repeat(300)),
    ];
    for input in cases {
        assert!(!scalar(&input).is_integer(), "{input}");
        assert!(
            Scalar::parse_from_cow_and_metadata(
                input.clone().into(),
                saphyr::ScalarStyle::Plain,
                Some(&int_tag())
            )
            .is_none(),
            "{input}"
        );
    }
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
    assert!(tagged.is_string_integer());
    assert!(!tagged.is_integer());
    assert!(matches!(
        parse("!!int 9223372036854775808"),
        Yaml::Value(Scalar::StringInteger(_))
    ));
}

#[test]
fn all_scalar_ownership_families_expose_large_integers() {
    let ordinary = scalar("42");
    let owned_ordinary = ordinary.clone().into_owned();
    assert!(ordinary.is_integer());
    assert!(!ordinary.is_string_integer());
    assert!(owned_ordinary.is_integer());
    assert!(!owned_ordinary.is_string_integer());
    assert_eq!(owned_ordinary.as_integer(), Some(42));
    assert_eq!(owned_ordinary.as_string_integer(), None);

    let borrowed = scalar("9223372036854775808");
    let owned = borrowed.clone().into_owned();
    assert_eq!(owned.as_string_integer(), Some("9223372036854775808"));
    assert!(!owned.is_integer());
    assert!(owned.is_string_integer());
    assert_eq!(owned.as_integer(), None);
    assert_eq!(owned.clone().into_i64(), None);
    assert_eq!(owned.as_scalar(), borrowed);

    let parsed_owned = ScalarOwned::parse_from_cow("9223372036854775808".into());
    assert_eq!(
        parsed_owned.as_string_integer(),
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
    assert!(!node.is_integer());
    assert!(node.is_string_integer());
    assert_eq!(node.as_string_integer(), Some("9223372036854775808"));
    assert_eq!(node.as_integer(), None);
    assert_eq!(node.clone().into_integer(), None);
    let owned_node = saphyr::YamlOwned::load_from_str("9223372036854775808")
        .unwrap()
        .remove(0);
    assert!(!owned_node.is_integer());
    assert!(owned_node.is_string_integer());
    assert_eq!(owned_node.as_string_integer(), Some("9223372036854775808"));
    assert_eq!(owned_node.as_integer(), None);
    let ordinary_owned_node = saphyr::YamlOwned::load_from_str("42").unwrap().remove(0);
    assert!(ordinary_owned_node.is_integer());
    assert!(!ordinary_owned_node.is_string_integer());
    assert_eq!(ordinary_owned_node.as_integer(), Some(42));

    let marked = saphyr::MarkedYaml::value_from_str("9223372036854775808");
    assert!(!marked.data.is_integer());
    assert!(marked.data.is_string_integer());
    assert_eq!(marked.data.as_integer(), None);
    assert_eq!(marked.data.as_string_integer(), Some("9223372036854775808"));
    let ordinary_marked = saphyr::MarkedYaml::value_from_str("42");
    assert!(ordinary_marked.data.is_integer());
    assert!(!ordinary_marked.data.is_string_integer());
    assert_eq!(ordinary_marked.data.as_integer(), Some(42));
    let marked_owned = saphyr::MarkedYamlOwned::value_from_str("9223372036854775808");
    assert!(!marked_owned.data.is_integer());
    assert!(marked_owned.data.is_string_integer());
    assert_eq!(marked_owned.data.as_integer(), None);
    assert_eq!(
        marked_owned.data.as_string_integer(),
        Some("9223372036854775808")
    );
    let ordinary_marked_owned = saphyr::MarkedYamlOwned::value_from_str("42");
    assert!(ordinary_marked_owned.data.is_integer());
    assert!(!ordinary_marked_owned.data.is_string_integer());
    assert_eq!(ordinary_marked_owned.data.as_integer(), Some(42));
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
    assert_eq!(deferred.as_string_integer(), Some(source));
}

#[test]
fn map_keys_values_and_emission_round_trip_exactly() {
    let map = parse(
        "{9223372036854775808: one, 9223372036854775809: two, 0x8000000000000000: equivalent}",
    );
    let mapping = map.as_mapping().unwrap();
    assert_eq!(mapping.len(), 2);
    assert!(mapping.keys().all(Yaml::is_string_integer));
    let sequence = parse("[9223372036854775808, 0x8000000000000000]");
    assert!(
        sequence
            .as_vec()
            .unwrap()
            .iter()
            .all(Yaml::is_string_integer)
    );
    let map_value = parse("{value: 9223372036854775808}");
    assert_eq!(
        map_value["value"].as_string_integer(),
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

#[test]
fn is_integer_reports_only_fixed_width_values() {
    assert!(scalar("42").is_integer());
    assert!(!scalar("42").is_string_integer());
    assert_eq!(scalar("42").as_integer(), Some(42));
    assert!(!scalar("9223372036854775808").is_integer());
    assert!(scalar("9223372036854775808").is_string_integer());
    for input in ["true", "null", "1.0", "word"] {
        assert!(!scalar(input).is_integer(), "{input}");
        assert!(!scalar(input).is_string_integer(), "{input}");
    }

    let ordinary_node = parse("42");
    assert!(ordinary_node.is_integer());
    assert!(!ordinary_node.is_string_integer());
    assert_eq!(ordinary_node.as_integer(), Some(42));
    let oversized_node = parse("9223372036854775808");
    assert!(!oversized_node.is_integer());
    assert!(oversized_node.is_string_integer());
    for input in ["true", "null", "1.0", "word"] {
        let node = parse(input);
        assert!(!node.is_integer(), "{input}");
        assert!(!node.is_string_integer(), "{input}");
    }

    let ordinary_marked = saphyr::MarkedYaml::value_from_str("42");
    assert!(ordinary_marked.data.is_integer());
    assert!(!ordinary_marked.data.is_string_integer());
    let oversized_marked = saphyr::MarkedYaml::value_from_str("9223372036854775808");
    assert!(!oversized_marked.data.is_integer());
    assert!(oversized_marked.data.is_string_integer());
    let ordinary_marked_owned = saphyr::MarkedYamlOwned::value_from_str("42");
    assert!(ordinary_marked_owned.data.is_integer());
    assert!(!ordinary_marked_owned.data.is_string_integer());
    let oversized_marked_owned = saphyr::MarkedYamlOwned::value_from_str("9223372036854775808");
    assert!(!oversized_marked_owned.data.is_integer());
    assert!(oversized_marked_owned.data.is_string_integer());
    assert_eq!(
        oversized_marked.data.as_string_integer(),
        Some("9223372036854775808")
    );
    assert_eq!(
        oversized_marked_owned.data.as_string_integer(),
        Some("9223372036854775808")
    );
    for input in ["true", "null", "1.0", "word"] {
        let marked = saphyr::MarkedYaml::value_from_str(input);
        assert!(!marked.data.is_integer(), "{input}");
        assert!(!marked.data.is_string_integer(), "{input}");
        let marked_owned = saphyr::MarkedYamlOwned::value_from_str(input);
        assert!(!marked_owned.data.is_integer(), "{input}");
        assert!(!marked_owned.data.is_string_integer(), "{input}");
    }
}
