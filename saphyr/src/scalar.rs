use std::borrow::Cow;

use ordered_float::OrderedFloat;

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
}
