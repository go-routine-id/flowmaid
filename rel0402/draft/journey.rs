
/// An element of a [`JourneyScene`] under a point
/// ([`JourneyScene::hit_test`]): its index into [`JourneyScene::tasks`],
/// [`JourneyScene::legend`] or [`JourneyScene::sections`].
///
/// `#[non_exhaustive]`: match it with a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum JourneyHit {
    /// A task's face, one of its actor marks, or its label.
    Task(usize),
    /// A legend item: its actor mark or its name.
    Legend(usize),
    /// A section's band (its name included).
    Section(usize),
}

/// Whether `p` lies in the actor mark `mark` centred on `c`, as
/// [`dot_svg`] draws it.
fn mark_contains(mark: ActorMark, c: (f64, f64), p: (f64, f64)) -> bool {
    use crate::pick::{in_disc, in_polygon, in_round_rect};
    let reach = 1.25 * DOT_R;
    match mark {
        ActorMark::Circle => in_disc(p, c, DOT_R),
        ActorMark::Square => {
            let h = 0.9 * DOT_R;
            in_round_rect(p, c.0 - h, c.1 - h, 2.0 * h, 2.0 * h, 1.0)
        }
        ActorMark::Diamond => in_polygon(
            p,
            &[
                (c.0, c.1 - reach),
                (c.0 + reach, c.1),
                (c.0, c.1 + reach),
                (c.0 - reach, c.1),
            ],
        ),
        ActorMark::Triangle => {
            let dx = reach * 3f64.sqrt() / 2.0;
            in_polygon(
                p,
                &[
                    (c.0, c.1 - reach),
                    (c.0 + dx, c.1 + reach / 2.0),
                    (c.0 - dx, c.1 + reach / 2.0),
                ],
            )
        }
    }
}

impl JourneyScene {
    /// The topmost element at scene point `(x, y)`, or `None`, with the
    /// conventions of [`crate::scene::Scene::hit_test`]. Z-order is
    /// [`to_svg`]'s: the legend over the tasks over the section bands. A
    /// task is its face, its actor marks and its label's text box (each
    /// line as wide as it measures, [`LINE_H`] tall); a legend item its
    /// mark and its name's box; a section its rounded band. The journey
    /// path is not an element (it joins the faces), and the title is
    /// none either. Every element here is a filled shape or text: `tol`
    /// widens none. `None` off the canvas (`0..=width` x `0..=height`).
    ///
    /// ```
    /// use flowmaid::{journey, model::Document, parser::parse_document};
    ///
    /// let src = "journey\n  section Morning\n    Coffee: 5: Me\n";
    /// let Ok(Document::Journey(d)) = parse_document(src) else { unreachable!() };
    /// let js = journey::scene(&d);
    /// let t = &js.tasks[0];
    /// assert_eq!(js.hit_test(t.cx, t.cy, 0.0), Some(journey::JourneyHit::Task(0)));
    /// ```
    pub fn hit_test(&self, x: f64, y: f64, _tol: f64) -> Option<JourneyHit> {
        use crate::pick::{in_box, in_disc, in_round_rect, text_box};
        if !crate::pick::on_canvas(x, y, self.width, self.height) {
            return None;
        }
        let p = (x, y);
        let font = f64::from(FONT);
        let legend = self.legend.iter().enumerate().rposition(|(a, it)| {
            let w = width_at(&it.name, font, false);
            mark_contains(actor_mark(a), (it.x + DOT_R, it.y), p)
                || in_box(p, text_box(it.x + 2.0 * DOT_R + 6.0, it.y, w, LINE_H, "start"))
        });
        if let Some(a) = legend {
            return Some(JourneyHit::Legend(a));
        }
        let task = self.tasks.iter().rposition(|t| {
            let lines: Vec<&str> = t.label.split('\n').collect();
            let w = lines
                .iter()
                .map(|l| width_at(l, font, false))
                .fold(0.0, f64::max);
            let n = lines.len() as f64;
            // Centred where `to_svg` centres the block.
            let cy = t.label_pos.1 + (n - 1.0) * LINE_H / 2.0;
            in_disc(p, (t.cx, t.cy), t.r)
                || t.actor_dots.iter().enumerate().any(|(k, &(dx, dy, _))| {
                    let mark = t.actor_ids.get(k).map_or(ActorMark::Circle, |&a| actor_mark(a));
                    mark_contains(mark, (dx, dy), p)
                })
                || (w > 0.0 && in_box(p, text_box(t.label_pos.0, cy, w, n * LINE_H, "middle")))
        });
        if let Some(i) = task {
            return Some(JourneyHit::Task(i));
        }
        self.sections
            .iter()
            .rposition(|b| in_round_rect(p, b.x, b.y, b.w, b.h, 6.0))
            .map(JourneyHit::Section)
    }
}
