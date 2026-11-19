
/// An element of a [`TimelineScene`] under a point
/// ([`TimelineScene::hit_test`]), by its indexes: `section` into
/// [`TimelineScene::sections`], `period` into that section's
/// [`SectionGlyph::periods`], `event` into that period's
/// [`PeriodGlyph::events`].
///
/// `#[non_exhaustive]`: match it with a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimelineHit {
    /// A section's header band (its name included) or its axis line.
    Section(usize),
    /// A period's label, its tick on the axis, or its dashed task line.
    Period { section: usize, period: usize },
    /// An event's text.
    Event {
        section: usize,
        period: usize,
        event: usize,
    },
}

impl TimelineScene {
    /// The topmost element at scene point `(x, y)`, or `None`, with the
    /// conventions of [`crate::scene::Scene::hit_test`]. Z-order is
    /// [`to_svg`]'s: a section draws its band, then its axis, then each
    /// period's task line, tick, label and events, and a later section
    /// draws over an earlier one. Texts are their boxes (as wide as their
    /// widest line measures, from the cap top of the first line to the
    /// descender of the last, centred as drawn), a tick
    /// its disc, a band its rounded rectangle (named sections only);
    /// the axis and the task lines are strokes `tol` widens (`<= 0`:
    /// exact; the nearest wins among the task lines). The title is not an
    /// element. `None` off the canvas (`0..=width` x `0..=height`).
    ///
    /// ```
    /// use flowmaid::{model::Document, parser::parse_document, timeline};
    ///
    /// let Ok(Document::Timeline(d)) = parse_document("timeline\n  2002 : LinkedIn\n") else {
    ///     unreachable!()
    /// };
    /// let ts = timeline::scene(&d);
    /// let p = &ts.sections[0].periods[0];
    /// let (_, y) = p.events[0];
    /// let hit = ts.hit_test(p.cx, y, 0.0);
    /// assert_eq!(hit, Some(timeline::TimelineHit::Event { section: 0, period: 0, event: 0 }));
    /// ```
    pub fn hit_test(&self, x: f64, y: f64, tol: f64) -> Option<TimelineHit> {
        use crate::pick::{in_box, in_disc, in_round_rect, seg_dist, Nearest};
        if !crate::pick::on_canvas(x, y, self.width, self.height) {
            return None;
        }
        let p = (x, y);
        let font = f64::from(FONT);
        // A centred text block's box, `bold` or not: as wide as its
        // widest line, from its first line's cap top to its last line's
        // descender ([`glyph_span`], what the task lines keep clear of).
        let block = |cx: f64, cy: f64, text: &str, bold: bool| {
            let lines: Vec<&str> = text.split('\n').collect();
            let w = lines
                .iter()
                .map(|l| width_at(l, font, bold))
                .fold(0.0, f64::max);
            let (top, bottom) = glyph_span(cy, lines.len() as f64);
            w > 0.0 && in_box(p, (cx - w / 2.0, top, w, bottom - top))
        };
        for (s, sec) in self.sections.iter().enumerate().rev() {
            // Texts and ticks, the last period's first.
            for (k, per) in sec.periods.iter().enumerate().rev() {
                for (e, (text, ey)) in per.events.iter().enumerate().rev() {
                    if block(per.cx, *ey, text, false) {
                        return Some(TimelineHit::Event {
                            section: s,
                            period: k,
                            event: e,
                        });
                    }
                }
                if block(per.cx, per.label_y, &per.label, true)
                    || in_disc(p, (per.cx, sec.axis_y), TICK_R)
                {
                    return Some(TimelineHit::Period {
                        section: s,
                        period: k,
                    });
                }
            }
            let mut near = Nearest::new(tol);
            for (k, per) in sec.periods.iter().enumerate() {
                for &(y0, y1) in &per.line_segments {
                    near.offer_stroke(k, seg_dist(p, (per.cx, y0), (per.cx, y1)), 1.2);
                }
            }
            if let Some(k) = near.get() {
                return Some(TimelineHit::Period {
                    section: s,
                    period: k,
                });
            }
            let mut axis = Nearest::new(tol);
            let d = seg_dist(p, (sec.x, sec.axis_y), (sec.x + sec.w, sec.axis_y));
            axis.offer_stroke(0, d, 2.0);
            if axis.get().is_some()
                || (!sec.name.is_empty() && in_round_rect(p, sec.x, sec.y, sec.w, SECTION_H, 4.0))
            {
                return Some(TimelineHit::Section(s));
            }
        }
        None
    }
}
