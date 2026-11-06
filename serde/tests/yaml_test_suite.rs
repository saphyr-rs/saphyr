//! Compare what we deserialize with what `saphyr` loads, for the inputs of the yaml-test-suite.

use std::{fmt, fs, path::Path};

use saphyr::{LoadableYamlNode, Scalar, Yaml};
use saphyr_serde::{ErrorKind, Options};
use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};

/// Any YAML value.
#[derive(Debug)]
enum Any {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    Str(String),
    Seq(Vec<Any>),
    Map(Vec<(Any, Any)>),
}

impl PartialEq for Any {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Any::Null, Any::Null) => true,
            (Any::Bool(a), Any::Bool(b)) => a == b,
            (Any::Int(a), Any::Int(b)) => a == b,
            (Any::Float(a), Any::Float(b)) => a == b || (a.is_nan() && b.is_nan()),
            (Any::Str(a), Any::Str(b)) => a == b,
            (Any::Seq(a), Any::Seq(b)) => a == b,
            (Any::Map(a), Any::Map(b)) => a == b,
            _ => false,
        }
    }
}

impl<'de> Deserialize<'de> for Any {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Any;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("any YAML value")
            }

            fn visit_unit<E>(self) -> Result<Any, E> {
                Ok(Any::Null)
            }

            fn visit_bool<E>(self, v: bool) -> Result<Any, E> {
                Ok(Any::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Any, E> {
                Ok(Any::Int(v.into()))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Any, E> {
                Ok(Any::Int(v.into()))
            }

            fn visit_i128<E>(self, v: i128) -> Result<Any, E> {
                Ok(Any::Int(v))
            }

            fn visit_u128<E>(self, v: u128) -> Result<Any, E> {
                Ok(Any::Int(i128::try_from(v).unwrap_or(i128::MAX)))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Any, E> {
                Ok(Any::Float(v))
            }

            fn visit_str<E>(self, v: &str) -> Result<Any, E> {
                Ok(Any::Str(v.to_owned()))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Any, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Any::Seq(items))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Any, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }
                Ok(Any::Map(entries))
            }
        }
        deserializer.deserialize_any(V)
    }
}

/// `None` if `saphyr` cannot give a value for it.
fn from_saphyr(yaml: &Yaml<'_>) -> Option<Any> {
    Some(match yaml {
        Yaml::Value(Scalar::Null) => Any::Null,
        Yaml::Value(Scalar::Boolean(b)) => Any::Bool(*b),
        Yaml::Value(Scalar::Integer(i)) => Any::Int((*i).into()),
        Yaml::Value(Scalar::FloatingPoint(f)) => Any::Float(f.0),
        Yaml::Value(Scalar::String(s)) => Any::Str(s.to_string()),
        Yaml::Sequence(items) => Any::Seq(items.iter().map(from_saphyr).collect::<Option<_>>()?),
        Yaml::Mapping(entries) => Any::Map(
            entries
                .iter()
                .map(|(k, v)| Some((from_saphyr(k)?, from_saphyr(v)?)))
                .collect::<Option<_>>()?,
        ),
        Yaml::Tagged(_, node) => from_saphyr(node)?,
        Yaml::Representation(..) | Yaml::Alias(_) | Yaml::BadValue => return None,
    })
}

fn visual_to_raw(yaml: &str) -> String {
    let mut yaml = yaml.to_owned();
    for (pat, replacement) in [
        ("␣", " "),
        ("»", "\t"),
        ("—", ""),
        ("←", "\r"),
        ("⇔", "\u{FEFF}"),
        ("↵", ""),
        ("∎\n", ""),
    ] {
        yaml = yaml.replace(pat, replacement);
    }
    yaml
}

#[test]
fn same_as_saphyr() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../parser/tests/yaml-test-suite/src");
    assert!(
        dir.is_dir(),
        "{} not found: the yaml-test-suite submodule is not checked out",
        dir.display()
    );
    let options = Options::default().allow_duplicate_keys(true);
    let mut compared = 0;
    let mut failures = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let file = fs::read_to_string(&path).unwrap();
        let cases = saphyr::Yaml::load_from_str(&file).unwrap();
        let mut inherited_yaml = String::new();
        for (index, case) in cases[0].as_vec().unwrap().iter().enumerate() {
            if let Some(yaml) = case.as_mapping_get("yaml").and_then(Yaml::as_str) {
                inherited_yaml = visual_to_raw(yaml);
            }
            let input = &inherited_yaml;
            let id = format!("{name}-{index}");

            let theirs: Result<Vec<_>, _> = Yaml::load_from_str(input);
            let ours: Result<Vec<Any>, _> =
                saphyr_serde::Deserializer::from_str_with_options(input, &options)
                    .map(|doc| doc.and_then(Any::deserialize))
                    .collect();
            match (theirs, ours) {
                (Ok(theirs), Ok(ours)) => {
                    let theirs: Option<Vec<Any>> = theirs.iter().map(from_saphyr).collect();
                    let Some(theirs) = theirs else { continue };
                    compared += 1;
                    if theirs != ours {
                        failures.push(format!(
                            "{id}: values differ\n{input}\n{theirs:?}\n{ours:?}"
                        ));
                    }
                }
                (Err(_), Err(_)) => compared += 1,
                // Something we are stricter about (like tags we do not support).
                (Ok(_), Err(e)) if e.kind() == ErrorKind::Data => {}
                (Ok(_), Err(e)) => failures.push(format!("{id}: only we fail: {e}\n{input}")),
                // We only check duplicates of scalar keys, and do so in the deserializer.
                (Err(e), Ok(_)) if e.to_string().contains("duplicated key") => {}
                (Err(e), Ok(_)) => failures.push(format!("{id}: only saphyr fails: {e}\n{input}")),
            }
        }
    }
    assert!(compared > 200, "compared {compared}");
    assert!(
        failures.is_empty(),
        "{} failures out of {compared}:\n{}",
        failures.len(),
        failures.join("\n=====\n")
    );
}
