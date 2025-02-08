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

/// Generate common methods for all YAML objects ([`Yaml`], [`YamlData`]).
///
/// The generated methods are:
///  - `as_*` access methods (including ref / ref mut versions for hash, vec and string)
///  - `into_*` conversion methods
///  - `is_*` introspection methods
///  - `or` and `borrowed_or` methods
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
macro_rules! define_yaml_object_impl (
    (
        $yaml:ty,
        < $( $generic:tt ),+ >,
        $( where { $($whereclause:tt)+ }, )?
        hashtype = $hashtype:ty,
        arraytype = $arraytype:ty,
        nodetype = $nodetype:ty
    ) => (
impl< $( $generic ),+ > $yaml $(where $($whereclause)+)? {
    define_as!(as_bool, bool, Boolean);
    define_as!(as_i64, i64, Integer);

    define_as_ref!(as_hash, &$hashtype, Hash);
    define_as_ref!(as_str, &str, String);
    define_as_ref!(as_vec, &$arraytype, Array);

    define_as_mut_ref!(as_mut_hash, &mut $hashtype, Hash);
    define_as_mut_ref!(as_mut_vec, &mut $arraytype, Array);

    define_into!(into_bool, bool, Boolean);
    define_into!(into_hash, $hashtype, Hash);
    define_into!(into_i64, i64, Integer);
    define_into!(into_vec, $arraytype, Array);

    define_is!(is_alias, Self::Alias(_));
    define_is!(is_array, Self::Array(_));
    define_is!(is_badvalue, Self::BadValue);
    define_is!(is_boolean, Self::Boolean(_));
    define_is!(is_hash, Self::Hash(_));
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

    /// Check whether `self` is a [`Self::Hash`] and that it contains the given key.
    ///
    /// This is equivalent to:
    /// ```ignore
    /// matches!(self, Self::Hash(ref x) if x.contains_key(&Yaml::<'_>::String(key.into())))
    /// ```
    ///
    /// # Return
    /// If the variant of `self` is `Self::Hash` and the mapping contains the key, returns `true`.
    /// Otherwise, returns `false`.
    #[must_use]
    pub fn contains_mapping_key(&self, key: &str) -> bool {
        self.as_mapping_get_impl(key).is_some()
    }

    /// Return the value associated to the given key if `self` is a [`Self::Hash`].
    ///
    /// This is equivalent to:
    /// ```ignore
    /// self.as_hash().flat_map(|mapping| mapping.get(key))
    /// ```
    ///
    /// # Return
    /// If the variant of `self` is `Self::Hash` and the mapping contains the key, returns the
    /// value associated with it.
    /// Otherwise, returns `None`.
    #[must_use]
    pub fn as_mapping_get(&self, key: &str) -> Option<&$nodetype> {
        self.as_mapping_get_impl(key)
    }

    /// Return the value associated to the given key if `self` is a [`Self::Hash`].
    ///
    /// This is equivalent to:
    /// ```ignore
    /// self.as_hash_mut().flat_map(|mapping| mapping.get_mut(key))
    /// ```
    ///
    /// # Return
    /// If the variant of `self` is `Self::Hash` and the mapping contains the key, returns the
    /// value associated with it.
    /// Otherwise, returns `None`.
    #[must_use]
    pub fn as_mapping_get_mut(&mut self, key: &str) -> Option<&mut $nodetype> {
        self.as_mapping_get_mut_impl(key)
    }
}
    );
);
