//! Advance / swimlane diagram rendering.
//!
//! Input is a small JSON object describing lanes, nodes inside lanes,
//! and edges between nodes. The engine lays out vertical or horizontal lanes,
//! orders nodes top-down (or left-to-right), and routes orthogonal edges.

use crate::json::{
    as_array, as_number, as_object, as_str, escape_json_str, obj_get, parse_json, JsonValue,
};
use crate::layout::{text_width, BASE_H, LINE_H, MIN_W, PAD_X};
use crate::model::{EdgeKind, NodeStyle, Shape};
use crate::parser::normalize_breaks;
use crate::scene::{escape, svg_open, SvgOptions};

/// A content-derived suffix for this diagram's marker ids — see
/// [`crate::scene::DefsKey`] for why it is a hash and not a counter.
fn scene_key(sc: &AdvanceScene) -> String {
    let mut k = crate::scene::DefsKey::new();
    for n in &sc.nodes {
        k.eat(n.id.as_bytes());
        k.eat_f64(n.x);
        k.eat_f64(n.y);
    }
    for e in &sc.edges {
        k.eat(e.from.as_bytes());
        k.eat(e.to.as_bytes());
        for p in &e.points {
            k.eat_f64(p.0);
            k.eat_f64(p.1);
        }
    }
    k.finish()
}

// ------------------------------------------------------------------
// Public error type
// ------------------------------------------------------------------

/// Something went wrong while parsing or laying out an advance diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceError {
    pub message: String,
}

impl std::fmt::Display for AdvanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AdvanceError {}

fn adv_err(message: impl Into<String>) -> AdvanceError {
    AdvanceError {
        message: message.into(),
    }
}

/// Build a parse error with line/column context baked into the message.
/// `line_no` is zero-based; `col` (when known) is a char offset into the
/// raw line and drives the `^` caret under the snippet.
fn text_err(
    source: &str,
    line_no: usize,
    col: Option<usize>,
    message: impl Into<String>,
) -> AdvanceError {
    let line_text = source.lines().nth(line_no).unwrap_or("");
    let snippet = line_text.trim_end();
    let mut out = format!("line {}: {}", line_no + 1, message.into());
    if !snippet.is_empty() {
        out.push_str("\n  ");
        out.push_str(snippet);
    }
    if let Some(c) = col {
        let c = c.min(snippet.chars().count());
        out.push_str("\n  ");
        for _ in 0..c {
            out.push(' ');
        }
        out.push('^');
    }
    adv_err(out)
}

// ------------------------------------------------------------------
// Public model & config
// ------------------------------------------------------------------

/// Orientation of the swimlanes and node flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdvanceDirection {
    /// Vertical columns (default), top-to-bottom node flow.
    #[default]
    Vertical,
    /// Horizontal rows, left-to-right node flow.
    Horizontal,
}

/// Node ordering strategy within each lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdvanceOrder {
    /// Follow declaration order in the JSON `nodes` array (default).
    #[default]
    Declaration,
    /// Topological sort on intra-lane edges with barycenter cross-lane refinement.
    Topology,
}

/// Deepest nesting accepted for lanes and sub-elements, in BOTH
/// front-ends — the text parser used to have no cap at all.
const MAX_NEST_DEPTH: usize = 16;
/// Where an anchor sits along its side when no offset is given.
const DEFAULT_ANCHOR_OFFSET: f64 = 0.5;

/// Height of a label band: one line, plus `LINE_H` per extra line.
fn label_band_h(label: &str) -> f64 {
    BASE_H + (crate::layout::line_count(label) - 1) as f64 * LINE_H
}

/// The edge separators the text DSL understands, shared by the edge
/// parser and by every place that must refuse an edge.
const EDGE_SEPARATORS: [&str; 4] = ["-->", "-.->", "==>", "---"];

/// The first edge separator that sits OUTSIDE every bracket and quote,
/// as `(byte offset, separator)`. A `---` inside a label is text, not an
/// operator, so `a[A] { p[x---y] }` is a block and not a broken edge.
///
/// Separators are tried in the order the edge parser has always tried
/// them, so a line like `a ---> b` still splits exactly where it used to.
fn find_edge_sep(line: &str) -> Option<(usize, &'static str)> {
    let bytes = line.as_bytes();
    let mut depth = vec![0i32; line.len() + 1];
    let (mut d, mut quote) = (0i32, false);
    for (i, c) in line.char_indices() {
        depth[i] = if quote { 1 } else { d };
        match c {
            '"' => quote = !quote,
            '[' | '(' | '{' if !quote => d += 1,
            ']' | ')' | '}' if !quote => d -= 1,
            _ => {}
        }
    }
    let free = |at: usize| at < bytes.len() && depth[at] == 0;
    EDGE_SEPARATORS.iter().find_map(|sep| {
        let mut from = 0;
        while let Some(rel) = line[from..].find(sep) {
            let at = from + rel;
            if free(at) {
                return Some((at, *sep));
            }
            from = at + 1;
        }
        None
    })
}

fn is_edge_line(line: &str) -> bool {
    find_edge_sep(line).is_some()
}

/// Top-level directives — never legal inside a node block, and never
/// the start of an inline block.
fn is_directive_line(line: &str) -> bool {
    [
        "style ",
        "class ",
        "classDef ",
        "config ",
        "title ",
        "desc ",
        "description ",
        "lane ",
        "swimlane",
    ]
    .iter()
    .any(|k| line.starts_with(k))
}

/// A named connection point on the boundary of a node or sub-element.
///
/// `offset` runs along the side: `0.0` is the left/top end, `1.0` the
/// right/bottom end, `0.5` the centre. The four plain sides behave as
/// built-in anchors at `0.5`, so `a:right` and a declared
/// `anchor r right 0.5` resolve to the same point.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceAnchor {
    pub id: String,
    pub side: AdvanceSide,
    pub offset: f64,
}

/// How a node (or element) stacks its sub-elements: as compartments
/// top-to-bottom, or side by side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ElementLayout {
    #[default]
    Column,
    Row,
}

impl ElementLayout {
    pub fn name(&self) -> &'static str {
        match self {
            ElementLayout::Column => "column",
            ElementLayout::Row => "row",
        }
    }
}

/// A sub-element — a compartment, cell or pin — inside a node. It has
/// its own id, can carry anchors and nested elements, and can be an
/// edge endpoint (`node.element`). Laid out without coordinates: the
/// parent grows to fit.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceElement {
    pub id: String,
    pub label: String,
    pub anchors: Vec<AdvanceAnchor>,
    pub elements: Vec<AdvanceElement>,
    pub layout: ElementLayout,
    /// Per-element style overrides; empty = inherit the node's look.
    pub style: NodeStyle,
}

/// What an edge end attaches to, beyond the node: a side keyword or a
/// named anchor.
#[derive(Debug, Clone, PartialEq)]
pub enum AnchorRef {
    Side(AdvanceSide),
    Named(String),
}

/// One end of an edge, as written: `node`, an optional path of
/// sub-element ids (`node.a.b`), and an optional `:side` / `@anchor`.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceEnd {
    pub node: String,
    pub path: Vec<String>,
    pub at: Option<AnchorRef>,
}

impl AdvanceEnd {
    /// A plain node reference with no path and no anchor.
    pub fn node(id: &str) -> Self {
        AdvanceEnd {
            node: id.to_string(),
            path: Vec::new(),
            at: None,
        }
    }

    /// Whether this end names something finer than a node side —
    /// a sub-element or a named anchor — and so must be resolved to
    /// an explicit point before routing.
    pub fn is_terminal(&self) -> bool {
        !self.path.is_empty() || matches!(self.at, Some(AnchorRef::Named(_)))
    }

    /// The reference as it is written in the text DSL and in JSON
    /// (`node.elem@anchor` / `node:side` / `node`).
    pub fn to_ref(&self) -> String {
        let mut s = self.node.clone();
        for seg in &self.path {
            s.push('.');
            s.push_str(seg);
        }
        match &self.at {
            Some(AnchorRef::Side(side)) => {
                s.push(':');
                s.push_str(side.name());
            }
            Some(AnchorRef::Named(a)) => {
                s.push('@');
                s.push_str(a);
            }
            None => {}
        }
        s
    }
}

/// Parse one edge end: `node ('.' element)* ('@' anchor | ':' side)?`.
/// `.` descends into sub-elements, `@` names an anchor, `:` picks a
/// side; `@` and `:` are terminal and exclusive. A `:` followed by a
/// word that is not a side keyword is left in the id, matching the
/// old behaviour, so the unknown-node error still names what was
/// typed.
fn parse_end(s: &str) -> Result<AdvanceEnd, String> {
    let s = s.trim();
    let (head, at) = if let Some((h, a)) = s.rsplit_once('@') {
        if a.is_empty() || a.contains('.') || a.contains(':') {
            return Err(format!("invalid anchor reference '{}'", s));
        }
        (h, Some(AnchorRef::Named(a.to_string())))
    } else if let Some((h, side)) = s.rsplit_once(':') {
        match parse_side(side) {
            Some(side) if !h.is_empty() && !h.ends_with(':') => (h, Some(AnchorRef::Side(side))),
            _ => (s, None),
        }
    } else {
        (s, None)
    };
    let mut segs = head.split('.');
    let node = segs.next().unwrap_or("").trim().to_string();
    if node.is_empty() {
        return Err(format!("invalid edge end '{}'", s));
    }
    let mut path = Vec::new();
    for seg in segs {
        let seg = seg.trim();
        if seg.is_empty() {
            return Err(format!("empty sub-element name in '{}'", s));
        }
        path.push(seg.to_string());
    }
    Ok(AdvanceEnd { node, path, at })
}

/// Ids take part in the reference grammar, so the two characters it
/// reserves may not appear in them.
fn check_id(kind: &str, id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err(format!("{} id cannot be empty", kind));
    }
    if let Some(c) = id.chars().find(|c| *c == '.' || *c == '@') {
        return Err(format!(
            "{} id '{}' may not contain '{}' — it is reserved for edge references (node.element@anchor)",
            kind, id, c
        ));
    }
    // An anchor is referenced as `node@id`, and `parse_end` refuses a
    // `@`-suffix containing ':', so a colon anywhere makes the anchor
    // undeclarable-in-practice.
    if kind == "anchor" && id.contains(':') {
        return Err(format!(
            "{} id '{}' may not contain ':' — it is reserved for edge references (node:side)",
            kind, id
        ));
    }
    // `a:right` would parse as node `a` on side `right` in every edge,
    // so such an id could never be referenced.
    if let Some((_, suffix)) = id.rsplit_once(':') {
        if parse_side(suffix).is_some() {
            return Err(format!(
                "{} id '{}' ends in the side keyword '{}' and could never be referenced by an edge",
                kind, id, suffix
            ));
        }
    }
    Ok(())
}

/// Split on `;` outside every bracket and quote, so a nested `{ a; b }`
/// and a `;` inside a label both stay whole.
fn split_top_level(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut quote) = (0i32, false);
    let mut start = 0;
    for (i, c) in body.char_indices() {
        match c {
            '"' => quote = !quote,
            '{' | '[' | '(' if !quote => depth += 1,
            '}' | ']' | ')' if !quote => depth -= 1,
            ';' if !quote && depth == 0 => {
                out.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&body[start..]);
    out
}

/// Whether a one-line body is unmistakably block content — several
/// statements, an `anchor`/`layout` directive, or a sub-element
/// declaration (`id[…]` / `id(…)`). A bare label such as `Text` is not,
/// which keeps `c {Text}` a diamond.
fn body_looks_like_block(body: &str) -> bool {
    let stmts = split_top_level(body);
    if stmts.len() > 1 {
        return true;
    }
    let st = stmts[0].trim();
    if st.starts_with("anchor ") || st.starts_with("layout ") {
        return true;
    }
    match st.find(['[', '(']) {
        Some(i) if i > 0 => st[..i]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-'),
        _ => false,
    }
}

/// `decl { stmt; stmt }` on one line. The split is the first `{` outside
/// any bracket or quote that leaves a valid node declaration in front
/// of it. A brace glued to the id (`c{Text}`, `h{{Hex}}`) is a shape,
/// and `c {Text}` with a bare id stays a diamond unless the body is
/// unmistakably block content. Edges and directives are never blocks.
fn split_inline_block(line: &str) -> Option<(&str, &str)> {
    // `lane … { … }` is the one directive that may open a block.
    let is_lane = line.starts_with("lane ");
    if !line.ends_with('}') || is_edge_line(line) || (is_directive_line(line) && !is_lane) {
        return None;
    }
    let bytes = line.as_bytes();
    let (mut sq, mut par, mut quote) = (0i32, 0i32, false);
    for (pos, c) in line.char_indices() {
        match c {
            '"' => quote = !quote,
            '[' if !quote => sq += 1,
            ']' if !quote => sq -= 1,
            '(' if !quote => par += 1,
            ')' if !quote => par -= 1,
            '{' if !quote && sq == 0 && par == 0 && pos > 0 => {
                let prev = bytes[pos - 1];
                if !(prev == b' ' || prev == b'\t' || prev == b']' || prev == b')' || prev == b'}')
                {
                    continue; // glued to the id: a shape brace
                }
                let prefix = line[..pos].trim_end();
                let body = line[pos + 1..line.len() - 1].trim();
                // A lane has no shape syntax to be confused with, so it
                // never needs the `c {Text}`-is-a-diamond guard below —
                // which used to swallow `lane l "L" { a }` whole.
                if is_lane {
                    return Some((prefix, body));
                }
                let closed_shape =
                    prefix.ends_with(']') || prefix.ends_with(')') || prefix.ends_with('}');
                if !closed_shape && !body_looks_like_block(body) {
                    continue;
                }
                match parse_text_node_shorthand(prefix) {
                    Some((id, _, _)) if !id.is_empty() => return Some((prefix, body)),
                    _ => continue,
                }
            }
            _ => {}
        }
    }
    None
}

/// Turn one source line into statements, expanding `decl { a; b }`
/// (recursively, so blocks may nest on one line) while every statement
/// keeps the line number it came from. Depth is capped like the JSON
/// front-end, so a hostile line errors instead of overflowing the stack.
fn expand_inline(
    line_no: usize,
    line: &str,
    depth: usize,
    out: &mut Vec<(usize, String)>,
) -> Result<(), String> {
    match split_inline_block(line) {
        Some((decl, body)) => {
            if depth >= MAX_NEST_DEPTH {
                return Err(format!("block nesting exceeds limit of {}", MAX_NEST_DEPTH));
            }
            out.push((line_no, format!("{} {{", decl)));
            for st in split_top_level(body) {
                let st = st.trim();
                if !st.is_empty() {
                    expand_inline(line_no, st, depth + 1, out)?;
                }
            }
            out.push((line_no, "}".to_string()));
        }
        None => out.push((line_no, line.to_string())),
    }
    Ok(())
}

/// Which sides of the `idx`-th of `len` children reach their parent's
/// boundary under `layout`. Order: `[left, right, top, bottom]`.
///
/// `top` is never exposed: the host's label band is always the first
/// occupant, so a lead out of a child's top would run through the
/// label text.
fn exposed_in(layout: ElementLayout, len: usize, idx: usize) -> [bool; 4] {
    let first = idx == 0;
    let last = idx + 1 == len;
    match layout {
        ElementLayout::Column => [true, true, false, last],
        ElementLayout::Row => [first, last, false, true],
    }
}

fn side_index(side: AdvanceSide) -> usize {
    match side {
        AdvanceSide::Left => 0,
        AdvanceSide::Right => 1,
        AdvanceSide::Top => 2,
        AdvanceSide::Bottom => 3,
    }
}

/// Walk `path` down from a node, returning the element it names and
/// the sides of that element that are exposed through EVERY level up
/// to the node boundary.
fn resolve_element<'a>(
    node: &'a AdvanceNode,
    path: &[String],
) -> Result<(&'a AdvanceElement, [bool; 4]), String> {
    let mut elems = &node.elements;
    let mut layout = node.layout;
    let mut exposed = [true; 4];
    let mut found: Option<&AdvanceElement> = None;
    for seg in path {
        let idx = elems
            .iter()
            .position(|e| &e.id == seg)
            .ok_or_else(|| format!("node '{}' has no sub-element '{}'", node.id, seg))?;
        let e = &elems[idx];
        let here = exposed_in(layout, elems.len(), idx);
        for (x, h) in exposed.iter_mut().zip(here) {
            *x = *x && h;
        }
        elems = &e.elements;
        layout = e.layout;
        found = Some(e);
    }
    found
        .map(|e| (e, exposed))
        .ok_or_else(|| "empty sub-element path".to_string())
}

/// Check one edge end against the declared nodes and work out the side
/// it attaches on. Errors name what is missing; an interior side on a
/// sub-element is refused here rather than drawn as a lead that would
/// pierce a sibling.
fn resolve_end_side(
    nodes: &[AdvanceNode],
    end: &AdvanceEnd,
) -> Result<Option<AdvanceSide>, String> {
    let node = nodes
        .iter()
        .find(|n| n.id == end.node)
        .ok_or_else(|| format!("edge references unknown node '{}'", end.node))?;
    let (anchors, exposed): (&Vec<AdvanceAnchor>, [bool; 4]) = if end.path.is_empty() {
        (&node.anchors, [true; 4])
    } else {
        let (e, exposed) = resolve_element(node, &end.path)?;
        (&e.anchors, exposed)
    };
    let side = match &end.at {
        None => return Ok(None),
        Some(AnchorRef::Side(side)) => *side,
        Some(AnchorRef::Named(id)) => {
            let host = std::iter::once(end.node.as_str())
                .chain(end.path.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(".");
            anchors
                .iter()
                .find(|a| &a.id == id)
                .map(|a| a.side)
                .ok_or_else(|| format!("'{}' has no anchor '{}'", host, id))?
        }
    };
    if !exposed[side_index(side)] {
        return Err(format!(
            "{} is not an exposed side — it faces a sibling or the label band inside the node",
            end.to_ref()
        ));
    }
    Ok(Some(side))
}

fn parse_layout_word(w: &str) -> Option<ElementLayout> {
    match w {
        "column" | "col" => Some(ElementLayout::Column),
        "row" => Some(ElementLayout::Row),
        _ => None,
    }
}

/// `anchor <id> <side> [offset]` inside a node or element block.
fn parse_anchor_line(rest: &str) -> Result<AdvanceAnchor, String> {
    let mut it = rest.split_whitespace();
    let id = it.next().ok_or("anchor needs an id")?.to_string();
    check_id("anchor", &id)?;
    let side_w = it
        .next()
        .ok_or_else(|| format!("anchor '{}' needs a side (left, right, top, bottom)", id))?;
    let side = parse_side(side_w)
        .ok_or_else(|| format!("unknown side '{}' for anchor '{}'", side_w, id))?;
    let offset = match it.next() {
        None => DEFAULT_ANCHOR_OFFSET,
        Some(o) => o
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
            .ok_or_else(|| {
                format!(
                    "anchor '{}' offset must be a number in 0..=1, got '{}'",
                    id, o
                )
            })?,
    };
    if it.next().is_some() {
        return Err(format!("too many words after anchor '{}'", id));
    }
    Ok(AdvanceAnchor { id, side, offset })
}

/// An anchor side on a node where an edge attaches (`a:right --> b:top`).
/// Edges without a side keep the automatic routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl AdvanceSide {
    /// The canonical keyword for this side.
    pub fn name(&self) -> &'static str {
        match self {
            AdvanceSide::Left => "left",
            AdvanceSide::Right => "right",
            AdvanceSide::Top => "top",
            AdvanceSide::Bottom => "bottom",
        }
    }
}

fn parse_side(s: &str) -> Option<AdvanceSide> {
    match s {
        "left" => Some(AdvanceSide::Left),
        "right" => Some(AdvanceSide::Right),
        "top" => Some(AdvanceSide::Top),
        "bottom" => Some(AdvanceSide::Bottom),
        _ => None,
    }
}

/// Visual styling overrides for advance diagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceStyle {
    pub lane_fill: String,
    pub lane_stroke: String,
    pub edge_color: String,
    pub text_color: String,
    pub label_fill: String,
}

pub const DEFAULT_LANE_FILL: &str = "#f9fafd";
pub const DEFAULT_LANE_STROKE: &str = "#d5d9ec";
pub const DEFAULT_EDGE_COLOR: &str = "#44507a";
pub const DEFAULT_TEXT_COLOR: &str = "#232840";
pub const DEFAULT_LABEL_FILL: &str = "#ffffff";

impl Default for AdvanceStyle {
    fn default() -> Self {
        Self {
            lane_fill: DEFAULT_LANE_FILL.to_string(),
            lane_stroke: DEFAULT_LANE_STROKE.to_string(),
            edge_color: DEFAULT_EDGE_COLOR.to_string(),
            text_color: DEFAULT_TEXT_COLOR.to_string(),
            label_fill: DEFAULT_LABEL_FILL.to_string(),
        }
    }
}

/// Per-edge visual overrides. `None` fields fall back to the global
/// [`AdvanceStyle`] at render time.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AdvanceEdgeStyle {
    /// `color:#rrggbb` — stroke color.
    pub color: Option<String>,
    /// `stroke-width:2px` — pixels.
    pub stroke_width: Option<f64>,
    /// `dash:4 2` — dash pattern (pixels) passed straight to
    /// `stroke-dasharray`; text DSL uses space-separated values, JSON
    /// takes any string (commas or spaces).
    pub dash: Option<String>,
    /// `label-fill:#ffffff` — edge label box background.
    pub label_fill: Option<String>,
}

impl AdvanceEdgeStyle {
    /// Overlay `over`'s set fields onto `self`.
    pub fn apply_over(&mut self, over: &AdvanceEdgeStyle) {
        if let Some(v) = &over.color {
            self.color = Some(v.clone());
        }
        if let Some(v) = over.stroke_width {
            self.stroke_width = Some(v);
        }
        if let Some(v) = &over.dash {
            self.dash = Some(v.clone());
        }
        if let Some(v) = &over.label_fill {
            self.label_fill = Some(v.clone());
        }
    }
}

/// Geometric and spacing configuration for advance diagrams.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdvanceConfig {
    pub margin: f64,
    pub lane_gap: f64,
    pub node_gap_y: f64,
    pub lane_pad_x: f64,
    pub lane_pad_y: f64,
    pub lane_title_h: f64,
    pub order: AdvanceOrder,
}

pub const DEFAULT_MARGIN: f64 = 24.0;
pub const DEFAULT_LANE_GAP: f64 = 40.0;
pub const DEFAULT_NODE_GAP_Y: f64 = 48.0;
pub const DEFAULT_LANE_PAD_X: f64 = 20.0;
pub const DEFAULT_LANE_PAD_Y: f64 = 40.0;
pub const DEFAULT_LANE_TITLE_H: f64 = 26.0;

impl Default for AdvanceConfig {
    fn default() -> Self {
        Self {
            margin: DEFAULT_MARGIN,
            lane_gap: DEFAULT_LANE_GAP,
            node_gap_y: DEFAULT_NODE_GAP_Y,
            lane_pad_x: DEFAULT_LANE_PAD_X,
            lane_pad_y: DEFAULT_LANE_PAD_Y,
            lane_title_h: DEFAULT_LANE_TITLE_H,
            order: AdvanceOrder::Declaration,
        }
    }
}

/// One swimlane column (or row). May contain recursive `children` sub-lanes.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceLane {
    pub id: String,
    pub title: String,
    pub children: Vec<AdvanceLane>,
}

/// One node inside a lane.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceNode {
    pub id: String,
    pub label: String,
    pub lane: String,
    pub shape: Shape,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub w: Option<f64>,
    pub h: Option<f64>,
    /// Per-node style overrides; empty = follow the shape theme.
    pub style: NodeStyle,
    /// Named connection points on the node boundary.
    pub anchors: Vec<AdvanceAnchor>,
    /// Sub-elements drawn inside the node; the node grows to fit.
    pub elements: Vec<AdvanceElement>,
    /// How `elements` stack.
    pub layout: ElementLayout,
}

/// One edge between two nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: EdgeKind,
    /// Per-edge style overrides; empty = follow the global [`AdvanceStyle`].
    pub style: AdvanceEdgeStyle,
    /// Anchor side on the source node; `None` = automatic routing.
    /// Derived from `from_end` (a named anchor contributes its side).
    pub from_side: Option<AdvanceSide>,
    /// Anchor side on the target node; `None` = automatic routing.
    pub to_side: Option<AdvanceSide>,
    /// The full source reference — node, sub-element path, anchor.
    pub from_end: AdvanceEnd,
    /// The full target reference.
    pub to_end: AdvanceEnd,
}

/// Parsed advance diagram, ready for layout.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AdvanceDiagram {
    pub title: Option<String>,
    pub description: Option<String>,
    pub direction: AdvanceDirection,
    pub style: AdvanceStyle,
    pub config: AdvanceConfig,
    pub lanes: Vec<AdvanceLane>,
    pub nodes: Vec<AdvanceNode>,
    pub edges: Vec<AdvanceEdge>,
}

/// Positioned geometry for an advance diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceScene {
    pub width: f64,
    pub height: f64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub direction: AdvanceDirection,
    pub style: AdvanceStyle,
    pub lanes: Vec<AdvanceSceneLane>,
    pub nodes: Vec<AdvanceSceneNode>,
    pub edges: Vec<AdvanceSceneEdge>,
    /// How many pairs of edges cross. Zero whenever a crossing-free
    /// orthogonal routing exists on the lattice and the router found it
    /// within its iteration budget; where none exists, the crossings
    /// that remain are the ones the escalated cost made cheapest.
    pub crossings: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceSceneLane {
    pub id: String,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceSceneNode {
    pub id: String,
    pub label: String,
    pub lane: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub shape: Shape,
    /// Per-node style overrides carried from the diagram model.
    pub style: NodeStyle,
    /// Placed sub-elements, flattened depth-first; `parent` links them.
    /// Indices are stable for [`AdvanceHit::Element`].
    pub elements: Vec<AdvanceSceneElement>,
    /// Every resolved anchor on the node or its sub-elements.
    pub anchors: Vec<AdvanceSceneAnchor>,
}

/// A placed sub-element. Like nodes, `x`/`y` is the centre.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceSceneElement {
    pub id: String,
    pub label: String,
    /// Ids from the node's first-level element down to this one.
    pub path: Vec<String>,
    /// Index of the parent in the node's flat `elements`, `None` for a
    /// direct child of the node.
    pub parent: Option<usize>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub style: NodeStyle,
}

/// A resolved anchor: an absolute point on its host's boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceSceneAnchor {
    pub id: String,
    pub side: AdvanceSide,
    /// Index into the node's `elements` when the anchor sits on a
    /// sub-element; `None` when it sits on the node itself.
    pub element: Option<usize>,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceSceneEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: EdgeKind,
    pub points: Vec<(f64, f64)>,
    /// Per-edge style overrides carried from the diagram model.
    pub style: AdvanceEdgeStyle,
    /// Anchor side on the source node, as requested by the input.
    pub from_side: Option<AdvanceSide>,
    /// Anchor side on the target node, as requested by the input.
    pub to_side: Option<AdvanceSide>,
    /// Resolved label anchor (center point), set during routing when the
    /// automatic placement dodges nodes/other labels.
    pub label_pos: Option<(f64, f64)>,
    /// Where the route leaves its source — on a node, sub-element or
    /// anchor boundary.
    pub from_point: (f64, f64),
    /// Where the route enters its target.
    pub to_point: (f64, f64),
    /// The source reference as written (`node.elem@anchor`).
    pub from_end: AdvanceEnd,
    /// The target reference as written.
    pub to_end: AdvanceEnd,
}

// ------------------------------------------------------------------
// Shape / EdgeKind string mapping (full parity)
// ------------------------------------------------------------------

fn parse_shape(s: &str) -> Result<Shape, AdvanceError> {
    match s {
        "rect" => Ok(Shape::Rect),
        "rounded" => Ok(Shape::Rounded),
        "stadium" => Ok(Shape::Stadium),
        "diamond" => Ok(Shape::Diamond),
        "circle" => Ok(Shape::Circle),
        "doublecircle" => Ok(Shape::DoubleCircle),
        "cylinder" => Ok(Shape::Cylinder),
        "subroutine" => Ok(Shape::Subroutine),
        "hexagon" => Ok(Shape::Hexagon),
        "parallelogram" => Ok(Shape::Parallelogram),
        "parallelogramalt" => Ok(Shape::ParallelogramAlt),
        "statestart" => Ok(Shape::StateStart),
        "stateend" => Ok(Shape::StateEnd),
        "forkbar" => Ok(Shape::ForkBar),
        other => Err(adv_err(format!(
            "unknown shape '{}', expected one of rect, rounded, stadium, diamond, circle, doublecircle, cylinder, subroutine, hexagon, parallelogram, parallelogramalt, statestart, stateend, forkbar",
            other
        ))),
    }
}

pub fn shape_name(s: Shape) -> &'static str {
    match s {
        Shape::Rect => "rect",
        Shape::Rounded => "rounded",
        Shape::Stadium => "stadium",
        Shape::Diamond => "diamond",
        Shape::Circle => "circle",
        Shape::DoubleCircle => "doublecircle",
        Shape::Cylinder => "cylinder",
        Shape::Subroutine => "subroutine",
        Shape::Hexagon => "hexagon",
        Shape::Parallelogram => "parallelogram",
        Shape::ParallelogramAlt => "parallelogramalt",
        Shape::StateStart => "statestart",
        Shape::StateEnd => "stateend",
        Shape::ForkBar => "forkbar",
    }
}

fn parse_edge_kind(s: &str) -> Result<EdgeKind, AdvanceError> {
    match s {
        "arrow" => Ok(EdgeKind::Arrow),
        "open" => Ok(EdgeKind::Open),
        "dotted" => Ok(EdgeKind::Dotted),
        "dottedopen" => Ok(EdgeKind::DottedOpen),
        "thick" => Ok(EdgeKind::Thick),
        "thickopen" => Ok(EdgeKind::ThickOpen),
        "invisible" => Ok(EdgeKind::Invisible),
        other => Err(adv_err(format!(
            "unknown edge kind '{}', expected one of arrow, open, dotted, dottedopen, thick, thickopen, invisible",
            other
        ))),
    }
}

pub fn edge_kind_name(k: EdgeKind) -> &'static str {
    match k {
        EdgeKind::Arrow => "arrow",
        EdgeKind::Open => "open",
        EdgeKind::Dotted => "dotted",
        EdgeKind::DottedOpen => "dottedopen",
        EdgeKind::Thick => "thick",
        EdgeKind::ThickOpen => "thickopen",
        EdgeKind::Invisible => "invisible",
    }
}

// ------------------------------------------------------------------
// Style / side parsing (JSON and text share these small helpers)
// ------------------------------------------------------------------

/// Parse a pixel quantity that may be a bare number or a `"Npx"` string.
fn px_number(v: &JsonValue) -> Result<f64, AdvanceError> {
    if let Some(n) = as_number(v) {
        if n.is_finite() && n > 0.0 {
            return Ok(n);
        }
        return Err(adv_err("stroke-width must be a positive finite number"));
    }
    if let Some(s) = as_str(v) {
        let cleaned = s.trim().trim_end_matches("px").trim();
        return cleaned
            .parse::<f64>()
            .map_err(|_| adv_err(format!("invalid stroke-width: '{}'", s)));
    }
    Err(adv_err("stroke-width must be a number or a 'Npx' string"))
}

/// Parse a node `style` object from JSON. Unknown keys are ignored.
fn parse_node_style_json(v: &JsonValue) -> Result<NodeStyle, AdvanceError> {
    let obj = as_object(v).ok_or_else(|| adv_err("node 'style' must be an object"))?;
    let mut st = NodeStyle::default();
    if let Some(s) = obj_get(obj, "fill").and_then(as_str) {
        st.fill = Some(s.to_string());
    }
    if let Some(s) = obj_get(obj, "stroke").and_then(as_str) {
        st.stroke = Some(s.to_string());
    }
    if let Some(s) = obj_get(obj, "color").and_then(as_str) {
        st.color = Some(s.to_string());
    }
    if let Some(v) = obj_get(obj, "stroke-width") {
        st.stroke_width = Some(px_number(v)?);
    }
    Ok(st)
}

/// Parse an edge `style` object from JSON. Unknown keys are ignored.
fn parse_edge_style_json(v: &JsonValue) -> Result<AdvanceEdgeStyle, AdvanceError> {
    let obj = as_object(v).ok_or_else(|| adv_err("edge 'style' must be an object"))?;
    let mut st = AdvanceEdgeStyle::default();
    if let Some(s) = obj_get(obj, "color").and_then(as_str) {
        st.color = Some(s.to_string());
    }
    if let Some(v) = obj_get(obj, "stroke-width") {
        st.stroke_width = Some(px_number(v)?);
    }
    if let Some(s) = obj_get(obj, "dash").and_then(as_str) {
        st.dash = Some(s.to_string());
    }
    if let Some(s) = obj_get(obj, "label-fill").and_then(as_str) {
        st.label_fill = Some(s.to_string());
    }
    Ok(st)
}

/// `{"id": "...", "side": "...", "offset": 0.5}`.
fn parse_anchor_json(v: &JsonValue, ctx: &str) -> Result<AdvanceAnchor, AdvanceError> {
    let obj = as_object(v).ok_or_else(|| adv_err(format!("{} anchor must be an object", ctx)))?;
    let id = obj_get(obj, "id")
        .and_then(as_str)
        .ok_or_else(|| adv_err(format!("{} anchor missing 'id'", ctx)))?
        .to_string();
    check_id("anchor", &id).map_err(adv_err)?;
    let side_s = obj_get(obj, "side")
        .and_then(as_str)
        .ok_or_else(|| adv_err(format!("{} anchor '{}' missing 'side'", ctx, id)))?;
    let side = parse_side(side_s).ok_or_else(|| {
        adv_err(format!(
            "{} anchor '{}': unknown side '{}'",
            ctx, id, side_s
        ))
    })?;
    let offset = match obj_get(obj, "offset") {
        None => DEFAULT_ANCHOR_OFFSET,
        Some(o) => as_number(o)
            .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
            .ok_or_else(|| {
                adv_err(format!(
                    "{} anchor '{}': offset must be a number in 0..=1",
                    ctx, id
                ))
            })?,
    };
    Ok(AdvanceAnchor { id, side, offset })
}

fn parse_layout_json(v: &JsonValue, ctx: &str) -> Result<ElementLayout, AdvanceError> {
    let w = as_str(v).ok_or_else(|| adv_err(format!("{} layout must be a string", ctx)))?;
    parse_layout_word(w).ok_or_else(|| {
        adv_err(format!(
            "{} layout: unknown '{}', expected column or row",
            ctx, w
        ))
    })
}

/// A sub-element object; nests through `"elements"`, capped like lanes.
fn parse_element_json(
    v: &JsonValue,
    ctx: &str,
    depth: usize,
) -> Result<AdvanceElement, AdvanceError> {
    if depth > MAX_NEST_DEPTH {
        return Err(adv_err(format!(
            "{}: sub-element nesting exceeds limit of {}",
            ctx, MAX_NEST_DEPTH
        )));
    }
    let obj =
        as_object(v).ok_or_else(|| adv_err(format!("{} sub-element must be an object", ctx)))?;
    let id = obj_get(obj, "id")
        .and_then(as_str)
        .ok_or_else(|| adv_err(format!("{} sub-element missing 'id'", ctx)))?
        .to_string();
    check_id("sub-element", &id).map_err(adv_err)?;
    let here = format!("{}.{}", ctx, id);
    let label = obj_get(obj, "label")
        .and_then(as_str)
        .unwrap_or(&id)
        .to_string();
    let mut anchors = Vec::new();
    if let Some(arr) = obj_get(obj, "anchors").and_then(as_array) {
        for a in arr {
            let a = parse_anchor_json(a, &here)?;
            if anchors.iter().any(|x: &AdvanceAnchor| x.id == a.id) {
                return Err(adv_err(format!("{}: duplicate anchor '{}'", here, a.id)));
            }
            anchors.push(a);
        }
    }
    let mut elements = Vec::new();
    if let Some(arr) = obj_get(obj, "elements").and_then(as_array) {
        for e in arr {
            let e = parse_element_json(e, &here, depth + 1)?;
            if elements.iter().any(|x: &AdvanceElement| x.id == e.id) {
                return Err(adv_err(format!(
                    "{}: duplicate sub-element '{}'",
                    here, e.id
                )));
            }
            elements.push(e);
        }
    }
    let layout = match obj_get(obj, "layout") {
        Some(v) => parse_layout_json(v, &here)?,
        None => ElementLayout::Column,
    };
    let style = match obj_get(obj, "style") {
        Some(v) => parse_node_style_json(v)?,
        None => NodeStyle::default(),
    };
    Ok(AdvanceElement {
        id,
        label,
        anchors,
        elements,
        layout,
        style,
    })
}

/// Parse a JSON `from_side`/`to_side` value. `"auto"` (or a missing key)
/// means automatic routing; anything else must be a known side keyword.
fn parse_side_json(v: &JsonValue) -> Result<Option<AdvanceSide>, AdvanceError> {
    let s = as_str(v).ok_or_else(|| adv_err("side must be a string"))?;
    match s {
        "auto" => Ok(None),
        _ => parse_side(s).map(Some).ok_or_else(|| {
            adv_err(format!(
                "unknown side '{}', expected one of left, right, top, bottom, auto",
                s
            ))
        }),
    }
}

// ------------------------------------------------------------------
// Parsing AdvanceDiagram
// ------------------------------------------------------------------

fn parse_lane_recursive(
    v: &JsonValue,
    depth: usize,
    lane_ids: &mut std::collections::HashSet<String>,
) -> Result<AdvanceLane, AdvanceError> {
    if depth > MAX_NEST_DEPTH {
        return Err(adv_err(format!(
            "lane nesting depth exceeds limit of {}",
            MAX_NEST_DEPTH
        )));
    }
    let obj = as_object(v).ok_or_else(|| adv_err("lane must be an object"))?;
    let id = obj_get(obj, "id")
        .and_then(as_str)
        .ok_or_else(|| adv_err("lane missing 'id'"))?
        .to_string();
    if !lane_ids.insert(id.clone()) {
        return Err(adv_err(format!("duplicate lane id '{}'", id)));
    }
    let title = obj_get(obj, "title")
        .and_then(as_str)
        .unwrap_or(&id)
        .to_string();

    let mut children = Vec::new();
    if let Some(c_arr) = obj_get(obj, "children").and_then(as_array) {
        for child_json in c_arr {
            children.push(parse_lane_recursive(child_json, depth + 1, lane_ids)?);
        }
    }
    Ok(AdvanceLane {
        id,
        title,
        children,
    })
}

impl AdvanceDiagram {
    pub fn parse(source: &str) -> Result<Self, AdvanceError> {
        let json = parse_json(source)?;
        let obj =
            as_object(&json).ok_or_else(|| adv_err("advance source must be a JSON object"))?;

        let title = obj_get(obj, "title")
            .and_then(as_str)
            .map(|s| s.to_string());
        let description = obj_get(obj, "description")
            .and_then(as_str)
            .map(|s| s.to_string());

        let direction = match obj_get(obj, "direction").and_then(as_str) {
            None | Some("vertical") => AdvanceDirection::Vertical,
            Some("horizontal") => AdvanceDirection::Horizontal,
            Some(other) => {
                return Err(adv_err(format!(
                    "unknown direction '{}', expected 'vertical' or 'horizontal'",
                    other
                )))
            }
        };

        let mut style = AdvanceStyle::default();
        if let Some(style_val) = obj_get(obj, "style") {
            let style_obj =
                as_object(style_val).ok_or_else(|| adv_err("'style' must be an object"))?;
            for (k, v) in style_obj {
                match k.as_str() {
                    "lane_fill" => {
                        style.lane_fill = as_str(v)
                            .ok_or_else(|| adv_err("style.lane_fill must be a string"))?
                            .to_string();
                    }
                    "lane_stroke" => {
                        style.lane_stroke = as_str(v)
                            .ok_or_else(|| adv_err("style.lane_stroke must be a string"))?
                            .to_string();
                    }
                    "edge_color" => {
                        style.edge_color = as_str(v)
                            .ok_or_else(|| adv_err("style.edge_color must be a string"))?
                            .to_string();
                    }
                    "text_color" => {
                        style.text_color = as_str(v)
                            .ok_or_else(|| adv_err("style.text_color must be a string"))?
                            .to_string();
                    }
                    "label_fill" => {
                        style.label_fill = as_str(v)
                            .ok_or_else(|| adv_err("style.label_fill must be a string"))?
                            .to_string();
                    }
                    _ => {} // Forward-compat
                }
            }
        }

        let mut config = AdvanceConfig::default();
        if let Some(cfg_val) = obj_get(obj, "config") {
            let cfg_obj =
                as_object(cfg_val).ok_or_else(|| adv_err("'config' must be an object"))?;
            for (k, v) in cfg_obj {
                match k.as_str() {
                    "margin" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.margin must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err(
                                "config.margin must be a non-negative finite number",
                            ));
                        }
                        config.margin = num;
                    }
                    "lane_gap" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.lane_gap must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err(
                                "config.lane_gap must be a non-negative finite number",
                            ));
                        }
                        config.lane_gap = num;
                    }
                    "node_gap_y" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.node_gap_y must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err(
                                "config.node_gap_y must be a non-negative finite number",
                            ));
                        }
                        config.node_gap_y = num;
                    }
                    "lane_pad_x" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.lane_pad_x must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err(
                                "config.lane_pad_x must be a non-negative finite number",
                            ));
                        }
                        config.lane_pad_x = num;
                    }
                    "lane_pad_y" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.lane_pad_y must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err(
                                "config.lane_pad_y must be a non-negative finite number",
                            ));
                        }
                        config.lane_pad_y = num;
                    }
                    "lane_title_h" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.lane_title_h must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err(
                                "config.lane_title_h must be a non-negative finite number",
                            ));
                        }
                        config.lane_title_h = num;
                    }
                    "order" => {
                        let ord_s =
                            as_str(v).ok_or_else(|| adv_err("config.order must be a string"))?;
                        config.order = match ord_s {
                            "declaration" => AdvanceOrder::Declaration,
                            "topology" => AdvanceOrder::Topology,
                            other => {
                                return Err(adv_err(format!(
                                "unknown config.order '{}', expected 'declaration' or 'topology'",
                                other
                            )))
                            }
                        };
                    }
                    _ => {}
                }
            }
        }

        let lanes_arr = obj_get(obj, "lanes")
            .and_then(as_array)
            .ok_or_else(|| adv_err("advance source must have a 'lanes' array"))?;
        let nodes_arr = obj_get(obj, "nodes")
            .and_then(as_array)
            .ok_or_else(|| adv_err("advance source must have a 'nodes' array"))?;
        let edges_arr = obj_get(obj, "edges").and_then(as_array).unwrap_or(&[]);

        let mut lanes = Vec::new();
        let mut lane_ids = std::collections::HashSet::new();
        for lane_json in lanes_arr {
            lanes.push(parse_lane_recursive(lane_json, 1, &mut lane_ids)?);
        }

        let mut nodes = Vec::new();
        let mut node_ids = std::collections::HashSet::new();
        let mut explicit_coords_count = 0;

        for (i, node_json) in nodes_arr.iter().enumerate() {
            let node_obj = as_object(node_json)
                .ok_or_else(|| adv_err(format!("nodes[{}] must be an object", i)))?;
            let id = obj_get(node_obj, "id")
                .and_then(as_str)
                .ok_or_else(|| adv_err(format!("nodes[{}] missing 'id'", i)))?
                .to_string();
            if !node_ids.insert(id.clone()) {
                return Err(adv_err(format!("duplicate node id '{}'", id)));
            }
            let lane = obj_get(node_obj, "lane")
                .and_then(as_str)
                .ok_or_else(|| adv_err(format!("nodes[{}] missing 'lane'", i)))?
                .to_string();
            if !lane_ids.contains(&lane) {
                return Err(adv_err(format!(
                    "nodes[{}] references unknown lane '{}'",
                    i, lane
                )));
            }
            let label = obj_get(node_obj, "label")
                .and_then(as_str)
                .unwrap_or(&id)
                .to_string();
            let shape = obj_get(node_obj, "shape")
                .and_then(as_str)
                .map(parse_shape)
                .transpose()?
                .unwrap_or(Shape::Rect);
            let style = match obj_get(node_obj, "style") {
                Some(v) => parse_node_style_json(v)?,
                None => NodeStyle::default(),
            };

            let x = obj_get(node_obj, "x").and_then(as_number);
            let y = obj_get(node_obj, "y").and_then(as_number);
            let w = obj_get(node_obj, "w").and_then(as_number);
            let h = obj_get(node_obj, "h").and_then(as_number);

            if let Some(vx) = x {
                if !vx.is_finite() {
                    return Err(adv_err(format!("nodes[{}].x must be a finite number", i)));
                }
            }
            if let Some(vy) = y {
                if !vy.is_finite() {
                    return Err(adv_err(format!("nodes[{}].y must be a finite number", i)));
                }
            }
            if let Some(vw) = w {
                if !vw.is_finite() || vw <= 0.0 {
                    return Err(adv_err(format!(
                        "nodes[{}].w must be a positive finite number",
                        i
                    )));
                }
            }
            if let Some(vh) = h {
                if !vh.is_finite() || vh <= 0.0 {
                    return Err(adv_err(format!(
                        "nodes[{}].h must be a positive finite number",
                        i
                    )));
                }
            }

            if x.is_some() || y.is_some() {
                if x.is_none() || y.is_none() {
                    return Err(adv_err(format!(
                        "nodes[{}] ('{}') has partial explicit coordinates (both x and y are required)",
                        i, id
                    )));
                }
                explicit_coords_count += 1;
            }

            check_id("node", &id).map_err(adv_err)?;
            let ctx = format!("nodes[{}] ('{}')", i, id);
            let mut anchors = Vec::new();
            if let Some(arr) = obj_get(node_obj, "anchors").and_then(as_array) {
                for a in arr {
                    let a = parse_anchor_json(a, &ctx)?;
                    if anchors.iter().any(|x: &AdvanceAnchor| x.id == a.id) {
                        return Err(adv_err(format!("{}: duplicate anchor '{}'", ctx, a.id)));
                    }
                    anchors.push(a);
                }
            }
            let mut elements = Vec::new();
            if let Some(arr) = obj_get(node_obj, "elements").and_then(as_array) {
                for e in arr {
                    let e = parse_element_json(e, &ctx, 1)?;
                    if elements.iter().any(|x: &AdvanceElement| x.id == e.id) {
                        return Err(adv_err(format!(
                            "{}: duplicate sub-element '{}'",
                            ctx, e.id
                        )));
                    }
                    elements.push(e);
                }
            }
            let layout = match obj_get(node_obj, "layout") {
                Some(v) => parse_layout_json(v, &ctx)?,
                None => ElementLayout::Column,
            };
            if !elements.is_empty() && !matches!(shape, Shape::Rect | Shape::Rounded) {
                return Err(adv_err(format!(
                    "{} is a {} — sub-elements need a rect or rounded node",
                    ctx,
                    shape_name(shape)
                )));
            }

            nodes.push(AdvanceNode {
                id,
                label,
                lane,
                shape,
                x,
                y,
                w,
                h,
                style,
                anchors,
                elements,
                layout,
            });
        }

        if explicit_coords_count > 0 && explicit_coords_count != nodes.len() {
            return Err(adv_err(
                "if any node specifies explicit coordinates (x, y), all nodes must specify them",
            ));
        }

        let mut edges = Vec::new();
        for (i, edge_json) in edges_arr.iter().enumerate() {
            let edge_obj = as_object(edge_json)
                .ok_or_else(|| adv_err(format!("edges[{}] must be an object", i)))?;
            let from_s = obj_get(edge_obj, "from")
                .and_then(as_str)
                .ok_or_else(|| adv_err(format!("edges[{}] missing 'from'", i)))?;
            let to_s = obj_get(edge_obj, "to")
                .and_then(as_str)
                .ok_or_else(|| adv_err(format!("edges[{}] missing 'to'", i)))?;
            // `from`/`to` accept the full reference grammar
            // (`node.elem@anchor`); the older `from_side`/`to_side`
            // keys still work and fill in a side when the string has none.
            let mut from_end =
                parse_end(from_s).map_err(|m| adv_err(format!("edges[{}]: {}", i, m)))?;
            let mut to_end =
                parse_end(to_s).map_err(|m| adv_err(format!("edges[{}]: {}", i, m)))?;
            let label = obj_get(edge_obj, "label")
                .and_then(as_str)
                .map(|s| s.to_string());
            let kind = obj_get(edge_obj, "kind")
                .and_then(as_str)
                .map(parse_edge_kind)
                .transpose()?
                .unwrap_or(EdgeKind::Arrow);
            let style = match obj_get(edge_obj, "style") {
                Some(v) => parse_edge_style_json(v)?,
                None => AdvanceEdgeStyle::default(),
            };
            if let Some(v) = obj_get(edge_obj, "from_side") {
                if let Some(side) = parse_side_json(v)? {
                    if from_end.at.is_none() {
                        from_end.at = Some(AnchorRef::Side(side));
                    }
                }
            }
            if let Some(v) = obj_get(edge_obj, "to_side") {
                if let Some(side) = parse_side_json(v)? {
                    if to_end.at.is_none() {
                        to_end.at = Some(AnchorRef::Side(side));
                    }
                }
            }
            let from_side = resolve_end_side(&nodes, &from_end)
                .map_err(|m| adv_err(format!("edges[{}]: {}", i, m)))?;
            let to_side = resolve_end_side(&nodes, &to_end)
                .map_err(|m| adv_err(format!("edges[{}]: {}", i, m)))?;
            edges.push(AdvanceEdge {
                from: from_end.node.clone(),
                to: to_end.node.clone(),
                label,
                kind,
                style,
                from_side,
                to_side,
                from_end,
                to_end,
            });
        }

        Ok(AdvanceDiagram {
            title,
            description,
            direction,
            style,
            config,
            lanes,
            nodes,
            edges,
        })
    }

    /// Parse a concise text-based swimlane notation into an [`AdvanceDiagram`].
    ///
    /// Syntax:
    /// ```text
    /// swimlane [horizontal]
    /// title "My Title"
    /// config margin 30
    /// lane l1 "Sales" {
    ///   a([Start])
    ///   b[Prepare Order]
    ///   lane sub "Sub-lane" {
    ///     c[Ship Package]
    ///   }
    /// }
    /// lane l2 "Fulfillment"
    ///   d[Receive]
    ///
    /// a --> b
    /// b -->|done| c
    /// c:right --> d:top
    /// style a fill:#fee,stroke:#900
    /// classDef warn fill:#fff3cd,stroke:#f0ad4e
    /// class b warn
    /// style c-->d color:#b00
    /// ```
    ///
    /// Nodes support `<br/>` line breaks and a trailing `id::class` shorthand.
    pub fn parse_text(source: &str) -> Result<AdvanceDiagram, AdvanceError> {
        let mut title = None;
        let mut description = None;
        let mut direction = AdvanceDirection::Vertical;
        let mut config = AdvanceConfig::default();
        let mut nodes: Vec<AdvanceNode> = Vec::new();
        let mut edges: Vec<AdvanceEdge> = Vec::new();

        // Lane declarations are collected as flat records during the scan and
        // assembled into a nested tree at the end (`depth` = brace level).
        let mut lane_recs: Vec<(String, String, usize)> = Vec::new();
        let mut lane_ids = std::collections::HashSet::new();
        let mut lane_stack: Vec<usize> = Vec::new();
        let mut lane_open_lines: Vec<usize> = Vec::new();
        let mut current_lane: Option<String> = None;

        let mut node_ids = std::collections::HashSet::new();

        // A node whose `{` block is open, and the sub-elements opened
        // inside it (innermost last). A node block cannot contain a lane
        // block, so `}` closes an element, then the node, then a lane.
        let mut cur_node: Option<AdvanceNode> = None;
        let mut open_elems: Vec<AdvanceElement> = Vec::new();
        let mut node_open_line = 0usize;

        // Deferred styling (classDef -> class assign -> explicit style line).
        let mut class_defs: std::collections::HashMap<String, NodeStyle> =
            std::collections::HashMap::new();
        let mut assigns: Vec<(String, String)> = Vec::new();
        let mut node_styles: Vec<(String, NodeStyle)> = Vec::new();
        let mut edge_styles: Vec<(usize, AdvanceEnd, AdvanceEnd, AdvanceEdgeStyle)> = Vec::new();

        // `decl { a; b }` on one line means the same as the block form.
        // It is expanded here so the loop sees one statement per entry,
        // each still carrying its original line number for errors.
        let mut stmts: Vec<(usize, String)> = Vec::new();
        for (line_no, raw_line) in source.lines().enumerate() {
            let line = raw_line.trim();
            // Comments and free-text directives are never blocks, whatever
            // they end with; expanding them used to break the parse.
            if line.starts_with('#')
                || line.starts_with("%%")
                || line.starts_with("//")
                || (is_directive_line(line) && !line.starts_with("lane "))
            {
                stmts.push((line_no, line.to_string()));
                continue;
            }
            expand_inline(line_no, line, 0, &mut stmts)
                .map_err(|m| text_err(source, line_no, None, m))?;
        }

        for (line_no, line) in &stmts {
            let line_no = *line_no;
            let line = line.as_str();
            if line.is_empty()
                || line.starts_with("%%")
                || line.starts_with("//")
                || line.starts_with('#')
            {
                continue;
            }

            // Closing brace: the innermost open block.
            if line == "}" {
                if let Some(node) = cur_node.as_mut() {
                    if let Some(elem) = open_elems.pop() {
                        // A sub-element block closed: attach to its parent.
                        match open_elems.last_mut() {
                            Some(parent) => parent.elements.push(elem),
                            None => node.elements.push(elem),
                        }
                    } else if let Some(node) = cur_node.take() {
                        // Compartments are rectangles; inside any other
                        // outline they poke through it and their corners
                        // fall outside the shape's hit area.
                        if !node.elements.is_empty()
                            && !matches!(node.shape, Shape::Rect | Shape::Rounded)
                        {
                            return Err(text_err(
                                source,
                                node_open_line,
                                None,
                                format!(
                                    "node '{}' is a {} — sub-elements need a rect or rounded node",
                                    node.id,
                                    shape_name(node.shape)
                                ),
                            ));
                        }
                        nodes.push(node);
                    }
                } else if lane_stack.pop().is_some() {
                    lane_open_lines.pop();
                    // After a top-level close the stack is empty and there is
                    // no lane scope anymore — later nodes must open a new lane.
                    current_lane = lane_stack.last().map(|&parent| lane_recs[parent].0.clone());
                } else {
                    return Err(text_err(source, line_no, Some(0), "unbalanced '}'"));
                }
                continue;
            }

            // Inside a node block only anchors, layout and sub-elements
            // are legal; anything else is a mistake worth naming.
            if let Some(node) = cur_node.as_mut() {
                let host_kind = if open_elems.is_empty() {
                    "node"
                } else {
                    "sub-element"
                };
                if let Some(rest) = line.strip_prefix("anchor ") {
                    let a =
                        parse_anchor_line(rest).map_err(|m| text_err(source, line_no, None, m))?;
                    let anchors = match open_elems.last_mut() {
                        Some(e) => &mut e.anchors,
                        None => &mut node.anchors,
                    };
                    if anchors.iter().any(|x| x.id == a.id) {
                        return Err(text_err(
                            source,
                            line_no,
                            None,
                            format!("duplicate anchor '{}'", a.id),
                        ));
                    }
                    anchors.push(a);
                    continue;
                }
                if let Some(rest) = line.strip_prefix("layout ") {
                    let l = parse_layout_word(rest.trim()).ok_or_else(|| {
                        text_err(
                            source,
                            line_no,
                            None,
                            format!("unknown layout '{}', expected column or row", rest.trim()),
                        )
                    })?;
                    match open_elems.last_mut() {
                        Some(e) => e.layout = l,
                        None => node.layout = l,
                    }
                    continue;
                }
                if is_edge_line(line) || is_directive_line(line) {
                    return Err(text_err(
                        source,
                        line_no,
                        None,
                        format!("'{}' is not allowed inside a {} block — only anchor, layout and sub-elements", line, host_kind),
                    ));
                }
                let (decl, opens) = match line.strip_suffix('{') {
                    Some(d) => (d.trim_end(), true),
                    None => (line, false),
                };
                let (id, label, _shape) = parse_text_node_shorthand(decl).ok_or_else(|| {
                    text_err(
                        source,
                        line_no,
                        None,
                        format!("invalid sub-element declaration '{}'", line),
                    )
                })?;
                check_id("sub-element", &id).map_err(|m| text_err(source, line_no, None, m))?;
                let siblings = match open_elems.last() {
                    Some(e) => &e.elements,
                    None => &node.elements,
                };
                if siblings.iter().any(|e| e.id == id) {
                    return Err(text_err(
                        source,
                        line_no,
                        None,
                        format!("duplicate sub-element '{}'", id),
                    ));
                }
                let elem = AdvanceElement {
                    id,
                    label: normalize_breaks(&label),
                    anchors: Vec::new(),
                    elements: Vec::new(),
                    layout: ElementLayout::Column,
                    style: NodeStyle::default(),
                };
                if opens {
                    if open_elems.len() >= MAX_NEST_DEPTH {
                        return Err(text_err(
                            source,
                            line_no,
                            None,
                            format!("sub-element nesting exceeds limit of {}", MAX_NEST_DEPTH),
                        ));
                    }
                    open_elems.push(elem);
                } else {
                    match open_elems.last_mut() {
                        Some(e) => e.elements.push(elem),
                        None => node.elements.push(elem),
                    }
                }
                continue;
            }

            // Header
            if line.starts_with("swimlane") {
                let rest = line.trim_start_matches("swimlane").trim();
                if rest.eq_ignore_ascii_case("horizontal") || rest.eq_ignore_ascii_case("lr") {
                    direction = AdvanceDirection::Horizontal;
                }
                continue;
            }

            // Title / Desc
            if line.starts_with("title ") {
                title = Some(
                    line.trim_start_matches("title ")
                        .trim()
                        .trim_matches('"')
                        .to_string(),
                );
                continue;
            }
            if line.starts_with("desc ") || line.starts_with("description ") {
                let d_str = if line.starts_with("desc ") {
                    line.trim_start_matches("desc ")
                } else {
                    line.trim_start_matches("description ")
                };
                description = Some(d_str.trim().trim_matches('"').to_string());
                continue;
            }

            // config directive: config <key> <value>
            if line.starts_with("config ") {
                parse_text_config(line, &mut config)
                    .map_err(|m| text_err(source, line_no, None, m))?;
                continue;
            }

            // Styling directives — must run before the edge branch because an
            // edge-style target (`style a-->b ...`) contains an edge operator.
            if line.starts_with("classDef ") {
                parse_class_def(line, &mut class_defs)
                    .map_err(|m| text_err(source, line_no, None, m))?;
                continue;
            }
            if line.starts_with("class ") {
                parse_class_assign(line, &mut assigns)
                    .map_err(|m| text_err(source, line_no, None, m))?;
                continue;
            }
            if line.starts_with("style ") {
                parse_style_line(line, line_no, &mut node_styles, &mut edge_styles)
                    .map_err(|m| text_err(source, line_no, None, m))?;
                continue;
            }

            // Lane declaration: lane <id> ["title"] [{  ...  }]
            if line.starts_with("lane ") {
                let rest = line.trim_start_matches("lane ").trim();
                let mut parts = rest.splitn(2, char::is_whitespace);
                let id = parts.next().unwrap_or("").trim().to_string();
                if id.is_empty() {
                    return Err(text_err(
                        source,
                        line_no,
                        Some(5),
                        "lane ID cannot be empty",
                    ));
                }
                let mut title_rest = parts.next().unwrap_or("").trim().to_string();
                let has_brace = title_rest.ends_with('{');
                if has_brace {
                    title_rest.pop();
                    title_rest = title_rest.trim().to_string();
                }
                let lane_title = if title_rest.is_empty() {
                    id.clone()
                } else {
                    normalize_breaks(title_rest.trim().trim_matches('"'))
                };
                if lane_ids.contains(&id) {
                    return Err(text_err(
                        source,
                        line_no,
                        Some(5),
                        format!("duplicate lane ID '{}'", id),
                    ));
                }
                lane_ids.insert(id.clone());
                let depth = lane_stack.len();
                lane_recs.push((id.clone(), lane_title, depth));
                if has_brace {
                    lane_stack.push(lane_recs.len() - 1);
                    lane_open_lines.push(line_no);
                }
                current_lane = Some(id);
                continue;
            }

            // Edges: A --> B, A -->|label| B, A -.-> B, etc. A separator
            // inside a label is text, so a line without one at bracket
            // depth 0 falls through to the node-declaration branch.
            if let Some((idx, sep)) = find_edge_sep(line) {
                let (from_str, rest) = (&line[..idx], &line[idx + sep.len()..]);

                let from_end =
                    parse_end(from_str).map_err(|m| text_err(source, line_no, None, m))?;
                let kind = match sep {
                    "-->" => EdgeKind::Arrow,
                    "-.->" => EdgeKind::Dotted,
                    "==>" => EdgeKind::Thick,
                    "---" => EdgeKind::Open,
                    _ => EdgeKind::Arrow,
                };

                let (label, to_str) = if rest.trim_start().starts_with('|') {
                    let trim_rest = rest.trim_start()[1..].trim_start();
                    if let Some(pipe_end) = trim_rest.find('|') {
                        let lbl = &trim_rest[..pipe_end];
                        let to_id = trim_rest[pipe_end + 1..].trim();
                        (Some(normalize_breaks(lbl.trim())), to_id.to_string())
                    } else {
                        (None, rest.trim().to_string())
                    }
                } else {
                    (None, rest.trim().to_string())
                };
                let to_end = parse_end(&to_str).map_err(|m| text_err(source, line_no, None, m))?;

                let from_side = resolve_end_side(&nodes, &from_end)
                    .map_err(|m| text_err(source, line_no, None, m))?;
                let to_side = resolve_end_side(&nodes, &to_end)
                    .map_err(|m| text_err(source, line_no, None, m))?;

                edges.push(AdvanceEdge {
                    from: from_end.node.clone(),
                    to: to_end.node.clone(),
                    label,
                    kind,
                    style: AdvanceEdgeStyle::default(),
                    from_side,
                    to_side,
                    from_end,
                    to_end,
                });
                continue;
            }

            // Node declaration inside the innermost lane.
            let lane = current_lane.as_ref().ok_or_else(|| {
                text_err(
                    source,
                    line_no,
                    None,
                    format!("node '{}' declared outside of any lane", line),
                )
            })?;

            // A trailing `{` opens a node block (anchors, layout,
            // sub-elements). It is stripped before the `::class`
            // shorthand is read, so `a[A]::hot {` works too.
            let (decl_line, opens_block) = match line.strip_suffix('{') {
                Some(d) => (d.trim_end(), true),
                None => (line, false),
            };
            let (decl, class) = split_node_class_shorthand(decl_line);
            let (id, label, shape) = parse_text_node_shorthand(decl).ok_or_else(|| {
                text_err(
                    source,
                    line_no,
                    None,
                    format!("invalid node declaration '{}'", line),
                )
            })?;
            check_id("node", &id).map_err(|m| text_err(source, line_no, None, m))?;
            if let Some(class_name) = class {
                assigns.push((id.clone(), class_name.to_string()));
            }
            let label = normalize_breaks(&label);

            if node_ids.contains(&id) {
                return Err(text_err(
                    source,
                    line_no,
                    None,
                    format!("duplicate node ID '{}'", id),
                ));
            }
            node_ids.insert(id.clone());

            let node = AdvanceNode {
                id,
                label,
                lane: lane.clone(),
                shape,
                x: None,
                y: None,
                w: None,
                h: None,
                style: NodeStyle::default(),
                anchors: Vec::new(),
                elements: Vec::new(),
                layout: ElementLayout::Column,
            };
            if opens_block {
                cur_node = Some(node);
                node_open_line = line_no;
            } else {
                nodes.push(node);
            }
        }

        if let Some(node) = &cur_node {
            return Err(text_err(
                source,
                node_open_line,
                None,
                format!("node '{}' block is never closed", node.id),
            ));
        }

        if !lane_stack.is_empty() {
            let idx = lane_stack[0];
            let open_line = lane_open_lines[0];
            return Err(text_err(
                source,
                open_line,
                None,
                format!("lane '{}' block is never closed", lane_recs[idx].0),
            ));
        }
        if lane_recs.is_empty() {
            return Err(text_err(
                source,
                0,
                None,
                "diagram must define at least one lane",
            ));
        }

        let lanes = assemble_lane_tree(&lane_recs);

        // Deferred styling resolution: classDef -> class assign -> style line.
        let mut resolved_class: std::collections::HashMap<String, NodeStyle> =
            std::collections::HashMap::new();
        for (name, st) in &class_defs {
            resolved_class.insert(name.clone(), st.clone());
        }
        let mut by_node: std::collections::HashMap<String, NodeStyle> =
            std::collections::HashMap::new();
        for (id, class_name) in &assigns {
            if let Some(st) = resolved_class.get(class_name) {
                by_node.entry(id.clone()).or_default().apply_over(st);
            }
        }
        for (id, st) in &node_styles {
            by_node.entry(id.clone()).or_default().apply_over(st);
        }
        for n in &mut nodes {
            if let Some(st) = by_node.get(&n.id) {
                n.style.apply_over(st);
            }
        }
        for (line_no, from, to, st) in &edge_styles {
            // A target naming a sub-element or anchor picks exactly that
            // edge; a plain `a-->b` (with or without sides) still styles
            // every edge between the two nodes, as it always has.
            let exact = from.is_terminal() || to.is_terminal();
            let mut hits = 0usize;
            for e in edges.iter_mut() {
                let matched = if exact {
                    e.from_end == *from && e.to_end == *to
                } else {
                    e.from == from.node && e.to == to.node
                };
                if matched {
                    e.style.apply_over(st);
                    hits += 1;
                }
            }
            if hits == 0 {
                return Err(text_err(
                    source,
                    *line_no,
                    None,
                    format!(
                        "style target '{}-->{}' matches no edge",
                        from.to_ref(),
                        to.to_ref()
                    ),
                ));
            }
        }

        Ok(AdvanceDiagram {
            title,
            description,
            direction,
            style: AdvanceStyle::default(),
            config,
            lanes,
            nodes,
            edges,
        })
    }
}

// ------------------------------------------------------------------
// Text DSL helpers
// ------------------------------------------------------------------

/// Strip a trailing `id::class` shorthand from a node declaration line.
/// Only applied when the suffix contains no shape brackets, so `a[Foo::Bar]`
/// (a `::` inside a label) is left untouched.
fn split_node_class_shorthand(line: &str) -> (&str, Option<&str>) {
    if let Some(sep) = line.rfind("::") {
        let after = &line[sep + 2..];
        if !after.is_empty()
            && !after.contains('[')
            && !after.contains(']')
            && !after.contains('(')
            && !after.contains(')')
            && !after.contains('{')
            && !after.contains('}')
        {
            return (line[..sep].trim_end(), Some(after.trim()));
        }
    }
    (line, None)
}

/// Split an edge endpoint into `(id, side)`. A side suffix is recognized
/// only when it is one of the known keywords (`:left`, `:right`, `:top`,
/// `:bottom`); anything else (including colons inside ids) is kept whole.
/// Split style properties on commas at paren depth 0 (so CSS function
/// values like `fill:rgb(255,0,0)` survive as one property).
fn split_props(s: &str) -> Result<Vec<&str>, String> {
    let mut items: Vec<&str> = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(format!("unbalanced '(' in style properties: '{}'", s));
    }
    items.push(&s[start..]);
    Ok(items)
}

/// Parse a stroke-width value from text-DSL properties (`2`, `2px`), with
/// the same finiteness/positivity guard as the JSON path (`px_number`).
fn parse_stroke_width_prop(v: &str) -> Result<f64, String> {
    let n: f64 = v
        .trim_end_matches("px")
        .trim()
        .parse()
        .map_err(|_| format!("invalid stroke-width: '{}'", v))?;
    if n.is_finite() && n > 0.0 {
        Ok(n)
    } else {
        Err(format!(
            "stroke-width must be a positive finite number, got '{}'",
            v
        ))
    }
}

/// Parse node style properties (`k:v,k:v`) mirroring [`crate::parser::parse_props`].
fn parse_node_style_props(s: &str) -> Result<NodeStyle, String> {
    let mut st = NodeStyle::default();
    for item in split_props(s)? {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let Some((k, v)) = item.split_once(':') else {
            return Err(format!("expected 'property:value', got '{}'", item));
        };
        let v = v.trim();
        match k.trim() {
            "fill" => st.fill = Some(v.to_string()),
            "stroke" => st.stroke = Some(v.to_string()),
            "color" => st.color = Some(v.to_string()),
            "stroke-width" => {
                st.stroke_width = Some(parse_stroke_width_prop(v)?);
            }
            _ => {}
        }
    }
    Ok(st)
}

/// Parse edge style properties (`k:v,k:v`).
fn parse_edge_style_props(s: &str) -> Result<AdvanceEdgeStyle, String> {
    let mut st = AdvanceEdgeStyle::default();
    for item in split_props(s)? {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let Some((k, v)) = item.split_once(':') else {
            return Err(format!("expected 'property:value', got '{}'", item));
        };
        let v = v.trim();
        match k.trim() {
            "color" => st.color = Some(v.to_string()),
            "stroke-width" => {
                st.stroke_width = Some(parse_stroke_width_prop(v)?);
            }
            "dash" => st.dash = Some(v.to_string()),
            "label-fill" => st.label_fill = Some(v.to_string()),
            _ => {}
        }
    }
    Ok(st)
}

fn parse_class_def(
    line: &str,
    class_defs: &mut std::collections::HashMap<String, NodeStyle>,
) -> Result<(), String> {
    let rest = line.trim_start_matches("classDef").trim();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("").trim().to_string();
    let props = parts.next().unwrap_or("").trim();
    if name.is_empty() {
        return Err("expected a class name after 'classDef'".to_string());
    }
    let st = parse_node_style_props(props)?;
    class_defs.insert(name, st);
    Ok(())
}

fn parse_class_assign(line: &str, assigns: &mut Vec<(String, String)>) -> Result<(), String> {
    let rest = line.trim_start_matches("class").trim();
    let mut parts = rest.rsplitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("").trim().to_string();
    let ids = parts.next().unwrap_or("").trim();
    if name.is_empty() || ids.is_empty() {
        return Err("expected 'class id1,id2 className'".to_string());
    }
    for id in ids.split(',') {
        let id = id.trim();
        if !id.is_empty() {
            assigns.push((id.to_string(), name.clone()));
        }
    }
    Ok(())
}

/// Parse `style <target> <props>` where target is a node id or an edge
/// literal `from-->to` (ports allowed: `a:right-->b:top`).
fn parse_style_line(
    line: &str,
    line_no: usize,
    node_styles: &mut Vec<(String, NodeStyle)>,
    edge_styles: &mut Vec<(usize, AdvanceEnd, AdvanceEnd, AdvanceEdgeStyle)>,
) -> Result<(), String> {
    let rest = line.trim_start_matches("style").trim();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let target = parts.next().unwrap_or("").trim();
    let props = parts.next().unwrap_or("").trim();
    if target.is_empty() {
        return Err("expected a node id or 'from-->to' after 'style'".to_string());
    }
    if let Some(idx) = target.find("-->") {
        let from = parse_end(&target[..idx])
            .map_err(|m| format!("invalid edge target '{}': {}", target, m))?;
        let to = parse_end(&target[idx + 3..])
            .map_err(|m| format!("invalid edge target '{}': {}", target, m))?;
        let st = parse_edge_style_props(props)?;
        edge_styles.push((line_no, from, to, st));
    } else {
        let st = parse_node_style_props(props)?;
        node_styles.push((target.to_string(), st));
    }
    Ok(())
}

fn parse_text_config(line: &str, config: &mut AdvanceConfig) -> Result<(), String> {
    let rest = line.trim_start_matches("config").trim();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let key = parts.next().unwrap_or("").trim();
    let val = parts.next().unwrap_or("").trim();
    if key.is_empty() {
        return Err("expected a config key after 'config'".to_string());
    }
    let num: Option<f64> = val.parse().ok();
    let set_num = |target: &mut f64| -> Result<(), String> {
        match num {
            Some(n) if n.is_finite() && n >= 0.0 => {
                *target = n;
                Ok(())
            }
            _ => Err(format!(
                "config.{} must be a non-negative finite number",
                key
            )),
        }
    };
    match key {
        "margin" => set_num(&mut config.margin),
        "lane_gap" => set_num(&mut config.lane_gap),
        "node_gap_y" => set_num(&mut config.node_gap_y),
        "lane_pad_x" => set_num(&mut config.lane_pad_x),
        "lane_pad_y" => set_num(&mut config.lane_pad_y),
        "lane_title_h" => set_num(&mut config.lane_title_h),
        "order" => {
            config.order = match val {
                "declaration" => AdvanceOrder::Declaration,
                "topology" => AdvanceOrder::Topology,
                _ => {
                    return Err(format!(
                        "unknown config.order '{}', expected 'declaration' or 'topology'",
                        val
                    ))
                }
            };
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Assemble the flat lane records into a nested [`AdvanceLane`] tree.
/// A record's parent is the nearest preceding record with a strictly
/// smaller depth; the tree is built by reverse-moving each lane into its
/// parent, so no clones are needed.
fn assemble_lane_tree(recs: &[(String, String, usize)]) -> Vec<AdvanceLane> {
    let mut parent_of: Vec<Option<usize>> = vec![None; recs.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, rec) in recs.iter().enumerate() {
        while let Some(&top) = stack.last() {
            if recs[top].2 >= rec.2 {
                stack.pop();
            } else {
                break;
            }
        }
        parent_of[i] = stack.last().copied();
        stack.push(i);
    }

    let mut lanes: Vec<AdvanceLane> = recs
        .iter()
        .map(|(id, title, _)| AdvanceLane {
            id: id.clone(),
            title: title.clone(),
            children: Vec::new(),
        })
        .collect();
    let mut roots: Vec<AdvanceLane> = Vec::new();
    for i in (0..recs.len()).rev() {
        let empty = AdvanceLane {
            id: String::new(),
            title: String::new(),
            children: Vec::new(),
        };
        match parent_of[i] {
            Some(p) => {
                let lane = std::mem::replace(&mut lanes[i], empty);
                // Reverse iteration visits later-declared siblings first, so
                // inserting at the front keeps declaration order in children.
                lanes[p].children.insert(0, lane);
            }
            None => {
                let lane = std::mem::replace(&mut lanes[i], empty);
                roots.push(lane);
            }
        }
    }
    roots.reverse();
    roots
}

fn parse_text_node_shorthand(s: &str) -> Option<(String, String, Shape)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if let Some(idx) = s.find("([") {
        if s.ends_with("])") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 2..s.len() - 2].trim().to_string();
            return Some((id, label, Shape::Stadium));
        }
    }
    if let Some(idx) = s.find("(((") {
        if s.ends_with(")))") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 3..s.len() - 3].trim().to_string();
            return Some((id, label, Shape::DoubleCircle));
        }
    }
    if let Some(idx) = s.find("((") {
        if s.ends_with("))") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 2..s.len() - 2].trim().to_string();
            return Some((id, label, Shape::Circle));
        }
    }
    if let Some(idx) = s.find("{{") {
        if s.ends_with("}}") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 2..s.len() - 2].trim().to_string();
            return Some((id, label, Shape::Hexagon));
        }
    }
    if let Some(idx) = s.find("{") {
        if s.ends_with("}") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 1..s.len() - 1].trim().to_string();
            return Some((id, label, Shape::Diamond));
        }
    }
    if let Some(idx) = s.find("[(") {
        if s.ends_with(")]") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 2..s.len() - 2].trim().to_string();
            return Some((id, label, Shape::Cylinder));
        }
    }
    if let Some(idx) = s.find("[[") {
        if s.ends_with("]]") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 2..s.len() - 2].trim().to_string();
            return Some((id, label, Shape::Subroutine));
        }
    }
    if let Some(idx) = s.find("[/") {
        if s.ends_with("/]") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 2..s.len() - 2].trim().to_string();
            return Some((id, label, Shape::Parallelogram));
        }
    }
    if let Some(idx) = s.find("[\\") {
        if s.ends_with("\\]") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 2..s.len() - 2].trim().to_string();
            return Some((id, label, Shape::ParallelogramAlt));
        }
    }
    if let Some(idx) = s.find("[") {
        if s.ends_with("]") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 1..s.len() - 1].trim().to_string();
            return Some((id, label, Shape::Rect));
        }
    }
    if let Some(idx) = s.find("(") {
        if s.ends_with(")") {
            let id = s[..idx].trim().to_string();
            let label = s[idx + 1..s.len() - 1].trim().to_string();
            return Some((id, label, Shape::Rounded));
        }
    }

    // Bare ID (defaults to Rect with ID as label)
    Some((s.to_string(), s.to_string(), Shape::Rect))
}

// ------------------------------------------------------------------
// Serializer (Round-trip)
// ------------------------------------------------------------------

fn lane_to_json_rec(l: &AdvanceLane, s: &mut String) {
    s.push_str("{\"id\":");
    s.push_str(&escape_json_str(&l.id));
    s.push_str(",\"title\":");
    s.push_str(&escape_json_str(&l.title));
    if !l.children.is_empty() {
        s.push_str(",\"children\":[");
        for (i, c) in l.children.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            lane_to_json_rec(c, s);
        }
        s.push(']');
    }
    s.push('}');
}

fn node_style_to_json(ns: &NodeStyle, s: &mut String) {
    let mut first = true;
    if let Some(fill) = &ns.fill {
        s.push_str(&format!("\"fill\":{}", escape_json_str(fill)));
        first = false;
    }
    if let Some(stroke) = &ns.stroke {
        if !first {
            s.push(',');
        }
        s.push_str(&format!("\"stroke\":{}", escape_json_str(stroke)));
        first = false;
    }
    if let Some(color) = &ns.color {
        if !first {
            s.push(',');
        }
        s.push_str(&format!("\"color\":{}", escape_json_str(color)));
        first = false;
    }
    if let Some(w) = ns.stroke_width {
        if !first {
            s.push(',');
        }
        s.push_str(&format!("\"stroke-width\":{:.1}", w));
    }
}

fn edge_style_to_json(es: &AdvanceEdgeStyle, s: &mut String) {
    let mut first = true;
    if let Some(color) = &es.color {
        s.push_str(&format!("\"color\":{}", escape_json_str(color)));
        first = false;
    }
    if let Some(w) = es.stroke_width {
        if !first {
            s.push(',');
        }
        s.push_str(&format!("\"stroke-width\":{:.1}", w));
        first = false;
    }
    if let Some(dash) = &es.dash {
        if !first {
            s.push(',');
        }
        s.push_str(&format!("\"dash\":{}", escape_json_str(dash)));
        first = false;
    }
    if let Some(label_fill) = &es.label_fill {
        if !first {
            s.push(',');
        }
        s.push_str(&format!("\"label-fill\":{}", escape_json_str(label_fill)));
    }
}

/// Serialize an [`AdvanceDiagram`] back to a valid JSON string.
pub fn to_json(d: &AdvanceDiagram) -> String {
    let mut s = String::new();
    s.push('{');

    if let Some(t) = &d.title {
        s.push_str("\"title\":");
        s.push_str(&escape_json_str(t));
        s.push(',');
    }
    if let Some(desc) = &d.description {
        s.push_str("\"description\":");
        s.push_str(&escape_json_str(desc));
        s.push(',');
    }
    if d.direction == AdvanceDirection::Horizontal {
        s.push_str("\"direction\":\"horizontal\",");
    }

    // Style
    s.push_str("\"style\":{");
    s.push_str(&format!(
        "\"lane_fill\":{},\"lane_stroke\":{},\"edge_color\":{},\"text_color\":{},\"label_fill\":{}",
        escape_json_str(&d.style.lane_fill),
        escape_json_str(&d.style.lane_stroke),
        escape_json_str(&d.style.edge_color),
        escape_json_str(&d.style.text_color),
        escape_json_str(&d.style.label_fill),
    ));
    s.push_str("},");

    // Config
    s.push_str("\"config\":{");
    s.push_str(&format!(
        "\"margin\":{:.1},\"lane_gap\":{:.1},\"node_gap_y\":{:.1},\"lane_pad_x\":{:.1},\"lane_pad_y\":{:.1},\"lane_title_h\":{:.1},\"order\":\"{}\"",
        d.config.margin,
        d.config.lane_gap,
        d.config.node_gap_y,
        d.config.lane_pad_x,
        d.config.lane_pad_y,
        d.config.lane_title_h,
        match d.config.order {
            AdvanceOrder::Declaration => "declaration",
            AdvanceOrder::Topology => "topology",
        }
    ));
    s.push_str("},");

    // Lanes
    s.push_str("\"lanes\":[");
    for (i, l) in d.lanes.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        lane_to_json_rec(l, &mut s);
    }
    s.push_str("],");

    // Nodes
    s.push_str("\"nodes\":[");
    for (i, n) in d.nodes.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"id\":{},\"label\":{},\"lane\":{},\"shape\":\"{}\"",
            escape_json_str(&n.id),
            escape_json_str(&n.label),
            escape_json_str(&n.lane),
            shape_name(n.shape)
        ));
        if let (Some(x), Some(y)) = (n.x, n.y) {
            s.push_str(&format!(",\"x\":{:.1},\"y\":{:.1}", x, y));
        }
        if let Some(w) = n.w {
            s.push_str(&format!(",\"w\":{:.1}", w));
        }
        if let Some(h) = n.h {
            s.push_str(&format!(",\"h\":{:.1}", h));
        }
        if n.style != NodeStyle::default() {
            s.push_str(",\"style\":{");
            node_style_to_json(&n.style, &mut s);
            s.push('}');
        }
        if n.layout != ElementLayout::Column {
            s.push_str(&format!(",\"layout\":\"{}\"", n.layout.name()));
        }
        if !n.anchors.is_empty() {
            s.push_str(",\"anchors\":[");
            for (j, a) in n.anchors.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                anchor_to_json(a, &mut s);
            }
            s.push(']');
        }
        if !n.elements.is_empty() {
            s.push_str(",\"elements\":[");
            for (j, e) in n.elements.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                element_to_json(e, &mut s);
            }
            s.push(']');
        }
        s.push('}');
    }
    s.push_str("],");

    // Edges
    s.push_str("\"edges\":[");
    for (i, e) in d.edges.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"from\":{},\"to\":{},\"kind\":\"{}\"",
            escape_json_str(&end_ref_for_json(&e.from_end)),
            escape_json_str(&end_ref_for_json(&e.to_end)),
            edge_kind_name(e.kind)
        ));
        if let Some(lbl) = &e.label {
            s.push_str(",\"label\":");
            s.push_str(&escape_json_str(lbl));
        }
        if e.style != AdvanceEdgeStyle::default() {
            s.push_str(",\"style\":{");
            edge_style_to_json(&e.style, &mut s);
            s.push('}');
        }
        if let Some(side) = e.from_side {
            s.push_str(&format!(",\"from_side\":\"{}\"", side.name()));
        }
        if let Some(side) = e.to_side {
            s.push_str(&format!(",\"to_side\":\"{}\"", side.name()));
        }
        s.push('}');
    }
    s.push(']');

    s.push('}');
    s
}

fn anchor_to_json(a: &AdvanceAnchor, s: &mut String) {
    s.push_str(&format!(
        "{{\"id\":{},\"side\":\"{}\",\"offset\":{}}}",
        escape_json_str(&a.id),
        a.side.name(),
        a.offset
    ));
}

fn element_to_json(e: &AdvanceElement, s: &mut String) {
    s.push_str(&format!(
        "{{\"id\":{},\"label\":{}",
        escape_json_str(&e.id),
        escape_json_str(&e.label)
    ));
    if e.layout != ElementLayout::Column {
        s.push_str(&format!(",\"layout\":\"{}\"", e.layout.name()));
    }
    if !e.anchors.is_empty() {
        s.push_str(",\"anchors\":[");
        for (i, a) in e.anchors.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            anchor_to_json(a, s);
        }
        s.push(']');
    }
    if !e.elements.is_empty() {
        s.push_str(",\"elements\":[");
        for (i, c) in e.elements.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            element_to_json(c, s);
        }
        s.push(']');
    }
    if e.style != NodeStyle::default() {
        s.push_str(",\"style\":{");
        node_style_to_json(&e.style, s);
        s.push('}');
    }
    s.push('}');
}

/// The `from`/`to` string for JSON: node, path and named anchor. A
/// plain side is NOT folded in here — it still travels as
/// `from_side`/`to_side`, so existing output stays byte-identical.
fn end_ref_for_json(end: &AdvanceEnd) -> String {
    let mut r = end.node.clone();
    for seg in &end.path {
        r.push('.');
        r.push_str(seg);
    }
    if let Some(AnchorRef::Named(a)) = &end.at {
        r.push('@');
        r.push_str(a);
    }
    r
}

/// Serialize an [`AdvanceScene`] to a JSON geometry representation.
pub fn scene_to_json(sc: &AdvanceScene) -> String {
    let mut s = String::new();
    s.push_str("{\"width\":");
    s.push_str(&format!("{:.1}", sc.width));
    s.push_str(",\"height\":");
    s.push_str(&format!("{:.1}", sc.height));

    if let Some(t) = &sc.title {
        s.push_str(",\"title\":");
        s.push_str(&escape_json_str(t));
    }
    if let Some(desc) = &sc.description {
        s.push_str(",\"description\":");
        s.push_str(&escape_json_str(desc));
    }
    if sc.direction == AdvanceDirection::Horizontal {
        s.push_str(",\"direction\":\"horizontal\"");
    }
    s.push_str(",\"crossings\":");
    s.push_str(&sc.crossings.to_string());

    s.push_str(",\"lanes\":[");
    for (i, lane) in sc.lanes.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"id\":{},\"title\":{},\"x\":{:.1},\"y\":{:.1},\"w\":{:.1},\"h\":{:.1}}}",
            escape_json_str(&lane.id),
            escape_json_str(&lane.title),
            lane.x,
            lane.y,
            lane.w,
            lane.h
        ));
    }
    s.push(']');

    s.push_str(",\"nodes\":[");
    for (i, node) in sc.nodes.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"id\":{},\"label\":{},\"lane\":{},\"x\":{:.1},\"y\":{:.1},\"w\":{:.1},\"h\":{:.1},\"shape\":\"{}\"",
            escape_json_str(&node.id),
            escape_json_str(&node.label),
            escape_json_str(&node.lane),
            node.x,
            node.y,
            node.w,
            node.h,
            shape_name(node.shape)
        ));
        if node.style != NodeStyle::default() {
            s.push_str(",\"style\":{");
            node_style_to_json(&node.style, &mut s);
            s.push('}');
        }
        if !node.elements.is_empty() {
            s.push_str(",\"elements\":[");
            for (j, el) in node.elements.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                s.push_str(&format!(
                    "{{\"id\":{},\"label\":{},\"path\":[{}],\"parent\":{},\"x\":{:.1},\"y\":{:.1},\"w\":{:.1},\"h\":{:.1}",
                    escape_json_str(&el.id),
                    escape_json_str(&el.label),
                    el.path.iter().map(|p| escape_json_str(p)).collect::<Vec<_>>().join(","),
                    el.parent.map_or("null".to_string(), |p| p.to_string()),
                    el.x,
                    el.y,
                    el.w,
                    el.h
                ));
                if el.style != NodeStyle::default() {
                    s.push_str(",\"style\":{");
                    node_style_to_json(&el.style, &mut s);
                    s.push('}');
                }
                s.push('}');
            }
            s.push(']');
        }
        if !node.anchors.is_empty() {
            s.push_str(",\"anchors\":[");
            for (j, a) in node.anchors.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                s.push_str(&format!(
                    "{{\"id\":{},\"side\":\"{}\",\"element\":{},\"x\":{:.1},\"y\":{:.1}}}",
                    escape_json_str(&a.id),
                    a.side.name(),
                    a.element.map_or("null".to_string(), |e| e.to_string()),
                    a.x,
                    a.y
                ));
            }
            s.push(']');
        }
        s.push('}');
    }
    s.push(']');

    s.push_str(",\"edges\":[");
    for (i, edge) in sc.edges.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"from\":{},\"to\":{},\"kind\":\"{}\"",
            escape_json_str(&edge.from),
            escape_json_str(&edge.to),
            edge_kind_name(edge.kind)
        ));
        if let Some(lbl) = &edge.label {
            s.push_str(",\"label\":");
            s.push_str(&escape_json_str(lbl));
        }
        s.push_str(",\"points\":[");
        for (j, p) in edge.points.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            s.push_str(&format!("[{:.1},{:.1}]", p.0, p.1));
        }
        s.push(']');
        if edge.style != AdvanceEdgeStyle::default() {
            s.push_str(",\"style\":{");
            edge_style_to_json(&edge.style, &mut s);
            s.push('}');
        }
        if let Some(side) = edge.from_side {
            s.push_str(&format!(",\"from_side\":\"{}\"", side.name()));
        }
        if let Some(side) = edge.to_side {
            s.push_str(&format!(",\"to_side\":\"{}\"", side.name()));
        }
        if let Some((lx, ly)) = edge.label_pos {
            s.push_str(&format!(",\"label_pos\":[{:.1},{:.1}]", lx, ly));
        }
        s.push_str(&format!(
            ",\"from_point\":[{:.1},{:.1}],\"to_point\":[{:.1},{:.1}]",
            edge.from_point.0, edge.from_point.1, edge.to_point.0, edge.to_point.1
        ));
        if edge.from_end.is_terminal() {
            s.push_str(&format!(
                ",\"from_end\":{}",
                escape_json_str(&edge.from_end.to_ref())
            ));
        }
        if edge.to_end.is_terminal() {
            s.push_str(&format!(
                ",\"to_end\":{}",
                escape_json_str(&edge.to_end.to_ref())
            ));
        }
        s.push('}');
    }
    s.push(']');

    s.push('}');
    s
}

// ------------------------------------------------------------------
// Layout & Geometry
// ------------------------------------------------------------------

/// Inset of sub-elements from their host's border.
const ELEM_PAD: f64 = 8.0;
/// Gap between sibling sub-elements.
const ELEM_GAP: f64 = 6.0;

/// Natural size of one sub-element: its label, plus room for its own
/// children below the label band.
fn measure_element(e: &AdvanceElement) -> (f64, f64) {
    let tw = e.label.split('\n').map(text_width).fold(0.0, f64::max);
    let band_h = label_band_h(&e.label);
    let base_w = (tw + 2.0 * PAD_X).max(MIN_W);
    if e.elements.is_empty() {
        return (base_w, band_h);
    }
    let (cw, ch) = measure_children(&e.elements, e.layout);
    (base_w.max(cw + 2.0 * ELEM_PAD), band_h + ch + ELEM_PAD)
}

/// Size of a stack of children under `layout`, without the host's
/// own padding.
fn measure_children(elems: &[AdvanceElement], layout: ElementLayout) -> (f64, f64) {
    let sizes: Vec<(f64, f64)> = elems.iter().map(measure_element).collect();
    let gaps = ELEM_GAP * (elems.len().saturating_sub(1)) as f64;
    match layout {
        ElementLayout::Column => (
            sizes.iter().map(|s| s.0).fold(0.0, f64::max),
            sizes.iter().map(|s| s.1).sum::<f64>() + gaps,
        ),
        ElementLayout::Row => (
            sizes.iter().map(|s| s.0).sum::<f64>() + gaps,
            sizes.iter().map(|s| s.1).fold(0.0, f64::max),
        ),
    }
}

/// A point on a rectangle's boundary: `t` runs 0..=1 along `side`.
fn rect_side_point(cx: f64, cy: f64, w: f64, h: f64, side: AdvanceSide, t: f64) -> (f64, f64) {
    match side {
        AdvanceSide::Left => (cx - w / 2.0, cy - h / 2.0 + t * h),
        AdvanceSide::Right => (cx + w / 2.0, cy - h / 2.0 + t * h),
        AdvanceSide::Top => (cx - w / 2.0 + t * w, cy - h / 2.0),
        AdvanceSide::Bottom => (cx - w / 2.0 + t * w, cy + h / 2.0),
    }
}

/// Place `elems` inside a host whose content area starts below its
/// label band. Children stretch across the cross axis so compartments
/// line up; along the main axis each keeps its measured size.
#[allow(clippy::too_many_arguments)]
fn place_elements(
    elems: &[AdvanceElement],
    layout: ElementLayout,
    left: f64,
    top: f64,
    avail_w: f64,
    avail_h: f64,
    path: &[String],
    parent: Option<usize>,
    out: &mut Vec<AdvanceSceneElement>,
    anchors: &mut Vec<AdvanceSceneAnchor>,
) {
    let sizes: Vec<(f64, f64)> = elems.iter().map(measure_element).collect();
    let mut cursor = 0.0;
    for (i, e) in elems.iter().enumerate() {
        let (mw, mh) = sizes[i];
        let (x, y, w, h) = match layout {
            ElementLayout::Column => {
                let r = (left, top + cursor, avail_w, mh);
                cursor += mh + ELEM_GAP;
                r
            }
            ElementLayout::Row => {
                let r = (left + cursor, top, mw, avail_h);
                cursor += mw + ELEM_GAP;
                r
            }
        };
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let mut here = path.to_vec();
        here.push(e.id.clone());
        let idx = out.len();
        out.push(AdvanceSceneElement {
            id: e.id.clone(),
            label: e.label.clone(),
            path: here.clone(),
            parent,
            x: cx,
            y: cy,
            w,
            h,
            style: e.style.clone(),
        });
        for a in &e.anchors {
            let (ax, ay) = rect_side_point(cx, cy, w, h, a.side, a.offset);
            anchors.push(AdvanceSceneAnchor {
                id: a.id.clone(),
                side: a.side,
                element: Some(idx),
                x: ax,
                y: ay,
            });
        }
        if !e.elements.is_empty() {
            let band_h = label_band_h(&e.label);
            place_elements(
                &e.elements,
                e.layout,
                x + ELEM_PAD,
                y + band_h,
                w - 2.0 * ELEM_PAD,
                h - band_h - ELEM_PAD,
                &here,
                Some(idx),
                out,
                anchors,
            );
        }
    }
}

/// Build a scene node at centre `(x, y)` with the given box, placing
/// its sub-elements and resolving every anchor to an absolute point.
/// Every construction of an [`AdvanceSceneNode`] from the model goes
/// through here so the flat element/anchor lists are always in step.
fn scene_node(n: &AdvanceNode, x: f64, y: f64, w: f64, h: f64) -> AdvanceSceneNode {
    let mut elements = Vec::new();
    let mut anchors: Vec<AdvanceSceneAnchor> = n
        .anchors
        .iter()
        .map(|a| {
            let (ax, ay) = rect_side_point(x, y, w, h, a.side, a.offset);
            AdvanceSceneAnchor {
                id: a.id.clone(),
                side: a.side,
                element: None,
                x: ax,
                y: ay,
            }
        })
        .collect();
    if !n.elements.is_empty() {
        let band_h = label_band_h(&n.label);
        place_elements(
            &n.elements,
            n.layout,
            x - w / 2.0 + ELEM_PAD,
            y - h / 2.0 + band_h,
            w - 2.0 * ELEM_PAD,
            h - band_h - ELEM_PAD,
            &[],
            None,
            &mut elements,
            &mut anchors,
        );
    }
    AdvanceSceneNode {
        id: n.id.clone(),
        label: n.label.clone(),
        lane: n.lane.clone(),
        x,
        y,
        w,
        h,
        shape: n.shape,
        style: n.style.clone(),
        elements,
        anchors,
    }
}

fn node_size(node: &AdvanceNode) -> (f64, f64) {
    let tw = node.label.split('\n').map(text_width).fold(0.0, f64::max);
    let extra = (node.label.split('\n').count().saturating_sub(1)) as f64 * LINE_H;
    let base_h = BASE_H + extra;
    let (calc_w, calc_h) = match node.shape {
        Shape::Rect | Shape::Rounded => ((tw + 2.0 * PAD_X).max(MIN_W), base_h),
        Shape::Stadium => ((tw + 2.0 * PAD_X + 12.0).max(MIN_W + 12.0), base_h),
        Shape::Subroutine | Shape::Parallelogram | Shape::ParallelogramAlt => {
            ((tw + 2.0 * PAD_X + 24.0).max(MIN_W + 24.0), base_h)
        }
        Shape::Hexagon => ((tw + 2.0 * PAD_X + 28.0).max(MIN_W + 28.0), base_h),
        Shape::Cylinder => ((tw + 2.0 * PAD_X).max(MIN_W), base_h + 16.0),
        Shape::Diamond => (((tw + 24.0) * 1.6).max(80.0), base_h * 1.7),
        Shape::Circle => {
            let d = (tw + 24.0).max(52.0).max(base_h);
            (d, d)
        }
        Shape::DoubleCircle => {
            let d = (tw + 32.0).max(60.0).max(base_h + 8.0);
            (d, d)
        }
        Shape::StateStart | Shape::StateEnd | Shape::ForkBar => (base_h, base_h),
    };
    // Sub-elements sit below the label band and the node grows to fit
    // them, on top of whatever the shape itself needs. An explicit
    // `w`/`h` is clamped to that minimum — a smaller box would place
    // compartments outside the node with negative heights.
    let (grown_w, grown_h) = if node.elements.is_empty() {
        (0.0, 0.0)
    } else {
        let (cw, ch) = measure_children(&node.elements, node.layout);
        (cw + 2.0 * ELEM_PAD, base_h + ch + ELEM_PAD)
    };
    (
        node.w.map_or(calc_w.max(grown_w), |w| w.max(grown_w)),
        node.h.map_or(calc_h.max(grown_h), |h| h.max(grown_h)),
    )
}

fn collect_all_lanes<'a>(lanes: &'a [AdvanceLane], out: &mut Vec<&'a AdvanceLane>) {
    for l in lanes {
        out.push(l);
        collect_all_lanes(&l.children, out);
    }
}

/// Map every lane id — nested ones included — to the index of the
/// TOP-LEVEL lane it lives under. [`lane_index_map`] is a flat
/// depth-first index over ALL lanes, which does not line up with an
/// enumeration of `d.lanes`: with `[A[A1, A2], B]` the flat indices are
/// A=0, A1=1, A2=2, B=3, so nodes in `A1` were counted toward `B` and
/// `B` itself was sized as if empty.
fn top_level_lane_map(d: &AdvanceDiagram) -> std::collections::HashMap<String, usize> {
    let mut out = std::collections::HashMap::new();
    for (i, lane) in d.lanes.iter().enumerate() {
        let mut stack = vec![lane];
        while let Some(l) = stack.pop() {
            out.insert(l.id.clone(), i);
            stack.extend(l.children.iter());
        }
    }
    out
}

fn lane_index_map(d: &AdvanceDiagram) -> std::collections::HashMap<String, usize> {
    let mut flat = Vec::new();
    collect_all_lanes(&d.lanes, &mut flat);
    flat.into_iter()
        .enumerate()
        .map(|(i, l)| (l.id.clone(), i))
        .collect()
}

fn node_index_map(d: &AdvanceDiagram) -> std::collections::HashMap<String, usize> {
    d.nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect()
}

// ------------------------------------------------------------------
// Node ordering within lane (Topology Sort + Barycenter)
// ------------------------------------------------------------------

fn order_lane_nodes(
    lane_node_indices: &[usize],
    d: &AdvanceDiagram,
    cfg: &AdvanceConfig,
) -> Vec<usize> {
    if cfg.order == AdvanceOrder::Declaration || lane_node_indices.len() <= 1 {
        return lane_node_indices.to_vec();
    }

    let n_count = lane_node_indices.len();
    let mut local_id_to_idx = std::collections::HashMap::with_capacity(n_count);
    for (pos, &global_i) in lane_node_indices.iter().enumerate() {
        local_id_to_idx.insert(d.nodes[global_i].id.as_str(), pos);
    }

    // Build intra-lane adj & in-degrees
    let mut adj = vec![Vec::new(); n_count];
    let mut in_degree = vec![0usize; n_count];
    for e in &d.edges {
        if let (Some(&u), Some(&v)) = (
            local_id_to_idx.get(e.from.as_str()),
            local_id_to_idx.get(e.to.as_str()),
        ) {
            if u != v {
                adj[u].push(v);
                in_degree[v] += 1;
            }
        }
    }

    // Kahn's algorithm with stable tie-breaking
    let mut ready = std::collections::BTreeSet::new();
    for i in 0..n_count {
        if in_degree[i] == 0 {
            ready.insert(i);
        }
    }

    let mut ordered_local = Vec::with_capacity(n_count);
    while let Some(&u) = ready.iter().next() {
        ready.remove(&u);
        ordered_local.push(u);
        for &v in &adj[u] {
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                ready.insert(v);
            }
        }
    }

    // Nodes involved in cycles remain
    if ordered_local.len() < n_count {
        for i in 0..n_count {
            if !ordered_local.contains(&i) {
                ordered_local.push(i);
            }
        }
    }

    // Barycenter refinement pass for cross-lane edges
    let global_node_idx = node_index_map(d);
    let mut barycenters: Vec<(usize, f64)> = Vec::with_capacity(n_count);
    for &u in &ordered_local {
        let global_u = lane_node_indices[u];
        let u_id = &d.nodes[global_u].id;
        let mut sum = 0.0;
        let mut count = 0usize;
        for e in &d.edges {
            if &e.from == u_id && !local_id_to_idx.contains_key(e.to.as_str()) {
                sum += global_node_idx[&e.to] as f64;
                count += 1;
            } else if &e.to == u_id && !local_id_to_idx.contains_key(e.from.as_str()) {
                sum += global_node_idx[&e.from] as f64;
                count += 1;
            }
        }
        let b_val = if count > 0 {
            sum / count as f64
        } else {
            f64::INFINITY
        };
        barycenters.push((u, b_val));
    }

    // Stable sort by barycenter if valid
    barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    barycenters
        .into_iter()
        .map(|(u, _)| lane_node_indices[u])
        .collect()
}

// ------------------------------------------------------------------
// Routing tuning constants & geometric helpers
// ------------------------------------------------------------------

const SELF_LOOP_DX: f64 = 28.0;
const SELF_LOOP_DROP: f64 = 12.0;
const PARALLEL_FAN: f64 = 16.0;
/// How far outside a node an edge travels before it may turn.
const PORT_LEAD: f64 = 18.0;

/// How far apart a loop's two ends are pushed when they share a side,
/// so the loop encloses something rather than doubling back on itself.
const SELF_LOOP_SPREAD: f64 = 11.0;

fn node_rect(n: &AdvanceSceneNode) -> (f64, f64, f64, f64) {
    (
        n.x - n.w / 2.0,
        n.y - n.h / 2.0,
        n.x + n.w / 2.0,
        n.y + n.h / 2.0,
    )
}

fn seg_crosses_rect(p0: (f64, f64), p1: (f64, f64), rect: (f64, f64, f64, f64)) -> bool {
    const EPS: f64 = 1e-9;
    let (l, t, r, b) = rect;
    let (x1, y1) = p0;
    let (x2, y2) = p1;
    if (x1 - x2).abs() < EPS && (y1 - y2).abs() < EPS {
        return false;
    }
    if (x1 - x2).abs() < EPS {
        if x1 <= l + EPS || x1 >= r - EPS {
            return false;
        }
        let (lo, hi) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
        hi > t + EPS && lo < b - EPS
    } else if (y1 - y2).abs() < EPS {
        if y1 <= t + EPS || y1 >= b - EPS {
            return false;
        }
        let (lo, hi) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
        hi > l + EPS && lo < r - EPS
    } else {
        false
    }
}

// ------------------------------------------------------------------
// Channel-grid router
// ------------------------------------------------------------------

/// How far outside a node the grid runs its channels.
const GRID_CLEAR: f64 = 12.0;
/// What a corner costs, in pixels of path length. High enough that the
/// router prefers a longer straight run over a shorter staircase.
const BEND_COST: f64 = 40.0;
/// Above this many lattice vertices the search is abandoned for a
/// direct route. A diagram that large is unreadable long before the
/// router is its problem.
const GRID_BUDGET: usize = 250_000;

/// Above this many lattice vertices an edge settles for its preferred
/// sides instead of searching the others. Choosing sides is a
/// refinement; on a diagram big enough for the lattice to pass this it
/// costs far more than the crossing it saves.
const SEARCH_BUDGET: usize = 2_000;

/// The negotiation re-routes every edge once per pass, so it is priced
/// like the first pass again and is skipped above this lattice size.
const NEGOTIATION_BUDGET: usize = 2_000;

/// How many rip-up passes to try before accepting what is left.
const NEGOTIATION_PASSES: usize = 4;

/// What each pass multiplies the price of a crossing by. Once it
/// dominates length and bends, a route that can avoid a crossing will.
const CROSS_ESCALATION: f64 = 4.0;
/// What crossing an already-routed edge costs. Well above a bend, so a
/// route takes a longer way round rather than cut across a neighbour —
/// but finite, so it still crosses when there is no alternative.
const CROSS_COST: f64 = 260.0;

/// Two lines meeting in a T, and two lines running along each other:
/// not crossings, but they read like one, so they are worth avoiding.
const TOUCH_COST: f64 = 90.0;

/// Overlap is charged by the pixel, because unlike a crossing it is not
/// an event at a point. Per pixel is also the only unit on which the
/// search and the ranking agree: the search sees a run of short lattice
/// steps where the ranking sees one long segment.
const SHARE_RATE: f64 = 3.0;

/// Which way the path was travelling when it reached a vertex; a bend
/// is counted when this changes.
/// A lattice of candidate channels built from the diagram's own
/// geometry: every node's sides with clearance and its centre lines,
/// the lane boundaries, and the endpoints of the route being drawn.
///
/// Routing on the lattice is orthogonal by construction, and a segment
/// counts only when it clears every node — which makes "no line through
/// a box" a property of the router instead of a check repeated per edge
/// kind.
/// How two axis-aligned segments interfere.
///
/// A true crossing is the expensive case, but two lines that meet in a
/// T, or run along each other, read as a crossing to whoever looks at
/// the diagram, so they count too — for less, because they are less
/// wrong. Corner touching corner is how neighbouring routes legitimately
/// share a lattice vertex and is free.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Conflict {
    None,
    /// One line ends on the other.
    Touch,
    /// They run along each other, over this many pixels.
    Share(f64),
    /// One passes over the other.
    Cross,
}

impl Conflict {
    /// `cross` is what a crossing costs, which the negotiation raises
    /// between passes until the crossings that can move, move.
    fn cost(self, cross: f64) -> f64 {
        match self {
            Conflict::None => 0.0,
            Conflict::Touch => TOUCH_COST,
            Conflict::Share(len) => SHARE_RATE * len,
            Conflict::Cross => cross,
        }
    }
}

fn seg_conflict(a: ((f64, f64), (f64, f64)), b: ((f64, f64), (f64, f64))) -> Conflict {
    const EPS: f64 = 1e-6;
    let vert = |s: ((f64, f64), (f64, f64))| (s.0 .0 - s.1 .0).abs() < EPS;
    let (av, bv) = (vert(a), vert(b));
    if av == bv {
        let (ac, bc) = if av {
            (a.0 .0, b.0 .0)
        } else {
            (a.0 .1, b.0 .1)
        };
        if (ac - bc).abs() > EPS {
            return Conflict::None; // parallel but on different lines
        }
        let span = |s: ((f64, f64), (f64, f64))| {
            if av {
                (s.0 .1.min(s.1 .1), s.0 .1.max(s.1 .1))
            } else {
                (s.0 .0.min(s.1 .0), s.0 .0.max(s.1 .0))
            }
        };
        let ((a0, a1), (b0, b1)) = (span(a), span(b));
        let overlap = a1.min(b1) - a0.max(b0);
        return if overlap > EPS {
            Conflict::Share(overlap)
        } else {
            Conflict::None
        };
    }
    let (v, h) = if av { (a, b) } else { (b, a) };
    let x = v.0 .0;
    let (vy0, vy1) = (v.0 .1.min(v.1 .1), v.0 .1.max(v.1 .1));
    let y = h.0 .1;
    let (hx0, hx1) = (h.0 .0.min(h.1 .0), h.0 .0.max(h.1 .0));
    if x < hx0 - EPS || x > hx1 + EPS || y < vy0 - EPS || y > vy1 + EPS {
        return Conflict::None;
    }
    match (
        x > hx0 + EPS && x < hx1 - EPS,
        y > vy0 + EPS && y < vy1 - EPS,
    ) {
        (true, true) => Conflict::Cross,
        (false, false) => Conflict::None,
        _ => Conflict::Touch,
    }
}

/// How many pairs of segments from different edges cross, and which
/// edges take part in at least one of them. The second half is the
/// negotiation's work list: those are the edges with something to gain
/// from being routed again.
fn crossings_of(routes: &[Vec<(f64, f64)>], live: &[bool]) -> (usize, Vec<usize>) {
    let mut count = 0;
    let mut involved = vec![false; routes.len()];
    for (i, a) in routes.iter().enumerate() {
        if !live[i] {
            continue;
        }
        for (j, b) in routes.iter().enumerate().skip(i + 1) {
            if !live[j] {
                continue;
            }
            for p in a.windows(2) {
                for q in b.windows(2) {
                    if seg_conflict((p[0], p[1]), (q[0], q[1])) == Conflict::Cross {
                        count += 1;
                        involved[i] = true;
                        involved[j] = true;
                    }
                }
            }
        }
    }
    let edges = involved
        .iter()
        .enumerate()
        .filter(|(_, on)| **on)
        .map(|(i, _)| i)
        .collect();
    (count, edges)
}

/// How many pairs of edges cross, counted once per pair of segments.
/// Only true crossings: edges that share an endpoint, meet in a T, or
/// run along each other are not counted, because none of them is an
/// edge passing over another.
fn count_crossings(edges: &[AdvanceSceneEdge]) -> usize {
    let mut n = 0;
    for (i, a) in edges.iter().enumerate() {
        if a.kind == EdgeKind::Invisible {
            continue;
        }
        for b in edges.iter().skip(i + 1) {
            if b.kind == EdgeKind::Invisible {
                continue;
            }
            for p in a.points.windows(2) {
                for q in b.points.windows(2) {
                    if seg_conflict((p[0], p[1]), (q[0], q[1])) == Conflict::Cross {
                        n += 1;
                    }
                }
            }
        }
    }
    n
}

/// What a finished route costs: its length, its corners, and every
/// conflict with the routes drawn before it. This is the yardstick an
/// auto-sided edge uses to choose which sides to leave and arrive on.
fn route_cost(
    points: &[(f64, f64)],
    routes: &[Vec<(f64, f64)>],
    live: &[bool],
    skip: usize,
    cross: f64,
) -> f64 {
    let length: f64 = points
        .windows(2)
        .map(|w| (w[0].0 - w[1].0).abs() + (w[0].1 - w[1].1).abs())
        .sum();
    length
        + BEND_COST * points.len().saturating_sub(2) as f64
        + route_conflicts(points, routes, live, skip, cross)
}

/// The conflict half of [`route_cost`] on its own, so a route that
/// already touches nothing can skip the search over the other sides.
fn route_conflicts(
    points: &[(f64, f64)],
    routes: &[Vec<(f64, f64)>],
    live: &[bool],
    skip: usize,
    cross: f64,
) -> f64 {
    points
        .windows(2)
        .map(|w| {
            routes
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != skip && live[*i])
                .flat_map(|(_, o)| o.windows(2))
                .map(|o| seg_conflict((w[0], w[1]), (o[0], o[1])).cost(cross))
                .sum::<f64>()
        })
        .sum()
}

struct RouteGrid {
    xs: Vec<f64>,
    ys: Vec<f64>,
    /// One entry per lattice step, so the search asks a question that
    /// was already answered instead of testing every node and every
    /// segment drawn so far on each of its millions of relaxations.
    /// `h[j * (xs - 1) + i]` is the step from `(i, j)` to `(i + 1, j)`;
    /// `v[j * xs + i]` the step from `(i, j)` to `(i, j + 1)`.
    blocked_h: Vec<bool>,
    blocked_v: Vec<bool>,
    /// What crossing the edges already routed would cost on that step.
    /// Priced, not forbidden — some diagrams have no planar routing, and
    /// a missing edge is worse than a crossed one.
    cost_h: Vec<f64>,
    cost_v: Vec<f64>,
    /// Kept for the fallback, which has no lattice to consult.
    rects: Vec<(f64, f64, f64, f64)>,
    /// The search's two working arrays, kept between calls: one edge
    /// runs the search up to sixteen times and every diagram runs it
    /// once per edge, so reallocating them each time was pure waste.
    scratch: std::cell::RefCell<(Vec<f64>, Vec<usize>)>,
}

fn sorted_unique(mut v: Vec<f64>) -> Vec<f64> {
    v.retain(|x| x.is_finite());
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    v
}

impl RouteGrid {
    fn build(
        nodes: &[AdvanceSceneNode],
        lanes: &[AdvanceSceneLane],
        extra: &[(f64, f64)],
        drawn: &[Vec<(f64, f64)>],
    ) -> Self {
        let mut xs = Vec::with_capacity(nodes.len() * 3 + lanes.len() * 2 + extra.len());
        let mut ys = Vec::with_capacity(xs.capacity());
        for n in nodes {
            let (l, t, r, b) = node_rect(n);
            xs.extend([l - GRID_CLEAR, r + GRID_CLEAR, n.x]);
            ys.extend([t - GRID_CLEAR, b + GRID_CLEAR, n.y]);
        }
        for l in lanes {
            xs.extend([l.x + GRID_CLEAR, l.x + l.w - GRID_CLEAR]);
            ys.extend([l.y + GRID_CLEAR, l.y + l.h - GRID_CLEAR]);
        }
        for (x, y) in extra {
            xs.push(*x);
            ys.push(*y);
        }
        let xs = sorted_unique(xs);
        let ys = sorted_unique(ys);
        let (nx, ny) = (xs.len(), ys.len());
        let mut g = RouteGrid {
            blocked_h: vec![false; nx.saturating_sub(1) * ny],
            blocked_v: vec![false; nx * ny.saturating_sub(1)],
            cost_h: vec![0.0; nx.saturating_sub(1) * ny],
            cost_v: vec![0.0; nx * ny.saturating_sub(1)],
            rects: nodes.iter().map(node_rect).collect(),
            scratch: std::cell::RefCell::new((Vec::new(), Vec::new())),
            xs,
            ys,
        };
        // Every node blocks, the route's own endpoints included: the
        // lattice carries only the part between the two leaders, which
        // are already outside both. The lead from a sub-element to its
        // node boundary is prepended around the lattice path, so it
        // never needs an exemption — granting one let a route wander
        // through the node it was leaving.
        for n in nodes {
            g.block(node_rect(n));
        }
        for seg in drawn.iter().flat_map(|pts| pts.windows(2)) {
            g.price((seg[0], seg[1]), CROSS_COST, 1.0);
        }
        g
    }

    /// Index range of axis values inside the open interval `(lo, hi)`.
    fn inside(axis: &[f64], lo: f64, hi: f64) -> std::ops::Range<usize> {
        axis.partition_point(|v| *v <= lo)..axis.partition_point(|v| *v < hi)
    }

    /// Index range of steps whose span meets `[lo, hi]`.
    fn spanning(axis: &[f64], lo: f64, hi: f64) -> std::ops::Range<usize> {
        let first = axis.partition_point(|v| *v < lo).saturating_sub(1);
        let last = axis.partition_point(|v| *v <= hi);
        first..last.min(axis.len().saturating_sub(1))
    }

    fn block(&mut self, rect: (f64, f64, f64, f64)) {
        let (l, t, r, b) = rect;
        let nx = self.xs.len();
        for j in Self::inside(&self.ys, t, b) {
            for i in Self::spanning(&self.xs, l, r) {
                if seg_crosses_rect((self.xs[i], self.ys[j]), (self.xs[i + 1], self.ys[j]), rect) {
                    self.blocked_h[j * (nx - 1) + i] = true;
                }
            }
        }
        for i in Self::inside(&self.xs, l, r) {
            for j in Self::spanning(&self.ys, t, b) {
                if seg_crosses_rect((self.xs[i], self.ys[j]), (self.xs[i], self.ys[j + 1]), rect) {
                    self.blocked_v[j * nx + i] = true;
                }
            }
        }
    }

    /// Add (`sign` 1.0) or take back (`sign` -1.0) one segment's
    /// contribution to what the lattice steps around it cost. Taking a
    /// route back out is what lets the negotiation re-route it against
    /// everyone else instead of only against the edges before it.
    fn price(&mut self, seg: ((f64, f64), (f64, f64)), cross: f64, sign: f64) {
        let (x0, x1) = (seg.0 .0.min(seg.1 .0), seg.0 .0.max(seg.1 .0));
        let (y0, y1) = (seg.0 .1.min(seg.1 .1), seg.0 .1.max(seg.1 .1));
        let nx = self.xs.len();
        for j in Self::inside(&self.ys, y0 - 1e-6, y1 + 1e-6) {
            for i in Self::spanning(&self.xs, x0, x1) {
                let step = ((self.xs[i], self.ys[j]), (self.xs[i + 1], self.ys[j]));
                self.cost_h[j * (nx - 1) + i] += sign * seg_conflict(step, seg).cost(cross);
            }
        }
        for i in Self::inside(&self.xs, x0 - 1e-6, x1 + 1e-6) {
            for j in Self::spanning(&self.ys, y0, y1) {
                let step = ((self.xs[i], self.ys[j]), (self.xs[i], self.ys[j + 1]));
                self.cost_v[j * nx + i] += sign * seg_conflict(step, seg).cost(cross);
            }
        }
    }

    /// Wipe every step's conflict cost and lay all the routes back down
    /// at `cross`. Raising the price of a crossing between passes means
    /// the lattice has to be told again what everything costs.
    fn reprice(&mut self, routes: &[Vec<(f64, f64)>], live: &[bool], cross: f64) {
        self.cost_h.iter_mut().for_each(|c| *c = 0.0);
        self.cost_v.iter_mut().for_each(|c| *c = 0.0);
        for (r, _) in routes.iter().zip(live).filter(|(_, on)| **on) {
            for w in r.windows(2) {
                self.price((w[0], w[1]), cross, 1.0);
            }
        }
    }

    fn size(&self) -> usize {
        self.xs.len().saturating_mul(self.ys.len())
    }

    fn at(v: f64, axis: &[f64]) -> Option<usize> {
        axis.iter().position(|a| (a - v).abs() < 1e-6)
    }

    /// What the step from `(i, j)` by `(di, dj)` costs beyond its
    /// length, or `None` if a node is in the way.
    fn step(&self, i: usize, j: usize, di: i64, dj: i64) -> Option<f64> {
        let nx = self.xs.len();
        let (k, blocked, cost) = if di != 0 {
            let lo = if di > 0 { i } else { i - 1 };
            (j * (nx - 1) + lo, &self.blocked_h, &self.cost_h)
        } else {
            let lo = if dj > 0 { j } else { j - 1 };
            (lo * nx + i, &self.blocked_v, &self.cost_v)
        };
        if blocked[k] {
            None
        } else {
            Some(cost[k])
        }
    }
}

/// Ordered by cost, then by state, so ties break the same way on every
/// run and the SVG stays a function of the input alone.
struct Queued(f64, usize);
impl PartialEq for Queued {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == std::cmp::Ordering::Equal
    }
}
impl Eq for Queued {}
impl Ord for Queued {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        // Reversed: BinaryHeap is a max-heap, the cheapest must pop.
        o.0.total_cmp(&self.0).then_with(|| o.1.cmp(&self.1))
    }
}
impl PartialOrd for Queued {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

/// Cheapest orthogonal path from `start` to `goal` on the lattice,
/// paying [`BEND_COST`] per corner.
///
/// A state is a lattice vertex plus the direction that reached it —
/// `0` start, `1` horizontal, `2` vertical — packed into one index so
/// the search reads and writes flat arrays instead of hashing.
///
/// `start` and `goal` are the leaders: a stub already stepped out of
/// each node along its side's normal. `ban_first` and `ban_last` are the
/// two moves that would undo those stubs by walking back along them; a
/// path that took one would be collapsed back into the node it was
/// meant to leave, so an edge with a port on the right would visibly
/// leave it by some other amount than the port says.
fn grid_route(
    g: &RouteGrid,
    start: (usize, usize),
    goal: (usize, usize),
    ban_first: Option<(i64, i64)>,
    ban_last: Option<(i64, i64)>,
) -> Option<Vec<(f64, f64)>> {
    let (nx, ny) = (g.xs.len(), g.ys.len());
    if nx == 0 || ny == 0 {
        return None;
    }
    let state = |i: usize, j: usize, d: usize| (j * nx + i) * 3 + d;
    let unpack = |k: usize| ((k / 3) % nx, (k / 3) / nx, k % 3);
    let heuristic =
        |i: usize, j: usize| (g.xs[i] - g.xs[goal.0]).abs() + (g.ys[j] - g.ys[goal.1]).abs();

    let mut scratch = g.scratch.borrow_mut();
    let (best, came) = &mut *scratch;
    best.clear();
    best.resize(nx * ny * 3, f64::INFINITY);
    came.clear();
    came.resize(nx * ny * 3, usize::MAX);
    let mut heap = std::collections::BinaryHeap::new();
    let s0 = state(start.0, start.1, 0);
    best[s0] = 0.0;
    heap.push(Queued(heuristic(start.0, start.1), s0));

    // The axis the stub came in on, so turning straight off it is
    // charged as the corner it is.
    let stub = match ban_first {
        Some((bi, _)) if bi != 0 => 1,
        Some(_) => 2,
        None => 0,
    };

    while let Some(Queued(f, cur)) = heap.pop() {
        let (i, j, dir) = unpack(cur);
        let g_cur = best[cur];
        // A cheaper way here was found after this entry was queued.
        if f > g_cur + heuristic(i, j) + 1e-9 {
            continue;
        }
        if (i, j) == goal {
            let mut pts = vec![(g.xs[i], g.ys[j])];
            let mut at = cur;
            while came[at] != usize::MAX {
                at = came[at];
                let (pi, pj, _) = unpack(at);
                pts.push((g.xs[pi], g.ys[pj]));
            }
            pts.reverse();
            return Some(collapse_collinear(dedup_pts(pts)));
        }
        for (di, dj) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
            let (ni, nj) = (i as i64 + di, j as i64 + dj);
            if ni < 0 || nj < 0 || ni as usize >= nx || nj as usize >= ny {
                continue;
            }
            let (ni, nj) = (ni as usize, nj as usize);
            if dir == 0 && ban_first == Some((di, dj)) {
                continue;
            }
            if (ni, nj) == goal && ban_last == Some((di, dj)) {
                continue;
            }
            let Some(cross) = g.step(i, j, di, dj) else {
                continue;
            };
            let step_dir = if di != 0 { 1 } else { 2 };
            let from_dir = if dir == 0 { stub } else { dir };
            let bend = if from_dir != step_dir { BEND_COST } else { 0.0 };
            let len = (g.xs[i] - g.xs[ni]).abs() + (g.ys[j] - g.ys[nj]).abs();
            let g_next = g_cur + len + bend + cross;
            let next = state(ni, nj, step_dir);
            if g_next < best[next] - 1e-9 {
                best[next] = g_next;
                came[next] = cur;
                heap.push(Queued(g_next + heuristic(ni, nj), next));
            }
        }
    }
    None
}

/// Drop the middle of any three points on one straight line, so a path
/// is a list of corners rather than of lattice vertices.
///
/// Only when the middle point lies *between* the other two. Three points
/// that go out and come back are collinear as well, and erasing the turn
/// there does not shorten the path — it replaces a spike with a line
/// straight through whatever the spike was going around.
fn collapse_collinear(pts: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let between = |a: f64, b: f64, c: f64| (a - b) * (c - b) < 1e-9;
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let same_x =
                (a.0 - b.0).abs() < 1e-9 && (b.0 - p.0).abs() < 1e-9 && between(a.1, b.1, p.1);
            let same_y =
                (a.1 - b.1).abs() < 1e-9 && (b.1 - p.1).abs() < 1e-9 && between(a.0, b.0, p.0);
            if same_x || same_y {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}
/// One edge's path across the lattice: out along each side's normal to
/// a leader, the lattice between them, then in.
///
/// The grid is passed in rather than built here, because one edge tries
/// the same lattice with up to sixteen pairs of sides and rebuilding it
/// for each was the single most expensive thing the router did.
fn route_on_grid(
    g: &RouteGrid,
    from_side: AdvanceSide,
    tp0: (f64, f64),
    bp0: (f64, f64),
    to_side: AdvanceSide,
    tp3: (f64, f64),
    bp3: (f64, f64),
) -> Vec<(f64, f64)> {
    let l0 = port_leader(bp0, from_side, PORT_LEAD);
    let l3 = port_leader(bp3, to_side, PORT_LEAD);
    let finish = |mid: Vec<(f64, f64)>| {
        let mut pts = vec![tp0, bp0];
        pts.extend(mid);
        pts.push(bp3);
        pts.push(tp3);
        collapse_collinear(dedup_pts(pts))
    };

    if g.size() <= GRID_BUDGET {
        if let (Some(sx), Some(sy), Some(gx), Some(gy)) = (
            RouteGrid::at(l0.0, &g.xs),
            RouteGrid::at(l0.1, &g.ys),
            RouteGrid::at(l3.0, &g.xs),
            RouteGrid::at(l3.1, &g.ys),
        ) {
            // The step that would walk back down each stub.
            let (dx, dy) = side_delta(from_side);
            let ban_first = Some((-dx, -dy));
            let ban_last = Some(side_delta(to_side));
            if let Some(path) = grid_route(g, (sx, sy), (gx, gy), ban_first, ban_last) {
                return finish(path);
            }
        }
    }
    // No lattice path — a diagram past the budget, or two anchors boxed
    // in. Both L-shapes between the leaders honour both sides; prefer
    // one that clears every node, and only if neither does accept a
    // shape that does not, because an edge has to be drawn somewhere.
    let elbows = [(l3.0, l0.1), (l0.0, l3.1)];
    let clean = elbows.iter().find(|c| {
        [(l0, **c), (**c, l3)]
            .iter()
            .all(|(p, q)| g.rects.iter().all(|r| !seg_crosses_rect(*p, *q, *r)))
    });
    finish(vec![l0, *clean.unwrap_or(&elbows[0]), l3])
}
/// Where a point on the ring sits, measured clockwise from its
/// top-left corner. Turns "which way round is shorter" into subtraction.
fn ring_param(p: (f64, f64), side: AdvanceSide, ring: (f64, f64, f64, f64)) -> f64 {
    let (x0, y0, x1, y1) = ring;
    let (w, h) = (x1 - x0, y1 - y0);
    match side {
        AdvanceSide::Top => p.0 - x0,
        AdvanceSide::Right => w + (p.1 - y0),
        AdvanceSide::Bottom => w + h + (x1 - p.0),
        AdvanceSide::Left => 2.0 * w + h + (y1 - p.1),
    }
}

/// The ring corners passed going from `s0` to `s3` the short way round.
/// A tie goes clockwise, so the same input always turns the same way.
fn ring_corners(ring: (f64, f64, f64, f64), s0: f64, s3: f64) -> Vec<(f64, f64)> {
    let (x0, y0, x1, y1) = ring;
    let (w, h) = (x1 - x0, y1 - y0);
    let per = 2.0 * (w + h);
    if per < 1e-9 {
        return Vec::new();
    }
    let corners = [
        (0.0, (x0, y0)),
        (w, (x1, y0)),
        (w + h, (x1, y1)),
        (2.0 * w + h, (x0, y1)),
    ];
    let cw = (s3 - s0).rem_euclid(per);
    let clockwise = cw <= per - cw + 1e-9;
    let arc = if clockwise { cw } else { per - cw };
    let mut passed: Vec<(f64, (f64, f64))> = corners
        .iter()
        .map(|(cs, p)| {
            let d = if clockwise {
                (cs - s0).rem_euclid(per)
            } else {
                (s0 - cs).rem_euclid(per)
            };
            (d, *p)
        })
        .filter(|(d, _)| *d > 1e-9 && *d < arc - 1e-9)
        .collect();
    passed.sort_by(|a, b| a.0.total_cmp(&b.0));
    passed.into_iter().map(|(_, p)| p).collect()
}

/// A loop from one terminal of a node back to another: out along each
/// side's normal onto a ring around the node, then round the ring the
/// short way.
///
/// The ring is what makes this general. One corner can only join two
/// adjacent sides; opposite sides need two corners, and the same side
/// needs the two terminals to be apart in the first place — which is
/// why the caller slides them.
fn route_loop(
    a: &AdvanceSceneNode,
    from_side: AdvanceSide,
    tp0: (f64, f64),
    bp0: (f64, f64),
    to_side: AdvanceSide,
    tp3: (f64, f64),
    bp3: (f64, f64),
) -> Vec<(f64, f64)> {
    let (l, t, r, b) = node_rect(a);
    let ring = (l - PORT_LEAD, t - PORT_LEAD, r + PORT_LEAD, b + PORT_LEAD);
    let onto = |p: (f64, f64), side| match side {
        AdvanceSide::Left => (ring.0, p.1),
        AdvanceSide::Right => (ring.2, p.1),
        AdvanceSide::Top => (p.0, ring.1),
        AdvanceSide::Bottom => (p.0, ring.3),
    };
    let p0 = onto(bp0, from_side);
    let p3 = onto(bp3, to_side);
    let mut pts = vec![tp0, bp0, p0];
    pts.extend(ring_corners(
        ring,
        ring_param(p0, from_side, ring),
        ring_param(p3, to_side, ring),
    ));
    pts.extend([p3, bp3, tp3]);
    collapse_collinear(dedup_pts(pts))
}

/// `spread` is how far out this loop sits from the node, and it is
/// never negative: parallel loops nest outwards. The signed fan that
/// separates parallel *edges* pulled the innermost loop back over the
/// node it was looping around.
fn route_self_loop(a: &AdvanceSceneNode, spread: f64, dir: AdvanceDirection) -> Vec<(f64, f64)> {
    match dir {
        AdvanceDirection::Vertical => {
            let p0 = (a.x + a.w / 2.0, a.y);
            let loop_x = a.x + a.w / 2.0 + SELF_LOOP_DX + spread;
            let loop_y = a.y + a.h / 2.0 + SELF_LOOP_DROP + spread;
            let p3 = (a.x, a.y + a.h / 2.0);
            vec![p0, (loop_x, p0.1), (loop_x, loop_y), (p3.0, loop_y), p3]
        }
        AdvanceDirection::Horizontal => {
            let p0 = (a.x, a.y + a.h / 2.0);
            let loop_y = a.y + a.h / 2.0 + SELF_LOOP_DX + spread;
            let loop_x = a.x + a.w / 2.0 + SELF_LOOP_DROP + spread;
            let p3 = (a.x + a.w / 2.0, a.y);
            vec![p0, (p0.0, loop_y), (loop_x, loop_y), (loop_x, p3.1), p3]
        }
    }
}

/// Midpoint of a node's side — the anchor a ported edge exits/enters.
fn side_point(n: &AdvanceSceneNode, side: AdvanceSide) -> (f64, f64) {
    match side {
        AdvanceSide::Left => (n.x - n.w / 2.0, n.y),
        AdvanceSide::Right => (n.x + n.w / 2.0, n.y),
        AdvanceSide::Top => (n.x, n.y - n.h / 2.0),
        AdvanceSide::Bottom => (n.x, n.y + n.h / 2.0),
    }
}

/// Point `lead` px outside `p` along the normal of `side` — the leader
/// segment that leaves a node perpendicular to its anchor side.
/// Which way a lattice step moves when it leaves through `side`.
fn side_delta(side: AdvanceSide) -> (i64, i64) {
    match side {
        AdvanceSide::Left => (-1, 0),
        AdvanceSide::Right => (1, 0),
        AdvanceSide::Top => (0, -1),
        AdvanceSide::Bottom => (0, 1),
    }
}

fn port_leader(p: (f64, f64), side: AdvanceSide, lead: f64) -> (f64, f64) {
    match side {
        AdvanceSide::Left => (p.0 - lead, p.1),
        AdvanceSide::Right => (p.0 + lead, p.1),
        AdvanceSide::Top => (p.0, p.1 - lead),
        AdvanceSide::Bottom => (p.0, p.1 + lead),
    }
}

/// The side the router would pick for `n` on its own, so a ported edge
/// with only one side specified still connects at the natural anchor.
fn natural_side(
    n: &AdvanceSceneNode,
    other: &AdvanceSceneNode,
    dir: AdvanceDirection,
    is_from: bool,
    same_lane: bool,
) -> AdvanceSide {
    match dir {
        AdvanceDirection::Vertical => {
            if same_lane {
                // Same lane: leave through the bottom/top toward the other node.
                if is_from {
                    if n.y < other.y {
                        AdvanceSide::Bottom
                    } else {
                        AdvanceSide::Top
                    }
                } else if n.y < other.y {
                    AdvanceSide::Top
                } else {
                    AdvanceSide::Bottom
                }
            } else if is_from {
                // Cross lane: exit sideways toward the other lane.
                if other.x >= n.x {
                    AdvanceSide::Right
                } else {
                    AdvanceSide::Left
                }
            } else if other.x >= n.x {
                AdvanceSide::Left
            } else {
                AdvanceSide::Right
            }
        }
        AdvanceDirection::Horizontal => {
            if same_lane {
                if is_from {
                    if n.x < other.x {
                        AdvanceSide::Right
                    } else {
                        AdvanceSide::Left
                    }
                } else if n.x < other.x {
                    AdvanceSide::Left
                } else {
                    AdvanceSide::Right
                }
            } else if is_from {
                if other.y >= n.y {
                    AdvanceSide::Bottom
                } else {
                    AdvanceSide::Top
                }
            } else if other.y >= n.y {
                AdvanceSide::Top
            } else {
                AdvanceSide::Bottom
            }
        }
    }
}

/// Prefer `wanted` if it is exposed; otherwise the first exposed side
/// in a fixed order, so the choice is deterministic.
fn pick_exposed_side(wanted: AdvanceSide, exposed: [bool; 4], dx: f64, dy: f64) -> AdvanceSide {
    if exposed[side_index(wanted)] {
        return wanted;
    }
    // Lean toward the target: the horizontal side facing it first, then
    // the vertical one, then their opposites.
    let (h, hb) = if dx >= 0.0 {
        (AdvanceSide::Right, AdvanceSide::Left)
    } else {
        (AdvanceSide::Left, AdvanceSide::Right)
    };
    let (v, vb) = if dy >= 0.0 {
        (AdvanceSide::Bottom, AdvanceSide::Top)
    } else {
        (AdvanceSide::Top, AdvanceSide::Bottom)
    };
    [h, v, hb, vb]
        .into_iter()
        .find(|s| exposed[side_index(*s)])
        .unwrap_or(wanted)
}

/// Resolve one edge end to `(terminal point, node-boundary point)`.
///
/// For a sub-element the terminal sits on the element's rect and the
/// boundary point is straight out along `side` on the node's rect —
/// the *lead* that crosses the endpoint's own node. For a node-level
/// anchor or a plain side the two points coincide.
///
/// Returns `None` only if the scene lacks an element or anchor the
/// parser already validated — an internal invariant, not user error.
/// How close to a corner a slid terminal may get, so an edge nudged
/// along a side still leaves from the side and not from its very edge.
const SIDE_INSET: f64 = 6.0;

/// Slide a point along the side it sits on, staying clear of the corners.
fn slide_along(
    p: (f64, f64),
    side: AdvanceSide,
    by: f64,
    rect: (f64, f64, f64, f64),
) -> (f64, f64) {
    let (l, t, r, b) = rect;
    let held = |v: f64, lo: f64, hi: f64| {
        if hi - lo < 1e-9 {
            (lo + hi) / 2.0
        } else {
            v.clamp(lo, hi)
        }
    };
    match side {
        AdvanceSide::Left | AdvanceSide::Right => {
            (p.0, held(p.1 + by, t + SIDE_INSET, b - SIDE_INSET))
        }
        AdvanceSide::Top | AdvanceSide::Bottom => {
            (held(p.0 + by, l + SIDE_INSET, r - SIDE_INSET), p.1)
        }
    }
}

/// Where an edge end attaches, as `(terminal, boundary)`: the point on
/// whatever it names, and the point on the node's own outline that the
/// lead reaches. They differ only for a sub-element.
///
/// `slide` nudges the terminal along its side so that edges which would
/// otherwise land on the same point do not. A named anchor is an exact
/// promise about where the line attaches and is never slid.
fn resolve_terminal(
    sn: &AdvanceSceneNode,
    end: &AdvanceEnd,
    side: AdvanceSide,
    slide: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let named = match &end.at {
        Some(AnchorRef::Named(id)) => Some(id.as_str()),
        _ => None,
    };
    if end.path.is_empty() {
        return Some(match named {
            Some(id) => {
                let a = sn
                    .anchors
                    .iter()
                    .find(|a| a.element.is_none() && a.id == id)?;
                ((a.x, a.y), (a.x, a.y))
            }
            None => {
                let p = slide_along(side_point(sn, side), side, slide, node_rect(sn));
                (p, p)
            }
        });
    }
    let (ei, el) = sn
        .elements
        .iter()
        .enumerate()
        .find(|(_, el)| el.path == end.path)?;
    let tp = match named {
        Some(id) => {
            let a = sn
                .anchors
                .iter()
                .find(|a| a.element == Some(ei) && a.id == id)?;
            (a.x, a.y)
        }
        None => slide_along(
            rect_side_point(el.x, el.y, el.w, el.h, side, DEFAULT_ANCHOR_OFFSET),
            side,
            slide,
            (
                el.x - el.w / 2.0,
                el.y - el.h / 2.0,
                el.x + el.w / 2.0,
                el.y + el.h / 2.0,
            ),
        ),
    };
    let (l, t, r, b) = node_rect(sn);
    let bp = match side {
        AdvanceSide::Left => (l, tp.1),
        AdvanceSide::Right => (r, tp.1),
        AdvanceSide::Top => (tp.0, t),
        AdvanceSide::Bottom => (tp.0, b),
    };
    Some((tp, bp))
}

/// Drop consecutive duplicate points (rounded path collapse leaves none).
fn dedup_pts(pts: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(pts.len());
    for p in pts {
        if let Some(last) = out.last() {
            if (last.0 - p.0).abs() < 1e-9 && (last.1 - p.1).abs() < 1e-9 {
                continue;
            }
        }
        out.push(p);
    }
    out
}

fn push_unique(out: &mut Vec<(f64, f64)>, c: (f64, f64)) {
    if !out
        .iter()
        .any(|p| (p.0 - c.0).abs() < 1e-6 && (p.1 - c.1).abs() < 1e-6)
    {
        out.push(c);
    }
}

/// Bounding box of an edge-label pill centred at `c`, matching the
/// renderer's `text_width + 14` wide, 18 tall box.
fn label_box(c: (f64, f64), label: &str) -> (f64, f64, f64, f64) {
    let lw = text_width(label) + 14.0;
    (c.0 - lw / 2.0, c.1 - 9.0, c.0 + lw / 2.0, c.1 + 9.0)
}

fn rects_overlap(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.2 && a.2 > b.0 && a.1 < b.3 && a.3 > b.1
}

/// Candidate label anchors in priority order: the renderer's default
/// spot first (so untouched labels stay byte-identical), then the
/// longest horizontal segment, the polyline midpoint, then each
/// segment midpoint — deduplicated.
fn label_candidates(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = points.len();
    let mut out: Vec<(f64, f64)> = Vec::new();
    if n == 0 {
        return out;
    }

    let mut best_vert: Option<(usize, f64)> = None;
    for i in 0..n.saturating_sub(1) {
        let (x1, y1) = points[i];
        let (x2, y2) = points[i + 1];
        if (x1 - x2).abs() < f64::EPSILON {
            let len = (y2 - y1).abs();
            if best_vert.map(|(_, l)| len > l).unwrap_or(true) {
                best_vert = Some((i, len));
            }
        }
    }
    match best_vert {
        Some((i, len)) if len >= 22.0 => {
            let (x1, y1) = points[i];
            let (_, y2) = points[i + 1];
            push_unique(&mut out, (x1 + 8.0, (y1 + y2) / 2.0));
        }
        _ => {
            let mid = points[n / 2];
            push_unique(&mut out, (mid.0, mid.1));
        }
    }

    let mut best_h: Option<(usize, f64)> = None;
    for i in 0..n.saturating_sub(1) {
        let (x1, y1) = points[i];
        let (x2, y2) = points[i + 1];
        if (y1 - y2).abs() < f64::EPSILON {
            let len = (x2 - x1).abs();
            if best_h.map(|(_, l)| len > l).unwrap_or(true) {
                best_h = Some((i, len));
            }
        }
    }
    if let Some((i, _)) = best_h {
        let (x1, y1) = points[i];
        let (x2, y2) = points[i + 1];
        push_unique(&mut out, ((x1 + x2) / 2.0, (y1 + y2) / 2.0));
    }

    let mid = points[n / 2];
    push_unique(&mut out, (mid.0, mid.1));
    for i in 0..n.saturating_sub(1) {
        let (x1, y1) = points[i];
        let (x2, y2) = points[i + 1];
        push_unique(&mut out, ((x1 + x2) / 2.0, (y1 + y2) / 2.0));
    }
    out
}

/// Pick the first candidate whose label pill avoids every other node
/// (endpoints excluded — a self-loop still dodges its own node) and
/// every already-placed label. Falls back to the first candidate.
fn choose_label_pos(
    candidates: &[(f64, f64)],
    label: &str,
    from: &str,
    to: &str,
    nodes: &[AdvanceSceneNode],
    placed: &[(f64, f64, f64, f64)],
) -> Option<(f64, f64)> {
    for &c in candidates {
        let lb = label_box(c, label);
        let is_self = from == to;
        let hits_node = nodes.iter().any(|n| {
            let endpoint = (n.id == from || n.id == to) && !is_self;
            !endpoint && rects_overlap(lb, node_rect(n))
        });
        if hits_node {
            continue;
        }
        if placed.iter().any(|pb| rects_overlap(lb, *pb)) {
            continue;
        }
        return Some(c);
    }
    candidates.first().copied()
}

fn route_edges(
    d: &AdvanceDiagram,
    nodes: &[AdvanceSceneNode],
    lanes: &[AdvanceSceneLane],
    dir: AdvanceDirection,
) -> Vec<AdvanceSceneEdge> {
    // Scene nodes come out of layout in lane/topology order, NOT in
    // declaration order — so they are looked up by id. Indexing them
    // with the model's order attached edges to the wrong node.
    let model_idx = node_index_map(d);
    let scene_idx: std::collections::HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut edge_scenes = Vec::with_capacity(d.edges.len());

    // Edges are fanned apart only when they would otherwise be drawn
    // on top of each other, so the key is the pair of terminals — two
    // edges between the same nodes but through different ports already
    // land in different places and must keep their full leaders.
    let fan_key = |e: &AdvanceEdge| {
        let side = |s: Option<AdvanceSide>| s.map(|s| s.name()).unwrap_or("");
        (
            format!("{}:{}", e.from_end.to_ref(), side(e.from_side)),
            format!("{}:{}", e.to_end.to_ref(), side(e.to_side)),
        )
    };
    let mut pair_totals: std::collections::HashMap<(String, String), usize> =
        std::collections::HashMap::new();
    for e in &d.edges {
        *pair_totals.entry(fan_key(e)).or_insert(0) += 1;
    }
    let mut pair_seen: std::collections::HashMap<(String, String), usize> =
        std::collections::HashMap::new();

    // Everything about an edge that the lattice has to know before it
    // can be built: which nodes, how far the edge is fanned, and which
    // sides it may leave and arrive on.
    struct EdgePlan {
        from_i: usize,
        to_i: usize,
        fan: f64,
        /// How far a plain loop nests out from its node. Always >= 0.
        spread: f64,
        plain_loop: bool,
        fss: Vec<AdvanceSide>,
        tss: Vec<AdvanceSide>,
    }

    let plans: Vec<EdgePlan> = d
        .edges
        .iter()
        .map(|e| {
            let from_i = scene_idx[e.from.as_str()];
            let to_i = scene_idx[e.to.as_str()];
            let (a, b) = (&nodes[from_i], &nodes[to_i]);

            let key = fan_key(e);
            let dup_i = {
                let v = pair_seen.entry(key.clone()).or_insert(0);
                let i = *v;
                *v += 1;
                i
            };
            let dup_n = pair_totals[&key];
            let fan = (dup_i as f64 - (dup_n as f64 - 1.0) / 2.0) * PARALLEL_FAN;
            let same_lane = a.lane == b.lane || from_i == to_i;

            // The sides this end may leave from, best guess first. A
            // declared port or a named anchor yields exactly one:
            // honouring it however long the path gets is what makes it
            // a port (D7). An automatic end offers the alternatives,
            // which the router falls back on only when its first choice
            // runs into an edge already drawn.
            let sides_for = |end: &AdvanceEnd,
                             given: Option<AdvanceSide>,
                             ni: usize,
                             is_from: bool|
             -> Vec<AdvanceSide> {
                // `natural_side` always takes (source, target);
                // `is_from` picks which of the two sides it computes.
                let natural = given.unwrap_or_else(|| natural_side(a, b, dir, is_from, same_lane));
                let exposed = if end.path.is_empty() {
                    [true; 4]
                } else {
                    match resolve_element(&d.nodes[ni], &end.path) {
                        Ok((_, exposed)) => exposed,
                        Err(_) => [true; 4],
                    }
                };
                let first = if end.path.is_empty() {
                    natural
                } else {
                    let (other, me) = if is_from { (b, a) } else { (a, b) };
                    pick_exposed_side(natural, exposed, other.x - me.x, other.y - me.y)
                };
                if given.is_some() || matches!(end.at, Some(AnchorRef::Named(_))) {
                    return vec![first];
                }
                let mut out = vec![first];
                out.extend(
                    [
                        AdvanceSide::Right,
                        AdvanceSide::Left,
                        AdvanceSide::Bottom,
                        AdvanceSide::Top,
                    ]
                    .into_iter()
                    .filter(|s| *s != first && exposed[side_index(*s)]),
                );
                out
            };
            // A node-to-itself edge that names nothing keeps the classic
            // loop; anything with a port, an anchor or a sub-element on
            // it must resolve its terminals first, or the loop would be
            // drawn between points the edge never asked for.
            let plain = |end: &AdvanceEnd, given: Option<AdvanceSide>| {
                given.is_none() && end.path.is_empty() && end.at.is_none()
            };
            EdgePlan {
                from_i,
                to_i,
                fan,
                spread: dup_i as f64 * PARALLEL_FAN,
                plain_loop: from_i == to_i
                    && plain(&e.from_end, e.from_side)
                    && plain(&e.to_end, e.to_side),
                fss: sides_for(&e.from_end, e.from_side, model_idx[&e.from], true),
                tss: sides_for(&e.to_end, e.to_side, model_idx[&e.to], false),
            }
        })
        .collect();

    // Both ends slide by the same amount, so parallel edges come out as
    // parallel lines rather than one line drawn twice. A loop back onto
    // the same side needs its two ends pushed apart on top of that, or
    // it has no width.
    let ends = |e: &AdvanceEdge, pl: &EdgePlan, fs: AdvanceSide, ts: AdvanceSide| {
        let (sf, st) = if pl.from_i == pl.to_i && fs == ts {
            (pl.fan - SELF_LOOP_SPREAD, pl.fan + SELF_LOOP_SPREAD)
        } else {
            (pl.fan, pl.fan)
        };
        let f = resolve_terminal(&nodes[pl.from_i], &e.from_end, fs, sf)
            .expect("terminal validated at parse time is missing from the scene");
        let t = resolve_terminal(&nodes[pl.to_i], &e.to_end, ts, st)
            .expect("terminal validated at parse time is missing from the scene");
        (f, t)
    };

    // One lattice for the whole diagram, carrying every leader any edge
    // could use. Rebuilding it per edge — let alone per candidate pair
    // of sides — made routing quadratic in the number of edges: each
    // rebuild re-read every route drawn before it.
    //
    // Every edge between two different nodes is routed on it, so "no
    // line through a box" is a property of the search rather than a
    // check repeated per kind of edge. A loop back onto one node is the
    // exception: it is not a path between two points, so it is drawn on
    // a ring around its node instead. Its points are priced into the
    // lattice afterwards, so later edges still avoid it.
    let mut leads = Vec::new();
    for (e, pl) in d.edges.iter().zip(&plans) {
        if pl.from_i == pl.to_i {
            continue;
        }
        for fs in &pl.fss {
            let ((_, bp0), _) = ends(e, pl, *fs, pl.tss[0]);
            leads.push(port_leader(bp0, *fs, PORT_LEAD));
        }
        for ts in &pl.tss {
            let (_, (_, bp3)) = ends(e, pl, pl.fss[0], *ts);
            leads.push(port_leader(bp3, *ts, PORT_LEAD));
        }
    }
    let mut grid = RouteGrid::build(nodes, lanes, &leads, &[]);

    // An invisible edge is never drawn, so it must not push a visible
    // one out of the way either.
    let live: Vec<bool> = d
        .edges
        .iter()
        .map(|e| e.kind != EdgeKind::Invisible)
        .collect();

    // One route per edge, by edge index. The negotiation below takes
    // routes back out and lays them down again, so they cannot simply
    // accumulate in the order they were drawn.
    let mut routes: Vec<Vec<(f64, f64)>> = Vec::with_capacity(d.edges.len());

    // Routing one edge against the routes in `routes`, ignoring the one
    // at `skip` — its own, when it is being re-routed.
    let route_one = |i: usize,
                     g: &RouteGrid,
                     routes: &[Vec<(f64, f64)>],
                     live: &[bool],
                     skip: usize,
                     cross: f64|
     -> Vec<(f64, f64)> {
        let (e, pl) = (&d.edges[i], &plans[i]);
        let (from_i, to_i) = (pl.from_i, pl.to_i);
        let a = &nodes[from_i];
        let (fss, tss) = (&pl.fss, &pl.tss);
        if pl.plain_loop {
            return route_self_loop(a, pl.spread, dir);
        }
        let route = |fs: AdvanceSide, ts: AdvanceSide| {
            let ((tp0, bp0), (tp3, bp3)) = ends(e, pl, fs, ts);
            if from_i == to_i {
                route_loop(a, fs, tp0, bp0, ts, tp3, bp3)
            } else {
                route_on_grid(g, fs, tp0, bp0, ts, tp3, bp3)
            }
        };
        let first = route(fss[0], tss[0]);
        // The preferred sides usually win outright; only an edge that
        // would run into an existing one pays for the search.
        if route_conflicts(&first, routes, live, skip, cross) <= 0.0
            || (fss.len() == 1 && tss.len() == 1)
            || g.size() > SEARCH_BUDGET
        {
            return first;
        }
        // One end at a time first: changing a single side is what
        // usually resolves a conflict, and it turns the sixteen pairs
        // into seven before the full cross.
        let mut pairs: Vec<(AdvanceSide, AdvanceSide)> = Vec::with_capacity(16);
        pairs.extend(fss[1..].iter().map(|fs| (*fs, tss[0])));
        pairs.extend(tss[1..].iter().map(|ts| (fss[0], *ts)));
        for fs in &fss[1..] {
            pairs.extend(tss[1..].iter().map(|ts| (*fs, *ts)));
        }
        let mut best = (route_cost(&first, routes, live, skip, cross), first);
        for (fs, ts) in &pairs {
            let pts = route(*fs, *ts);
            let conflicts = route_conflicts(&pts, routes, live, skip, cross);
            let cost = route_cost(&pts, routes, live, skip, cross);
            if cost < best.0 - 1e-9 {
                best = (cost, pts);
            }
            // Nothing beats touching nothing, and the order of the
            // candidates is fixed, so the first such route is the same
            // one on every run.
            if conflicts <= 0.0 {
                break;
            }
        }
        best.1
    };

    // First pass: each edge sees only the ones before it.
    for i in 0..d.edges.len() {
        let pts = route_one(i, &grid, &routes, &live, usize::MAX, CROSS_COST);
        if live[i] {
            for w in pts.windows(2) {
                grid.price((w[0], w[1]), CROSS_COST, 1.0);
            }
        }
        routes.push(pts);
    }

    // Then negotiate: take each edge back out and route it against
    // every other one, raising the price of a crossing each pass until
    // the crossings that can move have moved. This is what turns "the
    // edges before me" into "all the others", which is the difference
    // between a first-come-first-served artefact and a crossing that
    // the geometry actually forces.
    let (mut count, mut involved) = crossings_of(&routes, &live);
    if count > 0 && grid.size() <= NEGOTIATION_BUDGET {
        let mut cross = CROSS_COST;
        let mut best = (count, routes.clone());
        for _ in 0..NEGOTIATION_PASSES {
            cross *= CROSS_ESCALATION;
            grid.reprice(&routes, &live, cross);
            // The edges in a crossing go first: they are the ones with
            // something to gain, and moving them first gives the rest
            // room to follow.
            let mut order = involved.clone();
            order.extend((0..d.edges.len()).filter(|i| !involved.contains(i)));
            for i in order {
                if live[i] {
                    for w in routes[i].windows(2) {
                        grid.price((w[0], w[1]), cross, -1.0);
                    }
                }
                let pts = route_one(i, &grid, &routes, &live, i, cross);
                if live[i] {
                    for w in pts.windows(2) {
                        grid.price((w[0], w[1]), cross, 1.0);
                    }
                }
                routes[i] = pts;
            }
            (count, involved) = crossings_of(&routes, &live);
            if count == 0 {
                best = (0, Vec::new());
                break;
            }
            // A pass that gains nothing means the escalation has run
            // out of room to move anything: the crossings that are left
            // are the ones the geometry forces, and another pass would
            // only cost time.
            if count < best.0 {
                best = (count, routes.clone());
            } else {
                break;
            }
        }
        // A pass can make things worse; keep the best one seen.
        if best.0 < count {
            routes = best.1;
        }
    }

    // Boxes of labels already placed, so later labels dodge them too.
    let mut placed_labels: Vec<(f64, f64, f64, f64)> = Vec::new();

    for (i, (e, pl)) in d.edges.iter().zip(&plans).enumerate() {
        let a = &nodes[pl.from_i];
        let b = &nodes[pl.to_i];
        let points = std::mem::take(&mut routes[i]);
        let label_pos = match e.label.as_deref() {
            Some(label) => {
                let chosen = choose_label_pos(
                    &label_candidates(&points),
                    label,
                    &e.from,
                    &e.to,
                    nodes,
                    &placed_labels,
                );
                if let Some(c) = chosen {
                    placed_labels.push(label_box(c, label));
                }
                chosen
            }
            None => None,
        };

        let from_point = points.first().copied().unwrap_or((a.x, a.y));
        let to_point = points.last().copied().unwrap_or((b.x, b.y));
        edge_scenes.push(AdvanceSceneEdge {
            from: e.from.clone(),
            to: e.to.clone(),
            label: e.label.clone(),
            kind: e.kind,
            points,
            style: e.style.clone(),
            from_side: e.from_side,
            to_side: e.to_side,
            label_pos,
            from_point,
            to_point,
            from_end: e.from_end.clone(),
            to_end: e.to_end.clone(),
        });
    }

    edge_scenes
}

// ------------------------------------------------------------------
// Scene hit-testing (drag-and-drop / picking)
// ------------------------------------------------------------------

/// An element picked out of an [`AdvanceScene`] by [`AdvanceScene::hit_test`]
/// and friends — carries the index into `scene.{nodes,edges,lanes}`, and
/// `scene.nodes[i].id` recovers the stable id. Coordinates are SCENE-space:
/// a host converts screen→scene once (`(screen - pan) / zoom`) and passes a
/// tolerance in scene units, so zoom/pan never enter the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceHit {
    Node(usize),
    Edge(usize),
    Lane(usize),
    /// `(node, element)` — indices into `scene.nodes` and that node's
    /// flat `elements`.
    Element(usize, usize),
    /// `(node, anchor)` — indices into `scene.nodes` and that node's
    /// `anchors`.
    Anchor(usize, usize),
}

impl AdvanceScene {
    /// The topmost element at scene point `(x, y)`, or `None`. Z-order
    /// mirrors paint order: a node beats an overlapping edge, which
    /// beats a lane box behind them. `tol` (scene units) widens edge
    /// picking so thin routes are still selectable.
    pub fn hit_test(&self, x: f64, y: f64, tol: f64) -> Option<AdvanceHit> {
        // An anchor wins only on the node that is actually under the
        // point (or anywhere when the point is on no node), so an
        // occluded node's anchor cannot beat the visible one.
        let top = self.node_at(x, y);
        let anchor = match top {
            // Scope the search to the node under the point: taking the
            // globally nearest anchor and discarding it when it belongs
            // to someone else made the top node's OWN anchor unreachable
            // whenever an occluded node had one slightly nearer.
            Some(t) => self.nearest_anchor_on(t, x, y, tol).map(|a| (t, a)),
            None => self.anchor_at(x, y, tol),
        };
        if let Some((n, a)) = anchor {
            return Some(AdvanceHit::Anchor(n, a));
        }
        if let Some((n, e)) = self.element_at(x, y) {
            return Some(AdvanceHit::Element(n, e));
        }
        if let Some(i) = self.node_at(x, y) {
            return Some(AdvanceHit::Node(i));
        }
        if let Some(i) = self.edge_at(x, y, tol) {
            return Some(AdvanceHit::Edge(i));
        }
        self.lane_at(x, y).map(AdvanceHit::Lane)
    }

    /// The anchor within `tol` scene units of `(x, y)`, nearest first,
    /// as `(node, anchor)` — the snap target when an edge is being drawn.
    pub fn anchor_at(&self, x: f64, y: f64, tol: f64) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, f64)> = None;
        for (ni, n) in self.nodes.iter().enumerate() {
            for (ai, a) in n.anchors.iter().enumerate() {
                let d = (x - a.x).hypot(y - a.y);
                if d <= tol && best.map_or(true, |(_, _, bd)| d <= bd) {
                    best = Some((ni, ai, d));
                }
            }
        }
        best.map(|(n, a, _)| (n, a))
    }

    /// Index of the anchor on `node` nearest `(x, y)` within `tol`.
    fn nearest_anchor_on(&self, node: usize, x: f64, y: f64, tol: f64) -> Option<usize> {
        self.nodes
            .get(node)?
            .anchors
            .iter()
            .enumerate()
            .map(|(i, a)| (i, (x - a.x).hypot(y - a.y)))
            .filter(|(_, d)| *d <= tol)
            .min_by(|p, q| p.1.total_cmp(&q.1))
            .map(|(i, _)| i)
    }

    /// The innermost sub-element containing `(x, y)`, as
    /// `(node, element)`; `None` when the point is on no sub-element.
    pub fn element_at(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let ni = self.node_at(x, y)?;
        let n = &self.nodes[ni];
        let mut best: Option<(usize, usize)> = None;
        for (ei, el) in n.elements.iter().enumerate() {
            let inside = (x - el.x).abs() <= el.w / 2.0 && (y - el.y).abs() <= el.h / 2.0;
            if inside && best.map_or(true, |(_, depth)| el.path.len() >= depth) {
                best = Some((ei, el.path.len()));
            }
        }
        best.map(|(ei, _)| (ni, ei))
    }

    /// Index of the topmost node whose shape contains `(x, y)`, tested
    /// back-to-front (later-drawn wins, as painted). Shape-precise for
    /// diamonds and (double) circles — their bounding box would
    /// over-select the empty corners — and the bounding rectangle for
    /// every other shape.
    pub fn node_at(&self, x: f64, y: f64) -> Option<usize> {
        self.nodes.iter().rposition(|n| node_contains_adv(n, x, y))
    }

    /// Index of the edge nearest `(x, y)` within `tol` scene units, or
    /// `None`. Distance is measured to the drawn route polyline, and the
    /// NEAREST edge wins so overlapping routes resolve cleanly.
    pub fn edge_at(&self, x: f64, y: f64, tol: f64) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for (i, e) in self.edges.iter().enumerate() {
            if matches!(e.kind, EdgeKind::Invisible) {
                continue; // never drawn — never picked
            }
            let d = e
                .points
                .windows(2)
                .map(|w| point_seg_dist_adv(x, y, w[0], w[1]))
                .fold(f64::INFINITY, f64::min);
            // `<=` so a later-drawn edge wins a distance tie, matching
            // node_at's topmost-wins z-order.
            if d <= tol && best.map_or(true, |(_, bd)| d <= bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Index of the lane whose box contains `(x, y)`, or `None`. Nested
    /// lanes resolve to the SMALLEST containing box (the most specific
    /// one). Lanes paint behind nodes and edges, so prefer [`hit_test`]
    /// when you want proper z-order.
    pub fn lane_at(&self, x: f64, y: f64) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for (i, l) in self.lanes.iter().enumerate() {
            let inside = x >= l.x && x <= l.x + l.w && y >= l.y && y <= l.y + l.h;
            let area = l.w * l.h;
            let better = best.map_or(true, |(bi, ba)| area < ba || (area == ba && i < bi));
            if inside && better {
                best = Some((i, area));
            }
        }
        best.map(|(i, _)| i)
    }

    /// The node nearest `(x, y)` and its distance in scene units (0 when
    /// the point is inside the node's box) — edge-drawing snap:
    /// "drop near B → connect to B".
    pub fn nearest_node(&self, x: f64, y: f64) -> Option<(usize, f64)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let dx = (x - n.x).abs() - n.w / 2.0;
                let dy = (y - n.y).abs() - n.h / 2.0;
                (i, dx.max(0.0).hypot(dy.max(0.0)))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }
}

/// Whether a node's SHAPE (not just its bounding box) contains a point —
/// mirrors what the advance renderer actually draws: diamonds and
/// (double) circles are over-selected by their boxes, everything else
/// (including the rect-drawn state pseudostates) is a bounding rect.
fn node_contains_adv(n: &AdvanceSceneNode, x: f64, y: f64) -> bool {
    let (hw, hh) = (n.w / 2.0, n.h / 2.0);
    if hw <= 0.0 || hh <= 0.0 {
        return false;
    }
    let (dx, dy) = ((x - n.x).abs(), (y - n.y).abs());
    match n.shape {
        // Rhombus: |dx|/hw + |dy|/hh <= 1.
        Shape::Diamond => dx / hw + dy / hh <= 1.0,
        // Circle: the renderer draws `<circle r=w/2>` — a disc, not an
        // ellipse — so match that exactly for any w/h.
        Shape::Circle | Shape::DoubleCircle => dx * dx + dy * dy <= hw * hw,
        // Everything else: bounding rectangle.
        _ => dx <= hw && dy <= hh,
    }
}

/// Distance from `(x, y)` to a segment `a`–`b`.
fn point_seg_dist_adv(px: f64, py: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let len2 = abx * abx + aby * aby;
    let t = if len2 <= f64::EPSILON {
        0.0
    } else {
        (((px - a.0) * abx + (py - a.1) * aby) / len2).clamp(0.0, 1.0)
    };
    ((a.0 + t * abx) - px).hypot((a.1 + t * aby) - py)
}

// ------------------------------------------------------------------
// Recursive Dimension & Layout Helpers (Vertical & Horizontal)
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
struct LaneDim {
    w: f64,
    h: f64,
    children: Vec<LaneDim>,
}

fn compute_lane_dim_rec(
    lane: &AdvanceLane,
    lane_idx: &std::collections::HashMap<String, usize>,
    lane_node_lists: &[Vec<usize>],
    sizes: &[(f64, f64)],
    cfg: &AdvanceConfig,
    dir: AdvanceDirection,
) -> LaneDim {
    let li = lane_idx[&lane.id];
    let local_nodes = &lane_node_lists[li];

    match dir {
        AdvanceDirection::Vertical => {
            let mut direct_node_w: f64 = 0.0;
            let mut direct_node_h: f64 = 0.0;
            for &ni in local_nodes {
                direct_node_w = direct_node_w.max(sizes[ni].0);
                direct_node_h += sizes[ni].1 + cfg.node_gap_y;
            }
            if !local_nodes.is_empty() {
                direct_node_h -= cfg.node_gap_y;
            }

            if lane.children.is_empty() {
                let w = (direct_node_w + 2.0 * cfg.lane_pad_x).max(120.0);
                let h =
                    (cfg.lane_title_h + cfg.lane_pad_y + direct_node_h + cfg.lane_pad_y).max(120.0);
                LaneDim {
                    w,
                    h,
                    children: Vec::new(),
                }
            } else {
                let child_dims: Vec<LaneDim> = lane
                    .children
                    .iter()
                    .map(|c| compute_lane_dim_rec(c, lane_idx, lane_node_lists, sizes, cfg, dir))
                    .collect();
                let sum_children_w: f64 = child_dims.iter().map(|c| c.w).sum::<f64>()
                    + (child_dims.len().saturating_sub(1) as f64 * cfg.lane_gap);
                let max_children_h = child_dims.iter().map(|c| c.h).fold(0.0_f64, f64::max);

                let w = (sum_children_w + 2.0 * cfg.lane_pad_x)
                    .max(direct_node_w + 2.0 * cfg.lane_pad_x)
                    .max(120.0);
                let h = (cfg.lane_title_h
                    + cfg.lane_pad_y
                    + max_children_h
                    + cfg.lane_pad_y
                    + direct_node_h)
                    .max(120.0);
                LaneDim {
                    w,
                    h,
                    children: child_dims,
                }
            }
        }
        AdvanceDirection::Horizontal => {
            let mut direct_node_w: f64 = 0.0;
            let mut direct_node_h: f64 = 0.0;
            for &ni in local_nodes {
                direct_node_h = direct_node_h.max(sizes[ni].1);
                direct_node_w += sizes[ni].0 + cfg.lane_gap;
            }
            if !local_nodes.is_empty() {
                direct_node_w -= cfg.lane_gap;
            }

            if lane.children.is_empty() {
                let w =
                    (cfg.lane_title_h + cfg.lane_pad_x + direct_node_w + cfg.lane_pad_x).max(160.0);
                let h = (direct_node_h + 2.0 * cfg.lane_pad_y)
                    .max(cfg.lane_title_h + 2.0 * cfg.lane_pad_y)
                    .max(80.0);
                LaneDim {
                    w,
                    h,
                    children: Vec::new(),
                }
            } else {
                let child_dims: Vec<LaneDim> = lane
                    .children
                    .iter()
                    .map(|c| compute_lane_dim_rec(c, lane_idx, lane_node_lists, sizes, cfg, dir))
                    .collect();
                let sum_children_h: f64 = child_dims.iter().map(|c| c.h).sum::<f64>()
                    + (child_dims.len().saturating_sub(1) as f64 * cfg.lane_gap);
                let max_children_w = child_dims.iter().map(|c| c.w).fold(0.0_f64, f64::max);

                let w = (cfg.lane_title_h
                    + cfg.lane_pad_x
                    + max_children_w
                    + cfg.lane_pad_x
                    + direct_node_w)
                    .max(160.0);
                let h = (sum_children_h + 2.0 * cfg.lane_pad_y)
                    .max(direct_node_h + 2.0 * cfg.lane_pad_y)
                    .max(80.0);
                LaneDim {
                    w,
                    h,
                    children: child_dims,
                }
            }
        }
    }
}

fn equalize_sibling_heights(dim: &mut LaneDim) {
    if !dim.children.is_empty() {
        let max_h = dim.children.iter().map(|c| c.h).fold(0.0_f64, f64::max);
        for c in &mut dim.children {
            c.h = max_h;
            equalize_sibling_heights(c);
        }
    }
}

fn equalize_sibling_widths(dim: &mut LaneDim) {
    if !dim.children.is_empty() {
        let max_w = dim.children.iter().map(|c| c.w).fold(0.0_f64, f64::max);
        for c in &mut dim.children {
            c.w = max_w;
            equalize_sibling_widths(c);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_lanes_and_nodes_rec(
    lane: &AdvanceLane,
    dim: &LaneDim,
    x: f64,
    y: f64,
    d: &AdvanceDiagram,
    lane_idx: &std::collections::HashMap<String, usize>,
    ordered_node_indices: &[Vec<usize>],
    sizes: &[(f64, f64)],
    cfg: &AdvanceConfig,
    dir: AdvanceDirection,
    lane_scenes: &mut Vec<AdvanceSceneLane>,
    node_scenes: &mut Vec<AdvanceSceneNode>,
) {
    // Parents emitted first so painter's algorithm renders them behind children
    lane_scenes.push(AdvanceSceneLane {
        id: lane.id.clone(),
        title: lane.title.clone(),
        x,
        y,
        w: dim.w,
        h: dim.h,
    });

    match dir {
        AdvanceDirection::Vertical => {
            let mut children_h: f64 = 0.0;
            if !lane.children.is_empty() {
                let mut cur_x = x + cfg.lane_pad_x;
                let cur_y = y + cfg.lane_title_h + cfg.lane_pad_y;
                for (c, c_dim) in lane.children.iter().zip(&dim.children) {
                    emit_lanes_and_nodes_rec(
                        c,
                        c_dim,
                        cur_x,
                        cur_y,
                        d,
                        lane_idx,
                        ordered_node_indices,
                        sizes,
                        cfg,
                        dir,
                        lane_scenes,
                        node_scenes,
                    );
                    cur_x += c_dim.w + cfg.lane_gap;
                    children_h = children_h.max(c_dim.h);
                }
            }

            // Direct nodes inside this lane
            let li = lane_idx[&lane.id];
            let local_nodes = &ordered_node_indices[li];
            let mut cursor_y = if lane.children.is_empty() {
                y + cfg.lane_title_h + cfg.lane_pad_y
            } else {
                y + cfg.lane_title_h + cfg.lane_pad_y + children_h + cfg.lane_pad_y
            };

            for &ni in local_nodes {
                let n = &d.nodes[ni];
                let (nw, nh) = sizes[ni];
                let cx = x + dim.w / 2.0;
                let cy = cursor_y + nh / 2.0;
                cursor_y += nh + cfg.node_gap_y;
                node_scenes.push(scene_node(n, cx, cy, nw, nh));
            }
        }
        AdvanceDirection::Horizontal => {
            let mut children_w: f64 = 0.0;
            if !lane.children.is_empty() {
                let cur_x = x + cfg.lane_title_h + cfg.lane_pad_x;
                let mut cur_y = y + cfg.lane_pad_y;
                for (c, c_dim) in lane.children.iter().zip(&dim.children) {
                    emit_lanes_and_nodes_rec(
                        c,
                        c_dim,
                        cur_x,
                        cur_y,
                        d,
                        lane_idx,
                        ordered_node_indices,
                        sizes,
                        cfg,
                        dir,
                        lane_scenes,
                        node_scenes,
                    );
                    cur_y += c_dim.h + cfg.lane_gap;
                    children_w = children_w.max(c_dim.w);
                }
            }

            // Direct nodes inside this lane
            let li = lane_idx[&lane.id];
            let local_nodes = &ordered_node_indices[li];
            let mut cursor_x = if lane.children.is_empty() {
                x + cfg.lane_title_h + cfg.lane_pad_x
            } else {
                x + cfg.lane_title_h + cfg.lane_pad_x + children_w + cfg.lane_pad_x
            };

            for &ni in local_nodes {
                let n = &d.nodes[ni];
                let (nw, nh) = sizes[ni];
                let cx = cursor_x + nw / 2.0;
                let cy = y + dim.h / 2.0;
                cursor_x += nw + cfg.node_gap_y;
                node_scenes.push(scene_node(n, cx, cy, nw, nh));
            }
        }
    }
}

// ------------------------------------------------------------------
// Main Layout
// ------------------------------------------------------------------

/// Compute the positioned geometry for an advance diagram.
pub fn layout(d: &AdvanceDiagram) -> AdvanceScene {
    layout_inner(d, true)
}

/// `layout` with the routing made optional. A caller that only wants to
/// know where the nodes ended up — the drag path does, to learn their
/// order — would otherwise pay for a full routing pass and throw it
/// away, which was nearly free before the channel grid and is not now.
fn layout_inner(d: &AdvanceDiagram, route: bool) -> AdvanceScene {
    let cfg = &d.config;
    let lane_idx = lane_index_map(d);
    let total_lanes_count = lane_idx.len();
    let sizes: Vec<(f64, f64)> = d.nodes.iter().map(node_size).collect();

    // Check if explicit coordinates are supplied on all nodes
    let has_explicit_coords =
        !d.nodes.is_empty() && d.nodes.iter().all(|n| n.x.is_some() && n.y.is_some());

    if has_explicit_coords {
        let node_scenes: Vec<AdvanceSceneNode> = d
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| scene_node(n, n.x.unwrap(), n.y.unwrap(), sizes[i].0, sizes[i].1))
            .collect();
        let (lane_scenes, width, height) = build_lanes_around_nodes(d, &node_scenes);
        let edge_scenes = if route {
            route_edges(d, &node_scenes, &lane_scenes, d.direction)
        } else {
            Vec::new()
        };
        return fit_canvas(
            AdvanceScene {
                width,
                height,
                title: d.title.clone(),
                description: d.description.clone(),
                direction: d.direction,
                style: d.style.clone(),
                lanes: lane_scenes,
                nodes: node_scenes,
                crossings: count_crossings(&edge_scenes),
                edges: edge_scenes,
            },
            cfg.margin,
        );
    }

    if d.direction == AdvanceDirection::Horizontal {
        // Horizontal Layout: Rows stacked top-to-bottom (with recursive nested lane support)
        let mut lane_node_lists: Vec<Vec<usize>> = vec![Vec::new(); total_lanes_count];
        for (i, n) in d.nodes.iter().enumerate() {
            let li = lane_idx[&n.lane];
            lane_node_lists[li].push(i);
        }
        let ordered_node_indices: Vec<Vec<usize>> = lane_node_lists
            .iter()
            .map(|list| order_lane_nodes(list, d, cfg))
            .collect();

        let mut root_lane_dims: Vec<LaneDim> = d
            .lanes
            .iter()
            .map(|l| compute_lane_dim_rec(l, &lane_idx, &lane_node_lists, &sizes, cfg, d.direction))
            .collect();

        // Equalize sibling lane widths within recursive sub-trees
        for dim in &mut root_lane_dims {
            equalize_sibling_widths(dim);
        }

        // Uniform width for root rows
        let max_root_w = root_lane_dims.iter().map(|d| d.w).fold(0.0_f64, f64::max);
        for dim in &mut root_lane_dims {
            dim.w = max_root_w;
        }

        let mut lane_scenes = Vec::new();
        let mut node_scenes = Vec::with_capacity(d.nodes.len());
        let mut cur_y = cfg.margin;
        let start_x = cfg.margin;

        for (lane, dim) in d.lanes.iter().zip(&root_lane_dims) {
            emit_lanes_and_nodes_rec(
                lane,
                dim,
                start_x,
                cur_y,
                d,
                &lane_idx,
                &ordered_node_indices,
                &sizes,
                cfg,
                d.direction,
                &mut lane_scenes,
                &mut node_scenes,
            );
            cur_y += dim.h + cfg.lane_gap;
        }

        let total_w = cfg.margin + max_root_w + cfg.margin;
        let total_h = if d.lanes.is_empty() {
            cfg.margin * 2.0
        } else {
            cur_y - cfg.lane_gap + cfg.margin
        };

        let edge_scenes = if route {
            route_edges(d, &node_scenes, &lane_scenes, d.direction)
        } else {
            Vec::new()
        };
        return AdvanceScene {
            width: total_w,
            height: total_h,
            title: d.title.clone(),
            description: d.description.clone(),
            direction: d.direction,
            style: d.style.clone(),
            lanes: lane_scenes,
            nodes: node_scenes,
            crossings: count_crossings(&edge_scenes),
            edges: edge_scenes,
        };
    }

    // Vertical Mode (with arbitrary recursive nesting support)
    let mut lane_node_lists: Vec<Vec<usize>> = vec![Vec::new(); total_lanes_count];
    for (i, n) in d.nodes.iter().enumerate() {
        let li = lane_idx[&n.lane];
        lane_node_lists[li].push(i);
    }
    let ordered_node_indices: Vec<Vec<usize>> = lane_node_lists
        .iter()
        .map(|list| order_lane_nodes(list, d, cfg))
        .collect();

    let mut top_dims: Vec<LaneDim> = d
        .lanes
        .iter()
        .map(|l| compute_lane_dim_rec(l, &lane_idx, &lane_node_lists, &sizes, cfg, d.direction))
        .collect();

    // Equalize top-level lane heights and recursively for children
    let max_top_h = top_dims.iter().map(|c| c.h).fold(0.0_f64, f64::max);
    for dim in &mut top_dims {
        dim.h = max_top_h;
        equalize_sibling_heights(dim);
    }

    let mut lane_scenes = Vec::new();
    let mut node_scenes = Vec::with_capacity(d.nodes.len());
    let mut cur_x = cfg.margin;
    for (lane, dim) in d.lanes.iter().zip(&top_dims) {
        emit_lanes_and_nodes_rec(
            lane,
            dim,
            cur_x,
            cfg.margin,
            d,
            &lane_idx,
            &ordered_node_indices,
            &sizes,
            cfg,
            d.direction,
            &mut lane_scenes,
            &mut node_scenes,
        );
        cur_x += dim.w + cfg.lane_gap;
    }

    let total_width = if d.lanes.is_empty() {
        cfg.margin * 2.0
    } else {
        cur_x - cfg.lane_gap + cfg.margin
    };
    let total_height = max_top_h + 2.0 * cfg.margin;

    let edge_scenes = if route {
        route_edges(d, &node_scenes, &lane_scenes, d.direction)
    } else {
        Vec::new()
    };

    AdvanceScene {
        width: total_width,
        height: total_height,
        title: d.title.clone(),
        description: d.description.clone(),
        direction: d.direction,
        style: d.style.clone(),
        lanes: lane_scenes,
        nodes: node_scenes,
        crossings: count_crossings(&edge_scenes),
        edges: edge_scenes,
    }
}

// ------------------------------------------------------------------
// SVG Renderer
// ------------------------------------------------------------------

const FONT_SIZE: u32 = 13;

fn shape_style(shape: Shape) -> (String, String) {
    let ss = crate::style::shape_style(shape);
    (ss.fill.to_string(), ss.stroke.to_string())
}

fn render_node(s: &mut String, n: &AdvanceSceneNode, text_color: &str) {
    let cx = n.x;
    let cy = n.y;
    let w = n.w;
    let h = n.h;
    // Every colour below is escaped on its way into an attribute: it may
    // come from a `style` line, from JSON, or from a caller assembling an
    // AdvanceScene by hand, and none of those are vetted.
    let (mut fill, mut stroke) = shape_style(n.shape);
    if let Some(v) = &n.style.fill {
        fill = escape(v);
    }
    if let Some(v) = &n.style.stroke {
        stroke = escape(v);
    }
    let sw = n.style.stroke_width.unwrap_or(1.6);
    let style = format!(
        "fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.1}\"",
        fill, stroke, sw
    );
    let label_color = crate::scene::style_attr(n.style.color.as_deref(), text_color);

    match n.shape {
        Shape::Rect | Shape::Rounded | Shape::Stadium => {
            let rx = match n.shape {
                Shape::Rounded => 9.0,
                Shape::Stadium => h / 2.0,
                _ => 3.0,
            };
            s.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{:.1}\" {}/>\n",
                cx - w / 2.0,
                cy - h / 2.0,
                w,
                h,
                rx,
                style
            ));
        }
        Shape::Circle => {
            s.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" {}/>\n",
                cx,
                cy,
                w / 2.0,
                style
            ));
        }
        Shape::Diamond => {
            s.push_str(&format!(
                "<polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" {}/>\n",
                cx,
                cy - h / 2.0,
                cx + w / 2.0,
                cy,
                cx,
                cy + h / 2.0,
                cx - w / 2.0,
                cy,
                style
            ));
        }
        Shape::DoubleCircle => {
            for r in [w / 2.0, w / 2.0 - 4.0] {
                s.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" {}/>\n",
                    cx, cy, r, style
                ));
            }
        }
        Shape::Cylinder => {
            let (l, r, t, b) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
            let ry = 8.0_f64.min(h / 4.0);
            s.push_str(&format!(
                "<path d=\"M {l:.1} {ty:.1} A {rx:.1} {ry:.1} 0 0 0 {r:.1} {ty:.1} \
                 L {r:.1} {by:.1} A {rx:.1} {ry:.1} 0 0 1 {l:.1} {by:.1} Z\" {style}/>\n\
                 <path d=\"M {l:.1} {ty:.1} A {rx:.1} {ry:.1} 0 0 1 {r:.1} {ty:.1}\" \
                 fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw:.1}\"/>\n",
                l = l,
                r = r,
                ty = t + ry,
                by = b - ry,
                rx = w / 2.0,
                ry = ry,
                stroke = stroke,
                sw = sw,
                style = style,
            ));
        }
        Shape::Subroutine => {
            let (l, t) = (cx - w / 2.0, cy - h / 2.0);
            s.push_str(&format!(
                "<rect x=\"{l:.1}\" y=\"{t:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"3\" {style}/>\n\
                 <line x1=\"{l1:.1}\" y1=\"{t:.1}\" x2=\"{l1:.1}\" y2=\"{b:.1}\" stroke=\"{stroke}\" stroke-width=\"{sw:.1}\"/>\n\
                 <line x1=\"{r1:.1}\" y1=\"{t:.1}\" x2=\"{r1:.1}\" y2=\"{b:.1}\" stroke=\"{stroke}\" stroke-width=\"{sw:.1}\"/>\n",
                l = l, t = t, w = w, h = h, b = t + h, l1 = l + 8.0, r1 = l + w - 8.0,
                stroke = stroke, sw = sw, style = style,
            ));
        }
        Shape::Hexagon => {
            let (l, r, t, b) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
            let k = 14.0_f64.min(w / 4.0);
            s.push_str(&format!(
                "<polygon points=\"{a:.1},{cy:.1} {b1:.1},{t:.1} {c:.1},{t:.1} {r:.1},{cy:.1} {c:.1},{b:.1} {b1:.1},{b:.1}\" {style}/>\n",
                a = l, b1 = l + k, c = r - k, r = r, t = t, b = b, cy = cy, style = style,
            ));
        }
        Shape::Parallelogram | Shape::ParallelogramAlt => {
            let (l, r, t, b) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
            let k = 14.0_f64.min(w / 4.0);
            let pts = if matches!(n.shape, Shape::Parallelogram) {
                format!(
                    "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                    l + k,
                    t,
                    r,
                    t,
                    r - k,
                    b,
                    l,
                    b
                )
            } else {
                format!(
                    "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                    l,
                    t,
                    r - k,
                    t,
                    r,
                    b,
                    l + k,
                    b
                )
            };
            s.push_str(&format!("<polygon points=\"{}\" {} />\n", pts, style));
        }
        Shape::StateStart | Shape::StateEnd | Shape::ForkBar => {
            s.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" {}/>\n",
                cx - w / 2.0,
                cy - h / 2.0,
                w,
                h,
                style
            ));
        }
    }

    // Label — centred, or in the top band when sub-elements sit below.
    let lines: Vec<&str> = n.label.split('\n').collect();
    let line_count = lines.len();
    let band_h = label_band_h(&n.label);
    let label_cy = if n.elements.is_empty() {
        cy
    } else {
        cy - h / 2.0 + band_h / 2.0
    };
    let start_y = if line_count == 1 {
        label_cy
    } else {
        label_cy - ((line_count - 1) as f64 * LINE_H) / 2.0
    };
    for (i, line) in lines.iter().enumerate() {
        let y = start_y + i as f64 * LINE_H;
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-size=\"{}\" fill=\"{}\">{}</text>\n",
            cx,
            y,
            FONT_SIZE,
            label_color,
            escape(line)
        ));
    }

    // Sub-elements: compartments drawn on the node body, innermost
    // last so nested ones paint over their parent.
    if !n.elements.is_empty() {
        let node_stroke_raw = n
            .style
            .stroke
            .clone()
            .unwrap_or_else(|| shape_style(n.shape).1);
        let node_text_raw = n.style.color.as_deref().unwrap_or(text_color).to_string();
        for (i, el) in n.elements.iter().enumerate() {
            let has_children = n.elements.iter().any(|c| c.parent == Some(i));
            let fill = crate::scene::style_attr(el.style.fill.as_deref(), "#ffffff");
            let stroke = crate::scene::style_attr(el.style.stroke.as_deref(), &node_stroke_raw);
            let color = crate::scene::style_attr(el.style.color.as_deref(), &node_text_raw);
            s.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
                 fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>\n",
                el.x - el.w / 2.0,
                el.y - el.h / 2.0,
                el.w,
                el.h,
                fill,
                stroke
            ));
            let el_lines: Vec<&str> = el.label.split('\n').collect();
            let el_band = label_band_h(&el.label);
            let el_cy = if has_children {
                el.y - el.h / 2.0 + el_band / 2.0
            } else {
                el.y
            };
            let el_start = el_cy - ((el_lines.len() - 1) as f64 * LINE_H) / 2.0;
            for (j, line) in el_lines.iter().enumerate() {
                s.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                     font-size=\"{}\" fill=\"{}\">{}</text>\n",
                    el.x,
                    el_start + j as f64 * LINE_H,
                    FONT_SIZE,
                    color,
                    escape(line)
                ));
            }
        }
    }
}

/// Render an advance scene to an SVG string.
pub fn to_svg(sc: &AdvanceScene) -> String {
    to_svg_with(sc, &SvgOptions::default())
}

/// Render with explicit viewport options.
pub fn to_svg_with(sc: &AdvanceScene, opts: &SvgOptions) -> String {
    let mut s = String::new();
    let svg_title = sc.title.as_deref().unwrap_or("Advance diagram");
    svg_open(&mut s, sc.width, sc.height, FONT_SIZE, svg_title, opts);

    if let Some(desc) = &sc.description {
        s.push_str(&format!("<desc>{}</desc>\n", escape(desc)));
    }

    // One arrowhead marker per resolved edge color (first-seen order), so
    // a styled edge's arrowhead matches its stroke instead of the global.
    // Ids carry a hash of the scene: a counter is unique only within one
    // process, so two diagrams rendered separately and inlined on one
    // page both emitted `advance-arrow-1` and the second one's arrows
    // took the first one's colours.
    let key = scene_key(sc);
    let mut markers: Vec<(String, String)> = Vec::new();
    for e in &sc.edges {
        if matches!(e.kind, EdgeKind::Invisible) || !e.kind.has_arrow() {
            continue;
        }
        let color = crate::scene::style_attr(e.style.color.as_deref(), &sc.style.edge_color);
        if !markers.iter().any(|(c, _)| *c == color) {
            let id = format!("advance-arrow-{}-{}", key, markers.len() + 1);
            markers.push((color, id));
        }
    }
    let mut defs = String::new();
    for (color, id) in &markers {
        defs.push_str(&format!(
            "<marker id=\"{}\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" \
             markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
             <path d=\"M 0 1 L 9 5 L 0 9 z\" fill=\"{}\"/></marker>",
            id, color
        ));
    }
    if !defs.is_empty() {
        s.push_str(&format!("<defs>{}</defs>\n", defs));
    }

    // Lane backgrounds
    for lane in &sc.lanes {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"2\" rx=\"8\"/>\n",
            lane.x,
            lane.y,
            lane.w,
            lane.h,
            escape(&sc.style.lane_fill),
            escape(&sc.style.lane_stroke)
        ));
        let (tx, ty) = if sc.direction == AdvanceDirection::Horizontal {
            (lane.x + 12.0, lane.y + lane.h / 2.0)
        } else {
            (
                lane.x + DEFAULT_LANE_PAD_X,
                lane.y + DEFAULT_LANE_TITLE_H - 6.0,
            )
        };
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{}\" font-weight=\"bold\" \
             fill=\"{}\">{}</text>\n",
            tx,
            ty,
            FONT_SIZE,
            escape(&sc.style.text_color),
            escape(&lane.title)
        ));
    }

    // Edges
    for e in &sc.edges {
        if matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        let color = crate::scene::style_attr(e.style.color.as_deref(), &sc.style.edge_color);
        // An explicit style dash wins over the kind-based dashed style;
        // otherwise Thick stays solid, Dotted stays dashed.
        let styled_dash = e.style.dash.as_deref().map(escape);
        let dash = styled_dash.as_deref().or(match e.kind {
            EdgeKind::Dotted | EdgeKind::DottedOpen => Some("5 4"),
            _ => None,
        });
        let dash_attr = match dash {
            Some(d) => format!(" stroke-dasharray=\"{}\"", d),
            None => String::new(),
        };
        let sw = e.style.stroke_width.unwrap_or(match e.kind {
            EdgeKind::Thick | EdgeKind::ThickOpen => 3.4,
            _ => 1.7,
        });
        let marker = if e.kind.has_arrow() {
            markers
                .iter()
                .find(|(c, _)| *c == color)
                .map(|(_, id)| format!(" marker-end=\"url(#{})\"", id))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let d = if e.points.len() >= 2 {
            let mut d = format!("M {:.1} {:.1}", e.points[0].0, e.points[0].1);
            for i in 1..e.points.len() {
                d.push_str(&format!(" L {:.1} {:.1}", e.points[i].0, e.points[i].1));
            }
            d
        } else {
            String::new()
        };
        s.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\"{}{}/>\n",
            d, color, sw, dash_attr, marker
        ));
        if let Some(label) = &e.label {
            // Route-time label_pos (collision-dodged) wins; otherwise fall
            // back to the historical inline placement.
            let (lx, ly) = if let Some(pos) = e.label_pos {
                pos
            } else {
                let mut best_vert: Option<(f64, f64, f64)> = None;
                for i in 0..e.points.len().saturating_sub(1) {
                    let (x1, y1) = e.points[i];
                    let (x2, y2) = e.points[i + 1];
                    if (x1 - x2).abs() < f64::EPSILON {
                        let len = (y2 - y1).abs();
                        if best_vert.map(|(_, _, l)| len > l).unwrap_or(true) {
                            best_vert = Some((x1, (y1 + y2) / 2.0, len));
                        }
                    }
                }
                if let Some((x, y, len)) = best_vert {
                    if len >= 22.0 {
                        (x + 8.0, y)
                    } else {
                        let mid = e.points[e.points.len() / 2];
                        (mid.0, mid.1)
                    }
                } else {
                    let mid = e.points[e.points.len() / 2];
                    (mid.0, mid.1)
                }
            };
            let lw = text_width(label) + 14.0;
            let label_fill =
                crate::scene::style_attr(e.style.label_fill.as_deref(), &sc.style.label_fill);
            s.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"18\" \
                 fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" rx=\"3\"/>\n",
                lx - lw / 2.0,
                ly - 9.0,
                lw,
                label_fill,
                escape(&sc.style.lane_stroke)
            ));
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                 font-size=\"{}\" fill=\"{}\">{}</text>\n",
                lx,
                ly,
                FONT_SIZE,
                escape(&sc.style.text_color),
                escape(label)
            ));
        }
    }

    // Nodes
    for n in &sc.nodes {
        render_node(&mut s, n, &sc.style.text_color);
    }

    s.push_str("</svg>\n");
    s
}

// ------------------------------------------------------------------
// Public entry points
// ------------------------------------------------------------------

/// Parse an advance diagram from JSON, lay it out, and render to SVG.
pub fn render_advance_svg(source: &str) -> Result<String, AdvanceError> {
    let diagram = AdvanceDiagram::parse(source)?;
    let scene = layout(&diagram);
    Ok(to_svg(&scene))
}

/// Parse an advance diagram from JSON and return its positioned geometry.
pub fn layout_advance(source: &str) -> Result<AdvanceScene, AdvanceError> {
    let diagram = AdvanceDiagram::parse(source)?;
    Ok(layout(&diagram))
}

fn build_lanes_around_nodes(
    d: &AdvanceDiagram,
    nodes: &[AdvanceSceneNode],
) -> (Vec<AdvanceSceneLane>, f64, f64) {
    let lane_idx = lane_index_map(d);
    let mut flat_lanes = Vec::new();
    collect_all_lanes(&d.lanes, &mut flat_lanes);
    let mut bounds: Vec<Option<(f64, f64, f64, f64)>> = vec![None; flat_lanes.len()];

    for n in nodes {
        let li = lane_idx[&n.lane];
        let l = n.x - n.w / 2.0;
        let r = n.x + n.w / 2.0;
        let t = n.y - n.h / 2.0;
        let b = n.y + n.h / 2.0;
        bounds[li] = Some(match bounds[li] {
            None => (l, t, r, b),
            Some((cl, ct, cr, cb)) => (cl.min(l), ct.min(t), cr.max(r), cb.max(b)),
        });
    }

    let mut lanes = Vec::with_capacity(flat_lanes.len());
    let mut max_right: f64 = 0.0;
    let mut max_bottom: f64 = 0.0;

    for (i, lane) in flat_lanes.iter().enumerate() {
        let (l, t, r, b) = bounds[i].unwrap_or((
            d.config.margin,
            d.config.margin,
            d.config.margin + 120.0,
            d.config.margin + 120.0,
        ));
        let x = l - d.config.lane_pad_x;
        let y = (t - d.config.lane_title_h - d.config.lane_pad_y).min(d.config.margin);
        let w = (r - x + d.config.lane_pad_x).max(120.0);
        let h =
            (b - y + d.config.lane_pad_y).max(d.config.lane_title_h + 2.0 * d.config.lane_pad_y);

        max_right = max_right.max(x + w);
        max_bottom = max_bottom.max(y + h);

        lanes.push(AdvanceSceneLane {
            id: lane.id.clone(),
            title: lane.title.clone(),
            x,
            y,
            w,
            h,
        });
    }

    (
        lanes,
        max_right + d.config.margin,
        max_bottom + d.config.margin,
    )
}

fn validate_positions(d: &AdvanceDiagram, positions: &[f64]) -> Result<(), AdvanceError> {
    if positions.len() != d.nodes.len() * 2 {
        return Err(adv_err(format!(
            "expected {} coordinates for {} nodes, got {}",
            d.nodes.len() * 2,
            d.nodes.len(),
            positions.len()
        )));
    }
    if let Some(i) = positions.iter().position(|v| !v.is_finite()) {
        return Err(adv_err(format!("position[{}] is not a finite number", i)));
    }
    Ok(())
}

/// Place nodes at caller-supplied centres.
///
/// `positions` arrive in the order [`layout_advance`] emitted its nodes
/// — lane order, not declaration order — because that is the array a
/// host reads, drags one node in, and hands back. Taking the order from
/// [`layout`] itself is the only way it cannot drift from what the
/// caller was given; mapping `d.nodes` directly made two nodes in
/// different lanes trade places.
fn place_nodes_at_positions(d: &AdvanceDiagram, positions: &[f64]) -> Vec<AdvanceSceneNode> {
    let by_id: std::collections::HashMap<&str, &AdvanceNode> =
        d.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    layout_inner(d, false)
        .nodes
        .iter()
        .enumerate()
        .map(|(i, placed)| {
            let n = by_id[placed.id.as_str()];
            let (w, h) = node_size(n);
            scene_node(n, positions[i * 2], positions[i * 2 + 1], w, h)
        })
        .collect()
}

fn fit_canvas(mut sc: AdvanceScene, pad: f64) -> AdvanceScene {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    for l in &sc.lanes {
        min_x = min_x.min(l.x);
        min_y = min_y.min(l.y);
    }
    for n in &sc.nodes {
        min_x = min_x.min(n.x - n.w / 2.0);
        min_y = min_y.min(n.y - n.h / 2.0);
    }
    let dx = (pad - min_x).max(0.0);
    let dy = (pad - min_y).max(0.0);
    if dx > 0.0 || dy > 0.0 {
        for l in &mut sc.lanes {
            l.x += dx;
            l.y += dy;
        }
        for n in &mut sc.nodes {
            n.x += dx;
            n.y += dy;
            for el in &mut n.elements {
                el.x += dx;
                el.y += dy;
            }
            for a in &mut n.anchors {
                a.x += dx;
                a.y += dy;
            }
        }
        for e in &mut sc.edges {
            for p in &mut e.points {
                p.0 += dx;
                p.1 += dy;
            }
            e.from_point.0 += dx;
            e.from_point.1 += dy;
            e.to_point.0 += dx;
            e.to_point.1 += dy;
            if let Some(lp) = e.label_pos.as_mut() {
                lp.0 += dx;
                lp.1 += dy;
            }
        }
        sc.width += dx;
        sc.height += dy;
    }
    sc
}

/// Parse advance JSON, place nodes at caller-provided centre positions
/// (flat `[x0, y0, x1, y1, ...]` in the same order as the `nodes`
/// array emitted by [`layout_advance`]), recompute lane boxes and edge
/// routing, and render to SVG.
pub fn render_advance_routed(source: &str, positions: &[f64]) -> Result<String, AdvanceError> {
    let d = AdvanceDiagram::parse(source)?;
    validate_positions(&d, positions)?;
    let nodes = place_nodes_at_positions(&d, positions);
    let (lanes, width, height) = build_lanes_around_nodes(&d, &nodes);
    let edges = route_edges(&d, &nodes, &lanes, d.direction);
    let scene = fit_canvas(
        AdvanceScene {
            width,
            height,
            title: d.title.clone(),
            description: d.description.clone(),
            direction: d.direction,
            style: d.style.clone(),
            lanes,
            nodes,
            crossings: count_crossings(&edges),
            edges,
        },
        d.config.margin,
    );
    Ok(to_svg(&scene))
}

fn build_lanes_with_widths(
    d: &AdvanceDiagram,
    nodes: &[AdvanceSceneNode],
    lane_widths: &[f64],
    margin: f64,
    gap: f64,
) -> (Vec<AdvanceSceneLane>, f64, f64) {
    let lane_idx = top_level_lane_map(d);
    let mut lane_scenes = Vec::with_capacity(d.lanes.len());
    let mut x = margin;

    let mut lane_heights: Vec<f64> = Vec::with_capacity(d.lanes.len());
    for i in 0..d.lanes.len() {
        let max_bottom = nodes
            .iter()
            .filter(|n| lane_idx.get(n.lane.as_str()).copied() == Some(i))
            .map(|n| n.y + n.h / 2.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let h = if max_bottom.is_finite() {
            (max_bottom - margin + d.config.lane_pad_y)
                .max(d.config.lane_title_h + 2.0 * d.config.lane_pad_y)
        } else {
            d.config.lane_title_h + 2.0 * d.config.lane_pad_y
        };
        lane_heights.push(h);
    }
    let lane_h = lane_heights.iter().fold(0.0_f64, |m, h| m.max(*h));

    for (i, lane) in d.lanes.iter().enumerate() {
        let w = lane_widths[i];
        lane_scenes.push(AdvanceSceneLane {
            id: lane.id.clone(),
            title: lane.title.clone(),
            x,
            y: margin,
            w,
            h: lane_h,
        });

        x += w + gap;
    }

    let total_width = if d.lanes.is_empty() {
        margin * 2.0
    } else {
        x - gap + margin
    };
    let total_height = lane_h + 2.0 * margin;

    (lane_scenes, total_width, total_height)
}

/// Parse advance JSON, place nodes at caller-provided centre positions,
/// and render to SVG using caller-provided lane widths, margin, and gap.
///
/// NOTE: Non-goal combinations like horizontal direction return an error.
pub fn render_advance_routed_with_lanes(
    source: &str,
    positions: &[f64],
    lane_widths: &[f64],
    margin: f64,
    gap: f64,
) -> Result<String, AdvanceError> {
    let d = AdvanceDiagram::parse(source)?;
    if d.direction == AdvanceDirection::Horizontal {
        return Err(adv_err(
            "render_advance_routed_with_lanes is not supported in horizontal direction",
        ));
    }
    validate_positions(&d, positions)?;

    if lane_widths.len() != d.lanes.len() {
        return Err(adv_err(format!(
            "expected {} lane widths, got {}",
            d.lanes.len(),
            lane_widths.len()
        )));
    }
    if let Some(i) = lane_widths.iter().position(|w| !w.is_finite() || *w <= 0.0) {
        return Err(adv_err(format!(
            "lane_widths[{}] must be a positive finite number",
            i
        )));
    }
    if !margin.is_finite() || margin < 0.0 {
        return Err(adv_err("margin must be a non-negative finite number"));
    }
    if !gap.is_finite() || gap < 0.0 {
        return Err(adv_err("gap must be a non-negative finite number"));
    }

    let nodes = place_nodes_at_positions(&d, positions);
    let (lanes, width, height) = build_lanes_with_widths(&d, &nodes, lane_widths, margin, gap);
    let edges = route_edges(&d, &nodes, &lanes, d.direction);
    let scene = fit_canvas(
        AdvanceScene {
            width,
            height,
            title: d.title.clone(),
            description: d.description.clone(),
            direction: d.direction,
            style: d.style.clone(),
            lanes,
            nodes,
            crossings: count_crossings(&edges),
            edges,
        },
        margin,
    );
    Ok(to_svg(&scene))
}

/// Render a text-based swimlane diagram directly to an SVG string.
pub fn render_advance_text_svg(source: &str) -> Result<String, AdvanceError> {
    let d = AdvanceDiagram::parse_text(source)?;
    let scene = layout(&d);
    Ok(to_svg(&scene))
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_json() -> &'static str {
        r#"{
            "lanes": [
                {"id": "dev", "title": "Development"},
                {"id": "qa", "title": "QA"}
            ],
            "nodes": [
                {"id": "a", "label": "Design", "lane": "dev"},
                {"id": "b", "label": "Code", "lane": "dev"},
                {"id": "c", "label": "Test", "lane": "qa", "shape": "diamond"},
                {"id": "d", "label": "Sign off", "lane": "qa", "shape": "stadium"}
            ],
            "edges": [
                {"from": "a", "to": "b", "label": "implements"},
                {"from": "b", "to": "c"},
                {"from": "c", "to": "d"}
            ]
        }"#
    }

    #[test]
    fn parse_sample() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        assert_eq!(d.lanes.len(), 2);
        assert_eq!(d.nodes.len(), 4);
        assert_eq!(d.edges.len(), 3);
    }

    #[test]
    fn layout_has_geometry() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let sc = layout(&d);
        assert!(sc.width > 0.0);
        assert!(sc.height > 0.0);
        assert_eq!(sc.lanes.len(), 2);
        assert_eq!(sc.nodes.len(), 4);
        assert_eq!(sc.edges.len(), 3);
    }

    #[test]
    fn render_produces_svg() {
        let svg = render_advance_svg(sample_json()).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Development"));
        assert!(svg.contains("Design"));
        assert!(svg.contains("implements"));
    }

    #[test]
    fn invalid_json_rejected() {
        assert!(render_advance_svg("not json").is_err());
    }

    #[test]
    fn unknown_lane_rejected() {
        let src = r#"{"lanes":[{"id":"x"}],"nodes":[{"id":"n","lane":"y"}]}"#;
        assert!(AdvanceDiagram::parse(src).is_err());
    }

    #[test]
    fn unknown_edge_node_rejected() {
        let src = r#"{"lanes":[{"id":"x"}],"nodes":[{"id":"n","lane":"x"}],"edges":[{"from":"n","to":"z"}]}"#;
        assert!(AdvanceDiagram::parse(src).is_err());
    }

    #[test]
    fn shape_and_edge_kind_parity() {
        let src = r#"{
            "lanes":[{"id":"x"}],
            "nodes":[
                {"id":"a","lane":"x","shape":"doublecircle"},
                {"id":"b","lane":"x","shape":"statestart"},
                {"id":"c","lane":"x","shape":"stateend"},
                {"id":"d","lane":"x","shape":"forkbar"},
                {"id":"e","lane":"x","shape":"parallelogramalt"}
            ],
            "edges":[
                {"from":"a","to":"b","kind":"dottedopen"},
                {"from":"b","to":"c","kind":"thickopen"},
                {"from":"c","to":"d","kind":"invisible"}
            ]
        }"#;
        let d = AdvanceDiagram::parse(src).unwrap();
        assert_eq!(d.nodes[0].shape, Shape::DoubleCircle);
        assert_eq!(d.nodes[1].shape, Shape::StateStart);
        assert_eq!(d.nodes[2].shape, Shape::StateEnd);
        assert_eq!(d.nodes[3].shape, Shape::ForkBar);
        assert_eq!(d.nodes[4].shape, Shape::ParallelogramAlt);
        assert_eq!(d.edges[0].kind, EdgeKind::DottedOpen);
        assert_eq!(d.edges[1].kind, EdgeKind::ThickOpen);
        assert_eq!(d.edges[2].kind, EdgeKind::Invisible);
    }

    #[test]
    fn marker_ids_are_stable_per_diagram_and_differ_between_diagrams() {
        let first_id = |svg: &str| {
            let i = svg.find("id=\"advance-arrow-").expect("a marker");
            svg[i..].split('"').nth(1).unwrap().to_string()
        };
        // Same input, same ids: a counter made these differ, which broke
        // byte-identical output.
        let a = render_advance_svg(sample_json()).unwrap();
        let b = render_advance_svg(sample_json()).unwrap();
        assert_eq!(a, b, "the same diagram must render byte-identically");

        // Different diagrams, different ids — the counter restarted at 1
        // in every process, so two SVGs inlined on one page collided and
        // the second one's arrows took the first one's colours.
        let other = render_advance_svg(
            r#"{"lanes":[{"id":"z","title":"Z"}],
                "nodes":[{"id":"p","lane":"z"},{"id":"q","lane":"z"}],
                "edges":[{"from":"p","to":"q"}]}"#,
        )
        .unwrap();
        assert_ne!(first_id(&a), first_id(&other));

        // Several colours in one diagram stay distinct and are numbered
        // within their own namespace.
        let multi = render_advance_svg(
            r##"{"lanes":[{"id":"l","title":"L"}],
                "nodes":[{"id":"a","lane":"l"},{"id":"b","lane":"l"},{"id":"c","lane":"l"}],
                "edges":[{"from":"a","to":"b","style":{"color":"#188038"}},
                         {"from":"b","to":"c","style":{"color":"#b00020"}}]}"##,
        )
        .unwrap();
        let key = first_id(&multi).rsplit_once('-').unwrap().0.to_string();
        assert!(multi.contains(&format!("id=\"{key}-1\"")), "{multi}");
        assert!(multi.contains(&format!("id=\"{key}-2\"")), "{multi}");
        assert_eq!(multi.matches("<marker").count(), 2);
    }

    #[test]
    fn accessibility_title_and_description() {
        let src = r#"{
            "title": "Deployment Pipeline",
            "description": "Overview of stages",
            "lanes":[{"id":"x"}],
            "nodes":[{"id":"a","lane":"x"}]
        }"#;
        let svg = render_advance_svg(src).unwrap();
        assert!(svg.contains("<title>Deployment Pipeline</title>"));
        assert!(svg.contains("<desc>Overview of stages</desc>"));
    }

    #[test]
    fn explicit_geometry_and_serialization_roundtrip() {
        let src = concat!(
            "{\n",
            "  \"title\": \"T\",\n",
            "  \"style\": {\"lane_fill\": \"#112233\"},\n",
            "  \"config\": {\"margin\": 15.0, \"order\": \"topology\"},\n",
            "  \"lanes\": [{\"id\": \"l1\", \"title\": \"L1\"}],\n",
            "  \"nodes\": [{\"id\": \"n1\", \"label\": \"N1\", \"lane\": \"l1\", \"shape\": \"rect\", \"x\": 100.0, \"y\": 200.0, \"w\": 50.0, \"h\": 40.0}],\n",
            "  \"edges\": [{\"from\": \"n1\", \"to\": \"n1\", \"kind\": \"arrow\"}]\n",
            "}"
        );
        let d = AdvanceDiagram::parse(src).unwrap();
        let json_out = to_json(&d);
        let d2 = AdvanceDiagram::parse(&json_out).unwrap();
        assert_eq!(d.title, d2.title);
        assert_eq!(d.nodes, d2.nodes);
        assert_eq!(d.style.lane_fill, d2.style.lane_fill);
        assert_eq!(d.config.margin, d2.config.margin);
        assert_eq!(d.config.order, d2.config.order);
    }

    #[test]
    fn topology_order_reorders_chain() {
        let src = r#"{
            "config":{"order":"topology"},
            "lanes":[{"id":"l"}],
            "nodes":[
                {"id":"c","label":"C","lane":"l"},
                {"id":"a","label":"A","lane":"l"},
                {"id":"b","label":"B","lane":"l"}
            ],
            "edges":[
                {"from":"a","to":"b"},
                {"from":"b","to":"c"}
            ]
        }"#;
        let d = AdvanceDiagram::parse(src).unwrap();
        let sc = layout(&d);
        let ya = sc.nodes.iter().find(|n| n.id == "a").unwrap().y;
        let yb = sc.nodes.iter().find(|n| n.id == "b").unwrap().y;
        let yc = sc.nodes.iter().find(|n| n.id == "c").unwrap().y;
        assert!(ya < yb, "expected A before B in topology mode");
        assert!(yb < yc, "expected B before C in topology mode");
    }

    #[test]
    fn nested_lanes_containment_and_rendering() {
        let src = r#"{
            "lanes":[
                {
                    "id":"parent",
                    "title":"Parent",
                    "children":[
                        {"id":"c1","title":"Child 1"},
                        {"id":"c2","title":"Child 2"}
                    ]
                }
            ],
            "nodes":[
                {"id":"n1","lane":"c1"},
                {"id":"n2","lane":"c2"}
            ]
        }"#;
        let d = AdvanceDiagram::parse(src).unwrap();
        let sc = layout(&d);
        assert_eq!(sc.lanes.len(), 3);
        let parent = sc.lanes.iter().find(|l| l.id == "parent").unwrap();
        let c1 = sc.lanes.iter().find(|l| l.id == "c1").unwrap();
        let c2 = sc.lanes.iter().find(|l| l.id == "c2").unwrap();
        assert!(c1.x >= parent.x);
        assert!(c1.y >= parent.y);
        assert!(c2.x + c2.w <= parent.x + parent.w + 1e-9);
        assert!(c2.y + c2.h <= parent.y + parent.h + 1e-9);
        let svg = to_svg(&sc);
        assert!(svg.contains("Parent"));
        assert!(svg.contains("Child 1"));
        assert!(svg.contains("Child 2"));
    }

    #[test]
    fn horizontal_swimlanes() {
        let src = r#"{
            "direction": "horizontal",
            "lanes":[{"id":"l1","title":"Row 1"},{"id":"l2","title":"Row 2"}],
            "nodes":[
                {"id":"a","lane":"l1"},
                {"id":"b","lane":"l1"},
                {"id":"c","lane":"l2"}
            ],
            "edges":[{"from":"a","to":"b"},{"from":"a","to":"c"}]
        }"#;
        let d = AdvanceDiagram::parse(src).unwrap();
        let sc = layout(&d);
        assert_eq!(sc.direction, AdvanceDirection::Horizontal);
        let l1 = sc.lanes.iter().find(|l| l.id == "l1").unwrap();
        let l2 = sc.lanes.iter().find(|l| l.id == "l2").unwrap();
        assert_eq!(l1.x, l2.x);
        assert!(l1.y < l2.y);
        let na = sc.nodes.iter().find(|n| n.id == "a").unwrap();
        let nb = sc.nodes.iter().find(|n| n.id == "b").unwrap();
        assert_eq!(na.y, nb.y);
        assert!(na.x < nb.x);
        let svg = to_svg(&sc);
        assert!(svg.starts_with("<svg"));
    }

    #[test]
    fn horizontal_nested_lanes_layout_and_containment() {
        let src = r#"{
            "direction": "horizontal",
            "lanes": [
                {
                    "id": "dept",
                    "title": "Engineering",
                    "children": [
                        {"id": "fe", "title": "Frontend"},
                        {"id": "be", "title": "Backend"}
                    ]
                }
            ],
            "nodes": [
                {"id": "ui", "label": "Web App", "lane": "fe"},
                {"id": "api", "label": "REST API", "lane": "be"}
            ],
            "edges": [
                {"from": "ui", "to": "api", "label": "calls"}
            ]
        }"#;
        let d = AdvanceDiagram::parse(src).unwrap();
        let sc = layout(&d);
        assert_eq!(sc.lanes.len(), 3);
        let parent = sc.lanes.iter().find(|l| l.id == "dept").unwrap();
        let fe = sc.lanes.iter().find(|l| l.id == "fe").unwrap();
        let be = sc.lanes.iter().find(|l| l.id == "be").unwrap();

        assert!(fe.y >= parent.y);
        assert!(be.y >= fe.y + fe.h);
        assert!(parent.h >= fe.h + be.h);
    }

    #[test]
    fn routed_with_lanes_rejects_horizontal() {
        let src = r#"{"direction":"horizontal","lanes":[{"id":"l"}],"nodes":[]}"#;
        assert!(render_advance_routed_with_lanes(src, &[], &[100.0], 10.0, 10.0).is_err());
    }

    #[test]
    fn routed_renders_with_custom_positions() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let auto = layout(&d);
        let pos: Vec<f64> = auto.nodes.iter().flat_map(|n| [n.x + 10.0, n.y]).collect();
        let svg = render_advance_routed(sample_json(), &pos).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Code"));
        assert!(svg.contains("implements"));
    }

    #[test]
    fn routed_rejects_bad_position_length() {
        assert!(render_advance_routed(sample_json(), &[1.0, 2.0]).is_err());
    }

    #[test]
    fn routed_rejects_non_finite_positions() {
        assert!(render_advance_routed(sample_json(), &[f64::NAN; 8]).is_err());
        assert!(render_advance_routed(sample_json(), &[f64::INFINITY; 8]).is_err());
    }

    #[test]
    fn routed_with_lanes_renders_svg() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let auto = layout(&d);
        let pos: Vec<f64> = auto.nodes.iter().flat_map(|n| [n.x, n.y]).collect();
        let svg =
            render_advance_routed_with_lanes(sample_json(), &pos, &[200.0, 200.0], 24.0, 40.0)
                .unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Development"));
        assert!(svg.contains("QA"));
    }

    #[test]
    fn routed_with_lanes_uses_caller_widths() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let auto = layout(&d);
        let pos: Vec<f64> = auto.nodes.iter().flat_map(|n| [n.x, n.y]).collect();
        let sc = {
            let svg =
                render_advance_routed_with_lanes(sample_json(), &pos, &[300.0, 150.0], 10.0, 20.0)
                    .unwrap();
            let start = svg.find("width=\"").unwrap() + 7;
            let end = svg[start..].find('"').unwrap() + start;
            svg[start..end].parse::<f64>().unwrap()
        };
        assert!((sc - 490.0).abs() < 1.0, "expected width ~490, got {}", sc);
    }

    #[test]
    fn routed_with_lanes_rejects_wrong_width_count() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let pos: Vec<f64> = d.nodes.iter().flat_map(|_| [0.0, 0.0]).collect();
        assert!(
            render_advance_routed_with_lanes(sample_json(), &pos, &[200.0], 24.0, 40.0).is_err()
        );
    }

    #[test]
    fn routed_with_lanes_rejects_non_positive_width() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let pos: Vec<f64> = d.nodes.iter().flat_map(|_| [0.0, 0.0]).collect();
        assert!(
            render_advance_routed_with_lanes(sample_json(), &pos, &[200.0, -10.0], 24.0, 40.0)
                .is_err()
        );
        assert!(render_advance_routed_with_lanes(
            sample_json(),
            &pos,
            &[200.0, f64::NAN],
            24.0,
            40.0
        )
        .is_err());
    }

    #[test]
    fn routed_with_lanes_rejects_bad_margin_gap() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let pos: Vec<f64> = d.nodes.iter().flat_map(|_| [0.0, 0.0]).collect();
        assert!(
            render_advance_routed_with_lanes(sample_json(), &pos, &[200.0, 200.0], -1.0, 40.0)
                .is_err()
        );
        assert!(
            render_advance_routed_with_lanes(sample_json(), &pos, &[200.0, 200.0], 24.0, -1.0)
                .is_err()
        );
    }

    fn base_positions(d: &AdvanceDiagram) -> Vec<f64> {
        let sc = layout(d);
        let mut pos = Vec::with_capacity(d.nodes.len() * 2);
        for n in &sc.nodes {
            pos.push(n.x);
            pos.push(n.y);
        }
        pos
    }

    #[test]
    fn routed_shifts_content_dragged_above_or_left_of_canvas_into_view() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let mut positions = base_positions(&d);
        positions[0] = -500.0;
        positions[1] = -800.0;
        let nodes = place_nodes_at_positions(&d, &positions);
        let (lanes, width, height) = build_lanes_around_nodes(&d, &nodes);
        let edges = route_edges(&d, &nodes, &lanes, d.direction);
        let sc = fit_canvas(
            AdvanceScene {
                width,
                height,
                title: None,
                description: None,
                direction: d.direction,
                style: d.style.clone(),
                lanes,
                nodes,
                crossings: count_crossings(&edges),
                edges,
            },
            d.config.margin,
        );
        assert!(sc
            .nodes
            .iter()
            .all(|n| n.y - n.h / 2.0 >= d.config.margin - 1e-9));
        assert!(sc
            .nodes
            .iter()
            .all(|n| n.x - n.w / 2.0 >= d.config.margin - 1e-9));
        assert!(sc
            .lanes
            .iter()
            .all(|l| l.y >= d.config.margin - 1e-9 && l.x >= d.config.margin - 1e-9));
        assert!(sc
            .edges
            .iter()
            .all(|e| e.points.iter().all(|p| p.1 >= d.config.margin - 1e-9)));
        let max_bottom = sc
            .nodes
            .iter()
            .map(|n| n.y + n.h / 2.0)
            .fold(0.0_f64, f64::max);
        assert!(sc.height >= max_bottom + d.config.margin - 1e-9);
    }

    // ---- Text DSL ----

    fn text_diagram(src: &str) -> AdvanceDiagram {
        AdvanceDiagram::parse_text(src).unwrap()
    }

    fn node_lane<'a>(d: &'a AdvanceDiagram, id: &str) -> &'a str {
        d.nodes.iter().find(|n| n.id == id).unwrap().lane.as_str()
    }

    #[test]
    fn text_parses_flat_lanes_and_edges() {
        let d = text_diagram(
            "swimlane horizontal\n\
             lane l1 \"Sales\"\n\
             \x20 a([Start])\n\
             \x20 b[Prepare]\n\
             lane l2 \"QA\"\n\
             \x20 c{Check}\n\
             \n\
             a --> b\n\
             b -->|done| c\n",
        );
        assert_eq!(d.direction, AdvanceDirection::Horizontal);
        assert_eq!(d.lanes.len(), 2);
        assert_eq!(d.lanes[0].title, "Sales");
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.edges.len(), 2);
        assert_eq!(d.edges[1].label.as_deref(), Some("done"));
    }

    #[test]
    fn text_nested_lane_blocks() {
        let d = text_diagram(
            "lane top \"Top\" {\n\
             \x20 a\n\
             \x20 lane sub \"Sub\" {\n\
             \x20   b\n\
             \x20 }\n\
             \x20 c\n\
             }\n\
             lane other \"Other\"\n\
             \x20 d\n",
        );
        assert_eq!(d.lanes.len(), 2);
        assert_eq!(d.lanes[0].id, "top");
        assert_eq!(d.lanes[0].children.len(), 1);
        assert_eq!(d.lanes[0].children[0].id, "sub");
        assert_eq!(d.lanes[0].children[0].children.len(), 0);
        assert_eq!(d.lanes[1].id, "other");
        assert_eq!(node_lane(&d, "a"), "top");
        assert_eq!(node_lane(&d, "b"), "sub");
        assert_eq!(node_lane(&d, "c"), "top");
        assert_eq!(node_lane(&d, "d"), "other");
    }

    #[test]
    fn text_unbalanced_braces() {
        assert!(AdvanceDiagram::parse_text("lane a {\n  x\n}\n}").is_err());
        assert!(AdvanceDiagram::parse_text("lane a {\n  x\n").is_err());
        let err = AdvanceDiagram::parse_text("lane a {\n  x\n").unwrap_err();
        assert!(err.message.contains("never closed"));
    }

    #[test]
    fn text_nested_sibling_order_preserved() {
        // Siblings under one parent keep declaration order in the tree
        // (the flat->tree assembly must not reverse them).
        let d = text_diagram(
            "lane top \"Top\" {\n\
             \x20 lane a1 \"A1\" {\n\
             \x20   x\n\
             \x20 }\n\
             \x20 lane a2 \"A2\" {\n\
             \x20   y\n\
             \x20 }\n\
             }\n",
        );
        assert_eq!(d.lanes.len(), 1);
        let children: Vec<&str> = d.lanes[0].children.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(children, vec!["a1", "a2"]);
        // And the layout places A1 before A2 left-to-right.
        let sc = layout(&d);
        let x = |id: &str| {
            sc.lanes
                .iter()
                .find(|l| l.id == id)
                .map(|l| l.x)
                .unwrap_or(f64::NAN)
        };
        assert!(x("a1") < x("a2"), "a1 must render left of a2");
    }

    #[test]
    fn text_node_after_closed_lane_errors() {
        // A node declared after a top-level lane's closing brace is outside
        // any lane — it must not silently attach to the closed lane.
        let err = AdvanceDiagram::parse_text("lane top {\n  a\n}\nb\n").unwrap_err();
        assert!(err.message.contains("outside of any lane"));
    }

    #[test]
    fn text_style_node() {
        let d = text_diagram(
            "lane l\n\
             \x20 a\n\
             style a fill:#fee,stroke:#900,stroke-width:3px\n",
        );
        let st = &d.nodes[0].style;
        assert_eq!(st.fill.as_deref(), Some("#fee"));
        assert_eq!(st.stroke.as_deref(), Some("#900"));
        assert_eq!(st.stroke_width, Some(3.0));
    }

    #[test]
    fn text_stroke_width_rejects_bad_values() {
        // Same guard as the JSON path: non-finite / non-positive widths error.
        assert!(AdvanceDiagram::parse_text("lane l\n  a\nstyle a stroke-width:nan\n").is_err());
        assert!(AdvanceDiagram::parse_text("lane l\n  a\nstyle a stroke-width:inf\n").is_err());
        assert!(AdvanceDiagram::parse_text("lane l\n  a\nstyle a stroke-width:0\n").is_err());
        assert!(AdvanceDiagram::parse_text("lane l\n  a\nstyle a stroke-width:-2\n").is_err());
        // Positive finite values (with or without px) still parse.
        let d = text_diagram("lane l\n  a\nstyle a stroke-width:3px\n");
        assert_eq!(d.nodes[0].style.stroke_width, Some(3.0));
    }

    #[test]
    fn text_style_edge() {
        let d = text_diagram(
            "lane l\n\
             \x20 a\n\
             \x20 b\n\
             a --> b\n\
             style a-->b color:#b00,dash:3 2\n",
        );
        let es = &d.edges[0].style;
        assert_eq!(es.color.as_deref(), Some("#b00"));
        assert_eq!(es.dash.as_deref(), Some("3 2"));
    }

    #[test]
    fn text_class_def_resolved_after_usage() {
        let d = text_diagram(
            "lane l\n\
             \x20 a\n\
             class a warn\n\
             classDef warn fill:#fff3cd,stroke:#f0ad4e\n",
        );
        assert_eq!(d.nodes[0].style.fill.as_deref(), Some("#fff3cd"));
        assert_eq!(d.nodes[0].style.stroke.as_deref(), Some("#f0ad4e"));
    }

    #[test]
    fn text_class_shorthand() {
        let d = text_diagram(
            "lane l\n\
             \x20 a::warn\n\
             classDef warn fill:#fff3cd\n",
        );
        assert_eq!(d.nodes[0].id, "a");
        assert_eq!(d.nodes[0].style.fill.as_deref(), Some("#fff3cd"));
    }

    #[test]
    fn text_ports() {
        let d = text_diagram(
            "lane l\n\
             \x20 a\n\
             \x20 b\n\
             a:right --> b:top\n",
        );
        let e = &d.edges[0];
        assert_eq!(e.from_side, Some(AdvanceSide::Right));
        assert_eq!(e.to_side, Some(AdvanceSide::Top));
    }

    // ---- Phase 4: port-aware routing & label collision ----

    fn test_node(id: &str, x: f64, y: f64) -> AdvanceSceneNode {
        AdvanceSceneNode {
            id: id.to_string(),
            label: id.to_string(),
            lane: "l".to_string(),
            x,
            y,
            w: 60.0,
            h: 40.0,
            shape: Shape::Rect,
            style: NodeStyle::default(),
            elements: Vec::new(),
            anchors: Vec::new(),
        }
    }

    #[test]
    fn ports_affect_anchor() {
        let d = text_diagram(
            "lane l\n\
             \x20 a\n\
             \x20 b\n\
             \x20 c\n\
             a:right --> b:left\n\
             a:bottom --> b:top\n\
             b --> c:left\n",
        );
        let sc = layout(&d);
        let node = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap();
        let (a, b, c) = (node("a"), node("b"), node("c"));

        // Both sides pinned: first/last points sit exactly on the sides,
        // and the leader leaves the source along its side's normal.
        let e0 = &sc.edges[0];
        let p0 = e0.points[0];
        let p1 = e0.points[1];
        let last = *e0.points.last().unwrap();
        assert!((p0.0 - (a.x + a.w / 2.0)).abs() < 1e-6 && (p0.1 - a.y).abs() < 1e-6);
        assert!((p1.0 - (a.x + a.w / 2.0 + 18.0)).abs() < 1e-6 && (p1.1 - a.y).abs() < 1e-6);
        assert!((last.0 - (b.x - b.w / 2.0)).abs() < 1e-6 && (last.1 - b.y).abs() < 1e-6);

        let e1 = &sc.edges[1];
        let p0 = e1.points[0];
        let last = *e1.points.last().unwrap();
        assert!((p0.0 - a.x).abs() < 1e-6 && (p0.1 - (a.y + a.h / 2.0)).abs() < 1e-6);
        assert!((last.0 - b.x).abs() < 1e-6 && (last.1 - (b.y - b.h / 2.0)).abs() < 1e-6);

        // Only the target side pinned: the source falls back to the
        // natural anchor (b leaves through its bottom toward c).
        let e2 = &sc.edges[2];
        let p0 = e2.points[0];
        let last = *e2.points.last().unwrap();
        assert!((p0.0 - b.x).abs() < 1e-6 && (p0.1 - (b.y + b.h / 2.0)).abs() < 1e-6);
        assert!((last.0 - (c.x - c.w / 2.0)).abs() < 1e-6 && (last.1 - c.y).abs() < 1e-6);
    }

    #[test]
    fn label_avoids_node() {
        let nodes = vec![
            test_node("a", 100.0, 100.0),
            test_node("blocker", 108.0, 220.0),
            test_node("c", 100.0, 360.0),
        ];
        // Straight a→c run whose default label spot (x+8, mid) = (108,220)
        // sits right on top of the blocker node.
        let points = vec![
            (100.0, 140.0),
            (100.0, 300.0),
            (300.0, 300.0),
            (300.0, 340.0),
        ];
        let chosen =
            choose_label_pos(&label_candidates(&points), "XX", "a", "c", &nodes, &[]).unwrap();
        assert_eq!(chosen, (200.0, 300.0));
        let lb = label_box(chosen, "XX");
        assert!(
            !rects_overlap(lb, node_rect(&nodes[1])),
            "label overlaps blocker node"
        );
    }

    #[test]
    fn label_avoids_other_labels() {
        let nodes = vec![
            test_node("a", 100.0, 100.0),
            test_node("blocker", 108.0, 220.0),
            test_node("c", 100.0, 360.0),
        ];
        // First edge's label is pushed off its blocked default to (200,300).
        let first = vec![
            (100.0, 140.0),
            (100.0, 300.0),
            (300.0, 300.0),
            (300.0, 340.0),
        ];
        let first_pos =
            choose_label_pos(&label_candidates(&first), "XX", "a", "c", &nodes, &[]).unwrap();
        assert_eq!(first_pos, (200.0, 300.0));
        let placed = vec![label_box(first_pos, "XX")];

        // Second edge's default (200,285) would overlap the first label,
        // so it must move to the next free candidate.
        let second = vec![
            (192.0, 240.0),
            (192.0, 330.0),
            (320.0, 330.0),
            (320.0, 360.0),
        ];
        let unconstrained =
            choose_label_pos(&label_candidates(&second), "YY", "d", "f", &nodes, &[]).unwrap();
        assert_eq!(unconstrained, (200.0, 285.0));
        let second_pos =
            choose_label_pos(&label_candidates(&second), "YY", "d", "f", &nodes, &placed).unwrap();
        assert_eq!(second_pos, (256.0, 330.0));
        assert!(!rects_overlap(label_box(second_pos, "YY"), placed[0]));
    }

    // ---- Phase 5: per-node / per-edge rendering ----

    #[test]
    fn per_node_style_renders() {
        let d = text_diagram(
            "lane l\n\
             \x20 a\n\
             \x20 b\n\
             a --> b\n\
             style a fill:#fee,stroke:#900,stroke-width:4,color:#fff\n",
        );
        let scene = layout(&d);
        let svg = to_svg(&scene);
        assert!(
            svg.contains(r##"fill="#fee" stroke="#900" stroke-width="4.0""##),
            "node body style"
        );
        assert!(
            svg.contains(r##"fill="#fff">a</text>"##),
            "node label color"
        );
        // The unstyled node keeps the shape theme.
        assert!(svg.contains(r##"fill="#fafafa""##) || svg.contains(r##"fill="#ffffff""##));
    }

    #[test]
    fn per_edge_color_renders_with_matching_marker() {
        let d = text_diagram(
            "lane l\n\
             \x20 a\n\
             \x20 b\n\
             \x20 c\n\
             a --> b\n\
             b -->|via| c\n\
             style b-->c color:#f00,dash:3 2,stroke-width:2,label-fill:#eee\n",
        );
        let scene = layout(&d);
        let svg = to_svg(&scene);

        // A marker filled with the styled edge's stroke colour exists.
        let f00_idx = svg
            .find(r##"fill="#f00"/></marker>"##)
            .expect("no #f00 marker");
        let id_start = svg[..f00_idx].rfind(r#"id=""#).expect("no marker id") + 4;
        let id_end = svg[id_start..f00_idx]
            .find('"')
            .expect("unterminated marker id")
            + id_start;
        let f00_id = &svg[id_start..id_end];

        // The red edge path carries the style and references that marker.
        let red_start = svg.find(r##"stroke="#f00""##).expect("no red edge path");
        let path_start = svg[..red_start].rfind("<path").expect("no path element");
        let path_end = svg[path_start..].find("/>").expect("unterminated path") + path_start;
        let red_path = &svg[path_start..path_end];
        assert!(red_path.contains("stroke-width=\"2.0\""));
        assert!(red_path.contains("stroke-dasharray=\"3 2\""));
        assert!(red_path.contains(&format!("marker-end=\"url(#{})\"", f00_id)));

        // The styled edge's label pill uses its own label-fill.
        assert!(svg.contains(r##"fill="#eee""##));
    }

    // ---- Phase 6: scene hit-testing ----

    #[test]
    fn hit_test_node_edge_lane() {
        let d = text_diagram(
            "lane l\n\
             \x20 a\n\
             \x20 b\n\
             a --> b\n",
        );
        let sc = layout(&d);
        let a = sc.nodes.iter().position(|n| n.id == "a").unwrap();

        // A node's centre resolves to that node (topmost paint order).
        assert_eq!(
            sc.hit_test(sc.nodes[a].x, sc.nodes[a].y, 5.0),
            Some(AdvanceHit::Node(a))
        );

        // Midway between the two nodes the pick lands on the edge.
        let e = &sc.edges[0];
        let mid = {
            let mut best = None;
            for w in e.points.windows(2) {
                if (w[0].0 - w[1].0).abs() < f64::EPSILON {
                    let len = (w[1].1 - w[0].1).abs();
                    if best.map(|(_, _, l)| len > l).unwrap_or(true) {
                        best = Some((w[0].0, (w[0].1 + w[1].1) / 2.0, len));
                    }
                }
            }
            best.unwrap()
        };
        assert_eq!(sc.edge_at(mid.0, mid.1, 5.0), Some(0));
        assert_eq!(sc.hit_test(mid.0, mid.1, 5.0), Some(AdvanceHit::Edge(0)));

        // In the lane's top-left corner, away from nodes and edges, the
        // pick resolves to the lane.
        let (lx, ly) = (sc.lanes[0].x + 2.0, sc.lanes[0].y + 2.0);
        assert_eq!(sc.lane_at(lx, ly), Some(0));
        assert_eq!(sc.hit_test(lx, ly, 5.0), Some(AdvanceHit::Lane(0)));

        // Shape-precise: a diamond's box corner is empty even though the
        // bounding rect contains it.
        let d2 = text_diagram("lane l\n  d{Check}\n");
        let sc2 = layout(&d2);
        let nd = &sc2.nodes[0];
        let corner = (nd.x + nd.w / 2.0 - 1.0, nd.y + nd.h / 2.0 - 1.0);
        assert_eq!(sc2.node_at(corner.0, corner.1), None);
        assert_eq!(sc2.node_at(nd.x, nd.y), Some(0));
    }

    #[test]
    fn nearest_node_distance() {
        let d = text_diagram("lane l\n  a\n  b\n");
        let sc = layout(&d);
        let a = sc.nodes.iter().position(|n| n.id == "a").unwrap();
        let b = sc.nodes.iter().position(|n| n.id == "b").unwrap();

        // Inside a node the snap distance is 0.
        let (i, dist) = sc.nearest_node(sc.nodes[a].x, sc.nodes[a].y).unwrap();
        assert_eq!(i, a);
        assert!(dist < 1e-9);

        // Far right of the top node, the top node is still the nearest.
        let far_x = sc.nodes[a].x + sc.nodes[a].w / 2.0 + 50.0;
        let (i, dist) = sc.nearest_node(far_x, sc.nodes[a].y).unwrap();
        assert_eq!(i, a);
        assert!(dist > 40.0);

        // Both nodes are pickable via hit_test too.
        assert_eq!(sc.node_at(sc.nodes[b].x, sc.nodes[b].y), Some(b));
    }

    #[test]
    fn text_config() {
        let d = text_diagram(
            "lane l\n\
             \x20 a\n\
             config margin 42\n\
             config lane_gap 60\n\
             config order topology\n",
        );
        assert_eq!(d.config.margin, 42.0);
        assert_eq!(d.config.lane_gap, 60.0);
        assert_eq!(d.config.order, AdvanceOrder::Topology);
    }

    #[test]
    fn text_breaks() {
        let d = text_diagram(
            "lane l \"My<br/>Lane\"\n\
             \x20 a[Line1<br/>Line2]\n\
             \x20 b\n\
             a -->|go<br/>now| b\n",
        );
        assert_eq!(d.lanes[0].title, "My\nLane");
        assert_eq!(d.nodes[0].label, "Line1\nLine2");
        assert_eq!(d.edges[0].label.as_deref(), Some("go\nnow"));
    }

    #[test]
    fn text_error_has_snippet() {
        let err = AdvanceDiagram::parse_text("lane l\n  a --> b\n").unwrap_err();
        assert!(err.message.contains("line 2"));
        assert!(err.message.contains("a --> b"));
    }

    #[test]
    fn node_and_edge_style_json_roundtrip() {
        let src = r##"{
            "lanes":[{"id":"l"}],
            "nodes":[
                {"id":"a","label":"A","lane":"l","style":{"fill":"#fee","stroke":"#900","color":"#fff","stroke-width":2.0}},
                {"id":"b","label":"B","lane":"l"}
            ],
            "edges":[
                {"from":"a","to":"b","style":{"color":"#b00","dash":"3,2","stroke-width":2.5,"label-fill":"#eee"},"from_side":"right","to_side":"top"}
            ]
        }"##;
        let d = AdvanceDiagram::parse(src).unwrap();
        assert_eq!(d.nodes[0].style.fill.as_deref(), Some("#fee"));
        assert_eq!(d.nodes[0].style.stroke_width, Some(2.0));
        assert!(d.nodes[1].style == NodeStyle::default());
        let e = &d.edges[0];
        assert_eq!(e.style.color.as_deref(), Some("#b00"));
        assert_eq!(e.style.dash.as_deref(), Some("3,2"));
        assert_eq!(e.style.stroke_width, Some(2.5));
        assert_eq!(e.style.label_fill.as_deref(), Some("#eee"));
        assert_eq!(e.from_side, Some(AdvanceSide::Right));
        assert_eq!(e.to_side, Some(AdvanceSide::Top));
        // Round-trip through to_json preserves styles and sides.
        let d2 = AdvanceDiagram::parse(&to_json(&d)).unwrap();
        assert_eq!(d2.nodes, d.nodes);
        assert_eq!(d2.edges, d.edges);
    }

    #[test]
    fn side_parse_validates() {
        let json_str = |s: &str| parse_json(s).unwrap();
        assert_eq!(parse_side_json(&json_str("\"auto\"")).unwrap(), None);
        assert_eq!(
            parse_side_json(&json_str("\"top\"")).unwrap(),
            Some(AdvanceSide::Top)
        );
        assert!(parse_side_json(&json_str("\"north\"")).is_err());
        assert!(parse_side_json(&json_str("42")).is_err());
    }

    #[test]
    fn px_number_accepts_strings() {
        let json_str = |s: &str| parse_json(s).unwrap();
        assert_eq!(px_number(&json_str("4")).unwrap(), 4.0);
        assert_eq!(px_number(&json_str("\"4px\"")).unwrap(), 4.0);
        assert!(px_number(&json_str("\"wide\"")).is_err());
    }

    #[test]
    fn style_values_cannot_close_an_svg_attribute() {
        // Every colour an advance diagram can carry, from all three
        // entry points, including the diagram-level `style` object that
        // the first pass at this fix missed entirely.
        let breakout = |svg: &str| svg.contains("\" onload") || svg.contains("' onload");

        let text_node = "lane l \"L\"\na[A]\nstyle a fill:x\" onload=1\n";
        assert!(!breakout(&render_advance_text_svg(text_node).unwrap()));
        let text_edge =
            "lane l \"L\"\na[A]\nb[B]\na --> b\nstyle a-->b color:x' onload=1,dash:y\" onload=1\n";
        assert!(!breakout(&render_advance_text_svg(text_edge).unwrap()));

        for key in [
            "lane_fill",
            "lane_stroke",
            "text_color",
            "edge_color",
            "label_fill",
        ] {
            let json = format!(
                r##"{{"lanes":[{{"id":"l","title":"L"}}],
                   "nodes":[{{"id":"a","lane":"l","label":"A"}},{{"id":"b","lane":"l","label":"B"}}],
                   "edges":[{{"from":"a","to":"b","label":"e"}}],
                   "style":{{"{key}":"#fff\" onload=1"}}}}"##
            );
            let svg = render_advance_svg(&json).unwrap();
            assert!(!breakout(&svg), "style.{key} escaped its attribute:\n{svg}");
            // Positive too: unknown JSON style keys are ignored for
            // forward-compat, so if one of these is ever renamed the
            // render path would go untested while this test stayed green.
            assert!(
                svg.contains("#fff&quot; onload=1"),
                "style.{key} never reached the SVG — is the key still read?\n{svg}"
            );
        }

        let node_json = r#"{"lanes":[{"id":"l","title":"L"}],
            "nodes":[{"id":"a","lane":"l","label":"A","style":{"fill":"x\" onload=1"}}],
            "edges":[]}"#;
        assert!(!breakout(&render_advance_svg(node_json).unwrap()));
    }

    #[test]
    fn ordinary_advance_css_values_still_parse() {
        let svg =
            render_advance_text_svg("lane l \"L\"\na[A]\nstyle a fill:rgb(1, 2, 3)\n").unwrap();
        assert!(svg.contains("fill=\"rgb(1, 2, 3)\""), "{svg}");
    }

    // ------------------------------------------------------------
    // Terminals: anchors, sub-elements, reference grammar
    // ------------------------------------------------------------

    const TERMINALS: &str = "lane hw \"HW\" {\n\
        \x20 cpu[CPU] {\n\
        \x20   anchor out bottom 0.5\n\
        \x20   core0[Core 0]\n\
        \x20   core1[Core 1] { anchor irq right }\n\
        \x20 }\n\
        \x20 mem[Memory] {\n\
        \x20   bank0[Bank 0]\n\
        \x20   bank1[Bank 1]\n\
        \x20   layout row\n\
        \x20 }\n\
        }\n\
        lane sw \"SW\" {\n\
        \x20 bus[Bus]\n\
        }\n\
        cpu.core1@irq --> mem.bank0\n\
        cpu@out --> bus:top\n\
        cpu.core0 -->|dma| mem.bank1\n";

    fn scene_node_by<'a>(sc: &'a AdvanceScene, id: &str) -> &'a AdvanceSceneNode {
        sc.nodes.iter().find(|n| n.id == id).expect(id)
    }

    fn inside_strict(p: (f64, f64), n: &AdvanceSceneNode) -> bool {
        (p.0 - n.x).abs() < n.w / 2.0 - 0.5 && (p.1 - n.y).abs() < n.h / 2.0 - 0.5
    }

    #[test]
    fn parse_end_reads_every_form_of_the_grammar() {
        let e = parse_end("a").unwrap();
        assert_eq!(
            (e.node.as_str(), e.path.len(), e.at.is_none()),
            ("a", 0, true)
        );
        let e = parse_end("a:right").unwrap();
        assert_eq!(e.at, Some(AnchorRef::Side(AdvanceSide::Right)));
        let e = parse_end("a@out").unwrap();
        assert_eq!(e.at, Some(AnchorRef::Named("out".into())));
        let e = parse_end("a.b.c@p").unwrap();
        assert_eq!(e.path, vec!["b".to_string(), "c".to_string()]);
        assert_eq!(e.at, Some(AnchorRef::Named("p".into())));
        let e = parse_end("a.b:left").unwrap();
        assert_eq!(
            (e.path.len(), e.at),
            (1, Some(AnchorRef::Side(AdvanceSide::Left)))
        );
        // Round trip through the printed form.
        assert_eq!(parse_end("a.b.c@p").unwrap().to_ref(), "a.b.c@p");
        assert_eq!(parse_end("a:top").unwrap().to_ref(), "a:top");
        // A `:word` that is not a side stays in the id, as before.
        let e = parse_end("a:foo").unwrap();
        assert_eq!((e.node.as_str(), e.at.is_none()), ("a:foo", true));
        // Malformed.
        assert!(parse_end("a.").is_err());
        assert!(parse_end("a@").is_err());
        assert!(parse_end("a@x.y").is_err());
    }

    #[test]
    fn text_node_block_declares_anchors_and_sub_elements() {
        let d = text_diagram(TERMINALS);
        let cpu = d.nodes.iter().find(|n| n.id == "cpu").unwrap();
        assert_eq!(cpu.anchors.len(), 1);
        assert_eq!(
            (
                cpu.anchors[0].id.as_str(),
                cpu.anchors[0].side,
                cpu.anchors[0].offset
            ),
            ("out", AdvanceSide::Bottom, 0.5)
        );
        assert_eq!(
            cpu.elements
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["core0", "core1"]
        );
        // The one-line block form declared an anchor on core1.
        assert_eq!(cpu.elements[1].anchors[0].id, "irq");
        assert_eq!(cpu.elements[1].anchors[0].side, AdvanceSide::Right);
        let mem = d.nodes.iter().find(|n| n.id == "mem").unwrap();
        assert_eq!(mem.layout, ElementLayout::Row);
        // Edge ends carry the full reference; sides derive from anchors.
        assert_eq!(d.edges[0].from_end.to_ref(), "cpu.core1@irq");
        assert_eq!(d.edges[0].from_side, Some(AdvanceSide::Right));
        assert_eq!(d.edges[1].to_end.to_ref(), "bus:top");
        assert_eq!(d.edges[2].from_end.path, vec!["core0".to_string()]);
        assert_eq!(d.edges[2].label.as_deref(), Some("dma"));
        // Nodes after the block are still in the lane.
        assert_eq!(node_lane(&d, "mem"), "hw");
    }

    #[test]
    fn text_block_mistakes_are_named_with_a_line_number() {
        let err = |src: &str| AdvanceDiagram::parse_text(src).unwrap_err().message;
        let e = err("lane l \"L\"\na[A] {\n  p[P]\n");
        assert!(e.contains("line 2") && e.contains("never closed"), "{e}");
        let e = err("lane l \"L\"\na[A] {\n  p --> q\n}\n");
        assert!(
            e.contains("line 3") && e.contains("not allowed inside"),
            "{e}"
        );
        let e = err("lane l \"L\"\na[A] { p[P]; q[Q]; r[R] }\nb[B]\na.q:top --> b\n");
        assert!(
            e.contains("line 4") && e.contains("not an exposed side"),
            "{e}"
        );
        let e = err("lane l \"L\"\na.b[X]\n");
        assert!(e.contains("may not contain '.'"), "{e}");
        let e = err("lane l \"L\"\na[A] { anchor x left }\nb[B]\na@nope --> b\n");
        assert!(e.contains("has no anchor 'nope'"), "{e}");
        let e = err("lane l \"L\"\na[A] { anchor x left 1.5 }\n");
        assert!(e.contains("0..=1"), "{e}");
        let e = err("lane l \"L\"\na[A]\nb[B]\na.zz --> b\n");
        assert!(e.contains("no sub-element 'zz'"), "{e}");
    }

    #[test]
    fn exposed_side_rule_follows_the_layout() {
        // Column: left/right always, top only first, bottom only last.
        let ok = |src: &str| AdvanceDiagram::parse_text(src).is_ok();
        let base = "lane l \"L\"\na[A] { p[P]; q[Q]; r[R] }\nb[B]\n";
        assert!(ok(&format!("{base}a.q:left --> b\n")));
        assert!(ok(&format!("{base}a.r:bottom --> b\n")));
        assert!(!ok(&format!("{base}a.p:bottom --> b\n")));
        // `top` is never exposed: the label band sits above the first
        // child, and a lead through the label text is not a lead.
        assert!(!ok(&format!("{base}a.p:top --> b\n")));
        assert!(!ok(&format!("{base}a.r:top --> b\n")));
        // Row: bottom always, left only first, right only last.
        let row = "lane l \"L\"\na[A] { p[P]; q[Q]; r[R]; layout row }\nb[B]\n";
        assert!(ok(&format!("{row}a.q:bottom --> b\n")));
        assert!(!ok(&format!("{row}a.q:top --> b\n")));
        assert!(!ok(&format!("{row}a.q:left --> b\n")));
        assert!(ok(&format!("{row}a.r:right --> b\n")));
        // Nesting: exposure must hold at every level.
        let nest = "lane l \"L\"\na[A] { p[P] { x[X]; y[Y] }; q[Q] }\nb[B]\n";
        assert!(ok(&format!("{nest}a.p.x:left --> b\n")));
        assert!(
            !ok(&format!("{nest}a.p.y:bottom --> b\n")),
            "p is not last, so y's bottom is interior"
        );
        assert!(ok(&format!("{nest}a.q:bottom --> b\n")));
    }

    #[test]
    fn json_terminals_round_trip_through_to_json() {
        let d = text_diagram(TERMINALS);
        let json = to_json(&d);
        assert!(
            json.contains("\"anchors\":[{\"id\":\"out\",\"side\":\"bottom\",\"offset\":0.5}]"),
            "{json}"
        );
        assert!(json.contains("\"elements\":["), "{json}");
        assert!(json.contains("\"layout\":\"row\""), "{json}");
        assert!(json.contains("\"from\":\"cpu.core1@irq\""), "{json}");
        let back = AdvanceDiagram::parse(&json).unwrap();
        assert_eq!(back.nodes, d.nodes);
        assert_eq!(back.edges, d.edges);
        // A diagram with no terminals serialises exactly as before.
        let plain = text_diagram("lane l \"L\"\na[A]\nb[B]\na:right --> b:top\n");
        let j = to_json(&plain);
        assert!(
            j.contains("\"from\":\"a\",\"to\":\"b\"") && j.contains("\"from_side\":\"right\""),
            "{j}"
        );
        assert!(!j.contains("anchors") && !j.contains("elements"), "{j}");
    }

    #[test]
    fn json_input_accepts_terminals_and_checks_them() {
        let src = r#"{"lanes":[{"id":"l","title":"L"}],
            "nodes":[{"id":"a","lane":"l","label":"A","layout":"row",
                      "anchors":[{"id":"o","side":"top","offset":0.25}],
                      "elements":[{"id":"p","label":"P","anchors":[{"id":"q","side":"bottom"}]},{"id":"r"}]},
                     {"id":"b","lane":"l"}],
            "edges":[{"from":"a.p@q","to":"b","from_side":"left"},{"from":"a@o","to":"b:top"}]}"#;
        let d = AdvanceDiagram::parse(src).unwrap();
        assert_eq!(d.nodes[0].layout, ElementLayout::Row);
        assert_eq!(d.nodes[0].elements[1].label, "r");
        // `from_side` does not override an anchor's own side.
        assert_eq!(d.edges[0].from_side, Some(AdvanceSide::Bottom));
        assert_eq!(d.edges[1].to_side, Some(AdvanceSide::Top));
        let bad = src.replace("\"from\":\"a.p@q\"", "\"from\":\"a.p@zz\"");
        assert!(AdvanceDiagram::parse(&bad)
            .unwrap_err()
            .message
            .contains("no anchor 'zz'"));
        let bad = src.replace("\"id\":\"p\"", "\"id\":\"p.x\"");
        assert!(AdvanceDiagram::parse(&bad)
            .unwrap_err()
            .message
            .contains("may not contain '.'"));
    }

    #[test]
    fn layout_places_sub_elements_inside_a_node_that_grew_to_fit() {
        let sc = layout(&text_diagram(TERMINALS));
        let cpu = scene_node_by(&sc, "cpu");
        assert_eq!(cpu.elements.len(), 2);
        for el in &cpu.elements {
            assert!(
                el.x - el.w / 2.0 >= cpu.x - cpu.w / 2.0
                    && el.x + el.w / 2.0 <= cpu.x + cpu.w / 2.0,
                "{} overflows x",
                el.id
            );
            assert!(
                el.y - el.h / 2.0 >= cpu.y - cpu.h / 2.0
                    && el.y + el.h / 2.0 <= cpu.y + cpu.h / 2.0,
                "{} overflows y",
                el.id
            );
        }
        // Column: core1 below core0, same width. Row: bank1 right of bank0.
        assert!(
            cpu.elements[1].y > cpu.elements[0].y
                && (cpu.elements[1].w - cpu.elements[0].w).abs() < 1e-9
        );
        let mem = scene_node_by(&sc, "mem");
        assert!(
            mem.elements[1].x > mem.elements[0].x
                && (mem.elements[1].y - mem.elements[0].y).abs() < 1e-9
        );
        // The node is taller than a plain node would be.
        let plain = layout(&text_diagram("lane l \"L\"\ncpu[CPU]\n"));
        assert!(cpu.h > scene_node_by(&plain, "cpu").h);
        // Anchors sit exactly on their host's boundary.
        let out = cpu.anchors.iter().find(|a| a.id == "out").unwrap();
        assert!((out.y - (cpu.y + cpu.h / 2.0)).abs() < 1e-9 && out.element.is_none());
        let irq = cpu.anchors.iter().find(|a| a.id == "irq").unwrap();
        let core1 = &cpu.elements[irq.element.unwrap()];
        assert_eq!(core1.id, "core1");
        assert!((irq.x - (core1.x + core1.w / 2.0)).abs() < 1e-9);
    }

    #[test]
    fn a_terminal_edge_starts_on_its_element_and_leads_out_through_the_node() {
        let sc = layout(&text_diagram(TERMINALS));
        let cpu = scene_node_by(&sc, "cpu");
        let e = &sc.edges[0]; // cpu.core1@irq --> mem.bank0
        let irq = cpu.anchors.iter().find(|a| a.id == "irq").unwrap();
        assert!((e.from_point.0 - irq.x).abs() < 1e-9 && (e.from_point.1 - irq.y).abs() < 1e-9);
        assert_eq!(e.points[0], e.from_point);
        // The lead runs straight right of the anchor and clears cpu's
        // right edge before the route turns. The edge itself is not a
        // vertex of the polyline — it is collinear with the lead, so it
        // collapses away — but the segment must still cross it.
        assert!((e.points[1].1 - irq.y).abs() < 1e-9);
        assert!(e.points[1].0 > cpu.x + cpu.w / 2.0);
        // Target lands on bank0's boundary and the scene names both ends.
        let mem = scene_node_by(&sc, "mem");
        let bank0 = &mem.elements[0];
        let tp = e.to_point;
        let on_edge = (tp.0 - (bank0.x - bank0.w / 2.0)).abs() < 1e-9
            || (tp.0 - (bank0.x + bank0.w / 2.0)).abs() < 1e-9
            || (tp.1 - (bank0.y - bank0.h / 2.0)).abs() < 1e-9
            || (tp.1 - (bank0.y + bank0.h / 2.0)).abs() < 1e-9;
        assert!(on_edge, "to_point {:?} is not on bank0's boundary", tp);
        assert_eq!(e.from_end.to_ref(), "cpu.core1@irq");
        assert_eq!(e.to_end.to_ref(), "mem.bank0");
        // Everything stays orthogonal.
        for w in e.points.windows(2) {
            assert!(
                (w[0].0 - w[1].0).abs() < 1e-9 || (w[0].1 - w[1].1).abs() < 1e-9,
                "diagonal segment"
            );
        }
    }

    #[test]
    fn a_ported_edge_never_runs_back_through_its_own_nodes() {
        // The showcase defect: d:right --> b:top with b to the LEFT of d
        // used to leave right, then cut back across d and through b.
        let sc = layout(&text_diagram(
            "lane backend \"B\" {\n  b[API Gateway]\n}\nlane frontend \"F\" {\n  d[Dashboard]\n}\nd:right --> b:top\n",
        ));
        let (b, d) = (scene_node_by(&sc, "b"), scene_node_by(&sc, "d"));
        let e = &sc.edges[0];
        for w in e.points.windows(2) {
            for t in [0.25, 0.5, 0.75] {
                let p = (
                    w[0].0 + (w[1].0 - w[0].0) * t,
                    w[0].1 + (w[1].1 - w[0].1) * t,
                );
                assert!(
                    !inside_strict(p, b) && !inside_strict(p, d),
                    "segment {:?}-{:?} crosses a node",
                    w[0],
                    w[1]
                );
            }
        }
        // Still leaves d rightwards and enters b from above.
        assert!(e.points[1].0 > e.points[0].0);
        let last = e.points.len() - 1;
        assert!(e.points[last - 1].1 < e.points[last].1);
    }

    #[test]
    fn a_ported_edge_that_was_already_clear_is_unchanged() {
        let sc = layout(&text_diagram(
            "lane L \"L\"\na[A]\nlane R \"R\"\nc[C]\na:right --> c:left\n",
        ));
        let e = &sc.edges[0];
        // p0, leader, leader, p3 — all on one horizontal line, no detour.
        assert!(
            e.points.iter().all(|p| (p.1 - e.points[0].1).abs() < 1e-9),
            "{:?}",
            e.points
        );
        assert!(e.points.len() <= 4, "{:?}", e.points);
    }

    #[test]
    fn scene_json_carries_elements_anchors_and_terminal_points() {
        let sc = layout(&text_diagram(TERMINALS));
        let j = scene_to_json(&sc);
        assert!(j.contains("\"elements\":[{\"id\":\"core0\""), "{j}");
        assert!(j.contains("\"path\":[\"core0\"]"), "{j}");
        assert!(
            j.contains("\"anchors\":[")
                && j.contains("\"id\":\"irq\"")
                && j.contains("\"element\":1"),
            "{j}"
        );
        assert!(
            j.contains("\"from_point\":[") && j.contains("\"from_end\":\"cpu.core1@irq\""),
            "{j}"
        );
        // Plain scenes gain only the two points.
        let plain = scene_to_json(&layout(&text_diagram(
            "lane l \"L\"\na[A]\nb[B]\na --> b\n",
        )));
        assert!(
            plain.contains("\"from_point\":[")
                && !plain.contains("from_end")
                && !plain.contains("elements"),
            "{plain}"
        );
    }

    #[test]
    fn hit_test_prefers_anchor_then_element_then_node() {
        let sc = layout(&text_diagram(TERMINALS));
        let ni = sc.nodes.iter().position(|n| n.id == "cpu").unwrap();
        let cpu = &sc.nodes[ni];
        let irq = cpu.anchors.iter().position(|a| a.id == "irq").unwrap();
        let a = &cpu.anchors[irq];
        assert_eq!(
            sc.hit_test(a.x + 1.0, a.y - 1.0, 4.0),
            Some(AdvanceHit::Anchor(ni, irq))
        );
        assert_eq!(sc.anchor_at(a.x, a.y, 0.1), Some((ni, irq)));
        let core0 = &cpu.elements[0];
        assert_eq!(
            sc.hit_test(core0.x, core0.y, 0.5),
            Some(AdvanceHit::Element(ni, 0))
        );
        assert_eq!(sc.element_at(core0.x, core0.y), Some((ni, 0)));
        // The label band above the compartments is the node itself.
        assert_eq!(
            sc.hit_test(cpu.x, cpu.y - cpu.h / 2.0 + 10.0, 0.5),
            Some(AdvanceHit::Node(ni))
        );
        assert_eq!(sc.element_at(cpu.x, cpu.y - cpu.h / 2.0 + 10.0), None);
    }

    #[test]
    fn svg_draws_compartments_with_their_labels() {
        let svg = to_svg(&layout(&text_diagram(TERMINALS)));
        assert_eq!(svg.matches("rx=\"4\"").count(), 4, "{svg}");
        for l in [
            "CPU", "Core 0", "Core 1", "Memory", "Bank 0", "Bank 1", "Bus", "dma",
        ] {
            assert!(svg.contains(&format!(">{l}<")), "missing label {l}");
        }
        // A styled element escapes its colours like a node does.
        let styled = r#"{"lanes":[{"id":"l","title":"L"}],
            "nodes":[{"id":"a","lane":"l","elements":[{"id":"p","style":{"fill":"x\" onload=1"}}]}],"edges":[]}"#;
        let svg = render_advance_svg(styled).unwrap();
        assert!(!svg.contains("\" onload"), "{svg}");
    }

    #[test]
    fn inline_block_and_multi_line_block_parse_the_same() {
        let a = text_diagram("lane l \"L\"\na[A] { anchor o left; p[P]; q[Q]; layout row }\n");
        let b = text_diagram(
            "lane l \"L\"\na[A] {\n  anchor o left\n  p[P]\n  q[Q]\n  layout row\n}\n",
        );
        assert_eq!(a.nodes, b.nodes);
        // A diamond is still a diamond, not a block.
        let d = text_diagram("lane l \"L\"\nc{Check}\n");
        assert_eq!(d.nodes[0].shape, Shape::Diamond);
        assert!(d.nodes[0].elements.is_empty());
    }

    // ------------------------------------------------------------
    // Review findings on P1 — each of these was a reproduced defect
    // ------------------------------------------------------------

    #[test]
    fn comment_and_directive_lines_ending_in_a_brace_are_not_blocks() {
        let d = text_diagram("lane l \"L\"\na[A]\n# see a[A] { anchor x left }\nb[B]\n");
        assert_eq!(d.nodes.len(), 2);
        let d = text_diagram("lane l \"L\"\na[A]\n%% note {x}\nb[B]\n");
        assert_eq!(d.nodes.len(), 2);
        let d = text_diagram("title Release {v2}\nlane l \"L\"\na[A]\n");
        assert_eq!(d.title.as_deref(), Some("Release {v2}"));
        assert_eq!(d.nodes.len(), 1);
    }

    #[test]
    fn inline_block_nesting_is_capped_like_json() {
        let mut src = String::from("lane l \"L\"\na[A]");
        for i in 0..40 {
            src.push_str(&format!(" {{ e{i}[E]"));
        }
        src.push_str(&" }".repeat(40));
        src.push('\n');
        let e = AdvanceDiagram::parse_text(&src).unwrap_err();
        assert!(e.message.contains("nesting exceeds limit"), "{}", e.message);
        let mut src = String::from("lane l \"L\"\na[A] {\n");
        for i in 0..40 {
            src.push_str(&format!("e{i}[E] {{\n"));
        }
        let e = AdvanceDiagram::parse_text(&src).unwrap_err();
        assert!(e.message.contains("nesting exceeds limit"), "{}", e.message);
    }

    #[test]
    fn shape_braces_and_label_punctuation_are_not_block_syntax() {
        let d = text_diagram("lane l \"L\"\nc {Text}\nh {{Hex}}\n");
        assert_eq!(
            (d.nodes[0].shape, d.nodes[0].label.as_str()),
            (Shape::Diamond, "Text")
        );
        assert_eq!(
            (d.nodes[1].shape, d.nodes[1].label.as_str()),
            (Shape::Hexagon, "Hex")
        );
        assert!(d.nodes.iter().all(|n| n.elements.is_empty()));
        let d = text_diagram("lane l \"L\"\na[Set {x}] { p[Hello; world]; q[Q] }\n");
        assert_eq!(
            (d.nodes[0].id.as_str(), d.nodes[0].label.as_str()),
            ("a", "Set {x}")
        );
        assert_eq!(
            d.nodes[0]
                .elements
                .iter()
                .map(|e| e.label.as_str())
                .collect::<Vec<_>>(),
            ["Hello; world", "Q"]
        );
        let d = text_diagram("lane l \"L\"\nc{Check} { anchor y right }\n");
        assert_eq!(d.nodes[0].shape, Shape::Diamond);
        assert_eq!(d.nodes[0].anchors[0].id, "y");
    }

    #[test]
    fn directives_and_edges_inside_a_block_are_refused() {
        let err = |src: &str| AdvanceDiagram::parse_text(src).unwrap_err().message;
        for body in [
            "x --- y",
            "style a fill:#f00",
            "class a hot",
            "config margin 4",
            "lane z \"Z\"",
        ] {
            let e = err(&format!("lane l \"L\"\na[A] {{\n  {body}\n}}\n"));
            assert!(
                e.contains("not allowed inside"),
                "'{body}' slipped through: {e}"
            );
        }
        let e = err("lane l \"L\"\na[A] {\n  [P]\n}\n");
        assert!(
            e.contains("cannot be empty") || e.contains("invalid"),
            "{e}"
        );
    }

    #[test]
    fn style_targets_a_terminal_edge_by_its_reference() {
        let d = text_diagram(&format!(
            "{TERMINALS}style cpu.core1@irq-->mem.bank0 color:#ff0000\n"
        ));
        assert_eq!(d.edges[0].style.color.as_deref(), Some("#ff0000"));
        assert_eq!(
            d.edges[2].style.color, None,
            "the other cpu->mem edge is untouched"
        );
        let d = text_diagram(&format!("{TERMINALS}style cpu-->mem color:#00ff00\n"));
        assert_eq!(d.edges[0].style.color.as_deref(), Some("#00ff00"));
        assert_eq!(d.edges[2].style.color.as_deref(), Some("#00ff00"));
        let e = AdvanceDiagram::parse_text(&format!(
            "{TERMINALS}style cpu.core1@irq-->mem.bank1 color:#f00\n"
        ))
        .unwrap_err();
        assert!(
            e.message.contains("matches no edge") && e.message.contains("line 19"),
            "{}",
            e.message
        );
    }

    #[test]
    fn edges_attach_to_the_node_they_name_whatever_the_scene_order() {
        let src = r#"{"lanes":[{"id":"A","title":"A"},{"id":"B","title":"B"}],
            "nodes":[{"id":"n1","lane":"A"},{"id":"n2","lane":"B","elements":[{"id":"p"}]},{"id":"n3","lane":"A"}],
            "edges":[{"from":"n1","to":"n2.p"}]}"#;
        let sc = layout(&AdvanceDiagram::parse(src).unwrap());
        let n2 = scene_node_by(&sc, "n2");
        let e = &sc.edges[0];
        let p = &n2.elements[0];
        assert!(
            (e.to_point.0 - p.x).abs() <= p.w / 2.0 + 1e-9
                && (e.to_point.1 - p.y).abs() <= p.h / 2.0 + 1e-9,
            "edge ends at {:?}, not on n2.p at ({}, {})",
            e.to_point,
            p.x,
            p.y
        );
        assert_eq!(e.to_end.to_ref(), "n2.p");
    }

    #[test]
    fn parallel_ported_edges_keep_their_fan_after_the_detour() {
        let sc = layout(&text_diagram(
            "lane backend \"B\" {\n  b[API]\n}\nlane frontend \"F\" {\n  d[Dash]\n}\nd:right --> b:top\nd:right --> b:top\n",
        ));
        assert_ne!(
            sc.edges[0].points, sc.edges[1].points,
            "the two routes collapsed onto one channel"
        );
    }

    #[test]
    fn a_ported_channel_avoids_a_third_node() {
        let sc = layout(&text_diagram(
            "lane l \"L\" {\n  a[A]\n  m[A much wider middle node]\n  b[B]\n}\na:right --> b:right\n",
        ));
        let m = scene_node_by(&sc, "m");
        for w in sc.edges[0].points.windows(2) {
            for t in [0.25, 0.5, 0.75] {
                let p = (
                    w[0].0 + (w[1].0 - w[0].0) * t,
                    w[0].1 + (w[1].1 - w[0].1) * t,
                );
                assert!(
                    !inside_strict(p, m),
                    "segment {:?}-{:?} runs through m",
                    w[0],
                    w[1]
                );
            }
        }
    }

    #[test]
    fn explicit_size_is_clamped_so_sub_elements_fit() {
        let src = r#"{"lanes":[{"id":"l","title":"L"}],
            "nodes":[{"id":"a","lane":"l","w":60,"h":30,"layout":"row",
                      "elements":[{"id":"p","label":"Long label here"},{"id":"q"}]}],"edges":[]}"#;
        let sc = layout(&AdvanceDiagram::parse(src).unwrap());
        let a = scene_node_by(&sc, "a");
        assert!(
            a.w > 60.0 && a.h > 30.0,
            "node kept the too-small box: {}x{}",
            a.w,
            a.h
        );
        for el in &a.elements {
            assert!(
                el.w > 0.0 && el.h > 0.0,
                "{} has a non-positive size",
                el.id
            );
            assert!(
                inside_strict((el.x, el.y), a),
                "{} sits outside its node",
                el.id
            );
        }
        assert!(!to_svg(&sc).contains("height=\"-"));
    }

    #[test]
    fn fit_canvas_moves_edge_labels_with_their_routes() {
        let src = r#"{"lanes":[{"id":"l","title":"L"}],
            "nodes":[{"id":"a","lane":"l","x":0,"y":0},{"id":"b","lane":"l","x":200,"y":0}],
            "edges":[{"from":"a","to":"b","label":"hello"}]}"#;
        let sc = layout(&AdvanceDiagram::parse(src).unwrap());
        let e = &sc.edges[0];
        let (lx, ly) = e.label_pos.expect("label placed");
        let near = e.points.windows(2).any(|w| {
            let (x0, y0, x1, y1) = (
                w[0].0.min(w[1].0),
                w[0].1.min(w[1].1),
                w[0].0.max(w[1].0),
                w[0].1.max(w[1].1),
            );
            lx >= x0 - 30.0 && lx <= x1 + 30.0 && ly >= y0 - 30.0 && ly <= y1 + 30.0
        });
        assert!(
            near,
            "label at ({lx}, {ly}) is nowhere near its route {:?}",
            e.points
        );
    }

    #[test]
    fn hit_test_does_not_let_an_occluded_anchor_beat_the_visible_node() {
        let src = r#"{"lanes":[{"id":"l","title":"L"}],
            "nodes":[{"id":"under","lane":"l","x":100,"y":100,"anchors":[{"id":"u","side":"right"}]},
                     {"id":"over","lane":"l","x":110,"y":100}],
            "edges":[]}"#;
        let sc = layout(&AdvanceDiagram::parse(src).unwrap());
        let ui = sc.nodes.iter().position(|n| n.id == "under").unwrap();
        let oi = sc.nodes.iter().position(|n| n.id == "over").unwrap();
        let a = &sc.nodes[ui].anchors[0];
        assert_eq!(
            sc.node_at(a.x, a.y),
            Some(oi),
            "test setup: the anchor point is covered by `over`"
        );
        assert_eq!(sc.hit_test(a.x, a.y, 4.0), Some(AdvanceHit::Node(oi)));
    }

    #[test]
    fn sub_elements_need_a_rect_or_rounded_node() {
        // Compartments are rectangles; inside a diamond 6 of 8 corners
        // fell outside the outline and could not be picked.
        for decl in ["d{Dec}", "h{{Hex}}", "c((C))", "p[/P/]"] {
            let e = AdvanceDiagram::parse_text(&format!("lane l \"L\"\n{decl} {{ p[P] }}\n"))
                .unwrap_err();
            assert!(
                e.message
                    .contains("sub-elements need a rect or rounded node"),
                "{decl}: {}",
                e.message
            );
            assert!(e.message.contains("line 2"), "{}", e.message);
        }
        for decl in ["r[R]", "o(O)"] {
            assert!(
                AdvanceDiagram::parse_text(&format!("lane l \"L\"\n{decl} {{ p[P] }}\n")).is_ok(),
                "{decl}"
            );
        }
        let json = r#"{"lanes":[{"id":"l","title":"L"}],"nodes":[{"id":"d","lane":"l","shape":"diamond","elements":[{"id":"p"}]}],"edges":[]}"#;
        assert!(AdvanceDiagram::parse(json)
            .unwrap_err()
            .message
            .contains("need a rect or rounded node"));
        // A shape without sub-elements is still free to be anything.
        assert!(AdvanceDiagram::parse_text("lane l \"L\"\nd{Dec}\n").is_ok());
    }

    #[test]
    fn a_lane_block_on_one_line_works_like_a_node_block() {
        // Used to swallow the title and the node without a word.
        let d = text_diagram("lane l \"L\" { a[A]; lane s \"S\" { b[B] } }\n");
        assert_eq!(d.lanes[0].title, "L");
        assert_eq!(d.lanes[0].children[0].title, "S");
        assert_eq!(node_lane(&d, "a"), "l");
        assert_eq!(node_lane(&d, "b"), "s");
        // `c` follows a closed top-level lane block: no lane scope.
        assert!(AdvanceDiagram::parse_text("lane l \"L\" { a[A] }\nc[C]\n").is_err());
    }

    #[test]
    fn an_id_ending_in_a_side_keyword_is_refused() {
        let e = AdvanceDiagram::parse_text("lane l \"L\"\na:right[X]\n").unwrap_err();
        assert!(e.message.contains("side keyword 'right'"), "{}", e.message);
        // A colon that is not a side is still fine, as it always was.
        let d = text_diagram("lane l \"L\"\na:b[X]\nc[C]\na:b --> c\n");
        assert_eq!(d.edges[0].from, "a:b");
    }

    #[test]
    fn every_front_end_accepts_the_same_nesting_depth() {
        let ok_text = |src: &str| AdvanceDiagram::parse_text(src).is_ok();
        let inline = |n: usize| {
            let mut s = String::from("lane l \"L\"\na[A]");
            for i in 0..n {
                s.push_str(&format!(" {{ e{i}[E]"));
            }
            s.push_str(&" }".repeat(n));
            s.push('\n');
            s
        };
        let multi = |n: usize| {
            let mut s = String::from("lane l \"L\"\na[A] {\n");
            for i in 0..n {
                s.push_str(&format!("e{i}[E] {{\n"));
            }
            s.push_str(&"}\n".repeat(n + 1));
            s
        };
        let json = |n: usize| {
            let mut e = format!("{{\"id\":\"e{}\"}}", n - 1);
            for i in (0..n - 1).rev() {
                e = format!("{{\"id\":\"e{i}\",\"elements\":[{e}]}}");
            }
            format!("{{\"lanes\":[{{\"id\":\"l\",\"title\":\"L\"}}],\"nodes\":[{{\"id\":\"a\",\"lane\":\"l\",\"elements\":[{e}]}}],\"edges\":[]}}")
        };
        assert!(ok_text(&inline(MAX_NEST_DEPTH)) && !ok_text(&inline(MAX_NEST_DEPTH + 1)));
        assert!(ok_text(&multi(MAX_NEST_DEPTH)) && !ok_text(&multi(MAX_NEST_DEPTH + 1)));
        assert!(AdvanceDiagram::parse(&json(MAX_NEST_DEPTH)).is_ok());
        assert!(AdvanceDiagram::parse(&json(MAX_NEST_DEPTH + 1)).is_err());
    }

    // ------------------------------------------------------------
    // Whole-module review of advance mode
    // ------------------------------------------------------------

    const TWO_LANES: &str = r#"{"lanes":[{"id":"A","title":"A"},{"id":"B","title":"B"}],
        "nodes":[{"id":"n1","lane":"A"},{"id":"n2","lane":"B"},{"id":"n3","lane":"A"}],"edges":[]}"#;

    #[test]
    fn dragged_positions_round_trip_through_the_scene_order() {
        // The scene is what a host reads, drags a node in, and hands
        // back. Consuming it in declaration order made nodes in
        // different lanes trade places.
        let sc = layout_advance(TWO_LANES).unwrap();
        assert_eq!(
            sc.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
            ["n1", "n3", "n2"],
            "test rests on the scene NOT being in declaration order"
        );
        let positions: Vec<f64> = sc.nodes.iter().flat_map(|n| [n.x, n.y]).collect();
        let again = layout(&AdvanceDiagram::parse(TWO_LANES).unwrap());
        let placed =
            place_nodes_at_positions(&AdvanceDiagram::parse(TWO_LANES).unwrap(), &positions);
        for (want, got) in again.nodes.iter().zip(&placed) {
            assert_eq!(want.id, got.id);
            assert!(
                (want.x - got.x).abs() < 1e-9 && (want.y - got.y).abs() < 1e-9,
                "{} moved from ({}, {}) to ({}, {})",
                got.id,
                want.x,
                want.y,
                got.x,
                got.y
            );
        }
        // And the same through the public entry point.
        let svg = render_advance_routed(TWO_LANES, &positions).unwrap();
        for n in &sc.nodes {
            assert!(
                svg.contains(&format!("x=\"{:.1}\" y=\"{:.1}\"", n.x, n.y))
                    || svg.contains(&format!("cx=\"{:.1}\" cy=\"{:.1}\"", n.x, n.y)),
                "{} is not at its scene position",
                n.id
            );
        }
    }

    #[test]
    fn lane_boxes_are_sized_by_their_own_nodes_when_lanes_nest() {
        // `lane_index_map` is a flat depth-first index over ALL lanes,
        // so enumerating top-level lanes against it counted A1's nodes
        // toward B and left B sized as if empty.
        let deep = r#"{"lanes":[{"id":"A","title":"A","children":[{"id":"A1","title":"A1"},{"id":"A2","title":"A2"}]},{"id":"B","title":"B"}],
            "nodes":[{"id":"x","lane":"A1"},{"id":"y","lane":"A2"},
                     {"id":"z1","lane":"B"},{"id":"z2","lane":"B"},{"id":"z3","lane":"B"}],"edges":[]}"#;
        let base = layout_advance(deep).unwrap();
        let positions: Vec<f64> = base.nodes.iter().flat_map(|n| [n.x, n.y]).collect();
        let svg = render_advance_routed_with_lanes(deep, &positions, &[200.0, 200.0], 24.0, 24.0)
            .unwrap();
        // Both lane boxes reach the bottom-most node they actually hold.
        let deepest = base
            .nodes
            .iter()
            .map(|n| n.y + n.h / 2.0)
            .fold(0.0, f64::max);
        let mut lane_h = Vec::new();
        for cap in svg.split("<rect ").skip(1) {
            let g = |k: &str| {
                cap.split(&format!("{k}=\""))
                    .nth(1)
                    .and_then(|s| s.split('"').next())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0)
            };
            if (g("width") - 200.0).abs() < 1e-9 {
                lane_h.push(g("height"));
            }
        }
        assert_eq!(lane_h.len(), 2, "one box per top-level lane");
        for h in lane_h {
            assert!(
                h + 24.0 >= deepest,
                "lane box {h} does not reach the lowest node at {deepest}"
            );
        }
    }

    #[test]
    fn the_top_nodes_own_anchor_wins_even_when_another_is_nearer() {
        // Discarding the globally-nearest anchor when it belonged to an
        // occluded node left the visible node's anchor unreachable.
        let src = r#"{"lanes":[{"id":"l","title":"L"}],
            "nodes":[{"id":"under","lane":"l","x":100,"y":100,"anchors":[{"id":"u","side":"right"}]},
                     {"id":"over","lane":"l","x":110,"y":100,"anchors":[{"id":"o","side":"right"}]}],
            "edges":[]}"#;
        let sc = layout(&AdvanceDiagram::parse(src).unwrap());
        let ui = sc.nodes.iter().position(|n| n.id == "under").unwrap();
        let oi = sc.nodes.iter().position(|n| n.id == "over").unwrap();
        let ua = &sc.nodes[ui].anchors[0];
        let p = (ua.x - 2.0, ua.y);
        assert_eq!(
            sc.node_at(p.0, p.1),
            Some(oi),
            "setup: the point must sit on `over`"
        );
        assert_eq!(
            sc.anchor_at(p.0, p.1, 30.0),
            Some((ui, 0)),
            "setup: `under`'s anchor is the nearest"
        );
        assert_eq!(sc.hit_test(p.0, p.1, 30.0), Some(AdvanceHit::Anchor(oi, 0)));
    }

    #[test]
    fn edge_separators_inside_labels_are_text() {
        let d = text_diagram("lane l \"L\"\na[A] { p[x---y]; q[a-->b] }\n");
        assert_eq!(
            d.nodes[0]
                .elements
                .iter()
                .map(|e| e.label.as_str())
                .collect::<Vec<_>>(),
            ["x---y", "a-->b"]
        );
        let d = text_diagram("lane l \"L\"\na[x==>y]\nb[B]\na --- b\n");
        assert_eq!(d.nodes[0].label, "x==>y");
        assert_eq!((d.edges.len(), d.edges[0].kind), (1, EdgeKind::Open));
        // Separator precedence is unchanged: `a ---> b` still splits at
        // the `-->`, leaving `a -` as the source, exactly as before.
        let e = AdvanceDiagram::parse_text("lane l \"L\"\na[A]\nb[B]\na ---> b\n").unwrap_err();
        assert!(e.message.contains("unknown node 'a -'"), "{}", e.message);
    }

    #[test]
    fn a_one_line_lane_block_takes_bare_ids() {
        let d = text_diagram("lane l \"L\" { a }\n");
        assert_eq!(d.lanes[0].title, "L");
        assert_eq!(
            d.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
            ["a"]
        );
        let d = text_diagram("lane l \"L\" { a[A]; b }\n");
        assert_eq!(d.nodes.len(), 2);
    }

    #[test]
    fn an_anchor_id_with_a_colon_is_refused() {
        // `parse_end` rejects a ':' in an `@`-suffix, so such an anchor
        // could be declared but never referenced.
        let e =
            AdvanceDiagram::parse_text("lane l \"L\"\na[A] { anchor my:x right }\n").unwrap_err();
        assert!(e.message.contains("may not contain ':'"), "{}", e.message);
        let json = r#"{"lanes":[{"id":"l","title":"L"}],
            "nodes":[{"id":"a","lane":"l","anchors":[{"id":"m:x","side":"right"}]}],"edges":[]}"#;
        assert!(AdvanceDiagram::parse(json)
            .unwrap_err()
            .message
            .contains("may not contain ':'"));
    }

    #[test]
    fn a_style_target_matching_no_edge_is_an_error_either_way() {
        let base = "lane l \"L\"\na[A]\nb[B]\na --> b\n";
        // Plain node-pair target: used to be dropped in silence.
        let e =
            AdvanceDiagram::parse_text(&format!("{base}style zz-->yy color:#f00\n")).unwrap_err();
        assert!(
            e.message.contains("matches no edge") && e.message.contains("line 5"),
            "{}",
            e.message
        );
        // Nodes exist but carry no edge between them.
        let e = AdvanceDiagram::parse_text(&format!("{base}style b-->a color:#f00\n")).unwrap_err();
        assert!(e.message.contains("matches no edge"), "{}", e.message);
        // A real target still applies.
        let d = text_diagram(&format!("{base}style a-->b color:#f00\n"));
        assert_eq!(d.edges[0].style.color.as_deref(), Some("#f00"));
    }
    // ---- the channel-grid router -------------------------------------

    /// Every scenario the router has to survive, in one place, so a new
    /// property can be asserted across all of them by adding one loop.
    const ROUTER_CASES: [(&str, &str); 8] = [
        ("blocker between two lanes",
         "lane l \"L\"\na[A]\nlane m \"M\"\nm[Middle blocker]\nlane r \"R\"\nc[C]\na --> c\n"),
        ("blocker inside one lane",
         "lane l \"L\"\na[A]\nb[B]\nc[C]\na --> b\nb --> c\na --> c\n"),
        ("ports on the same side across a wider node",
         "lane l \"L\" {\n a[A]\n m[A much wider middle node]\n b[B]\n}\na:right --> b:right\n"),
        ("three lanes of three, crossing every way",
         "lane a \"A\"\na1[a1]\na2[a2]\na3[a3]\nlane b \"B\"\nb1[b1]\nb2[b2]\nb3[b3]\n\
          lane c \"C\"\nc1[c1]\nc2[c2]\nc3[c3]\na1 --> c3\na3 --> c1\nb1 --> c2\na2 --> b3\nb2 --> a1\n"),
        ("terminals", TERMINALS),
        ("loops on every pair of sides",
         "lane l \"L\"\na[A]\nb[B]\na:left --> a:right\na:top --> a:bottom\n\
          a:right --> a:right\na:right --> a:bottom\nb --> b\n"),
        ("loops between sub-elements of one node",
         "lane l \"L\"\na[A] {\n c0[C0]\n c1[C1]\n}\na.c0 --> a.c1\n"),
        ("five parallel ported edges past a blocker",
         "lane l \"L\" {\n a[A]\n b[B]\n c[C]\n}\na:bottom --> c:top\na:bottom --> c:top\n\
          a:bottom --> c:top\na:bottom --> c:top\na:bottom --> c:top\n"),
    ];

    /// A lead runs from a terminal to its own node's boundary and stops
    /// there; it never comes out the far side. Saying so precisely
    /// matters: "the first segment of its own edge" also describes a
    /// line drawn straight across the node, which is what a broken
    /// self-loop looks like.
    fn is_lead(w: &[(f64, f64)], rect: (f64, f64, f64, f64)) -> bool {
        let (l, t, r, b) = rect;
        let at = |a: f64, c: f64| (a - c).abs() < 1e-9;
        let across_x = (at(w[0].0, l) && at(w[1].0, r)) || (at(w[0].0, r) && at(w[1].0, l));
        let across_y = (at(w[0].1, t) && at(w[1].1, b)) || (at(w[0].1, b) && at(w[1].1, t));
        !across_x && !across_y
    }

    /// No line through a box. The one exception is deliberate: an edge
    /// that starts on a sub-element leads out through its own node to
    /// reach the boundary, on its first or last segment only.
    #[test]
    fn no_route_passes_through_a_node() {
        for (name, src) in ROUTER_CASES {
            let sc = layout(&text_diagram(src));
            for e in &sc.edges {
                let last = e.points.len() - 2;
                for (i, w) in e.points.windows(2).enumerate() {
                    for n in &sc.nodes {
                        let own = (n.id == e.from && i == 0) || (n.id == e.to && i == last);
                        if own && is_lead(w, node_rect(n)) {
                            continue;
                        }
                        assert!(
                            !seg_crosses_rect(w[0], w[1], node_rect(n)),
                            "{name}: {} --> {}, segment {i} {:?}-{:?} runs through {}",
                            e.from,
                            e.to,
                            w[0],
                            w[1],
                            n.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_route_is_orthogonal() {
        for (name, src) in ROUTER_CASES {
            let sc = layout(&text_diagram(src));
            for e in &sc.edges {
                for w in e.points.windows(2) {
                    assert!(
                        (w[0].0 - w[1].0).abs() < 1e-9 || (w[0].1 - w[1].1).abs() < 1e-9,
                        "{name}: diagonal segment {:?}-{:?}",
                        w[0],
                        w[1]
                    );
                }
            }
        }
    }

    /// Three nodes in a column with an edge skipping the middle one: the
    /// long edge has to leave through a side, because going straight
    /// down would run along the two short edges and through `b`.
    #[test]
    fn an_edge_detours_rather_than_run_over_the_edges_already_drawn() {
        let sc = layout(&text_diagram(
            "lane l \"L\"\na[A]\nb[B]\nc[C]\na --> b\nb --> c\na --> c\n",
        ));
        let skip = &sc.edges[2];
        let a = scene_node_by(&sc, "a");
        // It leaves through a side, not the bottom it shares with a --> b.
        assert!(
            (skip.points[0].1 - a.y).abs() < 1e-9,
            "a --> c left through the bottom"
        );
        for w in skip.points.windows(2) {
            for other in [&sc.edges[0], &sc.edges[1]] {
                for o in other.points.windows(2) {
                    assert_eq!(
                        seg_conflict((w[0], w[1]), (o[0], o[1])),
                        Conflict::None,
                        "the skipping edge touches a --> b or b --> c"
                    );
                }
            }
        }
    }

    /// A port is a promise about where the line leaves the box, so the
    /// leader may not be walked back over: the first segment has to
    /// carry the route clear of the node, not stop short inside it.
    #[test]
    fn a_ported_edge_keeps_its_whole_leader() {
        let sc = layout(&text_diagram(
            "lane l \"L\"\na[A]\nb[B]\nc[C]\na:right --> b:left\na:bottom --> b:top\n",
        ));
        let a = scene_node_by(&sc, "a");
        let e = &sc.edges[0];
        assert!((e.points[1].1 - a.y).abs() < 1e-9);
        assert!(
            (e.points[1].0 - (a.x + a.w / 2.0 + PORT_LEAD)).abs() < 1e-9,
            "leader is {:?}, expected {} px clear of the right edge",
            e.points[1],
            PORT_LEAD
        );
    }

    /// Two edges between the same nodes fan apart; two edges through
    /// different ports already land apart and keep their full leaders.
    #[test]
    fn only_edges_that_would_overlap_are_fanned() {
        let sc = layout(&text_diagram(
            "lane l \"L\"\na[A]\nb[B]\na --> b\na --> b\n",
        ));
        assert!(
            (sc.edges[0].points[0].0 - sc.edges[1].points[0].0).abs() > 1.0,
            "identical edges were drawn on top of each other"
        );
    }

    /// Same input, same bytes — the search must not depend on hash order.
    #[test]
    fn routing_is_deterministic() {
        for (name, src) in ROUTER_CASES {
            let d = text_diagram(src);
            let first = to_svg(&layout(&d));
            for _ in 0..4 {
                assert_eq!(to_svg(&layout(&d)), first, "{name} rendered differently");
            }
        }
    }
    /// A loop has to enclose something. Opposite sides in particular
    /// cannot be joined by one corner, and the shape that tried used to
    /// collapse into a line straight across the node — drawn under the
    /// node, so invisible, and unreachable by `hit_test`.
    #[test]
    fn a_self_loop_is_a_loop_on_every_pair_of_sides() {
        let sides = ["left", "right", "top", "bottom"];
        for from in sides {
            for to in sides {
                let src = format!("lane l \"L\"\na[A]\na:{from} --> a:{to}\n");
                let sc = layout(&text_diagram(&src));
                let e = &sc.edges[0];
                let a = scene_node_by(&sc, "a");
                assert!(
                    e.points.len() >= 4,
                    "a:{from} --> a:{to} drew {:?}, which is not a loop",
                    e.points
                );
                for w in e.points.windows(2) {
                    assert!(
                        !seg_crosses_rect(w[0], w[1], node_rect(a)) || is_lead(w, node_rect(a)),
                        "a:{from} --> a:{to} runs through its own node: {:?}",
                        e.points
                    );
                }
            }
        }
    }

    /// Terminals are resolved before the loop is drawn, not after. The
    /// dispatch used to test "same node?" first, so every edge from a
    /// node to itself silently lost the anchor or compartment it named.
    #[test]
    fn a_loop_between_sub_elements_lands_on_them() {
        let sc = layout(&text_diagram(
            "lane l \"L\"\na[A] {\n c0[C0]\n c1[C1]\n}\na.c0 --> a.c1\n",
        ));
        let a = scene_node_by(&sc, "a");
        let e = &sc.edges[0];
        let on = |p: (f64, f64), el: &AdvanceSceneElement| {
            let (l, t, r, b) = (
                el.x - el.w / 2.0,
                el.y - el.h / 2.0,
                el.x + el.w / 2.0,
                el.y + el.h / 2.0,
            );
            let at = |v: f64, c: f64| (v - c).abs() < 1e-9;
            (at(p.0, l) || at(p.0, r)) && p.1 >= t - 1e-9 && p.1 <= b + 1e-9
                || (at(p.1, t) || at(p.1, b)) && p.0 >= l - 1e-9 && p.0 <= r + 1e-9
        };
        assert!(on(e.from_point, &a.elements[0]), "from {:?}", e.from_point);
        assert!(on(e.to_point, &a.elements[1]), "to {:?}", e.to_point);
    }

    /// Edges between the same two points must not be drawn on top of
    /// each other. The fan slides their terminals along the side; when
    /// it lengthened the lead instead, the extra length was collinear
    /// with the route and collapsed straight back out again.
    #[test]
    fn parallel_edges_are_drawn_apart() {
        let sc = layout(&text_diagram(
            "lane l \"L\"\na[A]\nb[B]\na --> b\na --> b\na --> b\n",
        ));
        let starts: Vec<(f64, f64)> = sc.edges.iter().map(|e| e.points[0]).collect();
        for i in 0..starts.len() {
            for j in i + 1..starts.len() {
                let d = (starts[i].0 - starts[j].0).abs() + (starts[i].1 - starts[j].1).abs();
                assert!(
                    d > 1.0,
                    "edges {i} and {j} start at the same point {:?}",
                    starts[i]
                );
            }
        }
    }

    /// An edge that is never drawn must not push one that is. Invisible
    /// edges exist to space the layout, and they were being fed to the
    /// router as obstacles to avoid.
    #[test]
    fn an_invisible_edge_does_not_deflect_a_visible_one() {
        let scene = |extra: &str| {
            let src = format!(
                r#"{{"lanes":[{{"id":"l","title":"L"}}],
                    "nodes":[{{"id":"a","label":"A","lane":"l"}},
                             {{"id":"b","label":"B","lane":"l"}},
                             {{"id":"c","label":"C","lane":"l"}}],
                    "edges":[{extra}{{"from":"a","to":"c"}}]}}"#
            );
            let d = AdvanceDiagram::parse(&src).unwrap();
            let sc = layout(&d);
            let e = sc
                .edges
                .iter()
                .find(|e| e.kind != EdgeKind::Invisible)
                .unwrap();
            e.points.clone()
        };
        let hidden = r#"{"from":"a","to":"b","kind":"invisible"},
                        {"from":"b","to":"c","kind":"invisible"},"#;
        assert_eq!(
            scene(hidden),
            scene(""),
            "an invisible edge moved the visible one"
        );
    }

    /// The guarantee has to hold at a size no one tunes by hand.
    #[test]
    fn a_large_diagram_still_routes_around_every_node() {
        let mut src = String::from("lane l \"L\"\n");
        for i in 0..60 {
            src.push_str(&format!("x{i}[n{i}]\n"));
        }
        for i in 0..120 {
            src.push_str(&format!("x{} --> x{}\n", i % 60, (7 * i + 3) % 60));
        }
        let sc = layout(&text_diagram(&src));
        assert_eq!(sc.edges.len(), 120);
        for e in &sc.edges {
            let last = e.points.len() - 2;
            for (i, w) in e.points.windows(2).enumerate() {
                for n in &sc.nodes {
                    let own = (n.id == e.from && i == 0) || (n.id == e.to && i == last);
                    if own && is_lead(w, node_rect(n)) {
                        continue;
                    }
                    assert!(
                        !seg_crosses_rect(w[0], w[1], node_rect(n)),
                        "{} --> {} runs through {}",
                        e.from,
                        e.to,
                        n.id
                    );
                }
            }
        }
    }
    // ---- negotiated routing --------------------------------------------

    /// Diagrams that admit a crossing-free orthogonal routing must get
    /// one. This is the promise P3 exists to keep, and the number is on
    /// the scene so a host can check it too.
    #[test]
    fn a_planar_diagram_is_routed_without_a_single_crossing() {
        let cases: [(&str, &str); 7] = [
            ("showcase", include_str!("../examples/advance_swimlane.mmd")),
            ("terminals", TERMINALS),
            (
                "a chain down one lane",
                "lane l \"L\"\na[A]\nb[B]\nc[C]\nd[D]\na --> b\nb --> c\nc --> d\n",
            ),
            (
                "a chain that skips two of its own nodes",
                "lane l \"L\"\na[A]\nb[B]\nc[C]\nd[D]\na --> b\nb --> c\nc --> d\na --> d\n",
            ),
            (
                "a fan out and back in",
                "lane l \"L\"\nr[Root]\nlane m \"M\"\nx[X]\ny[Y]\nz[Z]\nlane e \"E\"\n\
                 j[Join]\nr --> x\nr --> y\nr --> z\nx --> j\ny --> j\nz --> j\n",
            ),
            (
                "three lanes of three, crossing every way",
                "lane a \"A\"\na1[a1]\na2[a2]\na3[a3]\nlane b \"B\"\nb1[b1]\nb2[b2]\nb3[b3]\n\
                 lane c \"C\"\nc1[c1]\nc2[c2]\nc3[c3]\na1 --> c3\na3 --> c1\nb1 --> c2\n\
                 a2 --> b3\nb2 --> a1\n",
            ),
            (
                "a blocker in the middle lane",
                "lane l \"L\"\na[A]\nlane m \"M\"\nm[Middle blocker]\nlane r \"R\"\nc[C]\na --> c\n",
            ),
        ];
        for (name, src) in cases {
            let sc = layout(&text_diagram(src));
            assert_eq!(sc.crossings, 0, "{name} was routed with crossings");
        }
    }

    /// `crossings` counts what is actually drawn, so a host can display
    /// it — and so this suite cannot pass by counting nothing.
    #[test]
    fn the_crossing_count_matches_the_geometry() {
        let sc = layout(&text_diagram(
            "lane l \"L\"\na[A]\nb[B]\nc[C]\na --> b\nb --> c\na --> c\n",
        ));
        let routes: Vec<Vec<(f64, f64)>> = sc.edges.iter().map(|e| e.points.clone()).collect();
        let live = vec![true; routes.len()];
        assert_eq!(sc.crossings, crossings_of(&routes, &live).0);
        assert!(
            sc.crossings <= 1,
            "{} crossings on three edges",
            sc.crossings
        );
    }

    /// A diagram with no crossing-free routing still gets drawn, and
    /// the count says so rather than the router giving up or looping.
    #[test]
    fn a_non_planar_diagram_still_renders_and_reports_its_crossings() {
        let mut src = String::from("lane l \"L\"\n");
        for i in 0..12 {
            src.push_str(&format!("x{i}[n{i}]\n"));
        }
        for i in 0..24 {
            src.push_str(&format!("x{} --> x{}\n", i % 12, (5 * i + 3) % 12));
        }
        let sc = layout(&text_diagram(&src));
        assert_eq!(sc.edges.len(), 24);
        assert!(
            sc.crossings > 0,
            "expected this one to be unroutable planar"
        );
        for e in &sc.edges {
            assert!(e.points.len() >= 2, "an edge was dropped");
        }
    }

    /// Negotiating must not make the answer depend on anything but the
    /// input: the rip-up order comes from the crossings, which come
    /// from the routes, which come from the diagram.
    #[test]
    fn negotiated_routing_is_deterministic() {
        let src = "lane a \"A\"\na1[a1]\na2[a2]\na3[a3]\nlane b \"B\"\nb1[b1]\nb2[b2]\nb3[b3]\n\
                   lane c \"C\"\nc1[c1]\nc2[c2]\nc3[c3]\na1 --> c3\na3 --> c1\nb1 --> c2\n\
                   a2 --> b3\nb2 --> a1\n";
        let d = text_diagram(src);
        let first = to_svg(&layout(&d));
        for _ in 0..4 {
            assert_eq!(to_svg(&layout(&d)), first);
        }
    }
}
