//! Scalar resolution, according to the YAML 1.2 Core Schema.

use saphyr::parse_core_schema_fp;
use saphyr_parser::{ScalarStyle, Tag};

use crate::{Error, Result};

/// What a scalar means, when we have no more information than its text, style and tag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Resolved {
    Null,
    Bool(bool),
    Int { negative: bool, magnitude: u128 },
    Float(f64),
    Str,
}

/// Resolve a scalar to a null, boolean, number or string, as the Core Schema says.
///
/// Without a tag, only plain scalars are resolved: any quoted or block scalar is a string. The
/// tags of the Core Schema (`!!int`, `!!str`, ...) force the resolution whatever the style, and
/// fail if the text does not fit. Other tags are ignored.
pub(crate) fn resolve(text: &str, style: ScalarStyle, tag: Option<&Tag>) -> Result<Resolved> {
    let Some(tag) = tag.filter(|tag| tag.is_yaml_core_schema()) else {
        return Ok(if style == ScalarStyle::Plain {
            infer(text)
        } else {
            Resolved::Str
        });
    };
    let resolved = match tag.suffix.as_str() {
        "str" => Some(Resolved::Str),
        "null" => null(text).then_some(Resolved::Null),
        "bool" => boolean(text).map(Resolved::Bool),
        "int" => int(text).map(|(negative, magnitude)| Resolved::Int {
            negative,
            magnitude,
        }),
        "float" => match infer(text) {
            Resolved::Int {
                negative,
                magnitude,
            } => {
                #[allow(clippy::cast_precision_loss)]
                let f = magnitude as f64;
                Some(Resolved::Float(if negative { -f } else { f }))
            }
            float @ Resolved::Float(_) => Some(float),
            _ => None,
        },
        other => {
            return Err(Error::data(format!("unsupported tag `!!{other}`")));
        }
    };
    resolved.ok_or_else(|| Error::data(format!("`{text}` is not a valid `!!{}`", tag.suffix)))
}

/// Guess what a plain, untagged scalar is.
fn infer(text: &str) -> Resolved {
    if null(text) {
        Resolved::Null
    } else if let Some(b) = boolean(text) {
        Resolved::Bool(b)
    } else if let Some((negative, magnitude)) = int(text) {
        Resolved::Int {
            negative,
            magnitude,
        }
    } else if let Some(f) = parse_core_schema_fp(text) {
        Resolved::Float(f)
    } else {
        Resolved::Str
    }
}

fn null(text: &str) -> bool {
    matches!(text, "" | "~" | "null" | "Null" | "NULL")
}

fn boolean(text: &str) -> Option<bool> {
    match text {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

/// Parse `[-+]?[0-9]+`, `0o[0-7]+` and `0x[0-9a-fA-F]+`, as a sign and magnitude.
///
/// Returns `None` if the text is not an integer, or does not fit in a `u128` (an `i128` if
/// negative). So a negative magnitude is at most 2<sup>127</sup>, and can be negated in an
/// `i128` (see [`negate`]).
fn int(text: &str) -> Option<(bool, u128)> {
    let digits_in = |digits: &str, radix: u32| {
        (!digits.is_empty() && digits.bytes().all(|b| char::from(b).is_digit(radix)))
            .then(|| u128::from_str_radix(digits, radix).ok())
            .flatten()
    };
    if let Some(digits) = text.strip_prefix("0x") {
        return digits_in(digits, 16).map(|magnitude| (false, magnitude));
    }
    if let Some(digits) = text.strip_prefix("0o") {
        return digits_in(digits, 8).map(|magnitude| (false, magnitude));
    }
    let (negative, digits) = match text.as_bytes().first()? {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    digits_in(digits, 10)
        .filter(|&magnitude| !negative || magnitude <= NEGATIVE_LIMIT)
        .map(|magnitude| (negative, magnitude))
}

/// The magnitude of `i128::MIN`.
const NEGATIVE_LIMIT: u128 = 1 << 127;

/// The `i128` for a negative number with this magnitude, as returned by [`int`].
pub(crate) fn negate(magnitude: u128) -> i128 {
    debug_assert!(magnitude <= NEGATIVE_LIMIT);
    // 2^127 as an i128 is i128::MIN, whose negation wraps to itself, which is right.
    #[allow(clippy::cast_possible_wrap)]
    (magnitude as i128).wrapping_neg()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> Resolved {
        resolve(text, ScalarStyle::Plain, None).unwrap()
    }

    #[test]
    fn core_schema() {
        assert_eq!(plain(""), Resolved::Null);
        assert_eq!(plain("~"), Resolved::Null);
        assert_eq!(plain("TRUE"), Resolved::Bool(true));
        assert_eq!(plain("yes"), Resolved::Str);
        assert_eq!(plain("y"), Resolved::Str);
        assert_eq!(plain("on"), Resolved::Str);
        assert_eq!(plain("tRue"), Resolved::Str);
        assert_eq!(
            plain("007"),
            Resolved::Int {
                negative: false,
                magnitude: 7
            }
        );
        assert_eq!(
            plain("-7"),
            Resolved::Int {
                negative: true,
                magnitude: 7
            }
        );
        assert_eq!(
            plain("0x1F"),
            Resolved::Int {
                negative: false,
                magnitude: 31
            }
        );
        assert_eq!(
            plain("0o17"),
            Resolved::Int {
                negative: false,
                magnitude: 15
            }
        );
        assert_eq!(plain("-0x1F"), Resolved::Str);
        assert_eq!(plain("0x"), Resolved::Str);
        assert_eq!(plain("+"), Resolved::Str);
        assert_eq!(plain("1_000"), Resolved::Str);
        assert_eq!(plain("1.5"), Resolved::Float(1.5));
        assert_eq!(plain("1e3"), Resolved::Float(1000.0));
        assert_eq!(plain("inf"), Resolved::Str);
        assert_eq!(plain("Infinity"), Resolved::Str);
        assert_eq!(plain(".inf"), Resolved::Float(f64::INFINITY));
        assert!(matches!(plain(".nan"), Resolved::Float(f) if f.is_nan()));
        // Too big for an integer: a float, like `saphyr` does.
        assert!(matches!(plain(&"9".repeat(60)), Resolved::Float(_)));
    }

    #[test]
    fn quoting_forces_strings() {
        assert_eq!(
            resolve("1", ScalarStyle::DoubleQuoted, None).unwrap(),
            Resolved::Str
        );
        assert_eq!(
            resolve("null", ScalarStyle::SingleQuoted, None).unwrap(),
            Resolved::Str
        );
        assert_eq!(
            resolve("", ScalarStyle::Literal, None).unwrap(),
            Resolved::Str
        );
    }

    #[test]
    fn core_tags() {
        let tag = |suffix: &str| Tag {
            handle: "tag:yaml.org,2002:".into(),
            suffix: suffix.into(),
        };
        let r = |text, suffix| resolve(text, ScalarStyle::Plain, Some(&tag(suffix)));
        assert_eq!(r("1", "str").unwrap(), Resolved::Str);
        assert_eq!(r("1", "float").unwrap(), Resolved::Float(1.0));
        assert_eq!(r("true", "str").unwrap(), Resolved::Str);
        assert!(r("a", "int").is_err());
        assert!(r("1", "bool").is_err());
        assert!(r("1", "binary").is_err());
        // Other tags are ignored.
        let custom = Tag {
            handle: "!".into(),
            suffix: "x".into(),
        };
        assert_eq!(
            resolve("1", ScalarStyle::Plain, Some(&custom)).unwrap(),
            Resolved::Int {
                negative: false,
                magnitude: 1
            }
        );
    }
}
