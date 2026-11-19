
/// An element of a [`PieScene`] under a point ([`PieScene::hit_test`]).
/// A legend row and a percentage label resolve to the slice they
/// describe.
///
/// `#[non_exhaustive]`: match it with a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PieHit {
    /// A wedge: its index into [`PieScene::slices`].
    Slice(usize),
    /// A percentage label or its leader: `label` indexes
    /// [`PieScene::labels`], `slice` the slice it labels.
    Label { label: usize, slice: usize },
    /// A legend row (swatch or text): its index into
    /// [`PieScene::legend`], which is also its slice's.
    Legend(usize),
}

impl PieScene {
    /// The topmost element at scene point `(x, y)`, or `None`, with the
    /// conventions of [`crate::scene::Scene::hit_test`]. Z-order is
    /// [`to_svg`]'s: the legend over the percentage labels over the
    /// wedges. A wedge is its angular sector inside the radius as drawn
    /// (a slice drawn as the whole disc covers the slices before it); a
    /// slice of 0 has none. A label is its text box (as wide and tall as
    /// the layout measures it, anchored as drawn) and its leader, a
    /// stroke `tol` widens (`<= 0`: exact; the nearest label wins); a
    /// legend row is its swatch and its text box. `None` off the canvas
    /// (`0..=width` x `0..=height`).
    ///
    /// ```
    /// use flowmaid::{model::Document, parser::parse_document, pie};
    ///
    /// let Ok(Document::Pie(d)) = parse_document("pie\n\"a\" : 3\n\"b\" : 1") else {
    ///     unreachable!()
    /// };
    /// let ps = pie::scene(&d);
    /// // Below the centre lies in `a` (it runs from 12 o'clock round to 9).
    /// let hit = ps.hit_test(ps.cx, ps.cy + ps.r / 3.0, 0.0);
    /// assert_eq!(hit, Some(pie::PieHit::Slice(0)));
    /// ```
    pub fn hit_test(&self, x: f64, y: f64, tol: f64) -> Option<PieHit> {
        use crate::pick::{in_box, in_round_rect, text_box, Nearest};
        if !crate::pick::on_canvas(x, y, self.width, self.height) {
            return None;
        }
        let p = (x, y);
        let legend = self.legend.iter().rposition(|row| {
            let w = text_width(&row.text) * FONT / TEXT_CALIBRATION;
            let line = crate::layout::LINE_H;
            in_round_rect(p, row.x, row.y - SWATCH / 2.0, SWATCH, SWATCH, 2.0)
                || in_box(p, text_box(row.x + SWATCH + 8.0, row.y, w, line, "start"))
        });
        if let Some(i) = legend {
            return Some(PieHit::Legend(i));
        }
        let mut near = Nearest::new(tol);
        for (k, l) in self.labels.iter().enumerate() {
            let text = text_box(l.x, l.y, label_w(&l.text), 2.0 * LABEL_HALF_H, l.anchor);
            near.offer_area(k, in_box(p, text));
            if l.leader.len() >= 2 {
                near.offer_stroke(k, crate::pick::polyline_dist(p, &l.leader), 1.0);
            }
        }
        if let Some(k) = near.get() {
            let slice = self.labels[k].slice;
            return Some(PieHit::Label { label: k, slice });
        }
        (0..self.slices.len())
            .rev()
            .find(|&i| self.slice_contains(i, p))
            .map(PieHit::Slice)
    }

    /// Whether `p` lies in slice `i` as [`to_svg`] fills it: nothing for
    /// a slice of 0, the whole disc for one drawn as a circle (~100%, or
    /// a dominant slice whose arc ends meet), else its sector.
    fn slice_contains(&self, i: usize, p: (f64, f64)) -> bool {
        let sl = &self.slices[i];
        let (dx, dy) = (p.0 - self.cx, p.1 - self.cy);
        if sl.frac <= 0.0 || dx.hypot(dy) > self.r {
            return false;
        }
        let a = polar(self.cx, self.cy, self.r, sl.start_angle);
        let b = polar(self.cx, self.cy, self.r, sl.end_angle);
        let coincident = (a.0 - b.0).hypot(a.1 - b.1) < 0.2;
        if sl.frac >= 1.0 - 1e-9 || (sl.frac > 0.5 && coincident) {
            return true;
        }
        // From 12 o'clock, clockwise, as `polar` measures.
        let t = dx.atan2(-dy).rem_euclid(TAU);
        [t, t + TAU]
            .iter()
            .any(|&t| t >= sl.start_angle && t <= sl.end_angle)
    }
}
