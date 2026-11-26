//! A bounded, arena-allocated representation of one YAML document.
//!
//! The parser's events are collected into a [`Document`] before deserializing. This is what makes
//! aliases possible (an alias refers to a node that has been seen earlier), and
//! what lets us enforce the [`Options`] limits _before_ any user code runs on the data.
//!
//! Aliases are not expanded: an alias node refers to its target by index. Instead, the size
//! (number of nodes once aliases are expanded) and height (nesting depth once aliases are
//! expanded) of each node are computed as nodes are completed, so that a document that would
//! expand to too much data is rejected without ever being expanded.

use std::{borrow::Cow, collections::HashMap, collections::HashSet, vec::Vec};

use saphyr_parser::{Event, Input, Marker, Parser, ScalarStyle, Tag};

use crate::{Error, Options, Result};

/// The index of a node in [`Document::nodes`].
pub(crate) type NodeId = usize;

pub(crate) struct Node<'de> {
    pub(crate) kind: Kind<'de>,
    pub(crate) tag: Option<Cow<'de, Tag>>,
    pub(crate) marker: Marker,
}

pub(crate) enum Kind<'de> {
    Scalar(Cow<'de, str>, ScalarStyle),
    Sequence(Vec<NodeId>),
    Mapping(Vec<(NodeId, NodeId)>),
    /// A reference to the node anchored earlier. The target is never an alias.
    Alias(NodeId),
}

pub(crate) struct Document<'de> {
    nodes: Vec<Node<'de>>,
    root: NodeId,
}

impl<'de> Document<'de> {
    /// The document of an empty stream: a single empty plain scalar, that is, `null`.
    pub(crate) fn empty() -> Self {
        Self {
            nodes: vec![Node {
                kind: Kind::Scalar(Cow::Borrowed(""), ScalarStyle::Plain),
                tag: None,
                marker: Marker::default(),
            }],
            root: 0,
        }
    }

    pub(crate) fn root(&self) -> NodeId {
        self.root
    }

    pub(crate) fn node(&self, id: NodeId) -> &Node<'de> {
        &self.nodes[id]
    }

    /// Follow an alias to the node it refers to. Other nodes are returned as is.
    pub(crate) fn resolve(&self, id: NodeId) -> NodeId {
        match self.nodes[id].kind {
            Kind::Alias(target) => target,
            _ => id,
        }
    }
}

/// What we know of a node that is complete.
#[derive(Clone, Copy)]
struct Shape {
    /// The number of nodes the node is made of, aliases expanded, plus the bytes of their text:
    /// copying a long string is not cheaper than copying as many nodes.
    size: usize,
    /// The nesting depth of the node, aliases expanded. Scalars have a height of 0.
    height: usize,
}

enum Content {
    Sequence(Vec<NodeId>),
    Mapping {
        entries: Vec<(NodeId, NodeId)>,
        key: Option<NodeId>,
    },
}

struct Frame<'de> {
    content: Content,
    anchor: usize,
    tag: Option<Cow<'de, Tag>>,
    marker: Marker,
    /// The accumulated shape of the children.
    size: usize,
    height: usize,
}

struct Builder<'a, 'de> {
    options: &'a Options,
    nodes: Vec<Node<'de>>,
    shapes: Vec<Shape>,
    /// Anchor IDs (as assigned by the parser) to the complete node they anchor.
    anchors: HashMap<usize, NodeId>,
    stack: Vec<Frame<'de>>,
    root: Option<NodeId>,
    /// The number of nodes added by aliases so far.
    alias_expansion: usize,
}

/// Read events from the parser until the end of the current document (`DocumentStart` has been
/// consumed already).
pub(crate) fn load_document<'de, T: Input>(
    parser: &mut Parser<'de, T>,
    options: &Options,
) -> Result<Document<'de>> {
    let mut builder = Builder {
        options,
        nodes: Vec::new(),
        shapes: Vec::new(),
        anchors: HashMap::new(),
        stack: Vec::new(),
        root: None,
        alias_expansion: 0,
    };
    loop {
        let (event, span) = match parser.next_event() {
            Some(event) => event?,
            None => {
                return Err(Error::new(
                    crate::ErrorKind::Syntax,
                    "unexpected end of stream",
                ));
            }
        };
        let marker = span.start;
        match event {
            Event::DocumentEnd => break,
            Event::Scalar(text, style, anchor, tag) => {
                let shape = Shape {
                    size: text.len().saturating_add(1),
                    height: 0,
                };
                let id = builder.add(Kind::Scalar(text, style), tag, marker, shape)?;
                builder.register_anchor(anchor, id);
                builder.attach(id)?;
            }
            Event::Alias(anchor) => {
                let Some(&target) = builder.anchors.get(&anchor) else {
                    return Err(Error::at(
                        crate::ErrorKind::Syntax,
                        "alias refers to an anchor that is not complete yet",
                        marker,
                    ));
                };
                let shape = builder.shapes[target];
                builder.alias_expansion = builder.alias_expansion.saturating_add(shape.size);
                if builder.alias_expansion > options.max_alias_expansion {
                    return Err(Error::limit("aliases expand to too much data", marker));
                }
                let id = builder.add(Kind::Alias(target), None, marker, shape)?;
                builder.attach(id)?;
            }
            Event::SequenceStart(anchor, tag) => {
                builder.start(Content::Sequence(Vec::new()), anchor, tag, marker)?;
            }
            Event::MappingStart(anchor, tag) => {
                builder.start(
                    Content::Mapping {
                        entries: Vec::new(),
                        key: None,
                    },
                    anchor,
                    tag,
                    marker,
                )?;
            }
            Event::SequenceEnd | Event::MappingEnd => builder.finish()?,
            // Not part of a document.
            Event::Nothing | Event::StreamStart | Event::StreamEnd | Event::DocumentStart(_) => {
                return Err(Error::at(
                    crate::ErrorKind::Syntax,
                    "unexpected event in document",
                    marker,
                ));
            }
        }
    }
    match (builder.root, builder.stack.is_empty()) {
        (Some(root), true) => Ok(Document {
            nodes: builder.nodes,
            root,
        }),
        _ => Err(Error::new(crate::ErrorKind::Syntax, "incomplete document")),
    }
}

impl<'de> Builder<'_, 'de> {
    fn add(
        &mut self,
        kind: Kind<'de>,
        tag: Option<Cow<'de, Tag>>,
        marker: Marker,
        shape: Shape,
    ) -> Result<NodeId> {
        if self.nodes.len() >= self.options.max_nodes {
            return Err(Error::limit("document has too many nodes", marker));
        }
        if shape.height > self.options.max_depth {
            return Err(Error::limit("document is nested too deeply", marker));
        }
        self.nodes.push(Node { kind, tag, marker });
        self.shapes.push(shape);
        Ok(self.nodes.len() - 1)
    }

    /// Anchor 0 means "no anchor".
    fn register_anchor(&mut self, anchor: usize, id: NodeId) {
        if anchor != 0 {
            self.anchors.insert(anchor, id);
        }
    }

    fn start(
        &mut self,
        content: Content,
        anchor: usize,
        tag: Option<Cow<'de, Tag>>,
        marker: Marker,
    ) -> Result<()> {
        // Bounds the memory the parser itself uses for nesting as well.
        if self.stack.len() >= self.options.max_depth {
            return Err(Error::limit("document is nested too deeply", marker));
        }
        self.stack.push(Frame {
            content,
            anchor,
            tag,
            marker,
            size: 1,
            height: 0,
        });
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        let frame = self
            .stack
            .pop()
            .ok_or_else(|| Error::new(crate::ErrorKind::Syntax, "unbalanced end of collection"))?;
        let shape = Shape {
            size: frame.size,
            height: frame.height + 1,
        };
        let kind = match frame.content {
            Content::Sequence(items) => Kind::Sequence(items),
            Content::Mapping { entries, key: None } => {
                if !self.options.allow_duplicate_keys {
                    self.check_duplicate_keys(&entries)?;
                }
                Kind::Mapping(entries)
            }
            Content::Mapping { key: Some(_), .. } => {
                return Err(Error::at(
                    crate::ErrorKind::Syntax,
                    "mapping key without a value",
                    frame.marker,
                ));
            }
        };
        let id = self.add(kind, frame.tag, frame.marker, shape)?;
        self.register_anchor(frame.anchor, id);
        self.attach(id)
    }

    /// Add a complete node to the collection being built, or make it the root.
    fn attach(&mut self, id: NodeId) -> Result<()> {
        let shape = self.shapes[id];
        let Some(parent) = self.stack.last_mut() else {
            if self.root.replace(id).is_some() {
                return Err(Error::new(
                    crate::ErrorKind::Syntax,
                    "document has more than one root node",
                ));
            }
            return Ok(());
        };
        parent.size = parent.size.saturating_add(shape.size);
        parent.height = parent.height.max(shape.height);
        match &mut parent.content {
            Content::Sequence(items) => items.push(id),
            Content::Mapping { entries, key } => match key.take() {
                None => *key = Some(id),
                Some(key) => entries.push((key, id)),
            },
        }
        Ok(())
    }

    fn check_duplicate_keys(&self, entries: &[(NodeId, NodeId)]) -> Result<()> {
        let mut seen = HashSet::with_capacity(entries.len());
        for &(key, _) in entries {
            let key = match self.nodes[key].kind {
                Kind::Alias(target) => target,
                _ => key,
            };
            if let Kind::Scalar(text, _) = &self.nodes[key].kind {
                if !seen.insert(text.as_ref()) {
                    return Err(Error::at(
                        crate::ErrorKind::Syntax,
                        format!("duplicate mapping key `{text}`"),
                        self.nodes[key].marker,
                    ));
                }
            }
        }
        Ok(())
    }
}
