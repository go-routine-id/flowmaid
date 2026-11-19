
// ------------------------------------------------------------------
// Picking
// ------------------------------------------------------------------

/// An element of a [`MindScene`] under a point ([`MindScene::hit_test`]):
/// its index into [`MindScene::nodes`] or [`MindScene::edges`]. A
/// connector's two nodes are [`link_ends`]`(d)[j]`.
///
/// `#[non_exhaustive]`: match it with a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MindHit {
    /// A node's shape.
    Node(usize),
    /// A connector's stroke.
    Link(usize),
}

impl MindScene {
    /// The topmost element at scene point `(x, y)`, or `None`, with the
    /// conventions of [`crate::scene::Scene::hit_test`]. Z-order is
    /// [`to_svg`]'s: nodes over connectors, a later node over an earlier
    /// one. A node is its painted shape (its [`perimeter`] polygon, its
    /// circle, its box, pill, or default box with rounded top corners),
    /// never widened; a connector is its stroke ([`MindLink::width`]
    /// wide, round ends) widened by `tol` scene units (`<= 0`: exact),
    /// the nearest connector winning. `None` off the canvas (`0..=width`
    /// x `0..=height`).
    ///
    /// ```
    /// use flowmaid::{mindmap, model::Document, parser::parse_document};
    ///
    /// let Ok(Document::Mindmap(d)) = parse_document("mindmap\n  root\n    child") else {
    ///     unreachable!()
    /// };
    /// let ms = mindmap::scene(&d);
    /// let n = &ms.nodes[1];
    /// assert_eq!(ms.hit_test(n.cx(), n.cy(), 4.0), Some(mindmap::MindHit::Node(1)));
    /// ```
    pub fn hit_test(&self, x: f64, y: f64, tol: f64) -> Option<MindHit> {
        if !crate::pick::on_canvas(x, y, self.width, self.height) {
            return None;
        }
        let p = (x, y);
        if let Some(i) = self.nodes.iter().rposition(|n| node_contains(n, p)) {
            return Some(MindHit::Node(i));
        }
        let mut near = crate::pick::Nearest::new(tol);
        for (j, e) in self.edges.iter().enumerate() {
            near.offer_stroke(j, crate::pick::cubic_dist(p, &e.bezier), e.width);
        }
        near.get().map(MindHit::Link)
    }
}

/// Whether `p` lies in node `n`'s shape as [`shape_svg`] paints it.
fn node_contains(n: &MindNodeBox, p: (f64, f64)) -> bool {
    use crate::pick::{in_disc, in_polygon, in_rect, in_round_rect};
    if let Some(pts) = perimeter(n) {
        return in_polygon(p, &pts);
    }
    match n.shape {
        MindShape::Circle => in_disc(p, (n.cx(), n.cy()), n.w.min(n.h) / 2.0),
        MindShape::Default => {
            if !in_rect(p, n.x, n.y, n.w, n.h) {
                return false;
            }
            // The top corners are quadratic arcs `r` round: from a
            // corner, the arc is where sqrt(dx / r) + sqrt(dy / r) = 1,
            // and the shape lies beyond it.
            let r = DEFAULT_RX.min(n.h / 2.0).min(n.w / 2.0);
            let (dx, dy) = ((p.0 - n.x).min(n.x + n.w - p.0), p.1 - n.y);
            r <= 0.0 || dx >= r || dy >= r || (dx / r).sqrt() + (dy / r).sqrt() >= 1.0
        }
        MindShape::Square => in_rect(p, n.x, n.y, n.w, n.h),
        _ => in_round_rect(p, n.x, n.y, n.w, n.h, n.h / 2.0),
    }
}
