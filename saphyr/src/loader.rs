//! The default loader.

use std::{collections::BTreeMap, sync::Arc};

use hashlink::LinkedHashMap;
use saphyr_parser::{Event, MarkedEventReceiver, Marker, ScanError, TScalarStyle, Tag};

use crate::{Hash, Yaml};

/// Main structure for parsing YAML.
///
/// The `YamlLoader` may load raw YAML documents or add metadata if needed. The type of the `Node`
/// dictates what data and metadata the loader will add to the `Node`.
///
/// Each node must implement [`LoadableYamlNode`]. The methods are required for the loader to
/// manipulate and populate the `Node`.
#[allow(clippy::module_name_repetitions)]
pub struct YamlLoader<Node>
where
    Node: LoadableYamlNode,
{
    /// The different YAML documents that are loaded.
    docs: Vec<Node>,
    // states
    // (current node, anchor_id) tuple
    doc_stack: Vec<(Node, usize)>,
    key_stack: Vec<Node>,
    anchor_map: BTreeMap<usize, Node>,
}

// For some reason, rustc wants `Node: Default` if I `#[derive(Default)]`.
impl<Node> Default for YamlLoader<Node>
where
    Node: LoadableYamlNode,
{
    fn default() -> Self {
        Self {
            docs: vec![],
            doc_stack: vec![],
            key_stack: vec![],
            anchor_map: BTreeMap::new(),
        }
    }
}

impl<Node> MarkedEventReceiver for YamlLoader<Node>
where
    Node: LoadableYamlNode,
{
    fn on_event(&mut self, ev: Event, marker: Marker) {
        match ev {
            Event::DocumentStart | Event::Nothing | Event::StreamStart | Event::StreamEnd => {
                // do nothing
            }
            Event::DocumentEnd => {
                match self.doc_stack.len() {
                    // empty document
                    0 => self
                        .docs
                        .push(Node::from_bare_yaml(Yaml::BadValue).with_marker(marker)),
                    1 => self.docs.push(self.doc_stack.pop().unwrap().0),
                    _ => unreachable!(),
                }
            }
            Event::SequenceStart(aid, _) => {
                self.doc_stack.push((
                    Node::from_bare_yaml(Yaml::Array(Vec::new())).with_marker(marker),
                    aid,
                ));
            }
            Event::SequenceEnd => {
                let node = self.doc_stack.pop().unwrap();
                self.insert_new_node(node);
            }
            Event::MappingStart(aid, _) => {
                self.doc_stack.push((
                    Node::from_bare_yaml(Yaml::Hash(Hash::new())).with_marker(marker),
                    aid,
                ));
                self.key_stack.push(Node::from_bare_yaml(Yaml::BadValue));
            }
            Event::MappingEnd => {
                self.key_stack.pop().unwrap();
                let node = self.doc_stack.pop().unwrap();
                self.insert_new_node(node);
            }
            Event::Scalar(v, style, aid, tag) => {
                let node = if style != TScalarStyle::Plain {
                    Yaml::String(v)
                } else if let Some(Tag {
                    ref handle,
                    ref suffix,
                }) = tag
                {
                    if handle == "tag:yaml.org,2002:" {
                        match suffix.as_ref() {
                            "bool" => {
                                // "true" or "false"
                                match v.parse::<bool>() {
                                    Err(_) => Yaml::BadValue,
                                    Ok(v) => Yaml::Boolean(v),
                                }
                            }
                            "int" => match v.parse::<i64>() {
                                Err(_) => Yaml::BadValue,
                                Ok(v) => Yaml::Integer(v),
                            },
                            "float" => match parse_f64(&v) {
                                Some(_) => Yaml::Real(v),
                                None => Yaml::BadValue,
                            },
                            "null" => match v.as_ref() {
                                "~" | "null" => Yaml::Null,
                                _ => Yaml::BadValue,
                            },
                            _ => Yaml::String(v),
                        }
                    } else {
                        Yaml::String(v)
                    }
                } else {
                    // Datatype is not specified, or unrecognized
                    Yaml::from_str(&v)
                };
                self.insert_new_node((Node::from_bare_yaml(node).with_marker(marker), aid));
            }
            Event::Alias(id) => {
                let n = match self.anchor_map.get(&id) {
                    Some(v) => v.clone(),
                    None => Node::from_bare_yaml(Yaml::BadValue),
                };
                self.insert_new_node((n.with_marker(marker), 0));
            }
        }
    }
}

impl<Node> YamlLoader<Node>
where
    Node: LoadableYamlNode,
{
    fn insert_new_node(&mut self, node: (Node, usize)) {
        // valid anchor id starts from 1
        if node.1 > 0 {
            self.anchor_map.insert(node.1, node.0.clone());
        }
        if let Some(parent) = self.doc_stack.last_mut() {
            let parent_node = &mut parent.0;
            if parent_node.is_array() {
                parent_node.array_mut().push(node.0);
            } else if parent_node.is_hash() {
                let cur_key = self.key_stack.last_mut().unwrap();
                // current node is a key
                if cur_key.is_badvalue() {
                    *cur_key = node.0;
                // current node is a value
                } else {
                    let hash = parent_node.hash_mut();
                    hash.insert(cur_key.take(), node.0);
                }
            }
        } else {
            self.doc_stack.push(node);
        }
    }

    /// Return the document nodes from `self`, consuming it in the process.
    #[must_use]
    pub fn into_documents(self) -> Vec<Node> {
        self.docs
    }
}

/// An error that happened when loading a YAML document.
#[derive(Debug, Clone)]
pub enum LoadError {
    /// An I/O error.
    IO(Arc<std::io::Error>),
    /// An error within the scanner. This indicates a malformed YAML input.
    Scan(ScanError),
    /// A decoding error (e.g.: Invalid UTF-8).
    Decode(std::borrow::Cow<'static, str>),
}

impl From<std::io::Error> for LoadError {
    fn from(error: std::io::Error) -> Self {
        LoadError::IO(Arc::new(error))
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match &self {
            LoadError::IO(e) => e,
            LoadError::Scan(e) => e,
            LoadError::Decode(_) => return None,
        })
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::IO(e) => e.fmt(f),
            LoadError::Scan(e) => e.fmt(f),
            LoadError::Decode(e) => e.fmt(f),
        }
    }
}

/// A trait providing methods used by the [`YamlLoader`].
///
/// This trait must be implemented on YAML node types (i.e.: [`Yaml`] and annotated YAML nodes). It
/// provides the necessary methods for [`YamlLoader`] to load data into the node.
pub trait LoadableYamlNode: Clone + std::hash::Hash + Eq {
    /// Create an instance of `Self` from a [`Yaml`].
    ///
    /// Nodes must implement this to be built. The optional metadata that they contain will be
    /// later provided by the loader and can be default initialized. The [`Yaml`] object passed as
    /// parameter may be of the [`Array`] or [`Hash`] variants. In this event, the inner container
    /// will always be empty. There is no need to traverse all elements to convert them from
    /// [`Yaml`] to `Self`.
    ///
    /// [`Array`]: `Yaml::Array`
    /// [`Hash`]: `Yaml::Hash`
    fn from_bare_yaml(yaml: Yaml) -> Self;

    /// Return whether the YAML node is an array.
    fn is_array(&self) -> bool;

    /// Return whether the YAML node is a hash.
    fn is_hash(&self) -> bool;

    /// Return whether the YAML node is `BadValue`.
    fn is_badvalue(&self) -> bool;

    /// Retrieve the array variant of the YAML node.
    ///
    /// # Panics
    /// This function panics if `self` is not an array.
    fn array_mut(&mut self) -> &mut Vec<Self>;

    /// Retrieve the hash variant of the YAML node.
    ///
    /// # Panics
    /// This function panics if `self` is not a hash.
    fn hash_mut(&mut self) -> &mut LinkedHashMap<Self, Self>;

    /// Take the contained node out of `Self`, leaving a `BadValue` in its place.
    #[must_use]
    fn take(&mut self) -> Self;

    /// Provide the marker for the node (builder-style).
    #[inline]
    #[must_use]
    fn with_marker(self, _: Marker) -> Self {
        self
    }
}

impl LoadableYamlNode for Yaml {
    fn from_bare_yaml(yaml: Yaml) -> Self {
        yaml
    }

    fn is_array(&self) -> bool {
        matches!(self, Yaml::Array(_))
    }

    fn is_hash(&self) -> bool {
        matches!(self, Yaml::Hash(_))
    }

    fn is_badvalue(&self) -> bool {
        matches!(self, Yaml::BadValue)
    }

    fn array_mut(&mut self) -> &mut Vec<Self> {
        if let Yaml::Array(x) = self {
            x
        } else {
            panic!("Called array_mut on a non-array");
        }
    }

    fn hash_mut(&mut self) -> &mut LinkedHashMap<Self, Self> {
        if let Yaml::Hash(x) = self {
            x
        } else {
            panic!("Called hash_mut on a non-hash");
        }
    }

    fn take(&mut self) -> Self {
        let mut taken_out = Yaml::BadValue;
        std::mem::swap(&mut taken_out, self);
        taken_out
    }
}

// parse f64 as Core schema
// See: https://github.com/chyh1990/yaml-rust/issues/51
pub(crate) fn parse_f64(v: &str) -> Option<f64> {
    match v {
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => Some(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => Some(f64::NEG_INFINITY),
        ".nan" | "NaN" | ".NAN" => Some(f64::NAN),
        _ => v.parse::<f64>().ok(),
    }
}
