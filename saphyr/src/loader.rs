//! The default loader.

use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use hashlink::LinkedHashMap;
use saphyr_parser::{
    BufferedInput, Event, Input, Marker, Parser, ScanError, Span, SpannedEventReceiver,
};

use crate::{Mapping, Yaml};

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

    /// Provide the span for the node (builder-style).
    ///
    /// Either [`with_span`] is used (typically for scalars) or both [`with_start_marker`] and
    /// [`with_end_marker`] are used (typically for collections).
    ///
    /// [`with_span`]: `LoadableYamlNode::with_span`
    /// [`with_start_marker`]: `LoadableYamlNode::with_start_marker`
    /// [`with_end_marker`]: `LoadableYamlNode::with_end_marker`
    #[inline]
    #[must_use]
    fn with_span(self, _: Span) -> Self {
        self
    }

    /// Provide the start-marker for the node (builder-style).
    ///
    /// If this method is used by the loader, a call to [`with_end_marker`] will follow later.
    ///
    /// [`with_end_marker`]: `LoadableYamlNode::with_end_marker`
    #[inline]
    #[must_use]
    fn with_start_marker(self, _: Marker) -> Self {
        self
    }

    /// Provide the end-marker for the node (builder-style).
    ///
    /// This method is called after a call to [`with_start_marker`].
    ///
    /// [`with_start_marker`]: `LoadableYamlNode::with_start_marker`
    #[inline]
    #[must_use]
    fn with_end_marker(self, _: Marker) -> Self {
        self
    }

    /// Load the given string as an array of YAML documents.
    ///
    /// The `source` is interpreted as YAML documents and is parsed. Parsing succeeds if and only
    /// if all documents are parsed successfully. An error in a latter document prevents the former
    /// from being returned.
    ///
    /// Most often, only one document is loaded in a YAML string. In this case, only the first element
    /// of the returned `Vec` will be used. Otherwise, each element in the `Vec` is a document:
    ///
    /// ```
    /// use saphyr::{LoadableYamlNode, Scalar, Yaml};
    ///
    /// let docs = Yaml::load_from_str(r#"
    /// First document
    /// ---
    /// - Second document
    /// "#).unwrap();
    /// let first_document = &docs[0]; // Select the first YAML document
    /// // The document is a string containing "First document".
    /// assert_eq!(*first_document, Yaml::Value(Scalar::String("First document".into())));
    ///
    /// let second_document = &docs[1]; // Select the second YAML document
    /// // The document is an array containing a single string, "Second document".
    /// assert_eq!(second_document[0], Yaml::Value(Scalar::String("Second document".into())));
    /// ```
    ///
    /// # Errors
    /// Returns [`ScanError`] when loading fails.
    fn load_from_str(source: &str) -> Result<Vec<Self>, ScanError> {
        Self::load_from_iter(source.chars())
    }

    /// Load the contents of the given iterator as an array of YAML documents.
    ///
    /// See [`load_from_str`] for details.
    ///
    /// # Errors
    /// Returns [`ScanError`] when loading fails.
    ///
    /// [`load_from_str`]: LoadableYamlNode::load_from_str
    fn load_from_iter<I: Iterator<Item = char>>(source: I) -> Result<Vec<Self>, ScanError> {
        let mut parser = Parser::new(BufferedInput::new(source));
        Self::load_from_parser(&mut parser)
    }

    /// Load the contents from the specified [`Parser`] as an array of YAML documents.
    ///
    /// See [`load_from_str`] for details.
    ///
    /// # Errors
    /// Returns [`ScanError`] when loading fails.
    ///
    /// [`load_from_str`]: LoadableYamlNode::load_from_str
    fn load_from_parser<I: Input>(parser: &mut Parser<'input, I>) -> Result<Vec<Self>, ScanError> {
        let mut loader = YamlLoader::default();
        parser.load(&mut loader, true)?;
        Ok(loader.into_documents())
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
    /// [`Scalar`]: crate::Scalar
    pub fn early_parse(&mut self, enabled: bool) {
        self.early_parse = enabled;
    }

    /// Return the document nodes from `self`, consuming it in the process.
    #[must_use]
    pub fn into_documents(self) -> Vec<Node> {
        self.docs
    }
}

impl<'input, Node> SpannedEventReceiver<'input> for YamlLoader<'input, Node>
where
    Node: LoadableYamlNode<'input>,
{
    fn on_event(&mut self, ev: Event<'input>, span: Span) {
        let mark = span.start;
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
                    Node::from_bare_yaml(Yaml::Sequence(Vec::new())).with_start_marker(mark),
                    aid,
                ));
            }
            Event::SequenceEnd => {
                let mut node = self.doc_stack.pop().unwrap();
                node.0 = node.0.with_end_marker(mark);
                self.insert_new_node(node);
            }
            Event::MappingStart(aid, _) => {
                self.doc_stack.push((
                    Node::from_bare_yaml(Yaml::Mapping(Mapping::new())).with_start_marker(mark),
                    aid,
                ));
                self.key_stack.push(Node::from_bare_yaml(Yaml::BadValue));
            }
            Event::MappingEnd => {
                self.key_stack.pop().unwrap();
                let mut node = self.doc_stack.pop().unwrap();
                node.0 = node.0.with_end_marker(mark);
                self.insert_new_node(node);
            }
            Event::Scalar(v, style, aid, tag) => {
                let node = if self.early_parse {
                    Yaml::value_from_cow_and_metadata(v, style, tag.as_ref())
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
