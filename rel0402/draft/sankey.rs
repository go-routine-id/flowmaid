
/// An element of a [`SankeyScene`] under a point
/// ([`SankeyScene::hit_test`]): its index into [`SankeyScene::nodes`] or
/// [`SankeyScene::links`].
///
/// `#[non_exhaustive]`: match it with a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SankeyHit {
    /// A node's bar, or its label.
    Node(usize),
    /// A link's ribbon.
    Link(usize),
}

impl SankeyScene {
    /// The topmost element at scene point `(x, y)`, or `None`, with the
    /// conventions of [`crate::scene::Scene::hit_test`]. Z-order is
    /// [`to_svg`]'s: the labels over the node bars over the ribbons, a
    /// later one over an earlier one. A label (its text box: each line
    /// as wide as it measures, [`LABEL_H`] tall, anchored as drawn)
    /// picks its node. A ribbon is the band [`to_svg`] fills: between its
    /// two curves, `thickness` apart at every x, so a thin flow is as
    /// thin to pick as it is drawn. Every element here is a filled
    /// shape: `tol` widens none. `None` off the canvas (`0..=width` x
    /// `0..=height`).
    ///
    /// ```
    /// use flowmaid::{model::Document, parser::parse_document, sankey};
    ///
    /// let Ok(Document::Sankey(d)) = parse_document("sankey-beta\nA,B,10\n") else {
    ///     unreachable!()
    /// };
    /// let ss = sankey::scene(&d);
    /// let (l, a) = (&ss.links[0], &ss.nodes[0]);
    /// let mid = ((l.x0 + l.x1) / 2.0, (l.y0 + l.y1) / 2.0);
    /// assert_eq!(ss.hit_test(mid.0, mid.1, 0.0), Some(sankey::SankeyHit::Link(0)));
    /// let at = (a.x + a.w / 2.0, a.y + a.h / 2.0);
    /// assert_eq!(ss.hit_test(at.0, at.1, 0.0), Some(sankey::SankeyHit::Node(0)));
    /// ```
    pub fn hit_test(&self, x: f64, y: f64, _tol: f64) -> Option<SankeyHit> {
        if !crate::pick::on_canvas(x, y, self.width, self.height) {
            return None;
        }
        let p = (x, y);
        let label = |n: &SankeyNodeGlyph| {
            let one = [n.label_text.clone()];
            let lines: &[String] = if n.label_lines.len() <= 1 {
                &one
            } else {
                &n.label_lines
            };
            let w = lines
                .iter()
                .map(|l| text_width(l) * FONT as f64 / TEXT_CALIBRATION)
                .fold(0.0, f64::max);
            let h = LABEL_H * lines.len() as f64;
            w > 0.0
                && crate::pick::in_box(
                    p,
                    crate::pick::text_box(n.label_pos.0, n.label_pos.1, w, h, n.label_anchor),
                )
        };
        if let Some(i) = self.nodes.iter().rposition(label) {
            return Some(SankeyHit::Node(i));
        }
        if let Some(i) = self
            .nodes
            .iter()
            .rposition(|n| crate::pick::in_rect(p, n.x, n.y, n.w, n.h))
        {
            return Some(SankeyHit::Node(i));
        }
        self.links
            .iter()
            .rposition(|l| ribbon_contains(l, p))
            .map(SankeyHit::Link)
    }
}

/// Whether `p` lies in link `l`'s ribbon as [`ribbon_path`] outlines
/// it. Its top edge is the cubic from `(x0, y0 - t/2)` to
/// `(x1, y1 - t/2)` with both controls at the middle x, whose x runs
/// `x0 + (x1 - x0)(1.5s - 1.5s² + s³)` (monotonic in `s`) and whose y
/// runs `y_top0 + (y_top1 - y_top0)(3s² - 2s³)`; its bottom edge is the
/// same curve `thickness` lower. So at `p`'s x the band spans one
/// thickness down from the top edge there.
fn ribbon_contains(l: &SankeyLinkGlyph, p: (f64, f64)) -> bool {
    let d = l.x1 - l.x0;
    if l.thickness.is_nan() || l.thickness <= 0.0 || d == 0.0 {
        return false;
    }
    let want = (p.0 - l.x0) / d;
    if !(0.0..=1.0).contains(&want) {
        return false;
    }
    let along = |s: f64| s * (1.5 - 1.5 * s + s * s);
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if along(mid) < want {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let s = (lo + hi) / 2.0;
    let top = l.y0 - l.thickness / 2.0 + (l.y1 - l.y0) * s * s * (3.0 - 2.0 * s);
    p.1 >= top && p.1 <= top + l.thickness
}
