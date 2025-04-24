//! Wrapper around a [YAML scalar](https://yaml.org/spec/1.2.2/#23-scalars).

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
    /// # Return
    /// Returns the parsed [`Scalar`].
    ///
    /// If `tag` is not [`None`] and `v` cannot be parsed as that specific tag, this function
    /// returns `None`.
    ///
    /// # Examples
    /// ```
    /// # use saphyr::{Scalar, ScalarStyle, Tag};
    /// use std::borrow::Cow::Owned;
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata("123".into(), ScalarStyle::Plain, None),
    ///     Some(Scalar::Integer(123))
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata(
    ///         "123".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Owned(Tag { handle: "tag:yaml.org,2002:".into(), suffix: "str".into() }))
    ///     ),
    ///     Some(Scalar::String("123".into()))
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata(
    ///         "not a number".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Owned(Tag { handle: "tag:yaml.org,2002:".into(), suffix: "int".into() }))
    ///     ),
    ///     None
    /// );
    /// assert_eq!(
    ///     Scalar::parse_from_cow_and_metadata(
    ///         "No".into(),
    ///         ScalarStyle::Plain,
    ///         Some(&Owned(Tag { handle: "tag:yaml.org,2002:".into(), suffix: "bool".into() }))
    ///     ),
    ///     None
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
        } else if let Some(Tag {
            ref handle,
            ref suffix,
        }) = tag.map(Cow::as_ref)
        {
            if handle == "tag:yaml.org,2002:" {
                match suffix.as_ref() {
                    "bool" => v.parse::<bool>().ok().map(Self::Boolean),
                    "int" => v.parse::<i64>().ok().map(Self::Integer),
                    "float" => parse_f64(&v).map(OrderedFloat).map(Self::FloatingPoint),
                    "null" => match v.as_ref() {
                        "~" | "null" => Some(Self::Null),
                        _ => None,
                    },
                    // If we have a tag we do not recognize, fallback to a string.
                    // If the tag is `str`, this falls here as well.
                    _ => Some(Self::String(v)),
                }
            } else {
                // If we have a tag we do not recognize, fallback to a string.
                Some(Self::String(v))
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
                return Self::Integer(i);
            }
        } else if let Some(number) = v.strip_prefix("0o") {
            if let Ok(i) = i64::from_str_radix(number, 8) {
                return Self::Integer(i);
            }
        } else if let Some(number) = v.strip_prefix('+') {
            if let Ok(i) = number.parse::<i64>() {
                return Self::Integer(i);
            }
        }
        match &*v {
            "~" | "null" | "NULL" => Self::Null,
            "true" => Self::Boolean(true),
            "false" => Self::Boolean(false),
            _ => {
                if let Ok(integer) = v.parse::<i64>() {
                    Self::Integer(integer)
                } else if let Some(float) = parse_f64(&v) {
                    Self::FloatingPoint(float.into())
                } else {
                    Self::String(v)
                }
            }
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
