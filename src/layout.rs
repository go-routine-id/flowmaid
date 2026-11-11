//! Sugiyama layout engine, following dagre (the algorithm mermaid.js
//! uses):
//!
//! 1. Detect back-edges via DFS so cycles don't break layering.
//! 2. Assign layers with longest-path (topological, Kahn-style).
//! 3. Split long edges into per-layer dummy chains (routing channels);
//!    a labelled long edge sizes its middle dummy to the label.
//! 4. Order within layers to cut crossings: weighted-median sweeps plus
//!    a local adjacent-swap transpose, keeping the fewest-crossing round.
//!    Subgraph members are held contiguous, bracketed by border walls.
//! 5. Assign coordinates with Brandes-Köpf: four vertical alignments
//!    blended by the per-node median (straight long edges, centred fans).
//!
//! Everything is computed in abstract coordinates (b = breadth,
//! l = layer / depth); the renderer maps them to final x,y
//! according to the diagram direction (TD/LR/BT/RL).

use crate::clearance::{FRAME_SEP, NODE_SEP, TIGHT_RUN_MIN};
use crate::labels::{FAN_FAR, FAN_SIBLING_GAP, LABEL_SIDE_AIR, LABEL_STRADDLE, STROKE_PAD};
use crate::levels::{BlockLinks, Tree, Unit as LevelUnit};
use crate::model::{Direction, EdgeKind, Graph, Node, Shape};
use crate::ports::PORT_INSET;
use crate::scene::{port_lean, ARROW_MARKER_PX};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::VecDeque;

/// Position & size of one node in abstract coordinates.
pub struct Placed {
    /// Centre point on the breadth axis.
    pub b: f64,
    /// Centre point on the layer axis.
    pub l: f64,
    /// Node size along the breadth axis.
    pub bsize: f64,
    /// Node size along the layer axis.
    pub lsize: f64,
    /// Layer index.
    pub layer: usize,
}

pub struct LayoutResult {
    pub nodes: Vec<Placed>,
    pub total_b: f64,
    pub total_l: f64,
    /// Per original edge: the abstract `(b, l)` waypoints its virtual-
    /// node chain passes through (empty for adjacent-layer edges). A
    /// renderer can spline through these so a long edge routes in the
    /// channel between layers instead of straight across the nodes.
    pub edge_paths: Vec<Vec<(f64, f64)>>,
}

/// Horizontal padding inside a node box, each side (issue #14: public
/// so embedders measuring text themselves can reproduce node sizes).
pub const PAD_X: f64 = 16.0;
/// Base node height for a single-line label.
pub const BASE_H: f64 = 38.0;
/// Minimum node width.
pub const MIN_W: f64 = 54.0;
/// Side of an unlabelled diamond — a stateDiagram `<<choice>>` (mermaid
/// draws it 28×28). The decision-diamond minimum exists to fit text in
/// the middle; with no text it would only inflate an empty branch point
/// (80×64.6) and push its outgoing ports apart.
pub const CHOICE_SIZE: f64 = 28.0;
/// Intrinsic (width, height) of a fork/join bar, before
/// `scene::fit_bars` lays it across the flow and stretches it over its
/// branches.
pub(crate) const BAR_SIZE: (f64, f64) = (60.0, 8.0);

/// Is a node of this shape and label a compact branch point: an
/// unlabelled diamond (a state `<<choice>>`, a flowchart `A{ }`), drawn
/// [`CHOICE_SIZE`] square?
pub(crate) fn is_branch_point(shape: Shape, label: &str) -> bool {
    shape == Shape::Diamond && label.trim().is_empty()
}
// Mermaid's dagre defaults (nodeSpacing / rankSpacing ≈ 50) — labels
// no longer ride between ranks (they own dummy slots), so the extra
// breathing room the old 62/84 reserved is dead space now.
pub(crate) const GAP_B: f64 = 50.0; // gap between REAL nodes within a layer (fork bars too)
pub(crate) const GAP_L: f64 = 60.0; // gap between layers (fold DP glue too)
const MARGIN: f64 = 28.0;
/// Gap contribution of an edge DUMMY within a layer (dagre's
/// `edgesep`): parallel long edges bundle into tight lanes instead of
/// spreading a full node-gap apart — key to mermaid's flowing look.
const EDGE_GAP: f64 = 18.0;
/// Extra half-gap added per subgraph boundary crossed between two
/// adjacent nodes in a layer — reserves each cluster box its padding.
const CLUSTER_PAD: f64 = 18.0;
/// Gap between a cluster's invisible border wall and its members —
/// small, since the wall only pins the band, not the visible box.
const BORDER_GAP: f64 = 4.0;
/// Rank room reserved for a cluster's title strip (mirrors the
/// scene-side `SUB_HEADER` box headroom).
pub(crate) const CLUSTER_HEADER: f64 = 26.0;
/// Padding a drawn cluster frame keeps around its members, per nesting
/// level (the scene-side `SUB_PAD`).
pub(crate) const FRAME_PAD: f64 = 14.0;
/// Least room between two frames stacked along the layer axis (and
/// between a lane-widened frame and its sibling, see `scene_clustered`).
pub(crate) const FRAME_GAP: f64 = 8.0;
/// Breadth-to-depth ratio a narrowed drawing aims for: narrowing rounds
/// stop once it is reached (G12).
const ASPECT_TARGET: f64 = 2.5;
/// Breadth-to-depth ratio past which a drawing counts as a ribbon and
/// is narrowed at all: the ratio the rounds aim for. It was 4 : 1, for
/// the longer edges and channel detours of a stacked or staggered
/// drawing outweighed the breadth saved on a moderately wide one; a
/// step now must give back breadth without growing area, edge length,
/// crossings or node hits past their slack (report M11), so a step that
/// costs more than it gives is refused at any ratio. Over the 164 of
/// some 1 800 generated and corpus flowcharts that narrowing between
/// 2.5 : 1 and 4 : 1 changes, it cuts their defects by a fifth (static)
/// to a quarter (`route()`) and their area by a tenth, crossings about
/// even (B8 review r1 #1); the corpus and fixtures are unchanged.
const RIBBON_ENTRY: f64 = ASPECT_TARGET;
/// Share of drawing area (breadth x depth, as the search judges them) a
/// narrowing step may add (report M11): narrowing is meant to fill the
/// drawing, not to grow it. fanout-frames' third frame stacked under
/// its first, beside the column of frames the hub feeds, grew it 9.7 %
/// (and pushed the frame's edge into the gap between two others); the
/// column now stays one column, as mermaid draws it.
const NARROW_AREA_SLACK: f64 = 0.02;
/// Share of total edge length (node centre to centre, both axes) a
/// narrowing step may add (report M11).
const NARROW_LEN_SLACK: f64 = 0.1;
/// Share of edge length along the flow a sibling stacking or wrap may
/// add (report M11): a box stacked under its sibling lengthens every
/// edge into it by the sibling's depth, a detour past it. Folding
/// fanout-frames' column of frames into two cut its area 12 % and its
/// total edge length 25 % (the fan's spread shrank) but grew the length
/// along the flow 60 %: every edge into the second column threading the
/// first. (A staggered leaf only steps one layer out beside its row:
/// leaf staggering is not held to it.) Folds rarely pass it — 3 of some
/// 1 800 generated and corpus flowcharts fold, a TB row of shallow
/// frames and leaves under one hub — for a later run's edges always
/// thread past the earlier run (B8 review r1 #5); trading the slack for
/// an exchange rate instead (each px of flow detour buying 1.7 px of
/// breadth) folded 86 generated drawings, and raised their drawn
/// crossings 20 to 30 %.
const NARROW_FLOW_SLACK: f64 = 0.15;
/// Fewest real nodes a layer needs before its leaves are staggered.
const STAGGER_MIN_NODES: usize = 6;
/// Most staggering proposals laid out per diagram (each one is a full
/// ordering and coordinate pass).
const STAGGER_MAX_ROUNDS: usize = 4;
/// Share of the breadth a stagger must save to be kept.
const STAGGER_MIN_GAIN: f64 = 0.03;
/// Share of extra crossings a narrowing round (stack or stagger) may
/// add: breadth bought with a few crossings in a dense drawing is
/// still a net win, a doubling of them is not.
const NARROW_CROSS_SLACK: f64 = 0.1;
/// Gauss-Seidel sweeps of the post-BK breadth compaction.
const COMPACT_SWEEPS: usize = 24;
/// Pull of a link between two edge dummies in that compaction (keeps
/// long channels straight; Gansner's omega).
const COMPACT_DUMMY_WEIGHT: f64 = 8.0;
/// Pull of a link between a real node and an edge dummy.
const COMPACT_MIXED_WEIGHT: f64 = 2.0;
/// Extra edge-through-node skeleton hits ([`node_hits`]) a narrowing
/// step may add, and all steps together over the packed drawing they
/// start from ([`compact_breadth`]): a little
/// re-threading is the price of a new layering, a pile of edges cut
/// across boxes is not.
const NARROW_HIT_SLACK: usize = 2;
/// Largest graph (nodes + edges) whose cycle breakers may come from the
/// walk in target declaration order ([`Walk::Fewest`]). On hand-written
/// diagrams that walk, where it reverses fewer edges, takes the
/// layering off the order the edge lines happen to be written in (B8
/// review r1 #3): of the 28 such flowcharts and state diagrams under
/// 80 lines in a 5 337-input set, static defects fall 133 -> 36 and
/// `route()` ones 132 -> 38. On large random graphs its shallower
/// layering reads as a ribbon and is packed tight: syn200 (518
/// elements) went 10 368 -> 3 726 px broad, its `route()` edges
/// through nodes 24 -> 71 and defects 259 -> 596; syn400 (beyond the
/// search budget) 1 396 -> 1 739 `route()` defects (B8b P2). Both now
/// keep the walk in source order, as before 0.35.
const WALK_MAX_ELEMENTS: usize = 400;
/// Crossings one edge-through-node skeleton hit ([`node_hits`]) weighs
/// when a narrowing step adds hits past NARROW_HIT_SLACK: the step must
/// save more crossings than this many per hit it adds. A line struck
/// through a box reads worse than a crossing, and `route()` has to
/// thread it out again (B8b P2).
const NARROW_HIT_COST: usize = 2;
/// Most sibling-stacking rounds per diagram.
const STACK_MAX_ROUNDS: usize = 4;
/// Sibling pairs laid out per stacking round (the narrowest wins).
const STACK_BEAM: usize = 3;
/// Least drop in the centre offset (share of half the breadth) that
/// moving the largest subgraph to another sibling slot must buy.
const CENTRE_MIN_GAIN: f64 = 0.05;
/// Share of extra breadth a centring move may cost.
const CENTRE_BREADTH_SLACK: f64 = 0.05;
/// Ordering-key distance that puts a subgraph before the first (or
/// after the last) sibling (keys are layer shares in `0..1`).
const CENTRE_KEY_STEP: f64 = 0.01;
/// Share of half the breadth around the centre that the largest node
/// slides into (D4): it moves only as far as that band, not to the
/// exact middle.
const CENTRE_SLIDE_BAND: f64 = 0.15;
/// Area ratio by which a top-level node must outsize every other
/// top-level node, and every other node of its layer, to count as the
/// largest object that D3/D4 centre (G3). Equal or near-equal boxes have
/// no largest one: they keep the author's declaration order.
const CENTRE_MIN_SIZE_RATIO: f64 = 1.3;
/// Least distance (px) the D4 slide must move the largest node; a
/// smaller nudge would only kink its edges for no visible gain (G8).
const CENTRE_SLIDE_MIN_MOVE: f64 = 12.0;
/// Two breadth positions closer than this (px) count as aligned: the
/// edge between them is drawn straight, and D4 keeps it so.
const ALIGN_EPS: f64 = 0.5;
/// Largest diagram (real nodes plus edges) that gets the layout search
/// (ribbon narrowing and largest-subgraph slots, each a few to a few
/// dozen full placements judged on drawn crossings and edge-through-node
/// hits) and the largest-node centring (D3 walk, D4 slide). Hand-written
/// flowcharts sit far below; a generated graph above it gets the single
/// crossing-minimised placement with no centring, which keeps layout time
/// linear in the number of placements (bench: 1000 nodes).
const SEARCH_MAX_ELEMENTS: usize = 600;
/// Element placements (real nodes plus edges, per candidate laid out)
/// the layout search may spend on one diagram: about twenty candidates
/// of the largest searched graph ([`SEARCH_MAX_ELEMENTS`]). The user's
/// diagrams lay out at most a dozen candidates of under a hundred
/// elements and never reach it; a large one stops narrowing or centring
/// once it is spent, keeping the best placement so far.
const SEARCH_WORK: usize = 12_000;

/// Width an edge-label box adds around its text (the scene draws it
/// `text_width + LABEL_PAD_W` wide).
pub(crate) const LABEL_PAD_W: f64 = 14.0;
/// Height of a drawn edge-label box.
pub(crate) const LABEL_BOX_H: f64 = 20.0;
/// Room a label kept between adjacent layers leaves to each end node
/// once its gap had to grow for it.
pub(crate) const LABEL_CLEAR: f64 = 6.0;

/// Room (px) a labelled edge between adjacent TD / BT layers keeps
/// between its label and either end (report M3, §2.6): an arrowhead and
/// a stub as long again as the label is tall, so the S the edge draws
/// to its target bends above or below the label instead of under it,
/// and the head never touches the chip. With the gap growing only when
/// the label would not fit at all, a 20 px chip swallowed a third of
/// the 60 px gap and the edge read as two hooks (long-labels, demo).
/// 72 px for a one-line label, as mermaid's label rank gives it.
pub(crate) const LABEL_RUN: f64 = ARROW_MARKER_PX + 15.0;

/// Room (px) back from the far end of a compact branch point's fan
/// line that its arrowhead takes from a label: [`ARROW_MARKER_PX`] plus
/// [`LABEL_CLEAR`], and as much again, since the label stage measures
/// the head back to the first sample of the drawn curve that far from
/// its tip.
const FAN_HEAD_ROOM: f64 = ARROW_MARKER_PX + 2.0 * LABEL_CLEAR;

/// Gap (px, border to border along the flow) between a compact branch
/// point and the far end of an edge of its fan, for a label reaching
/// `beyond` past its centre toward that end: the label stage keeps such
/// a label only past [`FAN_FAR`] of its line (where the fan's branches
/// have spread apart), clear of the far end's arrowhead
/// ([`FAN_HEAD_ROOM`]).
fn fan_gap(beyond: f64) -> f64 {
    (beyond + FAN_HEAD_ROOM) / (1.0 - FAN_FAR)
}

/// Report M3, B7 review r3 #1: the room (px) an LR / RL gap keeps along
/// the flow past the label of a styled edge across it (dotted, thick,
/// `linkStyle`-painted: [`crate::scene::label_stub`]) — each of its end
/// glyphs (one at least), and on either side of the chip
/// [`LABEL_CLEAR`] plus the run of stroke the style reads on; 0 for a
/// plain edge, whose gap keeps the plain head room. The label stage
/// keeps the chip off those runs.
fn styled_room(e: &crate::model::Edge) -> f64 {
    let stub = crate::scene::label_stub(e.kind, &e.meta);
    if stub <= 0.0 {
        return 0.0;
    }
    end_glyphs(e) as f64 * ARROW_MARKER_PX + 2.0 * (LABEL_CLEAR + stub)
}

/// How many end glyphs (arrowheads, circles, crosses) edge `e` draws,
/// one at least: the gap room an LR / RL label leaves along its line is
/// counted per glyph ([`lr_label_room`], [`styled_room`]).
fn end_glyphs(e: &crate::model::Edge) -> usize {
    let marks = usize::from(e.ends.start_marker(e.kind).is_some())
        + usize::from(e.ends.end_marker(e.kind).is_some());
    marks.max(1)
}

/// Report M3: the room (px) an LR / RL gap keeps along the flow past the
/// label of an edge across it — each of its end glyphs (one at least)
/// and [`LABEL_CLEAR`] on either side of the chip, or a styled edge's
/// [`styled_room`] where that is more. A double-headed edge (`<-->`)
/// keeps both heads' room (B7 review r4 #4): sized for one, its label
/// had no spot clear of both heads on its line and stepped above it.
fn lr_label_room(e: &crate::model::Edge) -> f64 {
    (2.0 * LABEL_CLEAR + end_glyphs(e) as f64 * ARROW_MARKER_PX).max(styled_room(e))
}

/// One line of a compact branch point's fan between two adjacent TD/BT
/// layers, in breadth: from where it leaves the point to the port at its
/// far end, with its label's width if it has one.
#[derive(Clone, Copy)]
struct FanLine {
    point: usize,
    from: f64,
    to: f64,
    width: Option<f64>,
}

/// Breadth of a fan line `share` (0..=1) of the way along the flow: the
/// scene draws a line between adjacent layers as an S (both control
/// points half-way along the flow), whose depth runs `1.5t - 1.5t^2 +
/// t^3` and breadth `3t^2 - 2t^3` of the way.
fn fan_line_at(line: &FanLine, share: f64) -> f64 {
    let share = share.clamp(0.0, 1.0);
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..32 {
        let t = (lo + hi) / 2.0;
        if 1.5 * t - 1.5 * t * t + t * t * t < share {
            lo = t;
        } else {
            hi = t;
        }
    }
    line.from + (line.to - line.from) * (3.0 * hi * hi - 2.0 * hi * hi * hi)
}

/// Per edge: how far the outermost of the parallel edges it belongs to
/// (one node pair, `par` = `scene::parallel_offsets`) sits off their
/// middle; 0 for an edge with no parallel twin.
fn parallel_reach(g: &Graph, par: &[f64]) -> Vec<f64> {
    let pair = |ei: usize| {
        let e = &g.edges[ei];
        (e.from.min(e.to), e.from.max(e.to))
    };
    let mut reach: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    for (ei, off) in par.iter().enumerate() {
        let r = reach.entry(pair(ei)).or_insert(0.0);
        *r = r.max(off.abs());
    }
    (0..par.len()).map(|ei| reach[&pair(ei)]).collect()
}

/// Where a compact branch point's fan line meets a boxy far end, off the
/// end's centre: [`port_lean`] toward the point (`toward`: its breadth
/// less the end's), shifted by its parallel offset `off`. Parallel lines
/// to one end (the outermost `reach` off their middle) keep their
/// spacing, as the port stage spaces their ends (`ports::assign`): their
/// shared lean only goes as far as keeps the outermost [`PORT_INSET`]
/// in from the box's corner.
fn fan_end_lean(shape: Shape, size: (f64, f64), toward: f64, off: f64, reach: f64) -> f64 {
    if reach <= 0.0 {
        return port_lean(shape, size, toward, off);
    }
    let span = (size.0 / 2.0 - PORT_INSET - reach).max(0.0);
    port_lean(shape, size, toward, 0.0).clamp(-span, span) + off
}

/// Every (note card, fork/join bar) pair a state note links (see
/// [`crate::style::is_note_card`]).
fn bar_cards(g: &Graph) -> Vec<(usize, usize)> {
    let card = |v: usize| {
        let n = &g.nodes[v];
        crate::style::is_note_card(&n.id, n.shape, &n.style)
    };
    let bar = |v: usize| g.nodes[v].shape == Shape::ForkBar;
    g.edges
        .iter()
        .filter_map(|e| match (card(e.from), card(e.to)) {
            (true, false) if bar(e.to) => Some((e.from, e.to)),
            (false, true) if bar(e.from) => Some((e.to, e.from)),
            _ => None,
        })
        .collect()
}

/// Per layered node, its neighbours one way (`preds` or `succs`).
type Links<'a> = Cow<'a, [Vec<usize>]>;

/// `preds` / `succs` without the links between a fork/join bar and a
/// state note card (see [`crate::style::is_note_card`]) one layer apart;
/// borrowed when the graph has none.
fn unlink_bar_cards<'a>(
    g: &Graph,
    preds: &'a [Vec<usize>],
    succs: &'a [Vec<usize>],
) -> (Links<'a>, Links<'a>) {
    let pairs = bar_cards(g);
    if pairs.is_empty() {
        return (Cow::Borrowed(preds), Cow::Borrowed(succs));
    }
    let (mut p, mut s) = (preds.to_vec(), succs.to_vec());
    for (a, b) in pairs {
        for (u, v) in [(a, b), (b, a)] {
            s[u].retain(|&w| w != v);
            p[v].retain(|&w| w != u);
        }
    }
    (Cow::Owned(p), Cow::Owned(s))
}

/// Most a gap grows (px, past the least it needs anyway) to stack the
/// labels of a compact branch point's fan.
const FAN_GAP_MAX: f64 = 4.0 * GAP_L;

/// Most a fan's labels grow their gap past the plain labelled gap
/// (`ext + 2 * LABEL_RUN`, report M3) in [`hub_label_room`]: a fan too
/// crowded for that keeps the plain gap (its labels then find their own
/// clear spots, as before).
const HUB_LABEL_GROW_MAX: f64 = 48.0;

/// Step (px) [`hub_label_room`] grows a trial gap by.
const HUB_LABEL_STEP: f64 = 2.0;

/// A fan's curves in [`hub_label_room`]: `(port, far end, edge)`.
type FanCurves = Vec<(f64, f64, usize)>;

/// Per lower layer: `(upper end, lower end, gap)` — the least border to
/// border gap at which a labelled edge across one gap, drawn as the
/// builder draws it (an S whose controls sit mid-gap), keeps its label
/// centred on its curve's middle clear of the other curves of each fan
/// it is part of (B7 review r1 #6, report M3: a hub's labels sat in the
/// half near the head, on the bend: `express` 5.7 px off its head's
/// base, where at the middle its box met `Pack`'s curve). A fan is the
/// edges between one node and the next layer, its ports modelled as the
/// ports stage spreads a side ([`crate::ports::PORT_EVEN_MAX`], ordered
/// by their far ends); the label clears another curve by
/// [`STROKE_PAD`]. Only gaps that need more than the plain labelled gap,
/// by at most [`HUB_LABEL_GROW_MAX`], get an entry. The label stage
/// prefers such a label's middle (`labels::place`).
fn hub_label_room(
    g: &Graph,
    back: &[bool],
    alayer: &[usize],
    bpos: &[f64],
    absize: &[f64],
    nlayers: usize,
    horizontal: bool,
) -> Vec<Vec<(usize, usize, f64)>> {
    let mut out: Vec<Vec<(usize, usize, f64)>> = vec![Vec::new(); nlayers];
    // Along LR / RL a label lies along its line, a whole width long, and
    // the labelled gap already holds it with the head (report M3); the
    // label stage slides it along its line off the others.
    if horizontal {
        return out;
    }
    let n = g.nodes.len();
    let short = |ei: usize| {
        let e = &g.edges[ei];
        e.from != e.to
            && e.from < n
            && e.to < n
            && !back[ei]
            && !matches!(e.kind, EdgeKind::Invisible)
            && alayer[e.from].abs_diff(alayer[e.to]) == 1
    };
    // Parallel twins stack their own labels. Edges per unordered pair of
    // ends, counted once: a hub's whole fan is asked about, so a scan of
    // every edge per edge would be quadratic.
    let mut pairs: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();
    for f in &g.edges {
        *pairs
            .entry((f.from.min(f.to), f.from.max(f.to)))
            .or_insert(0) += 1;
    }
    let twin = |ei: usize| {
        let e = &g.edges[ei];
        pairs[&(e.from.min(e.to), e.from.max(e.to))] > 1
    };
    // The fan on one side of node `v` (facing down the flow or up it):
    // its curves `(port, far end, edge)` as the ports stage spreads them,
    // or `None` when the side holds fewer than two ends or parallel twins.
    // It depends on the side alone, so it is built once per side and
    // shared by all the side's labelled edges (a 1000-edge hub asked it a
    // thousand times).
    let fan = |v: usize, down: bool| -> Option<FanCurves> {
        // Every end on v's side facing the gap, by its far end.
        let mut side: Vec<(f64, usize)> = g
            .edges
            .iter()
            .enumerate()
            .filter(|(j, f)| {
                let far = if f.from == v { f.to } else { f.from };
                (f.from == v || f.to == v)
                    && f.from != f.to
                    && far < n
                    && !back[*j]
                    && (alayer[far] > alayer[v]) == down
                    && alayer[far] != alayer[v]
            })
            .map(|(j, f)| (bpos[if f.from == v { f.to } else { f.from }], j))
            .collect();
        if side.len() < 2 {
            return None;
        }
        // Parallel twins leave their ports a twin offset apart and
        // stack their own labels: the even comb is not their model.
        let mut fars: Vec<usize> = side
            .iter()
            .map(|&(_, j)| {
                let f = &g.edges[j];
                if f.from == v {
                    f.to
                } else {
                    f.from
                }
            })
            .collect();
        fars.sort_unstable();
        if fars.windows(2).any(|w| w[0] == w[1]) {
            return None;
        }
        side.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let k = side.len();
        let pitch = ((absize[v] - 2.0 * PORT_INSET) / (k + 1) as f64)
            .clamp(crate::ports::PORT_SPACING, crate::ports::PORT_EVEN_MAX);
        let port = |i: usize| bpos[v] + (i as f64 - (k - 1) as f64 / 2.0) * pitch;
        Some(
            side.iter()
                .enumerate()
                .filter(|(_, &(_, j))| short(j))
                .map(|(i, &(far, j))| (port(i), far, j))
                .collect(),
        )
    };
    let mut fans: std::collections::HashMap<(usize, bool), Option<FanCurves>> =
        std::collections::HashMap::new();
    for (ei, e) in g.edges.iter().enumerate() {
        let Some(lbl) = &e.label else { continue };
        // A curve the user picked (`curve:` / `interpolate`) is not the
        // builder's S.
        if !short(ei) || twin(ei) || e.meta.curve.is_some() {
            continue;
        }
        // Half extents across the flow and along it.
        let (ha, hl) = ((text_width(lbl) + LABEL_PAD_W) / 2.0, LABEL_BOX_H / 2.0);
        let floor = 2.0 * hl + 2.0 * LABEL_RUN;
        let mut need = floor;
        for v in [e.from, e.to] {
            let other = if v == e.from { e.to } else { e.from };
            let down = alayer[other] > alayer[v];
            let curves = fans.entry((v, down)).or_insert_with(|| fan(v, down));
            let Some(curves) = curves.as_ref() else {
                continue;
            };
            let Some(&(p, b, _)) = curves.iter().find(|c| c.2 == ei) else {
                continue;
            };
            // The label's box at its curve's middle, grown by the stroke
            // pad, across the flow.
            let (x0, x1) = (
                (p + b) / 2.0 - ha - STROKE_PAD,
                (p + b) / 2.0 + ha + STROKE_PAD,
            );
            // An S runs between its two ends across the flow, so only a
            // curve whose ends straddle or meet the box's span can cross
            // it (the margin keeps rounding from deciding a near miss).
            let near: Vec<(f64, f64)> = curves
                .iter()
                .filter(|&&(q, f, j)| j != ei && q.max(f) + 1e-6 >= x0 && q.min(f) - 1e-6 <= x1)
                .map(|&(q, f, _)| (q, f))
                .collect();
            let clear = |gap: f64| {
                // No other curve of the fan crosses the box at its
                // curve's middle.
                let (y0, y1) = (gap / 2.0 - hl - STROKE_PAD, gap / 2.0 + hl + STROKE_PAD);
                let (s0, s1) = (s_curve_blend(gap, y0), s_curve_blend(gap, y1));
                near.iter().all(|&(q, f)| {
                    let (a, c) = (q + (f - q) * s0, q + (f - q) * s1);
                    a.max(c) < x0 || a.min(c) > x1
                })
            };
            let mut gap = need;
            while gap <= floor + HUB_LABEL_GROW_MAX && !clear(gap) {
                gap += HUB_LABEL_STEP;
            }
            if gap <= floor + HUB_LABEL_GROW_MAX {
                need = need.max(gap);
            }
        }
        if need > floor {
            let (lo, hi) = if alayer[e.from] < alayer[e.to] {
                (e.from, e.to)
            } else {
                (e.to, e.from)
            };
            out[alayer[hi]].push((lo, hi, need));
        }
    }
    out
}

/// How far (0 at its start, 1 at its end) across the flow the S the
/// builder draws across a gap `gap` deep, its controls mid-gap, has run
/// at depth `y`. It depends on the depths alone, so a fan's curves share
/// it: the S from `p` to `f` runs at `p + (f - p) * s_curve_blend(..)`.
fn s_curve_blend(gap: f64, y: f64) -> f64 {
    let depth = |t: f64| gap * (1.5 * t * (1.0 - t) + t * t * t);
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    let y = y.clamp(0.0, gap);
    for _ in 0..30 {
        let mid = (lo + hi) / 2.0;
        if depth(mid) < y {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let t = (lo + hi) / 2.0;
    3.0 * t * t - 2.0 * t * t * t
}

/// Least gap (px, at least `need`) between two TD/BT layers that lets
/// the label stage stack the labels of the compact branch points' fans
/// `lines` between them, as it does (`labels::stack_fans`): per point, in
/// breadth order, each label takes the first row (one label box plus
/// [`LABEL_CLEAR`] apart, back from [`FAN_HEAD_ROOM`]) past [`FAN_FAR`]
/// where it lies [`FAN_SIBLING_GAP`] clear of the point's labels in that
/// row and clear of the point's other lines. When no growth makes room
/// for every label on its line (a sibling runs within a box of it all
/// the way: two transitions to one state), a label with no row on its
/// line may take one just beside it, as the label stage then does. A
/// gap no growth makes room in stays `need`.
fn fan_rows_gap(lines: &[FanLine], need: f64) -> f64 {
    if lines.iter().all(|l| l.width.is_none()) {
        return need;
    }
    [false, true]
        .into_iter()
        .find_map(|beside| fan_rows_fit(lines, need, beside))
        .unwrap_or(need)
}

/// The least gap, from `need` grown by at most [`FAN_GAP_MAX`], that
/// fits the labels of `lines` in rows as [`fan_rows_gap`] describes,
/// letting a label with no clear row on its line sit beside it when
/// `beside`; `None` if no such gap does.
fn fan_rows_fit(lines: &[FanLine], need: f64, beside: bool) -> Option<f64> {
    let pitch = LABEL_BOX_H + LABEL_CLEAR;
    let fits = |gap: f64| {
        let mut points: Vec<usize> = lines.iter().map(|l| l.point).collect();
        points.sort_unstable();
        points.dedup();
        points.into_iter().all(|p| {
            let mut fan: Vec<&FanLine> = lines.iter().filter(|l| l.point == p).collect();
            fan.sort_by(|a, b| a.to.total_cmp(&b.to));
            // Per row: the label spans placed in it.
            let mut rows: Vec<Vec<(f64, f64)>> = Vec::new();
            fan.iter().all(|l| {
                let Some(w) = l.width else { return true };
                let shares: Vec<(usize, f64)> = (0..)
                    .map_while(|r: usize| {
                        let depth = FAN_HEAD_ROOM + LABEL_BOX_H / 2.0 + r as f64 * pitch;
                        let share = 1.0 - depth / gap;
                        (share >= FAN_FAR).then_some((r, share))
                    })
                    .collect();
                // The label's span `off` across from its line in row
                // `r`: (clear of the other lines, and then taken if clear
                // of its row's labels too).
                let mut take = |r: usize, share: f64, off: f64| {
                    let b = fan_line_at(l, share) + off;
                    let (x0, x1) = (b - w / 2.0, b + w / 2.0);
                    // The other lines where the box's near edge lies,
                    // closest to it there; beside its line, none nearer
                    // than its own.
                    let near = share - LABEL_BOX_H / 2.0 / gap;
                    let room = (STROKE_PAD + LABEL_CLEAR).max(off.abs() - w / 2.0);
                    let clear = fan.iter().filter(|o| !std::ptr::eq(**o, *l)).all(|o| {
                        let ob = fan_line_at(o, near);
                        ob < x0 - room || ob > x1 + room
                    });
                    if rows.len() <= r {
                        rows.resize(r + 1, Vec::new());
                    }
                    let free = rows[r]
                        .iter()
                        .all(|&(a, z)| z + FAN_SIBLING_GAP <= x0 || x1 + FAN_SIBLING_GAP <= a);
                    if clear && free {
                        rows[r].push((x0, x1));
                    }
                    (clear, clear && free)
                };
                let mut lined = false;
                for &(r, share) in &shares {
                    let (clear, taken) = take(r, share, 0.0);
                    if taken {
                        return true;
                    }
                    lined |= clear;
                }
                // Beside it only when no row on its line is clear of the
                // other lines: away from the nearer one first, then
                // toward it; LABEL_SIDE_AIR off, else half off.
                if !beside || lined {
                    return false;
                }
                let toward = fan
                    .iter()
                    .filter(|o| !std::ptr::eq(**o, *l))
                    .map(|o| o.to - l.to)
                    .min_by(|a, b| a.abs().total_cmp(&b.abs()))
                    .unwrap_or(0.0);
                let side = if toward > 0.0 { -1.0 } else { 1.0 };
                let offs = [w / 2.0 + LABEL_SIDE_AIR, w / 2.0 - LABEL_STRADDLE];
                shares.iter().any(|&(r, share)| {
                    [side, -side]
                        .into_iter()
                        .any(|s| offs.into_iter().any(|o| take(r, share, s * o).1))
                })
            })
        })
    };
    let least = need.max(fan_gap(LABEL_BOX_H / 2.0));
    let mut gap = least;
    while gap <= least + FAN_GAP_MAX {
        if fits(gap) {
            return Some(gap);
        }
        gap += LABEL_CLEAR / 2.0;
    }
    None
}

/// Breadths the coordinate stage spaces nodes by: `absize`, with the
/// far ends of a TD/BT compact branch point's fan widened when its
/// labels are too wide for rows (`fan_rows_gap` finds no gap where each
/// sits on its own line clear of the others' lines), so they can sit
/// side by side as mermaid lays them out (its labels are nodes). The
/// estimate packs the far ends side by side in their layer's order, the
/// point centred over them; each widened end gets its label's width,
/// [`FAN_SIBLING_GAP`] to its neighbour and room for both ends' lines
/// to lean (see `scene::anchor`), less the node gap. Parallel lines to
/// one end (two transitions to one state) put their labels beside them
/// instead: when those find no spot, that end alone is widened, by
/// [`FAN_TWIN_STEP`]s, until they do (see [`fan_twin_room`]). The boxes
/// keep their size. `None` when no fan needs it.
#[allow(clippy::too_many_arguments)]
fn fan_label_room(
    g: &Graph,
    back: &[bool],
    layers: &[Vec<usize>],
    alayer: &[usize],
    absize: &[f64],
    alsize: &[f64],
    n: usize,
    horizontal: bool,
) -> Option<Vec<f64>> {
    let branch = |v: usize| is_branch_point(g.nodes[v].shape, &g.nodes[v].label);
    if horizontal || !(0..g.nodes.len()).any(branch) {
        return None;
    }
    let par = crate::scene::parallel_offsets(g);
    let reach = parallel_reach(g, &par);
    // Per (point, far layer): its labelled short fan lines.
    let mut fans: BTreeMap<(usize, usize), Vec<FanLabel>> = BTreeMap::new();
    for (ei, e) in g.edges.iter().enumerate() {
        if e.from == e.to || back[ei] || matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        let (p, far) = match (branch(e.from), branch(e.to)) {
            (true, false) => (e.from, e.to),
            (false, true) => (e.to, e.from),
            _ => continue,
        };
        let Some(lbl) = &e.label else { continue };
        if alayer[p].abs_diff(alayer[far]) != 1 {
            continue;
        }
        let w = text_width(lbl) + LABEL_PAD_W;
        fans.entry((p, alayer[far]))
            .or_default()
            .push((far, w, par[ei], reach[ei]));
    }
    // How far a line leans off its far end's centre at most.
    let flat = |v: usize| match g.nodes[v].shape {
        Shape::Diamond
        | Shape::Circle
        | Shape::DoubleCircle
        | Shape::StateStart
        | Shape::StateEnd
        | Shape::ForkBar => 0.0,
        shape => port_lean(shape, (absize[v], alsize[v]), f64::MAX, 0.0).abs(),
    };
    let mut out: Option<Vec<f64>> = None;
    for ((p, li), fan) in fans {
        let mut ends: Vec<usize> = fan.iter().map(|l| l.0).collect();
        ends.sort_unstable();
        ends.dedup();
        if ends.len() < 2 {
            continue;
        }
        let layer = &layers[li];
        // The ends reached by parallel lines.
        let twins: Vec<usize> = ends
            .iter()
            .copied()
            .filter(|&v| fan.iter().filter(|l| l.0 == v).count() > 1)
            .collect();
        // The fan's lines with the far ends spaced by `sep`.
        let lines = |sep: &[f64]| -> Vec<FanLine> {
            let mut pos = vec![0.0; layer.len()];
            for i in 1..layer.len() {
                pos[i] = pos[i - 1] + pair_sep(layer[i - 1], layer[i], n, sep, None, None, None);
            }
            let at = |v: usize| layer.iter().position(|&w| w == v).map(|i| pos[i]);
            let bp = if twins.is_empty() {
                let (lo, hi) = ends
                    .iter()
                    .filter_map(|&v| at(v))
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |m, b| {
                        (m.0.min(b), m.1.max(b))
                    });
                (lo + hi) / 2.0
            } else {
                // The coordinate stage aligns the point with the median
                // of its lines' far ends: an end reached twice pulls it
                // that much nearer.
                let mut all: Vec<f64> = fan.iter().filter_map(|l| at(l.0)).collect();
                all.sort_by(f64::total_cmp);
                let k = all.len();
                if k == 0 {
                    0.0
                } else {
                    (all[(k - 1) / 2] + all[k / 2]) / 2.0
                }
            };
            fan.iter()
                .filter_map(|&(v, w, off, reach)| {
                    let b = at(v)?;
                    let lean = if flat(v) > 0.0 {
                        let size = (absize[v], alsize[v]);
                        fan_end_lean(g.nodes[v].shape, size, bp - b, off, reach)
                    } else {
                        off
                    };
                    Some(FanLine {
                        point: p,
                        from: bp + (b - bp).clamp(-CHOICE_SIZE / 4.0, CHOICE_SIZE / 4.0),
                        to: b + lean,
                        // Between two twins a label has no clear spot.
                        width: (off.abs() >= reach).then_some(w),
                    })
                })
                .collect()
        };
        let now = lines(absize);
        if fan_rows_fit(&now, 0.0, false).is_some()
            || !twins.is_empty() && fan_rows_fit(&now, 0.0, true).is_some()
        {
            continue;
        }
        if twins.is_empty() {
            let sep = out.get_or_insert_with(|| absize.to_vec());
            for &v in &ends {
                let w = fan
                    .iter()
                    .filter(|l| l.0 == v)
                    .map(|l| l.1)
                    .fold(0.0, f64::max);
                sep[v] = sep[v].max(w + FAN_SIBLING_GAP + 2.0 * flat(v) - GAP_B);
            }
        } else if let Some(room) =
            fan_twin_room(out.as_deref().unwrap_or(absize), &twins, &fan, &lines)
        {
            out = Some(room);
        }
    }
    out
}

/// A labelled line of a compact branch point's fan, as
/// [`fan_label_room`] estimates it: (far end, label width, parallel
/// offset, parallel reach; see [`fan_end_lean`]).
type FanLabel = (usize, f64, f64, f64);

/// Step (px) by which [`fan_twin_room`] widens a far end.
const FAN_TWIN_STEP: f64 = LABEL_CLEAR;

/// Rows of labels past the first that [`fan_twin_room`] lets a fan's gap
/// grow by before it widens the far ends further instead.
const FAN_TWIN_ROWS: f64 = 2.0;

/// `sep` with the far ends `twins` (each reached by parallel lines of
/// the fan `fan`) widened by the fewest [`FAN_TWIN_STEP`]s that let the
/// fan's labels fit in rows, the twins' beside their lines (`lines`
/// draws the fan for a spacing), in a gap of at most [`FAN_TWIN_ROWS`]
/// more rows than one; failing that, the fewest that fit at all. The
/// ends grow by at most twice the widest of their labels, one label
/// either side. `None` when even that leaves no room.
fn fan_twin_room(
    sep: &[f64],
    twins: &[usize],
    fan: &[FanLabel],
    lines: &dyn Fn(&[f64]) -> Vec<FanLine>,
) -> Option<Vec<f64>> {
    let most = twins
        .iter()
        .map(|&v| {
            fan.iter()
                .filter(|l| l.0 == v)
                .map(|l| l.1)
                .fold(0.0, f64::max)
        })
        .fold(0.0, f64::max);
    let steps = (2.0 * most / FAN_TWIN_STEP).ceil() as usize;
    let compact = fan_gap(LABEL_BOX_H / 2.0) + FAN_TWIN_ROWS * (LABEL_BOX_H + LABEL_CLEAR);
    let mut first: Option<Vec<f64>> = None;
    let mut room = sep.to_vec();
    for k in 1..=steps {
        for &v in twins {
            room[v] = sep[v] + k as f64 * FAN_TWIN_STEP;
        }
        match fan_rows_fit(&lines(&room), 0.0, true) {
            Some(gap) if gap <= compact => return Some(room),
            Some(_) if first.is_none() => first = Some(room.clone()),
            _ => {}
        }
    }
    first
}

/// Line height for multi-line labels (`<br/>` → newline).
pub const LINE_H: f64 = 17.0;

/// Font size (px) the estimated-width table is calibrated at:
/// [`text_width`]'s per-character advances are Helvetica values at
/// this size. To lay the same text out at another size, rescale —
/// `text_width(s) * size / TEXT_CALIBRATION` — instead of guessing
/// the table's base size.
pub const TEXT_CALIBRATION: f64 = 14.0;

/// Font size (px) subgraph titles are drawn at.
pub(crate) const TITLE_FONT: f64 = 12.0;
/// [`text_width`] (calibrated at [`TEXT_CALIBRATION`]) to title-font px.
pub(crate) const TITLE_SCALE: f64 = TITLE_FONT / TEXT_CALIBRATION;
/// Line advance of a multi-line (`<br/>`) subgraph title.
pub(crate) const TITLE_LINE_H: f64 = LINE_H * TITLE_SCALE;

/// Header room a subgraph title needs beyond a one-line strip: one
/// [`TITLE_LINE_H`] per extra `<br/>` line.
pub(crate) fn title_extra(title: &str) -> f64 {
    (title.split('\n').count().max(1) - 1) as f64 * TITLE_LINE_H
}

/// Estimated rendered width of the WIDEST line in `s` (labels may
/// be multi-line after `<br/>` normalisation), in px at
/// [`TEXT_CALIBRATION`]. `<b>`/`<i>` styling
/// tags are interpreted, not measured: bold runs count ~6% wider,
/// the tag characters themselves count zero.
pub fn text_width(s: &str) -> f64 {
    s.split('\n')
        .map(|l| {
            // No tag and no `$$` fence: one plain run (what `spans`
            // would return), measured without building it.
            if !l.contains(['<', '$']) {
                return line_width(l);
            }
            spans(l)
                .iter()
                .map(|(t, b, _)| line_width(t) * if *b { 1.06 } else { 1.0 })
                .sum()
        })
        .fold(0.0, f64::max)
}

/// Split one label line into styled runs `(text, bold, italic)`,
/// interpreting `<b>`/`<strong>` and `<i>`/`<em>` tags the way
/// mermaid does. Unknown tags stay literal text; unclosed tags just
/// style to the end of the line. Always returns at least one run.
///
/// KaTeX-style math (`$$…$$`, mermaid's math syntax) is tokenized as
/// its own *italic* run with the `$$` fences stripped — issue #12
/// Phase A: mermaid sources with math render as recognizable italic
/// TeX text (and are measured without the fences) instead of
/// breaking; real math layout is a later phase. An unclosed `$$`
/// stays literal.
///
/// Span boundaries follow mermaid's lazy `/\$\$(.*?)\$\$/` — but the
/// body must additionally HUG its fences (non-empty, no edge
/// whitespace, not starting with `$`). That is a **deliberate
/// divergence** from mermaid ≥10.9, which happily maths any padded
/// body and thereby mangles prose like `fee $$ plus $$ tax` into a
/// centered formula; here such spans stay literal text so pre-math
/// diagrams keep rendering as they always did. A body carrying a
/// style tag (`</b>` …) also stays literal so bold/italic state
/// can't get stuck.
pub fn spans(line: &str) -> Vec<(String, bool, bool)> {
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<(String, bool, bool)> = Vec::new();
    let (mut bold, mut italic) = (false, false);
    let mut cur = String::new();
    let mut i = 0;
    // Closing fence of a tag-carrying `$$…$$` that fell through to
    // normal processing: that exact `$$` must stay literal, or it
    // would re-open a span and shift every later fence out of phase.
    let mut spent_close: Option<usize> = None;
    while i < chars.len() {
        if spent_close == Some(i) {
            cur.push_str("$$");
            spent_close = None;
            i += 2;
            continue;
        }
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '$' {
            // Earliest closing `$$` — mermaid's lazy match.
            let close = (i + 2..chars.len().saturating_sub(1))
                .find(|&k| chars[k] == '$' && chars[k + 1] == '$');
            if let Some(k) = close {
                let body: String = chars[i + 2..k].iter().collect();
                let has_tag = has_style_tag(&body);
                let hugs = !body.is_empty()
                    && !body.starts_with('$')
                    && !body.starts_with(char::is_whitespace)
                    && !body.ends_with(char::is_whitespace);
                if hugs && !has_tag {
                    if !cur.is_empty() {
                        out.push((std::mem::take(&mut cur), bold, italic));
                    }
                    out.push((body, bold, true));
                    i = k + 2;
                    continue;
                }
                if !has_tag {
                    // Padded/empty body: keep mermaid's span
                    // boundaries but as literal text.
                    cur.extend(&chars[i..k + 2]);
                    i = k + 2;
                    continue;
                }
                // A style tag inside: fall through so the tag
                // machinery still interprets it, but remember the
                // closing fence — it may not open a new span.
                spent_close = Some(k);
            }
        }
        if chars[i] == '<' {
            if let Some(j) = chars[i..].iter().position(|&c| c == '>') {
                let tag: String = chars[i + 1..i + j]
                    .iter()
                    .collect::<String>()
                    .to_ascii_lowercase();
                let known = matches!(
                    tag.as_str(),
                    "b" | "/b" | "strong" | "/strong" | "i" | "/i" | "em" | "/em"
                );
                if known {
                    if !cur.is_empty() {
                        out.push((std::mem::take(&mut cur), bold, italic));
                    }
                    match tag.as_str() {
                        "b" | "strong" => bold = true,
                        "/b" | "/strong" => bold = false,
                        "i" | "em" => italic = true,
                        _ => italic = false,
                    }
                    i += j + 1;
                    continue;
                }
            }
        }
        cur.push(chars[i]);
        i += 1;
    }
    if !cur.is_empty() || out.is_empty() {
        out.push((cur, bold, italic));
    }
    out
}

/// Whether a would-be math body carries a style tag the span
/// machinery interprets (`<b>`, `</em>`, `<br/>`, …). A bare TeX
/// less-than (`x<y`, even `p<br_rate`) is NOT a tag: `<br` counts
/// only when followed (after optional spaces) by `>` or `/`.
fn has_style_tag(body: &str) -> bool {
    let low = body.to_ascii_lowercase();
    if [
        "<b>",
        "</b>",
        "<i>",
        "</i>",
        "<em>",
        "</em>",
        "<strong>",
        "</strong>",
    ]
    .iter()
    .any(|t| low.contains(t))
    {
        return true;
    }
    low.match_indices("<br").any(|(p, _)| {
        let tail = low[p + 3..].trim_start();
        tail.starts_with('>') || tail.starts_with('/')
    })
}

/// Number of text lines in a label (at least 1).
pub fn line_count(s: &str) -> usize {
    s.split('\n').count().max(1)
}

/// Greedy word wrap of `s` to lines at most `max_w` px wide, where a
/// line measures `text_width(line) * scale` (pass `size /
/// TEXT_CALIBRATION` for text drawn at another size). Existing `\n`
/// breaks stay; a single word wider than `max_w` keeps a line to
/// itself, unsplit, as mermaid's wrapped labels do. Runs of spaces at
/// a break are dropped. For the fixed-column diagrams (journey tasks,
/// timeline periods and events) whose text would otherwise widen its
/// column without bound.
pub(crate) fn wrap_text(s: &str, max_w: f64, scale: f64) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in s.split('\n') {
        if text_width(line) * scale <= max_w {
            out.push(line.to_string());
            continue;
        }
        let words: Vec<&str> = line.split(' ').filter(|w| !w.is_empty()).collect();
        let starts = crate::wrap::greedy(words.len(), max_w, |i, j| {
            text_width(&words[i..j].join(" ")) * scale
        });
        out.extend(lines_at(&words, &starts));
    }
    out.join("\n")
}

/// `words` joined into the lines starting at `starts` (see
/// [`crate::wrap`]), one space between words.
fn lines_at(words: &[&str], starts: &[usize]) -> Vec<String> {
    starts
        .iter()
        .enumerate()
        .map(|(k, &s)| words[s..starts.get(k + 1).copied().unwrap_or(words.len())].join(" "))
        .collect()
}

/// Width (px, at [`TEXT_CALIBRATION`]) a node label without a line break
/// of its own wraps at by default ([`wrap_label`], report M7): mermaid's
/// documented `flowchart.wrappingWidth` default. Mermaid 12.2.0 draws its
/// node labels narrower, at 120 px of its 14 px text (the size
/// flowmaid's estimate is calibrated at; measured on flowchart and
/// state nodes alike), but at 120 px or 150 px a dense drawing's three-
/// and four-word labels turn into two-line blocks that reshuffle its
/// layering — dense-td's canvas went 4 249 -> 6 687 px wide and its
/// drawn crossings 93 -> 214 — while at 200 px only real sentences wrap
/// (long-labels' 410 px title and 361 px question), and the dense
/// fixtures barely move. A diagram that wants mermaid 12's look sets
/// `%%{init: {"flowchart": {"wrappingWidth": 120}}}%%`, or
/// `wrappingWidth: 120` under `config.flowchart` in its frontmatter
/// ([`Graph::wrapping_width`]).
pub const WRAP_WIDTH: f64 = 200.0;

/// Node `node`'s label as it shows when labels wrap at `width` px
/// (`measure`: a line's width at [`TEXT_CALIBRATION`], as for
/// [`intrinsic_size_with`]; report M7). A label that breaks its own
/// lines (`<br/>`, a markdown string's newlines — already `\n`) or fits
/// comes back as it is; any other is broken at its spaces (and between
/// CJK characters, which break without them) into as few lines as keep
/// each within `width`, the lines evened out so no word is left
/// stranded on the last. `<b>` / `<i>` markup is kept, re-opened on
/// each line it spans. A word wider than `width` is never split (mermaid
/// does the same): its line sets the width the others may fill to, and
/// a lone word beside it joins it.
///
/// Shapes that grow both ways with their text — decision diamonds,
/// circles — wrap at `width` / [`DIAMOND_ASPECT_MAX`]: a rhombus is 1.6
/// times as wide as its text, so a decision is then no wider than a box
/// wrapping at `width` (and a long label no longer swells a circle).
/// A diamond that wraps takes as many lines as make its rhombus the
/// smallest, never flatter than [`DIAMOND_ASPECT_MAX`]
/// ([`intrinsic_size_wrapped`]); one whose label fits keeps its compact
/// one-line rhombus. The label in the model never changes: this is what
/// [`intrinsic_size_wrapped`] sizes the node for, and what the scene's
/// node label and the SVG show.
pub fn wrap_label(node: &Node, width: f64, measure: impl Fn(&str) -> f64) -> String {
    fit_label(node, Some(width), &measure).text
}

/// A node label as [`wrap_label`] lays it out.
#[derive(Clone)]
pub(crate) struct Fit {
    /// The lines as drawn (`\n`-separated markup).
    pub(crate) text: String,
    /// The label was too long for its shape's wrapping width (wrapped,
    /// or a word too long to wrap): a diamond then keeps within
    /// [`DIAMOND_ASPECT_MAX`] ([`intrinsic_size_wrapped`]).
    long: bool,
}

/// Most lines a wrapping diamond takes past what its width needs, in
/// search of its smallest rhombus ([`fit_label`]).
const DIAMOND_DEEPEN_MAX: usize = 3;

/// [`wrap_label`] at `width` (`None`: never wrap), and whether the
/// label was long for its shape.
pub(crate) fn fit_label(node: &Node, width: Option<f64>, measure: &dyn Fn(&str) -> f64) -> Fit {
    let label = node.label.as_str();
    let keep = |long| Fit {
        text: label.to_string(),
        long,
    };
    // `$$…$$` math reads as one italic run without its fences
    // ([`spans`]): kept whole rather than rebuilt without them.
    let Some(width) = width.filter(|_| !label.contains('\n') && !label.contains("$$")) else {
        return keep(false);
    };
    let width = match node.shape {
        Shape::Diamond | Shape::Circle | Shape::DoubleCircle => width / DIAMOND_ASPECT_MAX,
        _ => width,
    };
    if measure(label) <= width {
        return keep(false);
    }
    let text = if label.contains('<') {
        wrap_runs(node.shape, &spans(label), width, measure)
    } else {
        wrap_runs(node.shape, &[(label, false, false)], width, measure)
    };
    Fit { text, long: true }
}

/// The styled runs of a one-line label wrapped at `width` for a node
/// of shape `shape` ([`fit_label`]), as `\n`-separated markup.
fn wrap_runs<S: AsRef<str>>(
    shape: Shape,
    runs: &[(S, bool, bool)],
    width: f64,
    measure: &dyn Fn(&str) -> f64,
) -> String {
    let toks = crate::wrap::Tokens::new(runs);
    let w = toks.widths(measure);
    let mut starts = crate::wrap::wrap(&w, width);
    if shape == Shape::Diamond && starts.len() > 1 {
        // The rhombus `intrinsic_size_wrapped` draws round lines starting
        // at `st` (its width, and its height at its text's): fewer,
        // wider lines make it flat. The question takes a line more while
        // that keeps it within [`DIAMOND_ASPECT_MAX`] at its text's
        // height — a narrower, deeper block fills a rhombus better than
        // a wide one; when no depth up to [`DIAMOND_DEEPEN_MAX`] lines
        // more does, the one whose capped rhombus is smallest.
        let rhombus = |st: &[usize]| {
            let dw = ((w.widest_line(st) + 24.0) * 1.6).max(80.0);
            (dw, (BASE_H + (st.len() - 1) as f64 * LINE_H) * 1.7)
        };
        let fits = |(dw, dh): (f64, f64)| dw <= dh * DIAMOND_ASPECT_MAX;
        let area = |(dw, dh): (f64, f64)| dw * dh.max(dw / DIAMOND_ASPECT_MAX);
        let room = width.max(w.widest_line(&starts));
        let k0 = starts.len();
        let mut best = (area(rhombus(&starts)), starts.clone());
        let mut k = k0;
        while !fits(rhombus(&starts)) && k < k0 + DIAMOND_DEEPEN_MAX {
            k += 1;
            let Some(st) = crate::wrap::into_lines(w.len(), k, room, |i, j| w.line(i, j)) else {
                break;
            };
            let a = area(rhombus(&st));
            if a < best.0 {
                best = (a, st.clone());
            }
            starts = st;
        }
        if !fits(rhombus(&starts)) {
            starts = best.1;
        }
    }
    toks.lines(&starts).join("\n")
}

/// Width of a single line (Helvetica at [`TEXT_CALIBRATION`] px)
/// per character class.
/// Without real font metrics this stays approximate, but it is far
/// more accurate than a flat average: capitals ~9.7px, i/l ~3.4px,
/// m/W ~12-13px, CJK/emoji ~14px.
fn line_width(s: &str) -> f64 {
    // A full-width glyph advances 1em at the calibration size.
    const EM: f64 = TEXT_CALIBRATION;
    s.chars()
        .map(|c| match c {
            'i' | 'l' | 'j' => 3.4,
            ' ' | '.' | ',' | ':' | ';' | '!' | '\'' | 't' | 'f' | 'I' | '|' => 3.9,
            'r' | '(' | ')' | '[' | ']' | '-' | '/' => 4.7,
            's' | 'c' | 'k' | 'v' | 'x' | 'y' | 'z' | 'J' => 7.0,
            'm' | 'M' => 11.7,
            'w' => 10.1,
            'W' => 13.2,
            'A'..='Z' => 9.7,
            // Joiners and emoji variation selectors draw nothing.
            '\u{200D}' | '\u{FE0F}' => 0.0,
            '—' => EM,
            // Arrows, misc symbols and dingbats come from a fallback
            // font at about 1em — ~14px, not a Latin letter's 7.8.
            '\u{2190}'..='\u{21FF}' | '\u{2600}'..='\u{27BF}' => EM,
            c if (c as u32) >= 0x2E80 => 14.0, // CJK, emoji, wide symbols
            _ => 7.8,
        })
        .sum()
}

/// Intrinsic node size (width, height) in pixels, based on shape,
/// the estimated widest-line width, and the number of label lines.
///
/// Uses the built-in Helvetica estimate ([`text_width`]). Embedders
/// whose renderer uses a REAL font should size nodes with
/// [`intrinsic_size_wrapped`] (or [`intrinsic_size_with`] at the default
/// width) and feed the sizes, with the same measure, to
/// `scene::scene_sized_with` / `scene::route_sized_with` — that is the
/// documented path for accurate node boxes (issue #14), whose label
/// lines are the ones the boxes were sized for.
pub fn intrinsic_size(node: &Node) -> (f64, f64) {
    intrinsic_size_with(node, text_width)
}

/// Height (px) a cylinder adds over its text for its caps.
const CYLINDER_CAPS_H: f64 = 16.0;
/// Deepest (px) a cylinder's cap ellipse gets ([`cylinder_cap_ry`]): 2 px
/// past the old fixed 8. Mermaid's lid keeps deepening (15 px on a
/// 200 px drum), but here a deeper lid curves the flanks in so far that
/// an over-full side's corner ports (met as the box's) float off them:
/// dense-lr's 12-edge hub, 6.8 px at 13.5.
const CYLINDER_RY_MAX: f64 = 10.0;

/// Half height (px) of a cylinder's cap ellipse on a node `w` px wide:
/// mermaid's `rx / (2.5 + w / 50)` (`rx = w / 2`), so a narrow drum
/// gets a shallower lid (7.5 px on the narrowest node), up to
/// [`CYLINDER_RY_MAX`] from about 80 px wide. The node keeps its size
/// ([`intrinsic_size_with`]: [`CYLINDER_CAPS_H`] over its text), and the
/// painters take at most a quarter of a host-sized node's height.
pub(crate) fn cylinder_cap_ry(w: f64) -> f64 {
    let w = w.max(0.0);
    ((w / 2.0) / (2.5 + w / 50.0)).min(CYLINDER_RY_MAX)
}

/// [`intrinsic_size`] with an injectable text measurer: `measure`
/// receives one raw label LINE (may contain `<b>`/`<i>` tags — strip
/// or style them via [`spans`]) and returns its rendered width in px
/// at the diagram's 14px base font ([`TEXT_CALIBRATION`]). Shape
/// padding, multi-line growth and minimums are applied on top, using
/// the same public constants ([`PAD_X`], [`BASE_H`], [`MIN_W`],
/// [`LINE_H`]) the engine uses.
pub fn intrinsic_size_with(node: &Node, measure: impl Fn(&str) -> f64) -> (f64, f64) {
    intrinsic_size_wrapped(node, Some(WRAP_WIDTH), measure)
}

/// [`intrinsic_size_with`] for labels that wrap at `wrapping_width`
/// ([`Graph::wrapping_width`]: `None` keeps every label on the lines it
/// was written with): the node sized for its label as [`wrap_label`]
/// breaks it. For a diagram's own width, as its scene lays it out:
/// `intrinsic_size_wrapped(&g.nodes[v], g.wrapping_width(), measure)`
/// — and pass the same `measure` to `scene::scene_sized_with` /
/// `scene::route_sized_with`, so the lines the scene hands out for each
/// node are the ones its box was sized for.
pub fn intrinsic_size_wrapped(
    node: &Node,
    wrapping_width: Option<f64>,
    measure: impl Fn(&str) -> f64,
) -> (f64, f64) {
    fit_size(node, &fit_label(node, wrapping_width, &measure), &measure)
}

/// The size of node `node` drawn with its label laid out as `fit`.
pub(crate) fn fit_size(node: &Node, fit: &Fit, measure: &dyn Fn(&str) -> f64) -> (f64, f64) {
    let label = fit.text.as_str();
    let tw = label.split('\n').map(measure).fold(0.0, f64::max);
    // Height grows with extra label lines beyond the first.
    let extra = (line_count(label) - 1) as f64 * LINE_H;
    let base_h = BASE_H + extra;
    match node.shape {
        Shape::Rect | Shape::Rounded => ((tw + 2.0 * PAD_X).max(MIN_W), base_h),
        Shape::Stadium => ((tw + 2.0 * PAD_X + 12.0).max(MIN_W + 12.0), base_h),
        // Subroutine has inner side bars; parallelograms slant — both
        // need a bit of extra horizontal room.
        Shape::Subroutine | Shape::Parallelogram | Shape::ParallelogramAlt => {
            ((tw + 2.0 * PAD_X + 24.0).max(MIN_W + 24.0), base_h)
        }
        // Hexagon points eat horizontal space.
        Shape::Hexagon => ((tw + 2.0 * PAD_X + 28.0).max(MIN_W + 28.0), base_h),
        // Cylinder caps add vertical room (two of the narrowest caps: a
        // wider drum's deeper lid takes from its body, and its label
        // drops below the lid, see `scene::CYLINDER_LABEL_DROP`).
        Shape::Cylinder => ((tw + 2.0 * PAD_X).max(MIN_W), base_h + CYLINDER_CAPS_H),
        // A diamond with nothing to say is a branch point (a state
        // `<<choice>>`, or a flowchart `A{ }`): compact, mermaid-style.
        Shape::Diamond if is_branch_point(node.shape, &node.label) => (CHOICE_SIZE, CHOICE_SIZE),
        // Diamonds need extra room so the text fits in the middle. One
        // whose label was too long for it ([`fit_label`]: wrapped, or a
        // word too long to) is never flatter than [`DIAMOND_ASPECT_MAX`]
        // (report §2.12: 616 x 65 on long-labels), while a short label
        // keeps its compact one-line rhombus (report §6).
        Shape::Diamond => {
            let w = ((tw + 24.0) * 1.6).max(80.0);
            let h = base_h * 1.7;
            (
                w,
                if fit.long {
                    h.max(w / DIAMOND_ASPECT_MAX)
                } else {
                    h
                },
            )
        }
        Shape::Circle => {
            let d = (tw + 24.0).max(52.0).max(base_h);
            (d, d)
        }
        Shape::DoubleCircle => {
            let d = (tw + 32.0).max(60.0).max(base_h);
            (d, d)
        }
        // stateDiagram pseudostates: fixed size, no label.
        Shape::StateStart => (14.0, 14.0),
        Shape::StateEnd => (18.0, 18.0),
        Shape::ForkBar => BAR_SIZE,
    }
}

pub fn layout(g: &Graph) -> LayoutResult {
    layout_sized(g, &node_sizes(g))
}

/// Widest a decision diamond gets for its height: a 1.6 : 1 rhombus.
pub const DIAMOND_ASPECT_MAX: f64 = 1.6;

/// Every node's label as drawn and its intrinsic size, with the
/// built-in text estimate, each label wrapped once at the graph's width
/// ([`wrap_label`], [`intrinsic_size_wrapped`]): what `scene()`,
/// `route()`, `route_partial()` and the compact fold lay out.
pub(crate) fn node_texts(g: &Graph) -> (Vec<String>, Vec<(f64, f64)>) {
    let ww = g.wrapping_width();
    WRAP_MEMO.with(|memo| {
        let mut memo = memo.borrow_mut();
        if memo.len() > WRAP_MEMO_MAX {
            memo.clear();
        }
        g.nodes
            .iter()
            .map(|n| {
                // What the wrap depends on besides the text: the width, and
                // whether the shape wraps narrower (round) or deepens (a
                // diamond, see `fit_label`).
                let how = (
                    ww.map_or(u64::MAX, f64::to_bits),
                    match n.shape {
                        Shape::Diamond => 2u8,
                        Shape::Circle | Shape::DoubleCircle => 1,
                        _ => 0,
                    },
                );
                let fit = match memo
                    .get(n.label.as_str())
                    .and_then(|v| v.iter().find(|(k, _)| *k == how))
                {
                    Some((_, fit)) => fit.clone(),
                    None => {
                        let fit = fit_label(n, ww, &text_width);
                        memo.entry(n.label.clone())
                            .or_default()
                            .push((how, fit.clone()));
                        fit
                    }
                };
                let size = fit_size(n, &fit, &text_width);
                (fit.text, size)
            })
            .unzip()
    })
}

/// Most labels [`WRAP_MEMO`] holds before it starts over.
const WRAP_MEMO_MAX: usize = 4096;

/// A wrapping key: the graph's width (bits; `u64::MAX` for none) and the
/// shape's wrapping class.
type WrapKey = (u64, u8);

thread_local! {
    /// Node labels [`node_texts`] already wrapped with the built-in
    /// estimate, by their text: every `route` / `route_partial` frame of
    /// a drag lays out the same labels as the one before, and wrapping a
    /// long label costs more than the edges round it. A memo of
    /// [`fit_label`] — the answer is the same either way, so output stays
    /// deterministic — dropped whole once it holds [`WRAP_MEMO_MAX`]
    /// labels.
    static WRAP_MEMO: std::cell::RefCell<std::collections::HashMap<String, Vec<(WrapKey, Fit)>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Every node's intrinsic size ([`node_texts`]).
pub(crate) fn node_sizes(g: &Graph) -> Vec<(f64, f64)> {
    node_texts(g).1
}

/// Every node's label as drawn when lines are measured by `measure`:
/// wrapped at the graph's width ([`wrap_label`]).
pub(crate) fn node_labels(g: &Graph, measure: &dyn Fn(&str) -> f64) -> Vec<String> {
    let ww = g.wrapping_width();
    g.nodes
        .iter()
        .map(|n| fit_label(n, ww, measure).text)
        .collect()
}

/// Same as [`layout`] but with caller-provided node sizes (width,
/// height in pixels) — used by pipelines where node size doesn't
/// come from the label, e.g. ER entity tables or icon nodes.
pub fn layout_sized(g: &Graph, sizes: &[(f64, f64)]) -> LayoutResult {
    layout_core(g, sizes, None, &[], None).0
}

/// Cluster-aware layout. `node_cluster[v]` is v's subgraph path,
/// outermost id first (empty = top level). All real nodes and edges
/// are laid out in one global Sugiyama pass — no supernode collapse —
/// while each subgraph's members are kept contiguous within every
/// layer and separated from siblings, so cross-cluster edges route in
/// the channels between boxes instead of tangling through them. The
/// caller builds cluster rectangles from the returned member centres.
pub fn layout_clustered(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: &[Vec<usize>],
) -> LayoutResult {
    // The switch of the block layout is read here, once, for a call that
    // comes in through the public function ([`crate::levels`]).
    let blocks = (crate::levels::blocks_on() && crate::levels::border_edge(g, node_cluster))
        .then(BlockLinks::default);
    layout_core(g, sizes, Some(node_cluster), &[], blocks).0
}

/// Per original edge, per channel waypoint: the cluster path the
/// waypoint's dummy was laid out in (see [`layout_clustered_ranked`]).
pub(crate) type EdgeGroups = Vec<Vec<Vec<usize>>>;

/// Per node of a graph laid out as blocks: its chain of blocks as the
/// layering took them, outermost first — its subgraph path without the
/// subgraphs the lane rule dissolved ([`block_rank`]). Two nodes with
/// the same chain are plain nodes of one level: an upstream edge
/// between them keeps its lane ([`keeps_lane`]).
pub(crate) type BlockPaths = Vec<Vec<usize>>;

/// Whether an edge that runs against the flow is drawn as a back-edge
/// lane: always before blocks; in a block layout (`blocks`) only
/// between two plain nodes of one level. Every other upstream edge of a
/// block layout runs in a channel, out of its source's upstream side
/// and into its target's downstream side.
pub(crate) fn keeps_lane(blocks: Option<&[Vec<usize>]>, from: usize, to: usize) -> bool {
    blocks.map_or(true, |paths| paths[from] == paths[to])
}

/// The channel point that carries a chain's label in a block layout:
/// the middle one of the chain's run at the level of its ends' common
/// ancestor (`common`, their shared subgraph path), which lies outside
/// both end blocks; without such a run, the middle of the whole chain.
/// `paths` holds each channel point's subgraph path.
pub(crate) fn label_slot(paths: &[Vec<usize>], common: &[usize]) -> usize {
    let run: Vec<usize> = (0..paths.len())
        .filter(|&i| paths[i].as_slice() == common)
        .collect();
    if run.is_empty() {
        paths.len() / 2
    } else {
        run[run.len() / 2]
    }
}

/// A rank-only link `(from, to)`: every `to` node ranks at least one
/// layer below every `from` node.
pub(crate) type RankGroup = (Vec<usize>, Vec<usize>);

/// [`layout_clustered`] plus `rank_only` groups that rank their `to`
/// nodes below their `from` nodes and pull them together in the
/// ordering, but get no channel and are never drawn — the stand-ins for
/// edges that end on a whole subgraph box. Also returns, for every
/// edge, the cluster path of each channel waypoint, so the caller can
/// tell which box a lane runs inside; and, when the graph was laid out
/// as blocks (`Graph::block_links`), each node's chain of blocks as the
/// layering took them ([`BlockPaths`]).
pub(crate) fn layout_clustered_ranked(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: &[Vec<usize>],
    rank_only: &[RankGroup],
) -> (LayoutResult, EdgeGroups, Option<BlockPaths>) {
    layout_core(
        g,
        sizes,
        Some(node_cluster),
        rank_only,
        g.block_links.clone(),
    )
}

fn layout_core(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: Option<&[Vec<usize>]>,
    rank_only: &[RankGroup],
    blocks: Option<BlockLinks>,
) -> (LayoutResult, EdgeGroups, Option<BlockPaths>) {
    assert_eq!(
        sizes.len(),
        g.nodes.len(),
        "number of sizes must match number of nodes"
    );
    let groups: Vec<RankGroup> = rank_only.to_vec();
    let drawn = groups.len();
    // The search judges candidates on drawn crossings and hits; a graph
    // past the budget skips it and never pays for those measures.
    let search = within_search_budget(g);
    let plain = Opts {
        judge: search,
        ..Opts::default()
    };
    // Cycle breakers from whichever walk reverses fewer edges (B8 review
    // r1 #3) up to WALK_MAX_ELEMENTS; past it, the walk in source order.
    let walk = if g.nodes.len() + g.edges.len() <= WALK_MAX_ELEMENTS {
        Walk::Fewest
    } else {
        Walk::Source
    };
    let rk = rank(g, &groups, drawn, node_cluster, walk, blocks.as_ref());
    let mut layer = rk.layer.clone();
    if g.tight_sources {
        if let Some(tight) = tighten_sources(g, &groups, &rk, &layer) {
            layer = tight;
        }
    }
    let first = place(g, sizes, node_cluster, &groups, &rk, &layer, plain, None);
    let mut st = State {
        seed: first.order.clone(),
        groups,
        drawn,
        walk,
        blocks,
        rk,
        layer,
        best: first,
        opts: plain,
        work: SEARCH_WORK,
    };
    if !search {
        return (st.best.lo, st.best.groups, st.rk.blocks.map(|b| b.paths));
    }
    if st.best.lo.total_b > RIBBON_ENTRY * st.best.depth {
        narrow(g, sizes, node_cluster, &mut st);
    }
    centre_largest(g, sizes, node_cluster, &mut st);
    (st.best.lo, st.best.groups, st.rk.blocks.map(|b| b.paths))
}

/// Is `g` small enough for the layout search ([`SEARCH_MAX_ELEMENTS`])?
fn within_search_budget(g: &Graph) -> bool {
    g.nodes.len() + g.edges.len() <= SEARCH_MAX_ELEMENTS
}

/// Options of one [`place`] run.
#[derive(Clone, Copy, Default)]
struct Opts {
    /// Re-pack the breadth after Brandes-Köpf ([`compact_breadth`]).
    compact: bool,
    /// Override one subgraph's ordering key `(cluster id, key)` after
    /// the crossing-minimised ordering ([`centre_largest`]).
    hint: Option<(usize, f64)>,
    /// The graph is within the search budget: measure the placement
    /// ([`drawn_crossings`], [`node_hits`], the drawn breadth) and centre
    /// its largest top-level node (D3 walk, D4 slide). Off, the measures
    /// stay zero and the crossing-minimised order and BK positions stand.
    judge: bool,
}

/// The layering being improved and its best placement so far.
struct State {
    groups: Vec<RankGroup>,
    /// `groups[..drawn]` stand for drawn edges and box packing; the
    /// rest are the stackings and wraps narrowing added ([`rank`]).
    drawn: usize,
    /// The walk every candidate layering takes its cycle breakers from.
    walk: Walk,
    /// Block mode ([`rank`]): every candidate layering is a block
    /// layering.
    blocks: Option<BlockLinks>,
    /// A block layout: the first placement's final order, which every
    /// later placement starts from ([`BlockOrder::run`]).
    seed: Option<OrderSeed>,
    rk: Ranking,
    layer: Vec<usize>,
    best: Placement,
    /// Options `best` was placed with.
    opts: Opts,
    /// What is left of [`SEARCH_WORK`].
    work: usize,
}

impl State {
    /// Pay for one candidate placement of `g`; `false` (paying nothing)
    /// once the search budget cannot cover it.
    fn afford(&mut self, g: &Graph) -> bool {
        let cost = g.nodes.len() + g.edges.len();
        match self.work.checked_sub(cost) {
            Some(left) => {
                self.work = left;
                true
            }
            None => false,
        }
    }
}

/// Narrow a drawing far broader than deep (G12). Each step lays out a
/// few full candidates and keeps one only when it cuts the breadth by
/// at least STAGGER_MIN_GAIN while the drawn crossings stay within
/// NARROW_CROSS_SLACK of the start, so narrowing never tangles.
fn narrow(g: &Graph, sizes: &[(f64, f64)], node_cluster: Option<&[Vec<usize>]>, st: &mut State) {
    let ribbon = |p: &Placement| p.lo.total_b > ASPECT_TARGET * p.depth;
    let cap = crossing_budget(st.best.crossings);
    let packed_opts = Opts {
        compact: true,
        hint: None,
        judge: true,
    };
    // 0. Brandes-Köpf leaves room a plain left-packing does not need:
    // from here on every placement is compacted ([`compact_breadth`]).
    if !st.afford(g) {
        return;
    }
    let packed = place(
        g,
        sizes,
        node_cluster,
        &st.groups,
        &st.rk,
        &st.layer,
        packed_opts,
        st.seed.as_ref(),
    );
    if packed.lo.total_b <= st.best.lo.total_b && packed.crossings <= cap {
        st.best = packed;
        st.opts = packed_opts;
    }
    // Node hits may grow by NARROW_HIT_SLACK per step, and by no more
    // over the packed drawing the re-layering steps start from, so
    // several steps cannot pile up re-threadings.
    let hit_cap = st.best.hits + NARROW_HIT_SLACK;
    // A step must give back breadth without adding area or edge length
    // past their slack (report M11), crossings or node hits. Node hits
    // may grow past their slack only where the step saves more than
    // NARROW_HIT_COST crossings per hit it adds: dense-td's two biggest
    // frames stacked one way cut 33 crossings for 9 hits when its first
    // two nodes were declared the other way round, and refused, that
    // order drew it 5 240 px broad instead of 4 201 (B8 review r1 #3).
    let area = |p: &Placement| p.lo.total_b * p.depth;
    let better = |cand: &Placement, cur: &Placement| {
        cand.lo.total_b <= cur.lo.total_b * (1.0 - STAGGER_MIN_GAIN)
            && area(cand) <= area(cur) * (1.0 + NARROW_AREA_SLACK)
            && edge_length(g, &cand.lo, false)
                <= edge_length(g, &cur.lo, false) * (1.0 + NARROW_LEN_SLACK)
            && cand.crossings <= cap
            && (cand.hits <= hit_cap.min(cur.hits + NARROW_HIT_SLACK)
                || cand.crossings + NARROW_HIT_COST * cand.hits
                    < cur.crossings + NARROW_HIT_COST * cur.hits)
    };
    // A stacked box's in-edges thread past its sibling; a staggered leaf
    // only steps one layer out beside its row.
    let no_detour = |cand: &Placement, cur: &Placement| {
        edge_length(g, &cand.lo, true) <= edge_length(g, &cur.lo, true) * (1.0 + NARROW_FLOW_SLACK)
    };

    // 1. Sibling stacking: two sibling subgraphs that share layers sit
    // side by side, so their parent is as broad as both. The pairs that
    // would give back the most breadth are tried ranked one below the
    // other (a rank-only group, never drawn), unless an edge path runs
    // the other way; so is a wrap of three or more siblings sharing a
    // layer into two runs, the later run below the earlier
    // ([`wrap_candidates`], report M11). The smallest result wins the
    // round.
    if let Some(nc) = node_cluster {
        let mut tried: Vec<(usize, usize)> = Vec::new();
        let mut wraps_tried: Vec<RankGroup> = Vec::new();
        for _ in 0..STACK_MAX_ROUNDS {
            if !ribbon(&st.best) {
                break;
            }
            let pairs = stack_candidates(g, nc, &st.groups, &st.rk, &st.layer, &st.best.lo, &tried);
            let wraps: Vec<RankGroup> =
                wrap_candidates(g, nc, &st.groups, &st.rk, &st.layer, &st.best.lo)
                    .into_iter()
                    .filter(|w| !wraps_tried.contains(w))
                    .collect();
            if pairs.is_empty() && wraps.is_empty() {
                break;
            }
            let groups_to_try: Vec<(Option<(usize, usize)>, RankGroup)> = wraps
                .into_iter()
                .take(1)
                .map(|w| (None, w))
                .chain(
                    pairs
                        .iter()
                        .take(STACK_BEAM)
                        .map(|&(a, b)| (Some((a, b)), stack_group(nc, a, b))),
                )
                .collect();
            let mut won: Option<(Vec<RankGroup>, Ranking, Placement)> = None;
            for (pair, group) in groups_to_try {
                if !st.afford(g) {
                    break;
                }
                match pair {
                    Some(p) => tried.push(p),
                    None => wraps_tried.push(group.clone()),
                }
                let mut cand_groups = st.groups.clone();
                cand_groups.push(group);
                let crk = rank(
                    g,
                    &cand_groups,
                    st.drawn,
                    node_cluster,
                    st.walk,
                    st.blocks.as_ref(),
                );
                let cand = place(
                    g,
                    sizes,
                    node_cluster,
                    &cand_groups,
                    &crk,
                    &crk.layer,
                    packed_opts,
                    st.seed.as_ref(),
                );
                let cur = won.as_ref().map_or(&st.best, |w| &w.2);
                let smaller = (area(&cand), cand.lo.total_b) < (area(cur), cur.lo.total_b);
                if better(&cand, &st.best)
                    && no_detour(&cand, &st.best)
                    && (won.is_none() || smaller)
                {
                    won = Some((cand_groups, crk, cand));
                }
            }
            let Some((wg, wrk, wp)) = won else { break };
            st.groups = wg;
            st.layer = wrk.layer.clone();
            st.rk = wrk;
            st.best = wp;
            st.opts = packed_opts;
            // The other pairs now sit on new layers: judge them afresh.
            tried.clear();
        }
    }
    // 2. Sources pulled down next to their targets (dagre's network
    // simplex does the same): longest-path parks every source on the
    // first layer, however deep its targets sit, which widens it.
    if ribbon(&st.best) {
        if let Some(tight) =
            tighten_sources(g, &st.groups, &st.rk, &st.layer).filter(|_| st.afford(g))
        {
            let cand = place(
                g,
                sizes,
                node_cluster,
                &st.groups,
                &st.rk,
                &tight,
                packed_opts,
                st.seed.as_ref(),
            );
            if better(&cand, &st.best) {
                st.best = cand;
                st.layer = tight;
                st.opts = packed_opts;
            }
        }
    }
    // 3. Leaf staggering (graphviz `unflatten -l`): half the leaves of
    // the broadest layer step one layer out, so a fan of 12 boxes
    // becomes two offset rows — unless the graph keeps its leaves
    // flat (`Graph::flat_leaves`).
    let mut tried: Vec<usize> = Vec::new();
    for _ in 0..STAGGER_MAX_ROUNDS {
        if g.flat_leaves || !ribbon(&st.best) {
            break;
        }
        let Some((li, props)) = stagger_proposals(
            g,
            sizes,
            node_cluster,
            &st.groups,
            &st.rk,
            &st.layer,
            &st.best.lo,
            &tried,
        ) else {
            break;
        };
        tried.push(li);
        let mut won: Option<(Vec<usize>, Placement)> = None;
        for cand_layer in props {
            if !st.afford(g) {
                break;
            }
            let cand = place(
                g,
                sizes,
                node_cluster,
                &st.groups,
                &st.rk,
                &cand_layer,
                packed_opts,
                st.seed.as_ref(),
            );
            let cur = won.as_ref().map_or(&st.best, |w| &w.1);
            if better(&cand, &st.best) && cand.lo.total_b < cur.lo.total_b {
                won = Some((cand_layer, cand));
            }
        }
        if let Some((wl, wp)) = won {
            st.best = wp;
            st.layer = wl;
            st.opts = packed_opts;
            tried.clear();
        }
    }
}

/// The largest top-level object of a placement (G3): a node outside
/// every subgraph, or a whole top-level subgraph.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Unit {
    Node(usize),
    Cluster(usize),
}

/// Breadth span `(lo, hi)` of every top-level unit of `lo`, with its
/// estimated area: a node's own box, a subgraph's member extent grown
/// by one FRAME_PAD per nesting level (its frame).
fn unit_boxes(lo: &LayoutResult, nc: Option<&[Vec<usize>]>) -> Vec<(Unit, f64, f64, f64)> {
    let mut boxes: std::collections::BTreeMap<usize, (f64, f64, f64, f64)> =
        std::collections::BTreeMap::new();
    let mut out = Vec::new();
    for (v, p) in lo.nodes.iter().enumerate() {
        let path = nc.map_or(&[][..], |nc| nc[v].as_slice());
        let Some(&top) = path.first() else {
            out.push((
                Unit::Node(v),
                p.b - p.bsize / 2.0,
                p.b + p.bsize / 2.0,
                p.bsize * p.lsize,
            ));
            continue;
        };
        let pad = path.len() as f64 * FRAME_PAD;
        let e = boxes.entry(top).or_insert((
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ));
        e.0 = e.0.min(p.b - p.bsize / 2.0 - pad);
        e.1 = e.1.max(p.b + p.bsize / 2.0 + pad);
        e.2 = e.2.min(p.l - p.lsize / 2.0 - pad);
        e.3 = e.3.max(p.l + p.lsize / 2.0 + pad);
    }
    out.extend(
        boxes
            .into_iter()
            .map(|(c, (b0, b1, l0, l1))| (Unit::Cluster(c), b0, b1, (b1 - b0) * (l1 - l0))),
    );
    out
}

/// The largest unit of [`unit_boxes`] (the first one on a tie).
fn largest_unit(lo: &LayoutResult, nc: Option<&[Vec<usize>]>) -> Option<Unit> {
    let mut best: Option<(Unit, f64)> = None;
    for (u, _, _, area) in unit_boxes(lo, nc) {
        if best.map_or(true, |b| area > b.1) {
            best = Some((u, area));
        }
    }
    best.map(|b| b.0)
}

/// How far `unit` sits from the breadth centre of the drawing, as a
/// share of half the drawn breadth (0 = centred, 1 = at the rim): the
/// same measure as the quality harness' `center_off`, on the layout's
/// own estimate of what will be drawn ([`drawn_breadth`]).
fn centre_offset(lo: &LayoutResult, nc: Option<&[Vec<usize>]>, ext: (f64, f64), unit: Unit) -> f64 {
    let Some(&(_, b0, b1, _)) = unit_boxes(lo, nc).iter().find(|u| u.0 == unit) else {
        return 0.0;
    };
    let half = (ext.1 - ext.0) / 2.0;
    if half <= 0.0 {
        return 0.0;
    }
    (((b0 + b1) / 2.0 - (ext.0 + ext.1) / 2.0).abs() / half).min(1.0)
}

/// Centre the largest object (G3, D3). When it is a top-level subgraph,
/// it is tried at every slot of the sibling sequence (its ordering key
/// set between two siblings' keys) and the slot that brings it closest
/// to the breadth centre wins, provided the drawn crossings do not grow
/// and the breadth grows by at most CENTRE_BREADTH_SLACK. A node is
/// centred inside [`place`] instead ([`centre_node`]).
fn centre_largest(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: Option<&[Vec<usize>]>,
    st: &mut State,
) {
    let Some(nc) = node_cluster else { return };
    let Some(Unit::Cluster(c)) = largest_unit(&st.best.lo, Some(nc)) else {
        return;
    };
    let off = |p: &Placement| centre_offset(&p.lo, Some(nc), p.ext, Unit::Cluster(c));
    // The other top-level subgraphs, in ordering-key order.
    let mut others: Vec<f64> = st
        .best
        .ckeys
        .iter()
        .filter(|&&(s, _)| s != c && g.subgraphs[s].parent.is_none())
        .map(|&(_, k)| k)
        .collect();
    if others.is_empty() {
        return;
    }
    others.sort_by(f64::total_cmp);
    let own = st.best.ckeys.iter().find(|k| k.0 == c).map_or(0.0, |k| k.1);
    let mut keys: Vec<f64> = Vec::with_capacity(others.len() + 1);
    keys.push(others[0] - CENTRE_KEY_STEP);
    keys.extend(others.windows(2).map(|w| (w[0] + w[1]) / 2.0));
    keys.push(others[others.len() - 1] + CENTRE_KEY_STEP);
    let base = off(&st.best);
    // A slot must bring it CENTRE_MIN_GAIN nearer the centre: from
    // nearer than that, no slot can.
    if base < CENTRE_MIN_GAIN {
        return;
    }
    let mut won: Option<(f64, Placement, Opts)> = None;
    for key in keys {
        // The slot it already holds.
        let slot = |k: f64| others.iter().filter(|&&o| o < k).count();
        if slot(key) == slot(own) {
            continue;
        }
        if !st.afford(g) {
            break;
        }
        let opts = Opts {
            hint: Some((c, key)),
            ..st.opts
        };
        let cand = place(
            g,
            sizes,
            node_cluster,
            &st.groups,
            &st.rk,
            &st.layer,
            opts,
            st.seed.as_ref(),
        );
        let o = off(&cand);
        if cand.crossings <= st.best.crossings
            && cand.hits <= st.best.hits
            && cand.lo.total_b <= st.best.lo.total_b * (1.0 + CENTRE_BREADTH_SLACK)
            && o + CENTRE_MIN_GAIN <= won.as_ref().map_or(base, |w| w.0)
        {
            won = Some((o, cand, opts));
        }
    }
    if let Some((_, p, opts)) = won {
        st.best = p;
        st.opts = opts;
    }
}

/// Most crossings a narrowing round may end with, starting from `c`.
fn crossing_budget(c: usize) -> usize {
    c + (c as f64 * NARROW_CROSS_SLACK) as usize
}

/// Every member of subgraph `id` (at any depth).
fn members_of(nc: &[Vec<usize>], id: usize) -> Vec<usize> {
    (0..nc.len()).filter(|&v| nc[v].contains(&id)).collect()
}

/// The rank-only group that ranks all of `below` under all of `above`.
fn stack_group(nc: &[Vec<usize>], above: usize, below: usize) -> RankGroup {
    (members_of(nc, above), members_of(nc, below))
}

/// Total edge length of a placement, node centre to centre along both
/// axes, or along the flow only (`flow`; self-loops left out): what a
/// narrowing step must not grow past [`NARROW_LEN_SLACK`] /
/// [`NARROW_FLOW_SLACK`].
fn edge_length(g: &Graph, lo: &LayoutResult, flow: bool) -> f64 {
    g.edges
        .iter()
        .filter(|e| e.from != e.to)
        .map(|e| {
            let (a, b) = (&lo.nodes[e.from], &lo.nodes[e.to]);
            let along = (a.l - b.l).abs();
            if flow {
                along
            } else {
                (a.b - b.b).abs() + along
            }
        })
        .sum()
}

/// Wrap candidates (report M11): per parent box (or the top level), the
/// three or more child boxes that share its most shared layer, in drawn
/// order, split into two runs of about equal breadth — the later run
/// ranked below the earlier one (one rank-only group), so a column of
/// boxes fed from one side folds into two instead of moving a single
/// box past its sibling. Skipped when an edge path (or rank-only link)
/// leads from the later run back into the earlier one.
fn wrap_candidates(
    g: &Graph,
    nc: &[Vec<usize>],
    groups: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
    lo: &LayoutResult,
) -> Vec<RankGroup> {
    let n = layer.len();
    // Per box: parent, layer span, breadth span.
    let mut info: BTreeMap<usize, (Option<usize>, usize, usize, f64, f64)> = BTreeMap::new();
    for v in 0..n {
        let p = &lo.nodes[v];
        for (i, &c) in nc[v].iter().enumerate() {
            let parent = i.checked_sub(1).map(|k| nc[v][k]);
            let e = info.entry(c).or_insert((
                parent,
                layer[v],
                layer[v],
                f64::INFINITY,
                f64::NEG_INFINITY,
            ));
            e.1 = e.1.min(layer[v]);
            e.2 = e.2.max(layer[v]);
            e.3 = e.3.min(p.b - p.bsize / 2.0);
            e.4 = e.4.max(p.b + p.bsize / 2.0);
        }
    }
    let mut next: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (ei, e) in g.edges.iter().enumerate() {
        if !rk.back[ei] && e.from != e.to {
            next[e.from].push(e.to);
        }
    }
    for (from, to) in groups {
        for &a in from {
            next[a].extend(to.iter().copied());
        }
    }
    let mut parents: Vec<Option<usize>> = info.values().map(|i| i.0).collect();
    parents.sort_unstable();
    parents.dedup();
    let mut out: Vec<(f64, RankGroup)> = Vec::new();
    for parent in parents {
        let kids: Vec<usize> = info
            .iter()
            .filter(|(_, i)| i.0 == parent)
            .map(|(&c, _)| c)
            .collect();
        if kids.len() < 3 {
            continue;
        }
        // The layer most of them share (the first on a tie).
        let (lmin, lmax) = (
            kids.iter().map(|c| info[c].1).min().unwrap_or(0),
            kids.iter().map(|c| info[c].2).max().unwrap_or(0),
        );
        let at = |l: usize| {
            kids.iter()
                .filter(|c| info[c].1 <= l && l <= info[c].2)
                .count()
        };
        let Some(l) = (lmin..=lmax).max_by(|&a, &b| at(a).cmp(&at(b)).then(b.cmp(&a))) else {
            continue;
        };
        let mut row: Vec<usize> = kids
            .iter()
            .copied()
            .filter(|c| info[c].1 <= l && l <= info[c].2)
            .collect();
        if row.len() < 3 {
            continue;
        }
        row.sort_by(|a, b| info[a].3.total_cmp(&info[b].3).then(a.cmp(b)));
        let width = |c: &usize| info[c].4 - info[c].3;
        let total: f64 = row.iter().map(width).sum();
        let mut acc = 0.0;
        let mut split = 1;
        let mut best = f64::INFINITY;
        for k in 1..row.len() {
            acc += width(&row[k - 1]);
            let worse = acc.max(total - acc);
            if worse < best {
                best = worse;
                split = k;
            }
        }
        let members = |cs: &[usize]| -> Vec<usize> {
            (0..n)
                .filter(|&v| cs.iter().any(|c| nc[v].contains(c)))
                .collect()
        };
        let (first, then) = (members(&row[..split]), members(&row[split..]));
        // No path from the later run back into the earlier.
        let mut seen = vec![false; n];
        let mut stack = then.clone();
        while let Some(u) = stack.pop() {
            if std::mem::replace(&mut seen[u], true) {
                continue;
            }
            stack.extend(next[u].iter().copied());
        }
        if first.iter().any(|&v| seen[v]) {
            continue;
        }
        out.push((total, (first, then)));
    }
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    out.into_iter().map(|c| c.1).collect()
}

/// Sibling pairs `(above, below)` worth stacking, not yet in `tried`:
/// siblings that share a layer, where no edge path (or rank-only link)
/// runs from `below` back into `above`, most promising first (by the
/// breadth the narrower one would give back, as `lo` drew them).
fn stack_candidates(
    g: &Graph,
    nc: &[Vec<usize>],
    groups: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
    lo: &LayoutResult,
    tried: &[(usize, usize)],
) -> Vec<(usize, usize)> {
    let n = layer.len();
    // Per subgraph with members: parent, layer span, breadth span.
    let mut info: std::collections::BTreeMap<usize, (Option<usize>, usize, usize, f64, f64)> =
        std::collections::BTreeMap::new();
    for v in 0..n {
        let p = &lo.nodes[v];
        for (i, &c) in nc[v].iter().enumerate() {
            let parent = if i == 0 { None } else { Some(nc[v][i - 1]) };
            let e = info.entry(c).or_insert((
                parent,
                layer[v],
                layer[v],
                f64::INFINITY,
                f64::NEG_INFINITY,
            ));
            e.1 = e.1.min(layer[v]);
            e.2 = e.2.max(layer[v]);
            e.3 = e.3.min(p.b - p.bsize / 2.0);
            e.4 = e.4.max(p.b + p.bsize / 2.0);
        }
    }
    // Ranking successors (real edges and rank-only links), for the
    // "no path back" test.
    let mut next: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (ei, e) in g.edges.iter().enumerate() {
        if !rk.back[ei] && e.from != e.to {
            next[e.from].push(e.to);
        }
    }
    for (from, to) in groups {
        for &a in from {
            next[a].extend(to.iter().copied());
        }
    }
    let ids: Vec<usize> = info.keys().copied().collect();
    // Per subgraph (as `ids`): its members, and every node reachable
    // from them; one walk each instead of one per pair.
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); ids.len()];
    for (v, path) in nc.iter().enumerate() {
        for c in path {
            if let Ok(i) = ids.binary_search(c) {
                members[i].push(v);
            }
        }
    }
    let reach: Vec<Vec<bool>> = members
        .iter()
        .map(|src| {
            let mut seen = vec![false; n];
            let mut stack: Vec<usize> = src.clone();
            while let Some(u) = stack.pop() {
                if std::mem::replace(&mut seen[u], true) {
                    continue;
                }
                stack.extend(next[u].iter().copied());
            }
            seen
        })
        .collect();
    let reaches = |i: usize, j: usize| members[j].iter().any(|&v| reach[i][v]);
    let mut cands: Vec<(f64, usize, usize)> = Vec::new();
    for (i, &a) in ids.iter().enumerate() {
        for (j, &b) in ids.iter().enumerate().skip(i + 1) {
            let (ia, ib) = (info[&a], info[&b]);
            if ia.0 != ib.0 || ia.2 < ib.1 || ib.2 < ia.1 {
                continue; // not siblings, or already on disjoint layers
            }
            let gain = (ia.4 - ia.3).min(ib.4 - ib.3);
            // Keep the flow: the box something already leads into goes
            // below; with no path either way, both stackings are tried
            // (the earlier box on top first).
            let pairs: Vec<(usize, usize)> = match (reaches(i, j), reaches(j, i)) {
                (true, true) => Vec::new(),
                (false, true) => vec![(b, a)],
                (true, false) => vec![(a, b)],
                (false, false) => vec![(a, b), (b, a)],
            };
            for pair in pairs {
                if !tried.contains(&pair) {
                    cands.push((gain, pair.0, pair.1));
                }
            }
        }
    }
    cands.sort_by(|x, y| y.0.total_cmp(&x.0));
    cands.into_iter().map(|c| (c.1, c.2)).collect()
}

/// Which depth-first walk [`rank`] takes its cycle breakers from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Walk {
    /// Whichever of the walk in source order and the walk in target
    /// declaration order reverses fewer edges (source order on a tie).
    Fewest,
    /// Each node's links in source order only.
    Source,
}

/// Layering result shared by every candidate placement: the DFS cycle
/// breakers over the ranking graph (real nodes then one hub per
/// rank-only group) and its longest-path layers.
struct Ranking {
    /// Per ranking link (real edges, then hub links): a cycle breaker.
    back: Vec<bool>,
    /// Rank-only group `k`'s first hub link id.
    group_eid: Vec<usize>,
    /// Longest-path layer per REAL node.
    layer: Vec<usize>,
    /// Block mode: what the block layering adds ([`block_rank`]).
    /// `None`: the layering as it was before blocks.
    blocks: Option<BlockRanking>,
}

/// What a block layering ([`block_rank`]) hands on with its layers.
#[derive(Clone, Debug, Default)]
struct BlockRanking {
    /// Per edge: a flagged stand-in ([`BlockLinks`]) — no link, so the
    /// placement gives it no channel and no pull.
    flagged: Vec<bool>,
    /// Per node: its chain of blocks as the layering took them,
    /// outermost first — its subgraph path without the subgraphs the
    /// lane rule dissolved, whose members are units of the level round
    /// them.
    paths: Vec<Vec<usize>>,
    /// The levels the lane rule fired on, in firing order (`None`: the
    /// top level).
    fired: Vec<Option<usize>>,
}

/// Steps 1 and 2: mark back-edges, then longest-path layering; with
/// `node_cluster`, every box then takes the members nothing ranks into
/// down to its first row ([`seat_box_sources`]). The groups from
/// `rank_only[drawn..]` on are narrowing's stackings and wraps: they
/// rank like the others but say nothing about where a box starts.
///
/// With `blocks` (block mode) only the back-edge marks are taken from
/// here; the layers are [`block_rank`]'s, and a flagged pair is no link.
fn rank(
    g: &Graph,
    rank_only: &[RankGroup],
    drawn: usize,
    node_cluster: Option<&[Vec<usize>]>,
    walk: Walk,
    blocks: Option<&BlockLinks>,
) -> Ranking {
    let n = g.nodes.len();
    // Block mode needs the subgraph paths; without them it is off.
    let blocks = blocks.filter(|_| node_cluster.is_some());
    let flagged_edge = |ei: usize| blocks.is_some_and(|b| b.flagged_edge(ei));
    let flagged_group = |k: usize| blocks.is_some_and(|b| b.flagged_group(k));
    // Ranking graph: the real nodes, then one HUB per rank-only group
    // (`from -> hub` weighs one layer, `hub -> to` none), so a group
    // costs |from| + |to| links instead of every from x to pair.
    let nx = n + rank_only.len();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); nx];
    for (ei, e) in g.edges.iter().enumerate() {
        if !flagged_edge(ei) {
            adj[e.from].push((e.to, ei));
        }
    }
    // Hub links ride the same DFS / layering as real edges, numbered
    // after them; `group_eid[k]` is group k's first link id.
    let ne = g.edges.len();
    let mut nlinks = ne;
    let mut group_eid = Vec::with_capacity(rank_only.len());
    for (k, (from, to)) in rank_only.iter().enumerate() {
        group_eid.push(nlinks);
        if flagged_group(k) {
            nlinks += from.len() + to.len();
            continue;
        }
        for &a in from {
            adj[a].push((n + k, nlinks));
            nlinks += 1;
        }
        for &b in to {
            adj[n + k].push((b, nlinks));
            nlinks += 1;
        }
    }

    // --- 1. Mark back-edges (cycle breakers) with iterative DFS, twice:
    // following each node's links in source order, then in the order
    // their targets were declared. The walk that reverses fewer real
    // edges wins, source order on a tie (report §2.9, B8 review r1 #3).
    // A walk in source order alone took its cycle breakers from the
    // order the edge lines happened to be written in: bench-75 with its
    // edge lines shuffled drew 56 to 68 of its 75 edges along the flow
    // and laid out 1 772 to 2 129 px along it (now 66 to 68, and 1 772
    // to 1 906 px); over 1 800 generated and corpus flowcharts the 13
    // that change lose 85 % of their defects.
    let dfs_back = |adj: &[Vec<(usize, usize)>]| -> Vec<bool> {
        let mut state = vec![0u8; nx]; // 0 unvisited, 1 on stack, 2 done
        let mut back = vec![false; nlinks];
        for s in 0..nx {
            if state[s] != 0 {
                continue;
            }
            state[s] = 1;
            let mut stack: Vec<(usize, usize)> = vec![(s, 0)];
            while !stack.is_empty() {
                let (u, ci) = *stack.last().unwrap();
                if ci < adj[u].len() {
                    stack.last_mut().unwrap().1 += 1;
                    let (v, ei) = adj[u][ci];
                    if v == u {
                        back[ei] = true; // self-loop
                        continue;
                    }
                    match state[v] {
                        0 => {
                            state[v] = 1;
                            stack.push((v, 0));
                        }
                        1 => back[ei] = true, // edge back to an ancestor = cycle
                        _ => {}
                    }
                } else {
                    state[u] = 2;
                    stack.pop();
                }
            }
        }
        back
    };
    let mut back = dfs_back(&adj);
    if walk == Walk::Fewest {
        let mut by_target = adj.clone();
        for links in by_target.iter_mut() {
            links.sort_by_key(|&(v, ei)| (v, ei));
        }
        if by_target != adj {
            let alt = dfs_back(&by_target);
            let reversed = |b: &[bool]| b[..ne].iter().filter(|&&x| x).count();
            if reversed(&alt) < reversed(&back) {
                back = alt;
            }
        }
    }

    // Block mode: the marks above are the raw marks of the block
    // layering, which does the rest.
    if let (Some(nc), Some(links)) = (node_cluster, blocks) {
        let exact = walk == Walk::Fewest;
        let (layer, blocks) = block_rank(g, nc, rank_only, &group_eid, &back, links, exact);
        // An edge runs against the flow where its layers say so (the raw
        // marks no longer do: one dropped, or turned back by the block
        // order, points down again). A flagged group pulls nothing.
        let mut back = vec![false; nlinks];
        for (ei, e) in g.edges.iter().enumerate() {
            back[ei] = e.from == e.to || layer[e.to] < layer[e.from];
        }
        for (k, (from, to)) in rank_only.iter().enumerate() {
            if flagged_group(k) {
                for id in group_eid[k]..group_eid[k] + from.len() + to.len() {
                    back[id] = true;
                }
            }
        }
        return Ranking {
            back,
            group_eid,
            layer,
            blocks: Some(blocks),
        };
    }

    // --- 2. Longest-path layering on the DAG (back-edges excluded) ---
    let mut indeg = vec![0usize; nx];
    for links in &adj {
        for &(v, ei) in links {
            if !back[ei] {
                indeg[v] += 1;
            }
        }
    }
    let mut layer = vec![0usize; nx];
    let mut q: VecDeque<usize> = (0..nx).filter(|&v| indeg[v] == 0).collect();
    while let Some(u) = q.pop_front() {
        let step = usize::from(u < n); // hub -> to adds no layer
        for &(v, ei) in &adj[u] {
            if back[ei] {
                continue;
            }
            if layer[u] + step > layer[v] {
                layer[v] = layer[u] + step;
            }
            indeg[v] -= 1;
            if indeg[v] == 0 {
                q.push_back(v);
            }
        }
    }

    // --- 3. A box's sources down to its first row.
    if let Some(nc) = node_cluster {
        let stacks_from = group_eid.get(drawn).copied().unwrap_or(nlinks);
        seat_box_sources(nc, &g.lead_nodes, &adj, &back, stacks_from, &mut layer);
    }

    // --- 4. Lead nodes (a state `note left of X`) one layer before
    // their nearest target, as dagre's tight ranking puts them: the
    // passes above park every source on layer 0. Only a source moves, so
    // no other node's layer depends on it; through a rank-only hub the
    // target is the nearest member the hub ranks below it.
    // A rank-only group linking into the node (a transition into its
    // composite) only keeps it below the group's sources: the floor.
    for &v in g.lead_nodes.iter().filter(|&&v| v < n) {
        let into = |u: usize| adj[u].iter().any(|&(w, ei)| w == v && !back[ei]);
        let has_in = (0..n).any(into);
        let floor = (n..nx).filter(|&h| into(h)).map(|h| layer[h]).max();
        let reach = |w: usize| -> Option<usize> {
            if w < n {
                Some(layer[w])
            } else {
                adj[w].iter().map(|&(t, _)| layer[t]).min()
            }
        };
        let to = adj[v]
            .iter()
            .filter(|&&(_, ei)| !back[ei])
            .filter_map(|&(w, _)| reach(w))
            .min();
        if let Some(to) = to.filter(|&t| !has_in && t > layer[v] + 1) {
            layer[v] = (to - 1).max(floor.unwrap_or(0));
        }
    }

    layer.truncate(n);
    Ranking {
        back,
        group_eid,
        layer,
        blocks: None,
    }
}

/// A clustered layout graph's layering as blocks and as it was before
/// blocks, side by side ([`layerings`]).
pub(crate) struct Layerings {
    /// Each node's layer in the block layering.
    pub(crate) block: Vec<usize>,
    /// Each node's layer in the layering before blocks.
    pub(crate) before: Vec<usize>,
    /// The edges that run against the flow in the block layering.
    pub(crate) upstream: Vec<usize>,
    /// The levels the lane rule dissolved (`None`: the top level).
    pub(crate) lanes: Vec<Option<usize>>,
    /// Each node's chain of blocks in the block layering, outermost
    /// first (its subgraph path without the dissolved subgraphs).
    pub(crate) paths: Vec<Vec<usize>>,
}

/// Both layerings of the clustered layout graph `g` ([`rank`] with and
/// without `links`), for the reports and tests that compare them: the
/// inputs whose rows the block layout leaves as they were are judged by
/// "not worse than before".
pub(crate) fn layerings(
    g: &Graph,
    node_cluster: &[Vec<usize>],
    rank_only: &[RankGroup],
    links: &BlockLinks,
) -> Layerings {
    let walk = if g.nodes.len() + g.edges.len() <= WALK_MAX_ELEMENTS {
        Walk::Fewest
    } else {
        Walk::Source
    };
    let drawn = rank_only.len();
    let before = rank(g, rank_only, drawn, Some(node_cluster), walk, None);
    let block = rank(g, rank_only, drawn, Some(node_cluster), walk, Some(links));
    let upstream = (0..g.edges.len())
        .filter(|&ei| g.edges[ei].from != g.edges[ei].to && block.back[ei])
        .collect();
    let blocks = block.blocks.unwrap_or_default();
    Layerings {
        block: block.layer,
        before: before.layer,
        upstream,
        lanes: blocks.fired,
        paths: blocks.paths,
    }
}

/// A strongly connected component of a level's units is ordered exactly
/// (the fewest drawn links against the flow, by a programme over its
/// subsets) up to this many units: 4 096 subsets of 12 x 12 steps each.
/// A larger one keeps the walk's order.
const BLOCK_ORDER_EXACT_MAX: usize = 12;

/// The lane rule's share, as a fraction: a level whose every stacking
/// order turns MORE than this share of its drawn links (and in which a
/// path enters one block twice) keeps its blocks side by side.
const LANE_SHARE: (usize, usize) = (1, 3);

/// One link of the block layering between two nodes: a real edge or a
/// stand-in (`weight` 1, also an invisible one: the author asked for
/// that order; 0 for a note card's link, which is never worth turning
/// another link for), or one `from x to` pair of a rank-only group
/// (`weight` 0).
struct BlockLink {
    a: usize,
    b: usize,
    weight: usize,
    /// A cycle breaker of [`rank`]'s walk (a raw mark).
    raw: bool,
}

/// Block layering: every subgraph is ONE unit of the level round it.
/// Returns each node's layer and what the placement needs with it.
///
/// A level is the top level or a subgraph with members; its units are
/// its own nodes and its child subgraphs (blocks), in declaration order
/// ([`Tree`]). Every link belongs to the level of the lowest common
/// ancestor of its two ends and joins two units there. Deepest level
/// first:
///
/// - **Raw marks** (`raw`, [`rank`]'s cycle breakers) are kept only
///   where they break a cycle of their own level: the link's two units
///   lie in one strongly connected component of the level's links as
///   written. A kept mark is used reversed. Every other mark is dropped:
///   the node cycle it broke leaves its block and is broken once, at the
///   level of the frames it crosses (kept, the cycle would cost two
///   edges against the flow).
/// - **Block order**: two depth-first walks over the units — from the
///   units nothing links into first, and in plain declaration order,
///   each following a unit's links in declaration order of their targets
///   — turn the links into a unit still on the walk's path; the walk
///   that turns fewer drawn links is taken, the first on a tie. Per
///   strongly connected component of up to [`BLOCK_ORDER_EXACT_MAX`]
///   units the exact minimum `M` is known (ties to the earliest
///   declaration order), and the component takes that order instead when
///   the walk turns more than `2 * M + 1` of its drawn links: the
///   author's order stands until it costs over twice the minimum.
/// - **Lane rule** ([`LANE_SHARE`]): where every order of the units
///   turns more than a third of the level's drawn links as written AND
///   some path enters one child block twice, the level's child blocks
///   are dissolved for layering — their members and their own child
///   blocks become units of this level — and the level is solved again:
///   two swimlanes that hand a job back and forth stay side by side,
///   every arrow forward. A there-and-back exchange meets the first test
///   only and is stacked.
/// - **Rows** ([`level_rows`]): longest path, a unit as deep as its own
///   rows; a turned link constrains the other way round.
///
/// A node's layer is its own start plus the starts of the blocks round
/// it. So, at every level the lane rule left alone: a block's members
/// fill consecutive layers, two units a link joins never share a layer,
/// and sibling blocks never interleave.
///
/// `exact` is off past [`WALK_MAX_ELEMENTS`]: no component is solved.
/// Note cards are members of their composite and take a row there like
/// any member; a lead card ([`Graph::lead_nodes`]) then sits one row
/// before its target inside its level.
fn block_rank(
    g: &Graph,
    nc: &[Vec<usize>],
    rank_only: &[RankGroup],
    group_eid: &[usize],
    raw: &[bool],
    blocks: &BlockLinks,
    exact: bool,
) -> (Vec<usize>, BlockRanking) {
    let n = g.nodes.len();
    let ns = g.subgraphs.len();
    let card = |v: usize| {
        let node = &g.nodes[v];
        crate::style::is_note_card(&node.id, node.shape, &node.style)
    };
    let mut links: Vec<BlockLink> = Vec::new();
    for (ei, e) in g.edges.iter().enumerate() {
        if e.from != e.to && !blocks.flagged_edge(ei) {
            links.push(BlockLink {
                a: e.from,
                b: e.to,
                weight: usize::from(!card(e.from) && !card(e.to)),
                raw: raw[ei],
            });
        }
    }
    // The drawn links end here: the lane rule's path test reads them.
    let drawn_links = links.len();
    for (k, (from, to)) in rank_only.iter().enumerate() {
        if blocks.flagged_group(k) {
            continue;
        }
        let nf = from.len();
        for (fi, &a) in from.iter().enumerate() {
            for (ti, &b) in to.iter().enumerate() {
                if a != b {
                    let raw = raw[group_eid[k] + fi] || raw[group_eid[k] + nf + ti];
                    links.push(BlockLink {
                        a,
                        b,
                        weight: 0,
                        raw,
                    });
                }
            }
        }
    }

    let mut paths: Vec<Vec<usize>> = nc.to_vec();
    let mut tree = Tree::new(&paths, ns);
    // Per level of `tree`, its links.
    let bucket = |tree: &Tree| -> Vec<Vec<usize>> {
        let mut by: Vec<Vec<usize>> = vec![Vec::new(); tree.levels.len()];
        for (k, l) in links.iter().enumerate() {
            by[tree.join(l.a, l.b).0].push(k);
        }
        by
    };
    let mut by_level = bucket(&tree);
    // Deepest level first, the top level last. A level the lane rule
    // dissolves has been solved by then, and is no level afterwards.
    let mut order: Vec<(usize, usize)> = tree
        .levels
        .iter()
        .filter_map(|l| l.sub.map(|s| (l.depth, s)))
        .collect();
    order.sort_by_key(|&(depth, s)| (std::cmp::Reverse(depth), s));
    let order: Vec<Option<usize>> = order
        .into_iter()
        .map(|(_, s)| Some(s))
        .chain([None])
        .collect();

    let mut rows = vec![1usize; ns];
    let mut start_node = vec![0usize; n];
    let mut start_sub = vec![0usize; ns];
    let mut fired: Vec<Option<usize>> = Vec::new();
    // Unit index within the level being solved.
    let mut at_node = vec![usize::MAX; n];
    let mut at_sub = vec![usize::MAX; ns];
    // The node graph of the drawn links with every raw mark reversed
    // (acyclic), in topological order: built when the lane rule first
    // asks for it.
    let mut dag: Option<(Vec<usize>, Vec<Vec<usize>>)> = None;
    for lvl in order {
        loop {
            let li = match lvl {
                None => 0,
                Some(s) => match tree.level_of_sub[s] {
                    Some(li) => li,
                    None => break,
                },
            };
            let units: Vec<LevelUnit> = tree.levels[li].units.clone();
            let m = units.len();
            for (i, &u) in units.iter().enumerate() {
                match u {
                    LevelUnit::Node(v) => at_node[v] = i,
                    LevelUnit::Block(s) => at_sub[s] = i,
                }
            }
            let at = |u: LevelUnit| match u {
                LevelUnit::Node(v) => at_node[v],
                LevelUnit::Block(s) => at_sub[s],
            };
            // The level's links as written, between unit indices.
            let written: Vec<(usize, usize)> = by_level[li]
                .iter()
                .map(|&k| {
                    let (_, ua, ub) = tree.join(links[k].a, links[k].b);
                    (at(ua), at(ub))
                })
                .collect();
            let weight = |i: usize| links[by_level[li][i]].weight;
            let comp_w = components(m, &written);
            // Kept marks reversed, dropped ones as written.
            let kept = |i: usize| {
                links[by_level[li][i]].raw && comp_w[written[i].0] == comp_w[written[i].1]
            };
            let oriented: Vec<(usize, usize)> = (0..written.len())
                .map(|i| {
                    let (x, y) = written[i];
                    if kept(i) {
                        (y, x)
                    } else {
                        (x, y)
                    }
                })
                .collect();
            // Parallel links are one arc with their summed weight.
            let mut arcs: BTreeMap<(usize, usize), usize> = BTreeMap::new();
            for (i, &xy) in oriented.iter().enumerate() {
                *arcs.entry(xy).or_insert(0) += weight(i);
            }
            let arcs: Vec<((usize, usize), usize)> = arcs.into_iter().collect();
            let arc_of = |xy: (usize, usize)| {
                arcs.binary_search_by_key(&xy, |a| a.0)
                    .expect("every oriented link has its arc")
            };
            let is_block: Vec<bool> = units
                .iter()
                .map(|u| matches!(u, LevelUnit::Block(_)))
                .collect();
            let mut turned = block_order(&is_block, &arcs, exact);

            // Lane rule.
            let mut sizes_w = vec![0usize; m];
            for &c in &comp_w {
                sizes_w[c] += 1;
            }
            let has_block = is_block.iter().any(|&b| b);
            if has_block && sizes_w.iter().any(|&k| k >= 2) {
                let total: usize = (0..written.len()).map(weight).sum();
                // The least number of drawn links, as written, any order
                // turns: exact per component where it is small enough,
                // else what the order found above turns.
                let mut least = 0usize;
                for c in (0..m).filter(|&c| sizes_w[c] >= 2) {
                    let inside = |i: usize| comp_w[written[i].0] == c && comp_w[written[i].1] == c;
                    if exact && sizes_w[c] <= BLOCK_ORDER_EXACT_MAX {
                        let members: Vec<usize> = (0..m).filter(|&u| comp_w[u] == c).collect();
                        let local = |u: usize| members.binary_search(&u).unwrap_or(0);
                        let mut w = vec![vec![0u64; members.len()]; members.len()];
                        for i in (0..written.len()).filter(|&i| inside(i)) {
                            w[local(written[i].0)][local(written[i].1)] += weight(i) as u64;
                        }
                        least += least_turned(&w).0 as usize;
                    } else {
                        least += (0..written.len())
                            .filter(|&i| inside(i) && (kept(i) != turned[arc_of(oriented[i])]))
                            .map(weight)
                            .sum::<usize>();
                    }
                }
                if LANE_SHARE.1 * least > LANE_SHARE.0 * total {
                    let (topo, out) = dag.get_or_insert_with(|| {
                        let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
                        let mut indeg = vec![0usize; n];
                        for (k, l) in links[..drawn_links].iter().enumerate() {
                            if l.weight == 0 {
                                continue;
                            }
                            let (a, b) = if l.raw { (l.b, l.a) } else { (l.a, l.b) };
                            out[a].push(k);
                            indeg[b] += 1;
                        }
                        let mut topo: Vec<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
                        let mut i = 0;
                        while i < topo.len() {
                            let u = topo[i];
                            i += 1;
                            for &k in &out[u] {
                                let l = &links[k];
                                let v = if l.raw { l.a } else { l.b };
                                indeg[v] -= 1;
                                if indeg[v] == 0 {
                                    topo.push(v);
                                }
                            }
                        }
                        (topo, out)
                    });
                    // Does a path enter one child block twice through
                    // drawn links of this level?
                    let twice = units.iter().any(|&block| {
                        if !matches!(block, LevelUnit::Block(_)) {
                            return false;
                        }
                        let mut best = vec![0u8; n];
                        for &u in topo.iter() {
                            for &k in &out[u] {
                                let l = &links[k];
                                let v = if l.raw { l.a } else { l.b };
                                let (lv, cu, cv) = tree.join(u, v);
                                let enters = lv == li && cv == block && cu != cv;
                                let reach = best[u] + u8::from(enters);
                                if reach >= 2 {
                                    return true;
                                }
                                best[v] = best[v].max(reach);
                            }
                        }
                        false
                    });
                    if twice {
                        let mut gone = vec![false; ns];
                        for &u in &units {
                            match u {
                                LevelUnit::Node(v) => at_node[v] = usize::MAX,
                                LevelUnit::Block(s) => {
                                    at_sub[s] = usize::MAX;
                                    gone[s] = true;
                                }
                            }
                        }
                        for p in paths.iter_mut() {
                            p.retain(|&c| !gone[c]);
                        }
                        tree = Tree::new(&paths, ns);
                        by_level = bucket(&tree);
                        fired.push(lvl);
                        continue;
                    }
                }
            }

            // Rows: a link not turned puts its target after its source.
            let mut cons: Vec<(usize, usize)> = oriented
                .iter()
                .map(|&(x, y)| {
                    if turned[arc_of((x, y))] {
                        (y, x)
                    } else {
                        (x, y)
                    }
                })
                .collect();
            cons.sort_unstable();
            cons.dedup();
            turned.clear();
            let unit_rows: Vec<usize> = units
                .iter()
                .map(|&u| match u {
                    LevelUnit::Node(_) => 1,
                    LevelUnit::Block(s) => rows[s],
                })
                .collect();
            let mut start = level_rows(&unit_rows, &is_block, &mut cons);
            // A lead card one row before its target inside its level.
            for (i, &u) in units.iter().enumerate() {
                let LevelUnit::Node(v) = u else { continue };
                if !g.lead_nodes.contains(&v) || cons.iter().any(|&(_, j)| j == i) {
                    continue;
                }
                let to = cons
                    .iter()
                    .filter(|&&(a, _)| a == i)
                    .map(|&(_, j)| start[j])
                    .min();
                if let Some(to) = to.filter(|&t| t > start[i] + 1) {
                    start[i] = to - 1;
                }
            }
            for (i, &u) in units.iter().enumerate() {
                match u {
                    LevelUnit::Node(v) => {
                        start_node[v] = start[i];
                        at_node[v] = usize::MAX;
                    }
                    LevelUnit::Block(s) => {
                        start_sub[s] = start[i];
                        at_sub[s] = usize::MAX;
                    }
                }
            }
            if let Some(s) = lvl {
                rows[s] = (0..m)
                    .map(|i| start[i] + unit_rows[i])
                    .max()
                    .unwrap_or(1)
                    .max(1);
            }
            break;
        }
    }
    let layer: Vec<usize> = (0..n)
        .map(|v| start_node[v] + paths[v].iter().map(|&c| start_sub[c]).sum::<usize>())
        .collect();
    let flagged = (0..g.edges.len())
        .map(|ei| blocks.flagged_edge(ei))
        .collect();
    (
        layer,
        BlockRanking {
            flagged,
            paths,
            fired,
        },
    )
}

/// Which arcs of a level's unit graph run against the flow (block
/// order, see [`block_rank`]): `arcs` are `((from, to), drawn weight)`
/// between the units (`is_block` per unit, in declaration order),
/// sorted. Two walks, the one turning less drawn weight taken (the walk
/// from the sources on a tie); then, per strongly connected component
/// solved exactly (`exact`, up to [`BLOCK_ORDER_EXACT_MAX`] units), its
/// least-turning order instead where the walk turns more than
/// `2 * M + 1` there. The arcs left are acyclic: a walk turns every arc
/// into a unit on its path, an arc between two components lies on no
/// cycle, and a component that takes its exact order is a line.
///
/// A plain node nothing links into starts neither walk. It sits on the
/// first row wherever it is declared, so its place in the file must not
/// order the blocks below it: with the walk started at the first such
/// node, two actors declared the other way round stacked the public
/// fixture's blocks the other way round (the shared services below the
/// clusters they feed). The walk from the sources starts at what those
/// nodes link to — the unit most of their drawn links enter first, then
/// declaration order — and then at the blocks nothing links into; the
/// declared walk leaves those nodes for last.
fn block_order(is_block: &[bool], arcs: &[((usize, usize), usize)], exact: bool) -> Vec<bool> {
    let m = is_block.len();
    let mut out: Vec<Vec<(usize, usize)>> = vec![Vec::new(); m];
    let mut indeg = vec![0usize; m];
    for (id, &((x, y), _)) in arcs.iter().enumerate() {
        out[x].push((y, id));
        indeg[y] += 1;
    }
    let walk = |roots: &mut dyn Iterator<Item = usize>| -> Vec<bool> {
        let mut state = vec![0u8; m]; // 0 unvisited, 1 on the path, 2 done
        let mut turned = vec![false; arcs.len()];
        for s in roots {
            if state[s] != 0 {
                continue;
            }
            state[s] = 1;
            let mut stack: Vec<(usize, usize)> = vec![(s, 0)];
            while let Some(&(u, ci)) = stack.last() {
                let Some(&(v, id)) = out[u].get(ci) else {
                    state[u] = 2;
                    stack.pop();
                    continue;
                };
                if let Some(top) = stack.last_mut() {
                    top.1 += 1;
                }
                match state[v] {
                    0 => {
                        state[v] = 1;
                        stack.push((v, 0));
                    }
                    1 => turned[id] = true,
                    _ => {}
                }
            }
        }
        turned
    };
    let cost = |turned: &[bool]| -> usize {
        arcs.iter()
            .zip(turned)
            .filter(|(_, &t)| t)
            .map(|(a, _)| a.1)
            .sum()
    };
    let loose = |u: usize| indeg[u] == 0 && !is_block[u];
    // Per unit: the drawn links into it from loose nodes.
    let mut fed = vec![0usize; m];
    let mut entered = vec![false; m];
    for u in (0..m).filter(|&u| loose(u)) {
        for &(v, id) in &out[u] {
            fed[v] += arcs[id].1;
            entered[v] = true;
        }
    }
    let mut entries: Vec<usize> = (0..m).filter(|&u| entered[u]).collect();
    entries.sort_by_key(|&u| (std::cmp::Reverse(fed[u]), u));
    let from_sources = walk(
        &mut entries
            .into_iter()
            .chain((0..m).filter(|&u| indeg[u] == 0 && is_block[u]))
            .chain(0..m),
    );
    let declared = walk(
        &mut (0..m)
            .filter(|&u| !loose(u))
            .chain((0..m).filter(|&u| loose(u))),
    );
    let mut turned = if cost(&declared) < cost(&from_sources) {
        declared
    } else {
        from_sources
    };
    if !exact || !turned.iter().any(|&t| t) {
        return turned;
    }
    // The safety net, per strongly connected component.
    let pairs: Vec<(usize, usize)> = arcs.iter().map(|a| a.0).collect();
    let comp = components(m, &pairs);
    let mut size = vec![0usize; m];
    for &c in &comp {
        size[c] += 1;
    }
    for c in (0..m).filter(|&c| (2..=BLOCK_ORDER_EXACT_MAX).contains(&size[c])) {
        let members: Vec<usize> = (0..m).filter(|&u| comp[u] == c).collect();
        let local = |u: usize| members.binary_search(&u).unwrap_or(0);
        let inside: Vec<usize> = (0..arcs.len())
            .filter(|&id| comp[arcs[id].0 .0] == c && comp[arcs[id].0 .1] == c)
            .collect();
        let walked: usize = inside
            .iter()
            .filter(|&&id| turned[id])
            .map(|&id| arcs[id].1)
            .sum();
        if walked <= 1 {
            continue; // never above 2 * M + 1
        }
        // Drawn weight first; among equals, fewer undrawn arcs turned.
        let big = inside.len() as u64 + 1;
        let mut w = vec![vec![0u64; members.len()]; members.len()];
        for &id in &inside {
            let ((x, y), drawn) = arcs[id];
            w[local(x)][local(y)] += drawn as u64 * big + u64::from(drawn == 0);
        }
        let (least, pos) = least_turned(&w);
        if walked as u64 > 2 * (least / big) + 1 {
            for &id in &inside {
                let (x, y) = arcs[id].0;
                turned[id] = pos[local(x)] > pos[local(y)];
            }
        }
    }
    turned
}

/// The order of `w.len()` units (at most [`BLOCK_ORDER_EXACT_MAX`]) that
/// turns the least weight, `w[i][j]` being the weight of the links
/// `i -> j` (turned when `j` comes before `i`): that weight, and each
/// unit's position. Among the best orders the lexicographically smallest
/// sequence of unit indices. A programme over the subsets placed so far.
fn least_turned(w: &[Vec<u64>]) -> (u64, Vec<usize>) {
    let k = w.len();
    debug_assert!(k <= BLOCK_ORDER_EXACT_MAX);
    let full = (1usize << k) - 1;
    // Placing `j` next turns every link into it from a unit not placed.
    let step = |set: usize, j: usize| -> u64 {
        (0..k)
            .filter(|&i| i != j && set & (1 << i) == 0)
            .map(|i| w[i][j])
            .sum()
    };
    let mut best = vec![0u64; full + 1];
    for set in (0..full).rev() {
        best[set] = (0..k)
            .filter(|&j| set & (1 << j) == 0)
            .map(|j| step(set, j) + best[set | (1 << j)])
            .min()
            .unwrap_or(0);
    }
    let mut pos = vec![0usize; k];
    let mut set = 0usize;
    for place in 0..k {
        let next = (0..k)
            .filter(|&j| set & (1 << j) == 0)
            .find(|&j| step(set, j) + best[set | (1 << j)] == best[set]);
        let Some(j) = next else { break };
        pos[j] = place;
        set |= 1 << j;
    }
    (best[0], pos)
}

/// Strongly connected components of the graph of `m` vertices and
/// `arcs`: each vertex's component, named by its smallest vertex.
/// Kosaraju's two passes, without recursion.
fn components(m: usize, arcs: &[(usize, usize)]) -> Vec<usize> {
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); m];
    let mut into: Vec<Vec<usize>> = vec![Vec::new(); m];
    for &(x, y) in arcs {
        out[x].push(y);
        into[y].push(x);
    }
    // Vertices by finishing time of a depth-first walk.
    let mut seen = vec![false; m];
    let mut finished: Vec<usize> = Vec::with_capacity(m);
    for s in 0..m {
        if seen[s] {
            continue;
        }
        seen[s] = true;
        let mut stack: Vec<(usize, usize)> = vec![(s, 0)];
        while let Some(&(u, ci)) = stack.last() {
            match out[u].get(ci) {
                Some(&v) => {
                    if let Some(top) = stack.last_mut() {
                        top.1 += 1;
                    }
                    if !seen[v] {
                        seen[v] = true;
                        stack.push((v, 0));
                    }
                }
                None => {
                    finished.push(u);
                    stack.pop();
                }
            }
        }
    }
    let mut comp = vec![usize::MAX; m];
    for &s in finished.iter().rev() {
        if comp[s] != usize::MAX {
            continue;
        }
        let mut members = vec![s];
        comp[s] = s;
        let mut i = 0;
        while i < members.len() {
            let u = members[i];
            i += 1;
            for &v in &into[u] {
                if comp[v] == usize::MAX {
                    comp[v] = s;
                    members.push(v);
                }
            }
        }
        let name = members.iter().copied().min().unwrap_or(s);
        for v in members {
            comp[v] = name;
        }
    }
    comp
}

/// Rows of one level (see [`block_rank`]): each unit's first row, by
/// longest path over `cons` (`(i, j)`: unit `j` starts after unit `i`'s
/// last row; `rows[i]` rows per unit), then two refinements repeated in
/// this order until neither changes anything:
///
/// 1. *A source block sits next to what it feeds.* A block nothing
///    constrains starts just above the first unit it constrains (the
///    units it links to and the blocks rule 2 put after it) when that is
///    later, and keeps that start as a floor. Plain source nodes stay on
///    the first row.
/// 2. *No interleaving.* While two blocks `A`, `B` overlap in rows
///    without one holding the other (`start(A) < start(B) < end(A) <
///    end(B)`), `B` starts after `A`: the pair with the earliest
///    `start(A)`, then the earliest `start(B)`, then declaration order.
///    The constraint points from the earlier start to the later, so it
///    closes no cycle, and it is added to `cons`.
///
/// Rule 1 moves a block into its own slack only and rule 2 adds at most
/// one constraint per pair of blocks, so the loop ends within (pairs of
/// blocks + 2) passes.
fn level_rows(rows: &[usize], is_block: &[bool], cons: &mut Vec<(usize, usize)>) -> Vec<usize> {
    let m = rows.len();
    let solve = |cons: &[(usize, usize)], floor: &[usize]| -> Vec<usize> {
        let mut out: Vec<Vec<usize>> = vec![Vec::new(); m];
        let mut indeg = vec![0usize; m];
        for &(i, j) in cons {
            out[i].push(j);
            indeg[j] += 1;
        }
        let mut start = floor.to_vec();
        let mut queue: VecDeque<usize> = (0..m).filter(|&u| indeg[u] == 0).collect();
        let mut done = 0;
        while let Some(u) = queue.pop_front() {
            done += 1;
            for &v in &out[u] {
                start[v] = start[v].max(start[u] + rows[u]);
                indeg[v] -= 1;
                if indeg[v] == 0 {
                    queue.push_back(v);
                }
            }
        }
        debug_assert_eq!(done, m, "the constraints of a level are acyclic");
        start
    };
    let mut floor = vec![0usize; m];
    let mut start = solve(cons, &floor);
    let blocks: Vec<usize> = (0..m).filter(|&u| is_block[u]).collect();
    let pairs = blocks.len() * blocks.len().saturating_sub(1) / 2;
    for pass in 0.. {
        debug_assert!(pass < pairs + 2, "the rows of a level settle");
        if pass >= pairs + 2 {
            break;
        }
        let mut changed = false;
        for &s in &blocks {
            if cons.iter().any(|&(_, j)| j == s) {
                continue;
            }
            let near = cons
                .iter()
                .filter(|&&(i, _)| i == s)
                .map(|&(_, j)| start[j])
                .min();
            if let Some(to) = near.and_then(|t| t.checked_sub(rows[s])) {
                if to > start[s] {
                    floor[s] = to;
                    start = solve(cons, &floor);
                    changed = true;
                }
            }
        }
        loop {
            let end = |u: usize| start[u] + rows[u];
            let pair = blocks
                .iter()
                .flat_map(|&a| blocks.iter().map(move |&b| (a, b)))
                .filter(|&(a, b)| start[a] < start[b] && start[b] < end(a) && end(a) < end(b))
                .min_by_key(|&(a, b)| (start[a], start[b], a, b));
            let Some(pair) = pair else { break };
            if cons.contains(&pair) {
                break;
            }
            cons.push(pair);
            start = solve(cons, &floor);
            changed = true;
        }
        if !changed {
            break;
        }
    }
    start
}

/// Seat every box fed from outside the boxes below its feeders (report
/// §2.9, B8 review r1 #1 and B8b P1). Longest-path layering parks each
/// member nothing ranks into on the first layer: a hub fanning out into
/// boxes then sat level with the edge-less members of the boxes it
/// feeds through one member, and with their members that only link
/// onward, out of the box — its edges struck through their frames to
/// reach the next layer, and the frame spanned the hub's row. Mermaid
/// (dagre's nesting edges from the box's top border) draws the whole
/// box below the hub instead.
///
/// A box's first row is where its feeders put it: the row just below
/// the topmost node outside every box with a link into it (a real
/// edge, an edge to the box's frame through its rank-only stand-in).
/// Each SOURCE member — a member with no link in at all but from a
/// `lead` note card: edge-less, linking onward out of the box, or
/// leading inside it to a member the feeders rank (which then moves a
/// row down: f051's `n8 -> n9`) — ranks no higher than the first row of
/// the outermost box on its path that has feeders, and a box one of
/// those sources links into ranks its own first-row members (no link
/// in from inside it) below that source, so the line leaves the
/// source's row for the next instead of threading the boxes on its
/// row. Inside its outermost box a source may stay above an inner
/// box's own feeders: the rows it keeps lie within the outer frame
/// anyway.
///
/// A node outside the box and the boxes it leads into may move with
/// the seats only where it HANGS below the move: every link into it
/// comes from a node that moved too, so no other line stretches (x7's
/// `nightly job --> mail relay`). A box such a node belongs to follows
/// the seated box down whole, ranked below the moved node linking into
/// it (x1: the hub feeds Alpha and Gamma, Alpha leads into Beta and
/// Beta into Gamma; Gamma, else three rows deep beside Beta, sits one
/// row below it).
///
/// What stays put: a box with no feeder outside every box (dense-td's
/// and dense-lr's boxes fed from other boxes keep their sources on the
/// first layer: seating them there re-laid the drawings, dense-td's
/// drawn crossings 27 % up and its `route()` 40 % slower); a source a
/// cycle breaker reaches from outside the box (as every later layer
/// move leaves a cycle's end, [`NodeLinks`]); a feeder the box's own
/// sources reach, or a source the box it leads into leads back to (each
/// would close a cycle); and every box whose seating would move a node
/// outside it that does not hang below the move — it re-laid the rest
/// of the drawing for one box's rows (dense-lr: 25 of its 52 nodes a
/// layer down, six of them also ranked by nodes that stay).
///
/// Links with an id from `stacks_from` on (narrowing's stackings and
/// wraps) and cycle breakers neither feed a box nor make a member less
/// of a source: a stacking hub links every member of the lower box, so
/// counting it left the box's edge-less members level with the hub
/// again once narrowing stacked or wrapped it. Stackings still rank
/// the box below its sibling: they are links like any other in the
/// layering. `layer` (real nodes `0..nc.len()`, then rank-only hubs)
/// is relaid by longest path with those floors; `lead` cards are placed
/// after this, one layer before their target.
fn seat_box_sources(
    nc: &[Vec<usize>],
    lead: &[usize],
    adj: &[Vec<(usize, usize)>],
    back: &[bool],
    stacks_from: usize,
    layer: &mut [usize],
) {
    let n = nc.len();
    let nx = adj.len();
    let seats = |ei: usize| !back[ei] && ei < stacks_from;
    let mut is_lead = vec![false; n];
    for &l in lead.iter().filter(|&&l| l < n) {
        is_lead[l] = true;
    }
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); nx];
    // Cycle breakers into each node (stackings left out).
    let mut back_in: Vec<Vec<usize>> = vec![Vec::new(); nx];
    for (u, links) in adj.iter().enumerate() {
        for &(v, ei) in links {
            if seats(ei) && u != v {
                preds[v].push(u);
            } else if back[ei] && ei < stacks_from && u != v {
                back_in[v].push(u);
            }
        }
    }
    // A feeder of box `c`: a real node outside every box (no note
    // card), or a hub one of whose sources is.
    let top_level = |f: usize| f < n && !is_lead[f] && nc[f].is_empty();
    let boxes = nc.iter().flatten().map(|&c| c + 1).max().unwrap_or(0);
    let mut feeders: Vec<Vec<usize>> = vec![Vec::new(); boxes];
    for v in (0..n).filter(|&v| !nc[v].is_empty()) {
        for &x in &preds[v] {
            let feeds = if x < n {
                top_level(x)
            } else {
                preds[x].iter().any(|&f| top_level(f))
            };
            for &c in nc[v].iter().filter(|_| feeds) {
                if !feeders[c].contains(&x) {
                    feeders[c].push(x);
                }
            }
        }
    }
    let inside = |u: usize, c: usize| u < n && nc[u].contains(&c);
    let mut seat: Vec<Option<usize>> = (0..n)
        .map(|v| {
            if is_lead[v] || preds[v].iter().any(|&p| p >= n || !is_lead[p]) {
                return None;
            }
            let c = nc[v].iter().copied().find(|&c| !feeders[c].is_empty())?;
            let quiet = !back_in[v].iter().any(|&p| !inside(p, c));
            quiet.then_some(c)
        })
        .collect();
    // The layering graph plus two nodes per box: a min-node (`nx + c`)
    // its feeders link into and that links to the sources it seats, and
    // a max-node (`nx + boxes + c`) the seated sources of other boxes
    // link into and that links to the box's first-row members.
    let top = |c: usize| nx + c;
    let after = |c: usize| nx + boxes + c;
    let total = nx + 2 * boxes;
    let mut base: Vec<Vec<usize>> = vec![Vec::new(); total];
    for (u, links) in adj.iter().enumerate() {
        for &(v, ei) in links {
            if !back[ei] {
                base[u].push(v);
            }
        }
    }
    // The links a seat's move travels along (stackings left out: the
    // box stacked below a seated one moves with it, as it must).
    let mut onward: Vec<Vec<usize>> = vec![Vec::new(); total];
    for (u, links) in adj.iter().enumerate() {
        onward[u].extend(links.iter().filter(|&&(_, ei)| seats(ei)).map(|&(v, _)| v));
    }
    for y in 0..boxes {
        for u in (0..n).filter(|&u| inside(u, y) && !is_lead[u]) {
            if !preds[u].iter().any(|&p| inside(p, y)) {
                base[after(y)].push(u);
            }
        }
    }
    // Longest path over the layering graph with the seats in `seat`, a
    // min-node taking its topmost feeder's row; also each seated box's
    // links into the boxes that follow it. `None` if a cycle is left.
    let relay = |seat: &[Option<usize>],
                 chain: &[(usize, usize, usize)]|
     -> Option<(Vec<usize>, Vec<Vec<usize>>)> {
        let mut out = base.clone();
        // Links kept only where they close no cycle: (from, min- or
        // max-node).
        let mut cond: Vec<(usize, usize)> = Vec::new();
        let mut leads: Vec<Vec<usize>> = vec![Vec::new(); boxes];
        for &(c, u, y) in chain {
            if !cond.contains(&(u, after(y))) {
                cond.push((u, after(y)));
            }
            if !leads[c].contains(&y) {
                leads[c].push(y);
            }
        }
        for (v, s) in seat.iter().enumerate() {
            let Some(c) = *s else { continue };
            out[top(c)].push(v);
            for &x in &feeders[c] {
                if !cond.contains(&(x, top(c))) {
                    cond.push((x, top(c)));
                }
            }
            for &(t, ei) in &adj[v] {
                if t >= n || !seats(ei) {
                    continue;
                }
                if let Some(&y) = nc[t].iter().find(|&&y| !nc[v].contains(&y)) {
                    if !cond.contains(&(v, after(y))) {
                        cond.push((v, after(y)));
                    }
                    if !leads[c].contains(&y) {
                        leads[c].push(y);
                    }
                }
            }
        }
        for &(x, w) in &cond {
            out[x].push(w);
        }
        // A conditional link from a node its own target reaches (a
        // feeder hanging below the box's sources, a source the box it
        // leads into leads back to) is dropped: every cycle runs through
        // one, so the rest is acyclic.
        let mut targets: Vec<usize> = cond.iter().map(|&(_, w)| w).collect();
        targets.sort_unstable();
        targets.dedup();
        let mut drop: Vec<(usize, usize)> = Vec::new();
        for w in targets {
            let mut seen = vec![false; total];
            let mut stack = vec![w];
            while let Some(u) = stack.pop() {
                if std::mem::replace(&mut seen[u], true) {
                    continue;
                }
                stack.extend(out[u].iter().copied());
            }
            drop.extend(cond.iter().filter(|&&(x, cw)| cw == w && seen[x]));
        }
        for (x, w) in drop {
            if let Some(at) = out[x].iter().position(|&o| o == w) {
                out[x].remove(at);
            }
        }
        let step = |u: usize| usize::from(u < n);
        let mut indeg = vec![0usize; total];
        for links in &out {
            for &v in links {
                indeg[v] += 1;
            }
        }
        let mut value = vec![0usize; total];
        let mut row: Vec<Option<usize>> = vec![None; boxes];
        let mut q: VecDeque<usize> = (0..total).filter(|&v| indeg[v] == 0).collect();
        let mut done = 0;
        let is_top = |u: usize| (nx..nx + boxes).contains(&u);
        while let Some(u) = q.pop_front() {
            done += 1;
            if is_top(u) {
                value[u] = row[u - nx].unwrap_or(0);
            }
            for &v in &out[u] {
                let at = value[u] + step(u);
                if is_top(v) {
                    let r = &mut row[v - nx];
                    *r = Some(r.map_or(at, |r| r.min(at)));
                } else if at > value[v] {
                    value[v] = at;
                }
                indeg[v] -= 1;
                if indeg[v] == 0 {
                    q.push_back(v);
                }
            }
        }
        (done == total).then_some((value, leads))
    };
    // The boxes around each box.
    let mut outer: Vec<Vec<usize>> = vec![Vec::new(); boxes];
    for path in nc {
        for (i, &c) in path.iter().enumerate() {
            for &a in &path[..i] {
                if !outer[c].contains(&a) {
                    outer[c].push(a);
                }
            }
        }
    }
    // Boxes that follow a seated box down a chain of moved members:
    // (seated box, moved member outside the follower, follower).
    let mut chain: Vec<(usize, usize, usize)> = Vec::new();
    // Unseat every box whose seating moves a node outside it and the
    // boxes it leads into, unless that node hangs below the move alone,
    // until none does.
    for _ in 0..=4 * boxes + 2 {
        if seat.iter().all(Option::is_none) {
            return;
        }
        let Some((value, leads)) = relay(&seat, &chain) else {
            return;
        };
        let moved = |u: usize| u < nx && value[u] > layer[u];
        let mut spills = vec![false; boxes];
        let mut grown = false;
        for c in 0..boxes {
            // A member the seats leave above the first row (a source a
            // cycle breaker from outside reaches) keeps the frame on the
            // feeder's row: the seats would move the rest for nothing.
            if (0..n).any(|u| seat[u] == Some(c))
                && (0..n).any(|u| inside(u, c) && value[u] < value[top(c)])
            {
                spills[c] = true;
                continue;
            }
            let mut seen = vec![false; total];
            let mut stack: Vec<usize> = (0..n).filter(|&v| seat[v] == Some(c)).collect();
            if stack.is_empty() {
                continue;
            }
            stack.extend(leads[c].iter().map(|&y| after(y)));
            while let Some(u) = stack.pop() {
                if std::mem::replace(&mut seen[u], true) {
                    continue;
                }
                let own = inside(u, c) || leads[c].iter().any(|&y| inside(u, y));
                if u < n && moved(u) && !own {
                    // A node every link into which moved too hangs below
                    // the move alone: it moves along, no line stretches.
                    // A box it belongs to follows the box down whole.
                    let hangs = preds[u].iter().all(|&p| moved(p) || (p < n && is_lead[p]));
                    let follower = nc[u].iter().copied().find(|&y| !outer[c].contains(&y));
                    match (hangs, follower) {
                        (false, _) => {
                            spills[c] = true;
                            break;
                        }
                        (true, Some(y)) => {
                            for &p in preds[u].iter().filter(|&&p| moved(p) && !inside(p, y)) {
                                if !chain.contains(&(c, p, y)) {
                                    chain.push((c, p, y));
                                    grown = true;
                                }
                            }
                        }
                        (true, None) => {}
                    }
                }
                // Only a node that moved passes a move on.
                if u >= nx || moved(u) || (u < n && seat[u] == Some(c)) {
                    stack.extend(onward[u].iter().copied());
                    if u >= nx {
                        stack.extend(base[u].iter().copied());
                    }
                }
            }
        }
        if !spills.iter().any(|&s| s) && !grown {
            layer.copy_from_slice(&value[..nx]);
            return;
        }
        for s in seat.iter_mut() {
            if s.is_some_and(|c| spills[c]) {
                *s = None;
            }
        }
        chain.retain(|&(c, _, _)| !spills[c]);
    }
}

/// Per real node, the ranking links a layer move has to respect.
struct NodeLinks {
    /// Non-back predecessors (real nodes).
    ins: Vec<Vec<usize>>,
    /// Non-back successors (real nodes).
    outs: Vec<Vec<usize>>,
    /// Never moved: an end of a cycle breaker or self-loop.
    pinned: Vec<bool>,
    /// Deepest layer the rank-only groups allow (a group's `from` side
    /// stays above every `to` member).
    deepest: Vec<usize>,
    /// Shallowest layer they allow (signed: a stagger may open a layer
    /// above the first one).
    shallowest: Vec<isize>,
}

fn node_links(g: &Graph, groups: &[RankGroup], rk: &Ranking, layer: &[usize]) -> NodeLinks {
    let n = g.nodes.len();
    let mut nl = NodeLinks {
        ins: vec![Vec::new(); n],
        outs: vec![Vec::new(); n],
        pinned: vec![false; n],
        deepest: vec![usize::MAX; n],
        shallowest: vec![isize::MIN; n],
    };
    for (from, to) in groups {
        let (Some(lo_to), Some(hi_from)) = (
            to.iter().map(|&v| layer[v]).min(),
            from.iter().map(|&v| layer[v]).max(),
        ) else {
            continue;
        };
        for &v in from {
            nl.deepest[v] = nl.deepest[v].min(lo_to.saturating_sub(1));
        }
        for &v in to {
            nl.shallowest[v] = nl.shallowest[v].max(hi_from as isize + 1);
        }
    }
    for (ei, e) in g.edges.iter().enumerate() {
        if rk.back[ei] || e.from == e.to {
            nl.pinned[e.from] = true;
            nl.pinned[e.to] = true;
        } else {
            nl.outs[e.from].push(e.to);
            nl.ins[e.to].push(e.from);
        }
    }
    nl
}

/// Renumber layers so none is empty, keeping their order (every link
/// keeps pointing down, by the same or fewer layers).
fn compact_layers(mut layer: Vec<usize>) -> Vec<usize> {
    let mut used: Vec<usize> = layer.clone();
    used.sort_unstable();
    used.dedup();
    for l in layer.iter_mut() {
        *l = used.binary_search(l).unwrap_or(0);
    }
    layer
}

/// The fence a block layering puts round later layer moves
/// ([`tighten_sources`], [`stagger_proposals`]): a node never leaves the
/// rows of a block it is in, and no block's rows change.
struct Fence<'a> {
    /// Each node's chain of blocks ([`BlockRanking::paths`]).
    paths: &'a [Vec<usize>],
    /// Per block: its first and last layer.
    span: Vec<(usize, usize)>,
    /// Members per (block, layer).
    count: BTreeMap<(usize, usize), usize>,
}

impl<'a> Fence<'a> {
    fn new(blocks: &'a BlockRanking, layer: &[usize]) -> Fence<'a> {
        let subs = blocks.paths.iter().flatten().max().map_or(0, |&c| c + 1);
        let mut span = vec![(usize::MAX, 0usize); subs];
        let mut count: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        for (v, path) in blocks.paths.iter().enumerate() {
            for &c in path {
                span[c] = (span[c].0.min(layer[v]), span[c].1.max(layer[v]));
                *count.entry((c, layer[v])).or_insert(0) += 1;
            }
        }
        Fence {
            paths: &blocks.paths,
            span,
            count,
        }
    }

    /// Whether node `v` stays within the rows of every block round it
    /// on layer `to`.
    fn holds(&self, v: usize, to: usize) -> bool {
        self.paths[v]
            .iter()
            .all(|&c| self.span[c].0 <= to && to <= self.span[c].1)
    }

    /// Whether node `v` may move from layer `from` to layer `to`: it
    /// stays within its blocks' rows and leaves none of their rows
    /// empty.
    fn allows(&self, v: usize, from: usize, to: usize) -> bool {
        self.holds(v, to)
            && self.paths[v]
                .iter()
                .all(|&c| self.count.get(&(c, from)).copied().unwrap_or(0) > 1)
    }

    /// Record that node `v` moved from layer `from` to layer `to`.
    fn moved(&mut self, v: usize, from: usize, to: usize) {
        for &c in &self.paths[v] {
            if let Some(k) = self.count.get_mut(&(c, from)) {
                *k -= 1;
            }
            *self.count.entry((c, to)).or_insert(0) += 1;
        }
    }

    /// Whether `layer` (each node's new layer) leaves every block the
    /// rows it had.
    fn unchanged(&self, layer: &[usize]) -> bool {
        let mut span = vec![(usize::MAX, 0usize); self.span.len()];
        for (v, path) in self.paths.iter().enumerate() {
            for &c in path {
                span[c] = (span[c].0.min(layer[v]), span[c].1.max(layer[v]));
            }
        }
        span == self.span
    }
}

/// Each movable source one layer above its nearest target, or `None`
/// when no source moves. In block mode a source moves only within the
/// rows of its blocks ([`Fence`]).
fn tighten_sources(
    g: &Graph,
    groups: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
) -> Option<Vec<usize>> {
    let nl = node_links(g, groups, rk, layer);
    let mut fence = rk.blocks.as_ref().map(|b| Fence::new(b, layer));
    let mut out = layer.to_vec();
    let mut changed = false;
    for v in 0..layer.len() {
        if nl.pinned[v] || !nl.ins[v].is_empty() {
            continue;
        }
        // A source's targets all have an in-link, so none of them moves.
        let Some(to) = nl.outs[v].iter().map(|&w| layer[w]).min() else {
            continue;
        };
        let to = (to - 1).min(nl.deepest[v]);
        if to > layer[v] {
            if let Some(f) = fence.as_mut() {
                if !f.allows(v, layer[v], to) {
                    continue;
                }
                f.moved(v, layer[v], to);
            }
            out[v] = to;
            changed = true;
        }
    }
    changed.then(|| compact_layers(out))
}

/// Leaf-staggering proposals for the broadest layer not in `tried`
/// with at least [`STAGGER_MIN_NODES`] real nodes (graphviz
/// `unflatten -l`), returned with that layer:
/// - leaves: every second sink or edge-less node steps down one layer,
///   every second source steps up one;
/// - branches: every second node steps down one layer together with
///   everything it leads to, for rows where few nodes are leaves.
///
/// Every second is counted within each innermost cluster, in the order
/// `lo` drew the layer, so the two rows interleave.
#[allow(clippy::too_many_arguments)] // internal: one bundle of layering state
fn stagger_proposals(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: Option<&[Vec<usize>]>,
    groups: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
    lo: &LayoutResult,
    tried: &[usize],
) -> Option<(usize, Vec<Vec<usize>>)> {
    let n = layer.len();
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    let mut nl = node_links(g, groups, rk, layer);
    // A subgraph that sets a `direction` across the flow asked for one
    // row: its members never step out of it.
    if let Some(nc) = node_cluster {
        let across = |d: Direction| matches!(d, Direction::LR | Direction::RL) != horizontal;
        for v in 0..n {
            if nc[v]
                .iter()
                .any(|&c| g.subgraphs[c].direction.is_some_and(across))
            {
                nl.pinned[v] = true;
            }
        }
    }
    let nlayers = layer.iter().copied().max()? + 1;
    // Breadth estimate per layer: its real nodes and their gaps.
    let mut est = vec![0.0f64; nlayers];
    let mut count = vec![0usize; nlayers];
    for v in 0..n {
        est[layer[v]] += if horizontal { sizes[v].1 } else { sizes[v].0 } + GAP_B;
        count[layer[v]] += 1;
    }
    let li = (0..nlayers)
        .filter(|&li| count[li] >= STAGGER_MIN_NODES && !tried.contains(&li))
        .min_by(|&a, &b| est[b].total_cmp(&est[a]).then(a.cmp(&b)))?;
    // Every second member of each (innermost cluster, kind) bucket, in
    // drawn order.
    let every_second = |kind: &dyn Fn(usize) -> Option<isize>| -> Vec<(usize, isize)> {
        let mut buckets: std::collections::BTreeMap<(Option<usize>, isize), Vec<usize>> =
            std::collections::BTreeMap::new();
        for v in (0..n).filter(|&v| layer[v] == li && !nl.pinned[v]) {
            if let Some(d) = kind(v) {
                let c = node_cluster.and_then(|nc| nc[v].last().copied());
                buckets.entry((c, d)).or_default().push(v);
            }
        }
        let mut picks = Vec::new();
        for ((_, d), mut vs) in buckets {
            vs.sort_by(|&a, &b| lo.nodes[a].b.total_cmp(&lo.nodes[b].b).then(a.cmp(&b)));
            picks.extend(vs.iter().skip(1).step_by(2).map(|&v| (v, d)));
        }
        picks
    };
    let mut out: Vec<Vec<usize>> = Vec::new();
    // A source steps up only within its outermost box's rows: above
    // them it would draw the frame across the row of the node feeding
    // the box ([`seat_box_sources`]; h284's `queue queue` stepped back
    // up beside the hub `search`, whose edges then struck its frame).
    let mut box_rows: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    if let Some(nc) = node_cluster {
        for v in 0..n {
            if let Some(&c) = nc[v].first() {
                let top = box_rows.entry(c).or_insert(layer[v]);
                *top = (*top).min(layer[v]);
            }
        }
    }
    let box_top = |v: usize| -> bool {
        match node_cluster.and_then(|nc| nc[v].first()) {
            Some(c) => box_rows.get(c).is_some_and(|&top| top < layer[v]),
            None => true,
        }
    };
    // Block mode: a leaf steps only within the rows of its blocks. (The
    // first of every bucket stays, so no row of a block empties.)
    let fence = rk.blocks.as_ref().map(|b| Fence::new(b, layer));
    let fenced = |v: usize, d: isize| match (&fence, layer[v].checked_add_signed(d)) {
        (None, _) => true,
        (Some(f), Some(to)) => f.holds(v, to),
        // A fresh layer above the first: outside every block.
        (Some(f), None) => f.paths[v].is_empty(),
    };
    // Leaves: +1 = steps down (sink or edge-less), -1 = steps up.
    let leaves = every_second(&|v| {
        if nl.outs[v].is_empty() && layer[v] < nl.deepest[v] {
            Some(1).filter(|&d| fenced(v, d))
        } else if nl.ins[v].is_empty()
            && !nl.outs[v].is_empty()
            && layer[v] as isize > nl.shallowest[v]
            && box_top(v)
        {
            Some(-1).filter(|&d| fenced(v, d))
        } else {
            None
        }
    });
    if !leaves.is_empty() {
        let mut moved = layer.to_vec();
        // A source on the first layer needs a fresh layer above it.
        if li == 0 && leaves.iter().any(|m| m.1 < 0) {
            for l in moved.iter_mut() {
                *l += 1;
            }
        }
        for &(v, d) in &leaves {
            moved[v] = moved[v].checked_add_signed(d).unwrap_or(0);
        }
        out.push(compact_layers(moved));
    }
    // Branches: the seeds and everything downstream of them (edges and
    // rank-only links) shift one layer down together.
    let seeds = every_second(&|v| (!nl.outs[v].is_empty()).then_some(1));
    if !seeds.is_empty() {
        let mut down = vec![false; n];
        let mut stack: Vec<usize> = seeds.iter().map(|s| s.0).collect();
        while let Some(u) = stack.pop() {
            if std::mem::replace(&mut down[u], true) {
                continue;
            }
            stack.extend(nl.outs[u].iter().copied());
            for (from, to) in groups {
                if from.contains(&u) {
                    stack.extend(to.iter().copied());
                }
            }
        }
        let moved: Vec<usize> = (0..n).map(|v| layer[v] + usize::from(down[v])).collect();
        // A cycle breaker inside the moved set could now point the
        // other way: skip rather than re-break cycles. In block mode the
        // shift must leave every block its rows.
        if !(0..n).any(|v| down[v] && nl.pinned[v])
            && fence.as_ref().map_or(true, |f| f.unchanged(&moved))
        {
            out.push(compact_layers(moved));
        }
    }
    Some((li, out))
}

/// One laid-out candidate layering.
struct Placement {
    lo: LayoutResult,
    groups: EdgeGroups,
    /// [`drawn_crossings`] of the kept ordering.
    crossings: usize,
    /// [`node_hits`] of the drawn edge skeletons.
    hits: usize,
    /// Ordering key per subgraph `(id, key)`, by id ([`cluster_keys`]).
    ckeys: Vec<(usize, f64)>,
    /// Estimated drawn breadth span ([`drawn_breadth`]).
    ext: (f64, f64),
    /// The depth the ribbon tests read (G12): the judged layer lines'
    /// (see `place`'s `layer_lines`).
    depth: f64,
    /// A block layout's final order ([`OrderSeed`]).
    order: Option<OrderSeed>,
}

/// Steps 3 to 5 for one layering: dummy chains, ordering and
/// coordinates. `layer` holds one layer per real node and must keep
/// every non-back ranking link pointing down by at least its weight.
/// `seed`: the order a block layout's later placement starts from
/// ([`BlockOrder::run`]).
#[allow(clippy::too_many_arguments)] // internal: one bundle of layering state
fn place(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: Option<&[Vec<usize>]>,
    rank_only: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
    opts: Opts,
    seed: Option<&OrderSeed>,
) -> Placement {
    let n = g.nodes.len();
    let back = &rk.back;
    let group_eid = &rk.group_eid;
    // Each box's title-strip room: one line, plus any `<br/>` lines.
    let header: Vec<f64> = g
        .subgraphs
        .iter()
        .map(|s| CLUSTER_HEADER + title_extra(&s.title))
        .collect();
    // --- Virtual (dummy) nodes over an AUGMENTED graph. An edge that
    // spans >1 layer is broken into a chain of dummies (one per crossed
    // layer) so it takes part in ordering and RESERVES a routing channel
    // — real nodes spread around it instead of it cutting straight
    // across them. Augmented index space: real nodes `0..n`, dummies
    // `n..`. `absize`/`alsize` are the breadth/layer sizes per aug node.
    const DUMMY_B: f64 = 16.0;
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    let mut alayer = layer.to_vec();
    let mut absize: Vec<f64> = (0..n)
        .map(|v| if horizontal { sizes[v].1 } else { sizes[v].0 })
        .collect();
    let mut alsize: Vec<f64> = (0..n)
        .map(|v| if horizontal { sizes[v].0 } else { sizes[v].1 })
        .collect();
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); n];
    // Per original edge: dummy chain in the edge's own from→to order.
    let mut edge_chain: Vec<Vec<usize>> = vec![Vec::new(); g.edges.len()];
    // Adjacent pairs linked so far only by a 2-cycle's return edge,
    // which stands in for the first forward edge of the pair.
    let mut return_only: std::collections::HashSet<(usize, usize)> = Default::default();
    // Block mode: a flagged stand-in is no link (no channel, no pull).
    let flagged = |ei: usize| rk.blocks.as_ref().is_some_and(|b| b.flagged[ei]);
    let block_paths = rk.blocks.as_ref().map(|b| b.paths.as_slice());
    // A back-edge without a channel: every one before blocks; in a
    // block layout only one between two plain nodes of a level
    // ([`keeps_lane`]) and a note card's link — the others run up a
    // channel of their own, in the model the ordering minimises.
    let no_channel = |ei: usize| {
        let e = &g.edges[ei];
        let card = |v: usize| {
            let node = &g.nodes[v];
            crate::style::is_note_card(&node.id, node.shape, &node.style)
        };
        back[ei] && (keeps_lane(block_paths, e.from, e.to) || card(e.from) || card(e.to))
    };
    for (ei, e) in g.edges.iter().enumerate() {
        if e.from == e.to || flagged(ei) {
            continue; // self-loop — no layered path
        }
        // Order the endpoints by layer (lo below hi) to build the chain.
        let ascending = alayer[e.from] <= alayer[e.to];
        let (lo, hi) = if ascending {
            (e.from, e.to)
        } else {
            (e.to, e.from)
        };
        let (llo, lhi) = (alayer[lo], alayer[hi]);
        if lhi <= llo + 1 {
            // A 2-cycle's return edge runs beside its twin across the
            // gap: one link between the pair, not a second pull —
            // whichever of the two is written first. A doubled link
            // made BK's median land on the pair and pulled the node off
            // the middle of its fan: lr's Service Layer sat level with
            // Redis Cache instead of centred on Database / Redis when
            // `CACHE --> SVC` came first (report Q8). Parallel FORWARD
            // edges keep one link each: a twin pair then runs straight
            // down its own column, which the twin curves and their
            // labels are drawn for (deduplicated, a choice's
            // `low`/`medium` twins slanted across the fan).
            if llo < lhi {
                if back[ei] {
                    if !succs[lo].contains(&hi) {
                        succs[lo].push(hi);
                        preds[hi].push(lo);
                        return_only.insert((lo, hi));
                    }
                } else if !return_only.remove(&(lo, hi)) {
                    succs[lo].push(hi);
                    preds[hi].push(lo);
                }
            }
            continue; // same or adjacent layer — no dummies
        }
        // Invisible links (ranking-only) and back-edges get NO channel:
        // the former must not inflate the canvas, and a back-edge routed
        // up the middle would cross every forward edge — it keeps the
        // sideways bow instead. Both still influence ordering directly.
        if matches!(e.kind, EdgeKind::Invisible) || no_channel(ei) {
            succs[lo].push(hi);
            preds[hi].push(lo);
            continue;
        }
        let mut prev = lo;
        let mut chain = Vec::with_capacity(lhi - llo - 1);
        for lay in (llo + 1)..lhi {
            let d = alayer.len();
            alayer.push(lay);
            absize.push(DUMMY_B);
            alsize.push(0.0);
            preds.push(Vec::new());
            succs.push(Vec::new());
            succs[prev].push(d);
            preds[d].push(prev);
            chain.push(d);
            prev = d;
        }
        succs[prev].push(hi);
        preds[hi].push(prev);
        if !ascending {
            chain.reverse(); // store low→high chain in from→to order
        }
        // Reserve space for a long edge's label at the middle dummy of
        // its chain: sizing that dummy makes it a real obstacle, so the
        // labels of converging edges spread apart instead of piling up
        // (dagre models edge labels as nodes for exactly this reason).
        // (A block layout picks the dummy once the chain's subgraph
        // paths are known, below.)
        if let (Some(lbl), false, None) = (&e.label, chain.is_empty(), block_paths) {
            let mid = chain[chain.len() / 2];
            let lw = text_width(lbl) + LABEL_PAD_W;
            let lh = LINE_H * line_count(lbl) as f64 + 6.0;
            if horizontal {
                absize[mid] = absize[mid].max(lh);
                alsize[mid] = alsize[mid].max(lw);
            } else {
                absize[mid] = absize[mid].max(lw);
                alsize[mid] = alsize[mid].max(lh);
            }
        }
        edge_chain[ei] = chain;
    }
    // Rank-only groups order like invisible links (no chain, no
    // channel), each `from` and `to` member pulled by one partner — a
    // pair a real edge already joins pulls through that edge.
    let joined: std::collections::HashSet<(usize, usize)> = if rank_only.is_empty() {
        Default::default()
    } else {
        g.edges.iter().map(|e| (e.from, e.to)).collect()
    };
    for (k, (from, to)) in rank_only.iter().enumerate() {
        let (nf, nt) = (from.len(), to.len());
        if nf == 0 || nt == 0 {
            continue;
        }
        let m = nf.max(nt);
        for i in 0..m {
            let (fi, ti) = (i * nf / m, i * nt / m);
            let (a, b) = (from[fi], to[ti]);
            let cut = back[group_eid[k] + fi] || back[group_eid[k] + nf + ti];
            if a != b && !cut && alayer[a] < alayer[b] && !joined.contains(&(a, b)) {
                succs[a].push(b);
                preds[b].push(a);
            }
        }
    }
    let clustered = node_cluster.is_some();

    // Border walls: for every subgraph, a left and a right dummy at each
    // layer it spans, chained vertically so BK keeps the wall straight.
    // The ordering brackets each subgraph's members between its walls, so
    // members stay in a compact band even when external edges tug them —
    // and other nodes are held outside the band. `border_meta` records
    // (node, side, cluster-path-incl-c) for the nodes pushed here.
    let mut border_meta: Vec<(usize, u8, Vec<usize>)> = Vec::new();
    // cluster id -> (lo, hi) layer span over members — walls cover it,
    // and edge dummies use it to know when they are inside a cluster.
    // Only point-read (`get`), never iterated: BTreeMap purely for
    // hygiene / consistency with `span` below (whose KEY ITERATION is
    // the one that must stay ordered for deterministic node numbering).
    let mut cluster_span: std::collections::BTreeMap<usize, (usize, usize)> =
        std::collections::BTreeMap::new();
    // Extra layer-slot room per real node for title strips, before and
    // after it along the layer axis (see below).
    let mut lextra = vec![(0.0f64, 0.0f64); n];
    // Per real node: frame reach (towards lower, higher layers).
    let mut reach = vec![(0.0f64, 0.0f64); n];
    if let Some(nc) = node_cluster {
        // cluster id -> (path incl. c, lo layer, hi layer over members)
        let mut span: std::collections::BTreeMap<usize, (Vec<usize>, usize, usize)> =
            std::collections::BTreeMap::new();
        for v in 0..n {
            let p = &nc[v];
            for i in 0..p.len() {
                let e = span
                    .entry(p[i])
                    .or_insert_with(|| (p[..=i].to_vec(), alayer[v], alayer[v]));
                e.1 = e.1.min(alayer[v]);
                e.2 = e.2.max(alayer[v]);
            }
        }
        for (&c, &(_, lo, hi)) in &span {
            cluster_span.insert(c, (lo, hi));
        }
        // Reserve rank room for each box's TITLE STRIP: a member at its
        // cluster's top layer grows its layer-SLOT by a header's worth
        // per cluster that starts there (dagre solves this with ranked
        // border-top nodes; the inflated slot is the compact stand-in),
        // so labels between ranks never land on the header text. Only
        // the slot grows — the node keeps its real size, so edges still
        // end on its visible border — and only on the strip's side: the
        // screen top's (report M8, §2.8). A slot grown on both sides
        // left a strip's worth of dead room under every box's first row
        // (86 px from it to the next where any other gap is 60). In
        // LR/RL the strip lies across the breadth axis instead and is
        // reserved there (compaction). In BT the layers are drawn
        // bottom-up, so a box's screen top is its HIGHEST layer, the
        // side after it along the layer axis.
        if !horizontal {
            let bt = matches!(g.direction, Direction::BT);
            for v in 0..n {
                let mut extra = 0.0;
                for &c in &nc[v] {
                    let (_, lo, hi) = span[&c];
                    if alayer[v] == if bt { hi } else { lo } {
                        extra += header[c];
                    }
                }
                lextra[v] = if bt { (0.0, extra) } else { (extra, 0.0) };
            }
        }
        // How far the frames a node opens / closes reach past it along
        // the layer axis: one pad per level, plus the title strip on the
        // screen-top side (none in LR/RL, where it lies across).
        let hdr_lo = matches!(g.direction, Direction::TD);
        let hdr_hi = matches!(g.direction, Direction::BT);
        for v in 0..n {
            for &c in &nc[v] {
                let (_, lo, hi) = span[&c];
                if alayer[v] == lo {
                    reach[v].0 += FRAME_PAD + if hdr_lo { header[c] } else { 0.0 };
                }
                if alayer[v] == hi {
                    reach[v].1 += FRAME_PAD + if hdr_hi { header[c] } else { 0.0 };
                }
            }
        }
        // BTreeMap iterates keys in order — wall numbering is
        // deterministic by construction.
        let cids: Vec<usize> = span.keys().copied().collect();
        for c in cids {
            let (prefix, lo, hi) = span[&c].clone();
            let (mut prev_bl, mut prev_br): (Option<usize>, Option<usize>) = (None, None);
            for l in lo..=hi {
                for side in [1u8, 2u8] {
                    let d = alayer.len();
                    alayer.push(l);
                    absize.push(0.0);
                    alsize.push(0.0);
                    preds.push(Vec::new());
                    succs.push(Vec::new());
                    let prev = if side == 1 {
                        &mut prev_bl
                    } else {
                        &mut prev_br
                    };
                    if let Some(p) = *prev {
                        succs[p].push(d);
                        preds[d].push(p);
                    }
                    *prev = Some(d);
                    border_meta.push((d, side, prefix.clone()));
                }
            }
        }
    }

    let na = alayer.len();
    let nlayers = alayer.iter().copied().max().unwrap_or(0) + 1;
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); nlayers];
    for v in 0..na {
        layers[alayer[v]].push(v);
    }

    // Cluster path per augmented node: real nodes take the caller's
    // path; an edge dummy takes a PROGRESSIVE path (dagre's nesting
    // behaviour) — at layers inside an endpoint's cluster span it
    // adopts that endpoint's cluster path (target wins over source),
    // so a cross-cluster edge exits its source box through the bottom,
    // runs outside only while between boxes, then enters the target
    // box from the top and descends inside it — instead of orbiting
    // around the walled band. A border dummy carries its own cluster's
    // path. `border_side[v]` is 0 (none) / 1 (left) / 2 (right wall).
    let mut border_side = vec![0u8; na];
    let apath: Vec<Vec<usize>> = if let Some(nc) = node_cluster {
        let mut p: Vec<Vec<usize>> = vec![Vec::new(); na];
        p[..n].clone_from_slice(nc);
        // Longest prefix of `path` whose every cluster spans layer `l`.
        let fit = |path: &[usize], l: usize| -> usize {
            let mut k = 0;
            for &c in path {
                match cluster_span.get(&c) {
                    Some(&(lo, hi)) if lo <= l && l <= hi => k += 1,
                    _ => break,
                }
            }
            k
        };
        for (ei, chain) in edge_chain.iter().enumerate() {
            if chain.is_empty() {
                continue;
            }
            let e = &g.edges[ei];
            for &d in chain {
                let l = alayer[d];
                let kv = fit(&nc[e.to], l);
                let ku = fit(&nc[e.from], l);
                // A block layout: the deeper fit wins. Below the top
                // level the shared ancestor makes the target's prefix
                // fit on every layer, so a chain out of a nested block
                // took the ancestor's path on the block's remaining
                // layers, lay beside the block and crossed its side
                // wall. (Both fits past the common prefix: a level the
                // lane rule keeps side by side; the target's, as ever.)
                p[d] = if block_paths.is_some() && ku > kv {
                    nc[e.from][..ku].to_vec()
                } else if kv > 0 {
                    nc[e.to][..kv].to_vec()
                } else if ku > 0 {
                    nc[e.from][..ku].to_vec()
                } else {
                    common_prefix(&nc[e.from], &nc[e.to])
                };
            }
        }
        // A block layout's label slot: the chain's middle dummy at the
        // level of its ends' common ancestor, outside both end blocks
        // ([`label_slot`]); sized, it is a real obstacle (see above).
        if block_paths.is_some() {
            for (ei, chain) in edge_chain.iter().enumerate() {
                let e = &g.edges[ei];
                let (Some(lbl), false) = (&e.label, chain.is_empty()) else {
                    continue;
                };
                let paths: Vec<Vec<usize>> = chain.iter().map(|&d| p[d].clone()).collect();
                let mid = chain[label_slot(&paths, &common_prefix(&nc[e.from], &nc[e.to]))];
                let lw = text_width(lbl) + LABEL_PAD_W;
                let lh = LINE_H * line_count(lbl) as f64 + 6.0;
                if horizontal {
                    absize[mid] = absize[mid].max(lh);
                    alsize[mid] = alsize[mid].max(lw);
                } else {
                    absize[mid] = absize[mid].max(lw);
                    alsize[mid] = alsize[mid].max(lh);
                }
            }
        }
        for (d, side, path) in &border_meta {
            p[*d] = path.clone();
            border_side[*d] = *side;
        }
        p
    } else {
        Vec::new()
    };

    // --- 3. Reduce crossings: dagre-style ordering. Weighted-median
    // sweeps (down via preds, up via succs) followed by a local
    // adjacent-swap transpose each round. The median heuristic can
    // transiently worsen an ordering, so we keep the layering with the
    // fewest crossings seen across all rounds (keep-best) — this also
    // guards against regressing the natural insertion order on ties.
    // When clustered, each round finishes by regrouping every layer so
    // subgraph members stay contiguous, and the crossing count (hence
    // keep-best) is measured on that contiguous ordering.
    let mut pos = vec![0.0f64; na];
    for lv in &layers {
        for (i, &v) in lv.iter().enumerate() {
            pos[v] = i as f64;
        }
    }
    // A block layout orders level first ([`BlockOrder`]).
    let mut dummy_edge = vec![usize::MAX; na];
    for (ei, chain) in edge_chain.iter().enumerate() {
        for &d in chain {
            dummy_edge[d] = ei;
        }
    }
    let element = |v: usize| -> Option<(usize, usize)> {
        if v < n {
            Some((v, usize::MAX))
        } else {
            (dummy_edge[v] != usize::MAX).then(|| (dummy_edge[v], alayer[v]))
        }
    };
    let mut block_order = match (block_paths, node_cluster) {
        (Some(_), Some(nc)) => {
            let ns = g.subgraphs.len();
            let mut first = vec![usize::MAX; ns];
            let mut path_of: Vec<Option<Vec<usize>>> = vec![None; ns];
            for (v, path) in nc.iter().enumerate() {
                for (i, &c) in path.iter().enumerate() {
                    first[c] = first[c].min(v);
                    if path_of[c].is_none() {
                        path_of[c] = Some(path[..=i].to_vec());
                    }
                }
            }
            let mut canon = vec![(0usize, 0u8, 0usize, 0usize); na];
            for (v, key) in canon.iter_mut().enumerate().take(n) {
                *key = (v, 0, 0, 0);
            }
            let mut run = vec![usize::MAX; na];
            let mut runs = 0;
            for (ei, chain) in edge_chain.iter().enumerate() {
                let e = &g.edges[ei];
                let mut prev: Option<usize> = None;
                for &d in chain {
                    canon[d] = (e.from, 1, e.to, ei);
                    match prev {
                        Some(p) if apath[p] == apath[d] => run[d] = run[p],
                        _ => {
                            run[d] = runs;
                            runs += 1;
                        }
                    }
                    prev = Some(d);
                }
            }
            for (d, side, path) in &border_meta {
                let c = path.last().copied().unwrap_or(0);
                canon[*d] = (first[c], 2, usize::from(*side), 0);
            }
            let mut levels: Vec<(Vec<usize>, usize, usize)> =
                vec![(Vec::new(), 0, nlayers.saturating_sub(1))];
            let mut subs: Vec<usize> = cluster_span.keys().copied().collect();
            subs.sort_by_key(|&c| (path_of[c].as_ref().map_or(0, Vec::len), first[c]));
            for c in subs {
                if let (Some(path), Some(&(lo, hi))) = (path_of[c].clone(), cluster_span.get(&c)) {
                    levels.push((path, lo, hi));
                }
            }
            let mut bspan = vec![(0usize, 0usize); ns];
            for (&c, &span) in &cluster_span {
                bspan[c] = span;
            }
            let mut rspan = vec![(usize::MAX, 0usize); runs];
            for (d, &r) in run.iter().enumerate() {
                if r != usize::MAX {
                    rspan[r] = (rspan[r].0.min(alayer[d]), rspan[r].1.max(alayer[d]));
                }
            }
            Some(BlockOrder {
                bspan,
                rspan,
                apath: &apath,
                border: &border_side,
                preds: &preds,
                succs: &succs,
                alayer: &alayer,
                nlayers,
                canon,
                first,
                run,
                runs,
                levels,
                brank: vec![0.0; ns],
                slot: vec![0.0; na],
            })
        }
        _ => None,
    };
    if let Some(order) = block_order.as_mut() {
        let links: usize = succs.iter().map(Vec::len).sum();
        let starts = if opts.judge {
            (BLOCK_ORDER_WORK / (na + links).max(1)).clamp(1, BLOCK_ORDER_STARTS)
        } else {
            1
        };
        // (The reports measure the crossings start by start.)
        let starts = crate::levels::order_starts().unwrap_or(starts);
        order.run(
            starts,
            seed.map(|s| (s, &element as ElementOf<'_>)),
            &mut layers,
            &mut pos,
        );
    }
    if clustered && block_order.is_none() {
        let keys = cluster_keys(&layers, &apath, &border_side);
        for lv in layers.iter_mut() {
            enforce_contiguity(lv, &apath, &border_side, &mut pos, &keys);
        }
    }
    let mut best_layers = layers.clone();
    let mut best_cross = count_crossings(&layers, &succs, &alayer, nlayers);
    for _ in 0..8 {
        if block_order.is_some() {
            break;
        }
        for li in 1..nlayers {
            reorder(&mut layers[li], &preds, &mut pos);
        }
        for li in (0..nlayers.saturating_sub(1)).rev() {
            reorder(&mut layers[li], &succs, &mut pos);
        }
        transpose(
            &mut layers,
            &preds,
            &succs,
            &mut pos,
            nlayers,
            if clustered { Some(&apath) } else { None },
            if clustered { Some(&border_side) } else { None },
        );
        if clustered {
            let keys = cluster_keys(&layers, &apath, &border_side);
            for lv in layers.iter_mut() {
                enforce_contiguity(lv, &apath, &border_side, &mut pos, &keys);
            }
        }
        let c = count_crossings(&layers, &succs, &alayer, nlayers);
        if c < best_cross {
            best_cross = c;
            best_layers = layers.clone();
        }
        if best_cross == 0 {
            break;
        }
    }
    layers = best_layers;
    // A note card beside a fork/join bar goes past the bar's branches
    // in its layer, on its own side (after them for `right of`, before
    // them for `left of`): between two branches it would push them
    // apart under the bar spanning them.
    for (card, bar) in bar_cards(g) {
        let lv = &mut layers[alayer[card]];
        let branch = |v: &usize| *v != card && (preds[bar].contains(v) || succs[bar].contains(v));
        let (Some(first), Some(last)) = (lv.iter().position(branch), lv.iter().rposition(branch))
        else {
            continue;
        };
        let Some(at) = lv.iter().position(|&v| v == card) else {
            continue;
        };
        let to = if g.lead_nodes.contains(&card) {
            first.min(at)
        } else {
            last.max(at)
        };
        let v = lv.remove(at);
        lv.insert(to, v);
    }
    for lv in &layers {
        for (i, &v) in lv.iter().enumerate() {
            pos[v] = i as f64;
        }
    }
    if let (Some(order), false) = (block_order.as_ref(), bar_cards(g).is_empty()) {
        for lv in layers.iter_mut() {
            order.regroup(lv, &mut pos);
        }
    } else if clustered && !bar_cards(g).is_empty() {
        let keys = cluster_keys(&layers, &apath, &border_side);
        for lv in layers.iter_mut() {
            enforce_contiguity(lv, &apath, &border_side, &mut pos, &keys);
        }
    }
    // A subgraph [`centre_largest`] moves: its key changes, every layer
    // regroups around it. (Not in a block layout: the level orders
    // placed its blocks.)
    if let (true, Some((c, key)), true) = (clustered, opts.hint, block_order.is_none()) {
        let mut keys = cluster_keys(&layers, &apath, &border_side);
        keys.1.insert(c, key);
        for lv in layers.iter_mut() {
            enforce_contiguity(lv, &apath, &border_side, &mut pos, &keys);
        }
    }
    // The largest top-level node steps towards the middle of its layer
    // (G3), one crossing-neutral swap at a time. Like the rest of the
    // search it is skipped past the budget: on a layer of k nodes the
    // walk is up to k/2 swaps, each re-counting two gaps.
    // A bar's note card stays beside the bar's branches (above).
    let big = if opts.judge {
        largest_free_node(sizes, node_cluster, &layers, &alayer)
            .filter(|&v| bar_cards(g).iter().all(|&(card, _)| card != v))
    } else {
        None
    };
    if let Some(v) = big {
        centre_in_layer(
            v,
            &mut layers,
            &succs,
            &alayer,
            &mut pos,
            &apath,
            &border_side,
        );
    }
    // --- 4. Coordinates ---
    // A state's note card beside a fork/join bar is no branch of it: the
    // bar centres over its branches (as `scene::fit_bars` sizes it) and
    // the card takes the room beside them, so their link does not pull
    // either. It still orders the layers.
    let (cpreds, csuccs) = unlink_bar_cards(g, &preds, &succs);
    let (preds_c, succs_c) = (&cpreds[..], &csuccs[..]);
    // Breadth positions (b axis) via Brandes-Köpf: four vertical
    // alignments (up/down × left/right), each compacted independently,
    // then combined by the per-node median. This is dagre's coordinate
    // assignment — it pins each long edge's dummy chain into a straight
    // vertical run and centres nodes over their aligned neighbours.
    // The far ends of a compact branch point's fan whose labels need
    // to sit side by side get the room (see `fan_label_room`).
    let spaced = fan_label_room(g, back, &layers, &alayer, &absize, &alsize, n, horizontal);
    // A self-looped node keeps its loop's reach (and NODE_SEP) from its
    // high-side neighbour, where the scene draws the loop (report P0
    // #13: a channel beside it ran through the loop's apex). A
    // separation size counts on both sides; a node with no high-side
    // neighbour needs none. The node's own half of the gap between
    // neighbours (`pair_sep`: GAP_B / 2) already lies past its side, so
    // only what the loop reaches beyond it widens the node (stress's
    // `A ==> D` channel ran 54 px past the loop's apex, the canvas 72 px
    // wider than it needed).
    let loops = crate::scene::loop_reach(g);
    let spaced = if loops.iter().any(|&r| r > 0.0) {
        let mut sep = spaced.unwrap_or_else(|| absize.clone());
        for lay in &layers {
            for w in lay.windows(2) {
                let v = w[0];
                if v < n && loops[v] > 0.0 {
                    let past = (loops[v] + NODE_SEP - GAP_B / 2.0).max(0.0);
                    sep[v] = sep[v].max(absize[v] + 2.0 * past);
                }
            }
        }
        Some(sep)
    } else {
        spaced
    };
    let bsep: &[f64] = spaced.as_deref().unwrap_or(&absize);
    let mut bpos = coordinates_bk(
        n,
        na,
        nlayers,
        &layers,
        preds_c,
        succs_c,
        &alayer,
        bsep,
        if clustered { Some(&apath) } else { None },
        if clustered { Some(&border_side) } else { None },
        if clustered && horizontal {
            Some(header.as_slice())
        } else {
            None
        },
    );
    if opts.compact {
        compact_breadth(
            n,
            &layers,
            preds_c,
            succs_c,
            &alayer,
            bsep,
            if clustered { Some(&apath) } else { None },
            if clustered { Some(&border_side) } else { None },
            if clustered && horizontal {
                Some(header.as_slice())
            } else {
                None
            },
            &mut bpos,
        );
    }
    // ... then slides inside its layer's free room towards the breadth
    // centre of everything drawn (D4); the order stays, so no crossing
    // appears.
    let chains_b = |bpos: &[f64]| -> (f64, f64) {
        drawn_breadth(
            g,
            node_cluster,
            &absize,
            &alsize,
            &alayer,
            bpos,
            &edge_chain,
            block_paths,
        )
    };
    // A node already on a straight run with a neighbour stays: sliding it
    // would kink that edge into an S (G8).
    let aligned = |v: usize, bpos: &[f64]| {
        preds[v]
            .iter()
            .chain(&succs[v])
            .any(|&u| (bpos[u] - bpos[v]).abs() < ALIGN_EPS)
    };
    if let Some(v) = big.filter(|&v| !aligned(v, &bpos)) {
        let ext = chains_b(&bpos);
        let lay = &layers[alayer[v]];
        let s = lay.iter().position(|&w| w == v).unwrap_or(0);
        let ap = if clustered {
            Some(apath.as_slice())
        } else {
            None
        };
        let bs = if clustered {
            Some(border_side.as_slice())
        } else {
            None
        };
        let hb = if clustered && horizontal {
            Some(header.as_slice())
        } else {
            None
        };
        let half = absize[v] / 2.0;
        let lo = match s.checked_sub(1).map(|i| lay[i]) {
            Some(l) => bpos[l] + pair_sep(l, v, n, bsep, ap, bs, hb),
            None => ext.0 + half,
        };
        let hi = match lay.get(s + 1) {
            Some(&r) => bpos[r] - pair_sep(v, r, n, bsep, ap, bs, hb),
            None => ext.1 - half,
        };
        // Only as far as needed: into the middle CENTRE_SLIDE_BAND of
        // the breadth, so its edges keep their angles where they can.
        let (mid, band) = (
            (ext.0 + ext.1) / 2.0,
            (ext.1 - ext.0) / 2.0 * CENTRE_SLIDE_BAND,
        );
        if lo <= hi {
            let to = bpos[v].max(mid - band).min(mid + band).max(lo).min(hi);
            if (to - bpos[v]).abs() >= CENTRE_SLIDE_MIN_MOVE {
                bpos[v] = to;
            }
        }
    }

    // Layer positions (l axis): each layer is as tall as its tallest
    // REAL node (dummies contribute zero layer-size).
    // A box closing on one layer and another opening on the next must
    // not meet: their pads and title strips stack up with nesting depth,
    // which can outgrow the gap, so a layer moves down until the frames
    // reaching across clear by FRAME_GAP.
    let reach_of = |li: usize, side: usize| -> f64 {
        layers[li]
            .iter()
            .filter(|&&v| v < n)
            .map(|&v| alsize[v] / 2.0 + if side == 0 { reach[v].0 } else { reach[v].1 })
            .fold(0.0f64, f64::max)
    };
    // An edge between ADJACENT layers has no dummy to reserve its
    // label a slot: the label sits mid-gap, between the two borders.
    // Per upper layer: (lower end, upper end, label extent along the
    // layer axis, end room, whether a compact branch point's fan holds
    // it) for each labelled such edge — in LR/RL that extent is the
    // label's width, which easily outgrows GAP_L. The end room
    // (`Edge::end_room`, both ends summed: ER crow's feet) is gap the
    // label may not take, so an edge carrying any also gets an entry
    // without a label (extent 0): its glyphs must fit facing each other.
    // Last, the room past its label an LR / RL gap keeps (its glyphs and
    // clearances, or a styled edge's stroke runs: `lr_label_room`). An
    // end given as `None` is
    // a frame border the edge is cut back to (`Graph::frame_cut`), on
    // the farthest frame closing / opening on that layer.
    type ShortLabel = (Option<usize>, Option<usize>, f64, f64, bool, f64);
    let mut short_labels: Vec<Vec<ShortLabel>> = vec![Vec::new(); nlayers];
    // Per upper layer: (breadth centre, breadth extent, layer extent,
    // end room).
    let mut short_across: Vec<Vec<(f64, f64, f64, f64)>> = vec![Vec::new(); nlayers];
    // Per upper layer: the lines of compact branch points' fans
    // (exactly one end such a point, as the label stage tells them)
    // between it and the layer before, labelled or not (see `FanLine`).
    let mut fan_lines: Vec<Vec<FanLine>> = vec![Vec::new(); nlayers];
    let branch = |v: usize| is_branch_point(g.nodes[v].shape, &g.nodes[v].label);
    // Two transitions between one pair leave and reach it PARALLEL_GAP
    // apart (as `scene::anchor` spreads their ports).
    let par = if (0..g.nodes.len()).any(branch) {
        crate::scene::parallel_offsets(g)
    } else {
        Vec::new()
    };
    let par_reach = parallel_reach(g, &par);
    // A back-edge's lane leaves its source and enters its target by a
    // leg across the gap beside that end, `crate::lanes::LANE_STUB` out
    // (one `LANE_STAGGER` more per lane nested round it there), from a
    // port at the edge of the side's usable span (one `LANE_STAGGER`
    // further in per twin lane) to the lane past the node. A label
    // centred in that gap on an edge of the same node, reaching across
    // that span edge, keeps that leg's depth clear at both ends of the
    // gap, or it lies on the legs (x6-rl-loops: `forward` over both
    // `Gamma -> Alpha` lanes). Per labelled short edge: the room the
    // legs take, summed for both ends of the gap. Only in LR / RL, where
    // the label lies along the flow and fills the gap's length; a TD /
    // BT label is a row tall and slides along its line past the legs.
    let lane_par = if horizontal && par.is_empty() && back.iter().any(|&b| b) {
        crate::scene::parallel_offsets(g)
    } else {
        par.clone()
    };
    let lane_legs = |ei: usize, lo: usize, hi: usize, across: f64| -> f64 {
        if !horizontal {
            return 0.0;
        }
        let e = &g.edges[ei];
        let off = lane_par.get(ei).copied().unwrap_or(0.0);
        let centre = (bpos[lo] + bpos[hi]) / 2.0 + off;
        let (c0, c1) = (centre - across / 2.0, centre + across / 2.0);
        let mut deepest = 0.0f64;
        for v in [lo, hi] {
            // Lanes with a leg on `v`'s side facing this gap: into `lo`
            // (its downstream side), out of `hi` (its upstream side).
            let legs: Vec<usize> = g
                .edges
                .iter()
                .enumerate()
                .filter(|&(bj, b)| {
                    back[bj]
                        && b.from != b.to
                        && alayer[b.from].abs_diff(alayer[b.to]) > 1
                        && if v == lo { b.to == v } else { b.from == v }
                })
                .map(|(bj, _)| bj)
                .collect();
            if legs.is_empty() || (e.from != v && e.to != v) {
                continue;
            }
            let p = Placed {
                b: bpos[v],
                l: 0.0,
                bsize: absize[v],
                lsize: alsize[v],
                layer: alayer[v],
            };
            let span = crate::lanes::port_span(g.nodes[v].shape, &p);
            // Twin lanes (same far end) step their ports inward.
            let twins = legs
                .iter()
                .map(|&bj| {
                    let far = |b: &crate::model::Edge| if b.from == v { b.to } else { b.from };
                    legs.iter()
                        .filter(|&&bk| far(&g.edges[bk]) == far(&g.edges[bj]))
                        .count()
                })
                .max()
                .unwrap_or(1);
            let inward = (twins - 1) as f64 * crate::lanes::LANE_STAGGER;
            if c1 > span.1 - inward || c0 < span.0 + inward {
                let depth =
                    crate::lanes::LANE_STUB + (legs.len() - 1) as f64 * crate::lanes::LANE_STAGGER;
                deepest = deepest.max(depth);
            }
        }
        2.0 * deepest
    };
    for (ei, e) in g.edges.iter().enumerate() {
        let short = alayer[e.from].abs_diff(alayer[e.to]) == 1;
        // A stand-in for an edge that ends on a whole box (`K0 --> K1`,
        // `Graph::frame_cut`) is drawn only between the borders it is cut
        // back to; when those face each other across one gap its label
        // lies there, though the stand-in runs on to members deeper
        // inside (or back up, as a back-edge's does). Without room there
        // two composites stacked 18 px apart had their labels cover the
        // whole edge and its head (B7 review r4 #2). A stand-in whose
        // own ends are adjacent is measured as any short edge, below.
        if let Some(cut) = g
            .frame_cut
            .get(ei)
            .filter(|c| c[0].is_some() || c[1].is_some())
        {
            if let (Some(lbl), false) = (
                &e.label,
                e.from == e.to || matches!(e.kind, EdgeKind::Invisible) || short,
            ) {
                let (lo, hi, lo_cut, hi_cut) = if alayer[e.from] < alayer[e.to] {
                    (e.from, e.to, cut[0], cut[1])
                } else {
                    (e.to, e.from, cut[1], cut[0])
                };
                // The layer the border faces from: a box left on its
                // last layer, entered on its first.
                let lo_l = lo_cut
                    .and_then(|c| cluster_span.get(&c))
                    .map_or(alayer[lo], |s| s.1);
                let hi_l = hi_cut
                    .and_then(|c| cluster_span.get(&c))
                    .map_or(alayer[hi], |s| s.0);
                if hi_l == lo_l + 1 {
                    let w = text_width(lbl) + LABEL_PAD_W;
                    let ext = if horizontal { w } else { LABEL_BOX_H };
                    let room = e.end_room.0 + e.end_room.1;
                    let end = |v: usize, c: Option<usize>| c.is_none().then_some(v);
                    short_labels[hi_l].push((
                        end(lo, lo_cut),
                        end(hi, hi_cut),
                        ext,
                        room,
                        false,
                        lr_label_room(e),
                    ));
                    // No row of its own among the gap's labels: where the
                    // drawn run lies across the gap is up to the frames'
                    // ports, not the stand-in's members, and a composite's
                    // 2-cycle labels (`Idle --> Busy`, `Busy --> Idle`)
                    // already stand beside their lines, as drawn before.
                }
                continue;
            }
        }
        // A back-edge between adjacent layers is drawn across their gap
        // (`crate::lanes`: nothing lies between its ends), its label
        // there too: the gap fits it like a forward edge's (a 2-cycle's
        // two labels side by side).
        if e.from == e.to || (back[ei] && !short) || matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        let fan = match (branch(e.from), branch(e.to)) {
            _ if back[ei] => None,
            (true, false) => Some((e.from, e.to)),
            (false, true) => Some((e.to, e.from)),
            _ => None,
        };
        if let (Some((p, far)), true, false) = (fan, short, horizontal) {
            // Near its far end a fan's line runs at that end's port,
            // leaning toward the point on a box (see `scene::anchor`).
            let off = par.get(ei).copied().unwrap_or(0.0);
            let reach = par_reach.get(ei).copied().unwrap_or(0.0);
            let lean = match g.nodes[far].shape {
                Shape::Diamond
                | Shape::Circle
                | Shape::DoubleCircle
                | Shape::StateStart
                | Shape::StateEnd
                | Shape::ForkBar => off,
                shape => fan_end_lean(
                    shape,
                    (absize[far], alsize[far]),
                    bpos[p] - bpos[far],
                    off,
                    reach,
                ),
            };
            // It leaves the small diamond's border on the way to the far
            // end's centre: about a quarter of the diamond off its centre
            // for a slanted line.
            let toward = bpos[far] - bpos[p];
            fan_lines[alayer[e.from].max(alayer[e.to])].push(FanLine {
                point: p,
                from: bpos[p] + toward.clamp(-CHOICE_SIZE / 4.0, CHOICE_SIZE / 4.0),
                to: bpos[far] + lean,
                // A line with parallel twins on both sides has no spot
                // clear of them for its label: no gap makes one.
                width: e
                    .label
                    .as_ref()
                    .filter(|_| off.abs() >= reach)
                    .map(|l| text_width(l) + LABEL_PAD_W),
            });
        }
        let room = e.end_room.0 + e.end_room.1;
        if e.label.is_none() && room <= 0.0 {
            continue;
        }
        let (lo, hi) = if alayer[e.from] < alayer[e.to] {
            (e.from, e.to)
        } else {
            (e.to, e.from)
        };
        if alayer[hi] != alayer[lo] + 1 {
            continue;
        }
        let Some(lbl) = &e.label else {
            short_labels[alayer[hi]].push((Some(lo), Some(hi), 0.0, room, false, 0.0));
            continue;
        };
        let w = text_width(lbl) + LABEL_PAD_W;
        let (ext, across) = if horizontal {
            (w, LABEL_BOX_H)
        } else {
            (LABEL_BOX_H, w)
        };
        // The label sits mid-gap, so room a class diagram keeps at
        // either end of a labelled edge (`Graph::label_end_room`: glyph
        // and cardinalities) is kept at both.
        let label_room = 2.0 * g.label_end_room.get(ei).copied().unwrap_or(0.0);
        let room = (room + label_room).max(lane_legs(ei, lo, hi, across));
        short_labels[alayer[hi]].push((
            Some(lo),
            Some(hi),
            ext,
            room,
            fan.is_some(),
            lr_label_room(e),
        ));
        // In LR/RL a row along the layer axis costs a label's whole
        // width, and the scene never uses it: colliding labels there
        // slide apart ACROSS the flow (they are only LABEL_BOX_H tall),
        // so each gap just has to fit its widest label (above).
        if !horizontal {
            short_across[alayer[hi]].push(((bpos[lo] + bpos[hi]) / 2.0, across, ext, room));
        }
    }
    // TD/BT labels sharing one gap that would overlap side by side need
    // rows of their own, stacked along the layer axis (a fan of labelled
    // edges out of one node, or into one). Greedy interval packing on
    // each label's breadth span, centred between its ends; the gap must
    // hold every row with LABEL_CLEAR on both sides, plus one spare row:
    // the labels then slide along their own (slanted) curves to reach
    // a row, which never lands them exactly side by side. One row =
    // nothing overlaps = no requirement, so only colliding gaps grow.
    // (Siblings that would read as one phrase are kept apart where the
    // labels are placed, `labels::label_siblings`: the midpoint spans
    // here are not where a hub's labels end up, and reserving sibling
    // rows from them only grew a busy hub's gap.)
    let gap_need: Vec<f64> = short_across
        .iter_mut()
        .map(|spans| {
            spans.sort_by(|a, b| (a.0 - a.1 / 2.0).total_cmp(&(b.0 - b.1 / 2.0)));
            // Per row: (right end so far, largest extent along l).
            let mut rows: Vec<(f64, f64)> = Vec::new();
            for &(mid, across, ext, _) in spans.iter() {
                let left = mid - across / 2.0;
                match rows.iter_mut().find(|r| r.0 + LABEL_CLEAR <= left) {
                    Some(r) => *r = (mid + across / 2.0, r.1.max(ext)),
                    None => rows.push((mid + across / 2.0, ext)),
                }
            }
            if rows.len() < 2 {
                return 0.0;
            }
            let tallest = rows.iter().map(|r| r.1).fold(0.0, f64::max);
            // The rows stack between the end glyphs, never over them.
            let room = spans.iter().map(|s| s.3).fold(0.0, f64::max);
            rows.iter().map(|r| r.1).sum::<f64>()
                + tallest
                + room
                + 2.0 * LABEL_CLEAR * (rows.len() + 1) as f64
        })
        .zip(&fan_lines)
        .map(|(need, lines)| fan_rows_gap(lines, need))
        .collect();
    let hub_need = if g.edge_paint_width.is_none() {
        hub_label_room(g, back, &alayer, &bpos, &absize, nlayers, horizontal)
    } else {
        vec![Vec::new(); nlayers]
    };
    let gap_l = g.rank_gap.unwrap_or(GAP_L);
    // Per lower layer: the least distance between its line and the one
    // before it for every edge crossing a frame border in that gap to
    // cross it clear of its lateral run (B7 review r1 #2, #3; report M8).
    // A free curve drawn across a gap runs flattest midway between its
    // ends, and the router's channel legs run there too; a box's title
    // strip (with its pad) puts the box's border [`FRAME_PAD`] + header
    // above its first row, which with the strip no longer doubled (M8)
    // left the border 3 px off mid-gap: advanced's `Gate ==> Job Queue`
    // rode 122 px along Async Workers' top (and was rerouted into an L
    // inside it). Such a gap keeps the outermost border an edge's run
    // crosses — a box entered or left, title strip or pad —
    // [`FRAME_SEP`] off the run's middle, on the border's own side, when
    // the run's ends lie more than [`TIGHT_RUN_MIN`] apart sideways (a
    // steeper run crosses the border too steeply to ride it: Client ->
    // Platform's straight drop keeps the gap). A border one pad deep
    // never needs it (2 pads and 2 seps are under GAP_L); flowchart and
    // state drawings only. (Borders a run only passes beside are left
    // alone: keeping clear of those too re-laid dense-td 200 px deeper
    // and bench-75 wider, and their routers then rode more borders.)
    let cross_need: Vec<f64> = {
        let mut need = vec![0.0f64; nlayers];
        if clustered && g.edge_paint_width.is_none() {
            let span_of = |c: usize| cluster_span.get(&c).copied();
            let hdr_lo = matches!(g.direction, Direction::TD);
            let hdr_hi = matches!(g.direction, Direction::BT);
            for (ei, e) in g.edges.iter().enumerate() {
                if e.from == e.to || matches!(e.kind, EdgeKind::Invisible) {
                    continue;
                }
                // The edge's path down the layers (its chain is stored
                // in from -> to order).
                let chain = &edge_chain[ei];
                let path: Vec<usize> = if alayer[e.from] <= alayer[e.to] {
                    std::iter::once(e.from)
                        .chain(chain.iter().copied())
                        .chain(std::iter::once(e.to))
                        .collect()
                } else {
                    std::iter::once(e.to)
                        .chain(chain.iter().rev().copied())
                        .chain(std::iter::once(e.from))
                        .collect()
                };
                for w in path.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    let l = alayer[a];
                    // A back-edge's skip is drawn in a lane.
                    if alayer[b] != l + 1 || (bpos[a] - bpos[b]).abs() <= TIGHT_RUN_MIN {
                        continue;
                    }
                    // How far past each end the outermost border it
                    // crosses lies: the boxes left closing on `a`'s
                    // layer and entered opening on `b`'s, a pad each and
                    // the title strip on the screen-top side.
                    let depth = |at: usize, from: usize, open: bool| -> f64 {
                        apath[at]
                            .iter()
                            .filter(|c| !apath[from].contains(c))
                            .filter(|&&c| {
                                span_of(c)
                                    .is_some_and(|s| if open { s.0 == l + 1 } else { s.1 == l })
                            })
                            .map(|&c| {
                                let strip = if open { hdr_lo } else { hdr_hi };
                                FRAME_PAD + if strip { header[c] } else { 0.0 }
                            })
                            .sum()
                    };
                    let d = depth(a, b, false).max(depth(b, a, true));
                    if d > 0.0 {
                        let ends = alsize[a] / 2.0 + alsize[b] / 2.0;
                        need[l + 1] = need[l + 1].max(ends + 2.0 * (d + FRAME_SEP));
                    }
                }
            }
        }
        need
    };
    // Per lower layer: the least gap above it for the steps the edges
    // crossing it take (report B1, §2.1): every edge is drawn on the axes
    // (`crate::ortho`), each step across a gap on a track of its own, and
    // the steps whose spans overlap stack one track pitch apart between
    // a row's clearance above and a head's straight run below. A gap
    // holds two tracks at the plain [`GAP_L`]; a fan of three or more
    // steps one way grows it by a pitch per further track (flowchart and
    // state drawings only; the spans are read off the node and dummy
    // centres, which the ports spread a little). Past the router's cap
    // the ortho stage keeps the builders' curves, so no gap takes tracks
    // there: the count is the scene's ([`Graph::drawn_edge_count`]), as
    // the ortho stage counts it, and each part of a compound layout
    // counts its own, as its own scene does.
    let track_need: Vec<f64> = {
        let mut need = vec![0.0f64; nlayers];
        if g.edge_paint_width.is_none() && g.drawn_edge_count() <= crate::router::ROUTE_MAX_EDGES {
            let mut spans: Vec<Vec<(f64, f64)>> = vec![Vec::new(); nlayers];
            for (ei, e) in g.edges.iter().enumerate() {
                if e.from == e.to || matches!(e.kind, EdgeKind::Invisible) {
                    continue;
                }
                let chain = &edge_chain[ei];
                let path: Vec<usize> = if alayer[e.from] <= alayer[e.to] {
                    std::iter::once(e.from)
                        .chain(chain.iter().copied())
                        .chain(std::iter::once(e.to))
                        .collect()
                } else {
                    std::iter::once(e.to)
                        .chain(chain.iter().rev().copied())
                        .chain(std::iter::once(e.from))
                        .collect()
                };
                for w in path.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    if alayer[b] != alayer[a] + 1 || (bpos[a] - bpos[b]).abs() <= 1.0 {
                        continue;
                    }
                    spans[alayer[b]].push((bpos[a].min(bpos[b]), bpos[a].max(bpos[b])));
                }
            }
            for (li, sp) in spans.iter_mut().enumerate() {
                // The most spans over one point: the tracks the gap needs.
                let mut ev: Vec<(f64, i32)> = Vec::with_capacity(2 * sp.len());
                for &(a, b) in sp.iter() {
                    ev.push((a, 1));
                    ev.push((b, -1));
                }
                ev.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
                let (mut cur, mut most) = (0i32, 0i32);
                for (_, d) in ev {
                    cur += d;
                    most = most.max(cur);
                }
                if most >= 1 {
                    need[li] = crate::ortho::ROW_CLEAR
                        + crate::ortho::HEAD_LEG
                        + (most - 1) as f64 * crate::ortho::TRACK_PITCH;
                }
            }
        }
        need
    };
    // The layer lines and the depth they span. `judged`: as the layout
    // search judges a candidate (crossings, node hits, the ribbon tests)
    // — a title strip's slot grown on both sides, a labelled gap only as
    // deep as its label needs — which the drawing no longer takes (report
    // M3, M8): the room those two keep or give back makes the drawing
    // deeper or shallower without its layers being any fewer, and must
    // not decide a layering or an ordering. (Judged on the drawn depth,
    // dense-td — a 4.06 : 1 ribbon — went 3.9 : 1 with the head room and
    // stayed 6 768 px broad, and with the strips' dead room gone its
    // stacked candidates' skeletons, a row nearer, cut 8 more boxes.)
    let layer_lines = |judged: bool| -> (Vec<f64>, f64) {
        let mut lcoord = vec![0.0f64; nlayers];
        let mut cursor = MARGIN;
        let mut prev_after = 0.0;
        for li in 0..nlayers {
            // The layer's slot: how far it reaches before its line (the
            // line sits `before` past the slot's start) and after it —
            // its tallest node's halves plus any title strip on the
            // strip's side (on both, `judged`).
            let (before, after) = layers[li]
                .iter()
                .map(|&v| {
                    let (x0, x1) = lextra.get(v).copied().unwrap_or((0.0, 0.0));
                    if judged {
                        let half = (alsize[v] + 2.0 * (x0 + x1)) / 2.0;
                        (half, half)
                    } else {
                        (alsize[v] / 2.0 + x0, alsize[v] / 2.0 + x1)
                    }
                })
                .fold((0.0f64, 0.0f64), |a, b| (a.0.max(b.0), a.1.max(b.1)));
            if clustered && li > 0 {
                let at = lcoord[li - 1] + reach_of(li - 1, 1) + reach_of(li, 0) + FRAME_GAP;
                if at > cursor + before {
                    cursor = at - before;
                }
            }
            // A label that would cover its own ends grows the gap until
            // it clears both by LABEL_CLEAR; a label that already fits
            // leaves the gap — and the whole layout — exactly as it was.
            // An end whose frame closes / opens on its layer is reached
            // past that frame's border (the layer's farthest one), not
            // its own box: a label on the border line would hide it. End
            // room counts as part of the ends: the label (or, without
            // one, the other end's glyph) clears it by LABEL_CLEAR too.
            for &(s, t, ext, room, fan, lr_room) in &short_labels[li] {
                let lo_side = match s {
                    Some(s) if reach[s].1 <= 0.0 => alsize[s] / 2.0,
                    _ => reach_of(li - 1, 1),
                };
                let hi_side = match t {
                    Some(t) if reach[t].0 <= 0.0 => alsize[t] / 2.0,
                    _ => reach_of(li, 0),
                };
                let ends = lcoord[li - 1] + (lo_side + hi_side);
                let at = cursor + before;
                // In a drawing the flowchart painter draws, a labelled
                // gap keeps its label clear of the arrowhead too (report
                // M3, §2.6): across TD / BT a [`LABEL_RUN`] on either
                // side; along LR / RL, where the label is a whole width
                // long, the head (each head of a double-headed edge) and
                // a clearance on both sides — the label stage centres such
                // a label on the run clear of its head
                // (`labels::head_clear_spot`), so the clearance falls at
                // both ends of the chip (`lr_label_room`). A styled edge's gap
                // (dotted, thick, `linkStyle`-painted) keeps a run of its
                // stroke in sight past those clearances too
                // (`styled_room`), or the chip covers all but a dash of
                // it and its style no longer reads. (Growing every LR
                // gap by a `LABEL_RUN` either side re-laid dense-lr and
                // drew wide-labels flatter than `MAX_ASPECT`.) (Class
                // and ER scenes keep their glyph room in `room` and the
                // plain rule.)
                let flow = ext > 0.0 && g.edge_paint_width.is_none() && !judged;
                let need = match (flow, horizontal) {
                    (true, false) => Some(ext + 2.0 * LABEL_RUN),
                    (true, true) => Some(ext + lr_room),
                    (false, _) => None,
                };
                match need {
                    Some(need) if at - ends < room + need => {
                        cursor = cursor.max(ends + room + need - before);
                    }
                    // The plain rule grows a gap only where the label
                    // does not fit at all.
                    None if at - ends < ext + room => {
                        cursor = cursor.max(ends + room + ext + 2.0 * LABEL_CLEAR - before);
                    }
                    _ => {}
                }
                if fan && at - ends < fan_gap(ext / 2.0) {
                    cursor = cursor.max(ends + fan_gap(ext / 2.0) - before);
                }
            }
            // A hub fan's label clears its siblings' curves at its own
            // curve's middle (`hub_label_room`), from box border to box
            // border.
            if !judged {
                for &(s, t, need) in &hub_need[li] {
                    let ends = lcoord[li - 1] + (alsize[s] + alsize[t]) / 2.0;
                    if cursor + before - ends < need {
                        cursor = ends + need - before;
                    }
                }
            }
            if li > 0 && gap_need[li] > 0.0 {
                let prev = lcoord[li - 1] + prev_after;
                if cursor - prev < gap_need[li] {
                    cursor = prev + gap_need[li];
                }
            }
            if !judged && li > 0 && cross_need[li] > 0.0 {
                cursor = cursor.max(lcoord[li - 1] + cross_need[li] - before);
            }
            if !judged && li > 0 && track_need[li] > 0.0 {
                let prev = lcoord[li - 1] + prev_after;
                cursor = cursor.max(prev + track_need[li]);
            }
            prev_after = after;
            lcoord[li] = cursor + before;
            cursor += before + after + gap_l;
        }
        (lcoord, cursor - gap_l + MARGIN)
    };
    let (lcoord, total_l) = layer_lines(false);

    // Normalise so the diagram starts at MARGIN (real-node extent).
    let mut minb = f64::INFINITY;
    let mut maxb = f64::NEG_INFINITY;
    for v in 0..n {
        minb = minb.min(bpos[v] - absize[v] / 2.0);
        maxb = maxb.max(bpos[v] + absize[v] / 2.0);
    }
    if n == 0 {
        minb = 0.0;
        maxb = 0.0;
    }
    let shift = MARGIN - minb;
    for v in 0..na {
        bpos[v] += shift;
    }
    let total_b = (maxb - minb) + 2.0 * MARGIN;

    let nodes = (0..n)
        .map(|v| Placed {
            b: bpos[v],
            l: lcoord[alayer[v]],
            bsize: absize[v],
            lsize: alsize[v],
            layer: alayer[v],
        })
        .collect();

    // Edge waypoints (abstract b,l) from each edge's dummy chain, for a
    // renderer that wants to spline the edge through its channel.
    let edge_paths: Vec<Vec<(f64, f64)>> = edge_chain
        .iter()
        .map(|chain| {
            chain
                .iter()
                .map(|&d| (bpos[d], lcoord[alayer[d]]))
                .collect()
        })
        .collect();
    // The search judges the candidate on its judged layer lines (see
    // `layer_lines`).
    let (jl, depth) = if opts.judge {
        layer_lines(true)
    } else {
        (Vec::new(), total_l)
    };
    let (crossings, hits, ext) = if opts.judge {
        let paths: Vec<Vec<(f64, f64)>> = edge_chain
            .iter()
            .map(|chain| chain.iter().map(|&d| (bpos[d], jl[alayer[d]])).collect())
            .collect();
        let at = |v: usize| (bpos[v], jl[alayer[v]], absize[v], alsize[v]);
        (
            drawn_crossings(g, &paths, |v| (bpos[v], jl[alayer[v]])),
            node_hits(g, back, &paths, at),
            chains_b(&bpos),
        )
    } else {
        (0, 0, (0.0, 0.0))
    };
    let mut ckeys: Vec<(usize, f64)> = if clustered {
        cluster_keys(&layers, &apath, &border_side)
            .1
            .into_iter()
            .collect()
    } else {
        Vec::new()
    };
    ckeys.sort_by_key(|k| k.0);

    let edge_groups = edge_chain
        .iter()
        .map(|chain| {
            chain
                .iter()
                .map(|&d| apath.get(d).cloned().unwrap_or_default())
                .collect()
        })
        .collect();
    let order: Option<OrderSeed> = block_paths.map(|_| {
        layers
            .iter()
            .map(|lv| lv.iter().filter_map(|&v| element(v)).collect())
            .collect()
    });

    Placement {
        lo: LayoutResult {
            nodes,
            total_b,
            total_l,
            edge_paths,
        },
        groups: edge_groups,
        crossings,
        hits,
        ckeys,
        ext,
        depth,
        order,
    }
}

/// The largest node outside every subgraph, when there clearly is one:
/// its area must be CENTRE_MIN_SIZE_RATIO times that of every other node
/// outside the subgraphs and of every other real node on its layer.
/// Without such a node (equal boxes, a tie) nothing is centred and the
/// declaration order stands.
fn largest_free_node(
    sizes: &[(f64, f64)],
    node_cluster: Option<&[Vec<usize>]>,
    layers: &[Vec<usize>],
    alayer: &[usize],
) -> Option<usize> {
    let n = sizes.len();
    let area = |v: usize| sizes[v].0 * sizes[v].1;
    let free = |v: usize| node_cluster.map_or(true, |nc| nc[v].is_empty());
    let mut best: Option<usize> = None;
    for v in (0..n).filter(|&v| free(v)) {
        if best.map_or(true, |b| area(v) > area(b)) {
            best = Some(v);
        }
    }
    let v = best?;
    let rivals = (0..n)
        .filter(|&w| free(w))
        .chain(layers[alayer[v]].iter().copied().filter(|&w| w < n));
    let mut rivals = rivals.filter(|&w| w != v);
    rivals
        .all(|w| area(v) >= CENTRE_MIN_SIZE_RATIO * area(w))
        .then_some(v)
}

/// Walk `v` towards the middle slot of its layer by adjacent swaps with
/// nodes of its own subgraph path (never a wall), keeping each swap only
/// while the crossings do not grow (D3). A swap inside layer `li` only
/// changes the crossings of the two layer gaps beside it, so only those
/// are counted ([`gap_crossings`]).
fn centre_in_layer(
    v: usize,
    layers: &mut [Vec<usize>],
    succs: &[Vec<usize>],
    alayer: &[usize],
    pos: &mut [f64],
    apath: &[Vec<usize>],
    border: &[u8],
) {
    let li = alayer[v];
    let mut slot = vec![0usize; alayer.len()];
    for lv in layers.iter() {
        for (i, &w) in lv.iter().enumerate() {
            slot[w] = i;
        }
    }
    let local = |layers: &[Vec<usize>], slot: &[usize]| {
        let above = li
            .checked_sub(1)
            .map_or(0, |up| gap_crossings(&layers[up], succs, alayer, up, slot));
        above + gap_crossings(&layers[li], succs, alayer, li, slot)
    };
    let mut cross = local(layers, &slot);
    loop {
        let len = layers[li].len();
        let Some(i) = layers[li].iter().position(|&w| w == v) else {
            break;
        };
        let j = if i < (len - 1) / 2 {
            i + 1
        } else if i > len / 2 {
            i - 1
        } else {
            break;
        };
        let w = layers[li][j];
        if border.get(w).is_some_and(|&b| b != 0) || apath.get(w) != apath.get(v) {
            break;
        }
        layers[li].swap(i, j);
        slot.swap(v, w);
        let c = local(layers, &slot);
        if c > cross {
            layers[li].swap(i, j);
            slot.swap(v, w);
            break;
        }
        cross = c;
    }
    for (i, &w) in layers[li].iter().enumerate() {
        pos[w] = i as f64;
    }
}

/// Estimated breadth span `(lo, hi)` of what the scene will draw: every
/// node with one FRAME_PAD per subgraph around it, every channel dummy
/// (label slots included), and the lane of every back-edge, planned by
/// the scene's own rule ([`back_lanes`]).
#[allow(clippy::too_many_arguments)]
fn drawn_breadth(
    g: &Graph,
    node_cluster: Option<&[Vec<usize>]>,
    absize: &[f64],
    alsize: &[f64],
    alayer: &[usize],
    bpos: &[f64],
    edge_chain: &[Vec<usize>],
    blocks: Option<&[Vec<usize>]>,
) -> (f64, f64) {
    let n = g.nodes.len();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in 0..n {
        let pad = node_cluster.map_or(0.0, |nc| nc[v].len() as f64 * FRAME_PAD);
        lo = lo.min(bpos[v] - absize[v] / 2.0 - pad);
        hi = hi.max(bpos[v] + absize[v] / 2.0 + pad);
    }
    for &d in edge_chain.iter().flatten() {
        lo = lo.min(bpos[d] - absize[d] / 2.0);
        hi = hi.max(bpos[d] + absize[d] / 2.0);
    }
    let at: Vec<Placed> = (0..n)
        .map(|v| Placed {
            b: bpos[v],
            l: alayer[v] as f64,
            bsize: absize[v],
            lsize: alsize[v],
            layer: alayer[v],
        })
        .collect();
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    for (ei, lane) in back_lanes(g, &at, node_cluster, blocks, edge_chain, |d| {
        (bpos[d], alayer[d] as f64)
    }) {
        let half = crate::lanes::label_breadth(g.edges[ei].label.as_deref(), horizontal) / 2.0;
        lo = lo.min(lane.x - half);
        hi = hi.max(lane.x + half);
    }
    if lo > hi {
        (0.0, 0.0)
    } else {
        (lo, hi)
    }
}

/// The lane of every back-edge that gets one (`(edge, lane)`; a state
/// note card's link never does), by
/// the rule [`crate::lanes`] states — the scene draws them, and the
/// layout centres the drawing on them, from this one function. `at`
/// holds every node with its LAYER INDEX as its `l` (whatever the
/// caller's layer coordinates, the decisions are taken on the layers):
/// what reaches into a back-edge's band is every node on a layer
/// strictly between its ends, and a node on an end's own layer longer
/// along the flow than that end by twice [`crate::lanes::LANE_REACH`]
/// (it reaches that far past the end's side, nodes being centred on
/// their layer). Inside subgraphs
/// (`node_cluster`), the members of the innermost box holding both ends
/// are its obstacles and the rest is foreign ([`crate::lanes::Want`]).
/// The lane is judged for crossings against the skeletons of the other
/// edges (end centres and channel points, `dummy(d)` = a dummy's
/// (breadth, layer) — `edge_chain` lists each edge's), on the layers.
///
/// In a block layout (`blocks`, its [`BlockPaths`]) only an upstream
/// edge between two plain nodes of one level is a lane
/// ([`keeps_lane`]); every other upstream edge is drawn through its
/// channel, or as a step across the gap, and counts among the
/// skeletons.
pub(crate) fn back_lanes(
    g: &Graph,
    at: &[Placed],
    node_cluster: Option<&[Vec<usize>]>,
    blocks: Option<&[Vec<usize>]>,
    edge_chain: &[Vec<usize>],
    dummy: impl Fn(usize) -> (f64, f64),
) -> Vec<(usize, crate::lanes::Lane)> {
    use crate::lanes::{
        blocks_direct, direct_port, label_breadth, plan, port_span, Skeleton, Want, LANE_CLEAR,
    };
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    let drawn = |e: &crate::model::Edge| e.from != e.to && !matches!(e.kind, EdgeKind::Invisible);
    // A state note card's link elbows round as `scene::note_elbow` draws
    // it, or crosses the gap: never a lane.
    let card = |v: usize| {
        let n = &g.nodes[v];
        crate::style::is_note_card(&n.id, n.shape, &n.style)
    };
    let backs: Vec<usize> = (0..g.edges.len())
        .filter(|&ei| {
            let e = &g.edges[ei];
            drawn(e)
                && at[e.to].layer < at[e.from].layer
                && !card(e.from)
                && !card(e.to)
                && keeps_lane(blocks, e.from, e.to)
        })
        .collect();
    if backs.is_empty() {
        return Vec::new();
    }
    let loops = crate::scene::loop_reach(g);
    let skeletons: Vec<Skeleton> = g
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            drawn(e)
                && (at[e.to].layer >= at[e.from].layer
                    || !(keeps_lane(blocks, e.from, e.to) || card(e.from) || card(e.to)))
        })
        .map(|(ei, e)| {
            let mut pts = vec![(at[e.from].b, at[e.from].l)];
            pts.extend(edge_chain.get(ei).into_iter().flatten().map(|&d| dummy(d)));
            pts.push((at[e.to].b, at[e.to].l));
            Skeleton::new((e.from, e.to), pts)
        })
        .collect();
    let shared = |e: &crate::model::Edge| -> Vec<usize> {
        node_cluster.map_or_else(Vec::new, |nc| {
            nc[e.from]
                .iter()
                .zip(&nc[e.to])
                .take_while(|(x, y)| x == y)
                .map(|(x, _)| *x)
                .collect()
        })
    };
    let wants: Vec<Want> = backs
        .iter()
        .map(|&ei| {
            let e = &g.edges[ei];
            let (a, b) = (&at[e.from], &at[e.to]);
            let box_of = shared(e);
            let (mut obstacles, mut foreign) = (Vec::new(), Vec::new());
            let (mut between, mut beside) = (Vec::new(), Vec::new());
            let reach = 2.0 * crate::lanes::LANE_REACH;
            for (v, p) in at.iter().enumerate() {
                if v == e.from || v == e.to {
                    continue;
                }
                // A self-loop beside the node is in the way too.
                let span = (p.b - p.bsize / 2.0, p.b + p.bsize / 2.0 + loops[v]);
                if p.layer > b.layer && p.layer < a.layer {
                    between.push(span);
                } else if p.layer == a.layer && p.lsize > a.lsize + reach {
                    beside.push((span, true));
                } else if p.layer == b.layer && p.lsize > b.lsize + reach {
                    beside.push((span, false));
                } else {
                    continue;
                }
                if node_cluster.map_or(true, |nc| nc[v].starts_with(&box_of)) {
                    obstacles.push(span);
                } else {
                    foreign.push(span);
                }
            }
            let label = label_breadth(e.label.as_deref(), horizontal);
            let (sa, sb) = (g.nodes[e.from].shape, g.nodes[e.to].shape);
            let ports = (direct_port(sa, a, b.b), direct_port(sb, b, a.b));
            Want {
                key: (a.layer - b.layer) as f64,
                band: (b.layer as f64, a.layer as f64),
                obstacles,
                foreign,
                blocked: blocks_direct(&between, &beside, ports, label / 2.0),
                clear: LANE_CLEAR + label / 2.0,
                label_half: label / 2.0,
                spans: [port_span(sa, a), port_span(sb, b)],
                ends: (e.from, e.to),
            }
        })
        .collect();
    let lanes = plan(&wants, |i, x, placed| {
        let e = &g.edges[backs[i]];
        let (a, b) = (&at[e.from], &at[e.to]);
        let path = crate::lanes::judged_path(x, wants[i].spans, a.l, b.l, a.l - 0.5, b.l + 0.5);
        // A planned lane's legs a little nearer its ends than this one's
        // (on the layers both would sit mid-gap, on top of each other, and
        // never properly cross): a leg across the gap then crosses the
        // other lane running through it.
        let planned = crate::lanes::planned_skeletons(&wants, placed, i, |j, xj| {
            let f = &g.edges[backs[j]];
            let (pa, pb) = (&at[f.from], &at[f.to]);
            crate::lanes::judged_path(xj, wants[j].spans, pa.l, pb.l, pa.l - 0.4, pb.l + 0.4)
        });
        let ends = (e.from, e.to);
        crate::lanes::crossings(&path, ends, wants[i].spans, &skeletons)
            + crate::lanes::crossings(&path, ends, wants[i].spans, &planned)
    });
    backs
        .iter()
        .zip(lanes)
        .filter_map(|(&ei, lane)| lane.map(|l| (ei, l)))
        .collect()
}

/// Edge-node pairs where the straight skeleton of a drawn forward edge
/// (end centre, channel waypoints, end centre) runs through a real node
/// other than its ends (G6): a layering that threads its edges between
/// the boxes keeps this low. `at(v)` is `(b, l, bsize, lsize)`.
fn node_hits(
    g: &Graph,
    back: &[bool],
    edge_paths: &[Vec<(f64, f64)>],
    at: impl Fn(usize) -> (f64, f64, f64, f64),
) -> usize {
    let n = g.nodes.len();
    let boxes: Vec<(f64, f64, f64, f64)> = (0..n)
        .map(|v| {
            let (b, l, bs, ls) = at(v);
            (b - bs / 2.0, l - ls / 2.0, b + bs / 2.0, l + ls / 2.0)
        })
        .collect();
    // Nodes by their layer-axis start: a segment only tests the boxes
    // whose start lies before its far end and, within the tallest box's
    // reach, after its near end.
    let mut by_l: Vec<usize> = (0..n).collect();
    by_l.sort_by(|&a, &b| boxes[a].1.total_cmp(&boxes[b].1).then(a.cmp(&b)));
    let starts: Vec<f64> = by_l.iter().map(|&v| boxes[v].1).collect();
    let tallest = boxes.iter().map(|r| r.3 - r.1).fold(0.0f64, f64::max);
    // Per node: the last edge it was counted for (each pair counts once).
    let mut seen = vec![usize::MAX; n];
    let mut total = 0;
    for (ei, e) in g.edges.iter().enumerate() {
        if back[ei] || e.from == e.to || matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        let (a, z) = (at(e.from), at(e.to));
        let mut pts = vec![(a.0, a.1)];
        pts.extend(edge_paths[ei].iter().copied());
        pts.push((z.0, z.1));
        for s in pts.windows(2) {
            let (l0, l1) = (s[0].1.min(s[1].1), s[0].1.max(s[1].1));
            let from = starts.partition_point(|&y| y < l0 - tallest);
            let to = starts.partition_point(|&y| y < l1);
            for &v in &by_l[from..to] {
                if v != e.from && v != e.to && seen[v] != ei && seg_hits_box(s[0], s[1], boxes[v]) {
                    seen[v] = ei;
                    total += 1;
                }
            }
        }
    }
    total
}

/// Does segment `p`-`q` pass through the open box `(x0, y0, x1, y1)`?
/// (Liang-Barsky clipping.)
fn seg_hits_box(p: (f64, f64), q: (f64, f64), r: (f64, f64, f64, f64)) -> bool {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (den, num) in [
        (-dx, p.0 - r.0),
        (dx, r.2 - p.0),
        (-dy, p.1 - r.1),
        (dy, r.3 - p.1),
    ] {
        if den == 0.0 {
            if num <= 0.0 {
                return false;
            }
        } else {
            let t = num / den;
            if den < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    t0 < t1
}

/// Crossings between the straight-line skeletons of the drawn edges
/// (end centre, channel waypoints, end centre), back-edges and flat
/// edges included; edges sharing an end never count. Judges candidate
/// layerings on what will be drawn, where [`count_crossings`] only sees
/// adjacent-layer segments.
fn drawn_crossings(
    g: &Graph,
    edge_paths: &[Vec<(f64, f64)>],
    at: impl Fn(usize) -> (f64, f64),
) -> usize {
    // (from, to, skeleton polyline) per drawn edge.
    type Poly = (usize, usize, Vec<(f64, f64)>);
    let polys: Vec<Poly> = g
        .edges
        .iter()
        .zip(edge_paths)
        .filter(|(e, _)| e.from != e.to && !matches!(e.kind, EdgeKind::Invisible))
        .map(|(e, path)| {
            let mut pts = vec![at(e.from)];
            pts.extend(path.iter().copied());
            pts.push(at(e.to));
            (e.from, e.to, pts)
        })
        .collect();
    let cross = |a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)| -> bool {
        let side = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| {
            (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
        };
        let (d1, d2) = (side(a, b, c), side(a, b, d));
        let (d3, d4) = (side(c, d, a), side(c, d, b));
        d1 * d2 < 0.0 && d3 * d4 < 0.0
    };
    // Every skeleton segment with its edge and layer-axis span, swept
    // along that axis: two segments can only cross where their spans
    // overlap (an open overlap; a segment flat on one layer must lie
    // strictly inside the other's span), so each one is tested against
    // the still-open ones instead of against every other segment.
    // (poly index, start, end, layer-axis lo, layer-axis hi)
    type Seg = (usize, (f64, f64), (f64, f64), f64, f64);
    let mut segs: Vec<Seg> = Vec::new();
    for (pi, (_, _, pts)) in polys.iter().enumerate() {
        for w in pts.windows(2) {
            let (l0, l1) = (w[0].1.min(w[1].1), w[0].1.max(w[1].1));
            segs.push((pi, w[0], w[1], l0, l1));
        }
    }
    segs.sort_by(|a, b| a.3.total_cmp(&b.3).then(a.0.cmp(&b.0)));
    let shares_end = |i: usize, j: usize| {
        let ((fa, ta, _), (fb, tb, _)) = (&polys[i], &polys[j]);
        i == j || fa == fb || fa == tb || ta == fb || ta == tb
    };
    let mut open: Vec<usize> = Vec::new();
    let mut total = 0;
    for (si, s) in segs.iter().enumerate() {
        open.retain(|&o| segs[o].4 > s.3);
        for &o in &open {
            let t = &segs[o];
            if t.3 < s.4 && !shares_end(s.0, t.0) && cross(t.1, t.2, s.1, s.2) {
                total += 1;
            }
        }
        open.push(si);
    }
    total
}

/// The constants the generated sweep pins: the largest component
/// ordered exactly, the lane rule's share, the most starts.
#[cfg(test)]
pub(crate) const BLOCK_ORDER_PINS: (usize, (usize, usize), usize) =
    (BLOCK_ORDER_EXACT_MAX, LANE_SHARE, BLOCK_ORDER_STARTS);

/// The most starts the ordering of a block layout takes ([`BlockOrder`]):
/// the canonical order and eight shuffles of it. A shuffled start finds
/// orders no sweep reaches from the canonical one, and which start does
/// is the draw's: over the block fixtures the least crossings are
/// reached by the seventh start at the latest (a hub and five blocks),
/// over the larger drawings the package was judged on by the ninth; a
/// layered layout elsewhere spends seven by default (ELK's
/// thoroughness).
const BLOCK_ORDER_STARTS: usize = 9;

/// The work the starts of one ordering may cost, in elements (dummies
/// and walls included) plus links, summed over the starts: a drawing of
/// `e` elements takes `BLOCK_ORDER_WORK / e` starts, at least one and at
/// most [`BLOCK_ORDER_STARTS`], so the static scene of a large diagram
/// stays inside its time budget (twice the scene before blocks):
/// about 150 elements and links take all nine, the public fixture's 234
/// five, dense-td's 371 three.
const BLOCK_ORDER_WORK: usize = 1_400;

/// The final order of a block layout's placement, by element: per
/// layer, each real node `(node, usize::MAX)` and each channel dummy
/// `(edge, layer)`. The later placements of one layout call start from
/// the first one's ([`BlockOrder::warm`]).
pub(crate) type OrderSeed = Vec<Vec<(usize, usize)>>;

/// An augmented node's element of an [`OrderSeed`], if it has one (a
/// wall has none).
type ElementOf<'a> = &'a dyn Fn(usize) -> Option<(usize, usize)>;

/// A unit of a level on one layer, as the level's ordering sees it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OrderUnit {
    /// A direct node of the level (a real node; by augmented index).
    Node(usize),
    /// A child block: its members on the layer, kept together.
    Block(usize),
    /// A dummy of a run: the dummies of one channel that lie in the
    /// level itself on consecutive layers.
    Run(usize),
}

/// The ordering of a block layout (level first; see [`BlockOrder::run`]).
struct BlockOrder<'a> {
    apath: &'a [Vec<usize>],
    border: &'a [u8],
    preds: &'a [Vec<usize>],
    succs: &'a [Vec<usize>],
    alayer: &'a [usize],
    nlayers: usize,
    /// Per augmented node: its canonical key as a unit of its own level
    /// — a real node by its index (first mention), a channel dummy by
    /// its edge's source node, then (after a node of that key) by the
    /// target node and the edge. Nothing here reads the order of the
    /// edge statements, two parallel edges of one node pair excepted.
    canon: Vec<(usize, u8, usize, usize)>,
    /// Per subgraph: its least node index, a block's canonical key.
    first: Vec<usize>,
    /// Per augmented node: its run, or `usize::MAX`.
    run: Vec<usize>,
    runs: usize,
    /// The levels, top-down: (subgraph path, first layer, last layer).
    levels: Vec<(Vec<usize>, usize, usize)>,
    /// Per subgraph: its place among its sibling blocks (the level
    /// order's result), one for every layer they share.
    brank: Vec<f64>,
    /// Per augmented node: its slot in its own level on its layer —
    /// just before the block of that rank.
    slot: Vec<f64>,
    /// Per subgraph: the layers it spans.
    bspan: Vec<(usize, usize)>,
    /// Per run: the layers its dummies stand on.
    rspan: Vec<(usize, usize)>,
}

impl BlockOrder<'_> {
    /// One layer's members regrouped: at every level the blocks in
    /// their order, the level's nodes and runs in their slots between
    /// them by `key` (their sweep positions, or nothing: the canonical
    /// order decides), every block's members together.
    fn arrange(&self, items: &[usize], depth: usize, key: &dyn Fn(usize) -> f64) -> Vec<usize> {
        if items.len() <= 1 {
            return items.to_vec();
        }
        let mut groups: Vec<(Option<usize>, Vec<usize>)> = Vec::new();
        let mut index: BTreeMap<usize, usize> = BTreeMap::new();
        for &v in items {
            match self.apath[v].get(depth) {
                Some(&c) => match index.get(&c) {
                    Some(&gi) => groups[gi].1.push(v),
                    None => {
                        index.insert(c, groups.len());
                        groups.push((Some(c), vec![v]));
                    }
                },
                None => groups.push((None, vec![v])),
            }
        }
        type Key = (f64, f64, (usize, u8, usize, usize));
        let mut keyed: Vec<(Key, Option<usize>, Vec<usize>)> = groups
            .into_iter()
            .map(|(c, members)| {
                let v = members[0];
                let k = match (members.len(), self.border[v], c) {
                    // A bare wall anchors to its block's edge.
                    (1, 1, _) => (f64::NEG_INFINITY, 0.0, self.canon[v]),
                    (1, 2, _) => (f64::INFINITY, 0.0, self.canon[v]),
                    (_, _, Some(c)) => (self.brank[c], 0.0, (self.first[c], 0, 0, 0)),
                    _ => (self.slot[v], key(v), self.canon[v]),
                };
                (k, c, members)
            })
            .collect();
        keyed.sort_by(|a, b| {
            a.0 .0
                .total_cmp(&b.0 .0)
                .then(a.0 .1.total_cmp(&b.0 .1))
                .then(a.0 .2.cmp(&b.0 .2))
        });
        let mut out = Vec::with_capacity(items.len());
        for (_, c, members) in keyed {
            if c.is_some() && members.len() > 1 {
                out.extend(self.arrange(&members, depth + 1, key));
            } else {
                out.extend(members);
            }
        }
        out
    }

    /// [`reorder`] with its ties broken by the canonical order (a
    /// dummy's index follows the order of the edge statements).
    fn reorder(&self, layer: &mut Vec<usize>, nbrs: &[Vec<usize>], pos: &mut [f64]) {
        let mut keyed: Vec<(f64, usize)> = layer
            .iter()
            .map(|&v| {
                let key = if nbrs[v].is_empty() {
                    pos[v]
                } else {
                    wmedian(nbrs[v].iter().map(|&u| pos[u]))
                };
                (key, v)
            })
            .collect();
        keyed.sort_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then_with(|| self.canon[a.1].cmp(&self.canon[b.1]))
                .then(a.1.cmp(&b.1))
        });
        layer.clear();
        for (i, (_, v)) in keyed.into_iter().enumerate() {
            layer.push(v);
            pos[v] = i as f64;
        }
    }

    fn regroup(&self, layer: &mut Vec<usize>, pos: &mut [f64]) {
        if layer.len() > 1 {
            let arranged = self.arrange(layer, 0, &|v| pos[v]);
            *layer = arranged;
        }
        for (i, &v) in layer.iter().enumerate() {
            pos[v] = i as f64;
        }
    }

    /// The units of the level `path` on `layer`, in its order, with
    /// where each starts in the layer and how many members it has. They
    /// are contiguous: the level's own walls stand at their two ends.
    fn units(&self, layer: &[usize], path: &[usize]) -> Vec<(OrderUnit, usize, usize)> {
        let k = path.len();
        let mut out: Vec<(OrderUnit, usize, usize)> = Vec::with_capacity(layer.len());
        // (A subgraph has one path: its id at depth `k - 1` is the level.)
        let level = path.last();
        for (i, &v) in layer.iter().enumerate() {
            let p = &self.apath[v];
            if k > 0 && p.get(k - 1) != level {
                continue;
            }
            match p.get(k) {
                Some(&c) => match out.last_mut() {
                    Some((OrderUnit::Block(b), _, len)) if *b == c => *len += 1,
                    _ => out.push((OrderUnit::Block(c), i, 1)),
                },
                None if self.border[v] != 0 => {}
                None if self.run[v] != usize::MAX => out.push((OrderUnit::Run(self.run[v]), i, 1)),
                None => out.push((OrderUnit::Node(v), i, 1)),
            }
        }
        out
    }

    /// Write the level's units back to their layer in the order `order`
    /// (indices into `units`), each block's members as they stand.
    fn place_units(
        layer: &mut [usize],
        pos: &mut [f64],
        units: &[(OrderUnit, usize, usize)],
        order: &[usize],
    ) {
        let Some(&(_, start, _)) = units.first() else {
            return;
        };
        let members: Vec<usize> = order
            .iter()
            .flat_map(|&i| layer[units[i].1..units[i].1 + units[i].2].iter().copied())
            .collect();
        for (i, v) in members.into_iter().enumerate() {
            layer[start + i] = v;
            pos[v] = (start + i) as f64;
        }
    }

    /// Layered crossings in the gaps from layer `from` to layer `to`:
    /// per gap, the upper layer's links in its order, each counted
    /// against the links before it that end further on (a binary
    /// indexed tree over the lower layer's places; the links of one
    /// node never count against each other).
    fn crossings(&self, layers: &[Vec<usize>], pos: &[f64], from: usize, to: usize) -> usize {
        let mut total = 0;
        let mut tree: Vec<u32> = Vec::new();
        for li in from..to.min(self.nlayers.saturating_sub(1)) {
            let n = layers[li + 1].len();
            tree.clear();
            tree.resize(n + 1, 0);
            let mut seen = 0usize;
            for &u in &layers[li] {
                let links = self.succs[u].iter().filter(|&&v| self.alayer[v] == li + 1);
                for &v in links.clone() {
                    // Links seen so far that end at or before `v`.
                    let (mut i, mut upto) = (pos[v] as usize + 1, 0usize);
                    while i > 0 {
                        upto += tree[i] as usize;
                        i &= i - 1;
                    }
                    total += seen - upto;
                }
                for &v in links {
                    let mut i = pos[v] as usize + 1;
                    while i <= n {
                        tree[i] += 1;
                        i += i & i.wrapping_neg();
                    }
                    seen += 1;
                }
            }
        }
        total
    }

    /// One half sweep of a level's order (down: every layer by the
    /// layer above it; up: by the layer below). Each unit's barycentre
    /// is the mean place of its links' other ends in that layer, links
    /// inside one child block left out; a long unit (a block, a run)
    /// takes ONE key for all its layers, the mean of its barycentres,
    /// so two blocks keep one order on every layer they share and a run
    /// stays on one side of a block. Each layer is then the long units
    /// in that order with the level's nodes merged in by their own
    /// barycentres; ties fall back to the canonical order.
    fn half_sweep(
        &self,
        path: &[usize],
        (lo, hi): (usize, usize),
        down: bool,
        layers: &mut [Vec<usize>],
        pos: &mut [f64],
    ) {
        let k = path.len();
        let ns = self.first.len();
        let mut block = vec![(0.0f64, 0.0f64); ns];
        let mut runs = vec![(0.0f64, 0.0f64); self.runs];
        let mut own = vec![0.0f64; self.alayer.len()];
        let order: Vec<usize> = if down {
            (lo..=hi).collect()
        } else {
            (lo..=hi).rev().collect()
        };
        let canon = |u: OrderUnit, at: usize, layer: &[usize]| match u {
            OrderUnit::Block(c) => (self.first[c], 0, 0, 0),
            _ => self.canon[layer[at]],
        };
        for &l in &order {
            let units = self.units(&layers[l], path);
            if units.is_empty() {
                continue;
            }
            let beside = if down {
                l.checked_sub(1)
            } else {
                (l + 1 < self.nlayers).then_some(l + 1)
            };
            let len = layers[l].len() as f64;
            let mut keys: Vec<f64> = Vec::with_capacity(units.len());
            for &(u, at, n) in &units {
                // Whole numbers, so the order the links are listed in
                // (the order of the edge statements) cannot move a key.
                let (mut sum, mut links) = (0usize, 0usize);
                let mut blen = 1.0;
                if let Some(b) = beside {
                    blen = layers[b].len().max(1) as f64;
                    for &w in &layers[l][at..at + n] {
                        let nbrs = if down { &self.preds[w] } else { &self.succs[w] };
                        for &x in nbrs {
                            // (Inside the same child block: its id at
                            // this depth says so.)
                            let inside = matches!(u, OrderUnit::Block(c)
                                if self.apath[x].get(k) == Some(&c));
                            if self.alayer[x] == b && !inside {
                                sum += pos[x] as usize;
                                links += 1;
                            }
                        }
                    }
                }
                let key = if links > 0 {
                    (sum as f64 / links as f64 + 0.5) / blen
                } else {
                    (at as f64 + n as f64 / 2.0) / len
                };
                // Running mean for a long unit, so the sweep carries on
                // from where its earlier layers put it.
                let key = match u {
                    OrderUnit::Block(c) => {
                        block[c] = (block[c].0 + key, block[c].1 + 1.0);
                        block[c].0 / block[c].1
                    }
                    OrderUnit::Run(r) => {
                        runs[r] = (runs[r].0 + key, runs[r].1 + 1.0);
                        runs[r].0 / runs[r].1
                    }
                    OrderUnit::Node(v) => {
                        own[v] = key;
                        key
                    }
                };
                keys.push(key);
            }
            let mut by: Vec<usize> = (0..units.len()).collect();
            by.sort_by(|&a, &b| keys[a].total_cmp(&keys[b]).then(a.cmp(&b)));
            Self::place_units(&mut layers[l], pos, &units, &by);
        }
        // Every layer by the final keys.
        for l in lo..=hi {
            let units = self.units(&layers[l], path);
            let key = |u: OrderUnit| match u {
                OrderUnit::Block(c) => block[c].0 / block[c].1.max(1.0),
                OrderUnit::Run(r) => runs[r].0 / runs[r].1.max(1.0),
                OrderUnit::Node(v) => own[v],
            };
            let mut by: Vec<usize> = (0..units.len()).collect();
            by.sort_by(|&a, &b| {
                key(units[a].0).total_cmp(&key(units[b].0)).then_with(|| {
                    canon(units[a].0, units[a].1, &layers[l])
                        .cmp(&canon(units[b].0, units[b].1, &layers[l]))
                })
            });
            Self::place_units(&mut layers[l], pos, &units, &by);
        }
    }

    /// The layers a long unit of a level stands on.
    fn long_span(&self, u: OrderUnit) -> Option<(usize, usize)> {
        match u {
            OrderUnit::Block(c) => Some(self.bspan[c]),
            OrderUnit::Run(r) => Some(self.rspan[r]),
            OrderUnit::Node(_) => None,
        }
    }

    /// What exchanging two neighbours of one layer does to the layered
    /// crossings: `a` (the left one) and `b`, as `(start, len)` ranges
    /// of `layer`. `beside` holds, for the layer before and the layer
    /// after, the two units' ranges there when they change places on it
    /// in the same exchange: a pair of links that flips at both ends
    /// keeps its state. `scratch` is two buffers, kept between calls.
    fn exchange_delta(
        &self,
        layer: &[usize],
        (a, b): ((usize, usize), (usize, usize)),
        pos: &[f64],
        beside: [Option<((usize, usize), (usize, usize))>; 2],
        scratch: &mut (Vec<usize>, Vec<usize>),
    ) -> isize {
        let mut delta = 0isize;
        let l = self.alayer[layer[a.0]];
        let (xs, ys) = scratch;
        for (nbrs, other, both) in [
            (self.preds, l.wrapping_sub(1), beside[0]),
            (self.succs, l + 1, beside[1]),
        ] {
            let ends = |range: (usize, usize), out: &mut Vec<usize>| {
                out.clear();
                for &v in &layer[range.0..range.0 + range.1] {
                    out.extend(
                        nbrs[v]
                            .iter()
                            .filter(|&&x| self.alayer[x] == other)
                            .map(|&x| pos[x] as usize),
                    );
                }
                out.sort_unstable();
            };
            ends(a, xs);
            if xs.is_empty() {
                continue;
            }
            ends(b, ys);
            if ys.is_empty() {
                continue;
            }
            // The left unit is left now: a pair of links crosses while
            // the left one's far end is the further on.
            let (mut below, mut upto) = (0usize, 0usize);
            for &x in xs.iter() {
                while below < ys.len() && ys[below] < x {
                    below += 1;
                }
                while upto < ys.len() && ys[upto] <= x {
                    upto += 1;
                }
                delta += (ys.len() - upto) as isize - below as isize;
            }
            if let Some((ra, rb)) = both {
                let within = |r: (usize, usize), v: &[usize]| {
                    v.iter().filter(|&&p| p >= r.0 && p < r.0 + r.1).count() as isize
                };
                delta -= within(ra, xs) * within(rb, ys);
                delta += within(rb, xs) * within(ra, ys);
            }
        }
        delta
    }

    /// The weighted adjacent exchange of a level's order, after each
    /// half sweep: two units that stand next to each other on every
    /// layer they share change places — on all of those layers at once,
    /// so two blocks keep one order and a run one side of a block —
    /// when that lowers the layered crossings, every link of theirs
    /// counted (a bundle weighs its multiplicity). A barycentre gives a
    /// long unit ONE key for all its layers, the mean of what each
    /// layer asks for; the exchange is what then tries the neighbour
    /// that mean passed over (the public fixture's crossings fall from
    /// 9 to 6 with it, dense-td's layered ones by a quarter). With
    /// `tidy`, the level's nodes and runs are what is exchanged, a pair
    /// whose left one is later in the canonical order, also when the
    /// crossings only stay ([`BlockOrder::tidy`]). Each exchange lowers
    /// the crossings or the inversions against the canonical order, so
    /// it ends. Returns whether anything moved.
    fn exchange(
        &self,
        path: &[usize],
        (lo, hi): (usize, usize),
        tidy: bool,
        layers: &mut [Vec<usize>],
        pos: &mut [f64],
    ) -> bool {
        type Range = (usize, usize);
        let mut units: Vec<Vec<(OrderUnit, usize, usize)>> =
            (lo..=hi).map(|l| self.units(&layers[l], path)).collect();
        let canon = |u: OrderUnit, at: usize, layer: &[usize]| match u {
            OrderUnit::Block(c) => (self.first[c], 0, 0, 0),
            _ => self.canon[layer[at]],
        };
        let mut shared: Vec<(usize, usize, Range, Range)> = Vec::new();
        let mut scratch: (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
        let mut moved = false;
        let mut again = true;
        let mut passes = 0;
        while again && passes < EXCHANGE_PASSES {
            again = false;
            passes += 1;
            for l in lo..=hi {
                let mut i = 0;
                while i + 1 < units[l - lo].len() {
                    let (a, b) = (units[l - lo][i], units[l - lo][i + 1]);
                    i += 1;
                    // The layers the two share, with their places there.
                    shared.clear();
                    match (self.long_span(a.0), self.long_span(b.0)) {
                        (Some(sa), Some(sb)) => {
                            let (from, to) = (sa.0.max(sb.0), sa.1.min(sb.1));
                            // Judged once, from the first layer they share.
                            if from != l {
                                continue;
                            }
                            let mut apart = false;
                            for m in from..=to {
                                let us = &units[m - lo];
                                match us.iter().position(|u| u.0 == a.0) {
                                    Some(ia) if us.get(ia + 1).is_some_and(|u| u.0 == b.0) => {
                                        let (ua, ub) = (us[ia], us[ia + 1]);
                                        shared.push((m, ia, (ua.1, ua.2), (ub.1, ub.2)));
                                    }
                                    _ => {
                                        apart = true;
                                        break;
                                    }
                                }
                            }
                            if apart {
                                continue;
                            }
                        }
                        _ => shared.push((l, i - 1, (a.1, a.2), (b.1, b.2))),
                    }
                    if tidy {
                        let later = canon(a.0, a.1, &layers[l]) > canon(b.0, b.1, &layers[l]);
                        let block = |u: OrderUnit| matches!(u, OrderUnit::Block(_));
                        if !later || block(a.0) || block(b.0) {
                            continue;
                        }
                    }
                    // A pair of links that flips at one end only is met
                    // from that end; one that flips at both (the two
                    // units change places on the layer beside this one
                    // too), never.
                    let mut delta = 0isize;
                    for (si, &(m, _, ra, rb)) in shared.iter().enumerate() {
                        let on = |k: Option<usize>, layer: usize| {
                            k.and_then(|k| shared.get(k))
                                .filter(|s| s.0 == layer)
                                .map(|s| (s.2, s.3))
                        };
                        let beside = [
                            on(si.checked_sub(1), m.wrapping_sub(1)),
                            on(Some(si + 1), m + 1),
                        ];
                        delta +=
                            self.exchange_delta(&layers[m], (ra, rb), pos, beside, &mut scratch);
                    }
                    if delta > 0 || (delta == 0 && !tidy) {
                        continue;
                    }
                    for &(m, ia, ra, rb) in &shared {
                        layers[m][ra.0..rb.0 + rb.1].rotate_left(ra.1);
                        for at in ra.0..rb.0 + rb.1 {
                            pos[layers[m][at]] = at as f64;
                        }
                        let us = &mut units[m - lo];
                        us.swap(ia, ia + 1);
                        us[ia].1 = ra.0;
                        us[ia + 1].1 = ra.0 + rb.1;
                    }
                    moved = true;
                    again = true;
                }
            }
        }
        moved
    }

    /// The order of one level's units (the level `li` of `levels`):
    /// from the order they stand in and from its reverse, half sweeps
    /// down and up with an adjacent exchange after each half
    /// ([`BlockOrder::exchange`]), judged after each half by the layered
    /// crossings of the layers the level spans (and the gaps just
    /// outside it, whose far layers stay as the levels above left
    /// them); the least wins, the earlier on a tie. The result is each
    /// child block's rank and each node's and run dummy's slot between
    /// the blocks, which the sweeps over the whole graph then keep.
    fn order_level(&mut self, li: usize, layers: &mut [Vec<usize>], pos: &mut [f64]) {
        let (path, lo, hi) = self.levels[li].clone();
        let span = (lo, hi);
        let judge = |me: &Self, layers: &[Vec<usize>], pos: &[f64]| {
            me.crossings(layers, pos, lo.saturating_sub(1), hi + 1)
        };
        let restore = |layers: &mut [Vec<usize>], pos: &mut [f64], from: &[Vec<usize>]| {
            layers[lo..=hi].clone_from_slice(from);
            for lv in &layers[lo..=hi] {
                for (i, &v) in lv.iter().enumerate() {
                    pos[v] = i as f64;
                }
            }
        };
        let long = (lo..=hi).any(|l| {
            self.units(&layers[l], &path)
                .iter()
                .any(|u| !matches!(u.0, OrderUnit::Node(_)))
        });
        if long {
            let base: Vec<Vec<usize>> = layers[lo..=hi].to_vec();
            let mut best = (judge(self, layers, pos), base.clone());
            for start in 0..2 {
                if best.0 == 0 {
                    break;
                }
                restore(layers, pos, &base);
                // This start's own least: its rounds go on while they
                // lower it, whatever the other start reached.
                let mut least = best.0;
                if start == 1 {
                    for l in lo..=hi {
                        let units = self.units(&layers[l], &path);
                        let by: Vec<usize> = (0..units.len()).rev().collect();
                        Self::place_units(&mut layers[l], pos, &units, &by);
                    }
                    least = judge(self, layers, pos);
                    if least < best.0 {
                        best = (least, layers[lo..=hi].to_vec());
                    }
                }
                for _ in 0..LEVEL_ROUNDS {
                    let before = least;
                    for down in [true, false] {
                        self.half_sweep(&path, span, down, layers, pos);
                        self.exchange(&path, span, false, layers, pos);
                        let c = judge(self, layers, pos);
                        least = least.min(c);
                        if c < best.0 {
                            best = (c, layers[lo..=hi].to_vec());
                        }
                    }
                    if least == 0 || least >= before {
                        break;
                    }
                }
            }
            restore(layers, pos, &best.1);
        }
        // The blocks' ranks: one sequence that every layer's order is a
        // part of.
        let k = path.len();
        let mut blocks: Vec<usize> = Vec::new();
        let mut after: Vec<(usize, usize)> = Vec::new();
        for lv in &layers[lo..=hi] {
            let mut prev: Option<usize> = None;
            for (u, _, _) in self.units(lv, &path) {
                if let OrderUnit::Block(c) = u {
                    if !blocks.contains(&c) {
                        blocks.push(c);
                    }
                    if let Some(p) = prev {
                        after.push((p, c));
                    }
                    prev = Some(c);
                }
            }
        }
        blocks.sort_by_key(|&c| self.first[c]);
        let mut rank = 0.0;
        let mut left: Vec<usize> = blocks.clone();
        while !left.is_empty() {
            // The first block (canonically) no other waits before.
            let at = left
                .iter()
                .position(|&c| !after.iter().any(|&(p, q)| q == c && left.contains(&p)))
                .unwrap_or(0);
            let c = left.remove(at);
            self.brank[c] = rank;
            rank += 1.0;
        }
        for lv in &layers[lo..=hi] {
            let units = self.units(lv, &path);
            let mut next = rank;
            for &(u, at, _) in units.iter().rev() {
                match u {
                    OrderUnit::Block(c) => next = self.brank[c],
                    _ => self.slot[lv[at]] = next - 0.5,
                }
            }
            // The level's own walls keep to its ends.
            for &v in lv {
                if self.border[v] != 0 && self.apath[v].len() == k && self.apath[v] == path {
                    self.slot[v] = 0.0;
                }
            }
        }
    }

    /// The sweeps over the whole graph inside the slots the level
    /// orders left: weighted-median reorders and adjacent swaps as
    /// before blocks, every layer regrouped as soon as it is reordered,
    /// judged after the downward half and after the upward half (judged
    /// after the upward half only, every chain parks where its target
    /// is). Returns the crossings of the order kept.
    fn sweep(&self, layers: &mut Vec<Vec<usize>>, pos: &mut [f64]) -> usize {
        let n = self.nlayers;
        for lv in layers.iter_mut() {
            self.regroup(lv, pos);
        }
        let mut best = (self.crossings(layers, pos, 0, n), layers.clone());
        for _ in 0..SLOT_ROUNDS {
            if best.0 == 0 {
                break;
            }
            let before = best.0;
            for li in 1..n {
                self.reorder(&mut layers[li], self.preds, pos);
                self.regroup(&mut layers[li], pos);
            }
            let c = self.crossings(layers, pos, 0, n);
            if c < best.0 {
                best = (c, layers.clone());
            }
            for li in (0..n.saturating_sub(1)).rev() {
                self.reorder(&mut layers[li], self.succs, pos);
                self.regroup(&mut layers[li], pos);
            }
            transpose(
                layers,
                self.preds,
                self.succs,
                pos,
                n,
                Some(self.apath),
                Some(self.border),
            );
            for lv in layers.iter_mut() {
                self.regroup(lv, pos);
            }
            let c = self.crossings(layers, pos, 0, n);
            if c < best.0 {
                best = (c, layers.clone());
            }
            if best.0 >= before {
                break;
            }
        }
        *layers = best.1;
        for lv in layers.iter() {
            for (i, &v) in lv.iter().enumerate() {
                pos[v] = i as f64;
            }
        }
        best.0
    }

    /// The canonical order: on every layer the units of each level form
    /// one sequence by key, a block's members together inside it.
    fn canonical(&mut self, layers: &mut [Vec<usize>], pos: &mut [f64]) {
        self.brank.iter_mut().for_each(|r| *r = 0.0);
        self.slot.iter_mut().for_each(|s| *s = 0.0);
        for lv in layers.iter_mut() {
            if lv.len() > 1 {
                let arranged = self.arrange(lv, 0, &|_| 0.0);
                *lv = arranged;
            }
            for (i, &v) in lv.iter().enumerate() {
                pos[v] = i as f64;
            }
        }
    }

    /// Start `k > 0`, for the level `li`: the canonical order shuffled —
    /// one permutation of the level's long units, and per layer one of
    /// its own nodes; never a layer as a whole, so two blocks start in
    /// one order on every layer and a run on one side of a block. One
    /// generator stream per (start, level) — a level's draw then depends
    /// on its own units alone, not on how many the levels before it
    /// drew — the long units first, then the layers in ascending order;
    /// integer arithmetic only.
    fn shuffle(&self, li: usize, k: usize, layers: &mut [Vec<usize>], pos: &mut [f64]) {
        const MUL: u64 = 6_364_136_223_846_793_005;
        const INC: u64 = 1_442_695_040_888_963_407;
        let stream = k + SHUFFLE_STREAMS * li;
        let mut x = (stream as u64).wrapping_mul(MUL).wrapping_add(INC);
        let mut draw = |below: usize| -> usize {
            x = x.wrapping_mul(MUL).wrapping_add(INC);
            ((x >> 33) % below as u64) as usize
        };
        let mut fisher_yates = |n: usize| -> Vec<usize> {
            let mut p: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                p.swap(i, draw(i + 1));
            }
            p
        };
        let (path, lo, hi) = &self.levels[li];
        // The level's long units, in the order they are first met.
        let mut long: Vec<OrderUnit> = Vec::new();
        for lv in &layers[*lo..=*hi] {
            for (u, _, _) in self.units(lv, path) {
                if !matches!(u, OrderUnit::Node(_)) && !long.contains(&u) {
                    long.push(u);
                }
            }
        }
        let perm = fisher_yates(long.len());
        let place = |u: OrderUnit| long.iter().position(|&x| x == u).map_or(0, |i| perm[i]);
        for l in *lo..=*hi {
            let units = self.units(&layers[l], path);
            let (mut longs, mut shorts): (Vec<usize>, Vec<usize>) =
                (0..units.len()).partition(|&i| !matches!(units[i].0, OrderUnit::Node(_)));
            longs.sort_by_key(|&i| place(units[i].0));
            let p = fisher_yates(shorts.len());
            shorts = p.iter().map(|&i| shorts[i]).collect();
            // The layer's pattern of long and short places stays.
            let (mut li, mut si) = (0, 0);
            let by: Vec<usize> = units
                .iter()
                .map(|u| {
                    if matches!(u.0, OrderUnit::Node(_)) {
                        si += 1;
                        shorts[si - 1]
                    } else {
                        li += 1;
                        longs[li - 1]
                    }
                })
                .collect();
            Self::place_units(&mut layers[l], pos, &units, &by);
        }
    }

    /// The order a later placement of one layout call starts from: on
    /// each layer the elements `seed` (the first placement's final
    /// order) has there stay in that order, and a node or dummy new to
    /// the layer goes before the first kept element that follows it in
    /// the canonical order (at the end without one), inside its block.
    /// Returns whether any element is new.
    fn warm(
        &mut self,
        seed: &OrderSeed,
        element: ElementOf<'_>,
        layers: &mut [Vec<usize>],
        pos: &mut [f64],
    ) -> bool {
        self.canonical(layers, pos);
        let mut fresh = false;
        let mut key = vec![0.0f64; self.alayer.len()];
        for (l, lv) in layers.iter().enumerate() {
            let kept: BTreeMap<(usize, usize), usize> = seed
                .get(l)
                .into_iter()
                .flatten()
                .enumerate()
                .map(|(i, &e)| (e, i))
                .collect();
            // Canonical order of the layer, back to front: each new
            // element takes the key just before the kept one after it.
            let mut next = kept.len() as f64;
            for &v in lv.iter().rev() {
                match element(v).and_then(|e| kept.get(&e)) {
                    Some(&i) => {
                        next = i as f64;
                        key[v] = next;
                    }
                    None => {
                        fresh |= self.border[v] == 0;
                        key[v] = next - 0.5;
                    }
                }
            }
        }
        // A block's place: the mean of its members' keys over all its
        // layers (one for every layer, as the level orders keep it).
        let ns = self.first.len();
        let mut acc = vec![(0.0f64, 0.0f64); ns];
        for (v, path) in self.apath.iter().enumerate() {
            if self.border[v] != 0 {
                continue;
            }
            let share = key[v] / layers[self.alayer[v]].len().max(1) as f64;
            for &c in path {
                acc[c] = (acc[c].0 + share, acc[c].1 + 1.0);
            }
        }
        for c in 0..ns {
            self.brank[c] = acc[c].0 / acc[c].1.max(1.0);
        }
        for lv in layers.iter_mut() {
            let len = lv.len().max(1) as f64;
            for &v in lv.iter() {
                self.slot[v] = key[v] / len;
            }
            if lv.len() > 1 {
                let arranged = self.arrange(lv, 0, &|_| 0.0);
                *lv = arranged;
            }
            for (i, &v) in lv.iter().enumerate() {
                pos[v] = i as f64;
            }
        }
        fresh
    }

    /// How many pairs of a level's own nodes stand against the
    /// canonical (source) order on a layer they share.
    fn node_inversions(
        &self,
        path: &[usize],
        (lo, hi): (usize, usize),
        layers: &[Vec<usize>],
    ) -> usize {
        let mut total = 0;
        for lv in &layers[lo..=hi] {
            let nodes: Vec<usize> = self
                .units(lv, path)
                .iter()
                .filter_map(|&(u, at, _)| matches!(u, OrderUnit::Node(_)).then_some(lv[at]))
                .collect();
            for (i, &v) in nodes.iter().enumerate() {
                total += nodes[i + 1..]
                    .iter()
                    .filter(|&&w| self.canon[v] > self.canon[w])
                    .count();
            }
        }
        total
    }

    /// Source order among equals, for the members a channel holds out
    /// of place. A channel that ends on a member of its own level (an
    /// edge out of a block's last row, up past the rows before it)
    /// passes those rows on one side, and the sweeps settle for the
    /// member on that side of its row whatever the source says: taking
    /// the member back alone would cross the channel over its row
    /// mates' links, and moving the channel alone crosses it too. So
    /// the two are tried together: a channel that ends on a node out of
    /// source order goes, with that node, to the end the node belongs
    /// on — every run of it to that end of its own level, on every
    /// layer it has — the levels it passes are tidied again, and the
    /// change is kept when the layered crossings do not rise and the
    /// nodes are nearer their source order. At most [`FLIP_TRIES`]
    /// channels are tried, so a dense drawing, where a member out of
    /// source order is where its links want it, pays a bounded price.
    fn flip_chains(&self, layers: &mut [Vec<usize>], pos: &mut [f64]) {
        let judge = |me: &Self, layers: &[Vec<usize>], pos: &[f64]| {
            let inversions: usize = me
                .levels
                .iter()
                .map(|(path, lo, hi)| me.node_inversions(path, (*lo, *hi), layers))
                .sum();
            (me.crossings(layers, pos, 0, me.nlayers), inversions)
        };
        let span_of = |path: &[usize]| match path.last() {
            Some(&c) => self.bspan[c],
            None => (0, self.nlayers.saturating_sub(1)),
        };
        let mut now = judge(self, layers, pos);
        let mut tries = 0;
        // Each kept change lowers (crossings, inversions): it ends.
        while now.1 > 0 && tries < FLIP_TRIES {
            // The channels that end on a node standing out of source
            // order in its level: (edge, node, the end the node belongs
            // on — `true`: the low one).
            let mut cands: Vec<(usize, usize, bool)> = Vec::new();
            for (path, lo, hi) in &self.levels {
                for lv in &layers[*lo..=*hi] {
                    let nodes: Vec<usize> = self
                        .units(lv, path)
                        .iter()
                        .filter_map(|&(u, at, _)| matches!(u, OrderUnit::Node(_)).then_some(lv[at]))
                        .collect();
                    for (i, &v) in nodes.iter().enumerate() {
                        for &w in &nodes[i + 1..] {
                            if self.canon[v] <= self.canon[w] {
                                continue;
                            }
                            for (node, low) in [(w, true), (v, false)] {
                                for &x in self.preds[node].iter().chain(&self.succs[node]) {
                                    let edge = self.canon[x].3;
                                    if self.run[x] != usize::MAX
                                        && self.apath[x] == *path
                                        && !cands.contains(&(edge, node, low))
                                    {
                                        cands.push((edge, node, low));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let mut kept = false;
            for (edge, node, low) in cands {
                if tries == FLIP_TRIES {
                    break;
                }
                tries += 1;
                let before: Vec<Vec<usize>> = layers.to_vec();
                // The channel's dummies and the node, each to the end of
                // its own level on its layer.
                let mut paths: Vec<&[usize]> = Vec::new();
                for v in 0..self.alayer.len() {
                    let dummy = self.run[v] != usize::MAX && self.canon[v].3 == edge;
                    if !dummy && v != node {
                        continue;
                    }
                    let (l, path) = (self.alayer[v], self.apath[v].as_slice());
                    if !paths.contains(&path) {
                        paths.push(path);
                    }
                    let units = self.units(&layers[l], path);
                    let (Some(first), Some(last)) = (units.first(), units.last()) else {
                        continue;
                    };
                    let Some(at) = layers[l].iter().position(|&x| x == v) else {
                        continue;
                    };
                    let end = if low { first.1 } else { last.1 + last.2 - 1 };
                    let d = layers[l].remove(at);
                    layers[l].insert(end, d);
                    for (i, &x) in layers[l].iter().enumerate() {
                        pos[x] = i as f64;
                    }
                }
                for path in paths {
                    self.exchange(path, span_of(path), true, layers, pos);
                }
                let then = judge(self, layers, pos);
                if then.0 <= now.0 && then < now {
                    now = then;
                    kept = true;
                    break;
                }
                layers.clone_from_slice(&before);
                for lv in layers.iter() {
                    for (i, &v) in lv.iter().enumerate() {
                        pos[v] = i as f64;
                    }
                }
            }
            if !kept {
                break;
            }
        }
    }

    /// Source order among equals: after the winning start, two units of
    /// one level that stand next to each other (two members of a block,
    /// two nodes or runs between the same blocks) change places when
    /// the left one is later in the canonical order and the layered
    /// crossings do not rise ([`BlockOrder::exchange`]); each exchange
    /// removes one inversion against the canonical order, so it ends.
    /// (A shuffled start leaves the members that change no crossing
    /// wherever the shuffle put them.)
    fn tidy(&self, layers: &mut [Vec<usize>], pos: &mut [f64]) {
        for (path, lo, hi) in &self.levels {
            self.exchange(path, (*lo, *hi), true, layers, pos);
        }
    }

    /// One start's orders, from the order the layers stand in: the level
    /// orders top-down (each level's blocks and runs first, then inside
    /// each block), then the sweeps inside their slots; again while a
    /// pass lowers the layered crossings, at most [`LEVEL_PASSES`]
    /// times. Returns the layered crossings.
    fn settle(&mut self, layers: &mut Vec<Vec<usize>>, pos: &mut [f64]) -> usize {
        let mut best: Option<(usize, Vec<Vec<usize>>)> = None;
        for _ in 0..LEVEL_PASSES {
            for li in 0..self.levels.len() {
                self.order_level(li, layers, pos);
            }
            let c = self.sweep(layers, pos);
            if best.as_ref().map_or(true, |b| c < b.0) {
                best = Some((c, layers.clone()));
            } else {
                break;
            }
            if c == 0 {
                break;
            }
        }
        let (c, order) = best.unwrap_or((0, layers.clone()));
        *layers = order;
        for lv in layers.iter() {
            for (i, &v) in lv.iter().enumerate() {
                pos[v] = i as f64;
            }
        }
        c
    }

    /// The ordering of a block layout, level first.
    ///
    /// Every level's units are ordered before the members inside them:
    /// its child blocks and the channels passing it as whole units
    /// ([`BlockOrder::order_level`]), the root first, then each block's
    /// inside with everything outside it fixed. Flat sweeps then order
    /// the nodes and channel dummies inside the slots that leaves
    /// ([`BlockOrder::sweep`]). Flat sweeps alone cannot move a block
    /// past its sibling, and a block-swap judged by crossings changes
    /// nothing useful.
    ///
    /// This runs from `starts` starts — the canonical order, then
    /// shuffles of it ([`BlockOrder::shuffle`]) — and the start with the
    /// fewest layered crossings wins, the lowest on a tie; then the
    /// crossing-neutral neighbours go back to source order
    /// ([`BlockOrder::tidy`], [`BlockOrder::flip_chains`]). With `seed`
    /// (a later placement of the same layout call) there is one run,
    /// from the first placement's final order ([`BlockOrder::warm`]):
    /// its layers differ from the winner's, and from a start of its own
    /// it would lose to the multi-start best by construction.
    fn run(
        &mut self,
        starts: usize,
        seed: Option<(&OrderSeed, ElementOf<'_>)>,
        layers: &mut Vec<Vec<usize>>,
        pos: &mut [f64],
    ) {
        if let Some((seed, element)) = seed {
            if self.warm(seed, element, layers, pos) {
                self.settle(layers, pos);
            }
            self.tidy(layers, pos);
            return;
        }
        self.canonical(layers, pos);
        let base = layers.clone();
        let mut best: Option<(usize, Vec<Vec<usize>>)> = None;
        for k in 0..starts.max(1) {
            *layers = base.clone();
            for lv in layers.iter() {
                for (i, &v) in lv.iter().enumerate() {
                    pos[v] = i as f64;
                }
            }
            if k > 0 {
                for li in 0..self.levels.len() {
                    self.shuffle(li, k, layers, pos);
                }
            }
            let c = self.settle(layers, pos);
            if best.as_ref().map_or(true, |b| c < b.0) {
                best = Some((c, layers.clone()));
            }
            if c == 0 {
                break;
            }
        }
        if let Some((_, order)) = best {
            *layers = order;
        }
        for lv in layers.iter() {
            for (i, &v) in lv.iter().enumerate() {
                pos[v] = i as f64;
            }
        }
        self.tidy(layers, pos);
        // (Once a layout call: a later placement starts from this order.)
        self.flip_chains(layers, pos);
    }
}

/// Most passes of level orders and sweeps from one start: the first
/// orders each level's blocks while their insides still stand as the
/// start left them, the later ones with the insides ordered (the least
/// layered crossings of dense-td's starts fall from 57 to 38 with the
/// third); a pass that lowers nothing is the last.
const LEVEL_PASSES: usize = 3;
/// The generator streams a level has for its shuffles
/// ([`BlockOrder::shuffle`]): one per start, with room past
/// [`BLOCK_ORDER_STARTS`] for a count forced by a report. Pinned with
/// the starts (the crossings by start of the block fixtures): a level's
/// stream is `start + SHUFFLE_STREAMS * level`.
const SHUFFLE_STREAMS: usize = 21;
/// Most rounds (a half sweep down, one up) of a level's order.
const LEVEL_ROUNDS: usize = 12;
/// Most passes of one adjacent exchange over a level's layers.
const EXCHANGE_PASSES: usize = 8;
/// Most channels [`BlockOrder::flip_chains`] tries to move to the other
/// side of their levels in one ordering.
const FLIP_TRIES: usize = 12;
/// Most rounds of the sweeps inside the slots.
const SLOT_ROUNDS: usize = 16;

/// Reorder one layer by the weighted median of each node's neighbour
/// positions (dagre's heuristic — more robust to outliers than the
/// barycenter mean). Nodes without neighbours keep their position;
/// ties break by node index so the pass is deterministic.
fn reorder(layer: &mut Vec<usize>, nbrs: &[Vec<usize>], pos: &mut [f64]) {
    let mut keyed: Vec<(f64, usize)> = layer
        .iter()
        .map(|&v| {
            let ns = &nbrs[v];
            let key = if ns.is_empty() {
                pos[v]
            } else {
                wmedian(ns.iter().map(|&u| pos[u]))
            };
            (key, v)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    layer.clear();
    for (i, (_, v)) in keyed.into_iter().enumerate() {
        layer.push(v);
        pos[v] = i as f64;
    }
}

/// Dagre's weighted median of a set of neighbour positions. For an even
/// count the two central values are blended by the widths of the gaps
/// on either side, which biases towards the denser cluster.
fn wmedian(vals: impl Iterator<Item = f64>) -> f64 {
    let mut ps: Vec<f64> = vals.collect();
    ps.sort_by(f64::total_cmp);
    let m = ps.len();
    match m {
        0 => -1.0,
        1 => ps[0],
        2 => (ps[0] + ps[1]) / 2.0,
        _ => {
            let mid = m / 2;
            if m % 2 == 1 {
                ps[mid]
            } else {
                let left = ps[mid - 1] - ps[0];
                let right = ps[m - 1] - ps[mid];
                if left + right == 0.0 {
                    (ps[mid - 1] + ps[mid]) / 2.0
                } else {
                    (ps[mid - 1] * right + ps[mid] * left) / (left + right)
                }
            }
        }
    }
}

/// Local adjacent-swap pass (dagre's transpose). Repeatedly walk every
/// layer and swap neighbouring nodes whenever doing so lowers the count
/// of crossings they induce with the layers above and below, running to
/// convergence like dagre (a generous cap guards pathological inputs).
/// In clustered mode only same-cluster-path pairs swap — a cross-group
/// swap would be undone by the contiguity pass anyway (its "gain" was
/// an illusion), and border walls never move off their group's edge.
fn transpose(
    layers: &mut [Vec<usize>],
    preds: &[Vec<usize>],
    succs: &[Vec<usize>],
    pos: &mut [f64],
    nlayers: usize,
    apath: Option<&[Vec<usize>]>,
    border: Option<&[u8]>,
) {
    let mut improved = true;
    let mut guard = 0;
    while improved && guard < 20 {
        improved = false;
        guard += 1;
        for li in 0..nlayers {
            let len = layers[li].len();
            for i in 0..len.saturating_sub(1) {
                let v = layers[li][i];
                let w = layers[li][i + 1];
                if border.is_some_and(|b| b[v] != 0 || b[w] != 0)
                    || apath.is_some_and(|p| p[v] != p[w])
                {
                    continue;
                }
                let before = local_crossings(v, w, preds, pos) + local_crossings(v, w, succs, pos);
                let after = local_crossings(w, v, preds, pos) + local_crossings(w, v, succs, pos);
                if after < before {
                    layers[li].swap(i, i + 1);
                    pos[v] = (i + 1) as f64;
                    pos[w] = i as f64;
                    improved = true;
                }
            }
        }
    }
}

/// Crossings induced by placing `v` immediately left of `w`: every pair
/// of edges (v→a, w→b) into the same adjacent layer crosses when a sits
/// to the right of b.
fn local_crossings(v: usize, w: usize, nbrs: &[Vec<usize>], pos: &[f64]) -> usize {
    let mut c = 0;
    for &a in &nbrs[v] {
        for &b in &nbrs[w] {
            if pos[a] > pos[b] {
                c += 1;
            }
        }
    }
    c
}

/// Total edge crossings across every adjacent-layer boundary, counted
/// as position inversions among the lower endpoints. Only genuine
/// layer+1 segments participate (back/invisible links route elsewhere).
fn count_crossings(
    layers: &[Vec<usize>],
    succs: &[Vec<usize>],
    alayer: &[usize],
    nlayers: usize,
) -> usize {
    let mut pos = vec![0usize; alayer.len()];
    for lv in layers {
        for (i, &v) in lv.iter().enumerate() {
            pos[v] = i;
        }
    }
    (0..nlayers.saturating_sub(1))
        .map(|li| gap_crossings(&layers[li], succs, alayer, li, &pos))
        .sum()
}

/// Crossings in the gap between layer `li` (whose order is `upper`) and
/// layer `li + 1`, as position inversions among the lower endpoints;
/// `pos` is each node's slot in its layer.
fn gap_crossings(
    upper: &[usize],
    succs: &[Vec<usize>],
    alayer: &[usize],
    li: usize,
    pos: &[usize],
) -> usize {
    let mut es: Vec<(usize, usize)> = Vec::new();
    for &u in upper {
        for &v in &succs[u] {
            if alayer[v] == li + 1 {
                es.push((pos[u], pos[v]));
            }
        }
    }
    es.sort_unstable();
    let mut lower: Vec<usize> = es.iter().map(|e| e.1).collect();
    let mut buf = vec![0usize; lower.len()];
    inversions(&mut lower, &mut buf)
}

/// Number of pairs `i < j` with `a[i] > a[j]` (strict), by merge sort in
/// O(n log n); `a` comes back sorted, `buf` is scratch of the same length.
fn inversions(a: &mut [usize], buf: &mut [usize]) -> usize {
    let n = a.len();
    if n < 2 {
        return 0;
    }
    let mid = n / 2;
    let mut total = inversions(&mut a[..mid], &mut buf[..mid]);
    total += inversions(&mut a[mid..], &mut buf[mid..]);
    let (mut i, mut j) = (0, mid);
    for slot in buf.iter_mut() {
        // Equal values are not crossings: the left one goes first.
        if j == n || (i < mid && a[i] <= a[j]) {
            *slot = a[i];
            i += 1;
        } else {
            // a[j] jumps every left value still waiting: all are larger.
            total += mid - i;
            *slot = a[j];
            j += 1;
        }
    }
    a.copy_from_slice(buf);
    total
}

/// Brandes-Köpf coordinate assignment (breadth axis). Runs the four
/// vertical alignments (down/up × left/right), compacts each into a
/// non-overlapping layout, aligns them to the narrowest, and returns
/// the per-node median of the four candidates. Long-edge dummy chains
/// share a block per alignment, so they come out as straight vertical
/// runs — the hallmark of dagre's output.
#[allow(clippy::too_many_arguments)] // internal: one bundle of layered-graph state
fn coordinates_bk(
    n: usize,
    na: usize,
    nlayers: usize,
    layers: &[Vec<usize>],
    preds: &[Vec<usize>],
    succs: &[Vec<usize>],
    alayer: &[usize],
    absize: &[f64],
    apath: Option<&[Vec<usize>]>,
    border: Option<&[u8]>,
    header_b: Option<&[f64]>,
) -> Vec<f64> {
    if na == 0 {
        return Vec::new();
    }

    // Adjacent-layer neighbour sets (back/invisible links that skip a
    // layer are excluded — BK only reasons about layer±1 segments). A
    // 2-cycle lists its pair once (see the layering above); parallel
    // forward edges are listed once per edge on purpose (report Q8).
    let mut up: Vec<Vec<usize>> = vec![Vec::new(); na];
    let mut down: Vec<Vec<usize>> = vec![Vec::new(); na];
    for v in 0..na {
        for &u in &preds[v] {
            if alayer[u] + 1 == alayer[v] {
                up[v].push(u);
            }
        }
        for &w in &succs[v] {
            if alayer[v] + 1 == alayer[w] {
                down[v].push(w);
            }
        }
    }

    // Position of each node within its layer (natural, un-adjusted).
    let mut order0 = vec![0usize; na];
    for lay in layers {
        for (i, &v) in lay.iter().enumerate() {
            order0[v] = i;
        }
    }

    let conflicts = type1_conflicts(n, nlayers, layers, &up, &order0);

    // Four candidate assignments, keyed by (vert_up, horiz_right).
    let mut cands: Vec<Vec<f64>> = Vec::with_capacity(4);
    for &vert_up in &[false, true] {
        for &horiz_right in &[false, true] {
            // Build the adjusted layering: reverse layer order for the
            // "up" sweeps, reverse within-layer order for the "right".
            let mut al: Vec<Vec<usize>> = layers.to_vec();
            if vert_up {
                al.reverse();
            }
            if horiz_right {
                for lay in al.iter_mut() {
                    lay.reverse();
                }
            }
            let mut aorder = vec![0usize; na];
            for lay in &al {
                for (i, &v) in lay.iter().enumerate() {
                    aorder[v] = i;
                }
            }
            // Align towards the already-processed layer: predecessors
            // for a downward sweep, successors for an upward one.
            let neighbor = if vert_up { &down } else { &up };
            let (root, _align) = vertical_alignment(na, &al, neighbor, &aorder, &conflicts, border);
            let mut xs = horizontal_compaction(n, na, &al, &root, absize, apath, border, header_b);
            if horiz_right {
                for x in xs.iter_mut() {
                    *x = -*x;
                }
            }
            cands.push(xs);
        }
    }

    // Align the four to the narrowest, then take each node's median.
    align_candidates(&mut cands, n, absize);
    let mut bpos = vec![0.0f64; na];
    for v in 0..na {
        let mut q = [cands[0][v], cands[1][v], cands[2][v], cands[3][v]];
        q.sort_by(f64::total_cmp);
        bpos[v] = (q[1] + q[2]) / 2.0;
    }
    bpos
}

/// Mark type-1 conflicts: a non-inner segment that crosses an inner
/// segment (one strung between two dummy nodes). Aligning across such a
/// pair would kink the long edge, so BK forbids it. Stored symmetrically.
fn type1_conflicts(
    n: usize,
    nlayers: usize,
    layers: &[Vec<usize>],
    up: &[Vec<usize>],
    order: &[usize],
) -> std::collections::HashSet<(usize, usize)> {
    let is_dummy = |v: usize| v >= n;
    let mut conflicts = std::collections::HashSet::new();
    for li in 1..nlayers {
        let lower = &layers[li];
        let prev_len = layers[li - 1].len();
        let mut k0 = 0usize;
        let mut scan = 0usize;
        for (l1, &v) in lower.iter().enumerate() {
            // Inner segment: v is a dummy whose upper neighbour is a dummy.
            let w = if is_dummy(v) {
                up[v].iter().copied().find(|&u| is_dummy(u))
            } else {
                None
            };
            let is_last = l1 + 1 == lower.len();
            if w.is_some() || is_last {
                let k1 = w.map(|ww| order[ww]).unwrap_or(prev_len);
                for &scan_node in &lower[scan..=l1] {
                    for &u in &up[scan_node] {
                        let upos = order[u];
                        if (upos < k0 || upos > k1) && !(is_dummy(u) && is_dummy(scan_node)) {
                            let pair = if u < scan_node {
                                (u, scan_node)
                            } else {
                                (scan_node, u)
                            };
                            conflicts.insert(pair);
                        }
                    }
                }
                scan = l1 + 1;
                k0 = k1;
            }
        }
    }
    conflicts
}

/// One BK vertical alignment. Walks layers top-to-bottom (in adjusted
/// order); each node tries to align with its median neighbour in the
/// already-placed layer, forming blocks identified by a shared root.
///
/// With cluster `border` walls, every wall dummy always aligns with its
/// own chain neighbour, so each box side is ONE straight block, and no
/// other node may align across a wall's segment — otherwise a bent
/// wall lets one layer's members bulge into a sibling's band and the
/// drawn frames intersect.
fn vertical_alignment(
    na: usize,
    al: &[Vec<usize>],
    neighbor: &[Vec<usize>],
    aorder: &[usize],
    conflicts: &std::collections::HashSet<(usize, usize)>,
    border: Option<&[u8]>,
) -> (Vec<usize>, Vec<usize>) {
    let mut root: Vec<usize> = (0..na).collect();
    let mut align: Vec<usize> = (0..na).collect();
    let is_wall = |v: usize| border.is_some_and(|b| b[v] != 0);
    for lay in al {
        let mut prev_idx: i64 = -1;
        // Per slot: the neighbour-layer position of the next wall to
        // its right — an alignment must stay left of it.
        let mut next_wall = vec![i64::MAX; lay.len()];
        let mut nxt = i64::MAX;
        for (i, &v) in lay.iter().enumerate().rev() {
            next_wall[i] = nxt;
            if is_wall(v) {
                if let Some(&w) = neighbor[v].first() {
                    nxt = aorder[w] as i64;
                }
            }
        }
        for (i, &v) in lay.iter().enumerate() {
            if is_wall(v) {
                if let Some(&w) = neighbor[v].first() {
                    align[w] = v;
                    root[v] = root[w];
                    align[v] = root[w];
                    prev_idx = aorder[w] as i64;
                }
                continue;
            }
            let mut ws: Vec<usize> = neighbor[v].clone();
            if ws.is_empty() {
                continue;
            }
            ws.sort_by_key(|&w| aorder[w]);
            let m = ws.len();
            let lo = (m - 1) / 2;
            let hi = m / 2;
            for &w in &ws[lo..=hi] {
                let pair = if v < w { (v, w) } else { (w, v) };
                if align[v] == v
                    && prev_idx < aorder[w] as i64
                    && (aorder[w] as i64) < next_wall[i]
                    && !conflicts.contains(&pair)
                {
                    align[w] = v;
                    root[v] = root[w];
                    align[v] = root[w];
                    prev_idx = aorder[w] as i64;
                }
            }
        }
    }
    (root, align)
}

/// Compact the blocks of one alignment along the breadth axis. Builds
/// the block graph (min-separation edges between consecutive roots in a
/// layer), pushes every block as far left as its predecessors allow,
/// then pulls it back right toward its successors without overlap.
/// `header_b` is the title-strip room, per cluster, to keep outside
/// each box's LEFT wall (low breadth = screen top in LR/RL); `None`
/// when the strip lies on the layer axis instead.
#[allow(clippy::too_many_arguments)] // internal: one bundle of layered-graph state
fn horizontal_compaction(
    n: usize,
    na: usize,
    al: &[Vec<usize>],
    root: &[usize],
    absize: &[f64],
    apath: Option<&[Vec<usize>]>,
    border: Option<&[u8]>,
    header_b: Option<&[f64]>,
) -> Vec<f64> {
    // Block graph as adjacency lists over root nodes.
    let mut bin: Vec<Vec<(usize, f64)>> = vec![Vec::new(); na]; // (pred_root, sep)
    let mut bout: Vec<Vec<(usize, f64)>> = vec![Vec::new(); na]; // (succ_root, sep)
    let mut is_block = vec![false; na];
    for lay in al {
        let mut prev: Option<usize> = None;
        for &v in lay {
            let vr = root[v];
            is_block[vr] = true;
            if let Some(u) = prev {
                let ur = root[u];
                let sep = pair_sep(u, v, n, absize, apath, border, header_b);
                // Merge parallel separations by their max.
                if let Some(e) = bout[ur].iter_mut().find(|(t, _)| *t == vr) {
                    if sep > e.1 {
                        e.1 = sep;
                    }
                    if let Some(e2) = bin[vr].iter_mut().find(|(t, _)| *t == ur) {
                        e2.1 = e.1;
                    }
                } else {
                    bout[ur].push((vr, sep));
                    bin[vr].push((ur, sep));
                }
            }
            prev = Some(v);
        }
    }

    let mut xs = vec![0.0f64; na];
    // Pass 1 — leftmost feasible: post-order DFS so every predecessor
    // block is placed before the block that leans on it.
    let mut visited = vec![false; na];
    let mut stack: Vec<usize> = (0..na).filter(|&v| is_block[v]).collect();
    while let Some(elem) = stack.pop() {
        if visited[elem] {
            let mut x = 0.0f64;
            for &(p, sep) in &bin[elem] {
                x = x.max(xs[p] + sep);
            }
            xs[elem] = x;
        } else {
            visited[elem] = true;
            stack.push(elem);
            for &(p, _) in &bin[elem] {
                stack.push(p);
            }
        }
    }
    // Pass 2 — pull right toward successors to centre, never overlapping.
    let mut visited2 = vec![false; na];
    let mut stack2: Vec<usize> = (0..na).filter(|&v| is_block[v]).collect();
    while let Some(elem) = stack2.pop() {
        if visited2[elem] {
            let mut min = f64::INFINITY;
            for &(s, sep) in &bout[elem] {
                min = min.min(xs[s] - sep);
            }
            if min.is_finite() {
                xs[elem] = xs[elem].max(min);
            }
        } else {
            visited2[elem] = true;
            stack2.push(elem);
            for &(s, _) in &bout[elem] {
                stack2.push(s);
            }
        }
    }

    // Project block coordinates back onto every member node.
    let mut out = vec![0.0f64; na];
    for v in 0..na {
        out[v] = xs[root[v]];
    }
    out
}

/// Breadth compaction after Brandes-Köpf, for ribbon drawings (G12).
///
/// BK places whole vertical blocks, and a chain of separations that
/// zig-zags between blocks across layers can leave every layer three
/// times broader than what it holds. Here every node is its own unit
/// (a subgraph's border walls move as one straight unit, so frames
/// stay rectangles that clear their neighbours). The units are first
/// packed to the left, which gives the least breadth the layer order
/// allows; then Gauss-Seidel sweeps place each unit at the weighted
/// median of its adjacent-layer neighbours, clamped between its layer
/// neighbours' separations and that breadth, so the order is kept,
/// nothing overlaps and the drawing never grows back. Edge links weigh
/// more the more dummies they join (Gansner's 1/2/8), which keeps long
/// channels straight; a wall with nothing to follow hugs its members.
#[allow(clippy::too_many_arguments)] // internal: one bundle of layered-graph state
fn compact_breadth(
    n: usize,
    layers: &[Vec<usize>],
    preds: &[Vec<usize>],
    succs: &[Vec<usize>],
    alayer: &[usize],
    absize: &[f64],
    apath: Option<&[Vec<usize>]>,
    border: Option<&[u8]>,
    header_b: Option<&[f64]>,
    bpos: &mut [f64],
) {
    let na = bpos.len();
    let wall = |v: usize| border.map_or(0, |b| b[v]);
    // Unit per node: a wall joins the unit of the wall above it.
    let mut unit: Vec<usize> = (0..na).collect();
    for lv in layers {
        for &v in lv {
            if wall(v) != 0 {
                if let Some(&p) = preds[v].iter().find(|&&p| wall(p) == wall(v)) {
                    unit[v] = unit[p];
                }
            }
        }
    }
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); na];
    for v in 0..na {
        members[unit[v]].push(v);
    }
    // Layer slot per node, for its left and right neighbours.
    let mut slot = vec![0usize; na];
    for lv in layers {
        for (i, &v) in lv.iter().enumerate() {
            slot[v] = i;
        }
    }
    // Weighted adjacent-layer links (walls only pin bands).
    let weight = |u: usize, v: usize| match (u < n, v < n) {
        (true, true) => 1.0,
        (false, false) => COMPACT_DUMMY_WEIGHT,
        _ => COMPACT_MIXED_WEIGHT,
    };
    let mut links: Vec<Vec<(usize, f64)>> = vec![Vec::new(); na];
    for v in 0..na {
        if wall(v) != 0 {
            continue;
        }
        for &w in preds[v].iter().chain(&succs[v]) {
            if wall(w) == 0 && alayer[w].abs_diff(alayer[v]) == 1 {
                links[v].push((w, weight(v, w)));
            }
        }
    }
    let sep = |u: usize, v: usize| pair_sep(u, v, n, absize, apath, border, header_b);
    // 1. Pack every unit as far left as its layer neighbours allow (the
    // longest path over the left-of relation), which fixes the least
    // breadth `right` this layer order can have.
    let mut next: Vec<Vec<(usize, f64)>> = vec![Vec::new(); na];
    let mut indeg = vec![0usize; na];
    for lv in layers {
        for w in lv.windows(2) {
            next[unit[w[0]]].push((unit[w[1]], sep(w[0], w[1])));
            indeg[unit[w[1]]] += 1;
        }
    }
    let mut x = vec![0.0f64; na];
    for v in 0..na {
        x[unit[v]] = x[unit[v]].max(absize[v] / 2.0);
    }
    let mut queue: Vec<usize> = (0..na).filter(|&u| unit[u] == u && indeg[u] == 0).collect();
    let mut done = 0;
    while let Some(u) = queue.pop() {
        done += 1;
        for &(w, s) in &next[u] {
            x[w] = x[w].max(x[u] + s);
            indeg[w] -= 1;
            if indeg[w] == 0 {
                queue.push(w);
            }
        }
    }
    if done != (0..na).filter(|&u| unit[u] == u).count() {
        return; // the walls cross: keep Brandes-Köpf's placement
    }
    let right = (0..na)
        .map(|v| x[unit[v]] + absize[v] / 2.0)
        .fold(0.0f64, f64::max);
    for v in 0..na {
        bpos[v] = x[unit[v]];
    }
    // 2. Re-centre within that breadth.
    let mut seen = vec![usize::MAX; na];
    for round in 0..COMPACT_SWEEPS {
        let down = round % 2 == 0;
        let rightwards = round % 4 < 2;
        let order: Vec<usize> = if down {
            (0..layers.len()).collect()
        } else {
            (0..layers.len()).rev().collect()
        };
        for li in order {
            let lv = &layers[li];
            let idx: Vec<usize> = if rightwards {
                (0..lv.len()).collect()
            } else {
                (0..lv.len()).rev().collect()
            };
            for i in idx {
                let u = unit[lv[i]];
                if seen[u] == round {
                    continue;
                }
                seen[u] = round;
                let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
                let mut pull: Vec<(f64, f64)> = Vec::new();
                for &m in &members[u] {
                    let lay = &layers[alayer[m]];
                    let s = slot[m];
                    lo = lo.max(if s > 0 {
                        let l = lay[s - 1];
                        bpos[l] + sep(l, m)
                    } else {
                        absize[m] / 2.0
                    });
                    hi = hi.min(match lay.get(s + 1) {
                        Some(&r) => bpos[r] - sep(m, r),
                        None => right - absize[m] / 2.0,
                    });
                    pull.extend(links[m].iter().map(|&(w, wt)| (bpos[w], wt)));
                }
                if lo > hi {
                    continue; // float noise on a tight unit: leave it
                }
                let cur = bpos[members[u][0]];
                let want = if !pull.is_empty() {
                    weighted_median(&mut pull)
                } else {
                    match wall(members[u][0]) {
                        1 => hi,
                        2 => lo,
                        _ => cur,
                    }
                };
                let x = want.max(lo).min(hi);
                if x.is_finite() {
                    for &m in &members[u] {
                        bpos[m] = x;
                    }
                }
            }
        }
    }
}

/// The point that halves the total weight of `(position, weight)`
/// pairs (the lower one on an exact tie, the mean of the two middle
/// positions when the halves balance).
fn weighted_median(vals: &mut [(f64, f64)]) -> f64 {
    vals.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let total: f64 = vals.iter().map(|v| v.1).sum();
    let mut acc = 0.0;
    for (i, &(x, w)) in vals.iter().enumerate() {
        acc += w;
        if acc * 2.0 > total {
            return x;
        }
        if acc * 2.0 == total {
            return (x + vals.get(i + 1).map_or(x, |v| v.0)) / 2.0;
        }
    }
    vals.last().map_or(0.0, |v| v.0)
}

/// Least centre distance between `u` and its right-hand layer
/// neighbour `v`. Each node contributes half its separation class
/// (dagre): a real node half of GAP_B, an edge dummy half of EDGE_GAP —
/// so parallel edge channels bundle tightly while real nodes keep their
/// room.
fn pair_sep(
    u: usize,
    v: usize,
    n: usize,
    absize: &[f64],
    apath: Option<&[Vec<usize>]>,
    border: Option<&[u8]>,
    header_b: Option<&[f64]>,
) -> f64 {
    let half = |v: usize| if v < n { GAP_B / 2.0 } else { EDGE_GAP / 2.0 };
    // Widen the gap across a subgraph boundary so each box gets its
    // padding and cross-cluster channels have room.
    let cgap = apath.map_or(0.0, |p| cluster_gap(&p[u], &p[v]));
    // A wall hugs its members (only pins the band); ordinary neighbours
    // pay their separation-class halves.
    let is_wall = border.is_some_and(|b| b[u] != 0 || b[v] != 0);
    let base = if is_wall {
        BORDER_GAP
    } else {
        half(u) + half(v)
    };
    // A left wall next to anything outside its box: that is the side
    // the box's title strip is drawn on.
    let hdr = match (apath, border, header_b) {
        (Some(p), Some(b), Some(h)) => {
            let outside = |w: usize, x: usize| b[w] == 1 && !p[x].starts_with(&p[w]);
            // The strip room of the wall's own box.
            let room = |w: usize| p[w].last().map_or(0.0, |&c| h[c]);
            match (outside(u, v), outside(v, u)) {
                (true, true) => room(u).max(room(v)),
                (true, false) => room(u),
                (false, true) => room(v),
                (false, false) => 0.0,
            }
        }
        _ => 0.0,
    };
    absize[u] / 2.0 + base + absize[v] / 2.0 + cgap + hdr
}

/// Shift the four alignments so they share a reference frame, then leave
/// them for the median blend. Left-biased alignments align on their min
/// edge, right-biased on their max, matching dagre's `alignCoordinates`.
fn align_candidates(cands: &mut [Vec<f64>], n: usize, absize: &[f64]) {
    // Pick the narrowest (by real-node extent) as the anchor frame.
    let extent = |xs: &[f64]| -> (f64, f64) {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for v in 0..n {
            lo = lo.min(xs[v] - absize[v] / 2.0);
            hi = hi.max(xs[v] + absize[v] / 2.0);
        }
        (lo, hi)
    };
    let mut anchor = 0usize;
    let mut best_w = f64::INFINITY;
    for (i, xs) in cands.iter().enumerate() {
        let (lo, hi) = extent(xs);
        if hi - lo < best_w {
            best_w = hi - lo;
            anchor = i;
        }
    }
    let (amin, amax) = extent(&cands[anchor]);
    for (i, xs) in cands.iter_mut().enumerate() {
        if i == anchor {
            continue;
        }
        let (lo, hi) = extent(xs);
        // Even indices are left-biased (l), odd are right-biased (r).
        let delta = if i % 2 == 0 { amin - lo } else { amax - hi };
        if delta != 0.0 {
            for x in xs.iter_mut() {
                *x += delta;
            }
        }
    }
}

/// Longest shared prefix of two cluster paths (their deepest common
/// ancestor subgraph, as a path).
fn common_prefix(a: &[usize], b: &[usize]) -> Vec<usize> {
    a.iter()
        .zip(b)
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| *x)
        .collect()
}

/// Breadth padding to reserve between two adjacent nodes whose cluster
/// paths differ — one `CLUSTER_PAD` per subgraph boundary crossed
/// (leaving one nesting level + entering another counts both).
fn cluster_gap(a: &[usize], b: &[usize]) -> f64 {
    let common = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let boundaries = (a.len() - common) + (b.len() - common);
    boundaries as f64 * CLUSTER_PAD
}

/// Ordering keys shared by every layer: per node its normalised in-layer
/// position `(i + 0.5) / len`, and per cluster the mean of that over all
/// its non-wall members on ALL layers.
type OrderKeys = (Vec<f64>, std::collections::HashMap<usize, f64>);

/// One ordering key per cluster for the whole diagram, so sibling
/// groups keep the same left-to-right order on every layer they share
/// (dagre gets the same guarantee from its subgraph constraint graph).
/// Ordering each layer by its own mean let two siblings swap sides
/// between layers, which crosses their frames.
fn cluster_keys(layers: &[Vec<usize>], apath: &[Vec<usize>], border: &[u8]) -> OrderKeys {
    let mut nkey = vec![0.0f64; apath.len()];
    let mut acc: std::collections::HashMap<usize, (f64, f64)> = std::collections::HashMap::new();
    for lv in layers {
        let len = lv.len().max(1) as f64;
        for (i, &v) in lv.iter().enumerate() {
            nkey[v] = (i as f64 + 0.5) / len;
            if border[v] != 0 {
                continue;
            }
            for &c in &apath[v] {
                let e = acc.entry(c).or_insert((0.0, 0.0));
                e.0 += nkey[v];
                e.1 += 1.0;
            }
        }
    }
    let ckey = acc.into_iter().map(|(c, (s, k))| (c, s / k)).collect();
    (nkey, ckey)
}

/// Reorder one layer so every subgraph's members are contiguous
/// (recursively for nested subgraphs), while ordering the groups and
/// free nodes by their diagram-wide keys ([`cluster_keys`]) — keeping
/// the crossing-minimised arrangement as intact as the contiguity
/// constraint allows.
fn enforce_contiguity(
    layer: &mut Vec<usize>,
    apath: &[Vec<usize>],
    border: &[u8],
    pos: &mut [f64],
    keys: &OrderKeys,
) {
    if layer.len() <= 1 {
        return;
    }
    let arranged = arrange(layer, 0, apath, border, keys);
    *layer = arranged;
    for (i, &v) in layer.iter().enumerate() {
        pos[v] = i as f64;
    }
}

/// Recursive helper for [`enforce_contiguity`]: at `depth`, split items
/// into subgraph groups (by cluster id at that depth) and free
/// singletons, order those units by their keys, then recurse into
/// each multi-member group at `depth + 1`.
fn arrange(
    items: &[usize],
    depth: usize,
    apath: &[Vec<usize>],
    border: &[u8],
    keys: &OrderKeys,
) -> Vec<usize> {
    if items.len() <= 1 {
        return items.to_vec();
    }
    let cid = |v: usize| -> Option<usize> {
        let p = &apath[v];
        if p.len() > depth {
            Some(p[depth])
        } else {
            None
        }
    };
    // Group by cluster id at this depth, preserving first-seen order;
    // each free node (no cluster here) becomes its own unit.
    let mut groups: Vec<(Option<usize>, Vec<usize>)> = Vec::new();
    let mut index: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for &v in items {
        match cid(v) {
            Some(c) => {
                if let Some(&gi) = index.get(&c) {
                    groups[gi].1.push(v);
                } else {
                    index.insert(c, groups.len());
                    groups.push((Some(c), vec![v]));
                }
            }
            None => groups.push((None, vec![v])),
        }
    }
    let mut keyed: Vec<(f64, Option<usize>, Vec<usize>)> = groups
        .into_iter()
        .map(|(c, members)| {
            // A bare wall dummy anchors to its group's edge: the left
            // wall sorts before everything, the right wall after.
            let key = match (members.len(), border[members[0]], c) {
                (1, 1, _) => f64::NEG_INFINITY,
                (1, 2, _) => f64::INFINITY,
                (_, _, Some(c)) => keys.1.get(&c).copied().unwrap_or(keys.0[members[0]]),
                _ => keys.0[members[0]],
            };
            (key, c, members)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::with_capacity(items.len());
    for (_, c, members) in keyed {
        if c.is_some() && members.len() > 1 {
            out.extend(arrange(&members, depth + 1, apath, border, keys));
        } else {
            out.extend(members);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// fanout-frames' shape (report M11): a hub feeding a column of
    /// frames of very different heights is a ribbon, but stacking one
    /// frame under another (beside the column) grew the drawing's area
    /// and threaded that frame's edge past its sibling. Every frame now
    /// keeps the hub's next layer: one column, as mermaid draws it.
    #[test]
    fn a_hub_fed_column_of_frames_stays_one_column() {
        let mut src = String::from("flowchart LR\nhub[\"hub node with a long label\"]\n");
        for (k, n) in [5usize, 3, 6, 2, 1, 3].into_iter().enumerate() {
            src += &format!("subgraph g{k}[\"frame {k}\"]\ndirection TB\n");
            for i in 0..n {
                src += &format!("m{k}_{i}[\"member {i} of frame {k}<br/>second line\"]\n");
            }
            src += "end\n";
        }
        for k in 0..6 {
            src += &format!("hub --> g{k}\n");
        }
        let g = crate::parser::parse(&src).unwrap();
        let sc = crate::scene::scene(&g);
        let left: Vec<f64> = sc.clusters.iter().map(|c| c.x).collect();
        let hub = &sc.nodes[0];
        assert!(
            left.iter().all(|&x| x > hub.x + hub.w / 2.0),
            "every frame right of the hub: {left:?}"
        );
        let (lo, hi) = left
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |a, &x| {
                (a.0.min(x), a.1.max(x))
            });
        let widest = sc.clusters.iter().map(|c| c.w).fold(0.0, f64::max);
        assert!(
            hi - lo < widest / 2.0,
            "a frame stepped out of the column: lefts {left:?}"
        );
    }

    /// A box's loose parts sit on its top layer (report §2.9, B8 review
    /// r1 #1): a hub feeding a box through one member no longer leaves
    /// the box's edge-less members level with the hub (its edges struck
    /// through their frame to reach the member), a chain only the box's
    /// own members join starts on the member's row, and a nested box
    /// nothing links out of takes its parent's top row. A box nothing
    /// outside the boxes feeds keeps its place. A node outside the box
    /// that only the seated sources link into moves down with them
    /// (`src2 -> out2`, B8b r1); one that something else ranks too
    /// keeps the box where it was (`src3 -> out3` beside `pin -> out3`).
    #[test]
    fn a_box_seats_its_loose_parts_on_its_top_layer() {
        let src = "flowchart TB\nhub\nsubgraph g[box]\nlinked\nloose1\nloose2\nc1 --> c2\n\
                   subgraph inner[inner]\ni1\nend\nend\nsubgraph island[island]\nx1\nend\n\
                   subgraph o[outer]\nsrc2\nsubgraph in2[in2]\nx2\ny2\nend\nend\n\
                   subgraph p[pinned]\nsrc3\nfed3\nend\n\
                   hub --> linked\nsrc2 --> out2\nhub --> y2\n\
                   hub --> fed3\nsrc3 --> out3\npin --> out3\n";
        let g = crate::parser::parse(src).unwrap();
        let sc = crate::scene::scene(&g);
        let y = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap().y;
        let row = y("linked");
        assert!(y("hub") < row - 30.0, "the hub above its target");
        if crate::levels::blocks_on() {
            // Laid out as blocks, every box the hub feeds lies wholly
            // below the hub — also the pinned one, whose source no
            // longer stays level with the hub — and the box nothing
            // feeds starts on the first row.
            for id in [
                "loose1", "loose2", "c1", "i1", "src2", "x2", "y2", "src3", "fed3",
            ] {
                assert!(y(id) > y("hub") + 30.0, "{id} not below the hub");
            }
            assert!(y("c2") > y("c1") + 30.0, "the chain keeps its own step");
            assert!((y("x1") - y("hub")).abs() < 1.0);
            return;
        }
        for id in ["loose1", "loose2", "c1", "i1"] {
            assert!(
                (y(id) - row).abs() < 1.0,
                "{id} at {} off the box's row {row}",
                y(id)
            );
        }
        assert!(y("c2") > row + 30.0, "the chain keeps its own step");
        // The island box has no feeder: it stays on the first row.
        assert!((y("x1") - y("hub")).abs() < 1.0);
        // Seating the outer box's source src2 pushes out2, outside it,
        // a row down; nothing else ranks out2, so it moves along and
        // the whole box sits on the hub's next row.
        for id in ["src2", "x2", "y2"] {
            assert!((y(id) - row).abs() < 1.0, "{id} off the box's row");
        }
        assert!(y("out2") > row + 30.0);
        // out3 also hangs below `pin`, a top-level node on the first
        // row: seating src3 would stretch pin's line, so the box keeps
        // src3 on the hub's row (dense-lr: 25 of its 52 nodes a layer
        // down otherwise).
        assert!((y("src3") - y("hub")).abs() < 1.0);
        assert!(y("fed3") > y("src3") + 30.0);
    }

    /// A box a hub feeds sits wholly below the hub (B8b P1), with its
    /// members nothing ranks into on its first row: a source member that
    /// links out of the box (f182's `n13`) no longer stays level with
    /// the hub, dragging the frame up across the hub's row and its
    /// edges, and the box that source leads into follows it, wholly a
    /// row further down, so the line between them leaves one row for
    /// the next. A source that leads to a member the hub feeds takes
    /// the first row too, a row above that member (B8b r1).
    #[test]
    fn a_hub_fed_box_sits_below_its_hub() {
        let src = "flowchart TB\nhub\n\
                   subgraph g[box]\nfed\nsrc\nloose\nend\n\
                   subgraph h[next]\nother\nalso\nend\n\
                   subgraph k[kept]\nup --> down\nend\n\
                   src --> other\nhub --> fed\nhub --> also\nhub --> down\nhub --> leaf\n";
        let g = crate::parser::parse(src).unwrap();
        let sc = crate::scene::scene(&g);
        let y = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap().y;
        let frame = |id: &str| sc.clusters.iter().find(|c| c.id == id).unwrap();
        let hub = sc.nodes.iter().find(|n| n.id == "hub").unwrap();
        let below_hub = hub.y + hub.h / 2.0;
        assert!(frame("g").y > below_hub, "the box spans the hub's row");
        for id in ["src", "loose"] {
            assert!(
                (y(id) - y("fed")).abs() < 1.0,
                "{id} off the box's first row"
            );
        }
        // `next` follows the source that leads into it, whole.
        assert!(frame("h").y > frame("g").y + 30.0, "next box beside box");
        assert!((y("also") - y("other")).abs() < 1.0);
        // `up` leads to `down`, which the hub feeds: `up` takes the
        // box's first row and pushes `down` a row further (B8b r1:
        // f051's `n8 -> n9`), so the frame clears the hub's row too.
        assert!(frame("k").y > below_hub, "the kept box spans the hub's row");
        assert!((y("up") - y("fed")).abs() < 1.0);
        assert!(y("down") > y("up") + 30.0);
    }

    /// A box member that is a source and links out to a node outside
    /// every box (x7's `nightly job --> mail relay`, B8b r1) takes the
    /// box's first row like the rest: the node it links to hangs below
    /// it alone, so it moves a row down along with it. The frame no
    /// longer spans the gateway's row, and none of the gateway's edges
    /// strikes it, in `scene()` or `route()`.
    #[test]
    fn a_box_source_takes_its_sink_down_with_it() {
        let src = "flowchart TB\n  api[API gateway]\n  subgraph svc[Services]\n    s1[orders]\n\
                   s2[payments]\n    s3[nightly job]\n  end\n  api --> s1\n  api --> s2\n\
                   api --> health[health check]\n  api --> docs[docs site]\n  s3 --> mail[mail relay]\n";
        let g = crate::parser::parse(src).unwrap();
        let sc = crate::scene::scene(&g);
        let y = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap().y;
        let api = sc.nodes.iter().find(|n| n.id == "api").unwrap();
        let svc = sc.clusters.iter().find(|c| c.id == "svc").unwrap();
        assert!(
            svc.y > api.y + api.h / 2.0,
            "the box spans the gateway's row"
        );
        assert!(
            (y("s3") - y("s1")).abs() < 1.0,
            "nightly job off the box's row"
        );
        assert!(
            y("mail") > y("s3") + 30.0,
            "mail relay level with its source"
        );
        let rep = crate::quality::measure_source(src, false).unwrap();
        assert_eq!((rep.statik.edge_frame_x, rep.routed.edge_frame_x), (0, 0));
    }

    /// Boxes a chain of moved members leads through follow the seated
    /// box down whole (x1, B8b r1): the hub feeds Alpha and Gamma, an
    /// Alpha source links into Beta, a Beta source into Gamma. Seating
    /// Alpha below the hub moves Beta a row below it, and Gamma, whose
    /// linked member only Beta ranks, follows Beta down whole instead of
    /// spanning three rows beside it — each zone one row, one under the
    /// other, as mermaid draws it.
    #[test]
    fn boxes_a_moved_chain_leads_through_follow_it_down_whole() {
        let src = "flowchart TB\n  hub[Gateway]\n\
                   subgraph A[Alpha zone]\n a1[alpha entry]\n a2[alpha source]\n a3[alpha idle]\nend\n\
                   subgraph B[Beta zone]\n b1[beta in]\n b2[beta source]\nend\n\
                   subgraph C[Gamma zone]\n c1[gamma in]\n c2[gamma fed]\n c3[gamma idle]\nend\n\
                   hub --> a1\n hub --> c2\n hub --> leaf1[leaf one]\n hub --> leaf2[leaf two]\n\
                   a2 --> b1\n b2 --> c1\n";
        let g = crate::parser::parse(src).unwrap();
        let sc = crate::scene::scene(&g);
        let y = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap().y;
        let rows = [y("a1"), y("b1"), y("c1")];
        assert!(y("hub") + 30.0 < rows[0] && rows[0] + 30.0 < rows[1] && rows[1] + 30.0 < rows[2]);
        for (ids, row) in [
            (&["a1", "a2", "a3"][..], rows[0]),
            (&["b1", "b2"][..], rows[1]),
            (&["c1", "c2", "c3"][..], rows[2]),
        ] {
            for id in ids {
                assert!((y(id) - row).abs() < 1.0, "{id} off its zone's row");
            }
        }
        let rep = crate::quality::measure_source(src, false).unwrap();
        assert_eq!((rep.statik.edge_frame_x, rep.routed.edge_frame_x), (0, 0));
    }

    /// Narrowing's stackings and wraps rank-link every member of the
    /// lower box below the upper one; they no longer join the boxes'
    /// edge-less members to the hub, so a stacked tier keeps them on its
    /// first row instead of level with the hub (B8b P1, case b): no
    /// frame spans the load balancer's row, TB or LR, and in LR none of
    /// its edges strikes a frame on the way to its tier.
    #[test]
    fn a_stacked_box_keeps_its_loose_members_below_the_hub() {
        let tiers = "  lb[Load balancer]\n\
                     subgraph web[Web tier]\n w1[web 1]\n w2[web 2]\n w3[web 3]\nend\n\
                     subgraph app[App tier]\n ap1[app 1]\n ap2[app 2]\n ap3[app 3] --> ap4[app worker]\nend\n\
                     subgraph data[Data tier]\n d1[(primary)]\n d2[(replica)]\n d3[(backup)]\nend\n\
                     subgraph obs[Observability]\n m1[metrics]\n m2[logs]\nend\n\
                     lb --> w1\n lb --> ap1\n lb --> d1\n lb -.-> m1\n";
        let tb = format!("flowchart TB\n{tiers}");
        let g = crate::parser::parse(&tb).unwrap();
        let sc = crate::scene::scene(&g);
        let lb = sc.nodes.iter().find(|n| n.id == "lb").unwrap();
        for c in &sc.clusters {
            assert!(c.y > lb.y + lb.h / 2.0, "{} spans the hub's row", c.id);
        }
        let lr = format!(
            "flowchart LR\n{tiers}\
             subgraph q[Queues]\n q1[orders]\n q2[events]\nend\n\
             subgraph c[Caches]\n c1[redis]\n c2[memcache]\nend\n\
             lb --> q1\n lb --> c1\n"
        );
        let g = crate::parser::parse(&lr).unwrap();
        let sc = crate::scene::scene(&g);
        let lb = sc.nodes.iter().find(|n| n.id == "lb").unwrap();
        for c in &sc.clusters {
            assert!(c.x > lb.x + lb.w / 2.0, "{} spans the hub's column", c.id);
        }
        let rep = crate::quality::measure_source(&lr, false).unwrap();
        assert_eq!((rep.statik.edge_frame_x, rep.routed.edge_frame_x), (0, 0));
    }

    /// A hub feeding a row of shallow frames and leaves on one layer (a
    /// generated 4.9 : 1 ribbon) folds it into two rows (report M11):
    /// after one sibling stacking, a wrap ranks the later run of frames
    /// one row down — 2 093 -> 1 106 px of judged breadth for 10 % more
    /// edge length along the flow. (Where the later run's edges would
    /// thread past deep frames, NARROW_FLOW_SLACK refuses it.)
    #[test]
    fn a_hub_fed_row_of_shallow_frames_wraps_into_two_rows() {
        let src = "flowchart TB\n\
                     n0[\"aliqua eiusmod adipiscing elit<br/>sed elit\"]\n\
                     subgraph g0[\"eiusmod ut labore elit magna\"]\n\
                       direction LR\n\
                       n1([\"ut aliqua\"])\n\
                     end\n\
                     subgraph g1[\"eiusmod elit\"]\n\
                       direction TB\n\
                       n2(\"amet lorem elit aliqua<br/>sed\")\n\
                     end\n\
                     subgraph g2[\"sit lorem amet labore magna\"]\n\
                       direction LR\n\
                       n3([\"dolor consectetur magna consectetur<br/>amet incididunt dolor\"])\n\
                     end\n\
                     subgraph g3[\"do\"]\n\
                       n4[(\"dolor\")]\n\
                     end\n\
                     subgraph g4[\"nostrud elit do laboris laboris tempor elit\"]\n\
                       direction TB\n\
                       n5[\"magna incididunt\"]\n\
                       n6([\"ipsum<br/>elit<br/>elit elit magna\"])\n\
                       n7(\"sed<br/>amet amet nostrud\")\n\
                       n8[(\"ut elit adipiscing<br/>dolor laboris nostrud eiusmod\")]\n\
                       n9([\"incididunt sit incididunt<br/>ut veniam tempor nostrud\"])\n\
                       n10[(\"lorem aliqua<br/>tempor\")]\n\
                     end\n\
                     n11([\"ut incididunt lorem aliqua\"])\n\
                     n12([\"dolor magna adipiscing consectetur\"])\n\
                     n13([\"aliqua\"])\n\
                     n0 -.-> n1\n\
                     n0 --> g1\n\
                     n0 --> g2\n\
                     n0 -.-> g3\n\
                     n0 --> g4\n\
                     n0 --> n11\n\
                     n0 --> n12\n\
                     n0 --> n13\n";
        let g = crate::parser::parse(src).unwrap();
        let sc = crate::scene::scene(&g);
        let frame = |id: &str| sc.clusters.iter().find(|c| c.id == id).unwrap();
        let first_row = frame("g1").y + frame("g1").h;
        for id in ["g2", "g4"] {
            assert!(frame(id).y > first_row, "{id} not wrapped below g1");
        }
        assert!(sc.width < 2.0 * sc.height, "{} x {}", sc.width, sc.height);
    }

    /// The narrowing guards (report M11) read edge length both ways:
    /// node centre to centre on both axes, or along the flow only.
    #[test]
    fn edge_length_reads_both_axes_or_the_flow() {
        let mut g = Graph::default();
        let a = g.ensure_node("A", None, None);
        let b = g.ensure_node("B", None, None);
        g.add_edge(a, b, None, EdgeKind::Arrow);
        g.add_edge(a, a, None, EdgeKind::Arrow);
        let placed = |b: f64, l: f64| Placed {
            b,
            l,
            bsize: 10.0,
            lsize: 10.0,
            layer: 0,
        };
        let lo = LayoutResult {
            nodes: vec![placed(0.0, 0.0), placed(30.0, 40.0)],
            total_b: 0.0,
            total_l: 0.0,
            edge_paths: vec![Vec::new(), Vec::new()],
        };
        assert_eq!(
            edge_length(&g, &lo, false),
            70.0,
            "the self-loop counts nothing"
        );
        assert_eq!(edge_length(&g, &lo, true), 40.0);
    }

    /// Cycle breakers (B8b P2): a small graph takes them from whichever
    /// walk reverses fewer edges — here the walk in target order reverses
    /// `n2 -> n1` alone where source order reverses `n1 -> n2` and
    /// `n1 -> n3` — and a graph past WALK_MAX_ELEMENTS keeps the walk in
    /// source order, as random graphs drew better with it (syn200's
    /// shallower layering was packed into a ribbon whose `route()` threaded
    /// 71 edges through nodes, 24 before).
    #[test]
    fn large_graphs_keep_source_order_cycle_breakers() {
        let core =
            "flowchart TB\nn0\nn1\nn2\nn3\nn1-->n2\nn0-->n3\nn0-->n1\nn1-->n3\nn2-->n1\nn3-->n2\n";
        let g = crate::parser::parse(core).unwrap();
        let back = |w: Walk| -> Vec<usize> {
            let rk = rank(&g, &[], 0, None, w, None);
            (0..g.edges.len()).filter(|&e| rk.back[e]).collect()
        };
        assert_eq!(back(Walk::Source), vec![0, 3]);
        assert_eq!(back(Walk::Fewest), vec![4]);
        let layers = |src: &str| -> Vec<usize> {
            let g = crate::parser::parse(src).unwrap();
            let sizes = vec![(60.0, 40.0); g.nodes.len()];
            let (lo, ..) = layout_core(&g, &sizes, None, &[], None);
            lo.nodes[..4].iter().map(|p| p.layer).collect()
        };
        let small = layers(core);
        assert_eq!(small, rank(&g, &[], 0, None, Walk::Fewest, None).layer);
        let mut big = core.to_string();
        let pairs = WALK_MAX_ELEMENTS / 3 + 1;
        for i in 0..pairs {
            big += &format!("p{i} --> q{i}\n");
        }
        assert!(4 + 6 + 3 * pairs > WALK_MAX_ELEMENTS);
        assert_eq!(
            layers(&big),
            rank(&g, &[], 0, None, Walk::Source, None).layer
        );
        assert_ne!(small, layers(&big), "the walks lay the cycle out alike");
    }

    /// A wrap candidate (report M11) splits the siblings sharing their
    /// parent's most shared layer into two runs of about equal breadth,
    /// in drawn order, and is skipped when a path leads back from the
    /// later run into the earlier.
    #[test]
    fn wrap_candidates_split_a_shared_layer_into_two_even_runs() {
        let src = "flowchart LR\nhub\nsubgraph a\na1\nend\nsubgraph b\nb1\nend\nsubgraph c\nc1\nend\nsubgraph d\nd1\nend\nhub --> a1\nhub --> b1\nhub --> c1\nhub --> d1\n";
        let g = crate::parser::parse(src).unwrap();
        let nc: Vec<Vec<usize>> = (0..g.nodes.len())
            .map(|v| {
                g.subgraphs
                    .iter()
                    .position(|s| s.nodes.contains(&v))
                    .map_or(Vec::new(), |s| vec![s])
            })
            .collect();
        let sizes: Vec<(f64, f64)> = vec![(60.0, 40.0); g.nodes.len()];
        let rk = rank(&g, &[], 0, None, Walk::Fewest, None);
        let (lo, ..) = layout_core(&g, &sizes, Some(&nc), &[], None);
        let wraps = wrap_candidates(&g, &nc, &[], &rk, &rk.layer, &lo);
        assert_eq!(wraps.len(), 1, "{wraps:?}");
        let (first, then) = &wraps[0];
        assert_eq!((first.len(), then.len()), (2, 2), "{wraps:?}");
        // Drawn order: the first run lies before the second.
        let b = |v: &usize| lo.nodes[*v].b;
        assert!(
            first.iter().map(b).fold(f64::MIN, f64::max)
                < then.iter().map(b).fold(f64::MAX, f64::min)
        );
        // A link from the later run back into the earlier one forbids it.
        let mut back = src.to_string();
        let (x, y) = (&g.nodes[then[0]].id, &g.nodes[first[0]].id);
        back += &format!("{x} --> {y}\n");
        let g2 = crate::parser::parse(&back).unwrap();
        let rk2 = rank(&g2, &[], 0, None, Walk::Fewest, None);
        let wraps2 = wrap_candidates(&g2, &nc, &[], &rk2, &rk2.layer, &lo);
        assert!(
            wraps2
                .iter()
                .all(|(f, t)| !(t.contains(&then[0]) && f.contains(&first[0]))),
            "{wraps2:?}"
        );
    }

    /// `Edge::end_room` (ER crow's feet) widens a gap between adjacent
    /// layers by just enough for the room, the label and LABEL_CLEAR
    /// either side — and an edge without room or label leaves the gap
    /// exactly as it was. (An ER scene, whose painter strokes its own
    /// edges: a flowchart's labelled gap also keeps head room, see
    /// `a_labelled_gap_keeps_its_label_off_the_head`.)
    #[test]
    fn end_room_widens_an_adjacent_gap_for_glyphs_and_label() {
        let gap = |room: (f64, f64), label: Option<&str>, dir: Direction| {
            let mut g = Graph::default();
            g.edge_paint_width = Some(1.5);
            g.direction = dir;
            let a = g.ensure_node("A", None, None);
            let b = g.ensure_node("B", None, None);
            g.add_edge(a, b, label.map(str::to_string), EdgeKind::Open);
            g.edges[0].end_room = room;
            let sizes = [(80.0, 40.0), (80.0, 40.0)];
            let lo = layout_sized(&g, &sizes);
            let (pa, pb) = (&lo.nodes[0], &lo.nodes[1]);
            (pb.l - pb.lsize / 2.0) - (pa.l + pa.lsize / 2.0)
        };
        for dir in [Direction::TD, Direction::LR] {
            assert_eq!(gap((0.0, 0.0), None, dir), GAP_L, "{dir:?}");
            // Room that fits GAP_L changes nothing.
            assert_eq!(gap((10.0, 10.0), None, dir), GAP_L, "{dir:?}");
            // Glyphs too long to face each other in GAP_L.
            let g = gap((40.0, 30.0), None, dir);
            assert!(
                (g - (70.0 + 2.0 * LABEL_CLEAR)).abs() < 1e-9,
                "{dir:?}: {g}"
            );
        }
        // A label between the glyphs: along LR it takes its width.
        let w = text_width("a long relationship role") + LABEL_PAD_W;
        let g = gap(
            (30.0, 20.0),
            Some("a long relationship role"),
            Direction::LR,
        );
        assert!((g - (50.0 + w + 2.0 * LABEL_CLEAR)).abs() < 1e-9, "{g}");
        // Down the page, its height.
        let g = gap((30.0, 25.0), Some("role"), Direction::TD);
        assert!(
            (g - (55.0 + LABEL_BOX_H + 2.0 * LABEL_CLEAR)).abs() < 1e-9,
            "{g}"
        );
    }

    /// Report M3 (§2.6): in a flowchart a gap between adjacent layers
    /// carrying a label grows to hold the label, an arrowhead and a stub
    /// either side of it — 72 px down the page for a one-line label
    /// (the 60 px gap used to stay, the chip swallowing a third of it) —
    /// and along LR the label's width, the head and a clearance either
    /// side; a gap with an unlabelled edge stays [`GAP_L`].
    #[test]
    fn a_labelled_gap_keeps_its_label_off_the_head() {
        let gap = |label: Option<&str>, dir: Direction| {
            let mut g = Graph::default();
            g.direction = dir;
            let a = g.ensure_node("A", None, None);
            let b = g.ensure_node("B", None, None);
            g.add_edge(a, b, label.map(str::to_string), EdgeKind::Arrow);
            let lo = layout_sized(&g, &[(80.0, 40.0), (80.0, 40.0)]);
            let (pa, pb) = (&lo.nodes[0], &lo.nodes[1]);
            (pb.l - pb.lsize / 2.0) - (pa.l + pa.lsize / 2.0)
        };
        for dir in [Direction::TD, Direction::BT, Direction::LR, Direction::RL] {
            assert_eq!(gap(None, dir), GAP_L, "{dir:?}");
        }
        for dir in [Direction::TD, Direction::BT] {
            let g = gap(Some("yes"), dir);
            assert!((g - (LABEL_BOX_H + 2.0 * LABEL_RUN)).abs() < 1e-9, "{g}");
            assert!(g >= 72.0, "{dir:?}: {g}");
        }
        let w = text_width("a long branch label") + LABEL_PAD_W;
        for dir in [Direction::LR, Direction::RL] {
            let g = gap(Some("a long branch label"), dir);
            let want = w + 2.0 * LABEL_CLEAR + ARROW_MARKER_PX;
            assert!((g - want).abs() < 1e-9, "{dir:?}: {g} for {want}");
        }
    }

    /// B7 review r4 #4, #7: along LR a double-headed edge's labelled gap
    /// keeps room for both heads (sized for one, `<-->`'s label had no
    /// spot clear of both on its line and stepped above it), and a
    /// `linkStyle` width that repeats the plain one leaves a plain gap
    /// (it grew every labelled LR gap by about 30 px).
    #[test]
    fn an_lr_gap_counts_both_heads_and_ignores_a_plain_width() {
        let gap = |src: &str| {
            let g = crate::parser::parse(src).unwrap();
            let sc = crate::scene::scene(&g);
            let (a, b) = (&sc.nodes[0], &sc.nodes[1]);
            let e = &sc.edges[0];
            let (_, c, _) = e.label.as_ref().unwrap();
            ((b.x - b.w / 2.0) - (a.x + a.w / 2.0), c.1 - e.bezier[0].1)
        };
        let w = text_width("both ways here") + LABEL_PAD_W;
        let (two, off) = gap("flowchart LR\n  A[Alpha] <-->|both ways here| B[Beta]\n");
        let want = w + 2.0 * LABEL_CLEAR + 2.0 * ARROW_MARKER_PX;
        assert!((two - want).abs() < 1e-9, "{two} for {want}");
        assert!(off.abs() < 1e-9, "the label stays on its line: {off}");
        let (one, _) = gap("flowchart LR\n  A[Alpha] -->|both ways here| B[Beta]\n");
        assert!((one - (want - ARROW_MARKER_PX)).abs() < 1e-9, "{one}");
        let (plain_w, _) = gap(
            "flowchart LR\n  A[Alpha] -->|both ways here| B[Beta]\n  linkStyle default stroke-width:2px\n",
        );
        assert_eq!(plain_w, one);
        let (wide, _) = gap(
            "flowchart LR\n  A[Alpha] -->|both ways here| B[Beta]\n  linkStyle default stroke-width:3px\n",
        );
        assert!(wide > one + 20.0, "{wide} vs {one}");
    }

    /// B7 review r4 #2: a labelled edge between two composites whose
    /// stand-in runs on to members deeper inside (here out of a nested
    /// box) is still drawn only between the two borders, and that gap
    /// holds its label clear of both heads as a short edge's does — it
    /// used to stay FRAME_GAP-ish (21 px), the chips covering the whole
    /// edge and its arrowhead. Both labels lie in the gap, off the heads.
    #[test]
    fn a_labelled_gap_between_composites_holds_its_labels() {
        let doc = crate::parser::parse_document(
            "stateDiagram-v2
  state K0 {
    [*] --> K0_0
    state K0N {
      [*] --> a
      a --> b
    }
    K0_0 --> K0N
  }
  state K1 {
    [*] --> c
  }
  K0 --> K1 : g2
  K1 --> K0 : back
",
        )
        .unwrap();
        let crate::model::Document::State(g) = doc else {
            panic!("not a state diagram");
        };
        let sc = crate::scene::scene(&g);
        let frame = |id: &str| sc.clusters.iter().find(|c| c.id == id).unwrap();
        let (k0, k1) = (frame("K0"), frame("K1"));
        let (top, bottom) = (k0.y + k0.h, k1.y);
        assert!(
            bottom - top >= LABEL_BOX_H + 2.0 * LABEL_RUN - 1e-9,
            "gap {top} .. {bottom}"
        );
        for text in ["g2", "back"] {
            let e = sc
                .edges
                .iter()
                .find(|e| e.label.as_ref().is_some_and(|l| l.0 == text))
                .unwrap();
            let (_, c, _) = e.label.as_ref().unwrap();
            // Inside the gap, a head's length clear of either border.
            assert!(
                c.1 - LABEL_BOX_H / 2.0 >= top + ARROW_MARKER_PX
                    && c.1 + LABEL_BOX_H / 2.0 <= bottom - ARROW_MARKER_PX,
                "{text} at {c:?} in {top} .. {bottom}"
            );
        }
    }

    /// Report M7: a plain node label with no break of its own wraps at
    /// the graph's width, balanced and never mid-word; the model keeps
    /// the text as written (so `to_mermaid` writes no breaks), the scene
    /// shows and sizes the wrapped lines, and a label that breaks its
    /// own lines is left alone (no towers out of `<br/>` labels).
    #[test]
    fn plain_labels_wrap_balanced_and_only_in_the_layout() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/long-labels.mmd"
        ))
        .unwrap();
        let g = crate::parser::parse(&src).unwrap();
        assert_eq!(g.wrapping_width(), Some(WRAP_WIDTH));
        let title = "ADMINISTRATIVE REQUIREMENTS DOCUMENT VALIDATION";
        assert_eq!(g.nodes[0].label, title, "the model keeps the source text");
        let sc = crate::scene::scene(&g);
        let shown = &sc.nodes[0].label;
        assert_eq!(shown, "ADMINISTRATIVE\nREQUIREMENTS\nDOCUMENT VALIDATION");
        assert_eq!(shown.replace('\n', " "), title, "no word split or lost");
        // Sized for what it shows.
        let (w, h) = intrinsic_size(&g.nodes[0]);
        assert_eq!(
            (w, h),
            (
                (text_width(shown) + 2.0 * PAD_X).max(MIN_W),
                BASE_H + 2.0 * LINE_H
            )
        );
        assert!(shown.split('\n').all(|l| text_width(l) <= WRAP_WIDTH));
        // Balanced: "Return documents | to the applicant", no lone word.
        let d = sc.nodes.iter().find(|n| n.id == "D").unwrap();
        assert_eq!(d.label, "Return documents\nto the applicant");
        // Short labels and explicit breaks stay as written.
        let c = sc.nodes.iter().find(|n| n.id == "C").unwrap();
        assert_eq!(c.label, "Issue the certificate");
        let g2 = crate::parser::parse(
            "flowchart TD\nA[a fairly long first line that goes on<br/>and a second one]\n\
             B[<b>a bold run long enough to go over two lines</b> and a <i>plain</i> tail]\n\
             C[Pneumonoultramicroscopicsilicovolcanoconiosis_is_one_very_long_word x]\n\
             D[x Pneumonoultramicroscopicsilicovolcanoconiosis_is_one_very_long_word]\n\
             E[Pneumonoultramicroscopicsilicovolcanoconiosis_is_one_very_long_word and a few more words]",
        )
        .unwrap();
        let sc2 = crate::scene::scene(&g2);
        assert_eq!(sc2.nodes[0].label, g2.nodes[0].label);
        // Markup wraps too, its tags closed and re-opened on each line.
        assert_eq!(
            sc2.nodes[1].label,
            "<b>a bold run long enough to go</b>\n<b>over two lines</b> and a <i>plain</i> tail"
        );
        assert!(sc2.nodes[1]
            .label
            .lines()
            .all(|l| text_width(l) <= WRAP_WIDTH));
        // An over-long word is never split; a lone word beside it joins
        // its line instead of being stranded under it (or over it), and
        // the other lines may fill up to its width.
        for v in [2, 3] {
            assert_eq!(sc2.nodes[v].label, g2.nodes[v].label, "one line");
        }
        let e: Vec<&str> = sc2.nodes[4].label.lines().collect();
        assert_eq!(
            e,
            [
                "Pneumonoultramicroscopicsilicovolcanoconiosis_is_one_very_long_word",
                "and a few more words"
            ]
        );
        // The source round-trips without the layout's breaks.
        let text = crate::to_mermaid(&g);
        assert!(text.contains(title) && !text.contains("<br"), "{text}");
        // An embedder's real font wraps by its own measure.
        let wide = |l: &str| 2.0 * text_width(l);
        let (_, h2) = intrinsic_size_with(&g.nodes[2], wide);
        assert_eq!(
            h2,
            BASE_H + LINE_H,
            "twice as wide a font wraps the 121 px label"
        );
        assert_eq!(intrinsic_size_wrapped(&g.nodes[2], None, wide).1, BASE_H);
    }

    /// Report M7: the width comes from `flowchart.wrappingWidth` in an
    /// init directive or the frontmatter (a later directive winning),
    /// for flowcharts and state diagrams; class and ER boxes never wrap.
    #[test]
    fn wrapping_width_is_read_from_the_source() {
        let long = "A[Return documents to the applicant]";
        let lines = |src: &str| {
            let g = crate::parser::parse(src).unwrap();
            (
                g.wrapping_width(),
                crate::scene::scene(&g).nodes[0].label.lines().count(),
            )
        };
        assert_eq!(
            lines(&format!("flowchart TD\n{long}")),
            (Some(WRAP_WIDTH), 2)
        );
        assert_eq!(
            lines(&format!(
                "%%{{init: {{\"flowchart\": {{\"wrappingWidth\": 400}}}}}}%%\nflowchart TD\n{long}"
            )),
            (Some(400.0), 1)
        );
        assert_eq!(
            lines(&format!(
                "---\nconfig:\n  flowchart:\n    wrappingWidth: 80\n---\nflowchart TD\n{long}"
            )),
            (Some(80.0), 4)
        );
        // Mermaid lays directives over the frontmatter; nonsense is
        // ignored.
        assert_eq!(
            lines(&format!(
                "---\nconfig:\n  flowchart:\n    wrappingWidth: 80\n---\n\
                 %%{{init: {{\"flowchart\": {{\"wrappingWidth\": 400}}}}}}%%\nflowchart TD\n{long}"
            ))
            .0,
            Some(400.0)
        );
        assert_eq!(
            lines(&format!(
                "%%{{init: {{\"flowchart\": {{\"wrappingWidth\": -3}}}}}}%%\nflowchart TD\n{long}"
            ))
            .0,
            Some(WRAP_WIDTH)
        );
        // A configured width survives `to_mermaid`.
        let g = crate::parser::parse(&format!(
            "%%{{init: {{\"flowchart\": {{\"wrappingWidth\": 400}}}}}}%%\nflowchart TD\n{long}"
        ))
        .unwrap();
        let back = crate::parser::parse(&crate::to_mermaid(&g)).unwrap();
        assert_eq!(back.wrapping_width(), Some(400.0));
        // State diagrams wrap their states alike.
        let doc = crate::parser::parse_document(
            "%%{init: {\"flowchart\": {\"wrappingWidth\": 80}}}%%\nstateDiagram-v2\n\
             state \"Return documents to the applicant\" as S\n[*] --> S",
        )
        .unwrap();
        let crate::model::Document::State(sg) = doc else {
            panic!("a state diagram")
        };
        assert_eq!(sg.wrapping_width(), Some(80.0));
        // Off for an embedder that asks (class and ER graphs do).
        let mut g = crate::parser::parse(&format!("flowchart TD\n{long}")).unwrap();
        g.set_wrapping_width(None);
        assert_eq!(crate::scene::scene(&g).nodes[0].label.lines().count(), 1);
        g.set_wrapping_width(Some(f64::NAN));
        assert_eq!(g.wrapping_width(), None);
    }

    /// Report §2.12 / M7 / §6: a long decision wraps — at the graph's
    /// width over 1.6, so it is no wider than a box wrapping there — into
    /// a block no flatter than 1.6 : 1, while a short one keeps its
    /// compact one-line rhombus (fanout's "All succeeded?", shapes'
    /// "Diamond": not grown taller to the cap), and a label-less branch
    /// point keeps its small square. A longer question never draws a
    /// bigger rhombus than a shorter one wraps into.
    #[test]
    fn long_diamonds_wrap_within_the_cap_and_short_ones_stay_compact() {
        let src = "flowchart TD\nA{ok?} --> B{Is it valid?}\n\
                   B --> C{Are all documents complete and compliant with the rules?}\n\
                   C --> D{Supercalifragilisticexpialidocious} --> E{ }\n\
                   F{All succeeded?} --> G{Diamond} --> H{Payment approved?}\n\
                   H --> I{Is the user logged in?} --> J{Retry the charge once more?}\n\
                   J --> K{Is the payment valid for this order?}";
        let g = crate::parser::parse(src).unwrap();
        let sc = crate::scene::scene(&g);
        let ww = g.wrapping_width().unwrap() / DIAMOND_ASPECT_MAX;
        for (n, m) in sc.nodes.iter().zip(&g.nodes) {
            let tw = text_width(&m.label);
            if n.shape != Shape::Diamond || tw == 0.0 {
                continue;
            }
            if tw <= ww {
                // Short: one line, the rhombus fitted to it, uncapped.
                assert_eq!(n.label, m.label, "{}", n.id);
                assert!(
                    (n.w - ((tw + 24.0) * 1.6).max(80.0)).abs() < 1e-9 && n.h == BASE_H * 1.7,
                    "{}: {} x {}",
                    n.id,
                    n.w,
                    n.h
                );
            } else {
                assert!(
                    n.w <= n.h * DIAMOND_ASPECT_MAX + 1e-9,
                    "{}: {} x {}",
                    n.id,
                    n.w,
                    n.h
                );
            }
        }
        let node = |id: &str| sc.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!((node("A").w, node("A").h), (80.0, BASE_H * 1.7));
        for (id, w) in [("F", 192.0), ("G", 128.0)] {
            assert!((node(id).w - w).abs() < 1e-9 && node(id).h == BASE_H * 1.7);
        }
        for id in ["C", "H", "I", "J", "K"] {
            assert!(node(id).label.lines().count() > 1, "{id} wraps");
        }
        // A wrapped question fills its rhombus: its height is its
        // text's, not the cap's.
        for id in ["C", "H", "I", "J", "K"] {
            let n = node(id);
            let lines = n.label.lines().count() as f64;
            assert_eq!(n.h, (BASE_H + (lines - 1.0) * LINE_H) * 1.7, "{id}");
        }
        assert_eq!(node("C").label.lines().count(), 4);
        // Longer text, no smaller rhombus.
        let area = |id: &str| node(id).w * node(id).h;
        assert!(area("J") <= area("K"), "{} {}", area("J"), area("K"));
        // One word too long to wrap: capped, not a sliver.
        let d = node("D");
        assert!((d.w / d.h - DIAMOND_ASPECT_MAX).abs() < 1e-9);
        assert_eq!((node("E").w, node("E").h), (CHOICE_SIZE, CHOICE_SIZE));
    }

    /// Circles grow both ways with their text: a long label wraps at the
    /// same narrower width as a diamond's (mermaid wraps both of these),
    /// a short one keeps its line.
    #[test]
    fn long_circle_labels_wrap_narrower() {
        let g = crate::parser::parse(
            "flowchart TD\nA((A circle with a long label inside)) --> \
             B((Is this a long circle label here)) --> C(((A double circle with a long label)))\n\
             C --> D((short))",
        )
        .unwrap();
        let sc = crate::scene::scene(&g);
        for v in 0..3 {
            let n = &sc.nodes[v];
            assert_eq!(n.label.lines().count(), 2, "{}: {:?}", n.id, n.label);
            assert!(n.w < 140.0 && n.w == n.h, "{}: {} x {}", n.id, n.w, n.h);
        }
        assert_eq!(sc.nodes[3].label, "short");
    }

    /// CJK text breaks between characters (mermaid breaks a long
    /// Japanese label anywhere), never before closing punctuation or a
    /// small kana, and never after an opening bracket.
    #[test]
    fn cjk_labels_break_between_characters() {
        let g = crate::parser::parse(
            "flowchart TD\nA[日本語のラベルはとても長いのでここで折り返されるべきでしょうか テスト]\n\
             B[「東京」から「大阪」まで、新幹線で移動します。とても速いです。ありがとう。]",
        )
        .unwrap();
        let sc = crate::scene::scene(&g);
        for (n, m) in sc.nodes.iter().zip(&g.nodes) {
            let lines: Vec<&str> = n.label.lines().collect();
            assert!(lines.len() > 1, "{}: {:?}", n.id, n.label);
            for l in &lines {
                assert!(text_width(l) <= WRAP_WIDTH, "{l}");
            }
            // Nothing lost or added but the breaks (the one space kept).
            assert_eq!(n.label.replace(['\n', ' '], ""), m.label.replace(' ', ""));
            for (a, b) in lines.iter().zip(&lines[1..]) {
                let first = b.chars().next().unwrap();
                assert!(!"、。」）っゃゅょー".contains(first), "{b}");
                assert!(!"「（".contains(a.chars().last().unwrap()), "{a}");
            }
        }
    }

    #[test]
    fn wrap_text_is_greedy_keeps_breaks_and_never_splits_words() {
        // Fits: untouched (including inner runs of spaces).
        assert_eq!(wrap_text("short  text", 200.0, 1.0), "short  text");
        let s = "one two three four five six seven eight nine ten";
        let w = wrap_text(s, 80.0, 1.0);
        assert!(w.lines().count() > 1);
        assert_eq!(w.replace('\n', " "), s, "words kept in order");
        for line in w.lines() {
            assert!(text_width(line) <= 80.0, "{line:?} too wide");
        }
        // Greedy: no line could take the next line's first word.
        let lines: Vec<&str> = w.lines().collect();
        for pair in lines.windows(2) {
            let next = pair[1].split(' ').next().unwrap();
            assert!(text_width(&format!("{} {next}", pair[0])) > 80.0);
        }
        // An over-long word keeps a line to itself, unsplit.
        assert_eq!(
            wrap_text("a Pneumonoultramicroscopic b", 40.0, 1.0),
            "a\nPneumonoultramicroscopic\nb"
        );
        // Existing breaks stay; the scale measures at another size.
        assert_eq!(wrap_text("x\ny", 1.0, 1.0), "x\ny");
        let at_13 = wrap_text(s, 80.0, 13.0 / TEXT_CALIBRATION);
        assert!(at_13.lines().count() <= w.lines().count());
    }

    #[test]
    fn a_left_note_leads_its_state_inside_a_composite_entered_from_outside() {
        // `First --> Second` keeps Second's members below First (a
        // rank-only group into every member, the card included); that
        // floor must not park `left of b` on Second's top layer, two
        // layers ahead of b: the card leads b by one, beside a.
        let src = "stateDiagram-v2\n[*] --> First\nstate First {\n[*] --> fir\n}\n\
                   First --> Second\nstate Second {\n[*] --> a\na --> b\n}\n\
                   note left of b : left of b";
        let crate::model::Document::State(g) = crate::parser::parse_document(src).unwrap() else {
            panic!("a state diagram");
        };
        let sc = crate::scene::scene(&g);
        let at = |id: &str| &sc.nodes[g.node_index(id).unwrap()];
        let card = sc.nodes.iter().find(|n| n.label == "left of b").unwrap();
        assert!(
            (card.y - at("a").y).abs() < 1.0,
            "beside a: {} vs {}",
            card.y,
            at("a").y
        );
        assert!(card.y < at("b").y);
        // Still inside Second, below First.
        let frame = |id: &str| sc.clusters.iter().find(|c| c.id == id).unwrap();
        let (first, second) = (frame("First"), frame("Second"));
        assert!(card.y - card.h / 2.0 > second.y && card.y - card.h / 2.0 > first.y + first.h);
    }

    #[test]
    fn compact_choice_fans_get_room_for_their_label_rows() {
        // The S between adjacent layers: from `from` to `to`, half-way
        // across at half depth.
        let line = |to: f64, width: Option<f64>| FanLine {
            point: 0,
            from: 0.0,
            to,
            width,
        };
        let l = line(100.0, None);
        assert!(fan_line_at(&l, 0.0).abs() < 1e-6);
        assert!((fan_line_at(&l, 0.5) - 50.0).abs() < 1e-6);
        assert!((fan_line_at(&l, 1.0) - 100.0).abs() < 1e-6);
        // Unlabelled lines ask nothing.
        assert_eq!(fan_rows_gap(&[l], 50.0), 50.0);
        // Labels whose far ends lie wide apart share row 0, just past
        // FAN_FAR; close ones need a second row, a deeper gap.
        let one = fan_rows_gap(&[line(-150.0, Some(120.0)), line(150.0, Some(120.0))], 0.0);
        assert_eq!(one, fan_gap(LABEL_BOX_H / 2.0));
        let two = fan_rows_gap(&[line(-60.0, Some(120.0)), line(60.0, Some(120.0))], 0.0);
        assert!(two > one + LABEL_BOX_H, "{two} vs {one}");
        // A gap that is roomy already stays as it is.
        assert_eq!(fan_rows_gap(&[line(-150.0, Some(120.0))], 400.0), 400.0);
        // Laid out: the 4-way choice of wide labels gets its rows; the
        // same fan out of a labelled diamond (no fan) keeps the old gap.
        let src = |c: &str| {
            format!(
                "flowchart TD\nc{c} -->|priority customer| E[Express]\n\
                 c -->|regular customer| S[Standard]\nc -->|payment pending| H[Hold]\n\
                 c -->|fraud suspected| K[Cancel]"
            )
        };
        let gap = |c: &str| {
            let g = crate::parser::parse(&src(c)).unwrap();
            let sc = crate::scene::scene(&g);
            let (c, e) = (&sc.nodes[0], &sc.nodes[g.node_index("E").unwrap()]);
            (e.y - e.h / 2.0) - (c.y + c.h / 2.0)
        };
        assert!(
            gap("{ }") > gap("{ok?}"),
            "{} vs {}",
            gap("{ }"),
            gap("{ok?}")
        );
    }

    #[test]
    fn choice_twins_and_too_wide_fans_still_get_their_room() {
        let line = |from: f64, to: f64, width: f64| FanLine {
            point: 0,
            from,
            to,
            width: Some(width),
        };
        // Two lines to one state, PARALLEL_GAP apart: neither label has
        // a row clear of the other's line, so they go beside them, and
        // the third label takes a second row rather than crowd 'two'.
        let twins = [
            line(0.0, -8.0, 37.4),
            line(0.0, 8.0, 35.8),
            line(7.0, 89.0, 46.0),
        ];
        assert!(fan_rows_fit(&twins, 0.0, false).is_none());
        let gap = fan_rows_gap(&twins, 0.0);
        assert!(gap > fan_gap(LABEL_BOX_H / 2.0), "{gap}");
        // Beside is a last resort: lines a label fits on keep their gap.
        let apart = [line(0.0, -150.0, 120.0), line(0.0, 150.0, 120.0)];
        assert_eq!(fan_rows_gap(&apart, 0.0), fan_gap(LABEL_BOX_H / 2.0));
        // Laid out: labels wider than the room between their states
        // spread those states (labels side by side, mermaid-style); a
        // fan whose labels stack in rows keeps its compact spacing.
        let spacing = |src: &str| {
            let crate::model::Document::State(g) = crate::parser::parse_document(src).unwrap()
            else {
                panic!("a state diagram");
            };
            let sc = crate::scene::scene(&g);
            let x = |id: &str| sc.nodes[g.node_index(id).unwrap()].x;
            (x("C") - x("B"), x("D") - x("C"))
        };
        let wide = spacing(
            "stateDiagram-v2\nstate c <<choice>>\n[*] --> A\nA --> c\n\
             c --> B : this is a very long label for the first branch\n\
             c --> C : and an equally long label for branch two\n\
             c --> D : third one is also quite long indeed",
        );
        let w = text_width("and an equally long label for branch two") + LABEL_PAD_W;
        assert!(wide.0 > w && wide.1 > w, "{wide:?} vs {w}");
        let rows = spacing(
            "stateDiagram-v2\nstate c <<choice>>\n[*] --> A\nA --> c\n\
             c --> B : priority\nc --> C : regular\nc --> D : pending",
        );
        assert!(rows.0 < MIN_W + 2.0 * GAP_B, "{rows:?}");
    }

    #[test]
    fn unlabelled_diamond_is_a_compact_choice() {
        // A state `<<choice>>` has no text: mermaid draws it 28×28, not
        // the 80×64.6 a decision diamond needs round its label.
        let doc = crate::parser::parse_document(
            "stateDiagram-v2\nstate c <<choice>>\n[*] --> c\nc --> A : yes\nc --> B : no",
        )
        .unwrap();
        let crate::model::Document::State(g) = doc else {
            panic!("a state diagram");
        };
        let c = &g.nodes[g.node_index("c").unwrap()];
        assert_eq!(intrinsic_size(c), (CHOICE_SIZE, CHOICE_SIZE));
        // Labelled diamonds keep the text-fitting rule and its minimum;
        // a label too long for one line wraps, never flatter than
        // DIAMOND_ASPECT_MAX (report §2.12).
        let g = crate::parser::parse("flowchart TD\nA{ok?} --> B{a much longer question}").unwrap();
        let (wa, ha) = intrinsic_size(&g.nodes[0]);
        assert_eq!((wa, ha), (80.0, BASE_H * 1.7), "short label: the minimum");
        let (wb, hb) = intrinsic_size(&g.nodes[1]);
        let shown = wrap_label(&g.nodes[1], WRAP_WIDTH, text_width);
        assert_eq!(shown, "a much\nlonger\nquestion");
        let tw = text_width("question");
        assert_eq!(wb, (tw + 24.0) * 1.6, "long label: fitted to its lines");
        assert_eq!(hb, (BASE_H + 2.0 * LINE_H) * 1.7);
        // The compact choice still lays out and anchors on its border.
        let sc = crate::scene::scene(&crate::parser::parse("flowchart TD\nA{ } --> B").unwrap());
        assert_eq!((sc.nodes[0].w, sc.nodes[0].h), (CHOICE_SIZE, CHOICE_SIZE));
        let start = sc.edges[0].bezier[0];
        let bottom = sc.nodes[0].y + CHOICE_SIZE / 2.0;
        assert!((start.1 - bottom).abs() < 0.5, "edge leaves the vertex");
    }

    #[test]
    fn segment_box_test_is_open_and_exact() {
        let r = (0.0, 0.0, 10.0, 10.0);
        assert!(seg_hits_box((-5.0, 5.0), (15.0, 5.0), r));
        assert!(seg_hits_box((5.0, -5.0), (5.0, 2.0), r));
        // Touching a border, or passing beside, is no hit.
        assert!(!seg_hits_box((-5.0, 10.0), (15.0, 10.0), r));
        assert!(!seg_hits_box((-5.0, -1.0), (-1.0, 20.0), r));
    }

    #[test]
    fn swept_measures_match_the_all_pairs_ones() {
        // drawn_crossings and node_hits only test segments and boxes
        // whose layer spans overlap: the counts must equal a test of
        // every pair.
        let src = include_str!("../tests/fixtures/quality/dense-td.mmd");
        let g = crate::parser::parse(src).unwrap();
        let lo = layout(&g);
        let at = |v: usize| (lo.nodes[v].b, lo.nodes[v].l);
        type Poly = (usize, usize, Vec<(f64, f64)>);
        let polys: Vec<Poly> = g
            .edges
            .iter()
            .zip(&lo.edge_paths)
            .filter(|(e, _)| e.from != e.to && !matches!(e.kind, EdgeKind::Invisible))
            .map(|(e, p)| {
                let mut pts = vec![at(e.from)];
                pts.extend(p.iter().copied());
                pts.push(at(e.to));
                (e.from, e.to, pts)
            })
            .collect();
        let side = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| {
            (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
        };
        let mut want = 0;
        for (i, (fa, ta, pa)) in polys.iter().enumerate() {
            for (fb, tb, pb) in &polys[i + 1..] {
                if fa == fb || fa == tb || ta == fb || ta == tb {
                    continue;
                }
                for a in pa.windows(2) {
                    for b in pb.windows(2) {
                        if side(a[0], a[1], b[0]) * side(a[0], a[1], b[1]) < 0.0
                            && side(b[0], b[1], a[0]) * side(b[0], b[1], a[1]) < 0.0
                        {
                            want += 1;
                        }
                    }
                }
            }
        }
        assert!(want > 0, "fixture has no crossing to count");
        assert_eq!(drawn_crossings(&g, &lo.edge_paths, at), want);
        let back = vec![false; g.edges.len()];
        let r = |v: usize| {
            let p = &lo.nodes[v];
            (
                p.b - p.bsize / 2.0,
                p.l - p.lsize / 2.0,
                p.b + p.bsize / 2.0,
                p.l + p.lsize / 2.0,
            )
        };
        let mut want = 0;
        for (f, t, pts) in &polys {
            for v in (0..g.nodes.len()).filter(|v| v != f && v != t) {
                if pts.windows(2).any(|s| seg_hits_box(s[0], s[1], r(v))) {
                    want += 1;
                }
            }
        }
        let hits = node_hits(&g, &back, &lo.edge_paths, |v| {
            let p = &lo.nodes[v];
            (p.b, p.l, p.bsize, p.lsize)
        });
        assert_eq!(hits, want);
    }

    #[test]
    fn the_search_is_skipped_past_its_budget() {
        // A fan whose first leaf clearly outsizes the rest. Inside the
        // budget that leaf is centred (D3 walk, D4 slide); past it the
        // layout is the single crossing-minimised placement, which keeps
        // the declaration order. A long fan used to pay O(k^3) for the
        // walk alone (k/2 swaps, each an O(E^2) crossing count).
        let fan = |k: usize| {
            let mut src =
                String::from("flowchart TB\nr --> l0[a much much much longer leaf label]\n");
            src.extend((1..k).map(|i| format!("r --> l{i}\n")));
            crate::parser::parse(&src).unwrap()
        };
        let order = |g: &Graph| {
            let lo = layout(g);
            let mut leaves: Vec<usize> = (1..g.nodes.len()).collect();
            leaves.sort_by(|&a, &b| lo.nodes[a].b.total_cmp(&lo.nodes[b].b));
            leaves
        };
        let small = fan(5);
        assert!(within_search_budget(&small));
        assert_ne!(
            order(&small)[0],
            1,
            "big leaf not centred inside the budget"
        );
        let big = fan(400);
        assert!(!within_search_budget(&big));
        let o = order(&big);
        assert!(
            o.windows(2).all(|w| w[0] < w[1]),
            "declaration order changed past the budget"
        );
    }

    #[test]
    fn merge_inversions_match_the_pairwise_count() {
        // gap_crossings counts strict inversions; ties are no crossing.
        let mut seed = 7u64;
        for len in [0usize, 1, 2, 3, 8, 33, 100] {
            let a: Vec<usize> = (0..len)
                .map(|_| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    (seed >> 33) as usize % 10
                })
                .collect();
            let want = (0..len)
                .flat_map(|i| (i + 1..len).map(move |j| (i, j)))
                .filter(|&(i, j)| a[i] > a[j])
                .count();
            let (mut v, mut buf) = (a.clone(), vec![0; len]);
            assert_eq!(inversions(&mut v, &mut buf), want, "{a:?}");
        }
    }

    #[test]
    fn weighted_median_and_layer_compaction() {
        assert_eq!(
            weighted_median(&mut [(0.0, 1.0), (10.0, 8.0), (20.0, 1.0)]),
            10.0
        );
        assert_eq!(weighted_median(&mut [(0.0, 1.0), (10.0, 1.0)]), 5.0);
        // Empty layers vanish, order and relative depth stay.
        assert_eq!(compact_layers(vec![0, 3, 3, 5]), vec![0, 1, 1, 2]);
    }

    #[test]
    fn spans_interpret_bold_italic_and_keep_unknown_tags() {
        assert_eq!(spans("plain"), vec![("plain".into(), false, false)]);
        assert_eq!(
            spans("<b>Go</b> · MongoDB"),
            vec![
                ("Go".into(), true, false),
                (" · MongoDB".into(), false, false)
            ]
        );
        assert_eq!(
            spans("a <i>b</i> <strong>c</strong>"),
            vec![
                ("a ".into(), false, false),
                ("b".into(), false, true),
                (" ".into(), false, false),
                ("c".into(), true, false),
            ]
        );
        // Unknown tags stay literal; unclosed styles run to line end.
        assert_eq!(spans("<x>y"), vec![("<x>y".into(), false, false)]);
        assert_eq!(spans("<b>y"), vec![("y".into(), true, false)]);
        // Comparison `a < b` is untouched (no closing '>').
        assert_eq!(spans("a < b"), vec![("a < b".into(), false, false)]);
    }

    #[test]
    fn math_spans_render_as_italic_tex_with_fences_stripped() {
        // #12 Phase A: mermaid math syntax tokenizes instead of
        // breaking; the TeX body shows as an italic run.
        assert_eq!(spans("$$x^2$$"), vec![("x^2".into(), false, true)]);
        assert_eq!(
            spans("area $$\\frac{1}{2}$$ done"),
            vec![
                ("area ".into(), false, false),
                ("\\frac{1}{2}".into(), false, true),
                (" done".into(), false, false),
            ]
        );
        // Inside bold, math keeps the bold and adds italic.
        assert_eq!(
            spans("<b>E = $$mc^2$$</b>"),
            vec![("E = ".into(), true, false), ("mc^2".into(), true, true)]
        );
        // Unclosed / empty fences stay literal (prices, shell text).
        assert_eq!(spans("$$ 100"), vec![("$$ 100".into(), false, false)]);
        assert_eq!(spans("$$$$"), vec![("$$$$".into(), false, false)]);
        assert_eq!(spans("a $5 b $7"), vec![("a $5 b $7".into(), false, false)]);
        // Fences must HUG their body — a deliberate divergence from
        // mermaid's lazy regex (which would math the padded body and
        // mangle prose): stray double-dollars keep rendering as they
        // did pre-math.
        assert_eq!(
            spans("fee $$ plus $$ tax"),
            vec![("fee $$ plus $$ tax".into(), false, false)]
        );
        assert_eq!(spans("$$ x^2 $$"), vec![("$$ x^2 $$".into(), false, false)]);
        // A genuine TeX less-than IS math (only real style tags keep
        // a body literal).
        assert_eq!(spans("$$x<y$$"), vec![("x<y".into(), false, true)]);
        // A style tag inside a would-be body keeps the dollars
        // literal, so the tag is still interpreted and bold can't
        // get stuck on past its `</b>`.
        assert_eq!(
            spans("<b>x $$a</b>$$ y"),
            vec![("x $$a".into(), true, false), ("$$ y".into(), false, false)]
        );
        // Odd runs pair lazily like mermaid: the empty first match
        // degrades to literal, and everything stays one text run.
        assert_eq!(spans("$$$$$a$$"), vec![("$$$$$a$$".into(), false, false)]);
        // Adjacent maths both tokenize.
        assert_eq!(
            spans("$$a$$$$b$$"),
            vec![("a".into(), false, true), ("b".into(), false, true)]
        );
        // A tag-carrying span's closing fence is SPENT — it may not
        // re-open and shift later fences out of phase: `y` stays
        // literal, and a later hugging pair is still math.
        assert_eq!(
            spans("$$<b>x</b>$$y$$"),
            vec![
                ("$$".into(), false, false),
                ("x".into(), true, false),
                ("$$y$$".into(), false, false),
            ]
        );
        assert_eq!(
            spans("$$<i>a</i>$$ $$c$$"),
            vec![
                ("$$".into(), false, false),
                ("a".into(), false, true),
                ("$$ ".into(), false, false),
                ("c".into(), false, true),
            ]
        );
        // `<br` only counts as a tag when it actually closes as one —
        // a genuine TeX less-than stays math.
        assert_eq!(
            spans("$$p<br_rate$$"),
            vec![("p<br_rate".into(), false, true)]
        );
        // The fences don't count toward measured width.
        assert!(text_width("$$xy$$") < text_width("$$xy$$ ") + 1.0);
        assert_eq!(text_width("$$xy$$"), text_width("<i>xy</i>"));
    }

    #[test]
    fn styling_tags_do_not_inflate_text_width() {
        let tagged = text_width("<b>Rust</b> · Tonic gRPC");
        let plain = text_width("Rust · Tonic gRPC");
        assert!(tagged >= plain, "bold run measures slightly wider");
        assert!(
            tagged < plain * 1.1,
            "tags themselves must not count: {tagged} vs {plain}"
        );
    }

    #[test]
    fn bold_labels_render_as_svg_tspans_not_literal_tags() {
        let svg = crate::render_svg("flowchart TD\nA[\"<b>Go</b> service\"] --> B").unwrap();
        assert!(
            svg.contains("font-weight=\"bold\">Go</tspan>"),
            "bold tspan"
        );
        assert!(!svg.contains("&lt;b&gt;"), "no literal <b> in output");
    }

    #[test]
    fn symbol_glyphs_are_not_under_measured() {
        // Issue #61: arrows, em dashes and emoji render ~1em (14px at
        // the calibration size); a box sized for 7.8px each clipped
        // them. Joiners and variation selectors draw nothing.
        assert_eq!(text_width("→→→→→→→→→→"), 140.0);
        assert_eq!(text_width("——"), 28.0);
        assert_eq!(text_width("⚠\u{FE0F}"), text_width("⚠"));
        assert!(text_width("⛔ ✅ ✕") >= 3.0 * 14.0);
        // Plain Latin text keeps its old estimate.
        assert_eq!(text_width("Wazuh 4.14"), line_width("Wazuh 4.14"));
    }

    /// Strongly connected components are named by their smallest
    /// vertex, whatever the order of the arcs.
    #[test]
    fn components_name_each_cycle_by_its_first_vertex() {
        let arcs = [(0, 1), (1, 2), (2, 1), (2, 3), (4, 3), (3, 4), (5, 5)];
        assert_eq!(components(6, &arcs), [0, 1, 1, 3, 3, 5]);
        let mut back: Vec<(usize, usize)> = arcs.to_vec();
        back.reverse();
        assert_eq!(components(6, &back), [0, 1, 1, 3, 3, 5]);
        assert_eq!(components(3, &[]), [0, 1, 2]);
    }

    /// The exact order turns the least weight, and among equals is the
    /// earliest sequence of units.
    #[test]
    fn the_least_turning_order_prefers_declaration_order_among_equals() {
        // A triangle 0 -> 1 -> 2 -> 0: any order turns one link; the
        // declared one turns `2 -> 0`.
        let mut w = vec![vec![0u64; 3]; 3];
        w[0][1] = 1;
        w[1][2] = 1;
        w[2][0] = 1;
        assert_eq!(least_turned(&w), (1, vec![0, 1, 2]));
        // One link down, four back up: the other unit first.
        let mut w = vec![vec![0u64; 2]; 2];
        w[0][1] = 1;
        w[1][0] = 4;
        assert_eq!(least_turned(&w), (1, vec![1, 0]));
        // No link at all: declaration order, nothing turned.
        assert_eq!(least_turned(&vec![vec![0u64; 3]; 3]), (0, vec![0, 1, 2]));
    }

    /// The walk's order of a level stands until it turns more than
    /// `2 * M + 1` drawn links of a component.
    #[test]
    fn a_block_order_keeps_the_walk_within_twice_the_minimum() {
        let blocks = [true, true];
        // 0 -> 1 once, 1 -> 0 three times: the walk turns 3, the
        // minimum is 1, and 3 <= 2 * 1 + 1.
        let arcs = [((0, 1), 1), ((1, 0), 3)];
        assert_eq!(block_order(&blocks, &arcs, true), [false, true]);
        // Four times: 4 > 3, the component takes its exact order.
        let arcs = [((0, 1), 1), ((1, 0), 4)];
        assert_eq!(block_order(&blocks, &arcs, true), [true, false]);
        // Without the exact net (a drawing past the walk budget) the
        // walk stands.
        assert_eq!(block_order(&blocks, &arcs, false), [false, true]);
        // An undrawn link (weight 0) is turned for free and never makes
        // a drawn one turn.
        let arcs = [((0, 1), 0), ((1, 0), 1)];
        assert_eq!(block_order(&blocks, &arcs, true), [false, true]);
    }

    /// A plain node nothing links into does not start the walk: where
    /// its links enter does. Units: 0 and 1 loose nodes, 2 and 3 blocks
    /// in a cycle; node 0 feeds block 3 once, node 1 feeds block 2
    /// twice. The walk starts at block 2 (most entry links), whichever
    /// node is declared first.
    #[test]
    fn a_block_order_starts_where_the_loose_sources_enter() {
        let blocks = [false, false, true, true];
        let arcs = [((0, 3), 1), ((1, 2), 2), ((2, 3), 1), ((3, 2), 1)];
        // 3 -> 2 is turned: block 2 comes first.
        assert_eq!(
            block_order(&blocks, &arcs, true),
            [false, false, false, true]
        );
    }

    /// A level's rows: longest path, a source block beside what it
    /// feeds, and no two blocks interleaved.
    #[test]
    fn level_rows_seat_source_blocks_and_never_interleave() {
        // Units: 0 a node, 1 a block of three rows, 2 a block of two
        // rows, 3..=7 a chain of nodes, 8 a sink. 0 -> 1, 0 -> 3 -> .. ->
        // 7 -> 8, 2 -> 8.
        let rows = [1, 3, 2, 1, 1, 1, 1, 1, 1];
        let is_block = [false, true, true, false, false, false, false, false, false];
        let mut cons = vec![
            (0, 1),
            (0, 3),
            (3, 4),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 8),
            (2, 8),
        ];
        let start = level_rows(&rows, &is_block, &mut cons);
        // The source block would start at 6 - 2 = 4, interleaving with
        // nothing: block 1 holds rows 1-3, block 2 rows 4-5.
        assert_eq!(start[1], 1);
        assert_eq!(start[2], 4);
        assert_eq!(start[8], 6);
        // Two blocks that overlap without one holding the other: the
        // later one starts after the earlier.
        let rows = [3, 4, 2, 1, 1];
        let is_block = [true, true, true, false, false];
        let mut cons = vec![(3, 1), (4, 0), (2, 1)];
        let start = level_rows(&rows, &is_block, &mut cons);
        let end = |u: usize| start[u] + rows[u];
        for a in 0..3 {
            for b in 0..3 {
                assert!(
                    !(start[a] < start[b] && start[b] < end(a) && end(a) < end(b)),
                    "{start:?}"
                );
            }
        }
        assert_eq!((0..5).map(end).max(), Some(6));
    }

    /// The fence of a block layering: a node stays within the rows of
    /// its blocks and leaves none of them empty.
    #[test]
    fn a_fence_keeps_a_node_in_its_blocks_rows() {
        let blocks = BlockRanking {
            flagged: Vec::new(),
            // Nodes 0 and 1 in block 0, node 2 in block 1 inside block
            // 0, node 3 outside.
            paths: vec![vec![0], vec![0], vec![0, 1], vec![]],
            fired: Vec::new(),
        };
        let layer = [0, 2, 1, 0];
        let mut fence = Fence::new(&blocks, &layer);
        assert!(fence.holds(0, 2) && !fence.holds(0, 3));
        // Node 0 is alone on its block's first row: it may not leave it.
        assert!(!fence.allows(0, 0, 1));
        // Node 2 is all of block 1: it has nowhere to go.
        assert!(!fence.allows(2, 1, 2));
        // A node outside every block is free.
        assert!(fence.allows(3, 0, 5));
        fence.moved(3, 0, 5);
        assert!(fence.unchanged(&[0, 2, 1, 5]));
        assert!(!fence.unchanged(&[1, 2, 1, 0]));
    }

    /// A block layout's label slot is the middle of the chain's run at
    /// its ends' common level; without one, the middle of the chain.
    #[test]
    fn a_label_slot_is_the_middle_of_the_run_between_the_end_blocks() {
        // Two points inside the source block (subgraph 1), three
        // between the blocks, one inside the target block (2).
        let paths = vec![vec![1], vec![1], vec![], vec![], vec![], vec![2]];
        assert_eq!(label_slot(&paths, &[]), 3);
        // Both blocks inside subgraph 0: the run at that level.
        let nested = vec![vec![0, 1], vec![0], vec![0, 2]];
        assert_eq!(label_slot(&nested, &[0]), 1);
        // The chain never leaves the end blocks: its middle point.
        let inside = vec![vec![1], vec![1], vec![2], vec![2]];
        assert_eq!(label_slot(&inside, &[]), 2);
    }

    /// Only an upstream edge between two plain nodes of one level keeps
    /// its lane in a block layout; before blocks every back-edge does.
    #[test]
    fn a_lane_is_kept_between_two_plain_nodes_of_one_level() {
        let paths = vec![vec![0], vec![0], vec![0, 1], vec![]];
        assert!(keeps_lane(None, 0, 2));
        assert!(keeps_lane(Some(&paths), 0, 1));
        assert!(!keeps_lane(Some(&paths), 0, 2));
        assert!(!keeps_lane(Some(&paths), 3, 1));
    }
}
