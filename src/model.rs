//! Core data model: graph, nodes, edges.

use std::collections::HashMap;

/// Flow direction of the diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum Direction {
    /// Top-down. Alias: TB.
    #[default]
    TD,
    /// Left to right.
    LR,
    /// Right to left.
    RL,
    /// Bottom to top.
    BT,
}


/// Node shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `A[text]` — rectangle.
    Rect,
    /// `A(text)` — rounded corners.
    Rounded,
    /// `A([text])` — stadium / pill.
    Stadium,
    /// `A{text}` — diamond (decision).
    Diamond,
    /// `A((text))` — circle.
    Circle,
}

/// Edge line style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// `-->` regular arrow.
    Arrow,
    /// `---` plain line, no arrowhead.
    Open,
    /// `-.->` dotted line with arrowhead.
    Dotted,
    /// `-.-` dotted line, no arrowhead.
    DottedOpen,
    /// `==>` thick line with arrowhead.
    Thick,
    /// `===` thick line, no arrowhead.
    ThickOpen,
    /// `~~~` invisible link — participates in layout (ranking /
    /// ordering) but is never drawn.
    Invisible,
}

impl EdgeKind {
    /// Whether renderers should draw an arrowhead.
    pub fn has_arrow(self) -> bool {
        matches!(self, EdgeKind::Arrow | EdgeKind::Dotted | EdgeKind::Thick)
    }
}

/// Custom per-node styling from Mermaid `style` / `classDef`
/// lines. `None` fields fall back to the shape's theme color
/// (see [`crate::style::shape_style`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeStyle {
    /// `fill:#rrggbb`
    pub fill: Option<String>,
    /// `stroke:#rrggbb`
    pub stroke: Option<String>,
    /// `stroke-width:4px` (pixels)
    pub stroke_width: Option<f64>,
    /// `color:#rrggbb` — label text color.
    pub color: Option<String>,
}

impl NodeStyle {
    /// Overlay `over`'s set fields onto `self` (used to layer
    /// classDef under an explicit `style` line, which wins).
    pub fn apply_over(&mut self, over: &NodeStyle) {
        if let Some(v) = &over.fill {
            self.fill = Some(v.clone());
        }
        if let Some(v) = &over.stroke {
            self.stroke = Some(v.clone());
        }
        if let Some(v) = over.stroke_width {
            self.stroke_width = Some(v);
        }
        if let Some(v) = &over.color {
            self.color = Some(v.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: Shape,
    /// Custom colors; empty = follow the shape theme.
    pub style: NodeStyle,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
    pub kind: EdgeKind,
}

/// A `subgraph ... end` block: a titled cluster of nodes, possibly
/// nested. Membership is by node index; nested members belong to
/// the child, not the parent (walk `parent` links for the chain).
#[derive(Debug, Clone)]
pub struct Subgraph {
    pub id: String,
    pub title: String,
    /// Direct member node indices.
    pub nodes: Vec<usize>,
    /// Enclosing subgraph, when nested.
    pub parent: Option<usize>,
    /// Per-subgraph flow direction (`direction LR` inside the block).
    pub direction: Option<Direction>,
}

/// One endpoint of an edge that touches a subgraph — either a
/// regular node or a whole subgraph (its cluster box).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Node(usize),
    Sub(usize),
}

/// An edge where at least one endpoint is a subgraph
/// (`CF --> VPC`). Kept apart from [`Edge`] so the flat node→node
/// path stays untouched; consumed only by the clustered scene.
#[derive(Debug, Clone)]
pub struct SubEdge {
    pub from: End,
    pub to: End,
    pub label: Option<String>,
    pub kind: EdgeKind,
}

/// Parsed graph, ready for layout.
#[derive(Debug, Default)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
    /// Edges with a subgraph as an endpoint (see [`SubEdge`]).
    pub sub_edges: Vec<SubEdge>,
    index: HashMap<String, usize>,
}

impl Graph {
    /// Look up a node by id; create it if missing.
    /// The latest label/shape overrides earlier ones (Mermaid behaviour).
    pub fn ensure_node(&mut self, id: &str, label: Option<String>, shape: Option<Shape>) -> usize {
        if let Some(&i) = self.index.get(id) {
            if let Some(l) = label {
                self.nodes[i].label = l;
            }
            if let Some(s) = shape {
                self.nodes[i].shape = s;
            }
            i
        } else {
            let i = self.nodes.len();
            self.nodes.push(Node {
                id: id.to_string(),
                label: label.unwrap_or_else(|| id.to_string()),
                shape: shape.unwrap_or(Shape::Rect),
                style: NodeStyle::default(),
            });
            self.index.insert(id.to_string(), i);
            i
        }
    }

    pub fn add_edge(&mut self, from: usize, to: usize, label: Option<String>, kind: EdgeKind) {
        self.edges.push(Edge {
            from,
            to,
            label,
            kind,
        });
    }

    /// Index of an existing node by id, without creating it.
    pub fn node_index(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }
}

/// One parsed Mermaid document of any supported diagram type.
/// Produced by [`crate::parser::parse_document`].
#[derive(Debug)]
pub enum Document {
    Flowchart(Graph),
    Er(ErDiagram),
}

/// Entity-Relationship diagram (`erDiagram` header).
#[derive(Debug, Default)]
pub struct ErDiagram {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
    index: HashMap<String, usize>,
}

impl ErDiagram {
    /// Look up an entity by name; create it (attribute-less) if missing.
    /// Entities may be introduced by a relationship line alone.
    pub fn ensure_entity(&mut self, name: &str) -> usize {
        if let Some(&i) = self.index.get(name) {
            i
        } else {
            let i = self.entities.len();
            self.entities.push(Entity {
                name: name.to_string(),
                attrs: Vec::new(),
            });
            self.index.insert(name.to_string(), i);
            i
        }
    }
}

/// A database entity, rendered as a table.
#[derive(Debug)]
pub struct Entity {
    pub name: String,
    pub attrs: Vec<Attr>,
}

/// One attribute row inside an entity block:
/// `type name [PK|FK|UK] ["comment"]`.
#[derive(Debug)]
pub struct Attr {
    pub ty: String,
    pub name: String,
    pub keys: Vec<Key>,
    pub comment: Option<String>,
}

/// Attribute key marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Pk,
    Fk,
    Uk,
}

impl Key {
    /// Display tag as written in Mermaid.
    pub fn tag(self) -> &'static str {
        match self {
            Key::Pk => "PK",
            Key::Fk => "FK",
            Key::Uk => "UK",
        }
    }
}

/// Relationship between two entities. `card_from` describes the
/// `from` side, `card_to` the `to` side (crow's foot notation).
#[derive(Debug)]
pub struct Relation {
    pub from: usize,
    pub to: usize,
    pub card_from: Card,
    pub card_to: Card,
    /// `--` = identifying (solid line), `..` = non-identifying (dashed).
    pub identifying: bool,
    pub label: Option<String>,
}

/// Relationship cardinality on one side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    /// `||` exactly one.
    One,
    /// `|o` / `o|` zero or one.
    ZeroOne,
    /// `}o` / `o{` zero or many.
    ZeroMany,
    /// `}|` / `|{` one or many.
    OneMany,
}
