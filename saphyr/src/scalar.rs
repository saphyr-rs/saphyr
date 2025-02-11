use std::borrow::Cow;

use ordered_float::OrderedFloat;
use saphyr_parser::{ScalarStyle, Tag};

use crate::loader::parse_f64;

/// The resolved value of a scalar YAML node.
///
/// Scalar nodes are any leaf nodes when parsing YAML. In the [10.1 Failsafe
/// Schema](https://yaml.org/spec/1.2.2/#failsafe-schema), they would represent any `!!str` node.
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum Scalar<'input> {
    /// A null value ([10.2.1.1 Null](https://yaml.org/spec/1.2.2/#null)).
    Null,
    /// A boolean value ([10.2.1.2 Boolean](https://yaml.org/spec/1.2.2/#boolean)).
    Boolean(bool),
    /// An integer value ([10.2.1.3 Integer](https://yaml.org/spec/1.2.2/#integer)).
    Integer(i64),
    /// A floating point value ([10.2.1.4 Floating
    /// Point](https://yaml.org/spec/1.2.2/#floating-point)).
    FloatingPoint(OrderedFloat<f64>),
    /// A string ([10.1.1.3 Generic String](https://yaml.org/spec/1.2.2/#generic-string)).
    ///
    /// This variant is used when representing the node in any other representation fails.
    String(Cow<'input, str>),
}

impl<'input> Scalar<'input> {
    define_yaml_scalar_conversion_ops!();

    /// Parse a scalar node representation into a [`Scalar`].
    ///
    /// # Return
    /// Returns the parsed [`Scalar`].
    ///
    /// If `tag` is not `None` and `v` cannot be parsed as that specific tag, this function returns
    /// `None`.
    ///
    /// # Examples
    /// ```
    /// # use saphyr::{Scalar, ScalarStyle, Tag};
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_tag("123".into(), ScalarStyle::Plain, None),
    ///     Some(Scalar::Integer(123))
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_tag(
    ///         "123".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Tag { handle: "tag:yaml.org,2002:".into(), suffix: "str".into() })
    ///     ),
    ///     Some(Scalar::String("123".into()))
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_tag(
    ///         "not a number".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Tag { handle: "tag:yaml.org,2002:".into(), suffix: "int".into() })
    ///     ),
    ///     None
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_tag(
    ///         "No".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Tag { handle: "tag:yaml.org,2002:".into(), suffix: "bool".into() })
    ///     ),
    ///     None
    /// );
    /// ```
    pub fn parse_from_cow_and_tag(
        v: Cow<'input, str>,
        style: ScalarStyle,
        tag: Option<&Tag>,
    ) -> Option<Self> {
        if style != ScalarStyle::Plain {
            // Any quoted scalar is a string.
            Some(Scalar::String(v))
        } else if let Some(Tag {
            ref handle,
            ref suffix,
        }) = tag
        {
            if handle == "tag:yaml.org,2002:" {
                match suffix.as_ref() {
                    "bool" => v.parse::<bool>().ok().map(Scalar::Boolean),
                    "int" => v.parse::<i64>().ok().map(Scalar::Integer),
                    "float" => parse_f64(&v).map(OrderedFloat).map(Scalar::FloatingPoint),
                    "null" => match v.as_ref() {
                        "~" | "null" => Some(Scalar::Null),
                        _ => None,
                    },
                    // If we have a tag we do not recognize, fallback to a string.
                    // If the tag is `str`, this falls here as well.
                    _ => Some(Scalar::String(v)),
                }
            } else {
                // If we have a tag we do not recognize, fallback to a string.
                Some(Scalar::String(v))
            }
        } else {
            // No tag means we have to guess.
            Some(Self::parse_from_cow(v))
        }
    }

    /// Parse a scalar node representation into a [`Scalar`].
    ///
    /// This function cannot fail. It will fallback to [`Scalar::String`] if everything else fails.
    ///
    /// # Return
    /// Returns the parsed [`Scalar`].
    #[must_use]
    pub fn parse_from_cow(v: Cow<'input, str>) -> Self {
        if let Some(number) = v.strip_prefix("0x") {
            if let Ok(i) = i64::from_str_radix(number, 16) {
                return Scalar::Integer(i);
            }
        } else if let Some(number) = v.strip_prefix("0o") {
            if let Ok(i) = i64::from_str_radix(number, 8) {
                return Scalar::Integer(i);
            }
        } else if let Some(number) = v.strip_prefix('+') {
            if let Ok(i) = number.parse::<i64>() {
                return Scalar::Integer(i);
            }
        }
        match &*v {
            "~" | "null" | "NULL" => Scalar::Null,
            "true" => Scalar::Boolean(true),
            "false" => Scalar::Boolean(false),
            _ => {
                if let Ok(integer) = v.parse::<i64>() {
                    Scalar::Integer(integer)
                } else if let Some(float) = parse_f64(&v) {
                    Scalar::FloatingPoint(float.into())
                } else {
                    Scalar::String(v)
                }
            }
        }
    }
}
