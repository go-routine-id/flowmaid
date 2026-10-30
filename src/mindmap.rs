//! Mindmap rendering, in the Mermaid style: the root sits at the centre
//! and branches radiate outward, clockwise from 12 o'clock in source
//! order, sharing the whole circle. Every child hangs off its PARENT on
//! a spoke, and siblings are swept round their parent's outward
//! direction, each taking the first direction where its whole subtree
//! clears the siblings already placed — so leaves cluster round the node
//! they belong to, boxes never overlap and connectors never cross.
//! Spokes start at [`TRUNK`] from the root and [`SPOKE`] below it; a
//! crowded fan lengthens them only as far as it needs, and each child's
//! own spoke is then shortened as far as it stays clear, so a small
//! branch sits close in beside a bushy one. Every top-level branch gets
//! a stable accent that its whole subtree fills, and the seven node
//! shapes (default / rounded / square / circle / hexagon / bang / cloud)
//! are drawn for real.
//!
//! Like `pie`/`seq` there is nothing auto-routed: [`scene`] computes
//! every coordinate a painter needs, [`route`] rebuilds the scene from
//! dragged node centres, and [`to_svg`] serialises it. [`perimeter`],
//! [`underline`], [`icon_strokes`] and [`label_centre`] expose the rest
//! of the drawn geometry so a GUI painter can match the SVG exactly.

use crate::layout::{line_count, text_width, TEXT_CALIBRATION};
use crate::model::{MindShape, Mindmap, NodeStyle};
use crate::scene::{escape, svg_open, SvgOptions};
use crate::style::{css_hex, text_on};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// Horizontal padding inside a node box (mermaid uses 20 on 16px text).
pub const PAD_X: f64 = 16.0;
/// Vertical padding inside a node box.
pub const PAD_Y: f64 = 8.0;
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
/// Shortest visible connector (border to border) from the root to a
/// first-level branch: the length every trunk has while the branches
/// fit round the root at it.
pub const TRUNK: f64 = 46.0;
/// Shortest visible connector from any other node to its children (as
/// [`TRUNK`], the length while the children fit at it).
pub const SPOKE: f64 = 34.0;
/// Clear space kept between any two node boxes.
const SEP: f64 = 10.0;
/// Widest fan (radians) of a first-level node when a crowded root lays
/// its branches out narrow (see [`fan_centres`]).
const NARROW_FAN: f64 = 100.0 * PI / 180.0;
/// Fewest root branches for the narrow layout to be tried.
const NARROW_FROM: usize = 8;
/// Longest connector (px) the normal layout of a crowded root may have
/// before the narrow layout is tried.
const NARROW_TRUNK: f64 = 4.0 * TRUNK;
/// Fan (radians) a non-root node first spreads its children over, so
/// they stay in front of it.
const MAX_FAN: f64 = 160.0 * PI / 180.0;
/// Widest fan a crowded node may open to before its spokes lengthen
/// instead: children wrap round its sides (as mermaid's do) but keep
/// clear of the band behind it, where its own connector arrives.
const MAX_FAN_WIDE: f64 = 280.0 * PI / 180.0;
/// Angle a fan aims to give each child when there is room: children of
/// a node spread round it instead of stacking into a narrow column.
const FAN_PER_CHILD: f64 = 30.0 * PI / 180.0;
/// Attempts per fan before it gives up, each with its spokes ×1.25.
const FAN_GROWTHS: usize = 24;
/// Spoke searches stop once the gap that fits is within this factor of
/// one that does not (a few px on a long trunk).
const GAP_TOLERANCE: f64 = 1.06;
/// Most bisection steps a spoke search takes.
const GAP_BISECTIONS: usize = 7;
/// Factor by which children slide in per round once their fan fits
/// (see [`Fan::shorten`]).
const SHORTEN_STEP: f64 = 0.85;
/// Box-pair tests the fan layout may run before it falls back to the
/// ring layout — a time guard for very large or pathological maps (a
/// realistic map of a hundred nodes uses about a quarter of it).
const FAN_WORK: usize = 6_000_000;
/// Side of an `::icon(…)` glyph box, drawn left of the label.
pub const ICON: f64 = 14.0;
/// Gap between an icon glyph and the label text.
const ICON_GAP: f64 = 5.0;
/// Inset of the label from a circle's edge.
const CIRCLE_PAD: f64 = 9.0;
/// Depth of a bang's spikes (px, along the outline's normal).
pub const BANG_SPIKE: f64 = 10.0;
/// Target spacing of a bang's spike tips along its outline.
const BANG_STEP: f64 = 20.0;
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

/// Core of a bang: a stadium (half-length `l` of its straight sides,
/// cap radius `r`) round a `(lw, lh)` label block — caps twice the
/// block's half-height, sides just long enough that the block's
/// corners sit inside the caps. A stadium keeps a wide label's ends
/// round, so its spikes stay evenly spread there (an ellipse that flat
/// bunched them into a blunt, chopped-off end).
fn bang_core(lw: f64, lh: f64) -> (f64, f64) {
    let a = lw / 2.0 + 4.0;
    let b = lh / 2.0 + 2.0;
    let r = 2.0 * b;
    let l = a - (r * r - b * b).sqrt();
    if l > 0.0 {
        (l, r)
    } else {
        (0.0, r.max(a.hypot(b)))
    }
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
            let (l, r) = bang_core(lw, th);
            (2.0 * (l + r + BANG_SPIKE), 2.0 * (r + BANG_SPIKE))
        }
        MindShape::Hexagon => (base_w + base_h * 0.7, base_h),
        _ => (base_w, base_h),
    }
}

fn mind_node_size(n: &crate::model::MindNode) -> (f64, f64) {
    node_size(&n.text, n.shape, n.depth, n.icon.is_some())
}

// ------------------------------------------------------------------
// Layout
// ------------------------------------------------------------------

/// Distance from the centre of a `w × h` box to its border along the
/// unit vector `u`.
fn ext(w: f64, h: f64, u: (f64, f64)) -> f64 {
    let ex = if u.0.abs() > 1e-9 {
        w / 2.0 / u.0.abs()
    } else {
        f64::INFINITY
    };
    let ey = if u.1.abs() > 1e-9 {
        h / 2.0 / u.1.abs()
    } else {
        f64::INFINITY
    };
    ex.min(ey)
}

/// Centre distance from a parent box to a child box placed along `u`:
/// their borders `gap` apart on the line between their centres, and
/// never closer than [`SEP`] anywhere (two boxes stopping short along
/// a diagonal can still clip corners, so the boxes' Minkowski sum is
/// checked too).
fn spoke_len(p: (f64, f64), c: (f64, f64), u: (f64, f64), gap: f64) -> f64 {
    let along = ext(p.0, p.1, u) + ext(c.0, c.1, u) + gap;
    let apart = ext(p.0 + c.0 + 2.0 * SEP, p.1 + c.1 + 2.0 * SEP, u);
    along.max(apart)
}

/// `a` wrapped into `(-π, π]`.
fn wrap(a: f64) -> f64 {
    let mut a = a % TAU;
    if a <= -PI {
        a += TAU;
    } else if a > PI {
        a -= TAU;
    }
    a
}

/// A laid-out subtree as obstacles, relative to its root's centre: every
/// node box `(x0, y0, x1, y1)` (its own first) and every connector.
struct Shape {
    boxes: Vec<(f64, f64, f64, f64)>,
    segs: Vec<((f64, f64), (f64, f64))>,
    /// Bounding box of `boxes`, for a cheap first test.
    bbox: (f64, f64, f64, f64),
}

/// A laid-out subtree: its [`Shape`], and the centre of every strict
/// descendant (in preorder) relative to the subtree's root.
struct Entry {
    shape: Shape,
    pos: Vec<(f64, f64)>,
}

/// A child placed round its parent (at the origin): which subtree
/// layout ([`Entry`] index) and where its centre sits. Its spoke runs
/// from the origin to `o`.
#[derive(Clone, Copy)]
struct At {
    e: usize,
    o: (f64, f64),
}

type Rect = (f64, f64, f64, f64);
type Seg = ((f64, f64), (f64, f64));

/// Two boxes closer than `m` (on both axes)?
fn boxes_near(a: Rect, b: Rect, m: f64) -> bool {
    a.0 < b.2 + m && b.0 < a.2 + m && a.1 < b.3 + m && b.1 < a.3 + m
}

fn shift(b: Rect, o: (f64, f64)) -> Rect {
    (b.0 + o.0, b.1 + o.1, b.2 + o.0, b.3 + o.1)
}

fn shift_seg(s: Seg, o: (f64, f64)) -> Seg {
    ((s.0 .0 + o.0, s.0 .1 + o.1), (s.1 .0 + o.0, s.1 .1 + o.1))
}

/// Clear space a connector keeps from a box it does not belong to —
/// about half the thickest connector, so even a trunk never touches one.
const SEG_CLEAR: f64 = 5.0;

/// `s` passes through (or within [`SEG_CLEAR`] of) one of `sh`'s boxes
/// moved by `o`?
fn seg_hits_shape(s: Seg, sh: &Shape, o: (f64, f64)) -> bool {
    // Most segments tested pass nowhere near the shape (or a given box):
    // reject those on bounding boxes before clipping.
    let span = (
        s.0 .0.min(s.1 .0) - o.0,
        s.0 .1.min(s.1 .1) - o.1,
        s.0 .0.max(s.1 .0) - o.0,
        s.0 .1.max(s.1 .1) - o.1,
    );
    if !boxes_near(span, sh.bbox, SEG_CLEAR) {
        return false;
    }
    sh.boxes.iter().any(|&b| {
        if !boxes_near(span, b, SEG_CLEAR) {
            return false;
        }
        let b = shift(b, o);
        segment_hits_box(
            s.0,
            s.1,
            (
                b.0 - SEG_CLEAR,
                b.1 - SEG_CLEAR,
                b.2 + SEG_CLEAR,
                b.3 + SEG_CLEAR,
            ),
        )
    })
}

/// `s` passes through (or near) one of `sh`'s boxes, or crosses one of
/// its connectors, with `sh` moved by `o`?
fn spoke_hits(s: Seg, sh: &Shape, o: (f64, f64)) -> bool {
    if seg_hits_shape(s, sh, o) {
        return true;
    }
    let span = (
        s.0 .0.min(s.1 .0) - o.0,
        s.0 .1.min(s.1 .1) - o.1,
        s.0 .0.max(s.1 .0) - o.0,
        s.0 .1.max(s.1 .1) - o.1,
    );
    boxes_near(span, sh.bbox, 0.0)
        && sh.segs.iter().any(|&q| {
            let q = shift_seg(q, o);
            segments_cross(s.0, s.1, q.0, q.1)
        })
}

/// Directions are whole multiples of this many degrees, so a subtree
/// laid out for one direction is memoised and reused.
const DEG_STEP: i32 = 3;

fn rad(deg: i32) -> f64 {
    f64::from(deg) * PI / 180.0
}

/// Stride (degrees) of a sweep through blocked directions; see
/// [`Fan::first_clear`].
const STRIDE: i32 = 3 * DEG_STEP;

/// Depth (px, on both axes) by which a subtree layout turned from
/// another direction must overlap a sibling before a sweep skips that
/// direction untested (see [`Fan::surely_blocked`]).
const TURN_SLACK: f64 = 10.0;

/// Length of the stand-in for a node's own incoming connector: a ray
/// straight back from it, long enough to reach past its parent, so a
/// node's descendants never cross where its connector comes from.
const INBOUND_RAY: f64 = 4000.0;

/// The parent-centred fan layout. [`Fan::layout`] lays out a node's
/// subtree with the node at the origin, for a given incoming direction,
/// memoised per (node, direction) — the parent sweeps candidate
/// directions for each child, and every candidate needs that child's
/// exact subtree, because axis-aligned boxes do not rotate rigidly.
struct Fan<'a> {
    d: &'a Mindmap,
    size: &'a [(f64, f64)],
    /// Preorder of the tree and each node's `[start, end)` span in it,
    /// so a subtree's members are one contiguous slice.
    pre: Vec<usize>,
    span: Vec<(usize, usize)>,
    /// Per node, per whole-step direction: the subtree layout made for
    /// it — 0 not tried yet, 1 cannot be laid out, else `entries` index
    /// + 2 (allocated on a node's first layout).
    memo: Vec<Vec<u32>>,
    entries: Vec<Entry>,
    /// A leaf's one-box entry (its layout does not depend on direction).
    leaf: Vec<Option<usize>>,
    /// Shortest spoke gap a node's fan has fitted with so far, for any
    /// direction: the next direction starts there instead of failing its
    /// way up from the base length again (a crowded fan needs about the
    /// same room whichever way it faces).
    fitted: Vec<Option<f64>>,
    /// Longest spoke gap a node's fan has failed at so far, for any
    /// direction: with [`Fan::fitted`] it brackets the next direction's
    /// search, which then rarely needs to bisect again.
    failed: Vec<f64>,
    /// Box-pair tests run so far, against [`FAN_WORK`].
    work: std::cell::Cell<usize>,
    /// Unit vector of every whole-step direction (sweeps are a hot path).
    units: Vec<(f64, f64)>,
    /// Every first-level fan keeps within [`NARROW_FAN`] instead of
    /// opening up to [`MAX_FAN_WIDE`] (see [`fan_centres`]).
    narrow: bool,
    /// How a node's latest fan sat: its first child's direction behind
    /// the fan's own (`dir` minus it) and the separation it spread its
    /// children by — the first guess for its next direction.
    aimed: Vec<Option<(i32, i32)>>,
    /// The latest subtree layout made for each node: its direction and
    /// [`Entry`] (see [`Fan::surely_blocked`]).
    last: Vec<Option<(i32, usize)>>,
}

impl Fan<'_> {
    fn new<'a>(d: &'a Mindmap, size: &'a [(f64, f64)]) -> Option<Fan<'a>> {
        let n = d.nodes.len();
        let mut pre = Vec::with_capacity(n);
        let mut span = vec![(0, 0); n];
        let mut stack = vec![(0usize, false)];
        while let Some((i, done)) = stack.pop() {
            if done {
                span[i].1 = pre.len();
                continue;
            }
            span[i].0 = pre.len();
            pre.push(i);
            stack.push((i, true));
            for &c in d.nodes[i].children.iter().rev() {
                stack.push((c, false));
            }
        }
        if pre.len() != n {
            return None;
        }
        let mut fan = Fan {
            d,
            size,
            pre,
            span,
            memo: vec![Vec::new(); n],
            entries: Vec::new(),
            leaf: vec![None; n],
            fitted: vec![None; n],
            failed: vec![0.0; n],
            work: std::cell::Cell::new(0),
            units: (0..360 / DEG_STEP)
                .map(|k| (rad(k * DEG_STEP).cos(), rad(k * DEG_STEP).sin()))
                .collect(),
            last: vec![None; n],
            aimed: vec![None; n],
            narrow: false,
        };
        for i in 0..n {
            if d.nodes[i].children.is_empty() {
                let b = fan.boxed(i, (0.0, 0.0));
                fan.entries.push(Entry {
                    shape: Shape {
                        boxes: vec![b],
                        segs: Vec::new(),
                        bbox: b,
                    },
                    pos: Vec::new(),
                });
                fan.leaf[i] = Some(fan.entries.len() - 1);
            }
        }
        Some(fan)
    }

    /// Unit vector along `deg` degrees.
    fn unit(&self, deg: i32) -> (f64, f64) {
        let d = deg.rem_euclid(360);
        if d % DEG_STEP == 0 {
            self.units[(d / DEG_STEP) as usize]
        } else {
            (rad(d).cos(), rad(d).sin())
        }
    }

    /// Widest fan (radians) non-root node `i` may open to.
    fn widest(&self, i: usize) -> f64 {
        if self.narrow && self.d.nodes[i].depth == 1 {
            NARROW_FAN
        } else {
            MAX_FAN_WIDE
        }
    }

    fn spent(&self) -> bool {
        self.work.get() > FAN_WORK
    }

    fn boxed(&self, m: usize, at: (f64, f64)) -> Rect {
        let (w, h) = self.size[m];
        (
            at.0 - w / 2.0,
            at.1 - h / 2.0,
            at.0 + w / 2.0,
            at.1 + h / 2.0,
        )
    }

    /// The subtree of `c` laid out for incoming direction `deg`, as an
    /// index into `entries`; `None` when it cannot be laid out there.
    fn layout(&mut self, c: usize, deg: i32) -> Option<usize> {
        if let Some(e) = self.leaf[c] {
            return Some(e);
        }
        // Directions are whole steps; anything else rounds down.
        let deg = deg.rem_euclid(360).div_euclid(DEG_STEP) * DEG_STEP;
        let slot = (deg / DEG_STEP) as usize;
        if self.memo[c].is_empty() {
            self.memo[c] = vec![0; (360 / DEG_STEP) as usize];
        }
        match self.memo[c][slot] {
            0 => {}
            1 => return None,
            e => return Some(e as usize - 2),
        }
        let r = self.fan(c, deg).map(|pos| {
            let base = self.span[c].0 + 1;
            let mut boxes = vec![self.boxed(c, (0.0, 0.0))];
            let mut segs = Vec::with_capacity(pos.len());
            for (k, &p) in pos.iter().enumerate() {
                let m = self.pre[base + k];
                boxes.push(self.boxed(m, p));
                let parent = self.d.nodes[m].parent.unwrap_or(c);
                let from = if parent == c {
                    (0.0, 0.0)
                } else {
                    pos[self.span[parent].0 - base]
                };
                segs.push((from, p));
            }
            let bbox = boxes
                .iter()
                .fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |a, b| {
                    (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
                });
            self.entries.push(Entry {
                shape: Shape { boxes, segs, bbox },
                pos,
            });
            self.entries.len() - 1
        });
        self.memo[c][slot] = r.map_or(1, |e| e as u32 + 2);
        if let Some(e) = r {
            self.last[c] = Some((deg, e));
        }
        r
    }

    /// Child `c` of `i` placed for direction `deg`, spokes `gap` long.
    fn put(&mut self, i: usize, c: usize, deg: i32, gap: f64) -> Option<At> {
        let e = self.layout(c, deg)?;
        let u = self.unit(deg);
        let r = spoke_len(self.size[i], self.size[c], u, gap);
        Some(At {
            e,
            o: (u.0 * r, u.1 * r),
        })
    }

    /// Do two children of the same parent keep clear of each other:
    /// boxes [`SEP`] apart, no connector (spokes included) through the
    /// other's boxes, no crossings?
    fn siblings_clear(&self, a: At, b: At) -> bool {
        let (sa, sb) = (&self.entries[a.e].shape, &self.entries[b.e].shape);
        let spoke_a: Seg = ((0.0, 0.0), a.o);
        let spoke_b: Seg = ((0.0, 0.0), b.o);
        if !boxes_near(shift(sa.bbox, a.o), shift(sb.bbox, b.o), SEP + SEG_CLEAR) {
            // A subtree's own connectors run between its boxes' centres,
            // so inside its hull; only the spokes reach outside it — but
            // a spoke can still cut through the other hull between its
            // boxes, across one of its connectors.
            self.work.set(
                self.work.get() + sa.boxes.len() + sb.boxes.len() + sa.segs.len() + sb.segs.len(),
            );
            return !spoke_hits(spoke_a, sb, b.o) && !spoke_hits(spoke_b, sa, a.o);
        }
        self.work.set(
            self.work.get()
                + sa.boxes.len() * sb.boxes.len()
                + 2 * (sa.segs.len() + 1) * (sb.segs.len() + 1),
        );
        if sa.boxes.iter().any(|&p| {
            let p = shift(p, a.o);
            sb.boxes.iter().any(|&q| boxes_near(p, shift(q, b.o), SEP))
        }) {
            return false;
        }
        let segs_a = std::iter::once(spoke_a).chain(sa.segs.iter().map(|&s| shift_seg(s, a.o)));
        for s in segs_a {
            if seg_hits_shape(s, sb, b.o) {
                return false;
            }
            let crosses = std::iter::once(spoke_b)
                .chain(sb.segs.iter().map(|&q| shift_seg(q, b.o)))
                .any(|q| segments_cross(s.0, s.1, q.0, q.1));
            if crosses {
                return false;
            }
        }
        let mut segs_b = std::iter::once(spoke_b).chain(sb.segs.iter().map(|&s| shift_seg(s, b.o)));
        !segs_b.any(|s| seg_hits_shape(s, sa, a.o))
    }

    /// Does a placed child of `i` stay clear of `i` itself — its box
    /// (by [`SEP`]/2) and its incoming connector (`inbound`)?
    fn clear_of_parent(&self, i: usize, a: At, inbound: Option<Seg>) -> bool {
        let own = self.boxed(i, (0.0, 0.0));
        let sh = &self.entries[a.e].shape;
        self.work.set(self.work.get() + 2 * sh.boxes.len());
        // The child's own box keeps its distance through `spoke_len`,
        // and its spoke starts here; test everything below it.
        if sh.boxes[1..]
            .iter()
            .any(|&b| boxes_near(shift(b, a.o), own, SEP / 2.0))
        {
            return false;
        }
        let own_shape = Shape {
            boxes: vec![own],
            segs: Vec::new(),
            bbox: own,
        };
        if sh
            .segs
            .iter()
            .any(|&s| seg_hits_shape(shift_seg(s, a.o), &own_shape, (0.0, 0.0)))
        {
            return false;
        }
        inbound.map_or(true, |r| {
            !seg_hits_shape(r, sh, a.o)
                && !sh.segs.iter().any(|&s| {
                    let s = shift_seg(s, a.o);
                    segments_cross(r.0, r.1, s.0, s.1)
                })
        })
    }

    /// A guess at the spoke gap `i`'s children need once the base length
    /// has failed, so a big crowded fan does not fail its way up ×1.25 at
    /// a time: each child's subtree taken as a disc (its farthest corner
    /// from the child's centre), the discs packed round the fan's arc.
    /// Discs overstate a subtree's angle, so the guess is scaled down and
    /// the sweep still has the last word. `None` when a child's subtree
    /// cannot be laid out.
    fn gap_estimate(&mut self, i: usize, dir: i32, kids: &[usize]) -> Option<f64> {
        let root = self.d.nodes[i].parent.is_none();
        let arc = if root { TAU } else { self.widest(i) } * 0.9;
        let mut discs = Vec::with_capacity(kids.len());
        let mut reach = 0.0f64;
        for &c in kids {
            let e = self.layout(c, dir)?;
            let r = self.entries[e]
                .shape
                .boxes
                .iter()
                .map(|b| b.0.abs().max(b.2.abs()).hypot(b.1.abs().max(b.3.abs())))
                .fold(0.0, f64::max);
            discs.push(r + SEP / 2.0);
            let (w, h) = self.size[c];
            reach = reach.max(w.hypot(h) / 2.0);
        }
        let need = |dist: f64| -> f64 {
            discs
                .iter()
                .map(|&r| 2.0 * (r / dist).min(1.0).asin() + rad(DEG_STEP))
                .sum()
        };
        let (mut lo, mut hi) = (1.0f64, 1.0f64);
        while need(hi) > arc && hi < 1e7 {
            lo = hi;
            hi *= 2.0;
        }
        for _ in 0..40 {
            let mid = (lo + hi) / 2.0;
            if need(mid) > arc {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let (w, h) = self.size[i];
        Some(0.6 * (hi - w.hypot(h) / 2.0 - reach))
    }

    /// Would child `c`'s own box and spoke, for direction `deg`, clear
    /// the siblings in `placed`? A necessary condition for the whole
    /// subtree to clear them, at the cost of a few box tests.
    fn own_box_clear(&self, i: usize, c: usize, deg: i32, gap: f64, placed: &[(i32, At)]) -> bool {
        let u = self.unit(deg);
        let r = spoke_len(self.size[i], self.size[c], u, gap);
        let o = (u.0 * r, u.1 * r);
        let own = self.boxed(c, o);
        let spoke: Seg = ((0.0, 0.0), o);
        placed.iter().all(|&(_, b)| {
            let sb = &self.entries[b.e].shape;
            self.work.set(self.work.get() + sb.boxes.len());
            if !boxes_near(own, shift(sb.bbox, b.o), SEP + SEG_CLEAR) {
                return !seg_hits_shape(spoke, sb, b.o);
            }
            !sb.boxes
                .iter()
                .any(|&q| boxes_near(own, shift(q, b.o), SEP))
                && !seg_hits_shape(spoke, sb, b.o)
        })
    }

    /// Is child `c` of `i`, for direction `deg`, sure to hit a sibling in
    /// `placed` — judged without laying its subtree out for `deg`? Its
    /// latest layout (for another direction) is turned to `deg` box by
    /// box, and only an overlap deeper than [`TURN_SLACK`] on both axes
    /// counts: a turned layout is close to, not equal to, the one `deg`
    /// would get. Laying a subtree out is the sweep's main cost, and
    /// most directions a sweep steps through are blocked by the sibling
    /// just placed, so this skips most of them; a direction it lets
    /// through still gets the exact test.
    fn surely_blocked(&self, i: usize, c: usize, deg: i32, gap: f64, placed: &[(i32, At)]) -> bool {
        let slot = (deg.rem_euclid(360) / DEG_STEP) as usize;
        if placed.is_empty()
            || self.leaf[c].is_some()
            || self.memo[c].get(slot).is_some_and(|&m| m != 0)
        {
            return false;
        }
        let Some((from, e)) = self.last[c] else {
            return false;
        };
        let (cos, sin) = self.unit(deg - from);
        let u = self.unit(deg);
        let r = spoke_len(self.size[i], self.size[c], u, gap);
        let o = (u.0 * r, u.1 * r);
        let boxes = &self.entries[e].shape.boxes;
        self.work.set(self.work.get() + boxes.len());
        let m = TURN_SLACK;
        let deep =
            |a: Rect, b: Rect| a.0 + m < b.2 && b.0 + m < a.2 && a.1 + m < b.3 && b.1 + m < a.3;
        boxes[1..].iter().any(|&b| {
            let (cx, cy) = ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0);
            let (hw, hh) = ((b.2 - b.0) / 2.0, (b.3 - b.1) / 2.0);
            let (x, y) = (cx * cos - cy * sin + o.0, cx * sin + cy * cos + o.1);
            let p = (x - hw, y - hh, x + hw, y + hh);
            placed.iter().rev().any(|&(_, q)| {
                let sq = &self.entries[q.e].shape;
                deep(p, shift(sq.bbox, q.o)) && sq.boxes.iter().any(|&bq| deep(p, shift(bq, q.o)))
            })
        })
    }

    /// Sweep `i`'s children round it, clockwise in source order: child
    /// `j` first tries direction `first[j]`, or its predecessor's plus
    /// `extra` (at least one step) when that is later, then steps
    /// [`DEG_STEP`] at a time to the first direction where its whole
    /// subtree stays clear of `i` and of every child already placed.
    /// `None` once a child would pass `first[0]` plus `limit`. Every
    /// candidate is tested with the child's exact subtree for that
    /// direction, so the result is clean by construction.
    #[allow(clippy::too_many_arguments)]
    fn sweep(
        &mut self,
        i: usize,
        kids: &[usize],
        first: &[i32],
        extra: i32,
        limit: i32,
        gap: f64,
        inbound: Option<Seg>,
    ) -> Option<Vec<(i32, At)>> {
        let start = first[0];
        let mut out: Vec<(i32, At)> = Vec::with_capacity(kids.len());
        for (j, &c) in kids.iter().enumerate() {
            let from = out
                .last()
                .map_or(start, |&(p, _)| first[j].max(p + extra.max(DEG_STEP)));
            let mut test = |fan: &mut Self, t: i32| {
                // Cheap necessary tests first: the child's own box and
                // spoke, then its turned subtree, against the siblings
                // already placed. Most blocked directions fail here,
                // before the child's subtree has to be laid out for them.
                if !fan.own_box_clear(i, c, t, gap, &out) || fan.surely_blocked(i, c, t, gap, &out)
                {
                    return None;
                }
                fan.put(i, c, t, gap)
                    .filter(|&a| fan.fits_among(i, a, &out, usize::MAX, inbound))
            };
            let at = self.first_clear(from, start + limit, &mut test)?;
            out.push(at);
        }
        Some(out)
    }

    /// The first direction in `from..=to` (whole steps) where `test`
    /// places a child, found by striding [`STRIDE`] at a time and then
    /// stepping back through the directions skipped since the last miss:
    /// blocked directions come in runs, so this tests about a third of
    /// them. `None` when none fits or the work budget is spent.
    fn first_clear(
        &mut self,
        from: i32,
        to: i32,
        test: &mut dyn FnMut(&mut Self, i32) -> Option<At>,
    ) -> Option<(i32, At)> {
        let mut missed: Option<i32> = None;
        let mut t = from;
        loop {
            if self.spent() {
                return None;
            }
            let hit = if t > to { None } else { test(self, t) };
            if hit.is_some() || t > to {
                if let Some(m) = missed {
                    let mut s = m + DEG_STEP;
                    while s < t && s <= to {
                        if let Some(a) = test(self, s) {
                            return Some((s, a));
                        }
                        s += DEG_STEP;
                    }
                }
                return hit.map(|a| (t, a));
            }
            missed = Some(t);
            t += STRIDE;
        }
    }

    /// Does child placement `a` stay clear of `i` (see
    /// [`Fan::clear_of_parent`]) and of every sibling in `placed` but
    /// slot `skip`?
    fn fits_among(
        &self,
        i: usize,
        a: At,
        placed: &[(i32, At)],
        skip: usize,
        inbound: Option<Seg>,
    ) -> bool {
        // Newest siblings first: they are the likeliest clash.
        self.clear_of_parent(i, a, inbound)
            && (0..placed.len())
                .rev()
                .all(|q| q == skip || self.siblings_clear(a, placed[q].1))
    }

    /// A lone child: along `dir` when it fits there, else the nearest
    /// direction either side (no sibling to measure a fan against, so no
    /// tight sweep to run first).
    fn single(
        &mut self,
        i: usize,
        c: usize,
        dir: i32,
        gap: f64,
        inbound: Option<Seg>,
    ) -> Option<Vec<(i32, At)>> {
        let half = (MAX_FAN_WIDE * 90.0 / PI) as i32;
        let mut s = 0;
        while s <= half {
            for t in [dir + s, dir - s] {
                if self.spent() {
                    return None;
                }
                if let Some(a) = self.put(i, c, t, gap) {
                    if self.clear_of_parent(i, a, inbound) {
                        return Some(vec![(t, a)]);
                    }
                }
                if s == 0 {
                    break;
                }
            }
            s += DEG_STEP;
        }
        None
    }

    /// `i`'s children placed round it with every spoke `gap` long, or
    /// `None` when they do not fit at that length. A tight sweep (every
    /// child as close to its predecessor as it can go) measures the arc
    /// they need. The root then hands the spare arc out evenly round the
    /// full circle from 12 o'clock, each share ADDED to the separation
    /// that child needed in the tight sweep, so no sector is left empty.
    /// Any other node spreads its children up to [`FAN_PER_CHILD`]
    /// apart, centred on `dir`: a minimum separation, which a child that
    /// needs more absorbs, so a fan opens no wider than it has to and
    /// leaves its parent's other children room.
    fn arrange(
        &mut self,
        i: usize,
        kids: &[usize],
        dir: i32,
        gap: f64,
        inbound: Option<Seg>,
    ) -> Option<Vec<(i32, At)>> {
        let root = inbound.is_none();
        let k = kids.len();
        if k == 1 {
            // A root's only branch reads left to right, like the first
            // of two (which go right and left); any other lone child
            // carries straight on.
            let dir = if root { 0 } else { dir };
            return self.single(i, kids[0], dir, gap, inbound);
        }
        let deg = |r: f64| (r * 180.0 / PI).round() as i32;
        let snap = |d: f64| (d.round() as i32).div_euclid(DEG_STEP) * DEG_STEP;
        if root {
            let tight = self.sweep(i, kids, &vec![dir; k], 0, 360 - DEG_STEP, gap, None)?;
            // Room needed to close the circle: child 0 again, past the
            // last child, must clear children 1.. .
            let t0 = tight[0].0;
            let from = tight[k - 1].0.max(t0 + DEG_STEP);
            let mut test = |fan: &mut Self, t: i32| {
                let blocked = !fan.own_box_clear(i, kids[0], t, gap, &tight[1..])
                    || fan.surely_blocked(i, kids[0], t, gap, &tight[1..]);
                if blocked {
                    return None;
                }
                fan.put(i, kids[0], t, gap)
                    .filter(|&a| (1..k).all(|q| fan.siblings_clear(a, tight[q].1)))
            };
            let closed = self
                .first_clear(from, t0 + 360, &mut test)
                .map(|(t, _)| t - t0);
            let total = closed?;
            if total + DEG_STEP * k as i32 > 360 {
                return None;
            }
            // Child 0's subtree starts just clockwise of 12 o'clock,
            // half a share of the spare arc past it: offset by its own
            // counter-clockwise half-width.
            let a0 = tight[0].1;
            let lo = self.entries[a0.e]
                .shape
                .boxes
                .iter()
                .map(|&b| shift(b, a0.o))
                .flat_map(|b| [(b.0, b.1), (b.2, b.1), (b.2, b.3), (b.0, b.3)])
                .map(|(x, y)| wrap(y.atan2(x) - rad(t0)))
                .fold(0.0f64, f64::min);
            // A spread that its re-laid-out subtrees cannot keep is
            // retried with less of the spare arc, down to none (the
            // tight sweep turned to 12 o'clock), rather than growing
            // every spoke for the sake of even angles.
            for part in [1.0, 0.5, 0.25, 0.0] {
                let share = part * f64::from(360 - total) / k as f64;
                let start = -90.0 - lo * 180.0 / PI + share / 2.0;
                let first: Vec<i32> = tight
                    .iter()
                    .enumerate()
                    .map(|(j, &(t, _))| snap(start + f64::from(t - t0) + share * j as f64))
                    .collect();
                let limit = 360 - DEG_STEP - (share / 2.0) as i32;
                if let Some(out) = self.sweep(i, kids, &first, 0, limit, gap, None) {
                    return Some(out);
                }
            }
            None
        } else {
            let limit = deg(self.widest(i));
            // This node's fan for a neighbouring direction, turned to
            // `dir`, is a close guess: try that first, and keep it when
            // it still points along `dir`. Measuring the fan again with
            // a tight sweep lays every child's subtree out for
            // directions the final fan does not use, which on a deep
            // tree costs more than the rest of the layout.
            if let Some((lead, extra)) = self.aimed[i] {
                let from = snap(f64::from(dir - lead));
                if let Some(out) = self.sweep(i, kids, &vec![from; k], extra, limit, gap, inbound) {
                    if (self.heading(kids, &out) - f64::from(dir)).abs() <= f64::from(STRIDE) {
                        return Some(out);
                    }
                }
            }
            // The tight sweep starts half the widest fan back from `dir`
            // (never across the band behind `i`, where its connector
            // arrives) and may use all of it.
            let from = snap(f64::from(dir) - f64::from(limit) / 2.0);
            let tight = self.sweep(i, kids, &vec![from; k], 0, limit, gap, inbound)?;
            // A fan keeps to MAX_FAN when it fits there and opens only as
            // wide as it must (up to MAX_FAN_WIDE).
            let span = tight[k - 1].0 - tight[0].0;
            let arc = span.max(deg(MAX_FAN.min(self.widest(i))));
            let want = (deg(FAN_PER_CHILD) * (k as i32 - 1)).min(arc);
            let extra = snap(f64::from((want - span).max(0)) / (k - 1) as f64);
            let from = self.aim(i, kids, &tight, extra, dir);
            let (out, extra) = match self.sweep(i, kids, &vec![from; k], extra, limit, gap, inbound)
            {
                Some(out) => (out, extra),
                None => {
                    // Turning the fan reshaped it; try it tight.
                    let from = self.aim(i, kids, &tight, 0, dir);
                    (
                        self.sweep(i, kids, &vec![from; k], 0, limit, gap, inbound)?,
                        0,
                    )
                }
            };
            self.aimed[i] = Some((dir - out[0].0, extra));
            Some(out)
        }
    }

    /// Mean direction of a placed fan's children, weighted by subtree
    /// size (see [`Fan::aim`]).
    fn heading(&self, kids: &[usize], out: &[(i32, At)]) -> f64 {
        let (mut sum, mut weight) = (0.0, 0.0);
        for (&c, &(t, _)) in kids.iter().zip(out) {
            let w = (self.span[c].1 - self.span[c].0) as f64;
            sum += w * f64::from(t);
            weight += w;
        }
        sum / weight
    }

    /// Where a non-root fan starts so that it points along `dir`: its
    /// children at the tight sweep's separations (or `extra`, if more),
    /// turned so their mean direction, weighted by subtree size, is
    /// `dir` — a balanced fan is centred, while a child carrying most of
    /// the subtree carries straight on. Always centring would bend a
    /// long chain or a comb (leaves, then the child that goes on) the
    /// same way at every level until it curled into itself. The fan is
    /// kept within [`MAX_FAN_WIDE`] of `dir`.
    fn aim(&self, i: usize, kids: &[usize], tight: &[(i32, At)], extra: i32, dir: i32) -> i32 {
        let mut at = 0;
        let (mut sum, mut weight) = (0.0, 0.0);
        for (j, &c) in kids.iter().enumerate() {
            if j > 0 {
                at += extra.max(tight[j].0 - tight[j - 1].0);
            }
            let w = (self.span[c].1 - self.span[c].0) as f64;
            sum += w * f64::from(at);
            weight += w;
        }
        let half = (self.widest(i) * 90.0 / PI) as i32;
        let lead = ((sum / weight).round() as i32)
            .clamp(at - half, half)
            .max(at / 2 - half);
        (dir - lead).div_euclid(DEG_STEP) * DEG_STEP
    }

    /// Lay out `i`'s descendants round it (at the origin), its own
    /// connector arriving along `dir` degrees (screen coordinates:
    /// 0 = right, 90 = down). Returns every strict descendant's centre
    /// relative to `i`, in preorder; `None` when the fan cannot be made
    /// to fit or the work budget is spent.
    ///
    /// Each child sits on a spoke from `i` (see [`spoke_len`]), arranged
    /// by [`Fan::arrange`]. The base length ([`TRUNK`] from the root,
    /// [`SPOKE`] below) is tried first; a fan that does not fit there
    /// grows its spokes (×1.25, or straight to [`Fan::gap_estimate`])
    /// until it does, then bisects back down between the last length
    /// that failed and the first that fitted — so the shared length is
    /// no longer than the fan needs. Finally every child's own spoke is
    /// shortened as far as it stays clear (see [`Fan::shorten`]): a
    /// small branch sits close in even when a bushy sibling needs room.
    fn fan(&mut self, i: usize, dir: i32) -> Option<Vec<(f64, f64)>> {
        let kids = self.d.nodes[i].children.clone();
        let base = self.span[i].0 + 1;
        let mut pos = vec![(0.0, 0.0); self.span[i].1 - base];
        if kids.is_empty() {
            return Some(pos);
        }
        let root = self.d.nodes[i].parent.is_none();
        let floor = if root { TRUNK } else { SPOKE };
        let inbound = (!root).then(|| {
            let back = rad(dir) + PI;
            (
                (0.0, 0.0),
                (back.cos() * INBOUND_RAY, back.sin() * INBOUND_RAY),
            )
        });
        let mut gap = self.fitted[i].unwrap_or(floor);
        let mut estimated = self.fitted[i].is_some();
        // The longest gap known not to fit: for this direction once one
        // fails, until then the longest any direction failed at (or the
        // base length, untried, when a memoised gap fits first time).
        let mut failed = self.failed[i].max(floor).min(gap);
        let mut placed = None;
        for _ in 0..FAN_GROWTHS {
            if self.spent() {
                return None;
            }
            if let Some(p) = self.arrange(i, &kids, dir, gap, inbound) {
                placed = Some(p);
                break;
            }
            failed = gap;
            gap *= 1.25;
            if !estimated {
                // The base length failed: jump straight to (most of) the
                // estimate rather than ×1.25 a step at a time; the
                // bisection below takes back any overshoot.
                estimated = true;
                gap = gap.max(self.gap_estimate(i, dir, &kids)?);
            }
        }
        let mut placed = placed?;
        let mut tries = 0;
        while gap > failed * GAP_TOLERANCE && tries < GAP_BISECTIONS {
            tries += 1;
            let mid = (failed + gap) / 2.0;
            match self.arrange(i, &kids, dir, mid, inbound) {
                Some(p) => {
                    placed = p;
                    gap = mid;
                }
                None => failed = mid,
            }
        }
        if gap > floor {
            self.shorten(i, &kids, &mut placed, floor, gap, inbound);
        }
        self.fitted[i] = Some(self.fitted[i].map_or(gap, |g| g.min(gap)));
        if failed < gap {
            self.failed[i] = self.failed[i].max(failed);
        }
        for (&c, &(_, a)) in kids.iter().zip(&placed) {
            let at = self.span[c].0 - base;
            pos[at] = a.o;
            for (t, &q) in self.entries[a.e].pos.iter().enumerate() {
                pos[at + 1 + t] = (q.0 + a.o.0, q.1 + a.o.1);
            }
        }
        Some(pos)
    }

    /// Give each child of `i` its own spoke. A shared length sized for
    /// the bushiest child would leave a leaf branch at the end of a long
    /// empty trunk, so the children slide in along their directions
    /// together: round after round, each takes a step of
    /// [`SHORTEN_STEP`] towards `floor` if it stays clear of `i` and of
    /// its siblings as they now stand, until no child can move; then
    /// each closes the last step it missed by bisection. Moving together
    /// (rather than one child all the way at a time) keeps a small
    /// branch from taking the room next to the parent that its
    /// neighbours' spokes needed. The result is still clean: every
    /// accepted position passed the same tests as the sweep.
    #[allow(clippy::too_many_arguments)]
    fn shorten(
        &mut self,
        i: usize,
        kids: &[usize],
        placed: &mut [(i32, At)],
        floor: f64,
        gap: f64,
        inbound: Option<Seg>,
    ) {
        let k = kids.len();
        // Each child's gap now and the shortest it failed at.
        let mut now = vec![gap; k];
        let mut missed = vec![floor; k];
        let fits = |fan: &mut Self, placed: &[(i32, At)], j: usize, g: f64| {
            let (t, _) = placed[j];
            fan.put(i, kids[j], t, g)
                .filter(|&a| fan.fits_among(i, a, placed, j, inbound))
        };
        let mut moved = true;
        while moved && !self.spent() {
            moved = false;
            for j in 0..k {
                if now[j] <= floor {
                    continue;
                }
                let g = (now[j] * SHORTEN_STEP).max(floor);
                match fits(self, placed, j, g) {
                    Some(a) => {
                        placed[j].1 = a;
                        now[j] = g;
                        moved = true;
                    }
                    None => missed[j] = g,
                }
            }
        }
        for j in 0..k {
            let (mut lo, mut hi) = (missed[j], now[j]);
            while hi > lo * GAP_TOLERANCE && !self.spent() {
                let mid = (lo + hi) / 2.0;
                match fits(self, placed, j, mid) {
                    Some(a) => {
                        placed[j].1 = a;
                        hi = mid;
                    }
                    None => lo = mid,
                }
            }
        }
    }
}

/// Centres (root at the origin) from the parent-centred fan layout, or
/// `None` when it could not produce a clean layout — see [`scene`].
fn fan_centres(d: &Mindmap, size: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let mut fan = Fan::new(d, size)?;
    let run = |fan: &mut Fan| -> Option<Vec<(f64, f64)>> {
        let below = fan.fan(0, -90)?;
        let mut pos = vec![(0.0, 0.0); d.nodes.len()];
        for (k, &p) in below.iter().enumerate() {
            pos[fan.pre[1 + k]] = p;
        }
        clean(d, size, &pos).then_some(pos)
    };
    let wide = run(&mut fan);
    // A crowded root (many branches with subtrees of their own) needs
    // long trunks to fit its branches' fans side by side; narrow
    // first-level fans trade a little length below the branches for
    // shorter trunks. Keep whichever has the shorter longest connector.
    let kids = &d.nodes[0].children;
    let crowded = kids.len() >= NARROW_FROM
        && kids.iter().any(|&c| !d.nodes[c].children.is_empty())
        && wide.as_ref().map_or(true, |p| {
            let (len, at) = longest_connector(d, size, p);
            len > NARROW_TRUNK && d.nodes[at].depth == 1
        });
    if !crowded {
        return wide;
    }
    // Only the root's and the first level's layouts change; everything
    // deeper is laid out as before and stays memoised.
    for i in std::iter::once(0).chain(kids.iter().copied()) {
        fan.memo[i].clear();
        fan.fitted[i] = None;
        fan.failed[i] = 0.0;
        fan.aimed[i] = None;
        fan.last[i] = None;
    }
    fan.narrow = true;
    match (wide, run(&mut fan)) {
        (Some(w), Some(n)) => {
            let better = longest_connector(d, size, &n).0 < 0.9 * longest_connector(d, size, &w).0;
            Some(if better { n } else { w })
        }
        (w, n) => w.or(n),
    }
}

/// Longest visible connector (border to border) of a layout, and the
/// node it leads to.
fn longest_connector(d: &Mindmap, size: &[(f64, f64)], pos: &[(f64, f64)]) -> (f64, usize) {
    d.nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| n.parent.map(|p| (p, i)))
        .map(|(p, i)| {
            let (dx, dy) = (pos[i].0 - pos[p].0, pos[i].1 - pos[p].1);
            let len = dx.hypot(dy).max(1e-9);
            let u = (dx / len, dy / len);
            (
                len - ext(size[p].0, size[p].1, u) - ext(size[i].0, size[i].1, u),
                i,
            )
        })
        .fold((0.0, 0), |a, b| if b.0 > a.0 { b } else { a })
}

/// The fan layout's safety net: no two boxes closer than half of
/// [`SEP`], no connector crossing another, none running through a box
/// other than its own two ends. The sweep guarantees these by
/// construction; this re-checks the finished geometry (skipped on huge
/// maps, where the quadratic check would cost more than the layout).
fn clean(d: &Mindmap, size: &[(f64, f64)], pos: &[(f64, f64)]) -> bool {
    let n = pos.len();
    if n > 1500 {
        return true;
    }
    let boxes: Vec<(f64, f64, f64, f64)> = (0..n)
        .map(|i| {
            let (w, h) = size[i];
            (
                pos[i].0 - w / 2.0,
                pos[i].1 - h / 2.0,
                pos[i].0 + w / 2.0,
                pos[i].1 + h / 2.0,
            )
        })
        .collect();
    let m = SEP / 2.0 - 1e-6;
    for a in 0..n {
        for b in a + 1..n {
            let (p, q) = (boxes[a], boxes[b]);
            if p.0 < q.2 + m && q.0 < p.2 + m && p.1 < q.3 + m && q.1 < p.3 + m {
                return false;
            }
        }
    }
    let edges: Vec<(usize, usize)> = d
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(i, nd)| nd.parent.map(|p| (p, i)))
        .collect();
    for (x, &(a, b)) in edges.iter().enumerate() {
        for &(c, e) in &edges[x + 1..] {
            if a == c || a == e || b == c || b == e {
                continue;
            }
            if segments_cross(pos[a], pos[b], pos[c], pos[e]) {
                return false;
            }
        }
        for (i, bx) in boxes.iter().enumerate() {
            if i != a && i != b && segment_hits_box(pos[a], pos[b], *bx) {
                return false;
            }
        }
    }
    true
}

/// Proper crossing of segments `p1p2` and `p3p4`.
fn segments_cross(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), p4: (f64, f64)) -> bool {
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let (d1, d2) = (cross(p3, p4, p1), cross(p3, p4, p2));
    let (d3, d4) = (cross(p1, p2, p3), cross(p1, p2, p4));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// Does segment `a`→`b` pass through the box `(x0, y0, x1, y1)`?
/// (Liang–Barsky clip.)
fn segment_hits_box(a: (f64, f64), b: (f64, f64), bx: (f64, f64, f64, f64)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - bx.0),
        (dx, bx.2 - a.0),
        (-dy, a.1 - bx.1),
        (dy, bx.3 - a.1),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    t0 < t1
}

/// Angular half-width (radians) a node occupies on a ring of radius
/// `r` (fallback layout): the box diagonal projected onto the circle.
fn angular_half(diag: f64, r: f64) -> f64 {
    if r <= 0.0 {
        return 0.0;
    }
    ((diag * 0.5) / r).min(1.0).asin()
}

/// Bottom-up angular extent of each subtree on the rings (fallback).
fn extent(d: &Mindmap, i: usize, diag: &[f64], rad: &[f64], e: &mut [f64]) -> f64 {
    let own = if d.nodes[i].depth == 0 {
        0.0
    } else {
        2.0 * angular_half(diag[i], rad[d.nodes[i].depth]) + GAP
    };
    let kids = &d.nodes[i].children;
    let total = if kids.is_empty() {
        own
    } else {
        own.max(kids.iter().map(|&c| extent(d, c, diag, rad, e)).sum())
    };
    e[i] = total;
    total
}

/// Place subtree `i` inside the wedge `[a0, a1]` on the rings (fallback).
fn place(d: &Mindmap, e: &[f64], rad: &[f64], i: usize, a0: f64, a1: f64, c: &mut [(f64, f64)]) {
    let kids = &d.nodes[i].children;
    if kids.is_empty() {
        return;
    }
    let total: f64 = kids.iter().map(|&k| e[k]).sum();
    let mut acc = a0;
    for &k in kids {
        let span = if total > 0.0 {
            (a1 - a0) * (e[k] / total)
        } else {
            0.0
        };
        let (cs, ce) = (acc, acc + span);
        let mid = (cs + ce) / 2.0;
        let r = rad[d.nodes[k].depth];
        c[k] = (r * mid.cos(), r * mid.sin());
        place(d, e, rad, k, cs, ce, c);
        acc = ce;
    }
}

/// The fallback ring layout (flowmaid's original): depth `k` on a ring
/// of radius `k · RING` (grown until its nodes fit), each subtree owning
/// a wedge sized by its boxes' diagonals. Sparse, but overlap- and
/// crossing-free for any tree, so it backs the fan layout up.
fn ring_centres(d: &Mindmap, sizes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let diag: Vec<f64> = sizes.iter().map(|&(w, h)| w.hypot(h)).collect();
    let max_depth = d.nodes.iter().map(|n| n.depth).max().unwrap_or(0);
    let mut rad: Vec<f64> = (0..=max_depth).map(|k| RING * k as f64).collect();
    let mut demand: Vec<f64> = vec![0.0; max_depth + 1];
    for (i, n) in d.nodes.iter().enumerate() {
        if n.depth > 0 {
            demand[n.depth] += 2.0 * angular_half(diag[i], rad[n.depth]) + GAP;
        }
    }
    for k in 1..=max_depth {
        if demand[k] > TAU {
            rad[k] *= demand[k] / TAU;
        }
    }
    let mut e = vec![0.0f64; d.nodes.len()];
    extent(d, 0, &diag, &rad, &mut e);
    let mut c = vec![(0.0f64, 0.0f64); d.nodes.len()];
    place(d, &e, &rad, 0, -FRAC_PI_2, TAU - FRAC_PI_2, &mut c);
    c
}

/// Branch accent for a top-level subtree, by its order under the root.
fn branch_color(order: usize) -> &'static str {
    crate::style::accent(order)
}

/// Auto-layout: the parent-centred fan layout (see the module docs),
/// then [`route`] with those centres. Should a fan ever fail to fit or
/// the finished geometry fail its overlap/crossing check, the whole map
/// falls back to the ring layout, which is sparse but always clean. An
/// empty mindmap yields a minimal canvas.
pub fn scene(d: &Mindmap) -> MindScene {
    if d.nodes.is_empty() {
        return route(d, &[]);
    }
    let sizes: Vec<(f64, f64)> = d.nodes.iter().map(mind_node_size).collect();
    let c = fan_centres(d, &sizes).unwrap_or_else(|| ring_centres(d, &sizes));

    // Normalise so the whole tree's bounding box starts at MARGIN, then
    // hand the resulting centres to route().
    let (mut minx, mut miny) = (f64::MAX, f64::MAX);
    for (i, &(x, y)) in c.iter().enumerate() {
        minx = minx.min(x - sizes[i].0 / 2.0);
        miny = miny.min(y - sizes[i].1 / 2.0);
    }
    let centres: Vec<(f64, f64)> = c
        .iter()
        .map(|&(x, y)| (x + MARGIN - minx, y + MARGIN - miny))
        .collect();
    route(d, &centres)
}

/// Connector width by the child's depth: a heavy trunk from the root
/// halving per level (9, 4.5, 2.25, then 2), so the hierarchy reads
/// from the strokes alone (mermaid steps 11 → 5).
fn link_width(depth: usize) -> f64 {
    (9.0 * 0.5f64.powi(depth.saturating_sub(1) as i32)).max(2.0)
}

/// Build a scene from EXPLICIT node centres (`pos[i]` = centre of node
/// `i`, in world coords). Used for interactive drag: the caller moves a
/// centre and the connectors are recomputed to follow, without touching
/// the layout. Positions are taken as-is (not re-normalised), so
/// dragging never makes the diagram slide. `pos` must match node count.
pub fn route(d: &Mindmap, pos: &[(f64, f64)]) -> MindScene {
    if d.nodes.is_empty() || pos.len() != d.nodes.len() {
        return MindScene {
            width: 2.0 * MARGIN,
            height: 2.0 * MARGIN,
            nodes: Vec::new(),
            edges: Vec::new(),
        };
    }

    let mut branch_order = vec![0usize; d.nodes.len()];
    for (order, &ch) in d.nodes[0].children.iter().enumerate() {
        branch_order[ch] = order;
    }

    let mut nodes = Vec::with_capacity(d.nodes.len());
    for (i, n) in d.nodes.iter().enumerate() {
        let (w2, h2) = mind_node_size(n);
        // The root is the deep ink; branches fill with their accent.
        let fill = if n.depth == 0 {
            ROOT_FILL
        } else {
            branch_color(branch_order[n.branch.unwrap_or(i)])
        };
        // Text is white or dark ink by contrast with the fill actually
        // painted (white alone failed AA on most accents); a `classDef`
        // fill may be a named colour or `rgb()`, measured as hex.
        let painted = n.style.fill.as_deref().unwrap_or(fill);
        let text_color = text_on(&css_hex(painted).unwrap_or_else(|| painted.to_string()));
        let (font_size, font_weight) = font_of(n.depth);
        nodes.push(MindNodeBox {
            x: pos[i].0 - w2 / 2.0,
            y: pos[i].1 - h2 / 2.0,
            w: w2,
            h: h2,
            text: n.text.clone(),
            shape: n.shape,
            depth: n.depth,
            fill,
            text_color,
            font_size,
            font_weight,
            icon: n.icon.clone(),
            classes: n.classes.clone(),
            style: n.style.clone(),
        });
    }

    // Connectors: centre-to-centre, coloured by the child's branch and
    // tapering away from the root; the node boxes hide both endpoints.
    let mut edges = Vec::new();
    for (i, n) in d.nodes.iter().enumerate() {
        let Some(p) = n.parent else { continue };
        let (a, b) = (
            (nodes[p].cx(), nodes[p].cy()),
            (nodes[i].cx(), nodes[i].cy()),
        );
        let c1 = (a.0 + (b.0 - a.0) / 3.0, a.1 + (b.1 - a.1) / 3.0);
        let c2 = (a.0 + 2.0 * (b.0 - a.0) / 3.0, a.1 + 2.0 * (b.1 - a.1) / 3.0);
        edges.push(MindLink {
            bezier: [a, c1, c2, b],
            color: nodes[i].fill,
            width: link_width(n.depth),
        });
    }

    let width = nodes.iter().map(|n| n.x + n.w).fold(0.0, f64::max) + MARGIN;
    let height = nodes.iter().map(|n| n.y + n.h).fold(0.0, f64::max) + MARGIN;
    MindScene {
        width,
        height,
        nodes,
        edges,
    }
}

// ------------------------------------------------------------------
// Shapes
// ------------------------------------------------------------------

/// Perimeter points (absolute coords) for the polygon shapes — hexagon,
/// bang (spiky), cloud (lobed). `None` for box and circle shapes, which
/// a painter draws with its own rect / ellipse primitive (a circle has
/// `w == h`). Exposed so the SVG and any GUI painter share identical
/// geometry.
pub fn perimeter(n: &MindNodeBox) -> Option<Vec<(f64, f64)>> {
    match n.shape {
        MindShape::Hexagon => Some(hexagon_pts(n)),
        MindShape::Bang => Some(bang_pts(n)),
        MindShape::Cloud => Some(cloud_pts(n)),
        _ => None,
    }
}

/// The underline of a [`MindShape::Default`] node, as a rectangle
/// `(x, y, w, h)` along its flat bottom edge: paint it black at
/// [`UNDERLINE_SHADE`] opacity over the fill. `None` for other shapes.
pub fn underline(n: &MindNodeBox) -> Option<(f64, f64, f64, f64)> {
    (n.shape == MindShape::Default).then_some((n.x, n.y + n.h - UNDERLINE_H, n.w, UNDERLINE_H))
}

/// Where the label's text block is centred: the box centre, nudged
/// right by half the icon's room when the node has an `::icon`.
pub fn label_centre(n: &MindNodeBox) -> (f64, f64) {
    (n.cx() + icon_room(n.icon.is_some()) / 2.0, n.cy())
}

fn hexagon_pts(n: &MindNodeBox) -> Vec<(f64, f64)> {
    let (x, y, w, h) = (n.x, n.y, n.w, n.h);
    let k = (h * 0.5).min(w / 3.0);
    vec![
        (x + k, y),
        (x + w - k, y),
        (x + w, y + h / 2.0),
        (x + w - k, y + h),
        (x + k, y + h),
        (x, y + h / 2.0),
    ]
}

/// Parametric angles of `count` points spaced evenly by ARC LENGTH round
/// the ellipse `(rx, ry)`, the first at 12 o'clock, clockwise. Spacing
/// by parametric angle instead bunches points at the ends of a flat
/// ellipse (the old cloud's long spikes at 3 and 9 o'clock).
fn even_angles(rx: f64, ry: f64, count: usize, phase: f64) -> Vec<f64> {
    const SAMPLES: usize = 720;
    let at = |t: f64| (rx * t.cos(), ry * t.sin());
    let mut cum = Vec::with_capacity(SAMPLES + 1);
    cum.push(0.0f64);
    let mut prev = at(-FRAC_PI_2);
    for s in 1..=SAMPLES {
        let p = at(-FRAC_PI_2 + TAU * s as f64 / SAMPLES as f64);
        cum.push(cum[s - 1] + (p.0 - prev.0).hypot(p.1 - prev.1));
        prev = p;
    }
    let total = cum[SAMPLES];
    let mut out = Vec::with_capacity(count);
    let mut s = 0;
    for j in 0..count {
        let target = total * (j as f64 + phase) / count as f64;
        while s < SAMPLES && cum[s + 1] < target {
            s += 1;
        }
        let seg = (cum[s + 1] - cum[s]).max(1e-12);
        let f = ((target - cum[s]) / seg).clamp(0.0, 1.0);
        out.push(-FRAC_PI_2 + TAU * (s as f64 + f) / SAMPLES as f64);
    }
    out
}

/// Ramanujan's approximation of an ellipse's perimeter.
fn ellipse_perimeter(rx: f64, ry: f64) -> f64 {
    PI * (3.0 * (rx + ry) - ((3.0 * rx + ry) * (rx + 3.0 * ry)).sqrt())
}

/// A bang: spikes of one depth ([`BANG_SPIKE`], along the outline's
/// normal) evenly spaced by arc length round the stadium that holds the
/// label (see [`bang_core`]), the first tip at 12 o'clock. Scaling a
/// unit star by the box instead gave a flat label 4:1 longer spikes at
/// its sides than on top.
fn bang_pts(n: &MindNodeBox) -> Vec<(f64, f64)> {
    let (cx, cy) = (n.cx(), n.cy());
    let r = (n.h / 2.0 - BANG_SPIKE).max(1.0);
    let l = (n.w / 2.0 - BANG_SPIKE - r).max(0.0);
    let perimeter = 4.0 * l + TAU * r;
    let tips = ((perimeter / BANG_STEP).round() as usize).max(8);
    // Point and outward normal at arc length `s`, clockwise from the
    // top centre: top side, right cap, bottom side, left cap, top side.
    let at = |s: f64| -> ((f64, f64), (f64, f64)) {
        let cap = PI * r;
        if s < l {
            ((s, -r), (0.0, -1.0))
        } else if s < l + cap {
            let a = -FRAC_PI_2 + (s - l) / r;
            ((l + r * a.cos(), r * a.sin()), (a.cos(), a.sin()))
        } else if s < 3.0 * l + cap {
            ((l - (s - l - cap), r), (0.0, 1.0))
        } else if s < 3.0 * l + 2.0 * cap {
            let a = FRAC_PI_2 + (s - 3.0 * l - cap) / r;
            ((-l + r * a.cos(), r * a.sin()), (a.cos(), a.sin()))
        } else {
            ((s - 4.0 * l - 2.0 * cap, -r), (0.0, -1.0))
        }
    };
    (0..2 * tips)
        .map(|j| {
            let ((px, py), (nx, ny)) = at(perimeter * j as f64 / (2 * tips) as f64);
            let out = if j % 2 == 0 { BANG_SPIKE } else { 0.0 };
            (cx + px + out * nx, cy + py + out * ny)
        })
        .collect()
}

/// A cloud: round lobes of radius [`CLOUD_LOBE`] centred on an ellipse
/// that holds the label, spaced so neighbours overlap; the outline is
/// each lobe's outer arc between its two neighbours' crossings, sampled
/// as a polygon so painters need no arc primitive.
fn cloud_pts(n: &MindNodeBox) -> Vec<(f64, f64)> {
    let (cx, cy) = (n.cx(), n.cy());
    let r = CLOUD_LOBE;
    let (rx, ry) = ((n.w / 2.0 - r).max(1.0), (n.h / 2.0 - r).max(1.0));
    let m = ((ellipse_perimeter(rx, ry) / CLOUD_STEP).round() as usize).max(6);
    let centres: Vec<(f64, f64)> = even_angles(rx, ry, m, 0.0)
        .into_iter()
        .map(|t| (cx + rx * t.cos(), cy + ry * t.sin()))
        .collect();
    // Outer crossing of lobe j and lobe j+1.
    let cross: Vec<(f64, f64)> = (0..m)
        .map(|j| {
            let (a, b) = (centres[j], centres[(j + 1) % m]);
            let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let half = dx.hypot(dy) / 2.0;
            let t = (r * r - half * half).max(0.0).sqrt();
            // Normal to the chord, pointing away from the centre.
            let mut nrm = (dy, -dx);
            if nrm.0 * (mid.0 - cx) + nrm.1 * (mid.1 - cy) < 0.0 {
                nrm = (-nrm.0, -nrm.1);
            }
            let l = nrm.0.hypot(nrm.1).max(1e-12);
            (mid.0 + t * nrm.0 / l, mid.1 + t * nrm.1 / l)
        })
        .collect();
    const PER_LOBE: usize = 9;
    let mut pts = Vec::with_capacity(m * PER_LOBE);
    for j in 0..m {
        let c = centres[j];
        let from = cross[(j + m - 1) % m];
        let to = cross[j];
        let a0 = (from.1 - c.1).atan2(from.0 - c.0);
        let mut a1 = (to.1 - c.1).atan2(to.0 - c.0);
        // Clockwise (increasing angle on screen) round the outside.
        while a1 <= a0 {
            a1 += TAU;
        }
        for s in 0..PER_LOBE {
            let a = a0 + (a1 - a0) * s as f64 / PER_LOBE as f64;
            pts.push((c.0 + r * a.cos(), c.1 + r * a.sin()));
        }
    }
    pts
}

// ------------------------------------------------------------------
// Icons
// ------------------------------------------------------------------

/// Built-in pictograms for `::icon(…)`. flowmaid ships no icon font, so
/// a handful of common names (font-awesome / material style) map to
/// small line drawings; anything else gets a neutral sparkle. Each is a
/// set of polylines in a unit square.
fn icon_shape(icon: &str) -> Vec<Vec<(f64, f64)>> {
    let name = icon
        .split_whitespace()
        .last()
        .unwrap_or("")
        .to_ascii_lowercase();
    let has = |keys: &[&str]| keys.iter().any(|k| name.contains(k));
    let arc = |cx: f64, cy: f64, r: f64, a0: f64, a1: f64| -> Vec<(f64, f64)> {
        let steps = (((a1 - a0).abs() / (PI / 10.0)).ceil() as usize).max(2);
        (0..=steps)
            .map(|s| {
                let a = (a0 + (a1 - a0) * s as f64 / steps as f64).to_radians();
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    };
    let circle = |cx: f64, cy: f64, r: f64| arc(cx, cy, r, 0.0, 360.0);
    if has(&["file", "page", "doc"]) {
        vec![
            vec![
                (0.2, 0.0),
                (0.62, 0.0),
                (0.85, 0.24),
                (0.85, 1.0),
                (0.2, 1.0),
                (0.2, 0.0),
            ],
            vec![(0.62, 0.0), (0.62, 0.24), (0.85, 0.24)],
        ]
    } else if has(&["book"]) {
        vec![vec![
            (0.5, 0.2),
            (0.05, 0.06),
            (0.05, 0.86),
            (0.5, 1.0),
            (0.95, 0.86),
            (0.95, 0.06),
            (0.5, 0.2),
            (0.5, 1.0),
        ]]
    } else if has(&["code", "terminal"]) {
        vec![
            vec![(0.3, 0.2), (0.02, 0.5), (0.3, 0.8)],
            vec![(0.7, 0.2), (0.98, 0.5), (0.7, 0.8)],
            vec![(0.58, 0.08), (0.42, 0.92)],
        ]
    } else if has(&["cog", "gear", "setting", "wrench", "tool"]) {
        let mut v = vec![circle(0.5, 0.5, 0.2)];
        for s in 0..8 {
            let a = (s as f64 * 45.0).to_radians();
            v.push(vec![
                (0.5 + 0.3 * a.cos(), 0.5 + 0.3 * a.sin()),
                (0.5 + 0.48 * a.cos(), 0.5 + 0.48 * a.sin()),
            ]);
        }
        v
    } else if has(&["user", "person", "account", "people", "group", "team"]) {
        vec![circle(0.5, 0.3, 0.2), arc(0.5, 1.0, 0.42, 180.0, 360.0)]
    } else if has(&["home", "house"]) {
        vec![
            vec![(0.05, 0.5), (0.5, 0.08), (0.95, 0.5)],
            vec![(0.2, 0.38), (0.2, 0.95), (0.8, 0.95), (0.8, 0.38)],
        ]
    } else if has(&["star"]) {
        let pts = (0..=10)
            .map(|s| {
                let a = (-90.0 + 36.0 * s as f64).to_radians();
                let r = if s % 2 == 0 { 0.5 } else { 0.2 };
                (0.5 + r * a.cos(), 0.54 + r * a.sin())
            })
            .collect();
        vec![pts]
    } else if has(&["heart", "like", "love"]) {
        let pts = (0..=32)
            .map(|s| {
                let t = TAU * s as f64 / 32.0;
                let x = 16.0 * t.sin().powi(3);
                let y = 13.0 * t.cos()
                    - 5.0 * (2.0 * t).cos()
                    - 2.0 * (3.0 * t).cos()
                    - (4.0 * t).cos();
                (0.5 + x / 34.0, 0.45 - y / 34.0)
            })
            .collect();
        vec![pts]
    } else if has(&["check", "tick", "done"]) {
        vec![vec![(0.08, 0.55), (0.38, 0.84), (0.92, 0.18)]]
    } else if has(&["bolt", "flash", "lightning", "zap", "power"]) {
        vec![vec![
            (0.62, 0.0),
            (0.18, 0.56),
            (0.48, 0.56),
            (0.38, 1.0),
            (0.82, 0.42),
            (0.52, 0.42),
            (0.62, 0.0),
        ]]
    } else if has(&["cloud"]) {
        let mut v = arc(0.26, 0.62, 0.2, 90.0, 290.0);
        v.extend(arc(0.52, 0.48, 0.26, 200.0, 340.0));
        v.extend(arc(0.77, 0.62, 0.2, 250.0, 450.0));
        v.push((0.26, 0.82));
        vec![v]
    } else if has(&["database", "server", "storage", "disk"]) {
        // A cylinder: the top rim, then the sides and the bottom rim.
        let rim = |cy: f64, a0: f64, a1: f64| -> Vec<(f64, f64)> {
            arc(0.0, 0.0, 1.0, a0, a1)
                .into_iter()
                .map(|(x, y)| (0.5 + 0.4 * x, cy + 0.14 * y))
                .collect()
        };
        let mut body = vec![(0.9, 0.16)];
        body.extend(rim(0.84, 0.0, 180.0));
        body.push((0.1, 0.16));
        vec![rim(0.16, 0.0, 360.0), body]
    } else if has(&["globe", "world", "earth", "web", "internet", "language"]) {
        vec![
            circle(0.5, 0.5, 0.48),
            arc(0.5, 0.5, 0.48, 0.0, 360.0)
                .into_iter()
                .map(|(x, y)| (0.5 + (x - 0.5) * 0.42, y))
                .collect(),
            vec![(0.02, 0.5), (0.98, 0.5)],
        ]
    } else if has(&["search", "magnif", "find"]) {
        vec![circle(0.42, 0.42, 0.3), vec![(0.64, 0.64), (0.96, 0.96)]]
    } else if has(&["envelope", "mail", "inbox"]) {
        vec![
            vec![
                (0.02, 0.2),
                (0.98, 0.2),
                (0.98, 0.82),
                (0.02, 0.82),
                (0.02, 0.2),
            ],
            vec![(0.02, 0.2), (0.5, 0.56), (0.98, 0.2)],
        ]
    } else if has(&["lock", "secure", "key", "shield"]) {
        vec![
            vec![
                (0.15, 0.45),
                (0.85, 0.45),
                (0.85, 1.0),
                (0.15, 1.0),
                (0.15, 0.45),
            ],
            arc(0.5, 0.45, 0.26, 180.0, 360.0),
        ]
    } else {
        // Unknown icon: a quiet four-point sparkle marks "icon here".
        vec![vec![
            (0.5, 0.02),
            (0.62, 0.38),
            (0.98, 0.5),
            (0.62, 0.62),
            (0.5, 0.98),
            (0.38, 0.62),
            (0.02, 0.5),
            (0.38, 0.38),
            (0.5, 0.02),
        ]]
    }
}

/// The node's `::icon` glyph as polylines in absolute coordinates — an
/// [`ICON`]-sized square left of the label, stroked in the label colour
/// (1.5px, round joins). `None` when the node has no icon.
pub fn icon_strokes(n: &MindNodeBox) -> Option<Vec<Vec<(f64, f64)>>> {
    let icon = n.icon.as_deref()?;
    let (tw, _) = text_block(&n.text, n.depth);
    let (lx, cy) = label_centre(n);
    let x0 = lx - tw / 2.0 - ICON_GAP - ICON;
    let y0 = cy - ICON / 2.0;
    Some(
        icon_shape(icon)
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .map(|(u, v)| (x0 + u * ICON, y0 + v * ICON))
                    .collect()
            })
            .collect(),
    )
}

// ------------------------------------------------------------------
// SVG
// ------------------------------------------------------------------

/// Serialise a [`MindScene`] to a standalone SVG document.
pub fn to_svg(ms: &MindScene) -> String {
    to_svg_with(ms, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(ms: &MindScene, opts: &SvgOptions) -> String {
    let mut s = String::new();
    svg_open(&mut s, ms.width, ms.height, FONT, "Mind map", opts);

    // Connectors behind the nodes.
    for e in &ms.edges {
        let [a, c1, c2, b] = e.bezier;
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} C {:.1} {:.1} {:.1} {:.1} {:.1} {:.1}\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linecap=\"round\"/>\n",
            a.0, a.1, c1.0, c1.1, c2.0, c2.1, b.0, b.1, e.color, e.width
        ));
    }

    for n in &ms.nodes {
        // `:::` classes travel as a class attribute, as in mermaid, so
        // page CSS can still style the node.
        let group = !n.classes.is_empty();
        if group {
            s.push_str(&format!("<g class=\"{}\">\n", escape(&n.classes.join(" "))));
        }
        s.push_str(&shape_svg(n));
        if let Some(lines) = icon_strokes(n) {
            s.push_str(&icon_svg(n, &lines));
        }
        s.push_str(&label_svg(n));
        if group {
            s.push_str("</g>\n");
        }
    }

    s.push_str("</svg>\n");
    s
}

/// `fill` (and a `classDef` stroke) attributes for a node outline.
fn paint_attrs(n: &MindNodeBox) -> String {
    let mut a = format!("fill=\"{}\"", escape(n.paint()));
    if let Some(stroke) = &n.style.stroke {
        a.push_str(&format!(
            " stroke=\"{}\" stroke-width=\"{:.1}\"",
            escape(stroke),
            n.style.stroke_width.unwrap_or(2.0)
        ));
    }
    a
}

/// SVG for a node's filled outline.
fn shape_svg(n: &MindNodeBox) -> String {
    let (x, y, w, h) = (n.x, n.y, n.w, n.h);
    let paint = paint_attrs(n);
    if let Some(pts) = perimeter(n) {
        let points = pts
            .iter()
            .map(|(px, py)| format!("{:.1},{:.1}", px, py))
            .collect::<Vec<_>>()
            .join(" ");
        return format!("<polygon points=\"{}\" {}/>\n", points, paint);
    }
    match n.shape {
        MindShape::Circle => format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" {}/>\n",
            n.cx(),
            n.cy(),
            w.min(h) / 2.0,
            paint
        ),
        MindShape::Default => {
            // Rounded top corners, flat bottom carrying the underline.
            let r = DEFAULT_RX.min(h / 2.0).min(w / 2.0);
            let mut out = format!(
                "<path d=\"M {:.1} {:.1} V {:.1} Q {:.1} {:.1} {:.1} {:.1} H {:.1} \
                 Q {:.1} {:.1} {:.1} {:.1} V {:.1} Z\" {}/>\n",
                x,
                y + h,
                y + r,
                x,
                y,
                x + r,
                y,
                x + w - r,
                x + w,
                y,
                x + w,
                y + r,
                y + h,
                paint
            );
            if let Some((ux, uy, uw, uh)) = underline(n) {
                out.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                     fill=\"#000000\" fill-opacity=\"{}\"/>\n",
                    ux, uy, uw, uh, UNDERLINE_SHADE
                ));
            }
            out
        }
        _ => {
            let rx = match n.shape {
                MindShape::Square => 0.0,
                // A pill: fully round ends.
                _ => h / 2.0,
            };
            format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                 rx=\"{:.1}\" {}/>\n",
                x, y, w, h, rx, paint
            )
        }
    }
}

/// SVG for a node's icon glyph, with the icon name as its tooltip.
fn icon_svg(n: &MindNodeBox, lines: &[Vec<(f64, f64)>]) -> String {
    let mut d = String::new();
    for line in lines {
        for (k, (x, y)) in line.iter().enumerate() {
            d.push_str(&format!(
                "{}{:.1} {:.1}",
                if k == 0 { "M" } else { " L" },
                x,
                y
            ));
            d.push(' ');
        }
    }
    format!(
        "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" \
         stroke-linecap=\"round\" stroke-linejoin=\"round\"><title>{}</title></path>\n",
        d.trim_end(),
        escape(n.ink()),
        escape(n.icon.as_deref().unwrap_or(""))
    )
}

/// SVG for a node's centred (possibly multi-line) label.
fn label_svg(n: &MindNodeBox) -> String {
    let lines: Vec<&str> = n.text.split('\n').collect();
    let mut attrs = String::new();
    if (n.font_size - FONT as f64).abs() > 1e-9 {
        attrs.push_str(&format!(" font-size=\"{}\"", n.font_size));
    }
    match n.font_weight {
        700 => attrs.push_str(" font-weight=\"bold\""),
        400 => {}
        w => attrs.push_str(&format!(" font-weight=\"{w}\"")),
    }
    let (cx, cy) = label_centre(n);
    let line_h = LINE_H * n.font_size / FONT as f64;
    let first_dy = 0.32 * n.font_size - (lines.len() as f64 - 1.0) * line_h / 2.0;
    let mut spans = String::new();
    for (i, line) in lines.iter().enumerate() {
        let dy = if i == 0 { first_dy } else { line_h };
        spans.push_str(&format!(
            "<tspan x=\"{:.1}\" dy=\"{:.1}\">{}</tspan>",
            cx,
            dy,
            escape(line)
        ));
    }
    format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"{}\"{}>{}</text>\n",
        cx,
        cy,
        escape(n.ink()),
        attrs,
        spans
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn mind(src: &str) -> Mindmap {
        match parse_document(src).unwrap() {
            Document::Mindmap(m) => m,
            _ => panic!("expected a mindmap"),
        }
    }

    fn showcase() -> Mindmap {
        mind(include_str!("../examples/mindmap.mmd"))
    }

    fn by_text<'a>(ms: &'a MindScene, text: &str) -> &'a MindNodeBox {
        ms.nodes.iter().find(|n| n.text == text).expect(text)
    }

    #[test]
    fn builds_tree_from_indentation() {
        let m = mind("mindmap\n  root((Root))\n    A\n      A1\n    B\n");
        assert_eq!(m.nodes.len(), 4);
        assert_eq!(m.nodes[0].text, "Root");
        assert_eq!(m.nodes[0].shape, MindShape::Circle);
        assert_eq!(m.nodes[0].children.len(), 2);
        let a = m.nodes[0].children[0];
        assert_eq!(m.nodes[m.nodes[a].children[0]].text, "A1");
    }

    #[test]
    fn shapes_and_br_parse() {
        let m = mind("mindmap\nroot\n  [sq]\n  (round)\n  ((circ))\n  {{hex}}\n  a<br/>b\n");
        let shapes: Vec<MindShape> = m.nodes[1..].iter().map(|n| n.shape).collect();
        assert_eq!(
            shapes,
            [
                MindShape::Square,
                MindShape::Rounded,
                MindShape::Circle,
                MindShape::Hexagon,
                // No wrapper: mermaid's default node, not a pill.
                MindShape::Default,
            ]
        );
        assert_eq!(m.nodes.last().unwrap().text, "a\nb");
    }

    #[test]
    fn multiword_text_before_paren_stays_literal() {
        // An id can't contain a space, so "call foo(x)" is plain text,
        // not a rounded node labeled "x".
        let m = mind("mindmap\nRoot\n  call foo(x)\n  id(real)\n");
        assert_eq!(m.nodes[1].text, "call foo(x)");
        assert_eq!(m.nodes[1].shape, MindShape::Default);
        // A genuine adjacent id still strips.
        assert_eq!(m.nodes[2].text, "real");
        assert_eq!(m.nodes[2].shape, MindShape::Rounded);
    }

    #[test]
    fn radial_layout_stays_inside_canvas() {
        let ms = scene(&mind(
            "mindmap\n  root((Order))\n    Customer\n      Buat Order\n      Lihat Order\n    \
             Owner\n      Kelola\n      State Machine\n    Sistem\n",
        ));
        assert!(ms.width > 0.0 && ms.height > 0.0);
        for n in &ms.nodes {
            assert!(n.x >= -0.01 && n.y >= -0.01, "node top-left inside");
            assert!(n.x + n.w <= ms.width + 0.01, "node right inside canvas");
            assert!(n.y + n.h <= ms.height + 0.01, "node bottom inside canvas");
        }
        assert_eq!(ms.edges.len(), ms.nodes.len() - 1);
    }

    #[test]
    fn root_sits_near_the_centre() {
        let ms = scene(&mind("mindmap\nroot((R))\n  A\n  B\n  C\n  D\n"));
        let root = &ms.nodes[0];
        // Children radiate around the root, so it lands near the middle.
        assert!((root.cx() - ms.width / 2.0).abs() < ms.width * 0.28);
        assert!((root.cy() - ms.height / 2.0).abs() < ms.height * 0.28);
    }

    fn assert_no_overlap(ms: &MindScene) {
        let boxes: Vec<(f64, f64, f64, f64)> = ms
            .nodes
            .iter()
            .map(|n| (n.x, n.y, n.x + n.w, n.y + n.h))
            .collect();
        for i in 0..boxes.len() {
            for j in (i + 1)..boxes.len() {
                let (a, b) = (boxes[i], boxes[j]);
                let overlap = a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3;
                assert!(
                    !overlap,
                    "{:?} ({a:?}) overlaps {:?} ({b:?})",
                    ms.nodes[i].text, ms.nodes[j].text
                );
            }
        }
    }

    /// Connector segments (centre to centre) never cross each other.
    fn assert_no_crossing(ms: &MindScene) {
        let segs: Vec<((f64, f64), (f64, f64))> = ms
            .edges
            .iter()
            .map(|e| (e.bezier[0], e.bezier[3]))
            .collect();
        for (i, a) in segs.iter().enumerate() {
            for b in &segs[i + 1..] {
                assert!(
                    !segments_cross(a.0, a.1, b.0, b.1),
                    "connectors {a:?} and {b:?} cross"
                );
            }
        }
    }

    #[test]
    fn wide_siblings_do_not_overlap() {
        // Many long labels round the root: every box keeps clear.
        let mut src = String::from("mindmap\n  root(Center)\n");
        for i in 0..12 {
            src.push_str(&format!("    A very long branch label number {i:02}\n"));
        }
        let ms = scene(&mind(&src));
        assert_no_overlap(&ms);
        assert_no_crossing(&ms);
    }

    #[test]
    fn to_svg_draws_shapes_and_links() {
        let svg = to_svg(&scene(&mind(
            "mindmap\nroot((R))\n  A\n  ))boom((\n  )fog(\n  {{hx}}\n",
        )));
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("<circle"), "circle root");
        assert!(svg.contains("<polygon"), "bang/cloud/hexagon polygons");
        assert!(svg.contains("<path"), "connectors");
        assert!(
            svg.contains("fill=\"#ffffff\""),
            "root and indigo text is white"
        );
    }

    #[test]
    fn route_follows_dragged_positions() {
        let m = mind("mindmap\nroot((R))\n  A\n  B\n");
        let auto = scene(&m);
        // Seed from auto, then "drag" node A far to the right.
        let mut pos: Vec<(f64, f64)> = auto.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
        pos[1] = (pos[1].0 + 500.0, pos[1].1);
        let routed = route(&m, &pos);
        // Node A's box moved with it; the root→A connector's end follows.
        assert!((routed.nodes[1].cx() - pos[1].0).abs() < 0.01);
        let a_edge = &routed.edges[0];
        assert!((a_edge.bezier[3].0 - pos[1].0).abs() < routed.nodes[1].w);
        // Canvas grew to keep the dragged node inside.
        assert!(routed.width > auto.width);
        // Re-routing the auto layout's own centres reproduces it exactly.
        let same: Vec<(f64, f64)> = auto.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
        assert_eq!(to_svg(&route(&m, &same)), to_svg(&auto));
    }

    #[test]
    fn bang_and_cloud_have_perimeters() {
        let ms = scene(&mind("mindmap\nR\n  ))b((\n  )c(\n"));
        let bang = ms
            .nodes
            .iter()
            .find(|n| n.shape == MindShape::Bang)
            .unwrap();
        let cloud = ms
            .nodes
            .iter()
            .find(|n| n.shape == MindShape::Cloud)
            .unwrap();
        assert!(perimeter(bang).unwrap().len() >= 12);
        assert!(perimeter(cloud).unwrap().len() >= 12);
        // A default box has no polygon perimeter.
        assert!(perimeter(&ms.nodes[0]).is_none());
    }

    /// Report Q5: branch nodes hard-coded white text on their accent
    /// (amber 2.6:1, green 3.2:1); the scene's per-node `text_color`
    /// (what host painters use) must agree with the SVG.
    #[test]
    fn branch_text_reads_on_every_accent() {
        let n = crate::style::ACCENTS.len();
        let mut src = String::from("mindmap\nroot((R))\n");
        for i in 0..n {
            src.push_str(&format!("  Br{i}\n    Leaf{i}\n"));
        }
        let ms = scene(&mind(&src));
        let svg = to_svg(&ms);
        for i in 0..n {
            let accent = crate::style::accent(i);
            for label in [format!("Br{i}"), format!("Leaf{i}")] {
                let ink = crate::style::assert_text_reads(&svg, &label, accent);
                assert_eq!(ink, crate::style::text_on(accent), "{label}");
                let node = ms.nodes.iter().find(|b| b.text == label).unwrap();
                assert_eq!((node.fill, node.text_color), (accent, ink));
            }
        }
    }

    /// Visible connector length: border to border along the line
    /// between the two centres.
    fn visible_spokes(ms: &MindScene) -> Vec<f64> {
        ms.edges
            .iter()
            .map(|e| {
                let (a, b) = (e.bezier[0], e.bezier[3]);
                let len = (b.0 - a.0).hypot(b.1 - a.1);
                let u = ((b.0 - a.0) / len, (b.1 - a.1) / len);
                let at = |p: (f64, f64)| {
                    ms.nodes
                        .iter()
                        .find(|n| (n.cx() - p.0).abs() < 1e-6 && (n.cy() - p.1).abs() < 1e-6)
                        .unwrap()
                };
                let (pa, pb) = (at(a), at(b));
                len - ext(pa.w, pa.h, u) - ext(pb.w, pb.h, u)
            })
            .collect()
    }

    /// Report B6 (sparse-oversized-canvas, uneven-long-leaf-spokes,
    /// leaf-fan-converges): fixed rings put every depth on one circle
    /// round the root — 9% ink on a 909×773 canvas, leaf spokes 55-223px.
    #[test]
    fn showcase_is_compact_with_even_spokes() {
        let ms = scene(&showcase());
        assert_no_overlap(&ms);
        assert_no_crossing(&ms);
        let spokes = visible_spokes(&ms);
        let (lo, hi) = spokes
            .iter()
            .fold((f64::MAX, 0.0f64), |(lo, hi), &s| (lo.min(s), hi.max(s)));
        assert!(lo >= SPOKE - 0.01, "a spoke shorter than SPOKE: {spokes:?}");
        assert!(hi / lo <= 2.0, "spokes {lo:.0}..{hi:.0}: {spokes:?}");
        // Ink: the node boxes alone cover a fifth of the canvas (the old
        // ring layout: under a tenth).
        let area: f64 = ms.nodes.iter().map(|n| n.w * n.h).sum();
        let fill = area / (ms.width * ms.height);
        assert!(fill >= 0.2, "boxes cover only {:.0}%", fill * 100.0);
        // Leaves hang off their parent on a short spoke, not off a ring
        // round the root (edges run in node order, root excluded).
        let m = showcase();
        let leaf_spokes: Vec<f64> = m.nodes[1..]
            .iter()
            .zip(&spokes)
            .filter(|(n, _)| n.depth >= 2)
            .map(|(_, &s)| s)
            .collect();
        assert!(
            leaf_spokes.iter().all(|&s| s < 2.0 * SPOKE),
            "{leaf_spokes:?}"
        );
    }

    #[test]
    fn branches_run_clockwise_from_twelve_in_source_order() {
        let m = showcase();
        let ms = scene(&m);
        let root = &ms.nodes[0];
        let angles: Vec<f64> = m.nodes[0]
            .children
            .iter()
            .map(|&c| {
                let a = (ms.nodes[c].cy() - root.cy()).atan2(ms.nodes[c].cx() - root.cx());
                // Measured clockwise from 12 o'clock, in [0, 2π).
                (a + FRAC_PI_2).rem_euclid(TAU)
            })
            .collect();
        assert!(angles.windows(2).all(|w| w[0] < w[1]), "{angles:?}");
        // And so do the leaves of each branch, round their parent.
        for &b in &m.nodes[0].children {
            let kids = &m.nodes[b].children;
            let base = (ms.nodes[b].cy() - root.cy()).atan2(ms.nodes[b].cx() - root.cx());
            let rel: Vec<f64> = kids
                .iter()
                .map(|&k| {
                    let a = (ms.nodes[k].cy() - ms.nodes[b].cy())
                        .atan2(ms.nodes[k].cx() - ms.nodes[b].cx());
                    wrap(a - base)
                })
                .collect();
            assert!(rel.windows(2).all(|w| w[0] < w[1]), "{rel:?}");
        }
    }

    /// Report: `root((flowmaid))` was a 1.6:1 ellipse.
    #[test]
    fn circle_is_a_true_circle_round_its_label() {
        let ms = scene(&mind(
            "mindmap\nroot((flowmaid))\n  ((a much longer circle))\n",
        ));
        for n in &ms.nodes {
            assert_eq!(n.shape, MindShape::Circle);
            assert!((n.w - n.h).abs() < 1e-9, "{} is {}x{}", n.text, n.w, n.h);
            // The label block's corners sit inside the circle.
            let (tw, th) = text_block(&n.text, n.depth);
            assert!(
                tw.hypot(th) <= n.w,
                "{}: label {tw}x{th} in d={}",
                n.text,
                n.w
            );
        }
        let svg = to_svg(&ms);
        assert!(
            svg.contains("<circle") && !svg.contains("<ellipse"),
            "{svg}"
        );
    }

    /// Report: undelimited nodes drew as the same pill as `(rounded)`.
    #[test]
    fn default_rounded_and_square_draw_differently() {
        let ms = scene(&mind("mindmap\nRoot\n  plain\n  (pill)\n  [box]\n"));
        let svg = to_svg(&ms);
        let plain = by_text(&ms, "plain");
        assert_eq!(plain.shape, MindShape::Default);
        let (ux, uy, uw, uh) = underline(plain).unwrap();
        assert_eq!((ux, uw, uh), (plain.x, plain.w, UNDERLINE_H));
        assert!(
            (uy + uh - (plain.y + plain.h)).abs() < 1e-9,
            "on the bottom edge"
        );
        assert!(
            svg.contains("fill-opacity=\"0.24\""),
            "underline drawn: {svg}"
        );
        let pill = by_text(&ms, "pill");
        assert!(underline(pill).is_none());
        assert!(
            svg.contains(&format!("rx=\"{:.1}\"", pill.h / 2.0)),
            "a pill's ends are fully round"
        );
        assert!(svg.contains("rx=\"0.0\""), "a square has sharp corners");
    }

    /// Fraction of a closed polygon's vertices where it turns the
    /// "wrong" way (concave notches).
    fn concave_share(pts: &[(f64, f64)]) -> f64 {
        let n = pts.len();
        let turn = |i: usize| {
            let (a, b, c) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0)
        };
        let total: f64 = (0..n).map(turn).sum();
        let concave = (0..n).filter(|&i| turn(i) * total < 0.0).count();
        concave as f64 / n as f64
    }

    /// Report: the cloud was a jagged |sin| polygon that read as a
    /// second explosion, and the bang's spikes were 4× longer at the
    /// sides than on top.
    #[test]
    fn cloud_is_puffy_and_bang_spikes_are_even() {
        let ms = scene(&mind(
            "mindmap\nR\n  )Eleven diagram types(\n  ))Zero dependencies((\n",
        ));
        let cloud = by_text(&ms, "Eleven diagram types");
        let bang = by_text(&ms, "Zero dependencies");
        let (cp, bp) = (perimeter(cloud).unwrap(), perimeter(bang).unwrap());
        // A cloud is round lobes: only the valleys between lobes turn
        // inward. A bang notches at every other vertex.
        assert!(concave_share(&cp) < 0.2, "cloud {:.2}", concave_share(&cp));
        assert!(concave_share(&bp) > 0.4, "bang {:.2}", concave_share(&bp));
        // Valleys lie on the core stadium; every tip stands exactly
        // BANG_SPIKE off it — at the ends as much as on top.
        let (tw, th) = text_block(&bang.text, bang.depth);
        let (l, r) = bang_core(tw, th);
        let off_core = |x: f64, y: f64| {
            let (dx, dy) = (x - bang.cx(), y - bang.cy());
            (dx.abs() - l).max(0.0).hypot(dy) - r
        };
        for (j, &(x, y)) in bp.iter().enumerate() {
            let want = if j % 2 == 1 { 0.0 } else { BANG_SPIKE };
            assert!(
                (off_core(x, y) - want).abs() < 1e-6,
                "point {j}: {}",
                off_core(x, y)
            );
        }
        // The label's corners sit inside the core.
        assert!(off_core(bang.cx() + tw / 2.0, bang.cy() + th / 2.0) < 0.0);
        // The cloud keeps its label inside the ellipse its lobes ride on.
        let (tw, th) = text_block(&cloud.text, cloud.depth);
        let (ex, ey) = label_ellipse(tw, th);
        assert!((cloud.w / 2.0 - CLOUD_LOBE - ex).abs() < 1e-9);
        assert!((cloud.h / 2.0 - CLOUD_LOBE - ey).abs() < 1e-9);
        assert!((tw / 2.0 / ex).powi(2) + (th / 2.0 / ey).powi(2) < 1.0);
    }

    /// Report: the root was a pale #c9cfe0 ellipse (1.56:1 on white),
    /// the faintest thing on the canvas; trunks barely tapered (4.5 vs
    /// 3.5px).
    #[test]
    fn root_is_the_strongest_node_and_trunks_taper() {
        let ms = scene(&showcase());
        let root = &ms.nodes[0];
        assert_eq!(root.fill, ROOT_FILL);
        let c = crate::style::contrast_ratio(ROOT_FILL, "#ffffff").unwrap();
        assert!(c >= 7.0, "root fill only {c:.1}:1 on the white canvas");
        assert_eq!(root.text_color, "#ffffff");
        assert_eq!((root.font_size, root.font_weight), (ROOT_FONT, 700));
        let svg = to_svg(&ms);
        assert!(
            svg.contains("font-size=\"18\" font-weight=\"bold\""),
            "{svg}"
        );
        let trunk = ms.edges.iter().map(|e| e.width).fold(0.0, f64::max);
        let leaf = ms.edges.iter().map(|e| e.width).fold(f64::MAX, f64::min);
        assert!(trunk >= 2.0 * leaf, "trunk {trunk} vs leaf {leaf}");
        assert_eq!(link_width(1), 9.0);
    }

    /// Report: `::icon(…)` and `:::class` lines were dropped silently.
    #[test]
    fn icons_are_kept_and_drawn() {
        let m = mind("mindmap\nRoot\n  Docs\n  ::icon(fa fa-book)\n  Other\n    ::icon(mdi mdi-unheard-of)\n");
        assert_eq!(m.nodes[1].icon.as_deref(), Some("fa fa-book"));
        assert_eq!(m.nodes[2].icon.as_deref(), Some("mdi mdi-unheard-of"));
        assert_eq!(m.nodes.len(), 3, "decorations are not nodes");
        let ms = scene(&m);
        let plain = scene(&mind("mindmap\nRoot\n  Docs\n  Other\n"));
        let (with, without) = (by_text(&ms, "Docs"), by_text(&plain, "Docs"));
        assert!(
            (with.w - without.w - (ICON + ICON_GAP)).abs() < 1e-9,
            "room for it"
        );
        // The glyph sits left of the text, inside the box.
        let strokes = icon_strokes(with).unwrap();
        let (tw, _) = text_block("Docs", with.depth);
        let text_left = label_centre(with).0 - tw / 2.0;
        for &(x, y) in strokes.iter().flatten() {
            assert!(x >= with.x && x <= text_left, "{x} outside the icon slot");
            assert!(y >= with.y && y <= with.y + with.h);
        }
        let svg = to_svg(&ms);
        assert!(svg.contains("<title>fa fa-book</title>"), "{svg}");
        assert!(
            svg.contains("<title>mdi mdi-unheard-of</title>"),
            "unknown names still draw"
        );
        assert!(icon_strokes(by_text(&ms, "Root")).is_none());
    }

    #[test]
    fn classes_reach_the_svg_and_class_defs_colour_nodes() {
        let m = mind(
            "mindmap\nRoot\n  Hot\n  :::urgent large\n  Calm\n  :::quiet\n  \
             classDef in a sentence\nclassDef urgent fill:#ffe066,stroke:#c92a2a,stroke-width:3px\n\
             classDef quiet fill:#495057,color:#ffd43b\n",
        );
        let hot = &m.nodes[1];
        assert_eq!(hot.classes, ["urgent", "large"]);
        assert_eq!(hot.style.fill.as_deref(), Some("#ffe066"));
        // A line that only starts with the word stays a node.
        assert!(m.nodes.iter().any(|n| n.text == "classDef in a sentence"));
        let ms = scene(&m);
        let hot = by_text(&ms, "Hot");
        assert_eq!(hot.paint(), "#ffe066");
        // Text follows the custom fill's contrast, not the branch's.
        assert_eq!(hot.ink(), crate::style::text_on("#ffe066"));
        let calm = by_text(&ms, "Calm");
        assert_eq!((calm.paint(), calm.ink()), ("#495057", "#ffd43b"));
        let svg = to_svg(&ms);
        assert!(svg.contains("<g class=\"urgent large\">"), "{svg}");
        assert!(
            svg.contains("fill=\"#ffe066\" stroke=\"#c92a2a\" stroke-width=\"3.0\""),
            "{svg}"
        );
        assert!(svg.contains("fill=\"#ffd43b\""), "{svg}");
        // The branch colour is still what the connector uses.
        assert!(ms.edges.iter().all(|e| !e.color.starts_with("#ffe")));
    }

    /// Review: `classDef hot fill:yellow` got white text (1.07:1) — the
    /// contrast pick could only read hex, so a named fill fell back to
    /// white; `#ffff00` correctly got dark ink.
    #[test]
    fn a_named_class_fill_still_gets_a_readable_ink() {
        for fill in ["yellow", "Yellow", "rgb(255, 255, 0)", "#ff0"] {
            let m = mind(&format!(
                "mindmap\nRoot\n  Hot\n  :::hot\nclassDef hot fill:{fill}\n"
            ));
            let ms = scene(&m);
            let hot = by_text(&ms, "Hot");
            assert_eq!(hot.ink(), crate::style::text_on("#ffff00"), "{fill}");
            crate::style::assert_text_reads(&to_svg(&ms), "Hot", "#ffff00");
        }
        // A dark named fill keeps white.
        let m = mind("mindmap\nRoot\n  Hot\n  :::hot\nclassDef hot fill:navy\n");
        assert_eq!(by_text(&scene(&m), "Hot").ink(), "#ffffff");
    }

    #[test]
    fn class_names_and_colours_are_escaped() {
        let m = mind("mindmap\nRoot\n  A\n  :::x\"y\nclassDef x\"y fill:red\" onload=z\n");
        let svg = to_svg(&scene(&m));
        assert!(!svg.contains("\" onload=z"), "{svg}");
        assert!(!svg.contains("class=\"x\"y\""), "{svg}");
    }

    #[test]
    fn decorations_before_any_node_are_ignored() {
        let m = mind("mindmap\n::icon(fa fa-x)\n:::cls\nRoot\n");
        assert_eq!(m.nodes.len(), 1);
        assert!(m.nodes[0].icon.is_none() && m.nodes[0].classes.is_empty());
    }

    /// A deterministic spread of trees: whatever layout `scene` lands
    /// on, boxes never overlap and connectors never cross.
    #[test]
    fn random_trees_stay_clean() {
        let shapes = ["", "(", "[", "((", "{{", "))", ")"];
        let closes = ["", ")", "]", "))", "}}", "((", "("];
        for seed in 0..40u64 {
            let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
            let mut next = |m: u64| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x % m
            };
            let mut src = String::from("mindmap\n  Root topic\n");
            let mut depth = 1;
            for i in 0..(5 + next(30)) {
                depth = (depth as i64 + next(3) as i64 - 1).clamp(1, 4) as usize;
                let s = next(shapes.len() as u64) as usize;
                let words = ["node", "a longer label", "x", "multi<br/>line"][next(4) as usize];
                src.push_str(&format!(
                    "{}{}{words} {i}{}\n",
                    "  ".repeat(depth + 1),
                    shapes[s],
                    closes[s]
                ));
            }
            let ms = scene(&mind(&src));
            assert_no_overlap(&ms);
            assert_no_crossing(&ms);
        }
    }

    #[test]
    fn ring_fallback_is_clean_too() {
        // The fallback must stay a valid layout on its own.
        let m = showcase();
        let sizes: Vec<(f64, f64)> = m.nodes.iter().map(mind_node_size).collect();
        let pos = ring_centres(&m, &sizes);
        assert!(clean(&m, &sizes, &pos));
    }

    #[test]
    fn a_deep_bushy_tree_still_lays_out() {
        // Three children per node, six levels: the fan budget may run
        // out, in which case the ring layout takes over — either way
        // the scene is complete and clean.
        let mut src = String::from("mindmap\nR\n");
        fn grow(src: &mut String, depth: usize, max: usize) {
            if depth > max {
                return;
            }
            for k in 0..3 {
                src.push_str(&format!("{}n{depth}{k}\n", " ".repeat(depth * 2)));
                grow(src, depth + 1, max);
            }
        }
        grow(&mut src, 1, 6);
        let ms = scene(&mind(&src));
        assert_eq!(ms.nodes.len(), 1 + 3 + 9 + 27 + 81 + 243 + 729);
        assert_no_overlap(&ms);
    }

    #[test]
    fn showcase_and_a_big_crowded_map_use_the_fan_layout() {
        let m = showcase();
        let sizes: Vec<(f64, f64)> = m.nodes.iter().map(mind_node_size).collect();
        assert!(
            fan_centres(&m, &sizes).is_some(),
            "showcase fell back to rings"
        );
        // Ten branches of four topics of three leaves: the root needs a
        // long trunk, found by the estimate rather than ×1.25 steps from
        // the base length (which ran out of attempts).
        let mut src = String::from("mindmap\n  root((Big map))\n");
        for b in 0..10 {
            src.push_str(&format!("    Branch number {b}\n"));
            for c in 0..4 {
                src.push_str(&format!("      Topic {b}.{c}\n"));
                for l in 0..3 {
                    src.push_str(&format!("        leaf item {b}.{c}.{l}\n"));
                }
            }
        }
        let m = mind(&src);
        let sizes: Vec<(f64, f64)> = m.nodes.iter().map(mind_node_size).collect();
        assert!(
            fan_centres(&m, &sizes).is_some(),
            "big map fell back to rings"
        );
        let ms = scene(&m);
        assert_no_overlap(&ms);
        assert_no_crossing(&ms);
        // Leaves still cluster on short spokes round their topic.
        let spokes = visible_spokes(&ms);
        let leaves = m.nodes[1..]
            .iter()
            .zip(&spokes)
            .filter(|(n, _)| n.depth == 3);
        for (n, &s) in leaves {
            assert!(s < 3.0 * SPOKE, "{}: spoke {s:.0}", n.text);
        }
    }

    #[test]
    fn svg_is_deterministic() {
        let m = showcase();
        assert_eq!(to_svg(&scene(&m)), to_svg(&scene(&m)));
    }

    /// Nine branches of very different sizes: a leaf, a one-leaf and a
    /// two-leaf branch next to an eight- and a twelve-leaf one, a deep
    /// chain and long labels.
    const BUSHY: &str = "mindmap\n  Root\n    Alpha\n      A one\n      A two\n      A three\n      \
        A four\n      A five\n      A six\n      A seven\n      A eight\n    Beta\n      B one\n        \
        B one a\n        B one b\n        B one c\n      B two\n        B two a\n        B two b\n      \
        B three\n    Gamma\n      G one\n      G two\n        G two a\n          G two a i\n          \
        G two a ii\n            deep 1\n              deep 2\n                deep 3\n      G three\n    \
        Delta with a rather long branch label here\n      D1 with long text describing things\n      \
        D2 with even longer text describing more things\n      D3\n    Epsilon\n    Zeta\n      Z1\n      \
        Z2\n    Eta\n      E1\n    Theta\n      T1\n      T2\n      T3\n      T4\n      T5\n      T6\n      \
        T7\n      T8\n      T9\n      T10\n      T11\n      T12\n";

    /// `branches` first-level topics of `leaves` leaves each.
    fn grid(branches: usize, leaves: usize) -> Mindmap {
        let mut src = String::from("mindmap\n  root((Project))\n");
        for b in 0..branches {
            src.push_str(&format!("    Branch {b} topic\n"));
            for l in 0..leaves {
                src.push_str(&format!("      Item {b}.{l}\n"));
            }
        }
        mind(&src)
    }

    /// Widest wedge (degrees) round the root with no node box in it.
    fn empty_wedge(ms: &MindScene) -> f64 {
        let (cx, cy) = (ms.nodes[0].cx(), ms.nodes[0].cy());
        let mut arcs: Vec<(f64, f64)> = Vec::new();
        for n in &ms.nodes[1..] {
            let mid = (n.cy() - cy).atan2(n.cx() - cx);
            let rel = [
                (n.x, n.y),
                (n.x + n.w, n.y),
                (n.x, n.y + n.h),
                (n.x + n.w, n.y + n.h),
            ]
            .map(|(x, y)| wrap((y - cy).atan2(x - cx) - mid));
            let lo = rel.iter().copied().fold(f64::MAX, f64::min);
            let hi = rel.iter().copied().fold(f64::MIN, f64::max);
            let from = (mid + lo).rem_euclid(TAU);
            // Twice round, so an arc across angle 0 is seen whole.
            arcs.push((from, from + hi - lo));
            arcs.push((from + TAU, from + TAU + hi - lo));
        }
        arcs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mut widest, mut reach) = (0.0f64, arcs[0].1);
        for &(lo, hi) in &arcs[1..] {
            if lo > reach && lo < 2.0 * TAU {
                widest = widest.max(lo - reach);
            }
            reach = reach.max(hi);
        }
        widest.to_degrees()
    }

    /// Visible length of the connector into each node (index-parallel
    /// with the nodes; the root gets 0).
    fn spoke_into(m: &Mindmap, ms: &MindScene) -> Vec<f64> {
        let mut out = vec![0.0; m.nodes.len()];
        for (i, n) in m.nodes.iter().enumerate() {
            if let Some(p) = n.parent {
                let (a, b) = (&ms.nodes[p], &ms.nodes[i]);
                let (dx, dy) = (b.cx() - a.cx(), b.cy() - a.cy());
                let len = dx.hypot(dy);
                let u = (dx / len, dy / len);
                out[i] = len - ext(a.w, a.h, u) - ext(b.w, b.h, u);
            }
        }
        out
    }

    /// Review of B6: on ordinary maps the root fan stretched EVERY trunk
    /// to the one length its bushiest branch needed (365px on this map,
    /// a leaf-only branch included) and left all the spare arc in one
    /// empty sector (183° here), the root at a quarter of the width.
    #[test]
    fn a_mixed_map_shares_the_circle_and_sizes_each_trunk() {
        let m = mind(BUSHY);
        let ms = scene(&m);
        assert_no_overlap(&ms);
        assert_no_crossing(&ms);
        let wedge = empty_wedge(&ms);
        assert!(
            wedge < 45.0,
            "an empty sector of {wedge:.0}° round the root"
        );
        let spokes = spoke_into(&m, &ms);
        let trunk = |name: &str| spokes[m.nodes.iter().position(|n| n.text == name).unwrap()];
        // Each branch gets its own trunk: the small ones sit closer in
        // than the bushy ones (they all had 365px).
        let longest = m.nodes[0]
            .children
            .iter()
            .map(|&c| spokes[c])
            .fold(0.0, f64::max);
        assert!(
            trunk("Epsilon") < 2.0 * TRUNK,
            "Epsilon: trunk {:.0}",
            trunk("Epsilon")
        );
        for small in ["Eta", "Zeta"] {
            assert!(
                trunk(small) < 0.9 * longest,
                "{small}: trunk {:.0} of {longest:.0}",
                trunk(small)
            );
        }
        // And the shared length was searched back down, not left at the
        // first one that fitted.
        assert!(longest < 260.0, "longest trunk {longest:.0}");
        let root = &ms.nodes[0];
        assert!(
            (0.25..0.75).contains(&(root.cy() / ms.height)),
            "root at y {:.0}/{:.0}",
            root.cy(),
            ms.height
        );
    }

    /// The same review on uniform maps: 6-12 branches came out with one
    /// long trunk each (145-374px) and a 103-145° empty sector.
    #[test]
    fn uniform_maps_fill_the_circle_evenly() {
        for (b, l, longest) in [
            (5, 4, 2.0 * TRUNK),
            (8, 4, 220.0),
            (10, 3, 220.0),
            (12, 2, 200.0),
        ] {
            let m = grid(b, l);
            let ms = scene(&m);
            assert_no_overlap(&ms);
            assert_no_crossing(&ms);
            let wedge = empty_wedge(&ms);
            assert!(wedge < 30.0, "{b}x{l}: empty sector {wedge:.0}°");
            let spokes = spoke_into(&m, &ms);
            let trunk = m.nodes[0]
                .children
                .iter()
                .map(|&c| spokes[c])
                .fold(0.0, f64::max);
            assert!(trunk < longest, "{b}x{l}: longest trunk {trunk:.0}");
            let root = &ms.nodes[0];
            assert!(
                (root.cx() / ms.width - 0.5).abs() < 0.12,
                "{b}x{l}: root off-centre"
            );
            assert!(
                (root.cy() / ms.height - 0.5).abs() < 0.12,
                "{b}x{l}: root off-centre"
            );
        }
        // B6's "spokes even" holds while the circle has room: five
        // branches of four leaves keep every connector within 2× of the
        // shortest.
        let spokes = visible_spokes(&scene(&grid(5, 4)));
        let (lo, hi) = spokes
            .iter()
            .fold((f64::MAX, 0.0f64), |(lo, hi), &s| (lo.min(s), hi.max(s)));
        assert!(hi / lo <= 2.05, "5x4 spokes {lo:.0}..{hi:.0}");
    }

    /// 24 topics of three leaves each: the old shared trunk was 1058px.
    /// Branches now alternate in and out and their fans are kept narrow
    /// when that shortens the longest connector.
    #[test]
    fn a_crowded_hub_staggers_its_branches() {
        let m = grid(24, 3);
        let sizes: Vec<(f64, f64)> = m.nodes.iter().map(mind_node_size).collect();
        let pos = fan_centres(&m, &sizes).expect("fell back to rings");
        let (longest, _) = longest_connector(&m, &sizes, &pos);
        assert!(longest < 700.0, "longest connector {longest:.0}");
        let ms = scene(&m);
        assert_no_overlap(&ms);
        assert_no_crossing(&ms);
        assert!(empty_wedge(&ms) < 30.0);
    }

    /// Review: a chain spent 119-192ms before falling back to rings and
    /// came out as a 2255x6675 diagonal. A lone child carries straight
    /// on, and the root's lone branch goes right.
    #[test]
    fn a_chain_runs_straight_to_the_right() {
        let mut src = String::from("mindmap\nroot\n");
        for k in 1..200 {
            src.push_str(&format!("{}n{k}\n", " ".repeat(k)));
        }
        let m = mind(&src);
        let sizes: Vec<(f64, f64)> = m.nodes.iter().map(mind_node_size).collect();
        assert!(fan_centres(&m, &sizes).is_some(), "fell back to rings");
        let ms = scene(&m);
        let y = ms.nodes[0].cy();
        assert!(ms.nodes.iter().all(|n| (n.cy() - y).abs() < 1e-6));
        assert!(ms.nodes.windows(2).all(|w| w[1].cx() > w[0].cx()));
    }

    /// A comb (two leaves, then the child that goes on, at every level)
    /// bent the same way at every level until it curled into itself: at
    /// 16 levels it fell back to rings after 54ms. A fan now aims its
    /// heaviest child straight on.
    #[test]
    fn a_comb_still_gets_the_fan_layout() {
        let mut src = String::from("mindmap\nroot\n");
        for k in 1..16 {
            let pad = " ".repeat(k);
            src.push_str(&format!(
                "{pad}spine {k}\n{pad} leaf {k}.0\n{pad} leaf {k}.1\n"
            ));
        }
        let m = mind(&src);
        let sizes: Vec<(f64, f64)> = m.nodes.iter().map(mind_node_size).collect();
        assert!(fan_centres(&m, &sizes).is_some(), "fell back to rings");
    }

    /// Two subtrees whose boxes are far apart can still collide: a spoke
    /// cutting through the other's hull, between its boxes, across one
    /// of its connectors (found on a generated 107-node map, which then
    /// failed the final check and fell back to rings).
    #[test]
    fn a_spoke_may_not_cut_a_far_subtree_between_its_boxes() {
        let m = mind("mindmap\nR\n  A\n  B\n    B1\n");
        let sizes: Vec<(f64, f64)> = m.nodes.iter().map(mind_node_size).collect();
        let mut fan = Fan::new(&m, &sizes).unwrap();
        let lone = (-10.0, -10.0, 10.0, 10.0);
        fan.entries.push(Entry {
            shape: Shape {
                boxes: vec![lone],
                segs: Vec::new(),
                bbox: lone,
            },
            pos: Vec::new(),
        });
        // B at (100, 0) with B1 200px below it: A's spoke, from the
        // origin to (200, 100), crosses B's connector at (100, 50),
        // clear of both boxes.
        fan.entries.push(Entry {
            shape: Shape {
                boxes: vec![lone, (-10.0, 190.0, 10.0, 210.0)],
                segs: vec![((0.0, 0.0), (0.0, 200.0))],
                bbox: (-10.0, -10.0, 10.0, 210.0),
            },
            pos: vec![(0.0, 200.0)],
        });
        let n = fan.entries.len();
        let a = At {
            e: n - 2,
            o: (200.0, 100.0),
        };
        let b = At {
            e: n - 1,
            o: (100.0, 0.0),
        };
        assert!(!fan.siblings_clear(a, b));
        assert!(!fan.siblings_clear(b, a));
    }

    /// A spread of realistic maps (4-10 branches, up to seven topics of
    /// up to five leaves, the odd fourth level) all get the fan layout:
    /// none fails the final check.
    #[test]
    fn realistic_maps_use_the_fan_layout() {
        let words = [
            "plan", "design", "research", "budget", "launch", "review", "docs", "sales",
        ];
        for seed in 0..12u64 {
            let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
            let mut next = |m: u64| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x % m
            };
            let mut src = String::from("mindmap\n  root((Central idea))\n");
            let label = |next: &mut dyn FnMut(u64) -> u64| {
                (0..1 + next(3))
                    .map(|_| words[next(words.len() as u64) as usize])
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            for b in 0..4 + next(5) {
                src.push_str(&format!("    {} {b}\n", label(&mut next)));
                for _ in 0..1 + next(5) {
                    src.push_str(&format!("      {}\n", label(&mut next)));
                    for _ in 0..next(5) {
                        src.push_str(&format!("        {}\n", label(&mut next)));
                        if next(8) == 0 {
                            src.push_str(&format!("          {}\n", label(&mut next)));
                        }
                    }
                }
            }
            let m = mind(&src);
            let sizes: Vec<(f64, f64)> = m.nodes.iter().map(mind_node_size).collect();
            assert!(
                fan_centres(&m, &sizes).is_some(),
                "seed {seed} fell back to rings:\n{src}"
            );
        }
    }
}
