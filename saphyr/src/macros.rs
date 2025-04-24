//! Internal helpers for generating code.

/// Generate common conversion methods for scalar variants of YAML enums.
///
/// This is used by [`Scalar`] and [`ScalarOwned`].
///
/// [`Scalar`]: crate::Scalar
/// [`ScalarOwned`]: crate::ScalarOwned
macro_rules! define_yaml_scalar_conversion_ops (
    (owned) => (
define_yaml_scalar_conversion_ops!(base);
define_as_ref_mut!(as_string_mut, &mut String, String);
define_as_ref_mut!(as_str_mut,    &mut str,    String);
    );

    (borrowing) => (
define_yaml_scalar_conversion_ops!(base);
define_as_ref!(as_cow,         &Cow<'input, str>,     String);
define_as_ref_mut!(as_cow_mut, &mut Cow<'input, str>, String);
define_into!(into_cow,         Cow<'input, str>,      String);
define_as_ref_mut_pattern!(as_str_mut,    &mut str => Self::String(ref mut v) => Some(v.to_mut()));
    );

    (base) => ( // Methods common to the owned and borrowing variants.
// ---------- SCALAR CONVERSIONS ----------
define_as!(as_bool,           bool,              Boolean);
define_as!(as_integer,        i64,               Integer);
define_as!(as_floating_point, f64,               FloatingPoint);

define_as_ref!(as_str,        &str,              String);

define_as_ref_mut!(as_bool_mut,           &mut bool,             Boolean);
define_as_ref_mut!(as_integer_mut,        &mut i64,              Integer);
define_as_ref_mut!(as_floating_point_mut, &mut f64,              FloatingPoint);


define_into!(into_boolean, bool,             Boolean);
define_into!(into_i64,     i64,              Integer);
define_into!(into_f64,     f64,              FloatingPoint);
define_into!(into_string,  String,           String);

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
///  - `as_sequence_get` and `as_sequence_get_mut`
///  - `parse_representation` and `parse_representation_recursive`
///  - `value_from_*` methods
///
/// This also calls `define_yaml_object_index_traits_impl`, which creates the [`Index`] and
/// [`IndexMut`] impls.
///
/// [`Yaml`]: crate::Yaml
/// [`YamlData`]: crate::YamlData
/// [`Index`]: std::ops::Index
/// [`IndexMut`]: std::ops::IndexMut
macro_rules! define_yaml_object_impl (
    // ============================ OWNED VARIANT ============================
    (
        $yaml:ty,
        $( < $( $generic:tt ),+ >, )?
        $( where { $($whereclause:tt)+ }, )?
        mappingtype = $mappingtype:ty,
        sequencetype = $sequencetype:ty,
        nodetype = $nodetype:ty,
        scalartype = { $scalartype:tt },
        selfname = $selfname:literal,
        owned
    ) => (
        define_yaml_object_impl!(
            $yaml,
            $( < $($generic),+>, )?
            $(where { $($whereclause)+ }, )?
            mappingtype = $mappingtype,
            sequencetype = $sequencetype,
            nodetype = $nodetype,
            scalartype = { $scalartype },
            selfname = $selfname,
            base
        );
impl $(< $( $generic ),+ >)? $yaml $(where $($whereclause)+)? {
    define_as_ref_mut_pattern!(as_str_mut,            &mut str              => Self::Value($scalartype::String(ref mut v))        => Some(v.as_mut()));

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
                    $scalartype::parse_from_cow_and_metadata(value.into(), style, tag.as_ref())
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
}
    );

    // ============================ BORROWED VARIANT ============================
    (
        $yaml:ty,
        < $( $generic:tt ),+ >,
        $( where { $($whereclause:tt)+ }, )?
        mappingtype = $mappingtype:ty,
        sequencetype = $sequencetype:ty,
        nodetype = $nodetype:ty,
        scalartype = { $scalartype:tt },
        selfname = $selfname:literal,
        borrowing
    ) => (
        define_yaml_object_impl!(
            $yaml,
            < $($generic),+>,
            $(where { $($whereclause)+ }, )?
            mappingtype = $mappingtype,
            sequencetype = $sequencetype,
            nodetype = $nodetype,
            scalartype = { $scalartype },
            selfname = $selfname,
            base
        );
impl< $( $generic ),+ > $yaml $(where $($whereclause)+)? {
    define_as_ref_pattern!(as_cow,                    &Cow<'input, str>     => Self::Value($scalartype::String(ref v))            => Some(v));
    define_as_ref_mut_pattern!(as_cow_mut,            &mut Cow<'input, str> => Self::Value($scalartype::String(ref mut v))        => Some(v));
    define_into_pattern!(into_cow,                    Cow<'input, str>      => Self::Value($scalartype::String(v))                => Some(v));
    define_as_ref_mut_pattern!(as_str_mut,            &mut str              => Self::Value($scalartype::String(ref mut v))        => Some(v.to_mut()));

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
                    $scalartype::parse_from_cow_and_metadata(value.into(), style, tag.as_ref().map(|v| &**v))
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

    /// Convert a string to a scalar node.
    ///
    /// YAML nodes do not implement [`std::str::FromStr`] since the trait requires that conversion
    /// does not fail. This function attempts to parse the given string as a scalar node, falling
    /// back to a [`Scalar::String`].
    ///
    /// **Note:** This attempts to resolve the content as a scalar node. This means that `"a: b"`
    /// gets resolved to `Self::Value(Scalar::String("a: b"))` and not a mapping. If you want to
    /// parse a YAML document, use [`load_from_str`].
    ///
    /// # Examples
    /// ```
    /// # use saphyr::{Scalar, Yaml};
    /// assert!(matches!(Yaml::value_from_str("42"),   Yaml::Value(Scalar::Integer(42))));
    /// assert!(matches!(Yaml::value_from_str("0x2A"), Yaml::Value(Scalar::Integer(42))));
    /// assert!(matches!(Yaml::value_from_str("0o52"), Yaml::Value(Scalar::Integer(42))));
    /// assert!(matches!(Yaml::value_from_str("~"),    Yaml::Value(Scalar::Null)));
    /// assert!(matches!(Yaml::value_from_str("null"), Yaml::Value(Scalar::Null)));
    /// assert!(matches!(Yaml::value_from_str("true"), Yaml::Value(Scalar::Boolean(true))));
    /// assert!(matches!(Yaml::value_from_str("3.14"), Yaml::Value(Scalar::FloatingPoint(_))));
    /// assert!(matches!(Yaml::value_from_str("foo"),  Yaml::Value(Scalar::String(_))));
    /// ```
    ///
    /// [`load_from_str`]: crate::LoadableYamlNode::load_from_str
    #[must_use]
    pub fn value_from_str(v: &'input str) -> Self {
        Self::value_from_cow(v.into())
    }

    /// Same as [`Self::value_from_str`] but uses a [`String`] instead.
    #[must_use]
    pub fn scalar_from_string(v: String) -> Self {
        Self::value_from_cow(v.into())
    }

    /// Same as [`Self::value_from_str`] but uses a [`Cow`] instead.
    #[must_use]
    pub fn value_from_cow(v: Cow<'input, str>) -> Self {
        Self::Value(Scalar::parse_from_cow(v))
    }

    /// Convert a string to a  scalar node, abiding by the given metadata.
    ///
    /// The variant returned by this function will always be a [`Self::Value`], unless the tag
    /// forces a particular type and the representation cannot be parsed as this type, in which
    /// case it returns a [`Self::BadValue`].
    #[must_use]
    pub fn value_from_cow_and_metadata(
        v: Cow<'input, str>,
        style: ScalarStyle,
        tag: Option<&Tag>,
    ) -> Self {
        Scalar::parse_from_cow_and_metadata(v, style, tag).map_or(Self::BadValue, Self::Value)
    }
}
    );

    // ============================ COMMON TO BOTH ============================
    (
        $yaml:ty,
        $( < $( $generic:tt ),+ >, )?
        $( where { $($whereclause:tt)+ }, )?
        mappingtype = $mappingtype:ty,
        sequencetype = $sequencetype:ty,
        nodetype = $nodetype:ty,
        scalartype = { $scalartype:tt },
        selfname = $selfname:literal,
        base
    ) => (
impl $(< $( $generic ),+ >)? $yaml $(where $($whereclause)+)? {
    // ---------- SCALAR CONVERSIONS ----------
    define_as_pattern!(as_bool,                       bool                  => Self::Value($scalartype::Boolean(v))               => Some(v.into()));
    define_as_pattern!(as_integer,                    i64                   => Self::Value($scalartype::Integer(v))               => Some(v.into()));
    define_as_pattern!(as_floating_point,             f64                   => Self::Value($scalartype::FloatingPoint(v))         => Some(v.into()));
    define_as_ref_pattern!(as_str,                    &str                  => Self::Value($scalartype::String(v))                => Some(v));

    define_as_ref_mut_pattern!(as_bool_mut,           &mut bool             => Self::Value($scalartype::Boolean(ref mut v))       => Some(v));
    define_as_ref_mut_pattern!(as_integer_mut,        &mut i64              => Self::Value($scalartype::Integer(ref mut v))       => Some(v));
    define_as_ref_mut_pattern!(as_floating_point_mut, &mut f64              => Self::Value($scalartype::FloatingPoint(ref mut v)) => Some(v));

    define_into_pattern!(into_bool,                   bool                  => Self::Value($scalartype::Boolean(v))               => Some(v));
    define_into_pattern!(into_integer,                i64                   => Self::Value($scalartype::Integer(v))               => Some(v));
    define_into_pattern!(into_floating_point,         f64                   => Self::Value($scalartype::FloatingPoint(v))         => Some(v.into()));
    define_into_pattern!(into_string,                 String                => Self::Value($scalartype::String(v))                => Some(v.into()));

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
    define_is!(is_boolean,        Self::Value($scalartype::Boolean(_)));
    define_is!(is_integer,        Self::Value($scalartype::Integer(_)));
    define_is!(is_null,           Self::Value($scalartype::Null));
    define_is!(is_floating_point, Self::Value($scalartype::FloatingPoint(_)));
    define_is!(is_string,         Self::Value($scalartype::String(_)));

    define_is!(is_sequence,       Self::Sequence(_));
    define_is!(is_badvalue,       Self::BadValue);
    define_is!(is_mapping,        Self::Mapping(_));
    define_is!(is_alias,          Self::Alias(_));
    define_is!(is_representation, Self::Representation(..));
    define_is!(is_value,          Self::Value(_));

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
            Self::BadValue | Self::Value($scalartype::Null) => other,
            this => this,
        }
    }

    /// See [`Self::or`] for behavior.
    ///
    /// This performs the same operations, but with borrowed values for less linear pipelines.
    #[must_use]
    pub fn borrowed_or<'a>(&'a self, other: &'a Self) -> &'a Self {
        match self {
            Self::BadValue | Self::Value($scalartype::Null) => other,
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

define_yaml_object_index_traits_impl!(
    $yaml,
    $(< $( $generic ),+ >,)?
    $( where { $($whereclause)+ }, )?
    mappingtype = $mappingtype,
    sequencetype = $sequencetype,
    nodetype = $nodetype,
    scalartype = { $scalartype },
    selfname = $selfname
);
    );
);

/// Generate the [`Index`] and [`IndexMut`] impls for all YAML objects.
///
/// This is called by [`define_yaml_object_impl`].
///
/// [`Index`]: std::ops::Index
/// [`IndexMut`]: std::ops::IndexMut
macro_rules! define_yaml_object_index_traits_impl (
    (
        $yaml:ty,
        $(< $( $generic:tt ),+ >,)?
        $( where { $($whereclause:tt)+ }, )?
        mappingtype = $mappingtype:ty,
        sequencetype = $sequencetype:ty,
        nodetype = $nodetype:ty,
        scalartype = { $scalartype:tt },
        selfname = $selfname:literal
    ) => (
impl<'key $(, $($generic),+)? > Index<&'key str> for $yaml $( where $($whereclause)+ )? {
    type Output = $nodetype;

    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`$t::Mapping`].
    fn index(&self, idx: &'key str) -> &$nodetype {
        match self.as_mapping_get_impl(idx) {
            Some(value) => value,
            None => {
                if matches!(self, Self::Mapping(_)) {
                    panic!("Key '{idx}' not found in {} mapping", $selfname)
                } else {
                    panic!("Attempt to index {} with '{idx}' but it's not a mapping", $selfname)
                }
            }
        }
    }
}

impl<'key $(, $($generic),+)?> IndexMut<&'key str> for $yaml $( where $($whereclause)+ )? {
    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`$t::Mapping`].
    fn index_mut(&mut self, idx: &'key str) -> &mut $nodetype {
        assert!(
            matches!(self, Self::Mapping(_)),
            "Attempt to index {} with '{idx}' but it's not a mapping", $selfname
        );
        match self.as_mapping_get_mut_impl(idx) {
            Some(value) => value,
            None => {
                panic!("Key '{idx}' not found in {} mapping", $selfname)
            }
        }
    }
}

impl $(<$($generic),+>)? Index<usize> for $yaml $( where $($whereclause)+ )? {
    type Output = $nodetype;

    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`Index`]). If `self` is a
    /// [`$t::Sequence`], this is when the index is bigger or equal to the length of the underlying
    /// `Vec`. If `self` is a [`$t::Mapping`], this is when the mapping sequence
    /// does not contain [`Scalar::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`$t::Sequence`] nor a [`$t::Mapping`].
    ///
    /// [`Scalar::Integer`]: `crate::Scalar::Integer`
    fn index(&self, idx: usize) -> &$nodetype {
        match self {
            Self::Sequence(sequence) => sequence
                .get(idx)
                .unwrap_or_else(|| panic!("Index {idx} out of bounds in {} sequence", $selfname)),
            Self::Mapping(mapping) => {
                let key = i64::try_from(idx).unwrap_or_else(|_| {
                    panic!("Attempt to index {} mapping with overflowing index", $selfname)
                });
                mapping
                    .get(&Self::Value($scalartype::Integer(key)).into())
                    .unwrap_or_else(|| panic!("Key '{idx}' not found in {} mapping", $selfname))
            }
            _ => {
                panic!(
                    "Attempt to index {} with {idx} but it's not a mapping nor a sequence",
                    $selfname
                );
            }
        }
    }
}

impl $(<$($generic),+>)? IndexMut<usize> for $yaml $( where $($whereclause)+ )? {
    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`IndexMut`]). If `self` is
    /// a [`$t::Sequence`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`$t::Mapping`], this is when the mapping sequence does
    /// not contain [`Scalar::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`$t::Sequence`] nor a [`$t::Mapping`].
    ///
    /// [`Scalar::Integer`]: `crate::Scalar::Integer`
    fn index_mut(&mut self, idx: usize) -> &mut $nodetype {
        match self {
            Self::Sequence(sequence) => sequence
                .get_mut(idx)
                .unwrap_or_else(|| panic!("Index {idx} out of bounds in {} sequence", $selfname)),
            Self::Mapping(mapping) => {
                let key = i64::try_from(idx).unwrap_or_else(|_| {
                    panic!("Attempt to index {} mapping with overflowing index", $selfname)
                });
                mapping
                    .get_mut(&Self::Value($scalartype::Integer(key)).into())
                    .unwrap_or_else(|| panic!("Key {idx} not found in {} mapping", $selfname))
            }
            _ => {
                panic!(
                    "Attempt to index {} with {idx} but it's not a mapping nor a sequence",
                    $selfname
                )
            }
        }
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
