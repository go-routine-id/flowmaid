//! Sequence diagram rendering: participant boxes across the top,
//! dashed lifelines below, and one row per statement top-down.
//!
//! Unlike `er` / `class`, nothing here is a graph — the Sugiyama
//! pipeline is skipped entirely in favour of a custom linear
//! layout: column x-positions come from declaration order (gaps
//! widened until every message label and note fits), row
//! y-positions from source order.
//!
//! API shape matches the other diagram modules: [`scene`] computes
//! all geometry a GUI painter needs ([`SeqScene`]), [`to_svg`]
//! serialises any scene, and [`head`] exposes arrowheads as plain
//! polygons/segments so painters draw exactly what SVG exports.

use crate::layout::text_width;
use crate::model::{FrameKind, NoteSide, SeqHead, SeqItem, SequenceDiagram};
use crate::scene::{escape, svg_open, EDGE_COLOR, TEXT_COLOR};

/// Participant header box height.
pub const BOX_H: f64 = 34.0;
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
const FRAME_MARGIN: f64 = 10.0; // frame outset beyond the outer boxes

/// Lifelines + frame borders (lighter than the message lines).
const GUIDE_COLOR: &str = "#aeb6d8";
/// Note card fill/stroke (matches the amber "attention" theme).
const NOTE_FILL: &str = "#fcf2da";
const NOTE_STROKE: &str = "#d99114";

/// One participant header box (top of its column). Top-left corner
/// + size; index-aligned with [`SequenceDiagram::participants`].
#[derive(Debug, Clone)]
pub struct PartBox {
    pub label: String,
    pub actor: bool,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Dashed vertical lifeline below a participant box.
#[derive(Debug, Clone)]
pub struct Lifeline {
    pub x: f64,
    pub y0: f64,
    pub y1: f64,
}

/// One message: a polyline (2 points, or 4 for the self-message
/// loop) with a head glyph at the LAST point.
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
#[derive(Debug, Clone, Default)]
pub struct SeqScene {
    pub width: f64,
    pub height: f64,
    pub boxes: Vec<PartBox>,
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

/// Estimated width of a message label including the bold
/// `autonumber` prefix (small fudge for the bold face).
fn label_width(number: Option<usize>, text: &str) -> f64 {
    match number {
        Some(k) => text_width(&format!("{k}. {text}")) + 3.0,
        None => text_width(text),
    }
}

/// Close the innermost open activation bar of participant `p` at
/// height `y` (bars get a minimum height so `activate` immediately
/// followed by `deactivate` stays visible).
fn end_bar(sc: &mut SeqScene, act: &mut [Vec<f64>], p: usize, y: f64, x: &[f64]) {
    if let Some(y0) = act[p].pop() {
        sc.activations.push(ActBar {
            participant: p,
            x: x[p] + act[p].len() as f64 * 3.0,
            y0,
            y1: y.max(y0 + 6.0),
        });
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
    let mut gap = vec![0.0f64; n.saturating_sub(1)];
    for i in 0..gap.len() {
        gap[i] = bw[i] / 2.0 + bw[i + 1] / 2.0 + GAP_MIN;
    }
    let mut msg_no = 0usize;
    for item in &d.items {
        match item {
            SeqItem::Message { from, to, text, .. } => {
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
                    // Self-message label sits right of the loop —
                    // keep it clear of the next column's box.
                    gap[*from] = gap[*from].max(SELF_W + w + 16.0 + bw[*from + 1] / 2.0);
                }
            }
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

    let top = MARGIN;
    let life_top = top + BOX_H;

    // --- rows, top-down in source order ---
    let mut sc = SeqScene::default();
    let mut cur = life_top + 10.0;
    let mut msg_no = 0usize;
    // Per-participant stack of open activation bar tops.
    let mut act: Vec<Vec<f64>> = vec![Vec::new(); n];
    // Stack of open frame indices into sc.frames.
    let mut fstack: Vec<usize> = Vec::new();
    let frame_l = x[0] - bw[0] / 2.0 - FRAME_MARGIN;
    let frame_r = x[n - 1] + bw[n - 1] / 2.0 + FRAME_MARGIN;

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
                if from == to {
                    // Loop beside the lifeline, label to its right.
                    let cx = x[*from];
                    let y0 = cur + 10.0;
                    let y1 = y0 + SELF_H;
                    if *activate {
                        act[*to].push(y1);
                    }
                    sc.messages.push(MsgLine {
                        from: *from,
                        to: *to,
                        points: vec![(cx, y0), (cx + SELF_W, y0), (cx + SELF_W, y1), (cx, y1)],
                        dashed: *dashed,
                        head: *head,
                        number,
                        text: text.clone(),
                        label_pos: (cx + SELF_W + 8.0, (y0 + y1) / 2.0),
                        label_centered: false,
                    });
                    if *deactivate {
                        end_bar(&mut sc, &mut act, *from, y1, &x);
                    }
                    cur += SELF_H + 24.0;
                } else {
                    let ly = cur + 22.0;
                    let (x0, x1) = (x[*from], x[*to]);
                    if *activate {
                        act[*to].push(ly);
                    }
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
                    cur += ROW_H;
                }
            }
            SeqItem::Note { side, text } => {
                let w = text_width(text) + 2.0 * NOTE_PAD;
                let (nx, nw) = match side {
                    NoteSide::Over(a, Some(b)) if a != b => {
                        let (lo, hi) = (x[*a].min(x[*b]), x[*a].max(x[*b]));
                        let nw = (hi - lo + 2.0 * NOTE_PAD).max(w);
                        ((lo + hi) / 2.0 - nw / 2.0, nw)
                    }
                    NoteSide::Over(a, _) => (x[*a] - w / 2.0, w),
                    NoteSide::LeftOf(a) => (x[*a] - 8.0 - w, w),
                    NoteSide::RightOf(a) => (x[*a] + 8.0, w),
                };
                sc.notes.push(NoteBox {
                    x: nx,
                    y: cur + 4.0,
                    w: nw,
                    h: NOTE_H,
                    text: text.clone(),
                });
                cur += NOTE_H + 10.0;
            }
            SeqItem::Activate(p) => act[*p].push(cur + 2.0),
            SeqItem::Deactivate(p) => end_bar(&mut sc, &mut act, *p, cur + 2.0, &x),
            SeqItem::FrameStart { kind, label } => {
                let depth = fstack.len() as f64;
                fstack.push(sc.frames.len());
                sc.frames.push(FrameBox {
                    kind: *kind,
                    label: label.clone(),
                    x: frame_l + depth * 10.0,
                    y: cur + 6.0,
                    w: (frame_r - frame_l) - 2.0 * depth * 10.0,
                    h: 0.0, // patched at FrameEnd
                    dividers: Vec::new(),
                });
                cur += 30.0; // room for the label chip row
            }
            SeqItem::FrameElse { label } => {
                if let Some(&fi) = fstack.last() {
                    sc.frames[fi].dividers.push((cur + 10.0, label.clone()));
                }
                cur += 26.0;
            }
            SeqItem::FrameEnd => {
                if let Some(fi) = fstack.pop() {
                    sc.frames[fi].h = (cur + 6.0) - sc.frames[fi].y;
                }
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

    for (i, p) in d.participants.iter().enumerate() {
        sc.boxes.push(PartBox {
            label: p.label.clone(),
            actor: p.actor,
            x: x[i] - bw[i] / 2.0,
            y: top,
            w: bw[i],
            h: BOX_H,
        });
        sc.lifelines.push(Lifeline {
            x: x[i],
            y0: life_top,
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
        // Chip + `[label]` text may stick out of a narrow frame.
        let chip_w = text_width(f.kind.keyword()) + 14.0;
        hi = hi.max(f.x + chip_w + 6.0 + text_width(&bracketed(&f.label)));
        for (_, dl) in &f.dividers {
            let half = text_width(&bracketed(dl)) / 2.0;
            lo = lo.min(f.x + f.w / 2.0 - half);
            hi = hi.max(f.x + f.w / 2.0 + half);
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
        for b in &mut sc.boxes {
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
    sc.height = bottom + MARGIN;
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

/// Serialise a sequence scene to SVG.
pub fn to_svg(sc: &SeqScene) -> String {
    let mut s = String::new();
    svg_open(&mut s, sc.width, sc.height, 13);

    // Frame borders first (background)…
    for f in &sc.frames {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"1.2\"/>\n",
            f.x, f.y, f.w, f.h, GUIDE_COLOR
        ));
    }
    // …then lifelines and activation bars…
    for l in &sc.lifelines {
        s.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" \
             stroke-dasharray=\"4 4\"/>\n",
            l.x, l.y0, l.x, l.y1, GUIDE_COLOR
        ));
    }
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
    // …chips and dividers on top of the lifelines they cross.
    for f in &sc.frames {
        let kw = f.kind.keyword();
        let chip_w = text_width(kw) + 14.0;
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"18\" \
             fill=\"#eef1fb\" stroke=\"{}\"/>\n",
            f.x, f.y, chip_w, GUIDE_COLOR
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" fill=\"{}\">{}</text>\n",
            f.x + chip_w / 2.0,
            f.y + 9.0,
            TEXT_COLOR,
            escape(kw)
        ));
        if !f.label.is_empty() {
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" fill=\"{}\">{}</text>\n",
                f.x + chip_w + 6.0,
                f.y + 9.0,
                TEXT_COLOR,
                escape(&bracketed(&f.label))
            ));
        }
        for (dy, dl) in &f.dividers {
            s.push_str(&format!(
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" \
                 stroke-dasharray=\"5 4\"/>\n",
                f.x,
                dy,
                f.x + f.w,
                dy,
                GUIDE_COLOR
            ));
            if !dl.is_empty() {
                s.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                     fill=\"{}\">{}</text>\n",
                    f.x + f.w / 2.0,
                    dy + 12.0,
                    TEXT_COLOR,
                    escape(&bracketed(dl))
                ));
            }
        }
    }
    for nb in &sc.notes {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"1.2\"/>\n",
            nb.x, nb.y, nb.w, nb.h, NOTE_FILL, NOTE_STROKE
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
        for (i, (px, py)) in m.points.iter().enumerate() {
            let cmd = if i == 0 { 'M' } else { 'L' };
            path.push_str(&format!("{} {:.1} {:.1} ", cmd, px, py));
        }
        let dash = if m.dashed {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
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
    // Participant boxes last (crisp over the lifeline tops). Actors
    // get an outlined box, participants a filled one.
    for (i, b) in sc.boxes.iter().enumerate() {
        let accent = crate::style::accent(i);
        let (fill, stroke, text_fill) = if b.actor {
            ("#ffffff", accent, accent)
        } else {
            (accent, accent, "#ffffff")
        };
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"1.6\"/>\n",
            b.x, b.y, b.w, b.h, fill, stroke
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" fill=\"{}\">{}</text>\n",
            b.x + b.w / 2.0,
            b.y + b.h / 2.0,
            text_fill,
            escape(&b.label)
        ));
    }
    s.push_str("</svg>\n");
    s
}

fn write_head(s: &mut String, h: &Head) {
    if !h.polygon.is_empty() {
        let pts = h
            .polygon
            .iter()
            .map(|(x, y)| format!("{:.1},{:.1}", x, y))
            .collect::<Vec<_>>()
            .join(" ");
        s.push_str(&format!("<polygon points=\"{}\" fill=\"{}\"/>\n", pts, EDGE_COLOR));
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
        let d = sd("sequenceDiagram\nparticipant A as Alice\nactor B as Bob the User\nparticipant C");
        let got: Vec<(&str, &str, bool)> = d
            .participants
            .iter()
            .map(|p| (p.id.as_str(), p.label.as_str(), p.actor))
            .collect();
        assert_eq!(
            got,
            [("A", "Alice", false), ("B", "Bob the User", true), ("C", "C", false)]
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
            let SeqItem::Message { text, dashed, head, .. } = &d.items[0] else {
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
        let SeqItem::Message { activate, deactivate, .. } = &d.items[0] else {
            panic!()
        };
        assert!(*activate && !*deactivate, "`+` activates the target");
        let SeqItem::Message { activate, deactivate, .. } = &d.items[1] else {
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
        assert!(e.message.contains("without a matching activate"), "{}", e.message);
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
        let SeqItem::Message { text, .. } = &d.items[0] else { panic!() };
        assert_eq!(text, "hi");
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
        assert_eq!(kinds, [FrameKind::Alt, FrameKind::Par, FrameKind::Loop, FrameKind::Opt]);
        let elses = d.items.iter().filter(|i| matches!(i, SeqItem::FrameElse { .. })).count();
        let ends = d.items.iter().filter(|i| matches!(i, SeqItem::FrameEnd)).count();
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
        assert!(e.message.contains("unknown message operator"), "{}", e.message);
    }

    // ----------------------------- layout -----------------------------

    #[test]
    fn columns_follow_declaration_order_and_rows_increase() {
        let d = sd("sequenceDiagram\nA->>B: x\nC->>A: a much longer label here\nB-->>C: z");
        let sc = scene(&d);
        assert_eq!(sc.boxes.len(), 3);
        for i in 1..sc.boxes.len() {
            let (a, b) = (&sc.boxes[i - 1], &sc.boxes[i]);
            assert!(
                a.x + a.w < b.x,
                "box {i} must sit right of box {}",
                i - 1
            );
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
        // Label fits the canvas and clears the next column's box.
        let w = label_width(m.number, &m.text);
        assert!(m.label_pos.0 + w <= sc.width);
        assert!(m.label_pos.0 + w <= sc.boxes[1].x);
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
        assert!((bar.y0 - sc.messages[0].points[0].1).abs() < 1e-6, "starts at the + message");
        assert!((bar.y1 - sc.messages[2].points[0].1).abs() < 1e-6, "ends at the - message");
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
        let d = sd("sequenceDiagram\nautonumber\nA-->>B: dashed filled\nA-xB: crossed\nA->B: plain");
        let svg = to_svg(&scene(&d));
        assert!(svg.contains("stroke-dasharray=\"6 4\""), "dashed message line");
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
        assert!(svg.contains(NOTE_FILL) && svg.contains(NOTE_STROKE));
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
        assert!(d.items.iter().any(
            |i| matches!(i, SeqItem::Message { from, to, .. } if from == to)
        ));
        // Explicit + shorthand activations.
        assert!(d.items.iter().any(|i| matches!(i, SeqItem::Activate(_))));
        assert!(d.items.iter().any(|i| matches!(i, SeqItem::Deactivate(_))));
        assert!(d.items.iter().any(
            |i| matches!(i, SeqItem::Message { activate: true, .. })
        ));
        // Every note form.
        let sides: Vec<NoteSide> = d
            .items
            .iter()
            .filter_map(|i| match i {
                SeqItem::Note { side, .. } => Some(*side),
                _ => None,
            })
            .collect();
        assert!(sides.iter().any(|s| matches!(s, NoteSide::Over(_, Some(_)))));
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
        for want in [FrameKind::Loop, FrameKind::Opt, FrameKind::Alt, FrameKind::Par] {
            assert!(kinds.contains(&want), "missing frame {want:?}");
        }
        assert_eq!(
            d.items.iter().filter(|i| matches!(i, SeqItem::FrameElse { .. })).count(),
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
}
