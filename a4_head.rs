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
/// `p` has an open activation bar — the side of the innermost bar
/// that faces `toward` (an x), so heads stop at the bar's edge
/// instead of running into it and lines leave from its edge, not its
/// middle (mermaid does the same).
fn bar_side(act: &[Vec<f64>], x: &[f64], p: usize, toward: f64) -> f64 {
    match act[p].len() {
        0 => x[p],
        k => {
            let c = x[p] + (k - 1) as f64 * NEST_SHIFT;
            if toward > c {
                c + ACT_W / 2.0
            } else {
                c - ACT_W / 2.0
            }
        }
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
                if let Some(&fi) = fstack.last() {
                    sc.frames[fi].dividers.push((cur + 10.0, label.clone()));
                }
                last_msg = None;
                cur += 26.0;
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
/// vertically centered on `cy`), so the lifelines and bars it crosses
/// don't run through its glyphs. A plain `<rect>` rather than a
/// `paint-order` text halo, which older SVG renderers ignore.
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
    // …activation bars over both (a divider never strikes a bar)…
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
    // …then chips and branch labels, each label on a white card so
    // no lifeline runs through it. All conditions of a frame share
    // one left-aligned column beside the chip.
    for f in &sc.frames {
        let kw = f.kind.keyword();
        let cw = chip_w(f.kind);
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"18\" \
             fill=\"#eef1fb\" stroke=\"{}\"/>\n",
            f.x, f.y, cw, GUIDE_COLOR
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" fill=\"{}\">{}</text>\n",
            f.x + cw / 2.0,
            f.y + 9.0,
            TEXT_COLOR,
            escape(kw)
        ));
        let lx = frame_label_x(f);
        let rows = std::iter::once((f.y + 9.0, &f.label))
            .chain(f.dividers.iter().map(|(dy, dl)| (dy + 12.0, dl)));
        for (cy, label) in rows {
            if label.is_empty() {
                continue;
            }
            let text = bracketed(label);
            label_backing(&mut s, lx, cy, &text);
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
            let mut a = format!(
                " stroke-dasharray=\"{} {}\"",
                MSG_DASH.0 as i32, MSG_DASH.1 as i32
            );
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

