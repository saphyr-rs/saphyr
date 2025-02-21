/// A trait to safely index into a structure with an `Accessor`.
/// This will never panic and return an `Option::None` on failure.
pub trait SafelyIndex<X = Self> {
    /// Attempt to access a field
    fn get(&self, key: impl Into<Accessor>) -> Option<&X>;
}

/// A way to access fields via the [`SafelyIndex`] trait
pub enum Accessor {
    /// Accessing a string field from a mapping
    Field(String),
    /// Accessing an element from a sequence
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

impl<Z: SafelyIndex> SafelyIndex<Z> for Option<Z> {
    fn get(&self, key: impl Into<Accessor>) -> Option<&Z> {
        self.as_ref().and_then(|data| data.get(key))
    }
}

impl<Z: SafelyIndex> SafelyIndex<Z> for Option<&Z> {
    fn get(&self, key: impl Into<Accessor>) -> Option<&Z> {
        self.as_ref().and_then(|data| data.get(key))
    }
}

impl<T: SafelyIndex> SafelyIndex<T> for &T {
    fn get(&self, key: impl Into<Accessor>) -> Option<&T> {
        (*self).get(key)
    }
}

#[cfg(test)]
mod test {
    use std::string::ToString;

    use crate::{LoadableYamlNode, SafelyIndex, Yaml};

    #[test]
    fn indexing_into_available_structures() {
        let node = Yaml::load_from_str(
            r#"
person:
  name: "Jim Halpert"
  workplace:
    - name: "Dunder Mifflin"
      role: "Manager"
  tenure_years: 5
"#,
        )
        .unwrap()
        .first()
        .cloned()
        .unwrap();

        let name = node
            .get("person")
            .get("name")
            .and_then(|name| name.as_str())
            .map(ToString::to_string);

        assert!(name.is_some_and(|n| n == "Jim Halpert"));

        let role = node
            .get("person")
            .get("workplace")
            .get(0)
            .get("role".to_string()) //just for example...
            .and_then(|role| role.as_str())
            .map(ToString::to_string);

        assert!(role.is_some_and(|r| r == "Manager"));
    }
}
