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
    /// `A(((text)))` — double circle (terminal).
    DoubleCircle,
    /// `A[(text)]` — cylinder (database).
    Cylinder,
    /// `A[[text]]` — subroutine.
    Subroutine,
    /// `A{{text}}` — hexagon.
    Hexagon,
    /// `A[/text/]` — parallelogram.
    Parallelogram,
    /// `A[\text\]` — parallelogram, slanted the other way.
    ParallelogramAlt,
    /// stateDiagram `[*]` as a transition source — the initial
    /// pseudostate (small filled dot).
    StateStart,
    /// stateDiagram `[*]` as a transition target — the final
    /// pseudostate (ring with a filled core).
    StateEnd,
    /// stateDiagram `<<fork>>` / `<<join>>` — a thin filled bar.
    ForkBar,
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
    Class(ClassDiagram),
    Sequence(SequenceDiagram),
    Pie(PieChart),
    /// stateDiagram-v2 — mapped straight onto a [`Graph`] (state =
    /// rounded node, transition = edge, composite = subgraph), so the
    /// whole flowchart layout/SVG/drag pipeline is reused as-is.
    State(Graph),
    Mindmap(Mindmap),
}

/// UML class diagram (`classDiagram` header).
#[derive(Debug, Default)]
pub struct ClassDiagram {
    pub classes: Vec<Class>,
    pub relations: Vec<ClassRel>,
    index: HashMap<String, usize>,
}

impl ClassDiagram {
    /// Look up a class by name; create it (empty) if missing.
    pub fn ensure_class(&mut self, name: &str) -> usize {
        if let Some(&i) = self.index.get(name) {
            i
        } else {
            let i = self.classes.len();
            self.classes.push(Class {
                name: name.to_string(),
                fields: Vec::new(),
                methods: Vec::new(),
            });
            self.index.insert(name.to_string(), i);
            i
        }
    }

    pub fn class_index(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }
}

/// A class box: name header + fields compartment + methods compartment.
#[derive(Debug)]
pub struct Class {
    pub name: String,
    pub fields: Vec<Member>,
    pub methods: Vec<Member>,
}

/// One field or method row.
#[derive(Debug)]
pub struct Member {
    pub visibility: Visibility,
    /// Display text after the visibility marker
    /// (`name: Type` / `name(args) Ret`).
    pub text: String,
}

/// UML member visibility, shown as a leading glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,    // +
    Private,   // -
    Protected, // #
    Package,   // ~
    None,
}

impl Visibility {
    pub fn glyph(self) -> &'static str {
        match self {
            Visibility::Public => "+",
            Visibility::Private => "-",
            Visibility::Protected => "#",
            Visibility::Package => "~",
            Visibility::None => "",
        }
    }
}

/// A relationship between two classes. Normalised so the end glyph
/// (triangle / diamond / arrow) always sits at the `to` end.
#[derive(Debug)]
pub struct ClassRel {
    pub from: usize,
    pub to: usize,
    pub kind: RelKind,
    /// Dashed line (realization, dependency, `..` link).
    pub dashed: bool,
    pub from_card: Option<String>,
    pub to_card: Option<String>,
    pub label: Option<String>,
}

/// UML relationship type (determines line style + end glyph).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelKind {
    /// `<|--` — hollow triangle at the parent.
    Inheritance,
    /// `..|>` — hollow triangle, dashed line.
    Realization,
    /// `*--` — filled diamond.
    Composition,
    /// `o--` — hollow diamond.
    Aggregation,
    /// `-->` — open arrow.
    Association,
    /// `..>` — open arrow, dashed line.
    Dependency,
    /// `--` — plain line.
    Link,
}

/// Sequence diagram (`sequenceDiagram` header). Items keep source
/// order — the layout is one row per item, top-down.
#[derive(Debug, Default)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub items: Vec<SeqItem>,
    /// `autonumber` — messages get a bold `1.` … `n.` prefix.
    pub autonumber: bool,
    index: HashMap<String, usize>,
}

impl SequenceDiagram {
    /// Look up a participant by id; create it (label = id) if
    /// missing — the first mention in a message declares one, in
    /// order of appearance.
    pub fn ensure_participant(&mut self, id: &str) -> usize {
        if let Some(&i) = self.index.get(id) {
            i
        } else {
            let i = self.participants.len();
            self.participants.push(Participant {
                id: id.to_string(),
                label: id.to_string(),
                actor: false,
            });
            self.index.insert(id.to_string(), i);
            i
        }
    }

    pub fn participant_index(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }
}

/// One lifeline column: `participant A [as Label]` / `actor A`.
#[derive(Debug)]
pub struct Participant {
    pub id: String,
    pub label: String,
    /// Declared with `actor` — drawn as an outlined box instead of
    /// the filled participant box.
    pub actor: bool,
}

/// One statement of a sequence diagram, in source order.
#[derive(Debug)]
pub enum SeqItem {
    /// `A->>B: text` (any of the eight arrow operators, with the
    /// optional `+`/`-` activation shorthand before the target).
    Message {
        from: usize,
        to: usize,
        text: String,
        /// `--` operator variants — dashed line.
        dashed: bool,
        head: SeqHead,
        /// `+` before the target: activate `to` at this message.
        activate: bool,
        /// `-` before the target: deactivate `from` at this message.
        deactivate: bool,
    },
    /// `Note over A,B: text` / `Note left of A:` / `Note right of A:`.
    Note { side: NoteSide, text: String },
    /// `activate A`.
    Activate(usize),
    /// `deactivate A`.
    Deactivate(usize),
    /// `loop|opt|alt|par <label>` — opens a labeled frame.
    FrameStart { kind: FrameKind, label: String },
    /// `else <label>` (in `alt`) / `and <label>` (in `par`) —
    /// a dashed divider inside the innermost frame.
    FrameElse { label: String },
    /// `end` — closes the innermost frame.
    FrameEnd,
}

/// Arrowhead at the target end of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqHead {
    /// `->>` / `-->>` — filled triangle.
    Filled,
    /// Thin open V. No v1 operator maps to it (Mermaid's `->` is
    /// headless) — available to hosts building scenes by hand.
    Open,
    /// `-x` / `--x` — cross just before the lifeline.
    Cross,
    /// `-)` / `--)` — async open arrow.
    Async,
    /// `->` / `-->` — plain line, no head (Mermaid semantics).
    None,
}

/// Where a note attaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteSide {
    /// `Note over A[,B]` — spans one or two lifelines.
    Over(usize, Option<usize>),
    /// `Note left of A`.
    LeftOf(usize),
    /// `Note right of A`.
    RightOf(usize),
}

/// Kind of a `loop` / `opt` / `alt` / `par` frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Loop,
    Opt,
    Alt,
    Par,
}

impl FrameKind {
    /// Chip keyword as written in Mermaid.
    pub fn keyword(self) -> &'static str {
        match self {
            FrameKind::Loop => "loop",
            FrameKind::Opt => "opt",
            FrameKind::Alt => "alt",
            FrameKind::Par => "par",
        }
    }
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

/// Pie chart (`pie` header). Slices keep source order; a duplicate
/// label keeps one slice whose value is the LAST one written.
#[derive(Debug, Default)]
pub struct PieChart {
    /// `pie title …` or a standalone `title …` line.
    pub title: Option<String>,
    /// `pie showData` — legend entries carry the raw value.
    pub show_data: bool,
    pub slices: Vec<PieSlice>,
}

/// One pie data row: `"Quoted Label" : value` (non-negative).
#[derive(Debug)]
pub struct PieSlice {
    pub label: String,
    pub value: f64,
}

/// A mindmap (`mindmap` header): a single-rooted tree of text nodes
/// whose hierarchy comes from source indentation. `nodes[0]` is always
/// the root; every other node has a `parent`. Parse order is preserved,
/// so sibling order (and thus layout order) is stable across renders.
#[derive(Debug, Default)]
pub struct Mindmap {
    pub nodes: Vec<MindNode>,
}

/// One mindmap node. `text` may contain `\n` (from `<br/>`) for a
/// multi-line label. `branch` is the index of the depth-1 ancestor
/// this node descends from (its colored branch); `None` for the root.
#[derive(Debug, Clone)]
pub struct MindNode {
    pub text: String,
    pub shape: MindShape,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub depth: usize,
    pub branch: Option<usize>,
}

/// Node outline in a mindmap, from the wrapper around its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MindShape {
    /// `id(text)` or no wrapper — the default.
    #[default]
    Rounded,
    /// `id[text]`.
    Square,
    /// `id((text))`.
    Circle,
    /// `id{{text}}`.
    Hexagon,
    /// `id))text((` — a "bang"/explosion; approximated as a bold pill.
    Bang,
    /// `id)text(` — a cloud; approximated as an ellipse.
    Cloud,
}
