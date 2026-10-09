//! Wrapper around a [YAML scalar](https://yaml.org/spec/1.2.2/#23-scalars).

use alloc::{borrow::Cow, string::String};

use ordered_float::OrderedFloat;
use saphyr_parser::{ScalarStyle, Tag};

/// A canonical decimal integer that does not fit in `i64`.
///
/// Ordering compares canonical decimal text lexicographically, not numerically.
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct StringInteger<'input>(Cow<'input, str>);

impl<'input> StringInteger<'input> {
    /// Validate canonical decimal text and reject values that fit in `i64`.
    #[must_use]
    pub fn new(value: Cow<'input, str>) -> Option<Self> {
        let text = value.as_ref();
        if !is_canonical_decimal(text) || text.parse::<i64>().is_ok() {
            return None;
        }
        Some(Self(value))
    }

    /// Return the canonical decimal representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
    /// An integer value that does not fit in `i64`.
    StringInteger(StringInteger<'input>),
    /// A floating point value ([10.2.1.4 Floating
    /// Point](https://yaml.org/spec/1.2.2/#floating-point)).
    FloatingPoint(OrderedFloat<f64>),
    /// A string ([10.1.1.3 Generic String](https://yaml.org/spec/1.2.2/#generic-string)).
    ///
    /// This variant is used when representing the node in any other representation fails.
    String(Cow<'input, str>),
}

/// The resolved value of a scalar YAML node, freed from borrowing.
///
/// Scalar nodes are any leaf nodes when parsing YAML. In the [10.1 Failsafe
/// Schema](https://yaml.org/spec/1.2.2/#failsafe-schema), they would represent any `!!str` node.
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum ScalarOwned {
    /// A null value ([10.2.1.1 Null](https://yaml.org/spec/1.2.2/#null)).
    Null,
    /// A boolean value ([10.2.1.2 Boolean](https://yaml.org/spec/1.2.2/#boolean)).
    Boolean(bool),
    /// An integer value ([10.2.1.3 Integer](https://yaml.org/spec/1.2.2/#integer)).
    Integer(i64),
    /// An integer value that does not fit in `i64`.
    StringInteger(StringInteger<'static>),
    /// A floating point value ([10.2.1.4 Floating
    /// Point](https://yaml.org/spec/1.2.2/#floating-point)).
    FloatingPoint(OrderedFloat<f64>),
    /// A string ([10.1.1.3 Generic String](https://yaml.org/spec/1.2.2/#generic-string)).
    ///
    /// This variant is used when representing the node in any other representation fails.
    String(String),
}

impl<'input> Scalar<'input> {
    define_yaml_scalar_conversion_ops!(borrowing);

    /// Take ownership of `self` and turn it into a [`ScalarOwned`].
    #[must_use]
    pub fn into_owned(self) -> ScalarOwned {
        match self {
            Self::Null => ScalarOwned::Null,
            Self::Boolean(v) => ScalarOwned::Boolean(v),
            Self::Integer(v) => ScalarOwned::Integer(v),
            Self::StringInteger(v) => {
                ScalarOwned::StringInteger(StringInteger(Cow::Owned(v.0.into_owned())))
            }
            Self::FloatingPoint(v) => ScalarOwned::FloatingPoint(v),
            Self::String(v) => ScalarOwned::String(v.into_owned()),
        }
    }

    /// Parse a scalar node representation into a [`Scalar`].
    ///
    /// If `tag` is not [`None`]:
    ///   - If the handle is `tag:yaml.org,2022:`, attempt to parse as the given suffix. If parsing
    ///     fails or the suffix is unknown, return [`None`].
    ///   - If the handle is unknown, use the fallback parsing schema.
    ///
    /// # Return
    /// Returns the parsed [`Scalar`].
    ///
    /// # Examples
    /// ```
    /// # extern crate alloc;
    /// # use saphyr::{Scalar, ScalarStyle, Tag};
    /// use alloc::borrow::Cow::Owned;
    /// let yaml_handle = "tag:yaml.org,2002:".to_string();
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata("123".into(), ScalarStyle::Plain, None),
    ///     Some(Scalar::Integer(123))
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata(
    ///         "123".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Owned(Tag { handle: yaml_handle.clone(), suffix: "str".into() }))
    ///     ),
    ///     Some(Scalar::String("123".into()))
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata(
    ///         "not a number".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Owned(Tag { handle: yaml_handle.clone(), suffix: "int".into() }))
    ///     ),
    ///     None
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata(
    ///         "No".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Owned(Tag { handle: yaml_handle.clone(), suffix: "bool".into() }))
    ///     ),
    ///     None
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata(
    ///         "123".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Owned(Tag { handle: yaml_handle.clone(), suffix: "unknown".into() }))
    ///     ),
    ///     None
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata(
    ///         "123".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Owned(Tag { handle: "custom".into(), suffix: "a".into() }))
    ///     ),
    ///     Some(Scalar::Integer(123))
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata(
    ///         "123".into(),
    ///         ScalarStyle::SingleQuoted, // Quotation forces interpretation as str.
    ///         Some(&Owned(Tag { handle: "custom".into(), suffix: "a".into() }))
    ///     ),
    ///     Some(Scalar::String("123".into()))
    /// );
    /// ```
    pub fn parse_from_cow_and_metadata(
        v: Cow<'input, str>,
        style: ScalarStyle,
        tag: Option<&Cow<'input, Tag>>,
    ) -> Option<Self> {
        if style != ScalarStyle::Plain {
            // Any quoted scalar is a string.
            Some(Self::String(v))
        } else if let Some(tag) = tag.map(Cow::as_ref) {
            if tag.is_yaml_core_schema() {
                match tag.suffix.as_ref() {
                    "bool" => v.parse::<bool>().ok().map(Self::Boolean),
                    "int" => parse_integer(v).ok(),
                    "float" => parse_core_schema_fp(&v)
                        .map(OrderedFloat)
                        .map(Self::FloatingPoint),
                    "null" => match v.as_ref() {
                        "~" | "null" => Some(Self::Null),
                        _ => None,
                    },
                    "str" => Some(Self::String(v)),
                    // If we have a tag we do not recognize, return `None`.
                    _ => None,
                }
            } else {
                // If we have a tag we do not recognize, parse it regularly.
                // This will sound more intuitive when instance reading tagged scalars like
                // `!degree 50`.
                Some(Self::parse_from_cow(v))
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
        let v = match parse_integer(v) {
            Ok(integer) => return integer,
            Err(v) => v,
        };
        let s = &*v;
        let bytes = s.as_bytes();

        match bytes.len() {
            1 if bytes[0] == b'~' => return Self::Null,
            4 => {
                let f = bytes[0] & 0xDF;
                if f == b'N' && matches!(s, "null" | "Null" | "NULL") {
                    return Self::Null;
                } else if f == b'T' && matches!(s, "true" | "True" | "TRUE") {
                    return Self::Boolean(true);
                }
            }
            5 if matches!(s, "false" | "False" | "FALSE") => {
                return Self::Boolean(false);
            }
            _ => {}
        }

        if let Some(float) = parse_core_schema_fp(s) {
            return Self::FloatingPoint(float.into());
        }

        Self::String(v)
    }
}

impl ScalarOwned {
    define_yaml_scalar_conversion_ops!(owned);

    /// Borrow from `self` to create a [`Scalar`].
    ///
    /// Mutating the [`Scalar`] will not change the values of `self`. This method is meant for
    /// simplifying processing of scalars when owning the data is not required.
    ///
    /// For instance:
    /// ```
    /// # use saphyr::{Scalar, ScalarOwned};
    /// fn process(scalar: &Scalar<'_>) {
    ///   // ...
    /// }
    ///
    /// let scalar = Scalar::Integer(3);
    /// let owned_scalar = ScalarOwned::String("v".into());
    ///
    /// process(&scalar);
    /// // process(&owned_scalar); <-- Would require another implementation of `process` with
    /// //                             `ScalarOwned`.
    /// process(&owned_scalar.as_scalar()); // No need for duplication.
    /// ```
    #[must_use]
    pub fn as_scalar(&self) -> Scalar<'_> {
        match self {
            Self::Null => Scalar::Null,
            Self::Boolean(v) => Scalar::Boolean(*v),
            Self::Integer(v) => Scalar::Integer(*v),
            Self::StringInteger(v) => {
                Scalar::StringInteger(StringInteger(Cow::Borrowed(v.as_str())))
            }
            Self::FloatingPoint(v) => Scalar::FloatingPoint(*v),
            Self::String(v) => Scalar::String(v.as_str().into()),
        }
    }

    /// Parse a scalar node representation into a [`ScalarOwned`].
    ///
    /// # Return
    /// Returns the parsed [`ScalarOwned`].
    ///
    /// If `tag` is not [`None`] and `v` cannot be parsed as that specific tag, this function
    /// returns `None`.
    ///
    /// # Examples
    /// See [`Scalar::parse_from_cow_and_metadata`].
    pub fn parse_from_cow_and_metadata(
        v: Cow<'_, str>,
        style: ScalarStyle,
        tag: Option<&Cow<'_, Tag>>,
    ) -> Option<Self> {
        Scalar::parse_from_cow_and_metadata(v, style, tag).map(Scalar::into_owned)
    }

    /// Parse a scalar node representation into a [`ScalarOwned`].
    ///
    /// This function cannot fail. It will fallback to [`ScalarOwned::String`] if everything else
    /// fails.
    ///
    /// # Return
    /// Returns the parsed [`ScalarOwned`].
    #[must_use]
    pub fn parse_from_cow(v: Cow<'_, str>) -> Self {
        Scalar::parse_from_cow(v).into_owned()
    }
}

fn parse_integer(value: Cow<'_, str>) -> Result<Scalar<'_>, Cow<'_, str>> {
    let text = value.as_ref();
    let bytes = text.as_bytes();
    let (negative, digits, radix) = match bytes.first().copied() {
        Some(b'0') if bytes.get(1) == Some(&b'x') && text.is_char_boundary(2) => {
            (false, &text[2..], 16u16)
        }
        Some(b'0') if bytes.get(1) == Some(&b'o') && text.is_char_boundary(2) => {
            (false, &text[2..], 8u16)
        }
        Some(b'0') if bytes.get(1) == Some(&b'x') || bytes.get(1) == Some(&b'o') => {
            return Err(value);
        }
        Some(b'-') => (true, &text[1..], 10u16),
        Some(b'+') => (false, &text[1..], 10u16),
        _ => (false, text, 10u16),
    };
    if digits.is_empty()
        || !digits.bytes().all(|byte| match radix {
            8 => (b'0'..=b'7').contains(&byte),
            10 => byte.is_ascii_digit(),
            16 => byte.is_ascii_hexdigit(),
            _ => false,
        })
    {
        return Err(value);
    }

    if radix == 10 {
        if let Ok(integer) = text.parse::<i64>() {
            return Ok(Scalar::Integer(integer));
        }
        let canonical_digits = digits.trim_start_matches('0');
        let canonical_digits = if canonical_digits.is_empty() {
            "0"
        } else {
            canonical_digits
        };
        if !negative {
            if let Ok(integer) = canonical_digits.parse::<i64>() {
                return Ok(Scalar::Integer(integer));
            }
        } else if canonical_digits == "0" {
            return Ok(Scalar::Integer(0));
        }
        let canonical = if text == canonical_digits || (negative && digits == canonical_digits) {
            value
        } else if negative {
            Cow::Owned(alloc::format!("-{canonical_digits}"))
        } else {
            Cow::Owned(String::from(canonical_digits))
        };
        return Ok(Scalar::StringInteger(StringInteger(canonical)));
    }

    if let Ok(integer) = i64::from_str_radix(digits, u32::from(radix)) {
        return Ok(Scalar::Integer(integer));
    }

    let mut decimal = alloc::vec![0u8];
    for byte in digits.bytes() {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => unreachable!(),
        };
        let mut carry = u16::from(digit);
        for decimal_digit in &mut decimal {
            let value = u16::from(*decimal_digit) * radix + carry;
            *decimal_digit = (value % 10) as u8;
            carry = value / 10;
        }
        while carry > 0 {
            decimal.push((carry % 10) as u8);
            carry /= 10;
        }
    }
    while decimal.len() > 1 && decimal.last() == Some(&0) {
        decimal.pop();
    }
    let canonical: String = decimal
        .iter()
        .rev()
        .map(|digit| char::from(b'0' + digit))
        .collect();
    Ok(Scalar::StringInteger(StringInteger(Cow::Owned(canonical))))
}

fn is_canonical_decimal(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && !(text.starts_with('-') && digits == "0")
}

impl<'input> From<&'input ScalarOwned> for Scalar<'input> {
    fn from(value: &'input ScalarOwned) -> Self {
        value.as_scalar()
    }
}

/// Parse the given string as a floating point according to the core schema.
///
/// See [10.2.1.4](https://yaml.org/spec/1.2.2/#10214-floating-point) for the floating point
/// definition.
///
/// # Return
/// Returns `Some` if parsing succeeding, `None` otherwise. This function is used in the process of
/// parsing scalars, where failing to parse a scalar as a floating point is not an error. As such,
/// this function purposefully does not return a `Result`.
pub fn parse_core_schema_fp(v: &str) -> Option<f64> {
    match v {
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => Some(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => Some(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => Some(f64::NAN),
        // Test that `v` contains a digit so as not to pass in strings like `inf`,
        // which rust will parse as a float.
        _ if v.as_bytes().iter().any(u8::is_ascii_digit) => v.parse::<f64>().ok(),
        _ => None,
    }
}
