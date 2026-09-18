//! The default loader.

use alloc::{borrow::Cow, collections::BTreeMap, vec::Vec};
use core::marker::PhantomData;

use hashlink::LinkedHashMap;
use saphyr_parser::{
    BufferedInput, Event, Input, Marker, Parser, ScanError, Span, SpannedEventReceiver, Tag,
};
use thiserror::Error;

use crate::{Mapping, Yaml};

#[cfg(feature = "encoding")]
use alloc::sync::Arc;

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
    // (current node, anchor_id, tag) tuple
    doc_stack: Vec<(Node, usize, Option<Cow<'input, Tag>>)>,
    key_stack: Vec<Node>,
    anchor_map: BTreeMap<usize, Node>,
    marker: PhantomData<&'input u32>,
    /// See [`Self::early_parse()`]
    early_parse: bool,
    /// See [`Self::alias_node_budget()`]
    alias_node_budget: usize,
    /// Set once alias expansion exceeds `alias_node_budget`. Surfaced by
    /// [`LoadableYamlNode::load_from_parser`].
    alias_error: Option<ScanError>,
}

/// Default budget (in nodes) for anchor alias expansion, used unless
/// [`YamlLoader::alias_node_budget`] overrides it.
///
/// Resolving an [`Event::Alias`] clones the anchor's already-built subtree. Without a
/// limit, a handful of nested anchors can be crafted to expand into an exponential number
/// of nodes (a "billion laughs" attack), exhausting memory. See
/// <https://github.com/saphyr-rs/saphyr/issues/109>.
pub const DEFAULT_ALIAS_NODE_BUDGET: usize = 100_000;

/// A trait providing methods used by the [`YamlLoader`].
///
/// This trait must be implemented on YAML node types (i.e.: [`Yaml`] and annotated YAML nodes). It
/// provides the necessary methods for [`YamlLoader`] to load data into the node.
pub trait LoadableYamlNode<'input>: Clone + core::hash::Hash + Eq {
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
    /// [`Hash`]: core::hash::Hash
    /// [`Borrow<Self>`]: core::borrow::Borrow
    /// [`From<Self>`]: From
    /// [`PartialEq<Self>`]: PartialEq
    /// [`AnnotatedNode`]: crate::annotated::AnnotatedNode
    type HashKey: Eq + core::hash::Hash + core::borrow::Borrow<Self> + From<Self>;

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
    ///
    /// If the YAML node is a tagged variant, this function must inspect the underlying node.
    fn is_sequence(&self) -> bool;

    /// Return whether the YAML node is a hash.
    ///
    /// If the YAML node is a tagged variant, this function must inspect the underlying node.
    fn is_mapping(&self) -> bool;

    /// Return whether the YAML node is `BadValue`.
    fn is_badvalue(&self) -> bool;

    /// Retrieve the sequence variant of the YAML node.
    ///
    /// If the YAML node is a tagged variant, this function must inspect the underlying node.
    ///
    /// # Panics
    /// This function panics if `self` is not a sequence.
    fn sequence_mut(&mut self) -> &mut Vec<Self>;

    /// Retrieve the mapping variant of the YAML node.
    ///
    /// If the YAML node is a tagged variant, this function must inspect the underlying node.
    ///
    /// # Panics
    /// This function panics if `self` is not a mapping.
    fn mapping_mut(&mut self) -> &mut LinkedHashMap<Self::HashKey, Self>;

    /// Turn `self` into a `Tagged` node, using the given tag.
    ///
    /// # Return
    /// Returns a new instance of `Self` of `Tagged` variant with `tag` as the tag and `self` as
    /// the value.
    #[must_use]
    fn into_tagged(self, tag: Cow<'input, Tag>) -> Self;

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
        if let Some(err) = loader.alias_error.take() {
            return Err(err);
        }
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

    /// Set the maximum number of nodes that anchor alias expansion may produce.
    ///
    /// Resolving an [`Event::Alias`] clones the anchor's already-built subtree. Without a
    /// limit, a handful of nested anchors can be crafted to expand into an exponential
    /// number of nodes (a "billion laughs" attack), exhausting memory: see
    /// <https://github.com/saphyr-rs/saphyr/issues/109>. This budget bounds the total
    /// number of nodes that may be produced via alias resolution across the whole load,
    /// beyond which loading fails with a [`ScanError`]. Defaults to
    /// [`DEFAULT_ALIAS_NODE_BUDGET`].
    pub fn alias_node_budget(&mut self, budget: usize) {
        self.alias_node_budget = budget;
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
            Event::SequenceStart(aid, tag) => {
                self.doc_stack.push((
                    Node::from_bare_yaml(Yaml::Sequence(Vec::new())).with_start_marker(mark),
                    aid,
                    tag,
                ));
            }
            Event::MappingStart(aid, tag) => {
                self.doc_stack.push((
                    Node::from_bare_yaml(Yaml::Mapping(Mapping::new())).with_start_marker(mark),
                    aid,
                    tag,
                ));
                self.key_stack.push(Node::from_bare_yaml(Yaml::BadValue));
            }
            Event::MappingEnd | Event::SequenceEnd => {
                if ev == Event::MappingEnd {
                    self.key_stack.pop().unwrap();
                }

                let (mut node, anchor_id, tag) = self.doc_stack.pop().unwrap();
                node = node.with_end_marker(mark);
                if let Some(tag) = tag {
                    if !tag.is_yaml_core_schema() {
                        node = node.into_tagged(tag);
                    }
                }
                self.insert_new_node(node, anchor_id, None);
            }
            Event::Scalar(v, style, aid, tag) => {
                let node = if self.early_parse {
                    Yaml::value_from_cow_and_metadata(v, style, tag.as_ref())
                } else {
                    Yaml::Representation(v, style, tag.clone())
                };
                self.insert_new_node(Node::from_bare_yaml(node).with_span(span), aid, tag);
            }
            Event::Alias(id) => {
                let n = self.resolve_alias(id, mark);
                self.insert_new_node(n.with_span(span), 0, None);
            }
        }
    }
}

impl<'input, Node> YamlLoader<'input, Node>
where
    Node: LoadableYamlNode<'input>,
{
    /// Resolve an [`Event::Alias`] by cloning the referenced anchor's subtree.
    ///
    /// Returns a `BadValue` node, without cloning, if the anchor is unknown or if cloning
    /// it would push the total number of alias-produced nodes past `self.alias_node_budget`
    /// (in which case `self.alias_error` is set, once). The anchor's size is established by
    /// walking it directly (via a mutable borrow, so no clone is needed just to measure it),
    /// and that walk stops as soon as the budget is exhausted, so a single oversized anchor
    /// cannot be used to force an expensive full traversal either. See
    /// <https://github.com/saphyr-rs/saphyr/issues/109>.
    fn resolve_alias(&mut self, id: usize, mark: Marker) -> Node {
        if self.alias_error.is_some() {
            return Node::from_bare_yaml(Yaml::BadValue);
        }
        let Some(existing) = self.anchor_map.get_mut(&id) else {
            return Node::from_bare_yaml(Yaml::BadValue);
        };
        let mut remaining = self.alias_node_budget;
        if count_nodes_within_budget(existing, &mut remaining) {
            self.alias_node_budget = remaining;
            existing.clone()
        } else {
            self.alias_error = Some(ScanError::new_str(
                mark,
                "alias expansion exceeded the maximum node budget (possible billion-laughs attack)",
            ));
            Node::from_bare_yaml(Yaml::BadValue)
        }
    }

    fn insert_new_node(&mut self, mut node: Node, anchor_id: usize, tag: Option<Cow<'input, Tag>>) {
        // valid anchor id starts from 1
        if anchor_id > 0 {
            self.anchor_map.insert(anchor_id, node.clone());
        }
        if let Some((parent_node, _, _)) = self.doc_stack.last_mut() {
            if let Some(tag) = tag {
                if (node.is_sequence() || node.is_mapping()) && !tag.is_yaml_core_schema() {
                    node = node.into_tagged(tag);
                }
            }
            if parent_node.is_sequence() {
                parent_node.sequence_mut().push(node);
            } else if parent_node.is_mapping() {
                let cur_key = self.key_stack.last_mut().unwrap();
                if cur_key.is_badvalue() {
                    // current node is a key
                    *cur_key = node;
                } else {
                    // current node is a value
                    let hash = parent_node.mapping_mut();
                    hash.insert(cur_key.take().into(), node);
                }
            }
        } else {
            self.doc_stack.push((node, anchor_id, tag));
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
            alias_node_budget: DEFAULT_ALIAS_NODE_BUDGET,
            alias_error: None,
        }
    }
}

/// Recursively count the nodes making up `node`, decrementing `budget` as it goes.
///
/// Stops as soon as `budget` reaches zero and returns `false` in that case, without
/// establishing `node`'s full size (only that it exceeds `budget`). Returns `true` if the
/// whole subtree fits within `budget`, leaving `budget` decremented by `node`'s exact node
/// count so callers can accumulate usage across several calls instead of recomputing it.
///
/// Mapping keys are each counted as a single node rather than recursed into: the
/// `LoadableYamlNode::HashKey` type isn't itself a `LoadableYamlNode`, so its structure
/// isn't visible here. In practice alias-bomb payloads use sequence fan-out (as in the
/// reported attack), so this is not a gap in the defense the budget is meant to provide.
fn count_nodes_within_budget<'input, Node>(node: &mut Node, budget: &mut usize) -> bool
where
    Node: LoadableYamlNode<'input>,
{
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    if node.is_sequence() {
        for child in node.sequence_mut() {
            if !count_nodes_within_budget(child, budget) {
                return false;
            }
        }
    } else if node.is_mapping() {
        for (_, value) in node.mapping_mut().iter_mut() {
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            if !count_nodes_within_budget(value, budget) {
                return false;
            }
        }
    }
    true
}

/// An error that happened when loading a YAML document.
#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum LoadError {
    /// An I/O error.
    #[cfg(feature = "encoding")]
    #[error("{0}")]
    IO(#[source] Arc<std::io::Error>),
    /// An error within the scanner. This indicates a malformed YAML input.
    #[error("{0}")]
    Scan(#[source] ScanError),
    /// A decoding error (e.g.: Invalid UTF-8).
    #[error("{0}")]
    Decode(Cow<'static, str>),
}

#[cfg(feature = "encoding")]
impl From<std::io::Error> for LoadError {
    fn from(error: std::io::Error) -> Self {
        LoadError::IO(Arc::new(error))
    }
}
