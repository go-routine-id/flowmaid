
/// An element of a [`SeqScene`] under a point ([`SeqScene::hit_test`]).
/// A participant's header box, its footer box and its lifeline each
/// resolve to the participant's index (into [`SeqScene::boxes`],
/// [`SeqScene::footer`] and [`SeqScene::lifelines`] alike); the others
/// are indexes into their own vec.
///
/// `#[non_exhaustive]`: match it with a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SeqHit {
    /// A participant's header box (an actor's stick figure or name).
    Participant(usize),
    /// A participant's footer box (`mirrorActors`).
    Footer(usize),
    /// A participant's lifeline.
    Lifeline(usize),
    /// A message: its line, its head or its label.
    Message(usize),
    /// A note card.
    Note(usize),
    /// An activation bar.
    Activation(usize),
    /// A frame's drawn parts: its border, its keyword chip, its
    /// condition labels or an `else` / `and` divider.
    Frame(usize),
    /// Inside a frame on nothing drawn (the innermost frame there): a
    /// click there selects the frame without hiding what lies inside it.
    FrameInside(usize),
}

/// Height of a text line's box here: the card [`to_svg`] draws under a
/// frame's condition label.
const TEXT_BOX_H: f64 = 16.0;

/// Stroke widths [`to_svg`] draws with.
const MSG_STROKE: f64 = 1.6;
const FRAME_STROKE: f64 = 1.2;
const LIFELINE_STROKE: f64 = 1.0;

/// Distance from `p` to the outline of the rounded rectangle at `(x, y)`,
/// `w` x `h`, corners `r` round (clamped to half each side).
fn round_rect_outline_dist(p: (f64, f64), x: f64, y: f64, w: f64, h: f64, r: f64) -> f64 {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let (hw, hh) = (w / 2.0, h / 2.0);
    let (qx, qy) = (
        (p.0 - (x + hw)).abs() - (hw - r),
        (p.1 - (y + hh)).abs() - (hh - r),
    );
    let outside = qx.max(0.0).hypot(qy.max(0.0));
    (outside + qx.max(qy).min(0.0) - r).abs()
}

/// How far `p` lies from participant box `b`'s drawing: 0 inside its
/// filled box (corners 4 round), or for an actor inside its head or
/// name's box, else the distance to the figure's strokes beyond their
/// half width (infinity for a plain box).
fn part_dist(b: &PartBox, p: (f64, f64)) -> f64 {
    use crate::pick::{in_box, in_disc, in_round_rect, seg_dist, text_box};
    if !b.actor {
        return if in_round_rect(p, b.x, b.y, b.w, b.h, 4.0) {
            0.0
        } else {
            f64::INFINITY
        };
    }
    let fig = actor_figure(b);
    let name = text_box(
        fig.label_pos.0,
        fig.label_pos.1,
        text_width(&b.label),
        TEXT_BOX_H,
        "middle",
    );
    if in_disc(p, fig.head, fig.head_r) || in_box(p, name) {
        return 0.0;
    }
    fig.segments
        .iter()
        .map(|s| (seg_dist(p, s[0], s[1]) - MSG_STROKE / 2.0).max(0.0))
        .fold(f64::INFINITY, f64::min)
}

impl SeqScene {
    /// The topmost element at scene point `(x, y)`, or `None`, with the
    /// conventions of [`crate::scene::Scene::hit_test`]. Z-order is
    /// [`to_svg`]'s, top first: participant boxes (the footer row, then
    /// the header row), messages, notes, the frames' keyword and
    /// condition texts, activation bars, the frames' keyword chips and
    /// the cards under their conditions, `else` / `and` dividers,
    /// lifelines, frame borders, and last the inside of a frame, where
    /// nothing is drawn ([`SeqHit::FrameInside`]): a click inside a
    /// `loop` picks a lifeline, message or note there, not the frame.
    ///
    /// Filled shapes (boxes, bars, notes, chips, an actor's head) and
    /// texts (their boxes, as wide as the layout measures them and 16 px
    /// tall) are exact; strokes (message lines and heads, lifelines,
    /// dividers, frame borders, an actor's limbs) are widened by `tol`
    /// scene units (`<= 0`: exact), the nearest of a kind winning. `None`
    /// off the canvas (`0..=width` x `0..=height`).
    ///
    /// ```
    /// use flowmaid::{model::Document, parser::parse_document, seq};
    ///
    /// let Ok(Document::Sequence(d)) = parse_document("sequenceDiagram\nA->>B: hi") else {
    ///     unreachable!()
    /// };
    /// let sc = seq::scene(&d);
    /// let m = &sc.messages[0];
    /// let mid = ((m.points[0].0 + m.points[1].0) / 2.0, m.points[0].1);
    /// assert_eq!(sc.hit_test(mid.0, mid.1, 2.0), Some(seq::SeqHit::Message(0)));
    /// let b = &sc.boxes[1];
    /// let hit = sc.hit_test(b.x + b.w / 2.0, b.y + b.h / 2.0, 2.0);
    /// assert_eq!(hit, Some(seq::SeqHit::Participant(1)));
    /// ```
    pub fn hit_test(&self, x: f64, y: f64, tol: f64) -> Option<SeqHit> {
        use crate::pick::{
            in_box, in_polygon, in_rect, in_round_rect, polyline_dist, seg_dist, text_box, Nearest,
        };
        if !crate::pick::on_canvas(x, y, self.width, self.height) {
            return None;
        }
        let p = (x, y);
        // Participant boxes: the footer row is drawn after the header.
        for (row, boxes) in [(true, &self.footer), (false, &self.boxes)] {
            let mut near = Nearest::new(tol);
            for (k, b) in boxes.iter().enumerate() {
                near.offer(k, part_dist(b, p));
            }
            if let Some(k) = near.get() {
                return Some(if row {
                    SeqHit::Footer(k)
                } else {
                    SeqHit::Participant(k)
                });
            }
        }
        // Messages: line, head and label.
        let mut near = Nearest::new(tol);
        for (i, m) in self.messages.iter().enumerate() {
            near.offer_stroke(i, polyline_dist(p, &m.points), MSG_STROKE);
            let n = m.points.len();
            if n >= 2 {
                let h = head(m.points[n - 1], m.points[n - 2], m.head);
                near.offer_area(i, in_polygon(p, &h.polygon));
                for s in &h.segments {
                    near.offer_stroke(i, seg_dist(p, s[0], s[1]), MSG_STROKE);
                }
            }
            if !m.text.is_empty() || m.number.is_some() {
                let anchor = if m.label_centered { "middle" } else { "start" };
                let w = label_width(m.number, &m.text);
                let b = text_box(m.label_pos.0, m.label_pos.1, w, TEXT_BOX_H, anchor);
                near.offer_area(i, in_box(p, b));
            }
        }
        if let Some(i) = near.get() {
            return Some(SeqHit::Message(i));
        }
        if let Some(i) = self
            .notes
            .iter()
            .rposition(|nb| in_round_rect(p, nb.x, nb.y, nb.w, nb.h, 3.0))
        {
            return Some(SeqHit::Note(i));
        }
        // A frame's condition texts and keyword, drawn over the bars.
        let texts = |f: &FrameBox| {
            let kw = f.kind.keyword();
            let chip = (f.x + chip_w(f.kind) / 2.0, f.y + 9.0);
            in_box(p, text_box(chip.0, chip.1, text_width(kw), TEXT_BOX_H, "middle"))
                || frame_label_rows(f).any(|(cy, text)| {
                    let w = text_width(&text);
                    in_box(p, text_box(frame_label_x(f), cy, w, TEXT_BOX_H, "start"))
                })
        };
        if let Some(i) = self.frames.iter().rposition(texts) {
            return Some(SeqHit::Frame(i));
        }
        if let Some(i) = self
            .activations
            .iter()
            .rposition(|a| in_rect(p, a.x - ACT_W / 2.0, a.y0, ACT_W, a.y1 - a.y0))
        {
            return Some(SeqHit::Activation(i));
        }
        // The keyword chips and the cards under the conditions.
        let cards = |f: &FrameBox| {
            in_rect(p, f.x, f.y, chip_w(f.kind), 18.0)
                || frame_label_rows(f).any(|(cy, text)| {
                    let w = text_width(&text) + 4.0;
                    in_rect(p, frame_label_x(f) - 2.0, cy - 8.0, w, 16.0)
                })
        };
        if let Some(i) = self.frames.iter().rposition(cards) {
            return Some(SeqHit::Frame(i));
        }
        let mut dividers = Nearest::new(tol);
        for (i, f) in self.frames.iter().enumerate() {
            for (dy, _) in &f.dividers {
                let d = seg_dist(p, (f.x, *dy), (f.x + f.w, *dy));
                dividers.offer_stroke(i, d, FRAME_STROKE);
            }
        }
        if let Some(i) = dividers.get() {
            return Some(SeqHit::Frame(i));
        }
        let mut lifelines = Nearest::new(tol);
        for (k, l) in self.lifelines.iter().enumerate() {
            lifelines.offer_stroke(k, seg_dist(p, (l.x, l.y0), (l.x, l.y1)), LIFELINE_STROKE);
        }
        if let Some(k) = lifelines.get() {
            return Some(SeqHit::Lifeline(k));
        }
        let mut borders = Nearest::new(tol);
        for (i, f) in self.frames.iter().enumerate() {
            let d = round_rect_outline_dist(p, f.x, f.y, f.w, f.h, 4.0);
            borders.offer_stroke(i, d, FRAME_STROKE);
        }
        if let Some(i) = borders.get() {
            return Some(SeqHit::Frame(i));
        }
        self.frames
            .iter()
            .rposition(|f| in_rect(p, f.x, f.y, f.w, f.h))
            .map(SeqHit::FrameInside)
    }
}
