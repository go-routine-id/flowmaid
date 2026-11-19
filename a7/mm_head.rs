//! Mindmap rendering, in the Mermaid style: the root sits at the centre
//! and branches radiate outward, clockwise from 12 o'clock in source
//! order. Every child hangs off its PARENT on a short spoke of one
//! length ([`TRUNK`] from the root, [`SPOKE`] below that), and siblings
//! fan out around their parent's outward direction, each subtree packed
//! into its own angular cone as seen from the parent — so leaves cluster
//! round the node they belong to, boxes never overlap and connectors
//! never cross. Every top-level branch gets a stable accent that its
//! whole subtree fills, and the seven node shapes (default / rounded /
//! square / circle / hexagon / bang / cloud) are drawn for real.
//!
//! Like `pie`/`seq` there is nothing auto-routed: [`scene`] computes
//! every coordinate a painter needs, [`route`] rebuilds the scene from
//! dragged node centres, and [`to_svg`] serialises it. [`perimeter`],
//! [`underline`], [`icon_strokes`] and [`label_centre`] expose the rest
//! of the drawn geometry so a GUI painter can match the SVG exactly.

use crate::layout::{line_count, text_width, TEXT_CALIBRATION};
use crate::model::{MindShape, Mindmap, NodeStyle};
use crate::scene::{escape, svg_open, SvgOptions};
use crate::style::text_on;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// Horizontal padding inside a node box (mermaid uses 20 on 16px text).
pub const PAD_X: f64 = 16.0;
/// Vertical padding inside a node box.
pub const PAD_Y: f64 = 7.0;
/// Height of one label line at [`FONT`]; other sizes scale it.
pub const LINE_H: f64 = 18.0;
/// Ring spacing of the fallback layout (see [`scene`]): depth `k` on a
/// ring of radius `k · RING` around the root.
pub const RING: f64 = 172.0;
/// Angular separation (radians) kept between adjacent sibling wedges of
/// the fallback ring layout.
const GAP: f64 = 0.06;
/// Canvas margin around the whole tree.
pub const MARGIN: f64 = 28.0;
/// Base font size (px): every node below the first level.
pub const FONT: u32 = 14;
/// Font size of the root's label (bold).
pub const ROOT_FONT: f64 = 18.0;
/// Font size of the first-level branch labels (semibold).
pub const BRANCH_FONT: f64 = 15.0;
/// Visible connector length (border to border) from the root to a
/// first-level branch.
pub const TRUNK: f64 = 58.0;
/// Visible connector length from any other node to its children.
pub const SPOKE: f64 = 42.0;
/// Clear space kept between any two node boxes.
const SEP: f64 = 10.0;
/// Angular gap (radians) between neighbouring sibling cones.
const FAN_GAP: f64 = 0.06;
/// Widest fan (radians) a non-root node spreads its children over, so
/// they stay in front of it, away from its own incoming connector.
const MAX_FAN: f64 = 160.0 * PI / 180.0;
/// Angle a fan aims to give each child when there is room: children of
/// a node spread round it instead of stacking into a narrow column.
const FAN_PER_CHILD: f64 = 36.0 * PI / 180.0;
/// Packing rounds per fan (measure, pack, re-measure, ...).
const FAN_ROUNDS: usize = 4;
/// Spoke growths per fan before it gives up (each ×1.25).
const FAN_GROWTHS: usize = 10;
/// Subtree layouts the fan layout may run before it falls back to the
/// ring layout — a guard for pathologically deep and bushy trees.
const FAN_WORK: usize = 60_000;
/// Side of an `::icon(…)` glyph box, drawn left of the label.
pub const ICON: f64 = 14.0;
/// Gap between an icon glyph and the label text.
const ICON_GAP: f64 = 5.0;
/// Inset of the label from a circle's edge.
const CIRCLE_PAD: f64 = 9.0;
/// Depth of a bang's spikes (px, along the outline's normal).
pub const BANG_SPIKE: f64 = 9.0;
/// Target spacing of a bang's spike tips along its outline.
const BANG_STEP: f64 = 26.0;
/// Radius of a cloud's lobes.
pub const CLOUD_LOBE: f64 = 18.0;
/// Target spacing of a cloud's lobe centres along its outline: less
/// than twice [`CLOUD_LOBE`], so neighbouring lobes overlap into a
/// puffy outline with no gaps.
const CLOUD_STEP: f64 = 32.0;
/// Height of the [`MindShape::Default`] underline.
pub const UNDERLINE_H: f64 = 3.0;
/// Opacity of the black drawn over a default node's fill for its
/// underline — a darker shade of whatever the fill is, custom or not.
pub const UNDERLINE_SHADE: f64 = 0.24;
/// Corner radius of a [`MindShape::Default`] node's top corners.
const DEFAULT_RX: f64 = 6.0;

/// Root fill: a deep indigo ink, the strongest colour on the canvas, so
/// the centre reads first (the old pale `#c9cfe0` receded behind its
/// own branches). Its label is white (12:1).
pub const ROOT_FILL: &str = "#2f3768";

/// One laid-out mindmap node. `(x, y)` is the top-left; the label is
/// drawn centred (see [`label_centre`]). Colors are `#rrggbb` so
/// painters match the SVG.
///
/// `#[non_exhaustive]`: [`scene`] / [`route`] build these; read their
/// fields.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MindNodeBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub text: String,
    pub shape: MindShape,
    pub depth: usize,
    /// Palette fill (root ink or branch accent). A `classDef` fill in
    /// [`MindNodeBox::style`] overrides it; see [`MindNodeBox::paint`].
    pub fill: &'static str,
    /// Label colour reaching WCAG AA on the fill actually painted.
    pub text_color: &'static str,
    /// Label font size (px): [`ROOT_FONT`], [`BRANCH_FONT`] or [`FONT`].
    pub font_size: f64,
    /// Label font weight: 700 root, 600 first level, 400 below.
    pub font_weight: u16,
    /// The node's `::icon(…)`, verbatim; drawn as a small built-in glyph
    /// (see [`icon_strokes`]).
    pub icon: Option<String>,
    /// The node's `:::` classes, emitted as the SVG `class` attribute.
    pub classes: Vec<String>,
    /// `classDef` colours for this node (`fill`, `stroke`,
    /// `stroke-width`, `color`); unset fields keep the palette.
    pub style: NodeStyle,
}

impl MindNodeBox {
    pub fn cx(&self) -> f64 {
        self.x + self.w / 2.0
    }
    pub fn cy(&self) -> f64 {
        self.y + self.h / 2.0
    }
    /// The fill to paint: the `classDef` fill if any, else [`Self::fill`].
    pub fn paint(&self) -> &str {
        self.style.fill.as_deref().unwrap_or(self.fill)
    }
    /// The label colour to paint: the `classDef` `color` if any, else
    /// [`Self::text_color`].
    pub fn ink(&self) -> &str {
        self.style.color.as_deref().unwrap_or(self.text_color)
    }
}

/// A parent→child connector as a cubic Bézier (start, c1, c2, end),
/// centre-to-centre; both ends are hidden behind the node boxes.
#[derive(Debug, Clone)]
pub struct MindLink {
    pub bezier: [(f64, f64); 4],
    pub color: &'static str,
    pub width: f64,
}

/// Everything needed to draw a mindmap: canvas size, node boxes (parse
/// order, root first), and the connectors.
#[derive(Debug, Clone)]
pub struct MindScene {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<MindNodeBox>,
    pub edges: Vec<MindLink>,
}

/// Label font size and weight by depth: a bold root, semibold first
/// level, regular leaves — the hierarchy reads from the type as well as
/// from the connectors.
fn font_of(depth: usize) -> (f64, u16) {
    match depth {
        0 => (ROOT_FONT, 700),
        1 => (BRANCH_FONT, 600),
        _ => (FONT as f64, 400),
    }
}

/// Measured label block `(text width, text height)` at the node's font,
/// without the icon.
fn text_block(text: &str, depth: usize) -> (f64, f64) {
    let (size, weight) = font_of(depth);
    let scale = size / TEXT_CALIBRATION;
    // Bold runs measure ~6% wider (see `layout::text_width`).
    let bold = if weight >= 600 { 1.06 } else { 1.0 };
    let lines = line_count(text) as f64;
    (
        text_width(text) * scale * bold,
        lines * LINE_H * size / FONT as f64,
    )
}

/// Width the icon adds to a label block.
fn icon_room(icon: bool) -> f64 {
    if icon {
        ICON + ICON_GAP
    } else {
        0.0
    }
}

/// Half-axes of the ellipse that holds a `(lw, lh)` label block in a
/// bang or cloud: twice the block's half-height tall, and just wide
/// enough that the block's corners sit inside it — `(a/rx)² + (b/ry)²
/// = 1` with `ry = 2b` gives `rx = a·2/√3`.
fn label_ellipse(lw: f64, lh: f64) -> (f64, f64) {
    let a = lw / 2.0 + 4.0;
    let b = lh / 2.0 + 2.0;
    (a * 2.0 / 3f64.sqrt(), 2.0 * b)
}

/// Intrinsic size of a node from its label, shape, depth and icon.
fn node_size(text: &str, shape: MindShape, depth: usize, icon: bool) -> (f64, f64) {
    let (tw, th) = text_block(text, depth);
    let lw = tw + icon_room(icon);
    let base_w = lw + 2.0 * PAD_X;
    let base_h = th + 2.0 * PAD_Y;
    match shape {
        // A true circle round the label block.
        MindShape::Circle => {
            let d = lw.hypot(th) + 2.0 * CIRCLE_PAD;
            (d, d)
        }
        MindShape::Cloud => {
            let (rx, ry) = label_ellipse(lw, th);
            (2.0 * (rx + CLOUD_LOBE), 2.0 * (ry + CLOUD_LOBE))
        }
        MindShape::Bang => {
            let (rx, ry) = label_ellipse(lw, th);
            (2.0 * (rx + BANG_SPIKE), 2.0 * (ry + BANG_SPIKE))
        }
        MindShape::Hexagon => (base_w + base_h * 0.7, base_h),
        _ => (base_w, base_h),
    }
}

fn mind_node_size(n: &crate::model::MindNode) -> (f64, f64) {
    node_size(&n.text, n.shape, n.depth, n.icon.is_some())
}
