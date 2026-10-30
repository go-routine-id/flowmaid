//! Sequence diagram rendering: participant boxes across the top,
//! dashed lifelines below, one row per statement top-down, and the
//! participant boxes repeated underneath (mermaid's `mirrorActors`).
//!
//! Unlike `er` / `class`, nothing here is a graph — the Sugiyama
//! pipeline is skipped entirely in favour of a custom linear
//! layout: column x-positions come from declaration order (gaps
//! widened until every message label and note fits), row
//! y-positions from source order.
//!
//! API shape matches the other diagram modules: [`scene`] computes
//! all geometry a GUI painter needs ([`SeqScene`]), [`to_svg`]
//! serialises any scene, and [`head`] / [`actor_figure`] expose
//! arrowheads and actor stick figures as plain geometry so painters
//! draw exactly what SVG exports.

use crate::layout::text_width;
use crate::model::{FrameKind, NoteSide, SeqHead, SeqItem, SequenceDiagram};
use crate::scene::{escape, svg_open_a11y, A11y, SvgOptions, EDGE_COLOR, TEXT_COLOR};
use crate::style::NOTE_STYLE;

/// Participant header box height.
pub const BOX_H: f64 = 34.0;
/// Height of an actor's header (and footer) slot: the stick figure
/// with the actor's name below it (see [`actor_figure`]).
pub const ACTOR_H: f64 = 48.0;
/// Base row height of one message.
pub const ROW_H: f64 = 34.0;
/// Activation bar width.
pub const ACT_W: f64 = 8.0;
const MARGIN: f64 = 24.0;
const MIN_BOX_W: f64 = 80.0;
const GAP_MIN: f64 = 40.0; // minimum clearance between adjacent boxes
const SELF_W: f64 = 32.0; // self-message loop width
const SELF_H: f64 = 16.0; // self-message loop height
const NOTE_PAD: f64 = 12.0; // horizontal padding inside a note
const NOTE_H: f64 = 26.0;
const FRAME_MARGIN: f64 = 10.0; // frame outset beyond the outer boxes / its content
const FRAME_INSET: f64 = 10.0; // a nested frame's inset inside its parent
const NEST_SHIFT: f64 = 3.0; // a nested activation bar's shift right
/// Dash pattern of dashed messages (on, off) — see [`dash_offset`].
const MSG_DASH: (f64, f64) = (6.0, 4.0);

/// Lifelines + frame borders (lighter than the message lines).
const GUIDE_COLOR: &str = "#aeb6d8";

/// One participant box (header, or its footer mirror). Top-left
/// corner + size; index-aligned with
/// [`SequenceDiagram::participants`]. An actor's box is the slot its
/// stick figure and name are drawn in ([`actor_figure`]), [`ACTOR_H`]
/// tall.
#[derive(Debug, Clone)]
pub struct PartBox {
    pub label: String,
    pub actor: bool,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Dashed vertical lifeline from a participant's header box down to
/// its footer box (or the bottom row when there is no footer).
#[derive(Debug, Clone)]
pub struct Lifeline {
    pub x: f64,
    pub y0: f64,
    pub y1: f64,
}

/// One message: a polyline (2 points, or 4 for the self-message
/// loop) with a head glyph at the LAST point. Ends sit on the
/// lifeline, or on the facing edge of an open activation bar.
#[derive(Debug, Clone)]
pub struct MsgLine {
    pub from: usize,
    pub to: usize,
    pub points: Vec<(f64, f64)>,
    pub dashed: bool,
    pub head: SeqHead,
    /// `autonumber` — rendered as a bold `N. ` prefix.
    pub number: Option<usize>,
    pub text: String,
    /// Label anchor: centered above the line for normal messages,
    /// left-aligned beside the loop for self-messages.
    pub label_pos: (f64, f64),
    pub label_centered: bool,
}

/// A note card.
#[derive(Debug, Clone)]
pub struct NoteBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub text: String,
}

/// Activation bar overlaying a lifeline (white with the
/// participant's accent stroke). Nested bars shift right.
#[derive(Debug, Clone)]
pub struct ActBar {
    pub participant: usize,
    /// Bar center x.
    pub x: f64,
    pub y0: f64,
    pub y1: f64,
}

/// A `loop` / `opt` / `alt` / `par` frame: border rect, label chip
/// at the top-left, and dashed `else`/`and` dividers.
#[derive(Debug, Clone)]
pub struct FrameBox {
    pub kind: FrameKind,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// `else` / `and` dividers: (y, label).
    pub dividers: Vec<(f64, String)>,
}

/// All geometry of a laid-out sequence diagram, in final
/// coordinates — everything a GUI painter needs.
///
/// `#[non_exhaustive]`: build one with `SeqScene::default()` and set
/// the fields, so new geometry can be added without breaking hosts.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SeqScene {
    pub width: f64,
    pub height: f64,
    pub boxes: Vec<PartBox>,
    /// The participant boxes repeated below the lifelines (mermaid's
    /// `mirrorActors`): same x and size as [`boxes`](Self::boxes),
    /// index-aligned with them, top edge on the lifelines' bottom
    /// end. Empty when [`SequenceDiagram::mirror_actors`] is off.
    pub footer: Vec<PartBox>,
    pub lifelines: Vec<Lifeline>,
    pub messages: Vec<MsgLine>,
    pub notes: Vec<NoteBox>,
    pub activations: Vec<ActBar>,
    pub frames: Vec<FrameBox>,
}

/// A message arrowhead as plain geometry: a closed polygon (filled
/// triangle) and/or bare segments — same data feeds the SVG writer
/// and GUI painters.
#[derive(Debug, Clone, Default)]
pub struct Head {
    pub polygon: Vec<(f64, f64)>,
    pub segments: Vec<[(f64, f64); 2]>,
}

/// Head glyph at `tip`, where `back` is the adjacent point of the
/// message line (giving the inbound direction).
pub fn head(tip: (f64, f64), back: (f64, f64), kind: SeqHead) -> Head {
    let (dx, dy) = (back.0 - tip.0, back.1 - tip.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let u = (dx / len, dy / len); // unit vector pointing back along the line
    let nv = (-u.1, u.0);
    let p = |k: f64, t: f64| (tip.0 + u.0 * k + nv.0 * t, tip.1 + u.1 * k + nv.1 * t);
    match kind {
        SeqHead::Filled => Head {
            polygon: vec![tip, p(11.0, -5.5), p(11.0, 5.5)],
            segments: vec![],
        },
        SeqHead::Open | SeqHead::Async => Head {
            polygon: vec![],
            segments: vec![[p(11.0, -6.5), tip], [p(11.0, 6.5), tip]],
        },
        SeqHead::Cross => {
            // An X just before the lifeline (the line still reaches it).
            let c = p(7.0, 0.0);
            let q = |k: f64, t: f64| (c.0 + u.0 * k + nv.0 * t, c.1 + u.1 * k + nv.1 * t);
            Head {
                polygon: vec![],
                segments: vec![[q(-4.5, -4.5), q(4.5, 4.5)], [q(-4.5, 4.5), q(4.5, -4.5)]],
            }
        }
        SeqHead::None => Head::default(),
    }
}

/// An actor's UML stick figure as plain geometry — what [`to_svg`]
/// draws in place of a box, so GUI painters can match it: a head
/// circle (white fill, accent stroke), bare body / arm / leg strokes,
/// and the anchor of the actor's name, centered below the figure.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ActorFigure {
    /// Head circle center.
    pub head: (f64, f64),
    /// Head circle radius.
    pub head_r: f64,
    /// Body, arms and legs.
    pub segments: Vec<[(f64, f64); 2]>,
    /// Name anchor: horizontal center and vertical middle of the
    /// (bold, centered) text.
    pub label_pos: (f64, f64),
}

/// The stick figure for an actor's [`PartBox`] — header or footer
/// alike: figure at the top of the slot, name below it, all inside
/// the box ([`ACTOR_H`] tall).
pub fn actor_figure(b: &PartBox) -> ActorFigure {
    let cx = b.x + b.w / 2.0;
    let y = b.y;
    let (neck, hip, feet) = (y + 13.0, y + 24.0, y + 32.0);
    ActorFigure {
        head: (cx, y + 7.0),
        head_r: 6.0,
        segments: vec![
            [(cx, neck), (cx, hip)],                      // body
            [(cx - 9.0, y + 17.0), (cx + 9.0, y + 17.0)], // arms
            [(cx, hip), (cx - 8.0, feet)],                // legs
            [(cx, hip), (cx + 8.0, feet)],
        ],
        label_pos: (cx, y + 40.0),
    }
}

/// Estimated width of a message label including the bold
/// `autonumber` prefix (small fudge for the bold face).
fn label_width(number: Option<usize>, text: &str) -> f64 {
    match number {
        Some(k) => text_width(&format!("{k}. {text}")) + 3.0,
        None => text_width(text),
    }
}

/// How far right of its lifeline the innermost of `depth` open
/// activation bars reaches (0 with none) — where a self-message loop
/// starts. Matches [`end_bar`]'s [`NEST_SHIFT`] per nesting level.
fn bar_reach(depth: usize) -> f64 {
    match depth {
        0 => 0.0,
        k => (k - 1) as f64 * NEST_SHIFT + ACT_W / 2.0,
    }
}

/// Where a message meets participant `p`: its lifeline, or — while
/// `p` has open activation bars — the outer edge of the bar stack on
/// the side facing `toward` (an x), so heads stop at the bar instead
/// of running into it and lines leave from its edge, not its middle
/// (mermaid does the same). Nested bars shift right, so the stack's
/// left edge is the outermost bar's and its right edge the innermost
/// bar's ([`bar_reach`]).
fn bar_side(act: &[Vec<f64>], x: &[f64], p: usize, toward: f64) -> f64 {
    match act[p].len() {
        0 => x[p],
        k if toward > x[p] => x[p] + bar_reach(k),
        _ => x[p] - ACT_W / 2.0,
    }
}

/// Re-aim the ends of message `m` that sit on participant `p` at
/// `p`'s just-opened bar. An explicit `activate` FOLLOWS the message
/// it belongs to (mermaid starts the bar at that message), so the bar
/// did not exist yet when `m` was laid out.
fn snap_to_bar(m: &mut MsgLine, p: usize, act: &[Vec<f64>], x: &[f64]) {
    let last = m.points.len() - 1;
    if m.from == m.to {
        // Only the loop's return reaches the bar (it starts at the
        // return row); the outgoing leg stays where it was.
        if m.to == p {
            m.points[last].0 = bar_side(act, x, p, f64::INFINITY);
        }
        return;
    }
    if m.to == p {
        m.points[last].0 = bar_side(act, x, p, x[m.from]);
    }
    if m.from == p {
        m.points[0].0 = bar_side(act, x, p, x[m.to]);
    }
    m.label_pos.0 = (m.points[0].0 + m.points[last].0) / 2.0;
}

/// Close the innermost open activation bar of participant `p` at
/// height `y` (bars get a minimum height so `activate` immediately
/// followed by `deactivate` stays visible).
fn end_bar(sc: &mut SeqScene, act: &mut [Vec<f64>], p: usize, y: f64, x: &[f64]) {
    if let Some(y0) = act[p].pop() {
        sc.activations.push(ActBar {
            participant: p,
            x: x[p] + act[p].len() as f64 * NEST_SHIFT,
            y0,
            y1: y.max(y0 + 6.0),
        });
    }
}

/// Width of a frame's keyword chip (`loop`, `alt`, ...).
fn chip_w(kind: FrameKind) -> f64 {
    text_width(kind.keyword()) + 14.0
}

/// Left x of a frame's `[label]` and of its `else` / `and` divider
/// labels: one column right of the keyword chip, so every branch
/// condition of a frame lines up.
fn frame_label_x(f: &FrameBox) -> f64 {
    f.x + chip_w(f.kind) + 6.0
}

/// Every non-empty label of frame `f` as `(center y, "[label]")`:
/// its own label on the chip row, then one per `else` / `and` divider.
fn frame_label_rows(f: &FrameBox) -> impl Iterator<Item = (f64, String)> + '_ {
    std::iter::once((f.y + 9.0, f.label.as_str()))
        .chain(f.dividers.iter().map(|(dy, dl)| (dy + 9.0, dl.as_str())))
        .filter(|(_, label)| !label.is_empty())
        .map(|(cy, label)| (cy, bracketed(label)))
}

/// Width a frame needs so its chip and its widest branch label stay
/// inside its border.
fn frame_label_need(f: &FrameBox) -> f64 {
    let widest = f
        .dividers
        .iter()
        .map(|(_, l)| text_width(&bracketed(l)))
        .fold(text_width(&bracketed(&f.label)), f64::max);
    chip_w(f.kind) + 6.0 + widest + FRAME_MARGIN
}

/// Per-frame layout bookkeeping gathered in the row loop.
struct FrameInfo {
    /// Nesting depth (0 = outermost).
    depth: usize,
    /// Index of the enclosing frame in `SeqScene::frames`.
    parent: Option<usize>,
    /// Horizontal extent of everything drawn inside (nested frames'
    /// content included): the boxes of the columns its messages and
    /// notes touch, note cards, message lines and labels.
    content: Option<(f64, f64)>,
}

/// Grow the content extent of every open frame by `[lo, hi]`.
fn note_content(frames: &mut [FrameInfo], open: &[usize], lo: f64, hi: f64) {
    for &fi in open {
        let c = &mut frames[fi].content;
        *c = Some(c.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))));
    }
}

/// Final x / width of every frame, once all rows are laid out.
///
/// Default: full-width aligned bands — every frame at nesting depth
/// `d` shares one left and right edge, inset [`FRAME_INSET`] per
/// level, spanning all participant boxes plus [`FRAME_MARGIN`], and
/// widened together until each frame contains its content (a
/// `Note left of` the first or `right of` the last participant, a
/// self-message label beside the last lifeline) and its labels.
///
/// `scoped`: each frame spans only the columns it touches (mermaid's
/// look), then grows to contain its nested frames plus
/// [`FRAME_INSET`] and its labels.
fn place_frames(sc: &mut SeqScene, info: &[FrameInfo], x: &[f64], bw: &[f64], scoped: bool) {
    let n = x.len();
    let full_l = x[0] - bw[0] / 2.0 - FRAME_MARGIN;
    let full_r = x[n - 1] + bw[n - 1] / 2.0 + FRAME_MARGIN;
    let inset = |d: usize| d as f64 * FRAME_INSET;
    if !scoped {
        let (mut l, mut r) = (full_l, full_r);
        for fi in info {
            if let Some((lo, hi)) = fi.content {
                l = l.min(lo - FRAME_MARGIN - inset(fi.depth));
                r = r.max(hi + FRAME_MARGIN + inset(fi.depth));
            }
        }
        for (f, fi) in sc.frames.iter().zip(info) {
            r = r.max(l + 2.0 * inset(fi.depth) + frame_label_need(f));
        }
        for (f, fi) in sc.frames.iter_mut().zip(info) {
            f.x = l + inset(fi.depth);
            f.w = (r - l) - 2.0 * inset(fi.depth);
        }
        return;
    }
    // Children come after their parent in `sc.frames` (FrameStart
    // order), so walking backwards finalises every child first.
    for i in (0..sc.frames.len()).rev() {
        let (mut l, mut r) = match info[i].content {
            Some((lo, hi)) => (lo - FRAME_MARGIN, hi + FRAME_MARGIN),
            None => (full_l + inset(info[i].depth), full_r - inset(info[i].depth)),
        };
        for j in i + 1..sc.frames.len() {
            if info[j].parent == Some(i) {
                l = l.min(sc.frames[j].x - FRAME_INSET);
                r = r.max(sc.frames[j].x + sc.frames[j].w + FRAME_INSET);
            }
        }
        r = r.max(l + frame_label_need(&sc.frames[i]));
        sc.frames[i].x = l;
        sc.frames[i].w = r - l;
    }
}

/// Linear layout for a sequence diagram.
pub fn scene(d: &SequenceDiagram) -> SeqScene {
    let n = d.participants.len();
    if n == 0 {
        return SeqScene {
            width: 2.0 * MARGIN,
            height: 2.0 * MARGIN,
            ..Default::default()
        };
    }

    // --- columns: box sizes, then adjacent center-to-center gaps
    // widened until every label that spans them fits. ---
    let bw: Vec<f64> = d
        .participants
        .iter()
        .map(|p| (text_width(&p.label) + 28.0).max(MIN_BOX_W))
        .collect();
    let bh: Vec<f64> = d
        .participants
        .iter()
        .map(|p| if p.actor { ACTOR_H } else { BOX_H })
        .collect();
    // Header row height: the tallest slot (an actor's figure + name).
    let head_h = bh.iter().copied().fold(BOX_H, f64::max);
    let mut gap = vec![0.0f64; n.saturating_sub(1)];
    for i in 0..gap.len() {
        gap[i] = bw[i] / 2.0 + bw[i + 1] / 2.0 + GAP_MIN;
    }
    let mut msg_no = 0usize;
    // Open activation bars per participant, counted the way the row
    // loop below stacks them, so a self-message knows how far right
    // of its lifeline its loop starts.
    let mut depth = vec![0usize; n];
    for item in &d.items {
        match item {
            SeqItem::Message {
                from,
                to,
                text,
                activate,
                deactivate,
                ..
            } => {
                msg_no += 1;
                let w = label_width(d.autonumber.then_some(msg_no), text);
                if from != to {
                    // Spread the requirement over every gap the
                    // message spans, so the total span fits the label.
                    let (a, b) = (*from.min(to), *from.max(to));
                    let per = (w + 20.0) / (b - a) as f64;
                    for k in a..b {
                        gap[k] = gap[k].max(per);
                    }
                } else if *from + 1 < n {
                    // Self-message label sits right of the loop. On
                    // its row the next column is only a lifeline,
                    // maybe under an activation bar (its box is up in
                    // the header, or down in the footer), so the label
                    // clears that bar's left edge by 8px — not the
                    // box, which used to inflate this one gap.
                    let start = bar_reach(depth[*from]);
                    gap[*from] = gap[*from].max(start + SELF_W + 8.0 + w + 8.0 + ACT_W / 2.0);
                }
                if *activate {
                    depth[*to] += 1;
                }
                if *deactivate {
                    depth[*from] = depth[*from].saturating_sub(1);
                }
            }
            SeqItem::Activate(p) => depth[*p] += 1,
            SeqItem::Deactivate(p) => depth[*p] = depth[*p].saturating_sub(1),
            SeqItem::Note { side, text } => {
                let w = text_width(text) + 2.0 * NOTE_PAD;
                match side {
                    NoteSide::Over(a, Some(b)) if a != b => {
                        let (a, b) = (*a.min(b), *a.max(b));
                        let per = (w - 2.0 * NOTE_PAD) / (b - a) as f64;
                        for k in a..b {
                            gap[k] = gap[k].max(per);
                        }
                    }
                    NoteSide::Over(a, _) => {
                        if *a > 0 {
                            gap[*a - 1] = gap[*a - 1].max(w / 2.0 + 8.0);
                        }
                        if *a + 1 < n {
                            gap[*a] = gap[*a].max(w / 2.0 + 8.0);
                        }
                    }
                    NoteSide::LeftOf(a) => {
                        if *a > 0 {
                            gap[*a - 1] = gap[*a - 1].max(w + 16.0);
                        }
                    }
                    NoteSide::RightOf(a) => {
                        if *a + 1 < n {
                            gap[*a] = gap[*a].max(w + 16.0);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let mut x = vec![0.0f64; n];
    x[0] = MARGIN + bw[0] / 2.0;
    for i in 1..n {
        x[i] = x[i - 1] + gap[i - 1];
    }
    // Horizontal extent of column p's box.
    let col = |p: usize| (x[p] - bw[p] / 2.0, x[p] + bw[p] / 2.0);

    let top = MARGIN;

    // --- rows, top-down in source order ---
    let mut sc = SeqScene::default();
    let mut cur = top + head_h + 10.0;
    let mut msg_no = 0usize;
    // Per-participant stack of open activation bar tops.
    let mut act: Vec<Vec<f64>> = vec![Vec::new(); n];
    // Stack of open frame indices into sc.frames, and per-frame
    // bookkeeping (index-parallel with sc.frames) for `place_frames`.
    let mut fstack: Vec<usize> = Vec::new();
    let mut finfo: Vec<FrameInfo> = Vec::new();
    // The message on the row just above: (index into sc.messages, its
    // y). An explicit `activate` / `deactivate` belongs to it, as in
    // mermaid — the bar starts / ends level with it. Any other row
    // (note, frame edge) clears it.
    let mut last_msg: Option<(usize, f64)> = None;

    for item in &d.items {
        match item {
            SeqItem::Message {
                from,
                to,
                text,
                dashed,
                head,
                activate,
                deactivate,
            } => {
                msg_no += 1;
                let number = d.autonumber.then_some(msg_no);
                let (lo, hi);
                if from == to {
                    // Loop beside the lifeline (or its bar), label to
                    // its right.
                    let y0 = cur + 10.0;
                    let y1 = y0 + SELF_H;
                    let sx = bar_side(&act, &x, *from, f64::INFINITY);
                    if *activate {
                        act[*to].push(y1);
                    }
                    let ex = bar_side(&act, &x, *to, f64::INFINITY);
                    let label_x = sx + SELF_W + 8.0;
                    sc.messages.push(MsgLine {
                        from: *from,
                        to: *to,
                        points: vec![(sx, y0), (sx + SELF_W, y0), (sx + SELF_W, y1), (ex, y1)],
                        dashed: *dashed,
                        head: *head,
                        number,
                        text: text.clone(),
                        label_pos: (label_x, (y0 + y1) / 2.0),
                        label_centered: false,
                    });
                    if *deactivate {
                        end_bar(&mut sc, &mut act, *from, y1, &x);
                    }
                    last_msg = Some((sc.messages.len() - 1, y1));
                    lo = col(*from).0;
                    hi = col(*from).1.max(label_x + label_width(number, text));
                    cur += SELF_H + 24.0;
                } else {
                    let ly = cur + 22.0;
                    if *activate {
                        act[*to].push(ly);
                    }
                    // Ends on the bars' facing edges: after `+` opened
                    // the target's bar, before `-` closes the sender's.
                    let x0 = bar_side(&act, &x, *from, x[*to]);
                    let x1 = bar_side(&act, &x, *to, x[*from]);
                    sc.messages.push(MsgLine {
                        from: *from,
                        to: *to,
                        points: vec![(x0, ly), (x1, ly)],
                        dashed: *dashed,
                        head: *head,
                        number,
                        text: text.clone(),
                        label_pos: ((x0 + x1) / 2.0, ly - 10.0),
                        label_centered: true,
                    });
                    if *deactivate {
                        end_bar(&mut sc, &mut act, *from, ly, &x);
                    }
                    last_msg = Some((sc.messages.len() - 1, ly));
                    let half = label_width(number, text) / 2.0;
                    let (a, b) = (col(*from), col(*to));
                    lo = a.0.min(b.0).min((x0 + x1) / 2.0 - half);
                    hi = a.1.max(b.1).max((x0 + x1) / 2.0 + half);
                    cur += ROW_H;
                }
                note_content(&mut finfo, &fstack, lo, hi);
            }
            SeqItem::Note { side, text } => {
                let w = text_width(text) + 2.0 * NOTE_PAD;
                let (nx, nw, anchors) = match side {
                    NoteSide::Over(a, Some(b)) if a != b => {
                        let (lo, hi) = (x[*a].min(x[*b]), x[*a].max(x[*b]));
                        let nw = (hi - lo + 2.0 * NOTE_PAD).max(w);
                        ((lo + hi) / 2.0 - nw / 2.0, nw, (*a, *b))
                    }
                    NoteSide::Over(a, _) => (x[*a] - w / 2.0, w, (*a, *a)),
                    NoteSide::LeftOf(a) => (x[*a] - 8.0 - w, w, (*a, *a)),
                    NoteSide::RightOf(a) => (x[*a] + 8.0, w, (*a, *a)),
                };
                sc.notes.push(NoteBox {
                    x: nx,
                    y: cur + 4.0,
                    w: nw,
                    h: NOTE_H,
                    text: text.clone(),
                });
                let (a, b) = (col(anchors.0), col(anchors.1));
                let lo = nx.min(a.0).min(b.0);
                let hi = (nx + nw).max(a.1).max(b.1);
                note_content(&mut finfo, &fstack, lo, hi);
                last_msg = None;
                cur += NOTE_H + 10.0;
            }
            SeqItem::Activate(p) => {
                let y = last_msg.map_or(cur + 2.0, |(_, y)| y);
                act[*p].push(y);
                if let Some((mi, _)) = last_msg {
                    snap_to_bar(&mut sc.messages[mi], *p, &act, &x);
                }
            }
            SeqItem::Deactivate(p) => {
                let y = last_msg.map_or(cur + 2.0, |(_, y)| y);
                end_bar(&mut sc, &mut act, *p, y, &x);
            }
            SeqItem::FrameStart { kind, label } => {
                finfo.push(FrameInfo {
                    depth: fstack.len(),
                    parent: fstack.last().copied(),
                    content: None,
                });
                fstack.push(sc.frames.len());
                sc.frames.push(FrameBox {
                    kind: *kind,
                    label: label.clone(),
                    x: 0.0, // x / w set by place_frames
                    y: cur + 6.0,
                    w: 0.0,
                    h: 0.0, // patched at FrameEnd
                    dividers: Vec::new(),
                });
                last_msg = None;
                cur += 30.0; // room for the label chip row
            }
            SeqItem::FrameElse { label } => {
                // Same rhythm as the frame's top edge: the divider
                // where a frame border would go, its label in the
                // same left column one chip-row below it.
                if let Some(&fi) = fstack.last() {
                    sc.frames[fi].dividers.push((cur + 6.0, label.clone()));
                }
                last_msg = None;
                cur += 30.0;
            }
            SeqItem::FrameEnd => {
                if let Some(fi) = fstack.pop() {
                    sc.frames[fi].h = (cur + 6.0) - sc.frames[fi].y;
                }
                last_msg = None;
                cur += 16.0;
            }
        }
    }
    let bottom = cur + 8.0;
    // Unclosed frames / activations (possible on hand-built models;
    // the parser validates real input) extend to the bottom.
    while let Some(fi) = fstack.pop() {
        sc.frames[fi].h = bottom - sc.frames[fi].y;
    }
    for p in 0..n {
        while !act[p].is_empty() {
            end_bar(&mut sc, &mut act, p, bottom, &x);
        }
    }
    place_frames(&mut sc, &finfo, &x, &bw, d.scoped_frames);

    for (i, p) in d.participants.iter().enumerate() {
        let b = PartBox {
            label: p.label.clone(),
            actor: p.actor,
            x: x[i] - bw[i] / 2.0,
            y: top,
            w: bw[i],
            h: bh[i],
        };
        if d.mirror_actors {
            // The mirror's top edge is where the lifeline ends.
            sc.footer.push(PartBox {
                y: bottom,
                ..b.clone()
            });
        }
        sc.boxes.push(b);
        sc.lifelines.push(Lifeline {
            x: x[i],
            y0: top + bh[i],
            y1: bottom,
        });
    }

    // --- normalise: shift right so nothing starts left of the
    // margin, and size the canvas to the rightmost text/box. ---
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for b in &sc.boxes {
        lo = lo.min(b.x);
        hi = hi.max(b.x + b.w);
    }
    for nb in &sc.notes {
        lo = lo.min(nb.x);
        hi = hi.max(nb.x + nb.w);
    }
    for f in &sc.frames {
        lo = lo.min(f.x);
        hi = hi.max(f.x + f.w);
        // Labels stay inside their frame by construction
        // (place_frames); kept as a backstop for the canvas.
        let lx = frame_label_x(f);
        hi = hi.max(lx + text_width(&bracketed(&f.label)));
        for (_, dl) in &f.dividers {
            hi = hi.max(lx + text_width(&bracketed(dl)));
        }
    }
    for m in &sc.messages {
        let w = label_width(m.number, &m.text);
        if m.label_centered {
            lo = lo.min(m.label_pos.0 - w / 2.0);
            hi = hi.max(m.label_pos.0 + w / 2.0);
        } else {
            hi = hi.max(m.label_pos.0 + w);
        }
        for &(px, _) in &m.points {
            lo = lo.min(px);
            hi = hi.max(px);
        }
    }
    let dx = MARGIN - lo;
    if dx.abs() > 1e-9 {
        for b in sc.boxes.iter_mut().chain(sc.footer.iter_mut()) {
            b.x += dx;
        }
        for l in &mut sc.lifelines {
            l.x += dx;
        }
        for m in &mut sc.messages {
            for p in &mut m.points {
                p.0 += dx;
            }
            m.label_pos.0 += dx;
        }
        for nb in &mut sc.notes {
            nb.x += dx;
        }
        for a in &mut sc.activations {
            a.x += dx;
        }
        for f in &mut sc.frames {
            f.x += dx;
        }
    }
    sc.width = hi + dx + MARGIN;
    let foot_h = if d.mirror_actors { head_h } else { 0.0 };
    sc.height = bottom + foot_h + MARGIN;
    sc
}

/// `[label]` as mermaid prints frame/divider labels; empty stays empty.
fn bracketed(label: &str) -> String {
    if label.is_empty() {
        String::new()
    } else {
        format!("[{label}]")
    }
}

/// `stroke-dashoffset` for a dashed message line of drawn length
/// `len`, so the [`MSG_DASH`] pattern puts ink at BOTH ends: with the
/// pattern always starting at the source, the target end landed in a
/// gap whenever the length came out wrong, and a headless `-->` then
/// stopped visibly short of its lifeline. The offset splits the
/// leftover evenly, so both ends get the same partial dash.
fn dash_offset(len: f64) -> f64 {
    let (on, off) = MSG_DASH;
    let r = len.rem_euclid(on + off);
    if r <= on {
        (on - r) / 2.0
    } else {
        (2.0 * on + off - r) / 2.0
    }
}

/// Serialise a sequence scene to SVG.
pub fn to_svg(sc: &SeqScene) -> String {
    to_svg_with(sc, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(sc: &SeqScene, opts: &SvgOptions) -> String {
    to_svg_a11y_with(sc, &A11y::default(), opts)
}

/// White card behind a frame / divider label (left-aligned at `x`,
/// vertically centered on `cy`), so the lifelines it crosses don't
/// run through its glyphs. A plain `<rect>` rather than a
/// `paint-order` text halo, which older SVG renderers ignore. Painted
/// *under* the activation bars (the label glyphs go over them): a
/// card on top cut a white gap into any bar it crossed, which read as
/// two separate activations.
fn label_backing(s: &mut String, x: f64, cy: f64, text: &str) {
    s.push_str(&format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"16\" fill=\"#ffffff\"/>\n",
        x - 2.0,
        cy - 8.0,
        text_width(text) + 4.0
    ));
}

/// [`to_svg_with`] plus mermaid's `accTitle` / `accDescr` (see [`A11y`]).
pub(crate) fn to_svg_a11y_with(sc: &SeqScene, a11y: &A11y, opts: &SvgOptions) -> String {
    let (title, desc) = a11y.resolve("Sequence diagram");
    let mut s = String::new();
    svg_open_a11y(&mut s, sc.width, sc.height, 13, &title, desc, opts);

    // Frame borders first (background)…
    for f in &sc.frames {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"1.2\"/>\n",
            f.x, f.y, f.w, f.h, GUIDE_COLOR
        ));
    }
    // …then lifelines and the `else` / `and` dividers — as heavy as
    // the frame border they belong to, so they read as part of the
    // frame rather than as one more lifeline-weight guide…
    for l in &sc.lifelines {
        s.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" \
             stroke-dasharray=\"4 4\"/>\n",
            l.x, l.y0, l.x, l.y1, GUIDE_COLOR
        ));
    }
    for f in &sc.frames {
        for (dy, _) in &f.dividers {
            s.push_str(&format!(
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" \
                 stroke-width=\"1.2\" stroke-dasharray=\"5 4\"/>\n",
                f.x,
                dy,
                f.x + f.w,
                dy,
                GUIDE_COLOR
            ));
        }
    }
    // …then the label backgrounds: each frame's keyword chip and the
    // white cards under its branch labels…
    for f in &sc.frames {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"18\" \
             fill=\"#eef1fb\" stroke=\"{}\"/>\n",
            f.x,
            f.y,
            chip_w(f.kind),
            GUIDE_COLOR
        ));
        let lx = frame_label_x(f);
        for (cy, text) in frame_label_rows(f) {
            label_backing(&mut s, lx, cy, &text);
        }
    }
    // …activation bars over all of it (neither a divider nor a label
    // background ever cuts a bar into pieces)…
    for a in &sc.activations {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
             fill=\"#ffffff\" stroke=\"{}\" stroke-width=\"1.4\"/>\n",
            a.x - ACT_W / 2.0,
            a.y0,
            ACT_W,
            a.y1 - a.y0,
            crate::style::accent(a.participant)
        ));
    }
    // …then the chip keywords and branch labels over the bars. All
    // conditions of a frame share one left-aligned column beside the
    // chip.
    for f in &sc.frames {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" fill=\"{}\">{}</text>\n",
            f.x + chip_w(f.kind) / 2.0,
            f.y + 9.0,
            TEXT_COLOR,
            escape(f.kind.keyword())
        ));
        let lx = frame_label_x(f);
        for (cy, text) in frame_label_rows(f) {
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" fill=\"{}\">{}</text>\n",
                lx,
                cy,
                TEXT_COLOR,
                escape(&text)
            ));
        }
    }
    for nb in &sc.notes {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"1.2\"/>\n",
            nb.x, nb.y, nb.w, nb.h, NOTE_STYLE.fill, NOTE_STYLE.stroke
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             fill=\"{}\">{}</text>\n",
            nb.x + nb.w / 2.0,
            nb.y + nb.h / 2.0,
            TEXT_COLOR,
            escape(&nb.text)
        ));
    }
    for m in &sc.messages {
        let mut path = String::new();
        let mut len = 0.0;
        for (i, (px, py)) in m.points.iter().enumerate() {
            let cmd = if i == 0 { 'M' } else { 'L' };
            path.push_str(&format!("{} {:.1} {:.1} ", cmd, px, py));
            if i > 0 {
                let (qx, qy) = m.points[i - 1];
                len += ((px - qx).powi(2) + (py - qy).powi(2)).sqrt();
            }
        }
        let dash = if m.dashed {
            let o = dash_offset(len);
            // `{}` prints whole f64s bare: "6 4".
            let mut a = format!(" stroke-dasharray=\"{} {}\"", MSG_DASH.0, MSG_DASH.1);
            if o > 0.05 {
                a.push_str(&format!(" stroke-dashoffset=\"{o:.1}\""));
            }
            a
        } else {
            String::new()
        };
        s.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.6\"{}/>\n",
            path.trim_end(),
            EDGE_COLOR,
            dash
        ));
        let np = m.points.len();
        write_head(&mut s, &head(m.points[np - 1], m.points[np - 2], m.head));
        if m.text.is_empty() && m.number.is_none() {
            continue;
        }
        let anchor = if m.label_centered {
            " text-anchor=\"middle\""
        } else {
            ""
        };
        match m.number {
            Some(k) => s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\"{} fill=\"{}\">\
                 <tspan font-weight=\"bold\">{}. </tspan>{}</text>\n",
                m.label_pos.0,
                m.label_pos.1,
                anchor,
                TEXT_COLOR,
                k,
                escape(&m.text)
            )),
            None => s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\"{} fill=\"{}\">{}</text>\n",
                m.label_pos.0,
                m.label_pos.1,
                anchor,
                TEXT_COLOR,
                escape(&m.text)
            )),
        }
    }
    // Participant boxes last (crisp over the lifeline ends), header
    // row then the footer mirror. Participants get a filled box, actors
    // a stick figure with their name below. Text keeps ≥ 4.5:1 either
    // way: an actor's name is its accent darkened as needed for the
    // white canvas, a participant's is white or dark ink by contrast
    // with its fill.
    let n = sc.boxes.len();
    for (k, b) in sc.boxes.iter().chain(&sc.footer).enumerate() {
        let i = if n == 0 { k } else { k % n };
        let accent = crate::style::accent(i);
        if b.actor {
            write_actor(&mut s, b, accent, crate::style::accent_ink(i));
            continue;
        }
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"1.6\"/>\n",
            b.x, b.y, b.w, b.h, accent, accent
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" fill=\"{}\">{}</text>\n",
            b.x + b.w / 2.0,
            b.y + b.h / 2.0,
            crate::style::text_on(accent),
            escape(&b.label)
        ));
    }
    s.push_str("</svg>\n");
    s
}

/// An actor as its UML stick figure ([`actor_figure`]) in its accent,
/// name below in `ink`.
fn write_actor(s: &mut String, b: &PartBox, accent: &str, ink: &str) {
    let fig = actor_figure(b);
    s.push_str(&format!(
        "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"#ffffff\" stroke=\"{}\" \
         stroke-width=\"1.6\"/>\n",
        fig.head.0, fig.head.1, fig.head_r, accent
    ));
    let d = fig
        .segments
        .iter()
        .map(|[a, b]| format!("M {:.1} {:.1} L {:.1} {:.1}", a.0, a.1, b.0, b.1))
        .collect::<Vec<_>>()
        .join(" ");
    s.push_str(&format!(
        "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.6\" \
         stroke-linecap=\"round\"/>\n",
        d, accent
    ));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
         font-weight=\"bold\" fill=\"{}\">{}</text>\n",
        fig.label_pos.0,
        fig.label_pos.1,
        ink,
        escape(&b.label)
    ));
}

fn write_head(s: &mut String, h: &Head) {
    if !h.polygon.is_empty() {
        let pts = h
            .polygon
            .iter()
            .map(|(x, y)| format!("{:.1},{:.1}", x, y))
            .collect::<Vec<_>>()
            .join(" ");
        s.push_str(&format!(
            "<polygon points=\"{}\" fill=\"{}\"/>\n",
            pts, EDGE_COLOR
        ));
    }
    for [a, b] in &h.segments {
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} L {:.1} {:.1}\" fill=\"none\" stroke=\"{}\" \
             stroke-width=\"1.6\"/>\n",
            a.0, a.1, b.0, b.1, EDGE_COLOR
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn sd(src: &str) -> SequenceDiagram {
        match parse_document(src).unwrap() {
            Document::Sequence(d) => d,
            other => panic!("expected a sequence diagram, got {:?}", other),
        }
    }

    // ----------------------------- parsing -----------------------------

    #[test]
    fn participants_aliases_and_actors() {
        let d =
            sd("sequenceDiagram\nparticipant A as Alice\nactor B as Bob the User\nparticipant C");
        let got: Vec<(&str, &str, bool)> = d
            .participants
            .iter()
            .map(|p| (p.id.as_str(), p.label.as_str(), p.actor))
            .collect();
        assert_eq!(
            got,
            [
                ("A", "Alice", false),
                ("B", "Bob the User", true),
                ("C", "C", false)
            ]
        );
    }

    #[test]
    fn implicit_participants_appear_in_message_order() {
        let d = sd("sequenceDiagram\nA->>B: x\nC->>A: y");
        let ids: Vec<&str> = d.participants.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["A", "B", "C"]);
        // A later declaration updates an implicit participant in place.
        let d = sd("sequenceDiagram\nA->>B: x\nactor B as Bee");
        assert_eq!(d.participants[1].label, "Bee");
        assert!(d.participants[1].actor);
    }

    #[test]
    fn all_eight_arrow_ops_parse() {
        let cases = [
            ("->>", false, SeqHead::Filled),
            ("-->>", true, SeqHead::Filled),
            ("->", false, SeqHead::None),
            ("-->", true, SeqHead::None),
            ("-x", false, SeqHead::Cross),
            ("--x", true, SeqHead::Cross),
            ("-)", false, SeqHead::Async),
            ("--)", true, SeqHead::Async),
        ];
        for (op, want_dashed, want_head) in cases {
            let d = sd(&format!("sequenceDiagram\nA{op}B: hi"));
            let SeqItem::Message {
                text, dashed, head, ..
            } = &d.items[0]
            else {
                panic!("expected a message for op {op}");
            };
            assert_eq!(text, "hi", "op: {op}");
            assert_eq!(*dashed, want_dashed, "op: {op}");
            assert_eq!(*head, want_head, "op: {op}");
        }
    }

    #[test]
    fn activation_shorthand_and_keywords() {
        let d = sd("sequenceDiagram\nA->>+B: q\nB-->>-A: r\nactivate A\ndeactivate A");
        let SeqItem::Message {
            activate,
            deactivate,
            ..
        } = &d.items[0]
        else {
            panic!()
        };
        assert!(*activate && !*deactivate, "`+` activates the target");
        let SeqItem::Message {
            activate,
            deactivate,
            ..
        } = &d.items[1]
        else {
            panic!()
        };
        assert!(!*activate && *deactivate, "`-` deactivates the sender");
        assert!(matches!(d.items[2], SeqItem::Activate(0)));
        assert!(matches!(d.items[3], SeqItem::Deactivate(0)));
    }

    #[test]
    fn unbalanced_deactivate_is_a_line_error() {
        let e = parse_document("sequenceDiagram\nA->>B: hi\ndeactivate B").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(
            e.message.contains("without a matching activate"),
            "{}",
            e.message
        );
        // `-` shorthand deactivates the sender — B was never activated.
        let e = parse_document("sequenceDiagram\nA->>B: hi\nB-->>-A: bye").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.message.contains("not activated"), "{}", e.message);
    }

    #[test]
    fn autonumber_and_all_note_forms() {
        let d = sd(
            "sequenceDiagram\nautonumber\nA->>B: one\nNote over A,B: pair\n\
             note over A: solo\nNote left of A: l\nNOTE RIGHT OF B: r",
        );
        assert!(d.autonumber);
        let sides: Vec<NoteSide> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::Note { side, .. } => Some(*side),
                _ => None,
            })
            .collect();
        assert_eq!(
            sides,
            [
                NoteSide::Over(0, Some(1)),
                NoteSide::Over(0, None),
                NoteSide::LeftOf(0),
                NoteSide::RightOf(1),
            ]
        );
        // Rejecting autonumber arguments keeps the claim honest.
        assert!(parse_document("sequenceDiagram\nautonumber 10").is_err());
    }

    #[test]
    fn trailing_comment_is_stripped() {
        let d = sd("sequenceDiagram\nA->>B: hi %% trailing comment");
        let SeqItem::Message { text, .. } = &d.items[0] else {
            panic!()
        };
        assert_eq!(text, "hi");
    }

    #[test]
    fn note_over_same_participant_collapses_to_single() {
        // Bug hunt: `Note over A,A` used to become a zero-width span
        // Over(0, Some(0)); it must normalise to a single-participant
        // note Over(0, None).
        let d = sd("sequenceDiagram\nparticipant A\nNote over A,A: same");
        assert!(matches!(
            d.items[0],
            SeqItem::Note {
                side: NoteSide::Over(0, None),
                ..
            }
        ));
    }

    #[test]
    fn frames_parse_and_validate() {
        let d = sd(
            "sequenceDiagram\nalt ok\nA->>B: x\nelse bad\nB->>A: y\nend\n\
             par one\nA->>B: p\nand two\nA->>B: q\nend\n\
             loop each\nA->>B: l\nend\nopt maybe\nA->>B: o\nend",
        );
        let kinds: Vec<FrameKind> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::FrameStart { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect();
        assert_eq!(
            kinds,
            [
                FrameKind::Alt,
                FrameKind::Par,
                FrameKind::Loop,
                FrameKind::Opt
            ]
        );
        let elses = d
            .items
            .iter()
            .filter(|i| matches!(i, SeqItem::FrameElse { .. }))
            .count();
        let ends = d
            .items
            .iter()
            .filter(|i| matches!(i, SeqItem::FrameEnd))
            .count();
        assert_eq!((elses, ends), (2, 4));

        let e = parse_document("sequenceDiagram\nend").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.message.contains("without an open"), "{}", e.message);
        let e = parse_document("sequenceDiagram\nloop x\nA->>B: y").unwrap_err();
        assert_eq!(e.line, 2, "unclosed frame reports its opening line");
        assert!(e.message.contains("never closed"), "{}", e.message);
        let e = parse_document("sequenceDiagram\nloop x\nelse y\nend").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.message.contains("'alt'"), "{}", e.message);
        let e = parse_document("sequenceDiagram\nalt x\nand y\nend").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.message.contains("'par'"), "{}", e.message);
    }

    #[test]
    fn unsupported_sequence_elements_error_cleanly() {
        for src in [
            "sequenceDiagram\nrect rgb(0,0,0)",
            "sequenceDiagram\nbox Purple team",
            "sequenceDiagram\ncreate participant D",
            "sequenceDiagram\nbreak overload",
        ] {
            let e = parse_document(src).unwrap_err();
            assert_eq!(e.line, 2, "src: {src}");
            assert!(e.message.contains("not supported yet"), "{}", e.message);
        }
    }

    #[test]
    fn message_errors_have_line_numbers() {
        let e = parse_document("sequenceDiagram\nA->>B hi").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.message.contains("expected ':"), "{}", e.message);
        let e = parse_document("sequenceDiagram\nA==>B: hi").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(
            e.message.contains("unknown message operator"),
            "{}",
            e.message
        );
    }

    // ----------------------------- layout -----------------------------

    #[test]
    fn columns_follow_declaration_order_and_rows_increase() {
        let d = sd("sequenceDiagram\nA->>B: x\nC->>A: a much longer label here\nB-->>C: z");
        let sc = scene(&d);
        assert_eq!(sc.boxes.len(), 3);
        for i in 1..sc.boxes.len() {
            let (a, b) = (&sc.boxes[i - 1], &sc.boxes[i]);
            assert!(a.x + a.w < b.x, "box {i} must sit right of box {}", i - 1);
        }
        let ys: Vec<f64> = sc.messages.iter().map(|m| m.points[0].1).collect();
        for i in 1..ys.len() {
            assert!(ys[i] > ys[i - 1], "message rows must descend: {ys:?}");
        }
        // Lifelines run under the box centers.
        for (b, l) in sc.boxes.iter().zip(&sc.lifelines) {
            assert!((b.x + b.w / 2.0 - l.x).abs() < 1e-6);
            assert!(l.y1 > l.y0);
        }
    }

    #[test]
    fn self_message_stays_right_of_its_lifeline() {
        let d = sd("sequenceDiagram\nA->>A: think about it carefully\nA->>B: go");
        let sc = scene(&d);
        let m = &sc.messages[0];
        let lx = sc.lifelines[0].x;
        assert_eq!(m.points.len(), 4, "self-message is a loop");
        for &(px, _) in &m.points {
            assert!(px >= lx - 1e-6, "loop must not cross left of the lifeline");
        }
        assert!(m.label_pos.0 > lx, "label sits right of the lifeline");
        assert!(!m.label_centered);
        // The head points back into the lifeline at the loop's end.
        assert!((m.points[3].0 - lx).abs() < 1e-6);
        // Label fits the canvas and clears the next lifeline, with
        // room for an activation bar there. (It used to clear the next
        // column's BOX, which only exists up in the header row — that
        // over-reservation inflated the gap; report "uneven column
        // spacing".)
        let w = label_width(m.number, &m.text);
        assert!(m.label_pos.0 + w <= sc.width);
        assert!(m.label_pos.0 + w + 8.0 <= sc.lifelines[1].x - ACT_W / 2.0 + 1e-6);
    }

    #[test]
    fn long_labels_widen_the_gap() {
        let d = sd(
            "sequenceDiagram\nA->>B: an extremely long message label that must fit between lifelines",
        );
        let sc = scene(&d);
        let m = &sc.messages[0];
        let span = (m.points[1].0 - m.points[0].0).abs();
        assert!(
            span >= label_width(m.number, &m.text),
            "span {span} too small for the label"
        );
    }

    #[test]
    fn activation_bars_span_activate_to_deactivate() {
        let d = sd("sequenceDiagram\nA->>+B: q\nB->>B: work\nB-->>-A: r");
        let sc = scene(&d);
        assert_eq!(sc.activations.len(), 1);
        let bar = &sc.activations[0];
        assert_eq!(bar.participant, 1);
        assert!(
            (bar.y0 - sc.messages[0].points[0].1).abs() < 1e-6,
            "starts at the + message"
        );
        assert!(
            (bar.y1 - sc.messages[2].points[0].1).abs() < 1e-6,
            "ends at the - message"
        );
        assert!((bar.x - sc.lifelines[1].x).abs() < 1e-6);
        // An unclosed activation extends to the lifeline bottom.
        let sc = scene(&sd("sequenceDiagram\nA->>+B: q"));
        assert_eq!(sc.activations.len(), 1);
        assert!((sc.activations[0].y1 - sc.lifelines[1].y1).abs() < 1e-6);
    }

    // ----------------------------- SVG -----------------------------

    #[test]
    fn head_glyph_geometry() {
        let tip = (100.0, 50.0);
        let back = (0.0, 50.0);
        assert_eq!(head(tip, back, SeqHead::Filled).polygon.len(), 3);
        assert_eq!(head(tip, back, SeqHead::Cross).segments.len(), 2);
        assert_eq!(head(tip, back, SeqHead::Async).segments.len(), 2);
        assert_eq!(head(tip, back, SeqHead::Open).segments.len(), 2);
        let none = head(tip, back, SeqHead::None);
        assert!(none.polygon.is_empty() && none.segments.is_empty());
        // Filled triangle points at the tip, its base back along the line.
        let poly = head(tip, back, SeqHead::Filled).polygon;
        assert_eq!(poly[0], tip);
        assert!(poly[1].0 < tip.0 && poly[2].0 < tip.0);
    }

    #[test]
    fn svg_has_dashes_heads_and_autonumber_prefix() {
        let d =
            sd("sequenceDiagram\nautonumber\nA-->>B: dashed filled\nA-xB: crossed\nA->B: plain");
        let svg = to_svg(&scene(&d));
        assert!(
            svg.contains("stroke-dasharray=\"6 4\""),
            "dashed message line"
        );
        assert!(svg.contains("<polygon"), "filled head triangle");
        assert!(svg.contains("<tspan font-weight=\"bold\">1. </tspan>dashed filled"));
        assert!(svg.contains("<tspan font-weight=\"bold\">2. </tspan>crossed"));
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        assert!(svg.ends_with("</svg>\n"));
        // The cross adds two bare segments beyond the 3 message lines.
        let paths = svg.lines().filter(|l| l.starts_with("<path d=")).count();
        assert_eq!(paths, 3 + 2, "3 message lines + 2 cross segments");
    }

    fn svg_attr(s: &str, name: &str) -> f64 {
        let pat = format!("{name}=\"");
        let i = s.find(&pat).unwrap() + pat.len();
        s[i..i + s[i..].find('"').unwrap()].parse().unwrap()
    }

    /// Text content of a `<text>` line with inner tags removed
    /// (the bold autonumber `<tspan>`).
    fn text_content(line: &str) -> String {
        let inner = &line[line.find('>').unwrap() + 1..line.rfind("</text>").unwrap()];
        let mut out = String::new();
        let mut in_tag = false;
        for c in inner.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => out.push(c),
                _ => {}
            }
        }
        out
    }

    fn assert_text_contained(svg: &str) {
        let w = svg_attr(svg, "width");
        let h = svg_attr(svg, "height");
        for line in svg.lines().filter(|l| l.starts_with("<text")) {
            let x = svg_attr(line, "x");
            let y = svg_attr(line, "y");
            let inner = text_content(line);
            let tw = crate::layout::text_width(&inner);
            let (lo, hi) = if line.contains("text-anchor=\"middle\"") {
                (x - tw / 2.0, x + tw / 2.0)
            } else {
                (x, x + tw)
            };
            assert!(
                lo >= -1.0 && hi <= w + 1.0,
                "text {inner:?} [{lo:.0}..{hi:.0}] outside width {w}"
            );
            assert!((0.0..=h).contains(&y), "text y {y} outside height {h}");
        }
    }

    #[test]
    fn notes_and_edge_columns_stay_inside_the_canvas() {
        // A wide note LEFT of the first participant and RIGHT of the
        // last exercise the shift-right normalisation on both sides.
        let d = sd(
            "sequenceDiagram\nA->>B: hi\nNote left of A: a rather wide note text\n\
             Note right of B: another wide note on the right side\nNote over A,B: spanning both",
        );
        let svg = to_svg(&scene(&d));
        assert!(!svg.contains("NaN"));
        assert_text_contained(&svg);
        // Note card colors present.
        assert!(svg.contains(NOTE_STYLE.fill) && svg.contains(NOTE_STYLE.stroke));
    }

    /// End-to-end showcase guarding the whole feature set as one
    /// interacting whole: participants/actors with aliases, all
    /// eight arrows, autonumber, every note form, explicit and
    /// shorthand activations, and loop/opt/alt/par frames.
    #[test]
    fn sequence_showcase_parses_and_renders() {
        let d = sd(include_str!("../examples/sequence.mmd"));
        assert!(d.autonumber);
        let ids: Vec<&str> = d.participants.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["U", "FE", "API", "DB"]);
        assert!(d.participants[0].actor, "U is an actor");
        assert_eq!(d.participants[1].label, "Frontend");

        // All 8 (dashed, head) arrow combinations are exercised.
        let mut combos: Vec<(bool, SeqHead)> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::Message { dashed, head, .. } => Some((*dashed, *head)),
                _ => None,
            })
            .collect();
        combos.sort_by_key(|(d, h)| (*d, *h as usize));
        combos.dedup();
        assert_eq!(combos.len(), 8, "all 8 arrow variants: {combos:?}");
        // A self-message is present.
        assert!(d
            .items
            .iter()
            .any(|i| matches!(i, SeqItem::Message { from, to, .. } if from == to)));
        // Explicit + shorthand activations.
        assert!(d.items.iter().any(|i| matches!(i, SeqItem::Activate(_))));
        assert!(d.items.iter().any(|i| matches!(i, SeqItem::Deactivate(_))));
        assert!(d
            .items
            .iter()
            .any(|i| matches!(i, SeqItem::Message { activate: true, .. })));
        // Every note form.
        let sides: Vec<NoteSide> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::Note { side, .. } => Some(*side),
                _ => None,
            })
            .collect();
        assert!(sides
            .iter()
            .any(|s| matches!(s, NoteSide::Over(_, Some(_)))));
        assert!(sides.iter().any(|s| matches!(s, NoteSide::Over(_, None))));
        assert!(sides.iter().any(|s| matches!(s, NoteSide::LeftOf(_))));
        assert!(sides.iter().any(|s| matches!(s, NoteSide::RightOf(_))));
        // All four frame kinds, with else + and dividers.
        let kinds: Vec<FrameKind> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::FrameStart { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect();
        for want in [
            FrameKind::Loop,
            FrameKind::Opt,
            FrameKind::Alt,
            FrameKind::Par,
        ] {
            assert!(kinds.contains(&want), "missing frame {want:?}");
        }
        assert_eq!(
            d.items
                .iter()
                .filter(|i| matches!(i, SeqItem::FrameElse { .. }))
                .count(),
            2
        );

        // Scene sanity: bars exist, frames closed with real heights.
        let sc = scene(&d);
        assert_eq!(sc.frames.len(), 4);
        assert!(sc.frames.iter().all(|f| f.h > 0.0 && f.w > 0.0));
        assert!(sc.activations.len() >= 2);
        assert!(!sc.messages.is_empty() && sc.messages.len() == 14);

        // Render: finite canvas, numbered labels, dashes, frames'
        // chips, note colors, no NaN, and every text inside.
        let svg = to_svg(&sc);
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        assert!(svg.contains(">loop</text>") && svg.contains(">alt</text>"));
        assert!(svg.contains("[expired]"), "else divider label");
        assert!(svg.contains("<tspan font-weight=\"bold\">14. </tspan>"));
        assert!(svg.contains("stroke-dasharray=\"6 4\""));
        assert!(svg.contains("<polygon"), "filled arrowheads");
        assert_text_contained(&svg);
        // render_svg dispatches here too.
        let via_lib = crate::render_svg(include_str!("../examples/sequence.mmd")).unwrap();
        assert_eq!(via_lib, svg);
    }

    #[test]
    fn empty_scene_is_finite() {
        let sc = scene(&SequenceDiagram::default());
        assert!(sc.width > 0.0 && sc.height > 0.0);
        assert!(to_svg(&sc).ends_with("</svg>\n"));
    }

    /// Report Q5: participant boxes hard-coded white text on their
    /// accent fill, and actor boxes wrote the raw accent on white —
    /// both below WCAG AA for most accents.
    #[test]
    fn participant_and_actor_labels_read_on_every_accent() {
        let n = crate::style::ACCENTS.len();
        // Every accent once as a filled participant, and once as an
        // outlined actor (index + n wraps to the same accent).
        let mut src = String::from("sequenceDiagram\n");
        for i in 0..2 * n {
            let kind = if i < n { "participant" } else { "actor" };
            src.push_str(&format!("{kind} P{i}\n"));
        }
        src.push_str("P0->>P1: hi\n");
        let svg = crate::render_svg(&src).unwrap();
        for i in 0..2 * n {
            let label = format!("P{i}");
            if i < n {
                let accent = crate::style::accent(i);
                let ink = crate::style::assert_text_reads(&svg, &label, accent);
                assert_eq!(ink, crate::style::text_on(accent), "{label}");
            } else {
                let ink = crate::style::assert_text_reads(&svg, &label, "#ffffff");
                assert_eq!(ink, crate::style::accent_ink(i), "{label}");
            }
        }
    }

    // ------------------- report Q11 / M12 regressions -------------------

    /// The y each message row sits at.
    fn row_y(m: &MsgLine) -> f64 {
        m.points[0].1
    }

    /// Report Q11: an explicit `activate` / `deactivate` started and
    /// ended its bar one partial row (14px) late — `cur` had already
    /// moved past the message it belongs to. Both forms now agree.
    #[test]
    fn explicit_activate_bar_starts_and_ends_at_its_messages() {
        let explicit = scene(&sd("sequenceDiagram\nA->>B: go\nactivate B\nB->>C: ask\n\
             C-->>B: answer\nB-->>A: back\ndeactivate B"));
        let bar = &explicit.activations[0];
        assert_eq!(bar.participant, 1);
        assert!(
            (bar.y0 - row_y(&explicit.messages[0])).abs() < 1e-6,
            "y0 = `go` row"
        );
        assert!(
            (bar.y1 - row_y(&explicit.messages[3])).abs() < 1e-6,
            "y1 = `back` row"
        );
        // Same geometry as the `+` / `-` shorthand, message ends included
        // (the explicit form re-aims `go` at the bar opened after it).
        let short = scene(&sd("sequenceDiagram\nA->>+B: go\nB->>C: ask\n\
             C-->>B: answer\nB-->>-A: back"));
        let sb = &short.activations[0];
        assert_eq!((bar.y0, bar.y1, bar.x), (sb.y0, sb.y1, sb.x));
        for (a, b) in explicit.messages.iter().zip(&short.messages) {
            assert_eq!(a.points, b.points, "{}", a.text);
            assert_eq!(a.label_pos, b.label_pos, "{}", a.text);
        }
        // With no message right above (a note in between), the bar
        // starts below that note instead.
        let sc = scene(&sd(
            "sequenceDiagram\nA->>B: go\nNote over B: busy\nactivate B\n\
             B-->>A: done\ndeactivate B",
        ));
        let (bar, note) = (&sc.activations[0], &sc.notes[0]);
        assert!(bar.y0 >= note.y + note.h, "bar {} under the note", bar.y0);
        assert!((bar.y1 - row_y(&sc.messages[1])).abs() < 1e-6);
        // `activate` straight after `deactivate`-free message pairs
        // still shows a minimal bar.
        let sc = scene(&sd("sequenceDiagram\nA->>B: go\nactivate B\ndeactivate B"));
        assert!(sc.activations[0].y1 - sc.activations[0].y0 >= 6.0 - 1e-9);
    }

    /// Report M12: every endpoint sat on the lifeline centre, so heads
    /// ran 4px into the activation bar and lines left from its middle.
    #[test]
    fn message_ends_stop_at_activation_bar_edges() {
        let sc = scene(&sd(
            "sequenceDiagram\nA->>+B: q\nB->>C: fwd\nC-->>B: back\nB-->>-A: r\n\
             A->>B: after",
        ));
        let bar = &sc.activations[0];
        let (l, r) = (bar.x - ACT_W / 2.0, bar.x + ACT_W / 2.0);
        let m = &sc.messages;
        assert_eq!(
            m[0].points[1].0, l,
            "incoming from the left: bar's left edge"
        );
        assert_eq!(m[1].points[0].0, r, "outgoing rightward: bar's right edge");
        assert_eq!(
            m[2].points[1].0, r,
            "incoming from the right: bar's right edge"
        );
        assert_eq!(m[3].points[0].0, l, "the `-` message leaves the left edge");
        assert_eq!(m[4].points[1].0, sc.lifelines[1].x, "no bar: the lifeline");
        // The drawn head tip is that edge.
        let svg = to_svg(&sc);
        let tip = format!("<polygon points=\"{:.1},{:.1} ", l, row_y(&m[0]));
        assert!(svg.contains(&tip), "head tip at the bar edge: {tip}");

        // Nested bars: the stack's left edge is the outer bar's, its
        // right edge the inner bar's; a self-message loops from the
        // outer bar's right edge back to the new inner bar.
        let sc = scene(&sd(
            "sequenceDiagram\nA->>+B: q\nB->>+B: nest\nB->>C: fwd\n\
             B-->>-A: r1\nB-->>-A: r2",
        ));
        let x = sc.lifelines[1].x;
        let m = &sc.messages;
        assert_eq!(
            m[1].points[0].0,
            x + ACT_W / 2.0,
            "self loop leaves the outer bar"
        );
        assert_eq!(
            m[1].points[3].0,
            x + NEST_SHIFT + ACT_W / 2.0,
            "returns to the inner"
        );
        assert_eq!(m[2].points[0].0, x + NEST_SHIFT + ACT_W / 2.0);
        assert_eq!(
            m[3].points[0].0,
            x - ACT_W / 2.0,
            "left side: outer bar's edge"
        );
    }

    /// The acceptance check for M12 over real diagrams: no message end
    /// lies strictly inside an activation bar of its participant that
    /// covers its row.
    #[test]
    fn no_message_end_runs_into_a_bar() {
        for src in [
            include_str!("../examples/sequence.mmd"),
            "sequenceDiagram\nactor U\nU->>+S: request\nS->>+S: validate\nS->>W: job\n\
             activate W\nW-->>S: ok\nS->>S: log\nW->>W: crunch\ndeactivate W\n\
             S-->>-U: done\ndeactivate S",
        ] {
            let sc = scene(&sd(src));
            for msg in &sc.messages {
                let ends = [
                    (msg.from, msg.points[0]),
                    (msg.to, *msg.points.last().unwrap()),
                ];
                for (p, (ex, ey)) in ends {
                    for bar in sc.activations.iter().filter(|a| a.participant == p) {
                        if (bar.y0..=bar.y1).contains(&ey) {
                            assert!(
                                (ex - bar.x).abs() >= ACT_W / 2.0 - 1e-6,
                                "{:?} end x={ex} inside bar at {} ({src})",
                                msg.text,
                                bar.x
                            );
                        }
                    }
                }
            }
        }
    }

    /// Every frame whose rows hold `(y, lo, hi)` must contain `lo..hi`.
    fn assert_frames_contain(sc: &SeqScene, y: f64, lo: f64, hi: f64, what: &str) {
        for f in sc.frames.iter().filter(|f| (f.y..=f.y + f.h).contains(&y)) {
            assert!(
                f.x < lo && hi < f.x + f.w,
                "{what} [{lo:.1}..{hi:.1}] sticks out of {:?} frame [{:.1}..{:.1}]",
                f.kind,
                f.x,
                f.x + f.w
            );
        }
    }

    /// Report Q11: a `Note right of` the last / `left of` the first
    /// participant inside a frame hung half outside it (the frame was
    /// sized from the boxes alone). Frames now grow to contain their
    /// notes and self-message labels — and stay aligned full-width
    /// bands while doing it.
    #[test]
    fn notes_inside_frames_stay_inside_them() {
        for src in [
            include_str!("../examples/sequence.mmd"),
            "sequenceDiagram\nparticipant A as Alice\nparticipant B as Bob\nparticipant C as Carol\n\
             A->>+B: start\nalt ok\nB->>C: ask\nNote right of C: a fairly long note on the right\n\
             else fail\nB->>A: err\nend\nB-->>-A: done\nloop outer\nopt inner\n\
             Note left of A: left side note here\nA->>A: think hard about it\nend\nend\n\
             loop last\nC->>C: a self-message label beside the last lifeline\nend",
        ] {
            let sc = scene(&sd(src));
            for nb in &sc.notes {
                assert_frames_contain(&sc, nb.y + nb.h / 2.0, nb.x, nb.x + nb.w, &nb.text);
            }
            for m in sc.messages.iter().filter(|m| !m.label_centered) {
                let hi = m.label_pos.0 + label_width(m.number, &m.text);
                assert_frames_contain(&sc, row_y(m), m.points[0].0, hi, &m.text);
            }
            // Still the default full-width aligned bands: one left and
            // right edge per nesting depth, nested ones inset.
            let depth = |f: &FrameBox| {
                sc.frames
                    .iter()
                    .filter(|o| o.y < f.y && f.y + f.h <= o.y + o.h)
                    .count()
            };
            let outer = sc.frames.iter().find(|f| depth(f) == 0).unwrap();
            for f in &sc.frames {
                let d = depth(f) as f64 * FRAME_INSET;
                assert!((f.x - (outer.x + d)).abs() < 1e-6, "{:?} misaligned", f.kind);
                assert!((f.w - (outer.w - 2.0 * d)).abs() < 1e-6, "{:?} misaligned", f.kind);
            }
        }
    }

    /// A frame's condition labels (first branch and `else` / `and`)
    /// share one left-aligned column beside the chip (the `else` one
    /// used to be centred while the first was left-aligned), and a
    /// frame widens for a label longer than the diagram is wide.
    #[test]
    fn frame_labels_share_a_column_and_fit_their_frame() {
        let sc = scene(&sd("sequenceDiagram\nparticipant A\n\
             alt a very long condition label for a narrow diagram\nA->>A: x\n\
             else another rather long branch label here\nA->>A: y\nend"));
        let f = &sc.frames[0];
        let lx = frame_label_x(f);
        for l in std::iter::once(&f.label).chain(f.dividers.iter().map(|(_, l)| l)) {
            assert!(
                lx + text_width(&bracketed(l)) < f.x + f.w,
                "{l:?} sticks out"
            );
        }
        let svg = to_svg(&sc);
        for label in ["[a very long", "[another rather"] {
            let line = svg.lines().find(|l| l.contains(label)).unwrap();
            assert!(!line.contains("text-anchor"), "left-aligned: {line}");
            assert!(
                (svg_attr(line, "x") - lx).abs() < 0.06,
                "one column: {line}"
            );
        }
    }

    /// Report Q11: the frame and branch labels had nothing behind
    /// them, so the first lifeline's dashes ran through their glyphs.
    /// Each now sits on a white card as wide as its text (painted
    /// earlier, under the activation bars — see
    /// `label_cards_never_cut_an_activation_bar`).
    #[test]
    fn frame_labels_sit_on_white_cards() {
        let svg = crate::render_svg(include_str!("../examples/sequence.mmd")).unwrap();
        let lines: Vec<&str> = svg.lines().collect();
        for label in ["[session valid]", "[expired]", "[cache warmup]", "[audit]"] {
            let i = lines
                .iter()
                .position(|l| l.starts_with("<text") && l.contains(label))
                .unwrap();
            let (tx, ty) = (svg_attr(lines[i], "x"), svg_attr(lines[i], "y"));
            let card = lines[..i]
                .iter()
                .rev()
                .find(|l| {
                    l.contains("height=\"16\" fill=\"#ffffff\"")
                        && (svg_attr(l, "y") + 8.0 - ty).abs() < 1e-6
                })
                .unwrap_or_else(|| panic!("{label}: no card painted before it"));
            let (cx, cw) = (svg_attr(card, "x"), svg_attr(card, "width"));
            assert!(
                cx <= tx && tx + text_width(label) <= cx + cw + 0.1,
                "{label}"
            );
        }
    }

    /// Verifier finding on the Q11 label cards: they were painted
    /// AFTER the activation bars, so a bar running under a frame or
    /// divider label got a 16px white gap and read as two separate
    /// activations. Label backgrounds (cards and keyword chips) now go
    /// under the bars and the label text over them: no background may
    /// overlap a bar painted before it, and no bar may cover label
    /// glyphs painted before it. Covers the default bands (the first
    /// participant's bar under the label column), scoped frames (the
    /// leftmost touched column) and nested chips reaching a bar.
    #[test]
    fn label_cards_never_cut_an_activation_bar() {
        let cases = [
            "sequenceDiagram\nparticipant A\nparticipant B as Backend\nparticipant C as Cache\n\
             B->>+A: call into A\nalt cache hit and value fresh\nA->>C: get\nC-->>A: value\n\
             else cache miss or stale entry\nA->>B: load\nB-->>A: data\nelse\nA->>A: noop\nend\n\
             A-->>-B: result",
            "---\nconfig:\n  sequence:\n    scopedFrames: true\n---\nsequenceDiagram\n\
             participant A as Alice\nparticipant B as Bob\nparticipant C as Carol\n\
             A->>+B: open session\nalt acquire the lock\nB->>C: lock\nC-->>B: granted\n\
             else timeout\nB->>C: retry\nend\nloop poll\nB->>C: status\nend\nB-->>-A: done",
            "sequenceDiagram\nparticipant A\nparticipant B\nB->>+A: go\nloop outer\nopt mid\n\
             loop inner\npar deep\npar deeper\npar deepest\nA->>B: x\nend\nend\nend\nend\nend\nend\n\
             A-->>-B: z",
        ];
        type Rect = (usize, f64, f64, f64, f64);
        let rect = |i: usize, l: &str| -> Rect {
            let (x, y) = (svg_attr(l, "x"), svg_attr(l, "y"));
            (i, x, y, svg_attr(l, "width"), svg_attr(l, "height"))
        };
        let overlap = |a: &Rect, b: &Rect| {
            a.1 < b.1 + b.3 && b.1 < a.1 + a.3 && a.2 < b.2 + b.4 && b.2 < a.2 + a.4
        };
        for src in cases {
            let sc = scene(&sd(src));
            let svg = to_svg(&sc);
            let lines: Vec<&str> = svg.lines().collect();
            let bars: Vec<Rect> = lines
                .iter()
                .enumerate()
                .filter(|(_, l)| {
                    l.starts_with("<rect")
                        && l.contains(&format!("width=\"{ACT_W:.1}\""))
                        && l.contains("stroke-width=\"1.4\"")
                })
                .map(|(i, l)| rect(i, l))
                .collect();
            assert_eq!(bars.len(), sc.activations.len(), "{src}");
            let backgrounds: Vec<Rect> = lines
                .iter()
                .enumerate()
                .filter(|(_, l)| {
                    l.starts_with("<rect")
                        && (l.contains("height=\"16\" fill=\"#ffffff\"")
                            || l.contains("fill=\"#eef1fb\""))
                })
                .map(|(i, l)| rect(i, l))
                .collect();
            let mut crossed = 0;
            for bg in &backgrounds {
                for bar in bars.iter().filter(|b| overlap(b, bg)) {
                    crossed += 1;
                    assert!(
                        bg.0 < bar.0,
                        "{}\npainted over the bar\n{}",
                        lines[bg.0],
                        lines[bar.0]
                    );
                }
            }
            assert!(crossed > 0, "the case must put a label over a bar: {src}");
            // Every frame label / keyword is painted after every bar,
            // so a bar never hides its glyphs.
            let last_bar = bars.iter().map(|b| b.0).max().unwrap();
            for f in &sc.frames {
                let mut texts = vec![format!(">{}</text>", f.kind.keyword())];
                texts.extend(frame_label_rows(f).map(|(_, t)| format!(">{}</text>", escape(&t))));
                for t in texts {
                    let i = lines.iter().position(|l| l.ends_with(&t)).unwrap();
                    assert!(i > last_bar, "{t} painted under a bar");
                }
            }
        }
    }

    /// Report Q11 (layering): dividers were painted over activation
    /// bars, so a bar spanning an `else` got a dashed line across it.
    /// Bars now come after the dividers; dividers are as heavy as the
    /// frame border (1.2), not lifeline-weight.
    #[test]
    fn bars_paint_over_dividers_and_dividers_match_the_frame() {
        let svg = to_svg(&scene(&sd(
            "sequenceDiagram\nA->>+B: start\nalt ok\nB->>A: yes\n\
             else fail\nB->>A: no\nend\nB-->>-A: done",
        )));
        let divider = svg
            .lines()
            .position(|l| l.contains("stroke-dasharray=\"5 4\""))
            .unwrap();
        let bar_stroke = format!(
            "stroke=\"{}\" stroke-width=\"1.4\"",
            crate::style::accent(1)
        );
        let bar = svg.lines().position(|l| l.contains(&bar_stroke)).unwrap();
        assert!(divider < bar, "the bar is painted over the divider");
        let line = svg.lines().nth(divider).unwrap();
        assert!(line.contains("stroke-width=\"1.2\""), "{line}");
    }

    /// Report Q11: notes reused amber `#d99114` — `ACCENTS[2]`, the
    /// third participant's header and bar colour — so a note read as
    /// that participant's. Notes use `style::NOTE_STYLE` now.
    #[test]
    fn note_colour_is_no_participant_accent() {
        let svg = crate::render_svg(include_str!("../examples/sequence.mmd")).unwrap();
        let card = format!(
            "fill=\"{}\" stroke=\"{}\"",
            NOTE_STYLE.fill, NOTE_STYLE.stroke
        );
        assert_eq!(svg.matches(&card).count(), 4, "all four notes");
        for accent in crate::style::ACCENTS {
            assert_ne!(NOTE_STYLE.stroke, *accent);
            assert_ne!(NOTE_STYLE.fill, *accent);
        }
        assert!(!svg.contains("#fcf2da"), "old amber note fill");
    }

    /// Report M12: no participant boxes at the bottom; mermaid repeats
    /// them (`mirrorActors`, on by default), and so do we, unless the
    /// frontmatter turns it off.
    #[test]
    fn participant_boxes_mirror_below_the_lifelines() {
        let sc = scene(&sd("sequenceDiagram\nactor U\nU->>A: hi\nA->>B: there"));
        assert_eq!(sc.footer.len(), sc.boxes.len());
        let last_row = sc.messages.iter().map(row_y).fold(0.0, f64::max);
        for ((h, f), l) in sc.boxes.iter().zip(&sc.footer).zip(&sc.lifelines) {
            assert_eq!((h.x, h.w, h.h), (f.x, f.w, f.h));
            assert_eq!((&h.label, h.actor), (&f.label, f.actor));
            assert_eq!(f.y, l.y1, "the lifeline ends on its footer box");
            assert!(f.y > last_row);
            assert!(
                f.y + f.h + MARGIN <= sc.height + 1e-6,
                "footer inside the canvas"
            );
        }
        let svg = to_svg(&sc);
        assert_eq!(svg.matches(">A</text>").count(), 2, "header + footer");
        assert_eq!(svg.matches(">U</text>").count(), 2);

        let src = "---\nconfig:\n  sequence:\n    mirrorActors: false\n---\nsequenceDiagram\n\
                   A->>B: hi";
        let d = sd(src);
        assert!(!d.mirror_actors);
        let off = scene(&d);
        assert!(off.footer.is_empty());
        let on = scene(&sd("sequenceDiagram\nA->>B: hi"));
        assert!((on.height - off.height - BOX_H).abs() < 1e-6);
        assert_eq!(
            crate::render_svg(src).unwrap().matches(">A</text>").count(),
            1
        );
    }

    /// Report M12: `actor` drew as a pale outlined box shaped like the
    /// participants. It is a UML stick figure with its name below now,
    /// header and footer, inside its `ACTOR_H` slot.
    #[test]
    fn actor_draws_as_a_stick_figure() {
        let sc = scene(&sd(
            "sequenceDiagram\nactor U as Visitor\nparticipant S\nU->>S: hi",
        ));
        let b = &sc.boxes[0];
        assert!(b.actor);
        assert_eq!(b.h, ACTOR_H);
        let fig = actor_figure(b);
        let inside = |(px, py): (f64, f64)| {
            (b.x..=b.x + b.w).contains(&px) && (b.y..=b.y + b.h).contains(&py)
        };
        assert!(inside((fig.head.0, fig.head.1 - fig.head_r)));
        assert!(fig.segments.iter().flatten().all(|&p| inside(p)));
        // The name (≈13px tall, centred on label_pos) sits under the
        // feet and above the lifeline's start.
        let feet = fig
            .segments
            .iter()
            .flatten()
            .map(|p| p.1)
            .fold(0.0, f64::max);
        assert!(fig.label_pos.1 - 6.5 > feet);
        assert!(fig.label_pos.1 + 6.5 <= b.y + b.h);
        assert_eq!(sc.lifelines[0].y0, b.y + b.h);
        // Participants keep their box, top-aligned with the actor, and
        // the first row starts below the taller actor slot.
        assert_eq!((sc.boxes[1].h, sc.boxes[1].y), (BOX_H, b.y));
        assert_eq!(sc.lifelines[1].y0, b.y + BOX_H);
        assert!(row_y(&sc.messages[0]) > b.y + b.h);

        let svg = to_svg(&sc);
        assert_eq!(svg.matches("<circle").count(), 2, "header + footer figure");
        assert!(svg.contains("stroke-linecap=\"round\""));
        let slot = format!("<rect x=\"{:.1}\" y=\"{:.1}\"", b.x, b.y);
        assert!(!svg.contains(&slot), "no box drawn for the actor");
        // Name below the figure, centred, in the readable accent ink.
        let name = format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" fill=\"{}\">Visitor</text>",
            fig.label_pos.0,
            fig.label_pos.1,
            crate::style::accent_ink(0)
        );
        assert!(svg.contains(&name), "{svg}");
    }

    /// Report "uneven column spacing": a self-message reserved half of
    /// the NEXT participant's header box for its label, although on
    /// that row the next column is only a lifeline (maybe under a
    /// bar). It now clears the bar edge by 8px.
    #[test]
    fn self_message_gap_reserves_the_next_lifeline_not_its_box() {
        let sc = scene(&sd(
            "sequenceDiagram\nparticipant A as Alice\nparticipant B as Bob\n\
             participant C as Carol\nA->>A: think hard about it\nB->>C: x",
        ));
        let m = &sc.messages[0];
        let w = label_width(m.number, &m.text);
        let (xa, xb) = (sc.lifelines[0].x, sc.lifelines[1].x);
        assert!(
            (m.label_pos.0 + w + 8.0 + ACT_W / 2.0 - xb).abs() < 1e-6,
            "tight"
        );
        let old_rule = SELF_W + w + 16.0 + sc.boxes[1].w / 2.0;
        assert!(
            xb - xa < old_rule - 30.0,
            "gap {} vs old {old_rule}",
            xb - xa
        );
        // Under an open bar the loop starts at the bar's right edge, and
        // the gap grows by that much so the label still clears C.
        let sc = scene(&sd(
            "sequenceDiagram\nA->>+B: go\nB->>B: think hard about it\n\
             B-->>-A: done\nB->>C: x",
        ));
        let m = &sc.messages[1];
        assert_eq!(m.points[0].0, sc.activations[0].x + ACT_W / 2.0);
        let w = label_width(m.number, &m.text);
        assert!(m.label_pos.0 + w + 8.0 <= sc.lifelines[2].x - ACT_W / 2.0 + 1e-6);
    }

    /// Report "open dashed line short": the dash pattern always
    /// started at the source, so the target end of a headless `-->`
    /// could land in a 4px gap. The offset puts ink at both ends.
    #[test]
    fn dashed_lines_are_inked_at_both_ends() {
        let (on, off) = MSG_DASH;
        let period = on + off;
        let mut len = 0.5;
        while len < 400.0 {
            let o = dash_offset(len);
            let start = o.rem_euclid(period);
            let end = (len + o).rem_euclid(period);
            assert!(
                on - start >= 1.0 - 1e-9,
                "len {len}: start in a gap ({start})"
            );
            assert!(
                end <= on + 1e-9 && end >= 1.0 - 1e-9,
                "len {len}: end at {end}"
            );
            len += 0.37;
        }
        // The headless dashed message of the showcase carries it.
        let sc = scene(&sd(include_str!("../examples/sequence.mmd")));
        let m = sc
            .messages
            .iter()
            .find(|m| m.text == "redirect to login")
            .unwrap();
        let len = (m.points[1].0 - m.points[0].0).abs();
        let svg = to_svg(&sc);
        let o = dash_offset(len);
        if o > 0.05 {
            assert!(svg.contains(&format!("stroke-dashoffset=\"{o:.1}\"")));
        }
    }

    /// `config.sequence.scopedFrames: true` (a flowmaid key; mermaid's
    /// frames are always scoped) sizes each frame to the columns it
    /// touches, nests children inside with an inset, and still fits
    /// labels. Report "frames ignore participant scope", kept as an
    /// option so the aligned full-width bands stay the default.
    #[test]
    fn scoped_frames_cover_only_their_columns() {
        let src = "---\nconfig:\n  sequence:\n    scopedFrames: true\n---\nsequenceDiagram\n\
                   participant A as Alice\nparticipant B as Bob\nparticipant C as Carol\n\
                   participant D as Dave\nA->>B: hello\nloop every minute\nB->>C: ping\n\
                   opt sometimes\nC->>C: think\nNote right of C: hmm\nend\nend\n\
                   alt ok\nA->>B: fine\nelse long condition label that is wider than the frame\n\
                   B->>A: nope\nend\npar alpha\nC->>D: x\nand beta\nD->>C: y\nend";
        let d = sd(src);
        assert!(d.scoped_frames && d.mirror_actors);
        let sc = scene(&d);
        let (lp, lo, alt, par) = (&sc.frames[0], &sc.frames[1], &sc.frames[2], &sc.frames[3]);
        let life = |i: usize| sc.lifelines[i].x;
        assert!(
            lp.x > life(0) && lp.x < sc.boxes[1].x,
            "loop starts at Bob's box"
        );
        assert!(
            lp.x + lp.w > life(2) && lp.x + lp.w < life(3),
            "and ends past Carol"
        );
        assert!(lo.x >= lp.x + FRAME_INSET - 1e-6, "opt nests inside loop");
        assert!(lo.x + lo.w <= lp.x + lp.w - FRAME_INSET + 1e-6);
        assert!(
            alt.x < life(0) && alt.x + alt.w < life(3),
            "alt: Alice and Bob"
        );
        let need = frame_label_x(alt) + text_width(&bracketed(&alt.dividers[0].1));
        assert!(need < alt.x + alt.w, "the long else label fits");
        assert!(par.x > life(1) && par.x < life(2), "par: Carol and Dave");
        for nb in &sc.notes {
            assert_frames_contain(&sc, nb.y + nb.h / 2.0, nb.x, nb.x + nb.w, &nb.text);
        }
        // Off (the default) keeps the aligned full-width bands.
        let mut d = d;
        d.scoped_frames = false;
        let sc = scene(&d);
        assert_eq!(
            (sc.frames[0].x, sc.frames[0].w),
            (sc.frames[2].x, sc.frames[2].w)
        );
        assert_eq!(
            (sc.frames[2].x, sc.frames[2].w),
            (sc.frames[3].x, sc.frames[3].w)
        );
    }

    #[test]
    fn sequence_config_reads_only_its_own_booleans() {
        let d = SequenceDiagram::default();
        assert!(d.mirror_actors && !d.scoped_frames, "defaults");
        // Unknown keys, other sections and non-boolean values are skipped.
        let d = sd(
            "---\nconfig:\n  sankey:\n    mirrorActors: false\n  sequence:\n    \
                    mirrorActors: maybe\n    actorMargin: 50\n    scopedFrames: \"yes\"\n---\n\
                    sequenceDiagram\nA->>B: hi",
        );
        assert!(
            d.mirror_actors,
            "`maybe` keeps the default; sankey's key is not ours"
        );
        assert!(d.scoped_frames, "quoted YAML boolean");
    }
}
