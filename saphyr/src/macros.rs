/// Generate `as_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]).
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
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
        Self::$variant(v) => Some(v.into()),
        _ => None
    }
}
    );
);

/// Generate `as_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]), returning references.
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
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

/// Generate `as_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]), returning mutable
/// references.
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
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

/// Generate `into_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]).
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
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
        Self::$variant(v) => Some(v.into()),
        _ => None
    }
}
    );
);

/// Generate `is_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]).
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
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

/// Generate common conversion methods for scalar variants of YAML enums.
///
/// This is used by [`Scalar`].
///
/// [`Scalar`]: crate::Scalar
macro_rules! define_yaml_scalar_conversion_ops (
    () => (
define_is!(is_null, Self::Null);
define_is!(is_boolean, Self::Boolean(_));
define_is!(is_integer, Self::Integer(_));
define_is!(is_floating_point, Self::FloatingPoint(_));
define_is!(is_string, Self::String(_));

define_as!(as_bool, bool, Boolean);
define_as!(as_i64, i64, Integer);
define_as!(as_f64, f64, FloatingPoint);

define_as_ref!(as_str, &str, String);

define_as_mut_ref!(as_mut_bool, &mut bool, Boolean);
define_as_mut_ref!(as_mut_i64, &mut i64, Integer);
define_as_mut_ref!(as_mut_f64, &mut f64, FloatingPoint);
define_as_mut_ref!(as_mut_cow_str, &mut Cow<'input, str>, String);

define_into!(into_boolean, bool, Boolean);
define_into!(into_i64, i64, Integer);
define_into!(into_f64, f64, FloatingPoint);
define_into!(into_string, String, String);
    );
);

/// Generate common methods for all YAML objects ([`Yaml`], [`YamlData`]).
///
/// The generated methods are:
///  - `as_*` access methods (including ref / ref mut versions for mappings, vec and string)
///  - `into_*` conversion methods
///  - `is_*` introspection methods
///  - `or` and `borrowed_or` methods
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
// TODO(ethiraric, 10/02/2025): Use `define_yaml_scalar_conversion_ops`.
macro_rules! define_yaml_object_impl (
    (
        $yaml:ty,
        < $( $generic:tt ),+ >,
        $( where { $($whereclause:tt)+ }, )?
        mappingtype = $mappingtype:ty,
        sequencetype = $sequencetype:ty,
        nodetype = $nodetype:ty
    ) => (
impl< $( $generic ),+ > $yaml $(where $($whereclause)+)? {
    define_as!(as_bool, bool, Boolean);
    define_as!(as_i64, i64, Integer);

    define_as_ref!(as_mapping, &$mappingtype, Mapping);
    define_as_ref!(as_str, &str, String);
    define_as_ref!(as_sequence, &$sequencetype, Sequence);
    define_as_ref!(as_vec, &$sequencetype, Sequence);

    define_as_mut_ref!(as_mut_mapping, &mut $mappingtype, Mapping);
    define_as_mut_ref!(as_mut_sequence, &mut $sequencetype, Sequence);
    define_as_mut_ref!(as_mut_vec, &mut $sequencetype, Sequence);

    define_into!(into_bool, bool, Boolean);
    define_into!(into_mapping, $mappingtype, Mapping);
    define_into!(into_i64, i64, Integer);
    define_into!(into_vec, $sequencetype, Sequence);
    define_into!(into_sequence, $sequencetype, Sequence);

    define_is!(is_alias, Self::Alias(_));
    define_is!(is_sequence, Self::Sequence(_));
    define_is!(is_badvalue, Self::BadValue);
    define_is!(is_boolean, Self::Boolean(_));
    define_is!(is_mapping, Self::Mapping(_));
    define_is!(is_integer, Self::Integer(_));
    define_is!(is_null, Self::Null);
    define_is!(is_real, Self::Real(_));
    define_is!(is_string, Self::String(_));

    /// Get the inner object in the YAML enum if it is a [`String`].
    ///
    /// # Return
    /// If the variant of `self` is `Self::String`, return `Some(String)` with the `String`
    /// contained. Otherwise, return `None`.
    #[must_use]
    pub fn into_string(self) -> Option<String> {
        // We can't use the macro for this variant as we need to `.into_owned` the `Cow`.
        match self {
            Self::String(v) => Some(v.into_owned()),
            _ => None,
        }
    }

    /// Return the `f64` value contained in this YAML node.
    ///
    /// If the node is not a [`Self::Real`] YAML node or its contents is not a valid `f64`
    /// string, `None` is returned.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        if let Self::Real(ref v) = self {
            parse_f64(v)
        } else {
            None
        }
    }

    /// Return the `f64` value contained in this YAML node.
    ///
    /// If the node is not a [`Self::Real`] YAML node or its contents is not a valid `f64`
    /// string, `None` is returned.
    #[must_use]
    pub fn into_f64(self) -> Option<f64> {
        self.as_f64()
    }

    /// If a value is null or otherwise bad (see variants), consume it and
    /// replace it with a given value `other`. Otherwise, return self unchanged.
    ///
    /// ```
    /// use saphyr::Yaml;
    ///
    /// assert_eq!(Yaml::BadValue.or(Yaml::Integer(3)),  Yaml::Integer(3));
    /// assert_eq!(Yaml::Integer(3).or(Yaml::BadValue),  Yaml::Integer(3));
    /// ```
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        match self {
            Self::BadValue | Self::Null => other,
            this => this,
        }
    }

    /// See [`Self::or`] for behavior.
    ///
    /// This performs the same operations, but with borrowed values for less linear pipelines.
    #[must_use]
    pub fn borrowed_or<'a>(&'a self, other: &'a Self) -> &'a Self {
        match self {
            Self::BadValue | Self::Null => other,
            this => this,
        }
    }

    /// Check whether `self` is a [`Self::Mapping`] and that it contains the given key.
    ///
    /// This is equivalent to:
    /// ```ignore
    /// matches!(self, Self::Mapping(ref x) if x.contains_key(&Yaml::<'_>::String(key.into())))
    /// ```
    ///
    /// # Return
    /// If the variant of `self` is `Self::Mapping` and the mapping contains the key, returns `true`.
    /// Otherwise, returns `false`.
    #[must_use]
    pub fn contains_mapping_key(&self, key: &str) -> bool {
        self.as_mapping_get_impl(key).is_some()
    }

    /// Return the value associated to the given key if `self` is a [`Self::Mapping`].
    ///
    /// This is equivalent to:
    /// ```ignore
    /// self.as_mapping().flat_map(|mapping| mapping.get(key))
    /// ```
    ///
    /// # Return
    /// If the variant of `self` is `Self::Mapping` and the mapping contains the key, returns the
    /// value associated with it.
    /// Otherwise, returns `None`.
    #[must_use]
    pub fn as_mapping_get(&self, key: &str) -> Option<&$nodetype> {
        self.as_mapping_get_impl(key)
    }

    /// Return the value associated to the given key if `self` is a [`Self::Mapping`].
    ///
    /// This is equivalent to:
    /// ```ignore
    /// self.as_mapping_mut().flat_map(|mapping| mapping.get_mut(key))
    /// ```
    ///
    /// # Return
    /// If the variant of `self` is `Self::Mapping` and the mapping contains the key, returns the
    /// value associated with it.
    /// Otherwise, returns `None`.
    #[must_use]
    pub fn as_mapping_get_mut(&mut self, key: &str) -> Option<&mut $nodetype> {
        self.as_mapping_get_mut_impl(key)
    }
}
    );
);
