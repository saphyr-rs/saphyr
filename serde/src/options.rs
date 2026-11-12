/// Limits and behaviours of deserialization.
///
/// YAML can be used to craft small documents that are expensive to process: deeply nested
/// collections overflow the stack, and aliases can expand into exponentially more nodes than the
/// document contains (the "billion laughs" attack). The defaults of [`Options`] reject those. A
/// document that goes over a limit fails with [`ErrorKind::LimitExceeded`].
///
/// ```
/// let options = saphyr_serde::Options::default().max_depth(16);
/// let err = saphyr_serde::from_str_with_options::<Vec<()>>(&"[".repeat(17), &options).unwrap_err();
/// assert_eq!(err.kind(), saphyr_serde::ErrorKind::LimitExceeded);
/// ```
///
/// [`ErrorKind::LimitExceeded`]: crate::ErrorKind::LimitExceeded
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub(crate) max_depth: usize,
    pub(crate) max_nodes: usize,
    pub(crate) max_alias_expansion: usize,
    pub(crate) allow_duplicate_keys: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_depth: 128,
            max_nodes: 1_000_000,
            max_alias_expansion: 1_000_000,
            allow_duplicate_keys: false,
        }
    }
}

impl Options {
    /// The maximum nesting depth of collections, aliases included (default: 128).
    ///
    /// Deserialization and serialization recurse with the depth of the data. This is what protects
    /// the stack. When serializing, a value that is nested more deeply is an error as soon as it
    /// goes too deep, which also stops values that contain themselves.
    #[must_use]
    pub fn max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// The maximum number of nodes in one document, as written (default: 1,000,000).
    ///
    /// Aliases are not expanded for this limit, see [`Self::max_alias_expansion`].
    #[must_use]
    pub fn max_nodes(mut self, max_nodes: usize) -> Self {
        self.max_nodes = max_nodes;
        self
    }

    /// The maximum amount of data that aliases may add to one document (default: 1,000,000).
    ///
    /// Each alias counts for the size of the anchored node it refers to, nested aliases included,
    /// so exponential expansions are rejected before being expanded. The size of a node is one
    /// for each node it is made of, and one for each byte of the text of its scalars: an alias to
    /// a long string is not free, as deserializing into a `String` copies it.
    #[must_use]
    pub fn max_alias_expansion(mut self, max_alias_expansion: usize) -> Self {
        self.max_alias_expansion = max_alias_expansion;
        self
    }

    /// Whether a mapping may have the same scalar key twice (default: `false`).
    ///
    /// The YAML specification requires keys to be unique. Silently picking one of the values for a
    /// key is a source of confusion (and of security issues, when different tools pick different
    /// ones), so duplicates are rejected unless this is enabled. Keys are compared by their text,
    /// whatever their style: `1` and `"1"` are duplicates.
    ///
    /// Serialization rejects them too, in the keys that are scalars (this takes memory for the
    /// keys of each mapping that is open, as it must remember them).
    ///
    /// Only the text of scalar keys is compared, not their meaning: `1` and `0x1` are the same
    /// number but not duplicates, and a mapping with keys that are not scalars is not checked.
    /// Different parsers may disagree on which one wins.
    #[must_use]
    pub fn allow_duplicate_keys(mut self, allow_duplicate_keys: bool) -> Self {
        self.allow_duplicate_keys = allow_duplicate_keys;
        self
    }
}
