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

use crate::labels::{FAN_FAR, FAN_SIBLING_GAP, STROKE_PAD};
use crate::model::{Direction, EdgeKind, Graph, Node, Shape};
use crate::scene::{port_lean, ARROW_MARKER_PX};
use std::borrow::Cow;
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
/// is narrowed at all. Below it, re-layering costs more than it gives:
/// the longer edges and channel detours of a stacked or staggered
/// drawing outweigh the breadth saved on a moderately wide one.
const RIBBON_ENTRY: f64 = 4.0;
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
/// How far a back-edge bow clears the nodes it passes, before half its
/// label (the scene's `edge_points` bow clearance).
const BOW_CLEAR: f64 = 24.0;
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

/// Least gap (px, at least `need`) between two TD/BT layers that lets
/// the label stage stack the labels of the compact branch points' fans
/// `lines` between them, as it does (`labels::stack_fans`): per point, in
/// breadth order, each label takes the first row (one label box plus
/// [`LABEL_CLEAR`] apart, back from [`FAN_HEAD_ROOM`]) past [`FAN_FAR`]
/// where it lies [`FAN_SIBLING_GAP`] clear of the point's labels in that
/// row and clear of the point's other lines. A gap no growth makes room
/// in stays `need`.
fn fan_rows_gap(lines: &[FanLine], need: f64) -> f64 {
    if lines.iter().all(|l| l.width.is_none()) {
        return need;
    }
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
                (0..)
                    .map_while(|r: usize| {
                        let depth = FAN_HEAD_ROOM + LABEL_BOX_H / 2.0 + r as f64 * pitch;
                        let share = 1.0 - depth / gap;
                        (share >= FAN_FAR).then_some((r, share))
                    })
                    .any(|(r, share)| {
                        let b = fan_line_at(l, share);
                        let (x0, x1) = (b - w / 2.0, b + w / 2.0);
                        // The other lines where the box's near edge lies,
                        // closest to it there.
                        let near = share - LABEL_BOX_H / 2.0 / gap;
                        let clear = fan.iter().filter(|o| !std::ptr::eq(**o, *l)).all(|o| {
                            let ob = fan_line_at(o, near);
                            ob < x0 - STROKE_PAD - LABEL_CLEAR || ob > x1 + STROKE_PAD + LABEL_CLEAR
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
                        clear && free
                    })
            })
        })
    };
    let least = need.max(fan_gap(LABEL_BOX_H / 2.0));
    let mut gap = least;
    while gap <= least + FAN_GAP_MAX {
        if fits(gap) {
            return gap;
        }
        gap += LABEL_CLEAR / 2.0;
    }
    need
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
        let mut cur = String::new();
        for word in line.split(' ').filter(|w| !w.is_empty()) {
            if cur.is_empty() {
                cur.push_str(word);
                continue;
            }
            let cand = format!("{cur} {word}");
            if text_width(&cand) * scale > max_w {
                out.push(std::mem::replace(&mut cur, word.to_string()));
            } else {
                cur = cand;
            }
        }
        out.push(cur);
    }
    out.join("\n")
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
/// whose renderer uses a REAL font should use [`intrinsic_size_with`]
/// and feed the sizes to `scene::scene_sized` — that is the documented
/// path for accurate node boxes (issue #14).
pub fn intrinsic_size(node: &Node) -> (f64, f64) {
    intrinsic_size_with(node, text_width)
}

/// [`intrinsic_size`] with an injectable text measurer: `measure`
/// receives one raw label LINE (may contain `<b>`/`<i>` tags — strip
/// or style them via [`spans`]) and returns its rendered width in px
/// at the diagram's 14px base font ([`TEXT_CALIBRATION`]). Shape
/// padding, multi-line growth and minimums are applied on top, using
/// the same public constants ([`PAD_X`], [`BASE_H`], [`MIN_W`],
/// [`LINE_H`]) the engine uses.
pub fn intrinsic_size_with(node: &Node, measure: impl Fn(&str) -> f64) -> (f64, f64) {
    let tw = node.label.split('\n').map(&measure).fold(0.0, f64::max);
    // Height grows with extra label lines beyond the first.
    let extra = (line_count(&node.label) - 1) as f64 * LINE_H;
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
        // Cylinder caps add vertical room.
        Shape::Cylinder => ((tw + 2.0 * PAD_X).max(MIN_W), base_h + 16.0),
        // A diamond with nothing to say is a branch point (a state
        // `<<choice>>`, or a flowchart `A{ }`): compact, mermaid-style.
        Shape::Diamond if is_branch_point(node.shape, &node.label) => (CHOICE_SIZE, CHOICE_SIZE),
        // Diamonds need extra room so the text fits in the middle.
        Shape::Diamond => (((tw + 24.0) * 1.6).max(80.0), base_h * 1.7),
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
    let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
    layout_sized(g, &sizes)
}

/// Same as [`layout`] but with caller-provided node sizes (width,
/// height in pixels) — used by pipelines where node size doesn't
/// come from the label, e.g. ER entity tables or icon nodes.
pub fn layout_sized(g: &Graph, sizes: &[(f64, f64)]) -> LayoutResult {
    layout_core(g, sizes, None, &[]).0
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
    layout_core(g, sizes, Some(node_cluster), &[]).0
}

/// Per original edge, per channel waypoint: the cluster path the
/// waypoint's dummy was laid out in (see [`layout_clustered_ranked`]).
pub(crate) type EdgeGroups = Vec<Vec<Vec<usize>>>;

/// A rank-only link `(from, to)`: every `to` node ranks at least one
/// layer below every `from` node.
pub(crate) type RankGroup = (Vec<usize>, Vec<usize>);

/// [`layout_clustered`] plus `rank_only` groups that rank their `to`
/// nodes below their `from` nodes and pull them together in the
/// ordering, but get no channel and are never drawn — the stand-ins for
/// edges that end on a whole subgraph box. Also returns, for every
/// edge, the cluster path of each channel waypoint, so the caller can
/// tell which box a lane runs inside.
pub(crate) fn layout_clustered_ranked(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: &[Vec<usize>],
    rank_only: &[RankGroup],
) -> (LayoutResult, EdgeGroups) {
    layout_core(g, sizes, Some(node_cluster), rank_only)
}

fn layout_core(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: Option<&[Vec<usize>]>,
    rank_only: &[RankGroup],
) -> (LayoutResult, EdgeGroups) {
    assert_eq!(
        sizes.len(),
        g.nodes.len(),
        "number of sizes must match number of nodes"
    );
    let groups: Vec<RankGroup> = rank_only.to_vec();
    let rk = rank(g, &groups);
    let layer = rk.layer.clone();
    // The search judges candidates on drawn crossings and hits; a graph
    // past the budget skips it and never pays for those measures.
    let search = within_search_budget(g);
    let plain = Opts {
        judge: search,
        ..Opts::default()
    };
    let first = place(g, sizes, node_cluster, &groups, &rk, &layer, plain);
    let mut st = State {
        groups,
        rk,
        layer,
        best: first,
        opts: plain,
        work: SEARCH_WORK,
    };
    if !search {
        return (st.best.lo, st.best.groups);
    }
    if st.best.lo.total_b > RIBBON_ENTRY * st.best.lo.total_l {
        narrow(g, sizes, node_cluster, &mut st);
    }
    centre_largest(g, sizes, node_cluster, &mut st);
    (st.best.lo, st.best.groups)
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
    let ribbon = |p: &Placement| p.lo.total_b > ASPECT_TARGET * p.lo.total_l;
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
    );
    if packed.lo.total_b <= st.best.lo.total_b && packed.crossings <= cap {
        st.best = packed;
        st.opts = packed_opts;
    }
    // Node hits may grow by NARROW_HIT_SLACK per step, and by no more
    // over the packed drawing the re-layering steps start from, so
    // several steps cannot pile up re-threadings.
    let hit_cap = st.best.hits + NARROW_HIT_SLACK;
    let better = |cand: &Placement, cur: &Placement| {
        cand.lo.total_b <= cur.lo.total_b * (1.0 - STAGGER_MIN_GAIN)
            && cand.crossings <= cap
            && cand.hits <= hit_cap.min(cur.hits + NARROW_HIT_SLACK)
    };
    // 1. Sibling stacking: two sibling subgraphs that share layers sit
    // side by side, so their parent is as broad as both. The pairs that
    // would give back the most breadth are tried ranked one below the
    // other (a rank-only group, never drawn), unless an edge path runs
    // the other way; the narrowest result wins the round.
    if let Some(nc) = node_cluster {
        let mut tried: Vec<(usize, usize)> = Vec::new();
        for _ in 0..STACK_MAX_ROUNDS {
            if !ribbon(&st.best) {
                break;
            }
            let pairs = stack_candidates(g, nc, &st.groups, &st.rk, &st.layer, &st.best.lo, &tried);
            if pairs.is_empty() {
                break;
            }
            let mut won: Option<(Vec<RankGroup>, Ranking, Placement)> = None;
            for &(above, below) in pairs.iter().take(STACK_BEAM) {
                if !st.afford(g) {
                    break;
                }
                tried.push((above, below));
                let mut cand_groups = st.groups.clone();
                cand_groups.push(stack_group(nc, above, below));
                let crk = rank(g, &cand_groups);
                let cand = place(
                    g,
                    sizes,
                    node_cluster,
                    &cand_groups,
                    &crk,
                    &crk.layer,
                    packed_opts,
                );
                let cur = won.as_ref().map_or(&st.best, |w| &w.2);
                if better(&cand, &st.best) && cand.lo.total_b < cur.lo.total_b {
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
    // becomes two offset rows.
    let mut tried: Vec<usize> = Vec::new();
    for _ in 0..STAGGER_MAX_ROUNDS {
        if !ribbon(&st.best) {
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
        let cand = place(g, sizes, node_cluster, &st.groups, &st.rk, &st.layer, opts);
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
}

/// Steps 1 and 2: mark back-edges, then longest-path layering.
fn rank(g: &Graph, rank_only: &[RankGroup]) -> Ranking {
    let n = g.nodes.len();
    // Ranking graph: the real nodes, then one HUB per rank-only group
    // (`from -> hub` weighs one layer, `hub -> to` none), so a group
    // costs |from| + |to| links instead of every from x to pair.
    let nx = n + rank_only.len();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); nx];
    for (ei, e) in g.edges.iter().enumerate() {
        adj[e.from].push((e.to, ei));
    }
    // Hub links ride the same DFS / layering as real edges, numbered
    // after them; `group_eid[k]` is group k's first link id.
    let ne = g.edges.len();
    let mut nlinks = ne;
    let mut group_eid = Vec::with_capacity(rank_only.len());
    for (k, (from, to)) in rank_only.iter().enumerate() {
        group_eid.push(nlinks);
        for &a in from {
            adj[a].push((n + k, nlinks));
            nlinks += 1;
        }
        for &b in to {
            adj[n + k].push((b, nlinks));
            nlinks += 1;
        }
    }

    // --- 1. Mark back-edges (cycle breakers) with iterative DFS ---
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

    // --- 3. Lead nodes (a state `note left of X`) one layer before
    // their nearest target, as dagre's tight ranking puts them: the
    // pass above parks every source on layer 0. Only a source moves, so
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

/// Each movable source one layer above its nearest target, or `None`
/// when no source moves.
fn tighten_sources(
    g: &Graph,
    groups: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
) -> Option<Vec<usize>> {
    let nl = node_links(g, groups, rk, layer);
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
    // Leaves: +1 = steps down (sink or edge-less), -1 = steps up.
    let leaves = every_second(&|v| {
        if nl.outs[v].is_empty() && layer[v] < nl.deepest[v] {
            Some(1)
        } else if nl.ins[v].is_empty()
            && !nl.outs[v].is_empty()
            && layer[v] as isize > nl.shallowest[v]
        {
            Some(-1)
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
        // other way: skip rather than re-break cycles.
        if !(0..n).any(|v| down[v] && nl.pinned[v]) {
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
}

/// Steps 3 to 5 for one layering: dummy chains, ordering and
/// coordinates. `layer` holds one layer per real node and must keep
/// every non-back ranking link pointing down by at least its weight.
fn place(
    g: &Graph,
    sizes: &[(f64, f64)],
    node_cluster: Option<&[Vec<usize>]>,
    rank_only: &[RankGroup],
    rk: &Ranking,
    layer: &[usize],
    opts: Opts,
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
    for (ei, e) in g.edges.iter().enumerate() {
        if e.from == e.to {
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
            if llo < lhi {
                succs[lo].push(hi);
                preds[hi].push(lo);
            }
            continue; // same or adjacent layer — no dummies
        }
        // Invisible links (ranking-only) and back-edges get NO channel:
        // the former must not inflate the canvas, and a back-edge routed
        // up the middle would cross every forward edge — it keeps the
        // sideways bow instead. Both still influence ordering directly.
        if matches!(e.kind, EdgeKind::Invisible) || back[ei] {
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
        if let (Some(lbl), false) = (&e.label, chain.is_empty()) {
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
    // Extra layer-slot room per real node for title strips (see below).
    let mut lextra = vec![0.0f64; n];
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
        // end on its visible border. In LR/RL the strip lies across the
        // breadth axis instead and is reserved there (compaction). In BT
        // the layers are drawn bottom-up, so a box's screen top is its
        // HIGHEST layer.
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
                lextra[v] = extra * 2.0;
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
                p[d] = if kv > 0 {
                    nc[e.to][..kv].to_vec()
                } else if ku > 0 {
                    nc[e.from][..ku].to_vec()
                } else {
                    common_prefix(&nc[e.from], &nc[e.to])
                };
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
    if clustered {
        let keys = cluster_keys(&layers, &apath, &border_side);
        for lv in layers.iter_mut() {
            enforce_contiguity(lv, &apath, &border_side, &mut pos, &keys);
        }
    }
    let mut best_layers = layers.clone();
    let mut best_cross = count_crossings(&layers, &succs, &alayer, nlayers);
    for _ in 0..8 {
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
    if clustered && !bar_cards(g).is_empty() {
        let keys = cluster_keys(&layers, &apath, &border_side);
        for lv in layers.iter_mut() {
            enforce_contiguity(lv, &apath, &border_side, &mut pos, &keys);
        }
    }
    // A subgraph [`centre_largest`] moves: its key changes, every layer
    // regroups around it.
    if let (true, Some((c, key))) = (clustered, opts.hint) {
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
    let mut bpos = coordinates_bk(
        n,
        na,
        nlayers,
        &layers,
        preds_c,
        succs_c,
        &alayer,
        &absize,
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
            &absize,
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
            back,
            node_cluster,
            &absize,
            &alayer,
            bpos,
            edge_chain.iter().flatten().copied(),
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
            Some(l) => bpos[l] + pair_sep(l, v, n, &absize, ap, bs, hb),
            None => ext.0 + half,
        };
        let hi = match lay.get(s + 1) {
            Some(&r) => bpos[r] - pair_sep(v, r, n, &absize, ap, bs, hb),
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
    let mut lcoord = vec![0.0f64; nlayers];
    let mut cursor = MARGIN;
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
    type ShortLabel = (usize, usize, f64, f64, bool);
    let mut short_labels: Vec<Vec<ShortLabel>> = vec![Vec::new(); nlayers];
    // Per upper layer: (breadth centre, breadth extent, layer extent,
    // end room).
    let mut short_across: Vec<Vec<(f64, f64, f64, f64)>> = vec![Vec::new(); nlayers];
    // Per upper layer: the lines of compact branch points' fans
    // (exactly one end such a point, as the label stage tells them)
    // between it and the layer before, labelled or not (see `FanLine`).
    let mut fan_lines: Vec<Vec<FanLine>> = vec![Vec::new(); nlayers];
    let branch = |v: usize| is_branch_point(g.nodes[v].shape, &g.nodes[v].label);
    for (ei, e) in g.edges.iter().enumerate() {
        if e.from == e.to || back[ei] || matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        let fan = match (branch(e.from), branch(e.to)) {
            (true, false) => Some((e.from, e.to)),
            (false, true) => Some((e.to, e.from)),
            _ => None,
        };
        let short = alayer[e.from].abs_diff(alayer[e.to]) == 1;
        if let (Some((p, far)), true, false) = (fan, short, horizontal) {
            // Near its far end a fan's line runs at that end's port,
            // leaning toward the point on a box (see `scene::anchor`).
            let lean = match g.nodes[far].shape {
                Shape::Diamond
                | Shape::Circle
                | Shape::DoubleCircle
                | Shape::StateStart
                | Shape::StateEnd
                | Shape::ForkBar => 0.0,
                shape => port_lean(shape, (absize[far], alsize[far]), bpos[p] - bpos[far], 0.0),
            };
            // It leaves the small diamond's border on the way to the far
            // end's centre: about a quarter of the diamond off its centre
            // for a slanted line.
            let toward = bpos[far] - bpos[p];
            fan_lines[alayer[e.from].max(alayer[e.to])].push(FanLine {
                point: p,
                from: bpos[p] + toward.clamp(-CHOICE_SIZE / 4.0, CHOICE_SIZE / 4.0),
                to: bpos[far] + lean,
                width: e.label.as_ref().map(|l| text_width(l) + LABEL_PAD_W),
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
            short_labels[alayer[hi]].push((lo, hi, 0.0, room, false));
            continue;
        };
        let w = text_width(lbl) + LABEL_PAD_W;
        let (ext, across) = if horizontal {
            (w, LABEL_BOX_H)
        } else {
            (LABEL_BOX_H, w)
        };
        short_labels[alayer[hi]].push((lo, hi, ext, room, fan.is_some()));
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
    let mut prev_lh = 0.0;
    for li in 0..nlayers {
        let lh = layers[li]
            .iter()
            .map(|&v| alsize[v] + lextra.get(v).copied().unwrap_or(0.0))
            .fold(0.0f64, f64::max);
        if clustered && li > 0 {
            let at = lcoord[li - 1] + reach_of(li - 1, 1) + reach_of(li, 0) + FRAME_GAP;
            if at > cursor + lh / 2.0 {
                cursor = at - lh / 2.0;
            }
        }
        // A label that would cover its own ends grows the gap until it
        // clears both by LABEL_CLEAR; a label that already fits leaves
        // the gap — and the whole layout — exactly as it was. An end
        // whose frame closes / opens on its layer is reached past that
        // frame's border (the layer's farthest one), not its own box:
        // a label on the border line would hide it. End room counts as
        // part of the ends: the label (or, without one, the other
        // end's glyph) clears it by LABEL_CLEAR too.
        for &(s, t, ext, room, fan) in &short_labels[li] {
            let lo_side = if reach[s].1 > 0.0 {
                reach_of(li - 1, 1)
            } else {
                alsize[s] / 2.0
            };
            let hi_side = if reach[t].0 > 0.0 {
                reach_of(li, 0)
            } else {
                alsize[t] / 2.0
            };
            let ends = lcoord[li - 1] + (lo_side + hi_side);
            let at = cursor + lh / 2.0;
            if at - ends < ext + room {
                cursor = cursor.max(ends + room + ext + 2.0 * LABEL_CLEAR - lh / 2.0);
            }
            if fan && at - ends < fan_gap(ext / 2.0) {
                cursor = cursor.max(ends + fan_gap(ext / 2.0) - lh / 2.0);
            }
        }
        if li > 0 && gap_need[li] > 0.0 {
            let prev = lcoord[li - 1] + prev_lh / 2.0;
            if cursor - prev < gap_need[li] {
                cursor = prev + gap_need[li];
            }
        }
        prev_lh = lh;
        lcoord[li] = cursor + lh / 2.0;
        cursor += lh + GAP_L;
    }
    let total_l = cursor - GAP_L + MARGIN;

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
    let (crossings, hits, ext) = if opts.judge {
        (
            drawn_crossings(g, &edge_paths, |v| (bpos[v], lcoord[alayer[v]])),
            node_hits(g, back, &edge_paths, |v| {
                (bpos[v], lcoord[alayer[v]], absize[v], alsize[v])
            }),
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
/// (label slots included), and the sideways bow of every back-edge,
/// which clears the nodes on the layers it spans by BOW_CLEAR plus half
/// its label, on the side nearer its ends (as the scene draws it).
fn drawn_breadth(
    g: &Graph,
    back: &[bool],
    node_cluster: Option<&[Vec<usize>]>,
    absize: &[f64],
    alayer: &[usize],
    bpos: &[f64],
    dummies: impl Iterator<Item = usize>,
) -> (f64, f64) {
    let n = g.nodes.len();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in 0..n {
        let pad = node_cluster.map_or(0.0, |nc| nc[v].len() as f64 * FRAME_PAD);
        lo = lo.min(bpos[v] - absize[v] / 2.0 - pad);
        hi = hi.max(bpos[v] + absize[v] / 2.0 + pad);
    }
    for d in dummies {
        lo = lo.min(bpos[d] - absize[d] / 2.0);
        hi = hi.max(bpos[d] + absize[d] / 2.0);
    }
    for (ei, e) in g.edges.iter().enumerate() {
        if !back[ei] || e.from == e.to || matches!(e.kind, EdgeKind::Invisible) {
            continue;
        }
        let (l0, l1) = (
            alayer[e.from].min(alayer[e.to]),
            alayer[e.from].max(alayer[e.to]),
        );
        if l0 == l1 {
            continue;
        }
        let (mut el, mut er) = (f64::INFINITY, f64::NEG_INFINITY);
        for v in (0..n).filter(|&v| alayer[v] >= l0 && alayer[v] <= l1) {
            el = el.min(bpos[v] - absize[v] / 2.0);
            er = er.max(bpos[v] + absize[v] / 2.0);
        }
        let clear = BOW_CLEAR
            + e.label
                .as_ref()
                .map_or(0.0, |l| text_width(l) + LABEL_PAD_W)
                / 2.0;
        let mid = (bpos[e.from] + bpos[e.to]) / 2.0;
        if er + clear - mid <= mid - (el - clear) {
            hi = hi.max(er + clear);
        } else {
            lo = lo.min(el - clear);
        }
    }
    if lo > hi {
        (0.0, 0.0)
    } else {
        (lo, hi)
    }
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
    // layer are excluded — BK only reasons about layer±1 segments).
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

    /// `Edge::end_room` (ER crow's feet) widens a gap between adjacent
    /// layers by just enough for the room, the label and LABEL_CLEAR
    /// either side — and an edge without room or label leaves the gap
    /// exactly as it was.
    #[test]
    fn end_room_widens_an_adjacent_gap_for_glyphs_and_label() {
        let gap = |room: (f64, f64), label: Option<&str>, dir: Direction| {
            let mut g = Graph::default();
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
        // Labelled diamonds keep the text-fitting rule and its minimum.
        let g = crate::parser::parse("flowchart TD\nA{ok?} --> B{a much longer question}").unwrap();
        let (wa, ha) = intrinsic_size(&g.nodes[0]);
        assert_eq!((wa, ha), (80.0, BASE_H * 1.7), "short label: the minimum");
        let (wb, _) = intrinsic_size(&g.nodes[1]);
        let tw = text_width("a much longer question");
        assert_eq!(wb, (tw + 24.0) * 1.6, "long label: text-fitted");
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
}
