//! Core data model: graph, nodes, edges.

use std::collections::{BTreeMap, HashMap};

/// Flow direction of the diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    /// stateDiagram `<<fork>>` / `<<join>>` — a thin filled bar, laid
    /// across the flow and stretched to span its branches by the scene
    /// builders.
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

/// Glyph drawn at one end of an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Marker {
    /// Filled arrowhead (`-->`, and the `<` of `<-->`).
    Arrow,
    /// Small X (`--x`, `x--x`).
    Cross,
    /// Small filled circle (`--o`, `o--o`).
    Circle,
}

/// End glyphs layered over an edge's [`EdgeKind`]. `EdgeKind` stays
/// the line style plus the classic target arrowhead, so code that only
/// knows `EdgeKind` keeps drawing every edge sensibly (an `--x` edge
/// is `EdgeKind::Open` and degrades to a plain line); renderers that
/// know markers resolve each end with [`EdgeEnds::end_marker`] /
/// [`EdgeEnds::start_marker`].
///
/// The default (both `None`) is the pre-marker behaviour: no start
/// glyph, and an arrowhead at the target exactly when
/// [`EdgeKind::has_arrow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EdgeEnds {
    /// Glyph at the source end (`<-->`, `x--x`, `o--o`); `None` = none.
    pub start: Option<Marker>,
    /// Glyph at the target end; `None` = whatever the edge's kind
    /// implies (arrowhead for `Arrow`/`Dotted`/`Thick`, else nothing).
    pub end: Option<Marker>,
}

impl EdgeEnds {
    /// The glyph to draw at the target end of an edge of `kind`.
    /// Invisible links never draw one.
    pub fn end_marker(self, kind: EdgeKind) -> Option<Marker> {
        if kind == EdgeKind::Invisible {
            return None;
        }
        self.end
            .or_else(|| kind.has_arrow().then_some(Marker::Arrow))
    }

    /// The glyph to draw at the source end of an edge of `kind`.
    pub fn start_marker(self, kind: EdgeKind) -> Option<Marker> {
        if kind == EdgeKind::Invisible {
            return None;
        }
        self.start
    }
}

/// Edge animation speed from mermaid edge metadata
/// (`e1@{ animate: true }` = fast, `e1@{ animation: slow }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EdgeAnimation {
    /// Mermaid's `edge-animation-fast` (20 s dash cycle).
    Fast,
    /// Mermaid's `edge-animation-slow` (50 s dash cycle).
    Slow,
}

/// Per-edge curve from mermaid edge metadata (`e1@{ curve: linear }`),
/// named after the d3 curves mermaid maps them to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EdgeCurve {
    Basis,
    Linear,
    Step,
    StepBefore,
    StepAfter,
    /// Straight segments with radius-5 rounded corners.
    Rounded,
    Cardinal,
    CatmullRom,
    Natural,
    MonotoneX,
    MonotoneY,
    BumpX,
    BumpY,
}

/// Mermaid's curve names (case-sensitive) and the curve each selects.
const EDGE_CURVES: &[(&str, EdgeCurve)] = &[
    ("basis", EdgeCurve::Basis),
    ("linear", EdgeCurve::Linear),
    ("step", EdgeCurve::Step),
    ("stepBefore", EdgeCurve::StepBefore),
    ("stepAfter", EdgeCurve::StepAfter),
    ("rounded", EdgeCurve::Rounded),
    ("cardinal", EdgeCurve::Cardinal),
    ("catmullRom", EdgeCurve::CatmullRom),
    ("natural", EdgeCurve::Natural),
    ("monotoneX", EdgeCurve::MonotoneX),
    ("monotoneY", EdgeCurve::MonotoneY),
    ("bumpX", EdgeCurve::BumpX),
    ("bumpY", EdgeCurve::BumpY),
];

impl EdgeCurve {
    /// The curve a mermaid `curve:` value selects. Names are
    /// case-sensitive and anything unknown is `Basis` — mermaid's
    /// default branch.
    pub fn from_mermaid(name: &str) -> EdgeCurve {
        EDGE_CURVES
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(EdgeCurve::Basis, |&(_, c)| c)
    }

    /// The mermaid name of this curve.
    pub fn mermaid_name(self) -> &'static str {
        EDGE_CURVES
            .iter()
            .find(|(_, c)| *c == self)
            .map_or("basis", |&(n, _)| n)
    }
}

/// Mermaid's per-edge `@` syntax: a user id (`A e1@--> B`) and the
/// metadata a later `e1@{ … }` statement attaches to it. The default
/// (all `None`) is a plain edge and renders exactly as before.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct EdgeMeta {
    /// User-defined edge id. Ids are unique per graph: a repeated id
    /// leaves the later edge without one, as in mermaid.
    pub id: Option<String>,
    /// Resolved animation (mermaid's `animate` / `animation` keys).
    pub animation: Option<EdgeAnimation>,
    /// Per-edge curve (mermaid's `curve` key, or a `linkStyle …
    /// interpolate <curve>` statement); `None` = the default.
    pub curve: Option<EdgeCurve>,
    /// Paint from mermaid's `linkStyle` statements, already resolved
    /// (the `default` style overlaid with this edge's own). The default
    /// (all `None`) draws the edge exactly as before.
    pub style: LinkStyle,
}

/// Edge paint from mermaid's `linkStyle <n> <styles>` /
/// `linkStyle default <styles>`. `None` fields keep the edge kind's
/// built-in look (theme colour, 1.7px or 3.4px thick, dotted dash).
///
/// `Eq` holds because the parser never stores a non-finite
/// `stroke_width`; a hand-built NaN width compares unequal to itself.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct LinkStyle {
    /// `stroke:<colour>`: line colour; arrow, cross and circle
    /// markers follow it.
    pub stroke: Option<String>,
    /// `stroke-width:<n>[px]`, in pixels. A styled width keeps the
    /// arrowhead at its normal size (mermaid's markers don't scale).
    pub stroke_width: Option<f64>,
    /// `stroke-dasharray:<v>` (`5 5`; `0` makes a dotted edge solid).
    pub stroke_dasharray: Option<String>,
    /// `fill:<paint>`: fills the region the curve encloses; `None`
    /// means `none`.
    pub fill: Option<String>,
    /// `color:<colour>`: edge label text colour.
    pub color: Option<String>,
}

impl Eq for LinkStyle {}

impl LinkStyle {
    /// Whether no property is set (the edge draws unstyled).
    pub fn is_empty(&self) -> bool {
        *self == LinkStyle::default()
    }

    /// Overlay `over`'s set fields onto `self` (`over` wins).
    pub fn apply_over(&mut self, over: &LinkStyle) {
        let pick = |a: &mut Option<String>, b: &Option<String>| {
            if b.is_some() {
                a.clone_from(b);
            }
        };
        pick(&mut self.stroke, &over.stroke);
        pick(&mut self.stroke_dasharray, &over.stroke_dasharray);
        pick(&mut self.fill, &over.fill);
        pick(&mut self.color, &over.color);
        if over.stroke_width.is_some() {
            self.stroke_width = over.stroke_width;
        }
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

/// A node-to-node edge. `#[non_exhaustive]`: build one with
/// [`Graph::add_edge`] / [`Graph::add_edge_with_ends`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
    pub kind: EdgeKind,
    /// Cross / circle / two-way end glyphs (default: none beyond
    /// what `kind` implies).
    pub ends: EdgeEnds,
    /// Mermaid edge id + metadata (`A e1@--> B`, `e1@{ curve: … }`).
    pub meta: EdgeMeta,
    /// Length (px, along the edge) that end glyphs drawn by a renderer
    /// of its own occupy at the `(from, to)` ends — ER crow's feet. The
    /// layout spaces two adjacent layers so the glyphs, and the edge's
    /// label between them, all fit. `(0.0, 0.0)` (the default) = none.
    pub end_room: (f64, f64),
}

/// A `subgraph ... end` block: a titled cluster of nodes, possibly
/// nested. Membership is by node index; nested members belong to
/// the child, not the parent (walk `parent` links for the chain).
#[derive(Debug, Clone)]
pub struct Subgraph {
    pub id: String,
    /// Display title; `<br/>` in the source becomes `\n` (drawn as
    /// separate lines).
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
/// `#[non_exhaustive]`: the parser builds these; read their fields.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SubEdge {
    pub from: End,
    pub to: End,
    pub label: Option<String>,
    pub kind: EdgeKind,
    /// End glyphs, as on [`Edge::ends`].
    pub ends: EdgeEnds,
    /// Edge id + metadata, as on [`Edge::meta`].
    pub meta: EdgeMeta,
}

/// Parsed graph, ready for layout.
#[derive(Debug, Clone, Default)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
    /// Edges with a subgraph as an endpoint (see [`SubEdge`]).
    pub sub_edges: Vec<SubEdge>,
    /// Accessible title from mermaid's `accTitle:` statement — replaces
    /// the default `<title>` / `aria-label` of the SVG. `None` = absent
    /// or empty.
    pub acc_title: Option<String>,
    /// Accessible description from `accDescr:` or the multi-line
    /// `accDescr { … }` block — written as the SVG's `<desc>`.
    pub acc_descr: Option<String>,
    /// Mermaid `click` links and tooltips, keyed by node index.
    pub links: BTreeMap<usize, NodeLink>,
    /// Gap between layers for a synthetic graph that needs more than
    /// the flowchart's (class diagrams: room for a label, two
    /// cardinalities and an end glyph on one short edge); `None` = the
    /// default.
    pub(crate) rank_gap: Option<f64>,
    /// Per edge (index-parallel with `edges`; missing = 0): room a
    /// short edge's label keeps from each of its ends on top of the
    /// usual clearance — class diagrams put an end glyph and a
    /// cardinality there, which in LR / RL lie along the same axis as
    /// the label.
    pub(crate) label_end_room: Vec<f64>,
    /// Keep every layer's leaves on their layer: no leaf staggering
    /// when the drawing is a ribbon. Class diagrams set it — a
    /// hierarchy's children read as one row under their parent (as
    /// mermaid draws them), and a staggered second row interleaves its
    /// legs with the first row's on the parent's face, crossing them.
    pub(crate) flat_leaves: bool,
    /// Rank every source one layer above its nearest target, as dagre's
    /// network simplex does, instead of on the first layer (longest
    /// path's choice). Class diagrams set it: a note — a source linked
    /// to its class — then sits just above its class, and a class
    /// pointing at one deep in the hierarchy sits next to it.
    pub(crate) tight_sources: bool,
    /// Sources ranked one layer before their nearest target instead of
    /// on the first layer: a state `note left of X` card (mermaid's
    /// dagre ranks it just ahead of X; longest-path layering would park
    /// it at the top however deep X sits).
    pub(crate) lead_nodes: Vec<usize>,
    index: HashMap<String, usize>,
}

/// A link target from a mermaid `click` statement — the `target`
/// attribute of the node's `<a>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinkTarget {
    /// `_self`
    SelfFrame,
    /// `_blank`
    Blank,
    /// `_parent`
    Parent,
    /// `_top`
    Top,
}

/// Mermaid's `LINK_TARGET` tokens (case-sensitive) and their targets.
const LINK_TARGETS: &[(&str, LinkTarget)] = &[
    ("_self", LinkTarget::SelfFrame),
    ("_blank", LinkTarget::Blank),
    ("_parent", LinkTarget::Parent),
    ("_top", LinkTarget::Top),
];

impl LinkTarget {
    /// The target a mermaid token names (`_blank`, …); `None` for
    /// anything else.
    pub fn from_mermaid(s: &str) -> Option<LinkTarget> {
        LINK_TARGETS.iter().find(|(n, _)| *n == s).map(|&(_, t)| t)
    }

    /// The token as written in mermaid and in the SVG attribute.
    pub fn as_str(self) -> &'static str {
        LINK_TARGETS
            .iter()
            .find(|(_, t)| *t == self)
            .map_or("_self", |&(n, _)| n)
    }
}

/// What a mermaid `click` statement attached to a node. flowmaid runs
/// no script, so a callback is kept by name only (for `to_mermaid`)
/// and never drawn; the SVG wraps a linked node in `<a>` and shows the
/// tooltip as the node's `<title>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct NodeLink {
    /// The url, as written (entity codes decoded, trimmed). Only a url
    /// that passes [`sanitize_href`] is stored by the parser, and the
    /// SVG writer checks again.
    pub href: Option<String>,
    /// `_self` / `_blank` / `_parent` / `_top`; `None` = no `target`
    /// attribute (the same tab).
    pub target: Option<LinkTarget>,
    /// Hover text.
    pub tooltip: Option<String>,
    /// The JavaScript callback's name (`click A cb`, `click A call
    /// cb()`), arguments dropped. Never run.
    pub callback: Option<String>,
}

impl NodeLink {
    /// Whether nothing is set.
    pub fn is_empty(&self) -> bool {
        *self == NodeLink::default()
    }

    /// This link with an unsafe `href` dropped (see [`sanitize_href`]),
    /// or `None` when nothing is left. Scene builders hand links to
    /// host apps through this, so a [`Graph`] filled in by hand (not
    /// by the parser) never passes a `javascript:` url on.
    pub(crate) fn sanitized(&self) -> Option<NodeLink> {
        let link = NodeLink {
            href: self.href.as_deref().and_then(sanitize_href),
            ..self.clone()
        };
        (!link.is_empty()).then_some(link)
    }
}

/// Most times [`sanitize_href`] re-decodes a url (percent and entity
/// layers) while it still changes, so `%256A` cannot hide a `j`.
const SANITIZE_MAX_PASSES: usize = 8;
/// Url schemes that run or embed content and are never linked
/// (`@braintree/sanitize-url`, which mermaid uses).
const BLOCKED_SCHEMES: &[&str] = &["javascript", "data", "vbscript"];

/// A link url that is safe to put in an SVG `href`, or `None`.
///
/// Mermaid's default (`strict`) security level passes urls through
/// `@braintree/sanitize-url`; this ports its checks: after
/// repeatedly percent- and entity-decoding the url (HTML `&#NN;`,
/// `&#xNN;`, `&colon;`, `&tab;`, `&newline;` and mermaid's `#NN;`) and
/// dropping control, invisible and whitespace characters, a url whose
/// scheme starts with `javascript`, `data` or `vbscript` is refused,
/// as is one that is empty. Relative urls (`/x`, `./x`, `#x`, `?q`),
/// scheme-less ones and every other scheme pass. Unlike mermaid, a
/// passing url is returned as written (trimmed), not re-serialised,
/// and a refused one yields `None` instead of `about:blank`.
pub fn sanitize_href(url: &str) -> Option<String> {
    let url = url.trim();
    let mut s: String = url.to_string();
    for _ in 0..SANITIZE_MAX_PASSES {
        let next = strip_invisible(&decode_href_entities(&percent_decode(&s)));
        if next == s {
            break;
        }
        s = next;
    }
    let s = strip_invisible(&s);
    if s.is_empty() {
        return None;
    }
    if s.starts_with(['.', '/']) {
        return Some(url.to_string());
    }
    // Braintree reads the scheme as everything up to the LAST colon.
    if let Some(colon) = s.rfind(':') {
        let scheme = s[..colon]
            .trim_start_matches(|c: char| !(c.is_alphanumeric() || c == '_'))
            .to_ascii_lowercase();
        if BLOCKED_SCHEMES.iter().any(|b| scheme.starts_with(b)) {
            return None;
        }
    }
    Some(url.to_string())
}

/// Drop control characters, Unicode spaces and zero-width marks, and
/// all whitespace (braintree's `ctrlCharactersRegex` plus trim).
fn strip_invisible(s: &str) -> String {
    s.chars()
        .filter(|&c| {
            !(c.is_control()
                || c.is_whitespace()
                || ('\u{2000}'..='\u{200D}').contains(&c)
                || c == '\u{FEFF}')
        })
        .collect()
}

/// One pass of `%XX` decoding (invalid escapes stay literal; bytes
/// that don't form UTF-8 become U+FFFD).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && b[i + 1].is_ascii_hexdigit()
            && b[i + 2].is_ascii_hexdigit()
        {
            let hex = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
            out.push(hex(b[i + 1]) * 16 + hex(b[i + 2]));
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// One pass of the entity forms that can hide a scheme: HTML
/// `&#NN;` / `&#xNN;` (semicolon optional, as browsers allow),
/// `&colon;`, `&tab;`, `&newline;`, and mermaid's `#NN;`.
fn decode_href_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        if c == '&' || c == '#' {
            let named = [("&colon;", ':'), ("&tab;", '\t'), ("&newline;", '\n')];
            let hit = named.iter().find(|(n, _)| {
                rest.get(..n.len())
                    .is_some_and(|h| h.eq_ignore_ascii_case(n))
            });
            if let Some(&(n, ch)) = hit {
                out.push(ch);
                rest = &rest[n.len()..];
                continue;
            }
            // `&#…` (HTML) or `#…;` (mermaid, semicolon required).
            let (body, html) = if let Some(b) = rest.strip_prefix("&#") {
                (b, true)
            } else if let Some(b) = rest.strip_prefix('#') {
                (b, false)
            } else {
                out.push(c);
                rest = &rest[1..];
                continue;
            };
            let (radix, digits_at) = match body.chars().next() {
                Some('x' | 'X') if html => (16, 1),
                _ => (10, 0),
            };
            let digits: String = body[digits_at..]
                .chars()
                .take_while(|c| c.is_digit(radix))
                .collect();
            let after = &body[digits_at + digits.len()..];
            let semi = after.starts_with(';');
            if !digits.is_empty() && (html || semi) {
                if let Some(ch) = u32::from_str_radix(&digits, radix)
                    .ok()
                    .and_then(char::from_u32)
                {
                    out.push(ch);
                    rest = if semi { &after[1..] } else { after };
                    continue;
                }
            }
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
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
        self.add_edge_with_ends(from, to, label, kind, EdgeEnds::default());
    }

    /// [`Graph::add_edge`] with explicit end glyphs (`--x`, `<-->`, …).
    pub fn add_edge_with_ends(
        &mut self,
        from: usize,
        to: usize,
        label: Option<String>,
        kind: EdgeKind,
        ends: EdgeEnds,
    ) {
        self.edges.push(Edge {
            from,
            to,
            label,
            kind,
            ends,
            meta: EdgeMeta::default(),
            end_room: (0.0, 0.0),
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
    Journey(Journey),
    GitGraph(GitGraph),
    Architecture(Architecture),
    Timeline(Timeline),
    Sankey(SankeyDiagram),
}

/// UML class diagram (`classDiagram` header).
#[derive(Debug, Default)]
pub struct ClassDiagram {
    pub classes: Vec<Class>,
    pub relations: Vec<ClassRel>,
    /// Layout direction from mermaid's `direction TB|BT|LR|RL` line
    /// (default top-down; an unknown one is ignored, as in mermaid).
    /// Relations rank in SOURCE order along it:
    /// the left operand of `A <|-- B` sits before (above, in TB) the
    /// right one, so parents and wholes lead, as in UML and mermaid.
    pub direction: Direction,
    /// `note "text"` / `note for Class "text"` lines, in source order.
    pub notes: Vec<ClassNote>,
    /// Accessible title from mermaid's `accTitle:` statement — replaces
    /// the default `<title>` / `aria-label` of the SVG. `None` = absent
    /// or empty.
    pub acc_title: Option<String>,
    /// Accessible description from `accDescr:` or the multi-line
    /// `accDescr { … }` block — written as the SVG's `<desc>`.
    pub acc_descr: Option<String>,
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
                annotations: Vec::new(),
                generic: None,
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
/// Create one with [`ClassDiagram::ensure_class`] (the struct is
/// `#[non_exhaustive]`, so later versions may add fields).
#[derive(Debug)]
#[non_exhaustive]
pub struct Class {
    /// The class id: what relations, notes and members refer to
    /// (`Box` for `class Box~T~`).
    pub name: String,
    pub fields: Vec<Member>,
    pub methods: Vec<Member>,
    /// Stereotypes without the guillemets, in source order: `interface`
    /// from `<<interface>>` inside the class block or a standalone
    /// `<<interface>> Name` line. Drawn as `«interface»` above the name.
    pub annotations: Vec<String>,
    /// Generic parameter from `class Box~T~` (`T`), shown as `Box<T>`;
    /// `None` = not generic.
    pub generic: Option<String>,
}

impl Class {
    /// The header text: the name, plus `<T>` for a generic class.
    pub fn display_name(&self) -> String {
        match &self.generic {
            Some(t) => format!("{}<{}>", self.name, generics_to_angles(t)),
            None => self.name.clone(),
        }
    }
}

/// Mermaid's generic notation with angle brackets, by mermaid's own
/// rule (`parseGenericTypes`): within each comma-separated piece the
/// first and last `~` pair up, repeatedly, so `List~int~` reads
/// `List<int>` and `List~List~int~~` reads `List<List<int>>`; two
/// pieces holding one `~` each rejoin across their comma, so
/// `Map~K, V~` reads `Map<K, V>`. A lone `~` stays as written.
pub(crate) fn generics_to_angles(s: &str) -> String {
    if s.matches('~').count() < 2 {
        return s.to_string();
    }
    let pair = |piece: &str| -> String {
        let mut chars: Vec<char> = piece.chars().collect();
        // An odd count that starts with `~` keeps that leading one.
        let skip = usize::from(
            chars.iter().filter(|&&c| c == '~').count() % 2 == 1 && chars.first() == Some(&'~'),
        );
        loop {
            let first = chars
                .iter()
                .skip(skip)
                .position(|&c| c == '~')
                .map(|i| i + skip);
            let last = chars.iter().rposition(|&c| c == '~');
            match (first, last) {
                (Some(a), Some(b)) if a < b => {
                    chars[a] = '<';
                    chars[b] = '>';
                }
                _ => break,
            }
        }
        chars.into_iter().collect()
    };
    let pieces: Vec<&str> = s.split(',').collect();
    let mut out: Vec<String> = Vec::with_capacity(pieces.len());
    let mut i = 0;
    while i < pieces.len() {
        let one = |p: &str| p.matches('~').count() == 1;
        if one(pieces[i]) && i + 1 < pieces.len() && one(pieces[i + 1]) {
            out.push(pair(&format!("{},{}", pieces[i], pieces[i + 1])));
            i += 2;
        } else {
            out.push(pair(pieces[i]));
            i += 1;
        }
    }
    out.join(",")
}

/// A `note` in a class diagram: a yellow card, linked by a dotted line
/// to its class when written `note for Class "text"`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ClassNote {
    /// Note text; `\n` and `<br/>` in the source are line breaks.
    pub text: String,
    /// Index into [`ClassDiagram::classes`] for `note for Class`;
    /// `None` = a free-standing note (also `note for` a class the
    /// diagram never declares: the note does not create it).
    pub class: Option<usize>,
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
/// (triangle / diamond / arrow) always sits at the `to` end;
/// [`ClassRel::head_left`] remembers which operand the source wrote
/// first, which the layout ranks by.
#[derive(Debug)]
#[non_exhaustive]
pub struct ClassRel {
    pub from: usize,
    pub to: usize,
    pub kind: RelKind,
    /// Dashed line (realization, dependency, `..` link).
    pub dashed: bool,
    pub from_card: Option<String>,
    pub to_card: Option<String>,
    pub label: Option<String>,
    /// The source wrote the decorated end on the LEFT (`Animal <|--
    /// Dog`, `Order *-- Line`, `A <-- B`), so the source order is
    /// `to` → `from`. Mermaid ranks by source order (left operand
    /// first), which puts parents, interfaces and wholes above their
    /// children in a top-down diagram; see [`ClassRel::source_order`].
    pub head_left: bool,
}

impl ClassRel {
    /// A relation `from` → `to` of `kind` with its glyph at `to`, as
    /// `B --|> A` would parse: solid unless `kind` is realization or
    /// dependency, no cardinalities, no label, source order `from`
    /// first. The fields are public; adjust them after.
    pub fn new(from: usize, to: usize, kind: RelKind) -> ClassRel {
        ClassRel {
            from,
            to,
            kind,
            dashed: matches!(kind, RelKind::Realization | RelKind::Dependency),
            from_card: None,
            to_card: None,
            label: None,
            head_left: false,
        }
    }

    /// `(left, right)` class indexes as the source wrote them —
    /// `(to, from)` when [`ClassRel::head_left`], else `(from, to)`.
    /// The layout ranks the left operand first.
    pub fn source_order(&self) -> (usize, usize) {
        if self.head_left {
            (self.to, self.from)
        } else {
            (self.from, self.to)
        }
    }
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
#[derive(Debug)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub items: Vec<SeqItem>,
    /// `autonumber` — messages get a bold `1.` … `n.` prefix.
    pub autonumber: bool,
    /// Accessible title from mermaid's `accTitle:` statement — replaces
    /// the default `<title>` / `aria-label` of the SVG. `None` = absent
    /// or empty.
    pub acc_title: Option<String>,
    /// Accessible description from `accDescr:` or the multi-line
    /// `accDescr { … }` block — written as the SVG's `<desc>`.
    pub acc_descr: Option<String>,
    /// Repeat the participant boxes below the lifelines, as mermaid's
    /// `mirrorActors` does (on by default there and here). Read from
    /// frontmatter `config.sequence.mirrorActors`.
    pub mirror_actors: bool,
    /// Size each `loop` / `opt` / `alt` / `par` frame to the lifelines
    /// (and notes) it covers, as mermaid does, instead of flowmaid's
    /// default full-width aligned bands. Off by default; read from
    /// frontmatter `config.sequence.scopedFrames` (a flowmaid key that
    /// mermaid ignores — its frames are always scoped).
    pub scoped_frames: bool,
    index: HashMap<String, usize>,
}

impl Default for SequenceDiagram {
    fn default() -> Self {
        SequenceDiagram {
            participants: Vec::new(),
            items: Vec::new(),
            autonumber: false,
            acc_title: None,
            acc_descr: None,
            mirror_actors: true,
            scoped_frames: false,
            index: HashMap::new(),
        }
    }
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
    /// Declared with `actor` — drawn as a stick figure with its name
    /// below instead of the filled participant box.
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
    /// Flow direction from a `direction TB|BT|LR|RL` statement. The
    /// default is top-to-bottom ([`Direction::TD`]), as in mermaid.
    pub direction: Direction,
    /// Accessible title from mermaid's `accTitle:` statement — replaces
    /// the default `<title>` / `aria-label` of the SVG. `None` = absent
    /// or empty.
    pub acc_title: Option<String>,
    /// Accessible description from `accDescr:` or the multi-line
    /// `accDescr { … }` block — written as the SVG's `<desc>`.
    pub acc_descr: Option<String>,
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
                alias: None,
                attrs: Vec::new(),
            });
            self.index.insert(name.to_string(), i);
            i
        }
    }
}

/// A database entity, rendered as a table. `#[non_exhaustive]`: build
/// one with [`ErDiagram::ensure_entity`].
#[derive(Debug)]
#[non_exhaustive]
pub struct Entity {
    /// The entity's id: what relationships name it by.
    pub name: String,
    /// Display name from mermaid's alias syntax (`p[Person]`,
    /// `c["Customer Account"]`): drawn in place of `name`. `None` =
    /// draw `name`.
    pub alias: Option<String>,
    pub attrs: Vec<Attr>,
}

impl Entity {
    /// The text a table header shows: the alias when given, else the
    /// name.
    pub fn title(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }
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
/// `#[non_exhaustive]`: build one with `PieChart::default()` and set
/// the (public) fields.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct PieChart {
    /// `pie title …` or a standalone `title …` line.
    pub title: Option<String>,
    /// `pie showData` — legend entries carry the raw value.
    pub show_data: bool,
    pub slices: Vec<PieSlice>,
    /// Accessible title from mermaid's `accTitle:` statement — replaces
    /// the default `<title>` / `aria-label` of the SVG. `None` = absent
    /// or empty.
    pub acc_title: Option<String>,
    /// Accessible description from `accDescr:` or the multi-line
    /// `accDescr { … }` block — written as the SVG's `<desc>`.
    pub acc_descr: Option<String>,
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

impl Mindmap {
    /// Append a node under `parent` and return its index, filling in
    /// what the parser derives from indentation: `depth`, `branch` and
    /// the parent's `children`. `None` adds the root; once there is
    /// one, `None` hangs the node off the root, as the parser does with
    /// a stray shallow line, so `nodes[0]` stays the only root. The
    /// node starts without icon, classes or style — set them on
    /// `nodes[i]` afterwards.
    ///
    /// # Panics
    /// If `parent` is not an index into [`Mindmap::nodes`].
    pub fn add_node(
        &mut self,
        parent: Option<usize>,
        text: impl Into<String>,
        shape: MindShape,
    ) -> usize {
        let idx = self.nodes.len();
        let parent = parent.or(if idx > 0 { Some(0) } else { None });
        // A node's colored branch is its depth-1 ancestor; the root
        // has none. depth-1 nodes seed their own branch.
        let (depth, branch) = match parent {
            None => (0, None),
            Some(p) if self.nodes[p].depth == 0 => (1, Some(idx)),
            Some(p) => (self.nodes[p].depth + 1, self.nodes[p].branch),
        };
        self.nodes.push(MindNode {
            text: text.into(),
            shape,
            parent,
            children: Vec::new(),
            depth,
            branch,
            icon: None,
            classes: Vec::new(),
            style: NodeStyle::default(),
        });
        if let Some(p) = parent {
            self.nodes[p].children.push(idx);
        }
        idx
    }
}

/// One mindmap node. `text` may contain `\n` (from `<br/>`) for a
/// multi-line label. `branch` is the index of the depth-1 ancestor
/// this node descends from (its colored branch); `None` for the root.
///
/// `#[non_exhaustive]`: build one with [`Mindmap::add_node`] (the
/// parser uses it too) and set the (public) fields.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MindNode {
    pub text: String,
    pub shape: MindShape,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub depth: usize,
    pub branch: Option<usize>,
    /// The `::icon(…)` line under the node, verbatim (`fa fa-book`,
    /// `mdi mdi-skull`); `None` without one.
    pub icon: Option<String>,
    /// CSS classes from `:::a b` lines under the node, in order. The SVG
    /// carries them as a `class` attribute (mermaid leaves styling them
    /// to the page), and any matching mindmap `classDef` is folded into
    /// [`MindNode::style`].
    pub classes: Vec<String>,
    /// Colours from the `classDef`s of [`MindNode::classes`] (later
    /// classes win); empty `NodeStyle` = the branch colours.
    pub style: NodeStyle,
}

/// A user-journey diagram (`journey` header): an ordered list of
/// sections, each holding scored tasks. `actors` is the de-duplicated
/// set of every participant, in first-appearance order — its index is
/// the stable color key for the legend and per-task dots.
///
/// `#[non_exhaustive]`: build one with `Journey::default()` and set the
/// (public) fields.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Journey {
    pub title: Option<String>,
    pub sections: Vec<JourneySection>,
    pub actors: Vec<String>,
    /// Accessible title from mermaid's `accTitle:` statement — replaces
    /// the default `<title>` / `aria-label` of the SVG. `None` = absent
    /// or empty.
    pub acc_title: Option<String>,
    /// Accessible description from `accDescr:` or the multi-line
    /// `accDescr { … }` block — written as the SVG's `<desc>`.
    pub acc_descr: Option<String>,
}

/// A named group of consecutive tasks (`section X`). Tasks before the
/// first `section` land in one leading section with an empty name.
#[derive(Debug)]
pub struct JourneySection {
    pub name: String,
    pub tasks: Vec<JourneyTask>,
}

/// One step: `Task name: score: Actor1, Actor2`. `score` is clamped to
/// 1..=5 (satisfaction); `actors` are indices into [`Journey::actors`].
/// `#[non_exhaustive]`: build one with [`JourneyTask::new`] (the parser
/// uses it too) and set the (public) fields.
#[derive(Debug)]
#[non_exhaustive]
pub struct JourneyTask {
    pub name: String,
    /// The score rounded to a whole 1..=5 step (face colour and mouth).
    pub score: u8,
    pub actors: Vec<usize>,
    /// The score as written, clamped to 1..=5 but not rounded: mermaid
    /// reads it with `Number()`, so `3.5` is a valid score and sits
    /// halfway between 3 and 4 on the satisfaction curve.
    pub exact_score: f64,
}

impl JourneyTask {
    /// A task scored `score`, clamped to 1..=5 into
    /// [`JourneyTask::exact_score`] and rounded into
    /// [`JourneyTask::score`], exactly as the parser reads a score. A
    /// NaN (which the parser rejects) counts as the neutral 3.
    /// `actors` are indices into [`Journey::actors`].
    pub fn new(name: impl Into<String>, score: f64, actors: Vec<usize>) -> Self {
        let exact_score = if score.is_nan() {
            3.0
        } else {
            score.clamp(1.0, 5.0)
        };
        JourneyTask {
            name: name.into(),
            // `as` is exact here: the value is a whole number in 1..=5.
            score: exact_score.round() as u8,
            actors,
            exact_score,
        }
    }
}

/// Timeline diagram (`timeline` header). `#[non_exhaustive]`: build
/// one with `Timeline::default()` and set the (public) fields.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Timeline {
    pub title: Option<String>,
    pub direction: TimelineDirection,
    pub sections: Vec<TimelineSection>,
    /// Accessible title from mermaid's `accTitle:` statement — replaces
    /// the default `<title>` / `aria-label` of the SVG. `None` = absent
    /// or empty.
    pub acc_title: Option<String>,
    /// Accessible description from `accDescr:` or the multi-line
    /// `accDescr { … }` block — written as the SVG's `<desc>`.
    pub acc_descr: Option<String>,
}

/// Timeline layout orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineDirection {
    /// `timeline` / `timeline LR` — periods as left-to-right columns.
    #[default]
    LeftRight,
    /// `timeline TD` — periods stacked top-to-bottom.
    TopDown,
}

/// A named group of consecutive periods (`section X`). Periods before
/// the first `section` land in one leading section with an empty name.
#[derive(Debug)]
pub struct TimelineSection {
    pub name: String,
    pub periods: Vec<TimelinePeriod>,
}

/// One period: the time label plus its events.
#[derive(Debug)]
pub struct TimelinePeriod {
    pub period: String,
    pub events: Vec<String>,
}

/// Node outline in a mindmap, from the wrapper around its text.
///
/// `#[non_exhaustive]`: match it with a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MindShape {
    /// No wrapper — mermaid's default node: a box with softly rounded
    /// top corners and a darker underline along its flat bottom.
    #[default]
    Default,
    /// `id(text)` — a pill (fully rounded ends).
    Rounded,
    /// `id[text]` — sharp corners.
    Square,
    /// `id((text))` — a true circle (width = height).
    Circle,
    /// `id{{text}}`.
    Hexagon,
    /// `id))text((` — a "bang": a starburst of evenly spaced spikes of
    /// one depth around an ellipse that holds the label.
    Bang,
    /// `id)text(` — a cloud: round overlapping lobes around an ellipse
    /// that holds the label.
    Cloud,
}

/// Git graph orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitOrientation {
    /// Left-to-right (mermaid default).
    #[default]
    LR,
    /// Top-to-bottom.
    TB,
    /// Bottom-to-top.
    BT,
}

/// Visual weight of a git commit node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommitKind {
    /// Default solid commit.
    #[default]
    Normal,
    /// A reverted commit: drawn as a disc crossed out with an X.
    Reverse,
    /// Emphasised commit: drawn as a square instead of a disc.
    Highlight,
}

/// One commit in a `gitGraph`.
#[derive(Debug)]
pub struct GitCommit {
    /// Display id ("1", "init", ...).
    pub id: String,
    /// Index into [`GitGraph::branches`].
    pub branch: usize,
    /// Parent commit index on the same branch, or `None` for the first commit.
    pub parent: Option<usize>,
    /// Second parent for merge commits.
    pub second_parent: Option<usize>,
    /// Optional tag rendered above the commit.
    pub tag: Option<String>,
    /// Visual style.
    pub kind: CommitKind,
    /// Global commit sequence number (0-based), used for x placement.
    pub seq: usize,
}

/// One branch lane in a `gitGraph`.
#[derive(Debug)]
pub struct GitBranch {
    pub name: String,
    /// Parent branch index, if this branch was created with `branch`.
    pub parent_branch: Option<usize>,
    /// Commit index this branch was forked from.
    pub parent_commit: Option<usize>,
    /// Index of the latest commit on this branch.
    pub head: Option<usize>,
    /// Lane position, 0 = the first lane (top in LR, left in TB/BT):
    /// main first, then creation order, unless `branch name order: n`
    /// moved the lanes (see the gitGraph parser). The renderer ranks
    /// branches by this value (ties by index), so a hand-built graph
    /// may use any increasing numbers.
    pub order: usize,
}

/// A `gitGraph` diagram: branches, commits, and the current branch.
#[derive(Debug)]
pub struct GitGraph {
    pub orientation: GitOrientation,
    pub branches: Vec<GitBranch>,
    pub commits: Vec<GitCommit>,
    /// Index into [`GitGraph::branches`] of the active branch.
    pub current_branch: usize,
}

impl Default for GitGraph {
    /// A default git graph contains a single `main` branch, matching the
    /// parser's initial state. This keeps `GitGraph::default()` usable
    /// without panicking on `branches[0]`.
    fn default() -> Self {
        Self {
            orientation: GitOrientation::LR,
            branches: vec![GitBranch {
                name: "main".to_string(),
                parent_branch: None,
                parent_commit: None,
                head: None,
                order: 0,
            }],
            commits: Vec::new(),
            current_branch: 0,
        }
    }
}

/// Architecture diagram (`architecture-beta`).
///
/// `#[non_exhaustive]`: build one with `Architecture::default()` and
/// set the (public) fields.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Architecture {
    pub groups: Vec<ArchGroup>,
    /// Services and junctions, in declaration order; edges index this.
    pub services: Vec<ArchService>,
    pub edges: Vec<ArchEdge>,
    /// Accessible title from mermaid's `accTitle:` statement — replaces
    /// the default `<title>` / `aria-label` of the SVG. `None` = absent
    /// or empty.
    pub acc_title: Option<String>,
    /// Accessible description from `accDescr:` or the multi-line
    /// `accDescr { … }` block — written as the SVG's `<desc>`.
    pub acc_descr: Option<String>,
}

/// A container of services.
#[derive(Debug)]
pub struct ArchGroup {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub parent: Option<usize>,
}

/// A single service/node inside an architecture diagram.
/// `#[non_exhaustive]`: build one with [`ArchService::new`] and set the
/// (public) fields.
#[derive(Debug)]
#[non_exhaustive]
pub struct ArchService {
    pub id: String,
    /// Label drawn in the box; empty when the source gave no `[title]`.
    pub title: String,
    pub icon: Option<String>,
    pub group: Option<usize>,
    /// Declared with `junction` rather than `service`: a bare point
    /// where edges meet, drawn as a dot with no box, icon or title
    /// (`title` is empty and `icon` is `None`). Every side of a
    /// junction is its centre.
    pub junction: bool,
}

impl ArchService {
    /// A service `id` labelled `title` (may be empty), outside every
    /// group, without an icon. For a junction set
    /// [`ArchService::junction`] and leave `title` empty.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        ArchService {
            id: id.into(),
            title: title.into(),
            icon: None,
            group: None,
            junction: false,
        }
    }
}

/// One connection between two services via named ports.
/// `#[non_exhaustive]`: build one with [`ArchEdge::new`] and set the
/// (public) fields.
#[derive(Debug)]
#[non_exhaustive]
pub struct ArchEdge {
    pub from: usize,
    pub from_side: ArchSide,
    pub to: usize,
    pub to_side: ArchSide,
    /// Arrowhead on the `from` side (pointing toward `from`).
    pub arrow_start: bool,
    /// Arrowhead on the `to` side (pointing toward `to`).
    pub arrow_end: bool,
    /// Mermaid's `{group}` modifier on the `from` end
    /// (`a{group}:R -- L:b`): the edge leaves the frame of the group
    /// `from` sits in, not the service's own box. `false` for a service
    /// outside any group.
    pub from_group: bool,
    /// [`ArchEdge::from_group`] for the `to` end (`… L:b{group}`).
    pub to_group: bool,
    /// Edge label from mermaid's `a:R -[label]- L:b` form; `None` = an
    /// unlabelled `--` edge.
    pub title: Option<String>,
}

impl ArchEdge {
    /// A plain `from:from_side -- to_side:to` edge (indices into
    /// [`Architecture::services`]): no arrowheads, no `{group}`
    /// modifier, no label.
    pub fn new(from: usize, from_side: ArchSide, to: usize, to_side: ArchSide) -> Self {
        ArchEdge {
            from,
            from_side,
            to,
            to_side,
            arrow_start: false,
            arrow_end: false,
            from_group: false,
            to_group: false,
            title: None,
        }
    }
}

/// Port side on an architecture service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchSide {
    /// Top.
    #[default]
    T,
    /// Bottom.
    B,
    /// Left.
    L,
    /// Right.
    R,
}

// ------------------------------------------------------------------
// Sankey
// ------------------------------------------------------------------

/// Sankey diagram (`sankey-beta` header). The body is CSV — one
/// `source,target,value` row per link — and nodes are created on first
/// mention, so the model keeps them in first-appearance order for a
/// stable layout.
#[derive(Debug, Default)]
pub struct SankeyDiagram {
    /// Node labels, in first-appearance order. A link's endpoints are
    /// indices into this vector; mermaid uses the label as the id, so
    /// two rows naming the same label share one node.
    pub nodes: Vec<String>,
    pub links: Vec<SankeyLink>,
    pub config: SankeyConfig,
}

impl SankeyDiagram {
    /// Index of `label`, appending it if this is its first mention.
    pub fn ensure_node(&mut self, label: &str) -> usize {
        if let Some(i) = self.nodes.iter().position(|n| n == label) {
            i
        } else {
            self.nodes.push(label.to_string());
            self.nodes.len() - 1
        }
    }
}

/// One CSV row: a weighted flow from one node to another.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLink {
    pub source: usize,
    pub target: usize,
    pub value: f64,
}

/// How a link's ribbon is painted.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SankeyLinkColor {
    /// Fade from the source node's colour to the target's — mermaid's
    /// default.
    #[default]
    Gradient,
    /// Flat, taken from the source node.
    Source,
    /// Flat, taken from the target node.
    Target,
    /// Flat, taken verbatim from the config (any CSS colour).
    Fixed(String),
}

/// Where nodes with slack are pushed inside their column.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SankeyAlignment {
    /// Stretch so both edges are flush — mermaid's default.
    #[default]
    Justify,
    Left,
    Right,
    Center,
}

/// How sankey node labels are drawn (`config.sankey.labelStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SankeyLabelStyle {
    /// Plain text beside the node — mermaid's default.
    #[default]
    Legacy,
    /// Text over a white halo so it stays readable across ribbons; as
    /// in mermaid, labels left of the busiest column hang off their
    /// node's left side.
    Outlined,
}

/// The `config.sankey.*` block from YAML frontmatter. Defaults mirror
/// mermaid's `config.schema.yaml` exactly, so a diagram with no config
/// renders the way mermaid renders it.
///
/// `#[non_exhaustive]`: build one with `SankeyConfig::default()` and
/// set the fields you need.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SankeyConfig {
    pub width: f64,
    pub height: f64,
    pub link_color: SankeyLinkColor,
    pub node_alignment: SankeyAlignment,
    /// Draw each node's total next to its label.
    pub show_values: bool,
    /// Wrapped around a shown value (`prefix` + value + `suffix`).
    pub prefix: String,
    pub suffix: String,
    pub node_width: f64,
    pub node_padding: f64,
    /// `nodeColors`: node label → CSS colour, in source order. A listed
    /// node is painted in that colour verbatim (escaped in the SVG);
    /// the rest take the palette.
    pub node_colors: Vec<(String, String)>,
    /// `labelStyle`: plain or haloed labels.
    pub label_style: SankeyLabelStyle,
}

impl Default for SankeyConfig {
    fn default() -> Self {
        Self {
            width: 600.0,
            height: 400.0,
            link_color: SankeyLinkColor::Gradient,
            node_alignment: SankeyAlignment::Justify,
            show_values: true,
            prefix: String::new(),
            suffix: String::new(),
            node_width: 10.0,
            node_padding: 12.0,
            node_colors: Vec::new(),
            label_style: SankeyLabelStyle::Legacy,
        }
    }
}
