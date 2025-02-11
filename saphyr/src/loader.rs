//! The default loader.

use std::{borrow::Cow, collections::BTreeMap, marker::PhantomData, sync::Arc};

use hashlink::LinkedHashMap;
use saphyr_parser::{Event, ScanError, Span, SpannedEventReceiver, TScalarStyle, Tag};

use crate::{Mapping, Scalar, Yaml};

/// Main structure for parsing YAML.
///
/// The `YamlLoader` may load raw YAML documents or add metadata if needed. The type of the `Node`
/// dictates what data and metadata the loader will add to the `Node`.
///
/// Each node must implement [`LoadableYamlNode`]. The methods are required for the loader to
/// manipulate and populate the `Node`.
#[allow(clippy::module_name_repetitions)]
pub struct YamlLoader<'input, Node>
where
    Node: LoadableYamlNode<'input>,
{
    /// The different YAML documents that are loaded.
    docs: Vec<Node>,
    // states
    // (current node, anchor_id) tuple
    doc_stack: Vec<(Node, usize)>,
    key_stack: Vec<Node>,
    anchor_map: BTreeMap<usize, Node>,
    marker: PhantomData<&'input u32>,
    /// See [`Self::early_parse()`]
    early_parse: bool,
}

// For some reason, rustc wants `Node: Default` if I `#[derive(Default)]`.
impl<'input, Node> Default for YamlLoader<'input, Node>
where
    Node: LoadableYamlNode<'input>,
{
    fn default() -> Self {
        Self {
            docs: vec![],
            doc_stack: vec![],
            key_stack: vec![],
            anchor_map: BTreeMap::new(),
            marker: PhantomData,
            early_parse: true,
        }
    }
}

impl<'input, Node> SpannedEventReceiver<'input> for YamlLoader<'input, Node>
where
    Node: LoadableYamlNode<'input>,
{
    fn on_event(&mut self, ev: Event<'input>, span: Span) {
        match ev {
            Event::DocumentStart(_) | Event::Nothing | Event::StreamStart | Event::StreamEnd => {
                // do nothing
            }
            Event::DocumentEnd => {
                match self.doc_stack.len() {
                    // empty document
                    0 => self
                        .docs
                        .push(Node::from_bare_yaml(Yaml::BadValue).with_span(span)),
                    1 => self.docs.push(self.doc_stack.pop().unwrap().0),
                    _ => unreachable!(),
                }
            }
            Event::SequenceStart(aid, _) => {
                self.doc_stack.push((
                    Node::from_bare_yaml(Yaml::Sequence(Vec::new())).with_span(span),
                    aid,
                ));
            }
            Event::SequenceEnd => {
                let node = self.doc_stack.pop().unwrap();
                self.insert_new_node(node);
            }
            Event::MappingStart(aid, _) => {
                self.doc_stack.push((
                    Node::from_bare_yaml(Yaml::Mapping(Mapping::new())).with_span(span),
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
                let node = if self.early_parse {
                    parse_scalar_node(v, style, &tag)
                } else {
                    Yaml::Representation(v, style, tag)
                };
                self.insert_new_node((Node::from_bare_yaml(node).with_span(span), aid));
            }
            Event::Alias(id) => {
                let n = match self.anchor_map.get(&id) {
                    Some(v) => v.clone(),
                    None => Node::from_bare_yaml(Yaml::BadValue),
                };
                self.insert_new_node((n.with_span(span), 0));
            }
        }
    }
}

impl<'input, Node> YamlLoader<'input, Node>
where
    Node: LoadableYamlNode<'input>,
{
    /// Whether to parse scalars into their value while loading a YAML.
    ///
    /// If set to `true` (default), the loader will attempt to parse scalars into [`Scalar`]s. The
    /// loaded [`Yaml`] nodes will use the [`Value`] variant.
    /// If set to `false`, the loader will skip scalar parsing and only store the string
    /// representation in [`Representation`].
    ///
    /// [`Value`]: Yaml::Value
    /// [`Representation`]: Yaml::Representation
    pub fn early_parse(&mut self, enabled: bool) {
        self.early_parse = enabled;
    }

    /// Return the document nodes from `self`, consuming it in the process.
    #[must_use]
    pub fn into_documents(self) -> Vec<Node> {
        self.docs
    }

    fn insert_new_node(&mut self, node: (Node, usize)) {
        // valid anchor id starts from 1
        if node.1 > 0 {
            self.anchor_map.insert(node.1, node.0.clone());
        }
        if let Some(parent) = self.doc_stack.last_mut() {
            let parent_node = &mut parent.0;
            if parent_node.is_sequence() {
                parent_node.sequence_mut().push(node.0);
            } else if parent_node.is_mapping() {
                let cur_key = self.key_stack.last_mut().unwrap();
                if cur_key.is_badvalue() {
                    // current node is a key
                    *cur_key = node.0;
                } else {
                    // current node is a value
                    let hash = parent_node.mapping_mut();
                    hash.insert(cur_key.take().into(), node.0);
                }
            }
        } else {
            self.doc_stack.push(node);
        }
    }
}

/// Parse a scalar node representation into its value.
///
/// The variant returned by this function will always be a [`Yaml::Value`], unless the tag forces a
/// particular type and the representation cannot be parsed as this type, in which case it returns
/// a [`Yaml::BadValue`].
fn parse_scalar_node<'a>(v: Cow<'a, str>, style: TScalarStyle, tag: &Option<Tag>) -> Yaml<'a> {
    if style != TScalarStyle::Plain {
        Yaml::Value(Scalar::String(v))
    } else if let Some(Tag {
        ref handle,
        ref suffix,
    }) = tag
    {
        if handle == "tag:yaml.org,2002:" {
            match suffix.as_ref() {
                "bool" => match v.parse::<bool>() {
                    Err(_) => Yaml::BadValue,
                    Ok(v) => Yaml::Value(Scalar::Boolean(v)),
                },
                "int" => match v.parse::<i64>() {
                    Err(_) => Yaml::BadValue,
                    Ok(v) => Yaml::Value(Scalar::Integer(v)),
                },
                "float" => match parse_f64(&v) {
                    Some(f) => Yaml::Value(Scalar::FloatingPoint(f.into())),
                    None => Yaml::BadValue,
                },
                "null" => match v.as_ref() {
                    "~" | "null" => Yaml::Value(Scalar::Null),
                    _ => Yaml::BadValue,
                },
                _ => Yaml::Value(Scalar::String(v)),
            }
        } else {
            Yaml::Value(Scalar::String(v))
        }
    } else {
        // Datatype is not specified, or unrecognized
        Yaml::from_cow(v)
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
pub trait LoadableYamlNode<'input>: Clone + std::hash::Hash + Eq {
    /// The type of the key for the hash variant of the YAML node.
    ///
    /// The `HashKey` must be [`Eq`] and [`Hash`] to satisfy the hash map requirements.
    /// It must also be [`Borrow<Self>`] so the hash map can borrow the key to a node and compare
    /// it with a node.
    /// Furthermore, it must be [`From<Self>`] so we can create a key from a node.
    /// Finally, if indexing mappings with `&str` is desired, it must also implement
    /// [`PartialEq<Self>`].
    /// These constraints are also highlighted in [`AnnotatedNode`].
    ///
    /// This indirection is required to solve lifetime issues with the hash map in annotated YAMLs.
    /// More details about the issue and possible workarounds can be found
    /// [here](https://github.com/rust-lang/rust/issues/124614#issuecomment-2090725842). A previous
    /// attempt at solving lifetimes used capsules, but [`AnnotatedNode`] is sufficient.
    ///
    /// [`Hash`]: std::hash::Hash
    /// [`Borrow<Self>`]: std::borrow::Borrow
    /// [`From<Self>`]: From
    /// [`PartialEq<Self>`]: PartialEq
    /// [`AnnotatedNode`]: crate::annotated::AnnotatedNode
    type HashKey: Eq + std::hash::Hash + std::borrow::Borrow<Self> + From<Self>;

    /// Create an instance of `Self` from a [`Yaml`].
    ///
    /// Nodes must implement this to be built. The optional metadata that they contain will be
    /// later provided by the loader and can be default initialized. The [`Yaml`] object passed as
    /// parameter may be of the [`Sequence`] or [`Mapping`] variants. In this event, the inner
    /// container will always be empty. There is no need to traverse all elements to convert them
    /// from [`Yaml`] to `Self`.
    ///
    /// [`Sequence`]: `Yaml::Sequence`
    /// [`Mapping`]: `Yaml::Mapping`
    fn from_bare_yaml(yaml: Yaml<'input>) -> Self;

    /// Return whether the YAML node is an array.
    fn is_sequence(&self) -> bool;

    /// Return whether the YAML node is a hash.
    fn is_mapping(&self) -> bool;

    /// Return whether the YAML node is `BadValue`.
    fn is_badvalue(&self) -> bool;

    /// Retrieve the sequence variant of the YAML node.
    ///
    /// # Panics
    /// This function panics if `self` is not a sequence.
    fn sequence_mut(&mut self) -> &mut Vec<Self>;

    /// Retrieve the mapping variant of the YAML node.
    ///
    /// # Panics
    /// This function panics if `self` is not a mapping.
    fn mapping_mut(&mut self) -> &mut LinkedHashMap<Self::HashKey, Self>;

    /// Take the contained node out of `Self`, leaving a `BadValue` in its place.
    #[must_use]
    fn take(&mut self) -> Self;

    /// Provide the marker for the node (builder-style).
    #[inline]
    #[must_use]
    fn with_span(self, _: Span) -> Self {
        self
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
