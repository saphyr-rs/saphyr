/// Generate common conversion methods for scalar variants of YAML enums.
///
/// This is used by [`Scalar`].
///
/// [`Scalar`]: crate::Scalar
macro_rules! define_yaml_scalar_conversion_ops (
    () => (
// ---------- SCALAR CONVERSIONS ----------
define_as!(as_bool,           bool,              Boolean);
define_as!(as_integer,        i64,               Integer);
define_as!(as_floating_point, f64,               FloatingPoint);

define_as_ref!(as_str,        &str,              String);
define_as_ref!(as_cow,        &Cow<'input, str>, String);

define_as_ref_mut!(as_bool_mut,           &mut bool,             Boolean);
define_as_ref_mut!(as_integer_mut,        &mut i64,              Integer);
define_as_ref_mut!(as_floating_point_mut, &mut f64,              FloatingPoint);
define_as_ref_mut!(as_cow_mut,            &mut Cow<'input, str>, String);

define_as_ref_mut_pattern!(as_str_mut,    &mut str => Self::String(ref mut v) => Some(v.to_mut()));

define_into!(into_boolean, bool,             Boolean);
define_into!(into_i64,     i64,              Integer);
define_into!(into_f64,     f64,              FloatingPoint);
define_into!(into_string,  String,           String);
define_into!(into_cow,     Cow<'input, str>, String);

// ---------- VARIANT TESTING ----------
define_is!(is_null,           Self::Null);
define_is!(is_boolean,        Self::Boolean(_));
define_is!(is_integer,        Self::Integer(_));
define_is!(is_floating_point, Self::FloatingPoint(_));
define_is!(is_string,         Self::String(_));
    );
);

/// Generate common methods for all YAML objects ([`Yaml`], [`YamlData`]).
///
/// The generated methods are:
///  - `as_*` access methods (including ref / ref mut versions for mappings, vec and string)
///  - `into_*` conversion methods
///  - `is_*` introspection methods
///  - `or` and `borrowed_or` methods
///  - `contains_mapping_key`, `as_mapping_get`, `as_mapping_get_mut`
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
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
    // ---------- SCALAR CONVERSIONS ----------
    define_as_pattern!(as_bool,                       bool                  => Self::Value(Scalar::Boolean(v))               => Some(v.into()));
    define_as_pattern!(as_integer,                    i64                   => Self::Value(Scalar::Integer(v))               => Some(v.into()));
    define_as_pattern!(as_floating_point,             f64                   => Self::Value(Scalar::FloatingPoint(v))         => Some(v.into()));
    define_as_ref_pattern!(as_cow,                    &Cow<'input, str>     => Self::Value(Scalar::String(ref v))            => Some(v));
    define_as_ref_pattern!(as_str,                    &str                  => Self::Value(Scalar::String(v))                => Some(v));

    define_as_ref_mut_pattern!(as_bool_mut,           &mut bool             => Self::Value(Scalar::Boolean(ref mut v))       => Some(v));
    define_as_ref_mut_pattern!(as_integer_mut,        &mut i64              => Self::Value(Scalar::Integer(ref mut v))       => Some(v));
    define_as_ref_mut_pattern!(as_floating_point_mut, &mut f64              => Self::Value(Scalar::FloatingPoint(ref mut v)) => Some(v));
    define_as_ref_mut_pattern!(as_cow_mut,            &mut Cow<'input, str> => Self::Value(Scalar::String(ref mut v))        => Some(v));
    define_as_ref_mut_pattern!(as_str_mut,            &mut str              => Self::Value(Scalar::String(ref mut v))        => Some(v.to_mut()));

    define_into_pattern!(into_bool,                   bool                  => Self::Value(Scalar::Boolean(v))               => Some(v));
    define_into_pattern!(into_integer,                i64                   => Self::Value(Scalar::Integer(v))               => Some(v));
    define_into_pattern!(into_floating_point,         f64                   => Self::Value(Scalar::FloatingPoint(v))         => Some(v.into()));
    define_into_pattern!(into_cow,                    Cow<'input, str>      => Self::Value(Scalar::String(v))                => Some(v));
    define_into_pattern!(into_string,                 String                => Self::Value(Scalar::String(v))                => Some(v.into()));

    // ---------- MAPPING / SEQUENCE CONVERSIONS ----------
    define_as_ref!(as_mapping,          &$mappingtype,      Mapping);
    define_as_ref!(as_sequence,         &$sequencetype,     Sequence);
    define_as_ref!(as_vec,              &$sequencetype,     Sequence);

    define_as_ref_mut!(as_mapping_mut,  &mut $mappingtype,  Mapping);
    define_as_ref_mut!(as_sequence_mut, &mut $sequencetype, Sequence);
    define_as_ref_mut!(as_vec_mut,      &mut $sequencetype, Sequence);

    define_into!(into_mapping,          $mappingtype,       Mapping);
    define_into!(into_vec,              $sequencetype,      Sequence);
    define_into!(into_sequence,         $sequencetype,      Sequence);

    // ---------- VARIANT TESTING ----------
    define_is!(is_boolean,        Self::Value(Scalar::Boolean(_)));
    define_is!(is_integer,        Self::Value(Scalar::Integer(_)));
    define_is!(is_null,           Self::Value(Scalar::Null));
    define_is!(is_floating_point, Self::Value(Scalar::FloatingPoint(_)));
    define_is!(is_string,         Self::Value(Scalar::String(_)));

    define_is!(is_sequence,       Self::Sequence(_));
    define_is!(is_badvalue,       Self::BadValue);
    define_is!(is_mapping,        Self::Mapping(_));
    define_is!(is_alias,          Self::Alias(_));
    define_is!(is_representation, Self::Representation(..));
    define_is!(is_value,          Self::Value(_));

    /// If `self` is of the [`Self::Representation`] variant, parse it to the value.
    ///
    /// If `self` was [`Self::Value`], [`Self::Sequence`], [`Self::Mapping`] or [`Self::Alias`]
    /// upon calling, this function does nothing and returns `true`.
    ///
    /// If parsing fails, `*self` is assigned [`Self::BadValue`].
    ///
    /// # Return
    /// Returns `true` if `self` is successfully parsed, `false` otherwise.
    pub fn parse_representation(&mut self) -> bool {
        match self.take() {
            Self::Representation(value, style, tag) => {
                if let Some(scalar) =
                    Scalar::parse_from_cow_and_metadata(value, style, tag.as_ref())
                {
                    *self = Self::Value(scalar);
                    true
                } else {
                    *self = Self::BadValue;
                    false
                }
            }
            _ => true,
        }
    }

    /// Call [`Self::parse_representation`] on `self` and children nodes.
    ///
    /// If `self` was [`Self::Value`] or [`Self::Alias`] upon calling, this function does nothing
    /// and returns `true`.
    ///
    /// If [`Self::parse_representation`] fails on a descendent node, this function will not short
    /// circuit but still attempt to call [`Self::parse_representation`] on further nodes. Even if
    /// all further nodes succeed, this function will still return `false`.
    ///
    /// # Return
    /// Returns `true` if all `self` and its children are successfully parsed, `false` otherwise.
    #[allow(clippy::unnecessary_fold)]
    pub fn parse_representation_recursive(&mut self) -> bool {
        match self.take() {
            mut zelf @ Self::Representation(..) => {
                let succeeded = zelf.parse_representation();
                *self = zelf;
                succeeded
            }
            Self::Sequence(mut vec) => vec
                .iter_mut()
                .map(|v| v.parse_representation_recursive())
                // Using `all` here would short-circuit. We need a `fold` to continue parsing
                // further nodes even if parsing one fails.
                .fold(true, |a, b| a && b),
            Self::Mapping(mut map) => {
                let mut succeeded = true;
                // Keys are immutable. We cannot just do `map.iter_mut().map(...)`. We need to
                // tear apart the hashmap to rebuild it.
                let mut tmp = LinkedHashMap::default();
                std::mem::swap(&mut tmp, &mut map);

                // Turn the temporary into an iterator, call `parse_representation_recursive`
                map = tmp
                    .into_iter()
                    .map(|(mut k, mut v)| {
                        let a = k.parse_representation_recursive();
                        let b = v.parse_representation_recursive();
                        // Trying to fold the booleans whilst keeping an iterator with key-values and
                        // no unnecessary allocations is a pain. It's easier to use a captured
                        // variable.
                        succeeded = succeeded && a && b;
                        (k, v)
                    })
                    // Then collect the result back to our map...
                    .collect::<LinkedHashMap<_, _, _>>();
                // ... for reassigning into `self`.
                *self = Self::Mapping(map);
                succeeded
            }
            _ => true,
        }
    }

    /// If a value is null or otherwise bad (see variants), consume it and
    /// replace it with a given value `other`. Otherwise, return self unchanged.
    ///
    /// ```
    /// # use saphyr::{Scalar, Yaml};
    /// #
    /// assert_eq!(
    ///     Yaml::Value(Scalar::Null).or(Yaml::Value(Scalar::Integer(3))),
    ///     Yaml::Value(Scalar::Integer(3))
    /// );
    /// assert_eq!(
    ///     Yaml::Value(Scalar::Integer(3)).or(Yaml::Value(Scalar::Integer(7))),
    ///     Yaml::Value(Scalar::Integer(3))
    /// );
    /// ```
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        match self {
            Self::BadValue | Self::Value(Scalar::Null) => other,
            this => this,
        }
    }

    /// See [`Self::or`] for behavior.
    ///
    /// This performs the same operations, but with borrowed values for less linear pipelines.
    #[must_use]
    pub fn borrowed_or<'a>(&'a self, other: &'a Self) -> &'a Self {
        match self {
            Self::BadValue | Self::Value(Scalar::Null) => other,
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
    /// self.as_mapping().and_then(|mapping| mapping.get(key))
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
    /// self.as_mapping_mut().and_then(|mapping| mapping.get_mut(key))
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

    /// Return the value at the given index if `self` is a [`Self::Sequence`].
    ///
    /// This is equivalent to:
    /// ```ignore
    /// self.as_sequence().and_then(|seq| seq.get(idx))
    /// ```
    ///
    /// # Return
    /// If the variant of `self` is `Self::Sequence` and the index is not out of bounds, returns
    /// the value at the given index.
    /// Otherwise, returns `None`.
    #[must_use]
    pub fn as_sequence_get(&self, idx:usize) -> Option<&$nodetype> {
        self.as_sequence().and_then(|seq| seq.get(idx))
    }

    /// Return the value at the given index if `self` is a [`Self::Sequence`].
    ///
    /// This is equivalent to:
    /// ```ignore
    /// self.as_sequence_mut().and_then(|seq| seq.get_mut(idx))
    /// ```
    ///
    /// # Return
    /// If the variant of `self` is `Self::Sequence` and the index is not out of bounds, returns
    /// the value at the given index.
    /// Otherwise, returns `None`.
    #[must_use]
    pub fn as_sequence_get_mut(&mut self, idx:usize) -> Option<&mut $nodetype> {
        self.as_sequence_mut().and_then(|seq| seq.get_mut(idx))
    }
}
    );
);

// ================================== HIGH-LEVEL DEFINE MACROS ==================================

/// Generate `as_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]).
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
macro_rules! define_as (
    ($fn_name:ident, $t:ident, $variant:ident) => (
define_as_pattern!($fn_name, $t => Self::$variant(v) => Some(v.into()));
    );
);

/// Generate `as_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]), returning references.
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
macro_rules! define_as_ref (
    ($fn_name:ident, $t:ty, $variant:ident) => (
define_as_ref_pattern!($fn_name, $t => Self::$variant(ref v) => Some(v));
    );
);

/// Generate `as_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]), returning mutable
/// references.
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
macro_rules! define_as_ref_mut (
    ($fn_name:ident, $t:ty, $variant:ident) => (
define_as_ref_mut_pattern!($fn_name, $t => Self::$variant(ref mut v) => Some(v));
    );
);

/// Generate `into_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]).
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
macro_rules! define_into (
    ($fn_name:ident, $t:ty, $variant:ident) => (
define_into_pattern!($fn_name, $t => Self::$variant(v) => Some(v.into()));
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

// ================================== LOW-LEVEL DEFINE MACROS ==================================

/// Generate `as_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]).
///
/// Takes a match arm expression as parameter and pastes it in the `match`.
/// This variant is used explicitly when matching subobjects.
/// If matching a variant of `self`, use [`define_as`].
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
macro_rules! define_as_pattern (
    ($fn_name:ident, $t:ty => $($variant:tt)+ ) => (
/// Get a copy of the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Self::$variant`, return `Some($t)` with a copy of the `$t` contained.
/// Otherwise, return `None`.
#[must_use]
pub fn $fn_name(&self) -> Option<$t> {
    match *self {
        $($variant)+,
        _ => None
    }
}
    );
);

/// Generate `as_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]), returning references.
///
/// Takes a match arm expression as parameter and pastes it in the `match`.
/// This variant is used explicitly when matching subobjects.
/// If matching a variant of `self`, use [`define_as_ref`].
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
macro_rules! define_as_ref_pattern (
    ($fn_name:ident, $t:ty => $($variant:tt)+) => (
/// Get a reference to the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Self::$variant`, return `Some(&$t)` with the `$t` contained. Otherwise,
/// return `None`.
#[must_use]
pub fn $fn_name(&self) -> Option<$t> {
    match self {
        $($variant)+,
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
macro_rules! define_as_ref_mut_pattern (
    ($fn_name:ident, $t:ty => $($variant:tt)+) => (
/// Get a mutable reference to the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Self::$variant`, return `Some(&mut $t)` with the `$t` contained.
/// Otherwise, return `None`.
#[must_use]
pub fn $fn_name(&mut self) -> Option<$t> {
    match *self {
        $($variant)+,
        _ => None
    }
}
    );
);

/// Generate `into_TYPE` methods for YAML objects ([`Yaml`], [`YamlData`]).
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
macro_rules! define_into_pattern (
    ($fn_name:ident, $t:ty => $($variant:tt)+) => (
/// Get the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Self::$variant`, return `Some($t)` with the `$t` contained. Otherwise,
/// return `None`.
#[must_use]
pub fn $fn_name(self) -> Option<$t> {
    match self {
        $($variant)+,
        _ => None
    }
}
    );
);
