//! Wrapper around a [YAML scalar](https://yaml.org/spec/1.2.2/#23-scalars).

use alloc::{borrow::Cow, string::String};

use ordered_float::OrderedFloat;
use saphyr_parser::{ScalarStyle, Tag};

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
                    "bool" => parse_core_schema_bool(&v).map(Self::Boolean),
                    "int" => parse_core_schema_int(&v).map(Self::Integer),
                    "float" => parse_core_schema_fp(&v)
                        .map(OrderedFloat)
                        .map(Self::FloatingPoint),
                    "null" => is_core_schema_null(&v).then_some(Self::Null),
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
        let s = &*v;

        if is_core_schema_null(s) {
            Self::Null
        } else if let Some(b) = parse_core_schema_bool(s) {
            Self::Boolean(b)
        } else if let Some(i) = parse_core_schema_int(s) {
            Self::Integer(i)
        } else if let Some(f) = parse_core_schema_fp(s) {
            Self::FloatingPoint(f.into())
        } else {
            Self::String(v)
        }
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

impl<'input> From<&'input ScalarOwned> for Scalar<'input> {
    fn from(value: &'input ScalarOwned) -> Self {
        value.as_scalar()
    }
}

/// Return whether the given string is a null according to the core schema.
///
/// See [10.2.1.1](https://yaml.org/spec/1.2.2/#10211-null) for the null definition. Note that an
/// empty representation resolves to null, which is how an anchored-but-valueless node such as
/// `key: &anchor` is spelled.
#[must_use]
pub fn is_core_schema_null(v: &str) -> bool {
    matches!(v, "" | "~" | "null" | "Null" | "NULL")
}

/// Parse the given string as a boolean according to the core schema.
///
/// See [10.2.1.2](https://yaml.org/spec/1.2.2/#10212-boolean) for the boolean definition.
///
/// # Return
/// Returns `Some` if parsing succeeded, `None` otherwise. As with [`parse_core_schema_fp`], failing
/// to parse is not an error, so this does not return a `Result`.
#[must_use]
pub fn parse_core_schema_bool(v: &str) -> Option<bool> {
    match v {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

/// Parse the given string as an integer according to the core schema.
///
/// See [10.2.1.3](https://yaml.org/spec/1.2.2/#10213-integer) for the integer definition.
///
/// # Return
/// Returns `Some` if parsing succeeded, `None` otherwise. As with [`parse_core_schema_fp`], failing
/// to parse is not an error, so this does not return a `Result`.
#[must_use]
pub fn parse_core_schema_int(v: &str) -> Option<i64> {
    // `0x`/`0o` take no sign, but `from_str_radix` accepts one, hence the digit check.
    if let Some(digits) = v.strip_prefix("0x") {
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        return i64::from_str_radix(digits, 16).ok();
    }
    if let Some(digits) = v.strip_prefix("0o") {
        if digits.is_empty() || !digits.bytes().all(|b| matches!(b, b'0'..=b'7')) {
            return None;
        }
        return i64::from_str_radix(digits, 8).ok();
    }
    // Base 10 is `[-+]?[0-9]+`, which is exactly what `i64`'s own parser accepts.
    v.parse::<i64>().ok()
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
