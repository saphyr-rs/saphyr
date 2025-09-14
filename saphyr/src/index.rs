/// A trait to index without panicking into a structure through an [`Accessor`].
///
/// [`SafelyIndex`] is implemented on YAML objects to provide the [`get`] method to conveniently
/// access sequence or mapping elements. This is similar to [`Index`]ing, except that the [`get`]
/// method returns an [`Option`], using [`None`] rather than panicking when the requested index is
/// out of range.
///
/// [`get`]: SafelyIndex::get
/// [`Index`]: std::ops::Index
pub trait SafelyIndex<Node = Self> {
    /// Access a field of the given YAML object.
    ///
    /// # Return
    /// If the given index is valid within `self`, [`Some`] is returned with a reference to the
    /// indexed object. If `self` is not indexable or the index is out of bounds, this function
    /// returns [`None`].
    fn get(&self, key: impl Into<Accessor>) -> Option<&Node>;
}

/// A trait to index without panicking into a structure through an [`Accessor`] (mutable).
///
/// [`SafelyIndexMut`] is implemented on YAML objects to provide the [`get_mut`] method to
/// conveniently access sequence or mapping elements. This is similar to [`IndexMut`]ing, except
/// that the [`get_mut`] method returns an [`Option`], using [`None`] rather than panicking when
/// the requested index is out of range.
///
/// [`get_mut`]: SafelyIndexMut::get_mut
/// [`IndexMut`]: std::ops::Index
pub trait SafelyIndexMut<Node = Self> {
    /// Access a field of the given YAML object (mutable).
    ///
    /// # Return
    /// If the given index is valid within `self`, [`Some`] is returned with a reference to the
    /// indexed object. If `self` is not indexable or the index is out of bounds, this function
    /// returns [`None`].
    fn get_mut(&mut self, key: impl Into<Accessor>) -> Option<&mut Node>;
}

/// A [`SafelyIndex`] / [`SafelyIndexMut`] accessor.
pub enum Accessor {
    /// Accessing a string field from a mapping.
    Field(String),
    /// Accessing an element from a sequence or a mapping.
    Index(usize),
}

impl From<usize> for Accessor {
    fn from(val: usize) -> Self {
        Accessor::Index(val)
    }
}

impl From<String> for Accessor {
    fn from(val: String) -> Self {
        Accessor::Field(val)
    }
}

impl From<&str> for Accessor {
    fn from(val: &str) -> Self {
        Accessor::Field(val.to_string())
    }
}

impl<Node: SafelyIndex> SafelyIndex<Node> for Option<Node> {
    fn get(&self, key: impl Into<Accessor>) -> Option<&Node> {
        self.as_ref().and_then(|data| data.get(key))
    }
}

impl<Node: SafelyIndexMut> SafelyIndexMut<Node> for Option<Node> {
    fn get_mut(&mut self, key: impl Into<Accessor>) -> Option<&mut Node> {
        self.as_mut().and_then(|data| data.get_mut(key))
    }
}

impl<Node: SafelyIndex> SafelyIndex<Node> for Option<&Node> {
    fn get(&self, key: impl Into<Accessor>) -> Option<&Node> {
        self.as_ref().and_then(|data| data.get(key))
    }
}

impl<Node: SafelyIndexMut> SafelyIndexMut<Node> for Option<&mut Node> {
    fn get_mut(&mut self, key: impl Into<Accessor>) -> Option<&mut Node> {
        self.as_mut().and_then(|data| data.get_mut(key))
    }
}
