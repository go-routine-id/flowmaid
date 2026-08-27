//! Advance / swimlane diagram rendering.
//!
//! Input is a small JSON object describing lanes, nodes inside lanes,
//! and edges between nodes. The engine lays out vertical lanes and
//! top-down nodes, then routes orthogonal edges between them.

use crate::layout::{text_width, BASE_H, LINE_H, MIN_W, PAD_X};
use crate::model::{EdgeKind, Shape};
use crate::scene::{escape, svg_open, SvgOptions};

// ------------------------------------------------------------------
// Public error type
// ------------------------------------------------------------------

/// Something went wrong while parsing or laying out an advance diagram.
#[derive(Debug)]
pub struct AdvanceError {
    pub message: String,
}

impl std::fmt::Display for AdvanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

fn adv_err(message: impl Into<String>) -> AdvanceError {
    AdvanceError {
        message: message.into(),
    }
}

// ------------------------------------------------------------------
// Public model
// ------------------------------------------------------------------

/// One swimlane column.
#[derive(Debug, Clone)]
pub struct AdvanceLane {
    pub id: String,
    pub title: String,
}

/// One node inside a lane.
#[derive(Debug, Clone)]
pub struct AdvanceNode {
    pub id: String,
    pub label: String,
    pub lane: String,
    pub shape: Shape,
}

/// One edge between two nodes.
#[derive(Debug, Clone)]
pub struct AdvanceEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: EdgeKind,
}

/// Parsed advance diagram, ready for layout.
#[derive(Debug, Default)]
pub struct AdvanceDiagram {
    pub lanes: Vec<AdvanceLane>,
    pub nodes: Vec<AdvanceNode>,
    pub edges: Vec<AdvanceEdge>,
}

/// Positioned geometry for an advance diagram.
#[derive(Debug, Clone)]
pub struct AdvanceScene {
    pub width: f64,
    pub height: f64,
    pub lanes: Vec<AdvanceSceneLane>,
    pub nodes: Vec<AdvanceSceneNode>,
    pub edges: Vec<AdvanceSceneEdge>,
}

#[derive(Debug, Clone)]
pub struct AdvanceSceneLane {
    pub id: String,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone)]
pub struct AdvanceSceneNode {
    pub id: String,
    pub label: String,
    pub lane: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub shape: Shape,
}

#[derive(Debug, Clone)]
pub struct AdvanceSceneEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: EdgeKind,
    pub points: Vec<(f64, f64)>,
}

// ------------------------------------------------------------------
// Minimal zero-dependency JSON parser (subset sufficient for advance input)
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

struct JsonParser<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        Self { s, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.s[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn parse(&mut self) -> Result<JsonValue, AdvanceError> {
        self.skip_ws();
        self.parse_value()
    }

    fn parse_value(&mut self) -> Result<JsonValue, AdvanceError> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('t') => self.expect_word("true").map(|_| JsonValue::Bool(true)),
            Some('f') => self.expect_word("false").map(|_| JsonValue::Bool(false)),
            Some('n') => self.expect_word("null").map(|_| JsonValue::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            other => Err(adv_err(format!(
                "expected JSON value, got {:?}",
                other
            ))),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, AdvanceError> {
        self.bump(); // '{'
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(JsonValue::Object(pairs));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            if self.bump() != Some(':') {
                return Err(adv_err("expected ':' after object key"));
            }
            let value = self.parse_value()?;
            pairs.push((key, value));
            self.skip_ws();
            match self.bump() {
                Some(',') => continue,
                Some('}') => break,
                other => return Err(adv_err(format!(
                    "expected ',' or '}}' in object, got {:?}",
                    other
                ))),
            }
        }
        Ok(JsonValue::Object(pairs))
    }

    fn parse_array(&mut self) -> Result<JsonValue, AdvanceError> {
        self.bump(); // '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(JsonValue::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.bump() {
                Some(',') => continue,
                Some(']') => break,
                other => return Err(adv_err(format!(
                    "expected ',' or ']' in array, got {:?}",
                    other
                ))),
            }
        }
        Ok(JsonValue::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, AdvanceError> {
        if self.bump() != Some('"') {
            return Err(adv_err("expected string"));
        }
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{0008}'),
                    Some('f') => out.push('\u{000c}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        let hex: String = (0..4)
                            .filter_map(|_| self.bump())
                            .collect();
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| adv_err("invalid unicode escape"))?;
                        let c = char::from_u32(code)
                            .ok_or_else(|| adv_err("invalid unicode codepoint"))?;
                        out.push(c);
                    }
                    other => return Err(adv_err(format!(
                        "invalid escape sequence {:?}",
                        other
                    ))),
                },
                Some(c) => out.push(c),
                None => return Err(adv_err("unterminated string")),
            }
        }
        Ok(out)
    }

    fn parse_number(&mut self) -> Result<JsonValue, AdvanceError> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.bump();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }
        if self.peek() == Some('.') {
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        if let Some(c) = self.peek() {
            if c == 'e' || c == 'E' {
                self.bump();
                if let Some(c2) = self.peek() {
                    if c2 == '+' || c2 == '-' {
                        self.bump();
                    }
                }
                while let Some(c2) = self.peek() {
                    if c2.is_ascii_digit() {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
        }
        let num_str = &self.s[start..self.pos];
        num_str
            .parse::<f64>()
            .map(JsonValue::Number)
            .map_err(|_| adv_err(format!("invalid number {}", num_str)))
    }

    fn expect_word(&mut self, word: &str) -> Result<(), AdvanceError> {
        for c in word.chars() {
            if self.bump() != Some(c) {
                return Err(adv_err(format!("expected '{}'", word)));
            }
        }
        Ok(())
    }
}

fn parse_json(source: &str) -> Result<JsonValue, AdvanceError> {
    let mut p = JsonParser::new(source);
    let value = p.parse()?;
    p.skip_ws();
    if p.pos != p.s.len() {
        return Err(adv_err("trailing data after JSON value"));
    }
    Ok(value)
}

// ------------------------------------------------------------------
// Parse advance diagram from JSON
// ------------------------------------------------------------------

fn obj_get<'a>(obj: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn as_str(v: &JsonValue) -> Option<&str> {
    match v {
        JsonValue::String(s) => Some(s),
        _ => None,
    }
}

fn as_array(v: &JsonValue) -> Option<&[JsonValue]> {
    match v {
        JsonValue::Array(a) => Some(a),
        _ => None,
    }
}

fn as_object(v: &JsonValue) -> Option<&[(String, JsonValue)]> {
    match v {
        JsonValue::Object(o) => Some(o),
        _ => None,
    }
}

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
        other => Err(adv_err(format!("unknown shape '{}'", other))),
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
        other => Err(adv_err(format!("unknown edge kind '{}'", other))),
    }
}

impl AdvanceDiagram {
    pub fn parse(source: &str) -> Result<Self, AdvanceError> {
        let json = parse_json(source)?;
        let obj = as_object(&json).ok_or_else(|| adv_err("advance source must be a JSON object"))?;

        let lanes_arr = obj_get(obj, "lanes").and_then(as_array).ok_or_else(|| {
            adv_err("advance source must have a 'lanes' array")
        })?;
        let nodes_arr = obj_get(obj, "nodes").and_then(as_array).ok_or_else(|| {
            adv_err("advance source must have a 'nodes' array")
        })?;
        let edges_arr = obj_get(obj, "edges").and_then(as_array).unwrap_or(&[]);

        let mut lanes = Vec::new();
        let mut lane_ids = std::collections::HashSet::new();
        for (i, lane_json) in lanes_arr.iter().enumerate() {
            let lane_obj = as_object(lane_json).ok_or_else(|| {
                adv_err(format!("lanes[{}] must be an object", i))
            })?;
            let id = obj_get(lane_obj, "id")
                .and_then(as_str)
                .ok_or_else(|| adv_err(format!("lanes[{}] missing 'id'", i)))?
                .to_string();
            if !lane_ids.insert(id.clone()) {
                return Err(adv_err(format!("duplicate lane id '{}'", id)));
            }
            let title = obj_get(lane_obj, "title")
                .and_then(as_str)
                .unwrap_or(&id)
                .to_string();
            lanes.push(AdvanceLane { id, title });
        }

        let mut nodes = Vec::new();
        let mut node_ids = std::collections::HashSet::new();
        for (i, node_json) in nodes_arr.iter().enumerate() {
            let node_obj = as_object(node_json).ok_or_else(|| {
                adv_err(format!("nodes[{}] must be an object", i))
            })?;
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
            nodes.push(AdvanceNode {
                id,
                label,
                lane,
                shape,
            });
        }

        let mut edges = Vec::new();
        for (i, edge_json) in edges_arr.iter().enumerate() {
            let edge_obj = as_object(edge_json).ok_or_else(|| {
                adv_err(format!("edges[{}] must be an object", i))
            })?;
            let from = obj_get(edge_obj, "from")
                .and_then(as_str)
                .ok_or_else(|| adv_err(format!("edges[{}] missing 'from'", i)))?
                .to_string();
            let to = obj_get(edge_obj, "to")
                .and_then(as_str)
                .ok_or_else(|| adv_err(format!("edges[{}] missing 'to'", i)))?
                .to_string();
            if !node_ids.contains(&from) {
                return Err(adv_err(format!(
                    "edges[{}] references unknown node '{}'",
                    i, from
                )));
            }
            if !node_ids.contains(&to) {
                return Err(adv_err(format!(
                    "edges[{}] references unknown node '{}'",
                    i, to
                )));
            }
            let label = obj_get(edge_obj, "label").and_then(as_str).map(|s| s.to_string());
            let kind = obj_get(edge_obj, "kind")
                .and_then(as_str)
                .map(parse_edge_kind)
                .transpose()?
                .unwrap_or(EdgeKind::Arrow);
            edges.push(AdvanceEdge { from, to, label, kind });
        }

        Ok(AdvanceDiagram {
            lanes,
            nodes,
            edges,
        })
    }
}

// ------------------------------------------------------------------
// Layout
// ------------------------------------------------------------------

const MARGIN: f64 = 24.0;
const LANE_GAP: f64 = 40.0;
const NODE_GAP_Y: f64 = 48.0;
const LANE_PAD_X: f64 = 20.0;
const LANE_PAD_Y: f64 = 40.0;
const LANE_TITLE_H: f64 = 26.0;

fn node_size(node: &AdvanceNode) -> (f64, f64) {
    let tw = node.label.split('\n').map(text_width).fold(0.0, f64::max);
    let extra = (node.label.split('\n').count().saturating_sub(1)) as f64 * LINE_H;
    let base_h = BASE_H + extra;
    match node.shape {
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
    }
}

fn lane_index_map(d: &AdvanceDiagram) -> std::collections::HashMap<String, usize> {
    d.lanes
        .iter()
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

/// Route orthogonal edges between already-positioned nodes.
fn route_edges(d: &AdvanceDiagram, nodes: &[AdvanceSceneNode]) -> Vec<AdvanceSceneEdge> {
    let node_idx = node_index_map(d);
    let mut edge_scenes = Vec::with_capacity(d.edges.len());

    for e in &d.edges {
        let from_i = node_idx[&e.from];
        let to_i = node_idx[&e.to];
        let a = &nodes[from_i];
        let b = &nodes[to_i];

        let points = if a.lane == b.lane {
            // Same lane: straight vertical connection (top/bottom centre).
            // If the host dragged nodes horizontally apart, add one bend
            // in the middle so the edge never crosses a node.
            if a.y < b.y {
                let p0 = (a.x, a.y + a.h / 2.0);
                let p3 = (b.x, b.y - b.h / 2.0);
                if (a.x - b.x).abs() < f64::EPSILON {
                    vec![p0, p3]
                } else {
                    let mid_y = (p0.1 + p3.1) / 2.0;
                    vec![p0, (a.x, mid_y), (b.x, mid_y), p3]
                }
            } else {
                let p0 = (a.x, a.y - a.h / 2.0);
                let p3 = (b.x, b.y + b.h / 2.0);
                if (a.x - b.x).abs() < f64::EPSILON {
                    vec![p0, p3]
                } else {
                    let mid_y = (p0.1 + p3.1) / 2.0;
                    vec![p0, (a.x, mid_y), (b.x, mid_y), p3]
                }
            }
        } else {
            // Cross-lane: exit right side of source, enter left side of target.
            let p0 = (a.x + a.w / 2.0, a.y);
            let p3 = (b.x - b.w / 2.0, b.y);
            let mid_x = (p0.0 + p3.0) / 2.0;
            vec![p0, (mid_x, p0.1), (mid_x, p3.1), p3]
        };

        edge_scenes.push(AdvanceSceneEdge {
            from: e.from.clone(),
            to: e.to.clone(),
            label: e.label.clone(),
            kind: e.kind,
            points,
        });
    }

    edge_scenes
}

/// Compute the positioned geometry for an advance diagram.
pub fn layout(d: &AdvanceDiagram) -> AdvanceScene {
    let lane_idx = lane_index_map(d);

    // Compute size of every node.
    let sizes: Vec<(f64, f64)> = d.nodes.iter().map(node_size).collect();

    // Compute required lane width: max node width + horizontal padding.
    let mut lane_widths: Vec<f64> = vec![0.0; d.lanes.len()];
    for (i, n) in d.nodes.iter().enumerate() {
        let li = lane_idx[&n.lane];
        lane_widths[li] = lane_widths[li].max(sizes[i].0);
    }
    for w in &mut lane_widths {
        *w = (*w + 2.0 * LANE_PAD_X).max(120.0);
    }

    // Compute required lane height: title + nodes + gaps + padding.
    let mut lane_heights: Vec<f64> = vec![LANE_TITLE_H + LANE_PAD_Y; d.lanes.len()];
    for (i, n) in d.nodes.iter().enumerate() {
        let li = lane_idx[&n.lane];
        lane_heights[li] += sizes[i].1 + NODE_GAP_Y;
    }
    // Remove trailing gap for each lane.
    for h in &mut lane_heights {
        *h = (*h - NODE_GAP_Y + LANE_PAD_Y).max(120.0);
    }

    // Position lanes horizontally.
    let mut lane_scenes: Vec<AdvanceSceneLane> = Vec::with_capacity(d.lanes.len());
    let mut x = MARGIN;
    let mut total_height: f64 = 0.0;
    for (i, lane) in d.lanes.iter().enumerate() {
        let w = lane_widths[i];
        let h = lane_heights[i];
        lane_scenes.push(AdvanceSceneLane {
            id: lane.id.clone(),
            title: lane.title.clone(),
            x,
            y: MARGIN,
            w,
            h,
        });
        x += w + LANE_GAP;
        total_height = total_height.max(h);
    }
    let total_width = if d.lanes.is_empty() {
        MARGIN * 2.0
    } else {
        x - LANE_GAP + MARGIN
    };
    total_height += 2.0 * MARGIN;

    // Position nodes inside each lane (centre coordinates).
    let mut node_scenes: Vec<AdvanceSceneNode> = Vec::with_capacity(d.nodes.len());
    let mut lane_cursor_y: Vec<f64> = vec![MARGIN + LANE_TITLE_H + LANE_PAD_Y; d.lanes.len()];
    // Preserve declaration order: nodes are laid out in the order they
    // appear inside each lane. Iterate once and bucket by lane.
    let mut lane_node_lists: Vec<Vec<usize>> = vec![Vec::new(); d.lanes.len()];
    for (i, n) in d.nodes.iter().enumerate() {
        let li = lane_idx[&n.lane];
        lane_node_lists[li].push(i);
    }
    for (li, indices) in lane_node_lists.iter().enumerate() {
        let lane = &lane_scenes[li];
        for &ni in indices {
            let n = &d.nodes[ni];
            let (nw, nh) = sizes[ni];
            let cx = lane.x + lane.w / 2.0;
            let cy = lane_cursor_y[li] + nh / 2.0;
            lane_cursor_y[li] += nh + NODE_GAP_Y;
            node_scenes.push(AdvanceSceneNode {
                id: n.id.clone(),
                label: n.label.clone(),
                lane: n.lane.clone(),
                x: cx,
                y: cy,
                w: nw,
                h: nh,
                shape: n.shape,
            });
        }
    }

    // Route edges orthogonally between the positioned nodes.
    let edge_scenes = route_edges(d, &node_scenes);

    AdvanceScene {
        width: total_width,
        height: total_height,
        lanes: lane_scenes,
        nodes: node_scenes,
        edges: edge_scenes,
    }
}

// ------------------------------------------------------------------
// SVG renderer
// ------------------------------------------------------------------

const FONT_SIZE: u32 = 13;
const LANE_FILL: &str = "#f9fafd";
const LANE_STROKE: &str = "#d5d9ec";
const EDGE_COLOR: &str = "#44507a";
const TEXT_COLOR: &str = "#232840";

fn shape_style(shape: Shape) -> (String, String) {
    let ss = crate::style::shape_style(shape);
    (ss.fill.to_string(), ss.stroke.to_string())
}

fn render_node(s: &mut String, n: &AdvanceSceneNode) {
    let cx = n.x;
    let cy = n.y;
    let w = n.w;
    let h = n.h;
    let (fill, stroke) = shape_style(n.shape);
    let style = format!("fill=\"{}\" stroke=\"{}\" stroke-width=\"1.6\"", fill, stroke);

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
                 fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.6\"/>\n",
                l = l, r = r, ty = t + ry, by = b - ry, rx = w / 2.0, ry = ry,
                stroke = stroke, style = style,
            ));
        }
        Shape::Subroutine => {
            let (l, t) = (cx - w / 2.0, cy - h / 2.0);
            s.push_str(&format!(
                "<rect x=\"{l:.1}\" y=\"{t:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"3\" {style}/>\n\
                 <line x1=\"{l1:.1}\" y1=\"{t:.1}\" x2=\"{l1:.1}\" y2=\"{b:.1}\" stroke=\"{stroke}\" stroke-width=\"1.6\"/>\n\
                 <line x1=\"{r1:.1}\" y1=\"{t:.1}\" x2=\"{r1:.1}\" y2=\"{b:.1}\" stroke=\"{stroke}\" stroke-width=\"1.6\"/>\n",
                l = l, t = t, w = w, h = h, b = t + h, l1 = l + 8.0, r1 = l + w - 8.0,
                stroke = stroke, style = style,
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
                    l + k, t, r, t, r - k, b, l, b
                )
            } else {
                format!(
                    "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                    l, t, r - k, t, r, b, l + k, b
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

    // Label.
    let lines: Vec<&str> = n.label.split('\n').collect();
    let line_count = lines.len();
    let start_y = if line_count == 1 {
        cy
    } else {
        cy - ((line_count - 1) as f64 * LINE_H) / 2.0
    };
    for (i, line) in lines.iter().enumerate() {
        let y = start_y + i as f64 * LINE_H;
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-size=\"{}\" fill=\"{}\">{}</text>\n",
            cx,
            y,
            FONT_SIZE,
            TEXT_COLOR,
            escape(line)
        ));
    }
}

/// Render an advance scene to an SVG string.
pub fn to_svg(sc: &AdvanceScene) -> String {
    to_svg_with(sc, &SvgOptions::default())
}

/// Render with explicit viewport options.
pub fn to_svg_with(sc: &AdvanceScene, opts: &SvgOptions) -> String {
    let mut s = String::new();
    svg_open(
        &mut s,
        sc.width,
        sc.height,
        FONT_SIZE,
        "Advance diagram",
        opts,
    );

    s.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" \
         markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
         <path d=\"M 0 1 L 9 5 L 0 9 z\" fill=\"{}\"/></marker></defs>\n",
        EDGE_COLOR
    ));

    // Lane backgrounds.
    for lane in &sc.lanes {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"2\" rx=\"8\"/>\n",
            lane.x, lane.y, lane.w, lane.h, LANE_FILL, LANE_STROKE
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{}\" font-weight=\"bold\" \
             fill=\"{}\">{}</text>\n",
            lane.x + LANE_PAD_X,
            lane.y + LANE_TITLE_H - 6.0,
            FONT_SIZE,
            TEXT_COLOR,
            escape(&lane.title)
        ));
    }

    // Edges.
    for e in &sc.edges {
        if matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        let (dash, sw) = match e.kind {
            EdgeKind::Dotted | EdgeKind::DottedOpen => (" stroke-dasharray=\"5 4\"", 1.7),
            EdgeKind::Thick | EdgeKind::ThickOpen => ("", 3.4),
            _ => ("", 1.7),
        };
        let marker = if e.kind.has_arrow() {
            " marker-end=\"url(#arrow)\""
        } else {
            ""
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
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>\n",
            d, EDGE_COLOR, sw, dash, marker
        ));
        if let Some(label) = &e.label {
            // Place the label beside the longest vertical segment so
            // short same-lane labels don't sit on top of nodes.
            let mut best_vert: Option<(f64, f64, f64)> = None; // (x, y_mid, len)
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
            let (lx, ly) = if let Some((x, y, len)) = best_vert {
                if len >= 22.0 {
                    (x + 8.0, y)
                } else {
                    let mid = e.points[e.points.len() / 2];
                    (mid.0, mid.1)
                }
            } else {
                let mid = e.points[e.points.len() / 2];
                (mid.0, mid.1)
            };
            let lw = text_width(label) + 14.0;
            s.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"18\" \
                 fill=\"#ffffff\" stroke=\"{}\" stroke-width=\"1\" rx=\"3\"/>\n",
                lx - lw / 2.0,
                ly - 9.0,
                lw,
                LANE_STROKE
            ));
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                 font-size=\"{}\" fill=\"{}\">{}</text>\n",
                lx,
                ly,
                FONT_SIZE,
                TEXT_COLOR,
                escape(label)
            ));
        }
    }

    // Nodes (on top of lane backgrounds and edges).
    for n in &sc.nodes {
        render_node(&mut s, n);
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

/// Build lane boxes that tightly contain the given nodes, plus title
/// padding. Used when the host supplies node positions (drag mode).
fn build_lanes_around_nodes(
    d: &AdvanceDiagram,
    nodes: &[AdvanceSceneNode],
) -> (Vec<AdvanceSceneLane>, f64, f64) {
    let lane_idx = lane_index_map(d);
    let mut bounds: Vec<Option<(f64, f64, f64, f64)>> = vec![None; d.lanes.len()];

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

    let mut lanes = Vec::with_capacity(d.lanes.len());
    let mut max_right: f64 = 0.0;
    let mut max_bottom: f64 = 0.0;

    for (i, lane) in d.lanes.iter().enumerate() {
        let (l, t, r, b) = bounds[i]
            .unwrap_or((MARGIN, MARGIN, MARGIN + 120.0, MARGIN + 120.0));
        let x = l - LANE_PAD_X;
        let y = (t - LANE_TITLE_H - LANE_PAD_Y).min(MARGIN);
        let w = (r - x + LANE_PAD_X).max(120.0);
        let h = (b - y + LANE_PAD_Y).max(LANE_TITLE_H + 2.0 * LANE_PAD_Y);

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

    (lanes, max_right + MARGIN, max_bottom + MARGIN)
}

/// Parse advance JSON, place nodes at caller-provided centre positions
/// (flat `[x0, y0, x1, y1, ...]` in the same order as the `nodes`
/// array emitted by [`layout_advance`]), recompute lane boxes and edge
/// routing, and render to SVG.
pub fn render_advance_routed(source: &str, positions: &[f64]) -> Result<String, AdvanceError> {
    let d = AdvanceDiagram::parse(source)?;
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

    let sizes: Vec<(f64, f64)> = d.nodes.iter().map(node_size).collect();
    let node_scenes: Vec<AdvanceSceneNode> = d
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| AdvanceSceneNode {
            id: n.id.clone(),
            label: n.label.clone(),
            lane: n.lane.clone(),
            x: positions[i * 2],
            y: positions[i * 2 + 1],
            w: sizes[i].0,
            h: sizes[i].1,
            shape: n.shape,
        })
        .collect();

    let (lane_scenes, width, height) = build_lanes_around_nodes(&d, &node_scenes);
    let edge_scenes = route_edges(&d, &node_scenes);

    Ok(to_svg(&AdvanceScene {
        width,
        height,
        lanes: lane_scenes,
        nodes: node_scenes,
        edges: edge_scenes,
    }))
}

/// Build lane boxes from caller-provided widths, margin, and gap.
///
/// Lane `i` gets width `lane_widths[i]`; its x coordinate starts at
/// `margin` plus the sum of previous widths and gaps.  The height is
/// large enough for all nodes in that lane plus vertical padding, with
/// a minimum that always accommodates the lane title.
fn build_lanes_with_widths(
    d: &AdvanceDiagram,
    nodes: &[AdvanceSceneNode],
    lane_widths: &[f64],
    margin: f64,
    gap: f64,
) -> (Vec<AdvanceSceneLane>, f64, f64) {
    let lane_idx = lane_index_map(d);
    let mut lane_scenes = Vec::with_capacity(d.lanes.len());
    let mut x = margin;
    let mut total_height: f64 = 0.0;

    for (i, lane) in d.lanes.iter().enumerate() {
        let w = lane_widths[i];
        let lane_nodes: Vec<&AdvanceSceneNode> = nodes
            .iter()
            .filter(|n| lane_idx[&n.lane] == i)
            .collect();

        let h = if lane_nodes.is_empty() {
            LANE_TITLE_H + 2.0 * LANE_PAD_Y
        } else {
            let max_bottom = lane_nodes
                .iter()
                .map(|n| n.y + n.h / 2.0)
                .fold(f64::NEG_INFINITY, f64::max);
            (max_bottom - margin + LANE_PAD_Y).max(LANE_TITLE_H + 2.0 * LANE_PAD_Y)
        };

        lane_scenes.push(AdvanceSceneLane {
            id: lane.id.clone(),
            title: lane.title.clone(),
            x,
            y: margin,
            w,
            h,
        });

        x += w + gap;
        total_height = total_height.max(h);
    }

    let total_width = if d.lanes.is_empty() {
        margin * 2.0
    } else {
        x - gap + margin
    };
    let total_height = total_height + 2.0 * margin;

    (lane_scenes, total_width, total_height)
}

/// Parse advance JSON, place nodes at caller-provided centre positions
/// (flat `[x0, y0, x1, y1, ...]` in the same order as the `nodes`
/// array), and render to SVG using caller-provided lane widths,
/// margin, and inter-lane gap.
///
/// This is the engine side of resizable swimlane columns: the web host
/// drags the border between two lanes and tells the engine the new
/// widths, while node positions remain under host control.
pub fn render_advance_routed_with_lanes(
    source: &str,
    positions: &[f64],
    lane_widths: &[f64],
    margin: f64,
    gap: f64,
) -> Result<String, AdvanceError> {
    let d = AdvanceDiagram::parse(source)?;

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

    let sizes: Vec<(f64, f64)> = d.nodes.iter().map(node_size).collect();
    let node_scenes: Vec<AdvanceSceneNode> = d
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| AdvanceSceneNode {
            id: n.id.clone(),
            label: n.label.clone(),
            lane: n.lane.clone(),
            x: positions[i * 2],
            y: positions[i * 2 + 1],
            w: sizes[i].0,
            h: sizes[i].1,
            shape: n.shape,
        })
        .collect();

    let (lane_scenes, width, height) =
        build_lanes_with_widths(&d, &node_scenes, lane_widths, margin, gap);
    let edge_scenes = route_edges(&d, &node_scenes);

    Ok(to_svg(&AdvanceScene {
        width,
        height,
        lanes: lane_scenes,
        nodes: node_scenes,
        edges: edge_scenes,
    }))
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
        let svg = render_advance_routed_with_lanes(sample_json(), &pos, &[200.0, 200.0], 24.0, 40.0).unwrap();
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
            let svg = render_advance_routed_with_lanes(sample_json(), &pos, &[300.0, 150.0], 10.0, 20.0).unwrap();
            // Parse width out of the SVG root to verify total width.
            let start = svg.find("width=\"").unwrap() + 7;
            let end = svg[start..].find('"').unwrap() + start;
            svg[start..end].parse::<f64>().unwrap()
        };
        // margin + 300 + gap + 150 + margin = 10 + 300 + 20 + 150 + 10 = 490
        assert!((sc - 490.0).abs() < 1.0, "expected width ~490, got {}", sc);
    }

    #[test]
    fn routed_with_lanes_rejects_wrong_width_count() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let pos: Vec<f64> = d.nodes.iter().flat_map(|_| [0.0, 0.0]).collect();
        assert!(render_advance_routed_with_lanes(sample_json(), &pos, &[200.0], 24.0, 40.0).is_err());
    }

    #[test]
    fn routed_with_lanes_rejects_non_positive_width() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let pos: Vec<f64> = d.nodes.iter().flat_map(|_| [0.0, 0.0]).collect();
        assert!(render_advance_routed_with_lanes(sample_json(), &pos, &[200.0, -10.0], 24.0, 40.0).is_err());
        assert!(render_advance_routed_with_lanes(sample_json(), &pos, &[200.0, f64::NAN], 24.0, 40.0).is_err());
    }

    #[test]
    fn routed_with_lanes_rejects_bad_margin_gap() {
        let d = AdvanceDiagram::parse(sample_json()).unwrap();
        let pos: Vec<f64> = d.nodes.iter().flat_map(|_| [0.0, 0.0]).collect();
        assert!(render_advance_routed_with_lanes(sample_json(), &pos, &[200.0, 200.0], -1.0, 40.0).is_err());
        assert!(render_advance_routed_with_lanes(sample_json(), &pos, &[200.0, 200.0], 24.0, -1.0).is_err());
    }
}
