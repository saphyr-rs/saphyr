/// Generate `as_TYPE` methods for the [`crate::Yaml`] enum.
macro_rules! define_as (
    ($fn_name:ident, $t:ident, $variant:ident) => (
/// Get a copy of the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Self::$variant`, return `Some($t)` with a copy of the `$t` contained.
/// Otherwise, return `None`.
#[must_use]
pub fn $fn_name(&self) -> Option<$t> {
    match *self {
        Self::$variant(v) => Some(v),
        _ => None
    }
}
    );
);

/// Generate `as_TYPE` methods for the [`crate::Yaml`] enum, returning references.
macro_rules! define_as_ref (
    ($fn_name:ident, $t:ty, $variant:ident) => (
/// Get a reference to the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Self::$variant`, return `Some(&$t)` with the `$t` contained. Otherwise,
/// return `None`.
#[must_use]
pub fn $fn_name(&self) -> Option<$t> {
    match *self {
        Self::$variant(ref v) => Some(v),
        _ => None
    }
}
    );
);

/// Generate `as_TYPE` methods for the [`crate::Yaml`] enum, returning mutable references.
macro_rules! define_as_mut_ref (
    ($fn_name:ident, $t:ty, $variant:ident) => (
/// Get a mutable reference to the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Self::$variant`, return `Some(&mut $t)` with the `$t` contained.
/// Otherwise, return `None`.
#[must_use]
pub fn $fn_name(&mut self) -> Option<$t> {
    match *self {
        Self::$variant(ref mut v) => Some(v),
        _ => None
    }
}
    );
);

/// Generate `into_TYPE` methods for the [`crate::Yaml`] enum.
macro_rules! define_into (
    ($fn_name:ident, $t:ty, $variant:ident) => (
/// Get the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Self::$variant`, return `Some($t)` with the `$t` contained. Otherwise,
/// return `None`.
#[must_use]
pub fn $fn_name(self) -> Option<$t> {
    match self {
        Self::$variant(v) => Some(v),
        _ => None
    }
}
    );
);

/// Generate `is_TYPE` methods for the [`crate::Yaml`] enum.
macro_rules! define_is (
    ($fn_name:ident, $variant:pat) => (
/// Check whether the YAML enum contains the given variant.
///
/// # Return
/// If the variant of `self` is `Self::$variant`, return `true`. Otherwise, return `false`.
#[must_use]
pub fn $fn_name(&self) -> bool {
    matches!(self, $variant)
}
    );
);
