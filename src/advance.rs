//! Advance / swimlane diagram rendering.
//!
//! Input is a small JSON object describing lanes, nodes inside lanes,
//! and edges between nodes. The engine lays out vertical or horizontal lanes,
//! orders nodes top-down (or left-to-right), and routes orthogonal edges.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::json::{as_array, as_number, as_object, as_str, escape_json_str, obj_get, parse_json, JsonValue};
use crate::layout::{text_width, BASE_H, LINE_H, MIN_W, PAD_X};
use crate::model::{EdgeKind, Shape};
use crate::scene::{escape, svg_open, SvgOptions};

static MARKER_COUNTER: AtomicUsize = AtomicUsize::new(1);

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
}

/// One edge between two nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: EdgeKind,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdvanceSceneEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: EdgeKind,
    pub points: Vec<(f64, f64)>,
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
// Parsing AdvanceDiagram
// ------------------------------------------------------------------

fn parse_lane_recursive(
    v: &JsonValue,
    depth: usize,
    lane_ids: &mut std::collections::HashSet<String>,
) -> Result<AdvanceLane, AdvanceError> {
    if depth > 16 {
        return Err(adv_err("lane nesting depth exceeds limit of 16"));
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
    Ok(AdvanceLane { id, title, children })
}

impl AdvanceDiagram {
    pub fn parse(source: &str) -> Result<Self, AdvanceError> {
        let json = parse_json(source)?;
        let obj = as_object(&json).ok_or_else(|| adv_err("advance source must be a JSON object"))?;

        let title = obj_get(obj, "title").and_then(as_str).map(|s| s.to_string());
        let description = obj_get(obj, "description").and_then(as_str).map(|s| s.to_string());

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
            let style_obj = as_object(style_val)
                .ok_or_else(|| adv_err("'style' must be an object"))?;
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
            let cfg_obj = as_object(cfg_val)
                .ok_or_else(|| adv_err("'config' must be an object"))?;
            for (k, v) in cfg_obj {
                match k.as_str() {
                    "margin" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.margin must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err("config.margin must be a non-negative finite number"));
                        }
                        config.margin = num;
                    }
                    "lane_gap" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.lane_gap must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err("config.lane_gap must be a non-negative finite number"));
                        }
                        config.lane_gap = num;
                    }
                    "node_gap_y" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.node_gap_y must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err("config.node_gap_y must be a non-negative finite number"));
                        }
                        config.node_gap_y = num;
                    }
                    "lane_pad_x" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.lane_pad_x must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err("config.lane_pad_x must be a non-negative finite number"));
                        }
                        config.lane_pad_x = num;
                    }
                    "lane_pad_y" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.lane_pad_y must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err("config.lane_pad_y must be a non-negative finite number"));
                        }
                        config.lane_pad_y = num;
                    }
                    "lane_title_h" => {
                        let num = as_number(v)
                            .ok_or_else(|| adv_err("config.lane_title_h must be a number"))?;
                        if !num.is_finite() || num < 0.0 {
                            return Err(adv_err("config.lane_title_h must be a non-negative finite number"));
                        }
                        config.lane_title_h = num;
                    }
                    "order" => {
                        let ord_s = as_str(v)
                            .ok_or_else(|| adv_err("config.order must be a string"))?;
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

        let lanes_arr = obj_get(obj, "lanes").and_then(as_array).ok_or_else(|| {
            adv_err("advance source must have a 'lanes' array")
        })?;
        let nodes_arr = obj_get(obj, "nodes").and_then(as_array).ok_or_else(|| {
            adv_err("advance source must have a 'nodes' array")
        })?;
        let edges_arr = obj_get(obj, "edges").and_then(as_array).unwrap_or(&[]);

        let mut lanes = Vec::new();
        let mut lane_ids = std::collections::HashSet::new();
        for lane_json in lanes_arr {
            lanes.push(parse_lane_recursive(lane_json, 1, &mut lane_ids)?);
        }

        // Non-goal enforcement: horizontal mode + nested lanes is rejected
        if direction == AdvanceDirection::Horizontal && lanes.iter().any(|l| !l.children.is_empty()) {
            return Err(adv_err(
                "nested lanes (children) are not supported in horizontal direction",
            ));
        }

        let mut nodes = Vec::new();
        let mut node_ids = std::collections::HashSet::new();
        let mut explicit_coords_count = 0;

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
                    return Err(adv_err(format!("nodes[{}].w must be a positive finite number", i)));
                }
            }
            if let Some(vh) = h {
                if !vh.is_finite() || vh <= 0.0 {
                    return Err(adv_err(format!("nodes[{}].h must be a positive finite number", i)));
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

            nodes.push(AdvanceNode {
                id,
                label,
                lane,
                shape,
                x,
                y,
                w,
                h,
            });
        }

        if explicit_coords_count > 0 && explicit_coords_count != nodes.len() {
            return Err(adv_err(
                "if any node specifies explicit coordinates (x, y), all nodes must specify them",
            ));
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
            escape_json_str(&e.from),
            escape_json_str(&e.to),
            edge_kind_name(e.kind)
        ));
        if let Some(lbl) = &e.label {
            s.push_str(",\"label\":");
            s.push_str(&escape_json_str(lbl));
        }
        s.push('}');
    }
    s.push(']');

    s.push('}');
    s
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
            "{{\"id\":{},\"label\":{},\"lane\":{},\"x\":{:.1},\"y\":{:.1},\"w\":{:.1},\"h\":{:.1},\"shape\":\"{}\"}}",
            escape_json_str(&node.id),
            escape_json_str(&node.label),
            escape_json_str(&node.lane),
            node.x,
            node.y,
            node.w,
            node.h,
            shape_name(node.shape)
        ));
    }
    s.push(']');

    s.push_str(",\"edges\":[");
    for (i, edge) in sc.edges.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"from\":{},\"to\":{},\"kind\":\"{}\",\"points\":[",
            escape_json_str(&edge.from),
            escape_json_str(&edge.to),
            edge_kind_name(edge.kind)
        ));
        if let Some(lbl) = &edge.label {
            s.push_str(&format!("\"label\":{},", escape_json_str(lbl)));
        }
        for (j, p) in edge.points.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            s.push_str(&format!("[{:.1},{:.1}]", p.0, p.1));
        }
        s.push_str("]}");
    }
    s.push(']');

    s.push('}');
    s
}

// ------------------------------------------------------------------
// Layout & Geometry
// ------------------------------------------------------------------

fn node_size(node: &AdvanceNode) -> (f64, f64) {
    if let (Some(w), Some(h)) = (node.w, node.h) {
        return (w, h);
    }
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
    (node.w.unwrap_or(calc_w), node.h.unwrap_or(calc_h))
}

fn collect_all_lanes<'a>(lanes: &'a [AdvanceLane], out: &mut Vec<&'a AdvanceLane>) {
    for l in lanes {
        out.push(l);
        collect_all_lanes(&l.children, out);
    }
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
        let b_val = if count > 0 { sum / count as f64 } else { f64::INFINITY };
        barycenters.push((u, b_val));
    }

    // Stable sort by barycenter if valid
    barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    barycenters.into_iter().map(|(u, _)| lane_node_indices[u]).collect()
}

// ------------------------------------------------------------------
// Routing tuning constants & geometric helpers
// ------------------------------------------------------------------

const SELF_LOOP_DX: f64 = 28.0;
const SELF_LOOP_DROP: f64 = 12.0;
const PARALLEL_FAN: f64 = 16.0;
const SIDE_CHANNEL_INSET: f64 = 12.0;
const MIN_CHANNEL_GAP: f64 = 8.0;

fn node_rect(n: &AdvanceSceneNode) -> (f64, f64, f64, f64) {
    (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0)
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

fn route_self_loop(a: &AdvanceSceneNode, fan: f64, dir: AdvanceDirection) -> Vec<(f64, f64)> {
    match dir {
        AdvanceDirection::Vertical => {
            let p0 = (a.x + a.w / 2.0, a.y);
            let loop_x = a.x + a.w / 2.0 + SELF_LOOP_DX + fan;
            let loop_y = a.y + a.h / 2.0 + SELF_LOOP_DROP + fan.abs();
            let p3 = (a.x, a.y + a.h / 2.0);
            vec![p0, (loop_x, p0.1), (loop_x, loop_y), (p3.0, loop_y), p3]
        }
        AdvanceDirection::Horizontal => {
            let p0 = (a.x, a.y + a.h / 2.0);
            let loop_y = a.y + a.h / 2.0 + SELF_LOOP_DX + fan;
            let loop_x = a.x + a.w / 2.0 + SELF_LOOP_DROP + fan.abs();
            let p3 = (a.x + a.w / 2.0, a.y);
            vec![p0, (p0.0, loop_y), (loop_x, loop_y), (loop_x, p3.1), p3]
        }
    }
}

fn same_lane_blocked(
    a: &AdvanceSceneNode,
    b: &AdvanceSceneNode,
    nodes: &[AdvanceSceneNode],
    dir: AdvanceDirection,
) -> bool {
    match dir {
        AdvanceDirection::Vertical => {
            let (lo_y, hi_y) = if a.y < b.y {
                (a.y + a.h / 2.0, b.y - b.h / 2.0)
            } else {
                (b.y + b.h / 2.0, a.y - a.h / 2.0)
            };
            if hi_y <= lo_y {
                return false;
            }
            let x = a.x;
            nodes.iter().any(|n| {
                n.id != a.id
                    && n.id != b.id
                    && n.lane == a.lane
                    && seg_crosses_rect((x, lo_y), (x, hi_y), node_rect(n))
            })
        }
        AdvanceDirection::Horizontal => {
            let (lo_x, hi_x) = if a.x < b.x {
                (a.x + a.w / 2.0, b.x - b.w / 2.0)
            } else {
                (b.x + b.w / 2.0, a.x - a.w / 2.0)
            };
            if hi_x <= lo_x {
                return false;
            }
            let y = a.y;
            nodes.iter().any(|n| {
                n.id != a.id
                    && n.id != b.id
                    && n.lane == a.lane
                    && seg_crosses_rect((lo_x, y), (hi_x, y), node_rect(n))
            })
        }
    }
}

fn route_same_lane(
    a: &AdvanceSceneNode,
    b: &AdvanceSceneNode,
    nodes: &[AdvanceSceneNode],
    fan: f64,
    dir: AdvanceDirection,
) -> Vec<(f64, f64)> {
    match dir {
        AdvanceDirection::Vertical => {
            if same_lane_blocked(a, b, nodes, dir) {
                let max_right = nodes
                    .iter()
                    .filter(|n| n.lane == a.lane)
                    .map(|n| n.x + n.w / 2.0)
                    .fold(f64::NEG_INFINITY, f64::max);
                let detour_x = max_right + SIDE_CHANNEL_INSET + fan;
                let p0 = (a.x + a.w / 2.0, a.y);
                let p3 = (b.x + b.w / 2.0, b.y);
                return vec![p0, (detour_x, p0.1), (detour_x, p3.1), p3];
            }
            let (p0, p3) = if a.y < b.y {
                ((a.x, a.y + a.h / 2.0), (b.x, b.y - b.h / 2.0))
            } else {
                ((a.x, a.y - a.h / 2.0), (b.x, b.y + b.h / 2.0))
            };
            if (a.x - b.x).abs() < f64::EPSILON {
                if fan.abs() < f64::EPSILON {
                    vec![p0, p3]
                } else {
                    let spread_x = fan * 0.5;
                    let p0s = (p0.0 + spread_x, p0.1);
                    let p3s = (p3.0 + spread_x, p3.1);
                    let mid_y = (p0.1 + p3.1) / 2.0;
                    vec![p0s, (p0s.0, mid_y), (p3s.0, mid_y), p3s]
                }
            } else {
                let mid_y = (p0.1 + p3.1) / 2.0 + fan;
                vec![p0, (a.x, mid_y), (b.x, mid_y), p3]
            }
        }
        AdvanceDirection::Horizontal => {
            if same_lane_blocked(a, b, nodes, dir) {
                let max_bottom = nodes
                    .iter()
                    .filter(|n| n.lane == a.lane)
                    .map(|n| n.y + n.h / 2.0)
                    .fold(f64::NEG_INFINITY, f64::max);
                let detour_y = max_bottom + SIDE_CHANNEL_INSET + fan;
                let p0 = (a.x, a.y + a.h / 2.0);
                let p3 = (b.x, b.y + b.h / 2.0);
                return vec![p0, (p0.0, detour_y), (p3.0, detour_y), p3];
            }
            let (p0, p3) = if a.x < b.x {
                ((a.x + a.w / 2.0, a.y), (b.x - b.w / 2.0, b.y))
            } else {
                ((a.x - a.w / 2.0, a.y), (b.x + b.w / 2.0, b.y))
            };
            if (a.y - b.y).abs() < f64::EPSILON {
                if fan.abs() < f64::EPSILON {
                    vec![p0, p3]
                } else {
                    let spread_y = fan * 0.5;
                    let p0s = (p0.0, p0.1 + spread_y);
                    let p3s = (p3.0, p3.1 + spread_y);
                    let mid_x = (p0.0 + p3.0) / 2.0;
                    vec![p0s, (mid_x, p0s.1), (mid_x, p3s.1), p3s]
                }
            } else {
                let mid_x = (p0.0 + p3.0) / 2.0 + fan;
                vec![p0, (mid_x, a.y), (mid_x, b.y), p3]
            }
        }
    }
}

fn crossing_x_interval(
    n: &AdvanceSceneNode,
    a_id: &str,
    b_id: &str,
    lo_y: f64,
    hi_y: f64,
) -> Option<(f64, f64)> {
    if n.id == a_id || n.id == b_id {
        return None;
    }
    let (l, t, r, b) = node_rect(n);
    if t < hi_y && b > lo_y {
        Some((l, r))
    } else {
        None
    }
}

fn nudge_mid_x(
    mid_x: f64,
    p0: (f64, f64),
    p3: (f64, f64),
    a: &AdvanceSceneNode,
    b: &AdvanceSceneNode,
    nodes: &[AdvanceSceneNode],
) -> f64 {
    let (lo_y, hi_y) = if p0.1 < p3.1 { (p0.1, p3.1) } else { (p3.1, p0.1) };
    if (hi_y - lo_y).abs() < f64::EPSILON {
        return mid_x;
    }
    let blocked = nodes.iter().any(|n| {
        n.id != a.id
            && n.id != b.id
            && seg_crosses_rect((mid_x, lo_y), (mid_x, hi_y), node_rect(n))
    });
    if !blocked {
        return mid_x;
    }

    let (lo_x, hi_x) = if p0.0 < p3.0 { (p0.0, p3.0) } else { (p3.0, p0.0) };
    let mut covered: Vec<(f64, f64)> = nodes
        .iter()
        .filter_map(|n| crossing_x_interval(n, &a.id, &b.id, lo_y, hi_y))
        .map(|(l, r)| (l.max(lo_x), r.min(hi_x)))
        .filter(|(l, r)| l < r)
        .collect();
    covered.sort_unstable_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut best: Option<(f64, f64)> = None;
    let mut cursor = lo_x;
    let mut consider = |from: f64, to: f64| {
        let w = to - from;
        if w >= MIN_CHANNEL_GAP && best.map(|(_, bw)| w > bw).unwrap_or(true) {
            best = Some(((from + to) / 2.0, w));
        }
    };
    for (l, r) in covered {
        if l > cursor {
            consider(cursor, l);
        }
        cursor = cursor.max(r);
    }
    consider(cursor, hi_x);

    best.map(|(c, _)| c).unwrap_or(mid_x)
}

fn route_cross_lane(
    a: &AdvanceSceneNode,
    b: &AdvanceSceneNode,
    nodes: &[AdvanceSceneNode],
    fan: f64,
    dir: AdvanceDirection,
) -> Vec<(f64, f64)> {
    match dir {
        AdvanceDirection::Vertical => {
            let (p0, p3) = if b.x >= a.x {
                ((a.x + a.w / 2.0, a.y), (b.x - b.w / 2.0, b.y))
            } else {
                ((a.x - a.w / 2.0, a.y), (b.x + b.w / 2.0, b.y))
            };
            let mid_x = nudge_mid_x((p0.0 + p3.0) / 2.0 + fan, p0, p3, a, b, nodes);
            vec![p0, (mid_x, p0.1), (mid_x, p3.1), p3]
        }
        AdvanceDirection::Horizontal => {
            let (p0, p3) = if b.y >= a.y {
                ((a.x, a.y + a.h / 2.0), (b.x, b.y - b.h / 2.0))
            } else {
                ((a.x, a.y - a.h / 2.0), (b.x, b.y + b.h / 2.0))
            };
            let mid_y = (p0.1 + p3.1) / 2.0 + fan;
            vec![p0, (p0.0, mid_y), (p3.0, mid_y), p3]
        }
    }
}

fn route_edges(
    d: &AdvanceDiagram,
    nodes: &[AdvanceSceneNode],
    dir: AdvanceDirection,
) -> Vec<AdvanceSceneEdge> {
    let node_idx = node_index_map(d);
    let mut edge_scenes = Vec::with_capacity(d.edges.len());

    let mut pair_totals: std::collections::HashMap<(String, String), usize> =
        std::collections::HashMap::new();
    for e in &d.edges {
        *pair_totals
            .entry((e.from.clone(), e.to.clone()))
            .or_insert(0) += 1;
    }
    let mut pair_seen: std::collections::HashMap<(String, String), usize> =
        std::collections::HashMap::new();

    for e in &d.edges {
        let from_i = node_idx[&e.from];
        let to_i = node_idx[&e.to];
        let a = &nodes[from_i];
        let b = &nodes[to_i];

        let key = (e.from.clone(), e.to.clone());
        let dup_i = {
            let v = pair_seen.entry(key.clone()).or_insert(0);
            let i = *v;
            *v += 1;
            i
        };
        let dup_n = pair_totals[&key];
        let fan = (dup_i as f64 - (dup_n as f64 - 1.0) / 2.0) * PARALLEL_FAN;

        let points = if from_i == to_i {
            route_self_loop(a, fan, dir)
        } else if a.lane == b.lane {
            route_same_lane(a, b, nodes, fan, dir)
        } else {
            route_cross_lane(a, b, nodes, fan, dir)
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

// ------------------------------------------------------------------
// Recursive Lane Layout Calculations
// ------------------------------------------------------------------

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
) -> LaneDim {
    let li = lane_idx[&lane.id];
    let local_nodes = &lane_node_lists[li];

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
        let h = (cfg.lane_title_h + cfg.lane_pad_y + direct_node_h + cfg.lane_pad_y).max(120.0);
        LaneDim { w, h, children: Vec::new() }
    } else {
        let child_dims: Vec<LaneDim> = lane
            .children
            .iter()
            .map(|c| compute_lane_dim_rec(c, lane_idx, lane_node_lists, sizes, cfg))
            .collect();
        let sum_children_w: f64 = child_dims.iter().map(|c| c.w).sum::<f64>()
            + (child_dims.len().saturating_sub(1) as f64 * cfg.lane_gap);
        let max_children_h = child_dims.iter().map(|c| c.h).fold(0.0_f64, f64::max);

        let w = (sum_children_w + 2.0 * cfg.lane_pad_x).max(direct_node_w + 2.0 * cfg.lane_pad_x).max(120.0);
        let h = (cfg.lane_title_h + cfg.lane_pad_y + max_children_h + cfg.lane_pad_y + direct_node_h).max(120.0);
        LaneDim { w, h, children: child_dims }
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

// ------------------------------------------------------------------
// Main Layout
// ------------------------------------------------------------------

/// Compute the positioned geometry for an advance diagram.
pub fn layout(d: &AdvanceDiagram) -> AdvanceScene {
    let cfg = &d.config;
    let lane_idx = lane_index_map(d);
    let total_lanes_count = lane_idx.len();
    let sizes: Vec<(f64, f64)> = d.nodes.iter().map(node_size).collect();

    // Check if explicit coordinates are supplied on all nodes
    let has_explicit_coords = !d.nodes.is_empty() && d.nodes.iter().all(|n| n.x.is_some() && n.y.is_some());

    if has_explicit_coords {
        let node_scenes: Vec<AdvanceSceneNode> = d.nodes.iter().enumerate().map(|(i, n)| {
            AdvanceSceneNode {
                id: n.id.clone(),
                label: n.label.clone(),
                lane: n.lane.clone(),
                x: n.x.unwrap(),
                y: n.y.unwrap(),
                w: sizes[i].0,
                h: sizes[i].1,
                shape: n.shape,
            }
        }).collect();
        let (lane_scenes, width, height) = build_lanes_around_nodes(d, &node_scenes);
        let edge_scenes = route_edges(d, &node_scenes, d.direction);
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
                edges: edge_scenes,
            },
            cfg.margin,
        );
    }

    if d.direction == AdvanceDirection::Horizontal {
        // Horizontal Layout: Rows stacked top-to-bottom
        let mut row_node_lists: Vec<Vec<usize>> = vec![Vec::new(); total_lanes_count];
        for (i, n) in d.nodes.iter().enumerate() {
            let li = lane_idx[&n.lane];
            row_node_lists[li].push(i);
        }
        let ordered_row_nodes: Vec<Vec<usize>> = row_node_lists
            .iter()
            .map(|list| order_lane_nodes(list, d, cfg))
            .collect();

        let mut row_heights: Vec<f64> = vec![0.0; d.lanes.len()];
        let mut row_widths: Vec<f64> = vec![0.0; d.lanes.len()];

        for (i, list) in ordered_row_nodes.iter().enumerate() {
            let mut max_nh: f64 = 0.0;
            let mut sum_nw = 0.0;
            for &ni in list {
                max_nh = max_nh.max(sizes[ni].1);
                sum_nw += sizes[ni].0 + cfg.lane_gap;
            }
            if !list.is_empty() {
                sum_nw -= cfg.lane_gap;
            }
            row_heights[i] = (max_nh + 2.0 * cfg.lane_pad_y).max(cfg.lane_title_h + 2.0 * cfg.lane_pad_y).max(80.0);
            row_widths[i] = (cfg.lane_title_h + cfg.lane_pad_x + sum_nw + cfg.lane_pad_x).max(160.0);
        }

        let uniform_h = row_heights.iter().fold(0.0_f64, |m, h| m.max(*h));
        let uniform_w = row_widths.iter().fold(0.0_f64, |m, w| m.max(*w));

        let mut lane_scenes = Vec::with_capacity(d.lanes.len());
        let mut cur_y = cfg.margin;
        for lane in &d.lanes {
            lane_scenes.push(AdvanceSceneLane {
                id: lane.id.clone(),
                title: lane.title.clone(),
                x: cfg.margin,
                y: cur_y,
                w: uniform_w,
                h: uniform_h,
            });
            cur_y += uniform_h + cfg.lane_gap;
        }
        let total_w = cfg.margin + uniform_w + cfg.margin;
        let total_h = if d.lanes.is_empty() { cfg.margin * 2.0 } else { cur_y - cfg.lane_gap + cfg.margin };

        let mut node_scenes = Vec::with_capacity(d.nodes.len());
        for (li, list) in ordered_row_nodes.iter().enumerate() {
            let lane = &lane_scenes[li];
            let cy = lane.y + lane.h / 2.0;
            let mut cursor_x = lane.x + cfg.lane_title_h + cfg.lane_pad_x;
            for &ni in list {
                let n = &d.nodes[ni];
                let (nw, nh) = sizes[ni];
                let cx = cursor_x + nw / 2.0;
                cursor_x += nw + cfg.node_gap_y;
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

        let edge_scenes = route_edges(d, &node_scenes, d.direction);
        return AdvanceScene {
            width: total_w,
            height: total_h,
            title: d.title.clone(),
            description: d.description.clone(),
            direction: d.direction,
            style: d.style.clone(),
            lanes: lane_scenes,
            nodes: node_scenes,
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
        .map(|l| compute_lane_dim_rec(l, &lane_idx, &lane_node_lists, &sizes, cfg))
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

    let edge_scenes = route_edges(d, &node_scenes, d.direction);

    AdvanceScene {
        width: total_width,
        height: total_height,
        title: d.title.clone(),
        description: d.description.clone(),
        direction: d.direction,
        style: d.style.clone(),
        lanes: lane_scenes,
        nodes: node_scenes,
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

    // Label
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
            text_color,
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
    let svg_title = sc.title.as_deref().unwrap_or("Advance diagram");
    svg_open(&mut s, sc.width, sc.height, FONT_SIZE, svg_title, opts);

    if let Some(desc) = &sc.description {
        s.push_str(&format!("<desc>{}</desc>\n", escape(desc)));
    }

    let marker_id = format!("advance-arrow-{}", MARKER_COUNTER.fetch_add(1, Ordering::Relaxed));
    s.push_str(&format!(
        "<defs><marker id=\"{}\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" \
         markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
         <path d=\"M 0 1 L 9 5 L 0 9 z\" fill=\"{}\"/></marker></defs>\n",
        marker_id, sc.style.edge_color
    ));

    // Lane backgrounds
    for lane in &sc.lanes {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"2\" rx=\"8\"/>\n",
            lane.x, lane.y, lane.w, lane.h, sc.style.lane_fill, sc.style.lane_stroke
        ));
        let (tx, ty) = if sc.direction == AdvanceDirection::Horizontal {
            (lane.x + 12.0, lane.y + lane.h / 2.0)
        } else {
            (lane.x + DEFAULT_LANE_PAD_X, lane.y + DEFAULT_LANE_TITLE_H - 6.0)
        };
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{}\" font-weight=\"bold\" \
             fill=\"{}\">{}</text>\n",
            tx,
            ty,
            FONT_SIZE,
            sc.style.text_color,
            escape(&lane.title)
        ));
    }

    // Edges
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
            format!(" marker-end=\"url(#{})\"", marker_id)
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
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>\n",
            d, sc.style.edge_color, sw, dash, marker
        ));
        if let Some(label) = &e.label {
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
                 fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" rx=\"3\"/>\n",
                lx - lw / 2.0,
                ly - 9.0,
                lw,
                sc.style.label_fill,
                sc.style.lane_stroke
            ));
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                 font-size=\"{}\" fill=\"{}\">{}</text>\n",
                lx,
                ly,
                FONT_SIZE,
                sc.style.text_color,
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
        let (l, t, r, b) = bounds[i]
            .unwrap_or((d.config.margin, d.config.margin, d.config.margin + 120.0, d.config.margin + 120.0));
        let x = l - d.config.lane_pad_x;
        let y = (t - d.config.lane_title_h - d.config.lane_pad_y).min(d.config.margin);
        let w = (r - x + d.config.lane_pad_x).max(120.0);
        let h = (b - y + d.config.lane_pad_y).max(d.config.lane_title_h + 2.0 * d.config.lane_pad_y);

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

    (lanes, max_right + d.config.margin, max_bottom + d.config.margin)
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

fn place_nodes_at_positions(
    d: &AdvanceDiagram,
    positions: &[f64],
) -> Vec<AdvanceSceneNode> {
    let sizes: Vec<(f64, f64)> = d.nodes.iter().map(node_size).collect();
    d.nodes
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
        }
        for e in &mut sc.edges {
            for p in &mut e.points {
                p.0 += dx;
                p.1 += dy;
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
    let edges = route_edges(&d, &nodes, d.direction);
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
    let lane_idx = lane_index_map(d);
    let mut lane_scenes = Vec::with_capacity(d.lanes.len());
    let mut x = margin;

    let mut lane_heights: Vec<f64> = Vec::with_capacity(d.lanes.len());
    for i in 0..d.lanes.len() {
        let max_bottom = nodes
            .iter()
            .filter(|n| lane_idx[&n.lane] == i)
            .map(|n| n.y + n.h / 2.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let h = if max_bottom.is_finite() {
            (max_bottom - margin + d.config.lane_pad_y).max(d.config.lane_title_h + 2.0 * d.config.lane_pad_y)
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
        return Err(adv_err("render_advance_routed_with_lanes is not supported in horizontal direction"));
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
    let (lanes, width, height) =
        build_lanes_with_widths(&d, &nodes, lane_widths, margin, gap);
    let edges = route_edges(&d, &nodes, d.direction);
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
            edges,
        },
        margin,
    );
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
    fn marker_id_namespacing() {
        let svg1 = render_advance_svg(sample_json()).unwrap();
        let svg2 = render_advance_svg(sample_json()).unwrap();
        let m1 = svg1.find("id=\"advance-arrow-").unwrap();
        let m2 = svg2.find("id=\"advance-arrow-").unwrap();
        let id1 = &svg1[m1..m1 + 25];
        let id2 = &svg2[m2..m2 + 25];
        assert_ne!(id1, id2);
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
    fn horizontal_and_nested_errors() {
        let src = r#"{
            "direction":"horizontal",
            "lanes":[{"id":"p","children":[{"id":"c"}]}],
            "nodes":[]
        }"#;
        assert!(AdvanceDiagram::parse(src).is_err());
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
        let edges = route_edges(&d, &nodes, d.direction);
        let sc = fit_canvas(AdvanceScene { width, height, title: None, description: None, direction: d.direction, style: d.style.clone(), lanes, nodes, edges }, d.config.margin);
        assert!(sc.nodes.iter().all(|n| n.y - n.h / 2.0 >= d.config.margin - 1e-9));
        assert!(sc.nodes.iter().all(|n| n.x - n.w / 2.0 >= d.config.margin - 1e-9));
        assert!(sc.lanes.iter().all(|l| l.y >= d.config.margin - 1e-9 && l.x >= d.config.margin - 1e-9));
        assert!(sc.edges.iter().all(|e| e.points.iter().all(|p| p.1 >= d.config.margin - 1e-9)));
        let max_bottom = sc.nodes.iter().map(|n| n.y + n.h / 2.0).fold(0.0_f64, f64::max);
        assert!(sc.height >= max_bottom + d.config.margin - 1e-9);
    }
}
