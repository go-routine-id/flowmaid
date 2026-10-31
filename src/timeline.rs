//! Timeline rendering: a horizontal axis of period columns with events
//! stacked beneath, and sections drawn as colored header bands.
//!
//! Mirrors mermaid's timeline structure: in `LR` (the default) sections
//! sit side by side and all periods share one horizontal axis; in `TD`
//! sections stack top-to-bottom, each with its own axis. Periods run
//! left-to-right as columns in either direction; a dashed "task" line
//! drops from each period's tick on the axis down through its events,
//! drawn only in the gaps between them so it never strikes through
//! their text. Period labels are bold, events regular; text wider than
//! [`WRAP_W`] wraps instead of stretching its column.
//!
//! Like `pie`/`journey` there is nothing draggable and no `route()`:
//! [`scene`] computes every coordinate and [`to_svg`] serialises it.

use crate::layout::{text_width, wrap_text, LINE_H, TEXT_CALIBRATION};
use crate::model::{Timeline, TimelineDirection, TimelinePeriod};
use crate::scene::{
    escape, svg_open_a11y, svg_text_lines, svg_text_multiline, A11y, SvgOptions, TEXT_COLOR,
};
use crate::style::accent;

/// Canvas margin.
pub const PAD: f64 = 24.0;
/// Title band height (when a title is present).
pub const TITLE_H: f64 = 34.0;
/// Extra room between the title band and the first section header.
pub const TITLE_GAP: f64 = 10.0;
/// Section header band height.
pub const SECTION_H: f64 = 30.0;
/// Gap between a section header and the axis/period labels.
pub const GAP: f64 = 14.0;
/// Vertical gap between stacked sections in `TD` mode.
pub const SECTION_GAP: f64 = 24.0;
/// Gap between the axis and the first event (and between events).
pub const EVENT_GAP: f64 = 8.0;
/// Minimum period column width.
pub const COL_MIN: f64 = 96.0;
/// Horizontal padding inside a period column.
pub const COL_PAD: f64 = 14.0;
/// Period labels and events wider than this wrap onto more lines.
pub const WRAP_W: f64 = 160.0;
/// Base font size.
pub const FONT: u32 = 13;
/// Axis line color: a neutral at 3.2:1 on white, so the spine reads as
/// the diagram's backbone (WCAG 1.4.11 asks 3:1 of meaningful lines).
/// (0.32 and earlier: `#c2c8dc`, the axis and the dashed task lines
/// alike; the task lines keep that colour as [`TASK_LINE_COLOR`].)
pub const AXIS_COLOR: &str = "#8a90a8";
/// Dashed task-line color — paler than the axis, it only links a
/// period to its events.
pub const TASK_LINE_COLOR: &str = "#c2c8dc";
/// Tick color of the unnamed leading section (periods before the first
/// `section`), which has no band to explain an accent.
pub const UNNAMED_COLOR: &str = AXIS_COLOR;
/// Tick radius on the axis.
pub const TICK_R: f64 = 4.5;
/// Title font size.
const TITLE_FONT: f64 = 17.0;
/// Width factor of bold text over regular (as `text_width` counts
/// `<b>` runs).
const BOLD: f64 = 1.06;
/// Room kept between a task-line segment and the text it stops at.
const TEXT_CLEAR: f64 = 2.0;

/// Number of rendered lines in a label (after `<br>` folding).
fn line_count(s: &str) -> f64 {
    s.split('\n').count() as f64
}

/// Width of `s` drawn at `size` px (bold when `bold`).
fn width_at(s: &str, size: f64, bold: bool) -> f64 {
    text_width(s) * size / TEXT_CALIBRATION * if bold { BOLD } else { 1.0 }
}

/// `s` wrapped at [`WRAP_W`] (measured at [`FONT`]).
fn wrap(s: &str) -> String {
    wrap_text(s, WRAP_W, f64::from(FONT) / TEXT_CALIBRATION)
}

/// One period with its text wrapped for drawing.
struct Wrapped {
    label: String,
    events: Vec<String>,
}

impl Wrapped {
    fn new(p: &TimelinePeriod) -> Self {
        Wrapped {
            label: wrap(&p.period),
            events: p.events.iter().map(|e| wrap(e)).collect(),
        }
    }

    /// Column width: wide enough for its (bold) label and its events.
    fn width(&self) -> f64 {
        let font = f64::from(FONT);
        let mut w = width_at(&self.label, font, true) + 2.0 * COL_PAD;
        for e in &self.events {
            w = w.max(width_at(e, font, false) + 2.0 * COL_PAD);
        }
        w.max(COL_MIN)
    }
}

/// Width a named section's header needs for its bold name.
fn name_w(name: &str) -> f64 {
    if name.is_empty() {
        0.0
    } else {
        width_at(name, f64::from(FONT), true) + 2.0 * COL_PAD
    }
}

/// Vertical extent of the glyphs of a centred text block of `lines`
/// lines around `cy` (cap top to descender), for the task-line gaps.
fn glyph_span(cy: f64, lines: f64) -> (f64, f64) {
    let half = (lines - 1.0) * LINE_H / 2.0;
    (cy - half - 5.5, cy + half + 7.0)
}

/// A colored header band for one section, plus its period columns.
#[derive(Debug, Clone)]
pub struct SectionGlyph {
    pub name: String,
    /// Band fill and tick color: an accent per NAMED section, in order;
    /// [`UNNAMED_COLOR`] for the unnamed leading section.
    pub color: &'static str,
    /// Header band rect (x, y, w; height is [`SECTION_H`]).
    pub x: f64,
    pub y: f64,
    pub w: f64,
    /// Y of this section's horizontal axis line.
    pub axis_y: f64,
    pub periods: Vec<PeriodGlyph>,
}

/// One period column: a dashed task line, its label, and stacked events.
/// `#[non_exhaustive]`: [`scene`] builds these; read their fields.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PeriodGlyph {
    /// The period text as drawn (bold; wrapped at [`WRAP_W`]).
    pub label: String,
    /// Column centre X.
    pub cx: f64,
    /// Vertical centre of the period label (above the axis).
    pub label_y: f64,
    /// Bottom of the dashed task line (top is the section's `axis_y`):
    /// the top of the last event's text. Draw it as
    /// [`PeriodGlyph::line_segments`], which skip the event texts.
    pub line_bottom: f64,
    /// (event text as drawn — wrapped at [`WRAP_W`] — and its vertical
    /// centre) per event, top to bottom.
    pub events: Vec<(String, f64)>,
    /// The drawn pieces of the task line, as `(y_top, y_bottom)` at
    /// `cx`: from under the tick to the first event, then the gaps
    /// between events — never through text.
    pub line_segments: Vec<(f64, f64)>,
}

/// Everything needed to draw a timeline.
#[derive(Debug, Clone)]
pub struct TimelineScene {
    pub width: f64,
    pub height: f64,
    pub direction: TimelineDirection,
    pub title: Option<String>,
    pub title_pos: (f64, f64),
    pub sections: Vec<SectionGlyph>,
}

/// Lay out one section's period columns left-to-right starting at
/// `left` on the axis at `axis_y`, at least `min_w` wide in total (a
/// shortfall is spread evenly over the columns). Returns the period
/// glyphs, the section's content width (sum of columns) and the bottom
/// of its lowest text.
fn layout_section_periods(
    periods: &[TimelinePeriod],
    left: f64,
    axis_y: f64,
    min_w: f64,
) -> (Vec<PeriodGlyph>, f64, f64) {
    let wrapped: Vec<Wrapped> = periods.iter().map(Wrapped::new).collect();
    let mut widths: Vec<f64> = wrapped.iter().map(Wrapped::width).collect();
    let have: f64 = widths.iter().sum();
    if !widths.is_empty() && have < min_w {
        let extra = (min_w - have) / widths.len() as f64;
        for w in &mut widths {
            *w += extra;
        }
    }
    let mut glyphs = Vec::with_capacity(periods.len());
    let mut x = left;
    let mut bottom = axis_y;
    for (p, w) in wrapped.into_iter().zip(widths) {
        let cx = x + w / 2.0;
        let label_y = axis_y - EVENT_GAP - line_count(&p.label) * LINE_H / 2.0;
        let mut events = Vec::with_capacity(p.events.len());
        let mut segments = Vec::new();
        // The task line runs from under the tick to each event's text
        // top, then resumes under that text.
        let mut seg_top = axis_y + TICK_R + 1.0;
        let mut line_bottom = axis_y;
        let mut ey = axis_y + EVENT_GAP;
        for ev in p.events {
            let lines = line_count(&ev);
            let cy = ey + lines * LINE_H / 2.0;
            let (g0, g1) = glyph_span(cy, lines);
            if g0 - TEXT_CLEAR > seg_top {
                segments.push((seg_top, g0 - TEXT_CLEAR));
            }
            line_bottom = g0 - TEXT_CLEAR;
            seg_top = g1 + TEXT_CLEAR;
            events.push((ev, cy));
            ey += lines * LINE_H + EVENT_GAP;
        }
        if !events.is_empty() {
            bottom = bottom.max(ey - EVENT_GAP);
        }
        glyphs.push(PeriodGlyph {
            label: p.label,
            cx,
            label_y,
            line_bottom,
            events,
            line_segments: segments,
        });
        x += w;
    }
    (glyphs, x - left, bottom)
}

/// Section colors: an accent per named section, counted over named
/// sections only, so the unnamed leading group (neutral) does not shift
/// the colors of the bands after it.
fn section_colors(d: &Timeline) -> Vec<&'static str> {
    let mut named = 0usize;
    d.sections
        .iter()
        .map(|s| {
            if s.name.is_empty() {
                UNNAMED_COLOR
            } else {
                named += 1;
                accent(named - 1)
            }
        })
        .collect()
}

/// Canvas width for content `content_w` wide and the title, plus the
/// x shift that centres the content under a wider title.
fn fit_title(d: &Timeline, content_w: f64) -> (f64, f64) {
    let title_w = d
        .title
        .as_ref()
        .map_or(0.0, |t| width_at(t, TITLE_FONT, true) + 2.0 * PAD);
    let width = content_w.max(title_w);
    (width, (width - content_w) / 2.0)
}

/// Shift a section (band and periods) right by `dx`.
fn shift_section(sec: &mut SectionGlyph, dx: f64) {
    sec.x += dx;
    for p in &mut sec.periods {
        p.cx += dx;
    }
}

/// Compute all geometry.
pub fn scene(d: &Timeline) -> TimelineScene {
    match d.direction {
        TimelineDirection::LeftRight => layout_lr(d),
        TimelineDirection::TopDown => layout_td(d),
    }
}

/// Top of the content: under the title band (and its gap) if any.
fn content_top(d: &Timeline) -> f64 {
    PAD + if d.title.is_some() {
        TITLE_H + TITLE_GAP
    } else {
        0.0
    }
}

/// Tallest wrapped period label of `periods`, in px (at least a line).
fn label_h_max<'a>(periods: impl Iterator<Item = &'a TimelinePeriod>) -> f64 {
    periods
        .map(|p| line_count(&wrap(&p.period)) * LINE_H)
        .fold(LINE_H, f64::max)
}

/// `LR` layout: sections side by side, one shared horizontal axis.
fn layout_lr(d: &Timeline) -> TimelineScene {
    let top = content_top(d);
    // One axis shared by every section: below the tallest period label.
    let label_h = label_h_max(d.sections.iter().flat_map(|s| s.periods.iter()));
    // Reserve a band only when at least one section is named; a section-less
    // timeline sits directly on the axis (no empty colored strip up top).
    let band_h = if d.sections.iter().any(|s| !s.name.is_empty()) {
        SECTION_H
    } else {
        0.0
    };
    let axis_y = top + band_h + GAP + label_h + EVENT_GAP;

    let colors = section_colors(d);
    let mut sections = Vec::with_capacity(d.sections.len());
    let mut x = PAD;
    let mut deepest = axis_y;
    for (si, sec) in d.sections.iter().enumerate() {
        let (periods, w, bottom) =
            layout_section_periods(&sec.periods, x, axis_y, name_w(&sec.name));
        let w = if w > 0.0 {
            w
        } else {
            COL_MIN.max(name_w(&sec.name))
        };
        deepest = deepest.max(bottom);
        sections.push(SectionGlyph {
            name: sec.name.clone(),
            color: colors[si],
            x,
            y: top,
            w,
            axis_y,
            periods,
        });
        // Sections touch in LR: one continuous axis line.
        x += w;
    }
    let content_w = if sections.is_empty() {
        2.0 * PAD
    } else {
        x + PAD
    };
    let height = deepest + PAD;

    let (width, dx) = fit_title(d, content_w);
    for sec in &mut sections {
        shift_section(sec, dx);
    }
    let title_pos = (width / 2.0, PAD + TITLE_H / 2.0);

    TimelineScene {
        width,
        height,
        direction: TimelineDirection::LeftRight,
        title: d.title.clone(),
        title_pos,
        sections,
    }
}

/// `TD` layout: sections stacked top-to-bottom, each with its own axis.
fn layout_td(d: &Timeline) -> TimelineScene {
    let y = content_top(d);

    let colors = section_colors(d);
    let mut sections = Vec::with_capacity(d.sections.len());
    let mut max_w = 0.0f64;
    let mut y_cursor = y;
    for (si, sec) in d.sections.iter().enumerate() {
        let label_h = label_h_max(sec.periods.iter());
        let band_h = if sec.name.is_empty() { 0.0 } else { SECTION_H };
        let axis_y = y_cursor + band_h + GAP + label_h + EVENT_GAP;
        let (periods, w, deepest) =
            layout_section_periods(&sec.periods, PAD, axis_y, name_w(&sec.name));
        let w = if w > 0.0 {
            w
        } else {
            COL_MIN.max(name_w(&sec.name))
        };
        max_w = max_w.max(w);
        sections.push(SectionGlyph {
            name: sec.name.clone(),
            color: colors[si],
            x: PAD,
            y: y_cursor,
            w,
            axis_y,
            periods,
        });
        y_cursor = deepest + SECTION_GAP;
    }

    let content_w = if sections.is_empty() {
        2.0 * PAD
    } else {
        max_w + 2.0 * PAD
    };
    let height = if sections.is_empty() {
        y + 2.0 * PAD
    } else {
        y_cursor - SECTION_GAP + PAD
    };

    let (width, dx) = fit_title(d, content_w);
    for sec in &mut sections {
        shift_section(sec, dx);
    }
    let title_pos = (width / 2.0, PAD + TITLE_H / 2.0);

    TimelineScene {
        width,
        height,
        direction: TimelineDirection::TopDown,
        title: d.title.clone(),
        title_pos,
        sections,
    }
}

/// Serialise a [`TimelineScene`] to a standalone SVG document.
pub fn to_svg(ts: &TimelineScene) -> String {
    to_svg_with(ts, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(ts: &TimelineScene, opts: &SvgOptions) -> String {
    to_svg_a11y_with(ts, &A11y::default(), opts)
}

/// [`to_svg_with`] plus mermaid's `accTitle` / `accDescr` (see
/// [`A11y`]). Without an `accTitle` the timeline's own title is the
/// accessible name, then "Timeline".
pub(crate) fn to_svg_a11y_with(ts: &TimelineScene, a11y: &A11y, opts: &SvgOptions) -> String {
    let (title, desc) = a11y.resolve(ts.title.as_deref().unwrap_or("Timeline"));
    let mut s = String::new();
    svg_open_a11y(&mut s, ts.width, ts.height, FONT, &title, desc, opts);

    // Title.
    if let Some(title) = &ts.title {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
            ts.title_pos.0,
            ts.title_pos.1,
            TITLE_FONT,
            TEXT_COLOR,
            escape(title)
        ));
    }

    for sec in &ts.sections {
        // A section header band is only drawn for *named* sections — the
        // implicit leading section (periods before the first `section`)
        // has an empty name and sits directly on the axis, as in mermaid.
        if !sec.name.is_empty() {
            s.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
                 fill=\"{}\"/>\n",
                sec.x, sec.y, sec.w, SECTION_H, sec.color
            ));
            // White or dark ink by contrast with the band's accent.
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                 font-weight=\"bold\" fill=\"{}\">{}</text>\n",
                sec.x + sec.w / 2.0,
                sec.y + SECTION_H / 2.0,
                crate::style::text_on(sec.color),
                escape(&sec.name)
            ));
        }

        // Horizontal axis line.
        s.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" \
             stroke-width=\"2\"/>\n",
            sec.x,
            sec.axis_y,
            sec.x + sec.w,
            sec.axis_y,
            AXIS_COLOR
        ));

        for p in &sec.periods {
            // Dashed task line in the gaps between events, then the
            // tick on top of the axis (white ring keeps it crisp where
            // it sits on the line).
            for &(y0, y1) in &p.line_segments {
                s.push_str(&format!(
                    "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" \
                     stroke-width=\"1.2\" stroke-dasharray=\"4,4\"/>\n",
                    p.cx, y0, p.cx, y1, TASK_LINE_COLOR
                ));
            }
            s.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\" stroke=\"#ffffff\" \
                 stroke-width=\"1.5\"/>\n",
                p.cx, sec.axis_y, TICK_R, sec.color
            ));

            // Period label (bold, above the axis) and its events (below).
            svg_text_lines(
                &mut s,
                p.cx,
                p.label_y,
                TEXT_COLOR,
                &p.label,
                " font-weight=\"bold\"",
                false,
            );
            for (text, y) in &p.events {
                svg_text_multiline(&mut s, p.cx, *y, TEXT_COLOR, text);
            }
        }
    }

    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn timeline(src: &str) -> Timeline {
        match parse_document(src).unwrap() {
            Document::Timeline(t) => t,
            _ => panic!("expected a timeline"),
        }
    }

    #[test]
    fn parses_period_and_events_inline() {
        let t = timeline("timeline\n  2002 : LinkedIn\n  2004 : Facebook : Google\n");
        assert_eq!(t.direction, TimelineDirection::LeftRight);
        assert_eq!(t.sections.len(), 1);
        let periods = &t.sections[0].periods;
        assert_eq!(periods.len(), 2);
        assert_eq!(periods[0].period, "2002");
        assert_eq!(periods[0].events, vec!["LinkedIn"]);
        assert_eq!(periods[1].period, "2004");
        assert_eq!(periods[1].events, vec!["Facebook", "Google"]);
    }

    #[test]
    fn continuation_line_extends_previous_period() {
        let t = timeline("timeline\n  2004 : Facebook\n  : Google\n");
        let periods = &t.sections[0].periods;
        assert_eq!(periods.len(), 1);
        assert_eq!(periods[0].events, vec!["Facebook", "Google"]);
    }

    #[test]
    fn colon_without_whitespace_is_literal() {
        let t = timeline("timeline\n  1998 : https://example.com : A:B\n");
        let periods = &t.sections[0].periods;
        assert_eq!(periods[0].events, vec!["https://example.com", "A:B"]);
    }

    #[test]
    fn title_section_and_direction_are_parsed() {
        let t = timeline(
            "timeline TD\n  title History\n  section Web\n    1998 : WWW\n  section Social\n    2004 : FB\n",
        );
        assert_eq!(t.direction, TimelineDirection::TopDown);
        assert_eq!(t.title.as_deref(), Some("History"));
        assert_eq!(t.sections.len(), 2);
        assert_eq!(t.sections[0].name, "Web");
        assert_eq!(t.sections[1].name, "Social");
    }

    #[test]
    fn indented_header_keeps_direction() {
        let t = timeline("   timeline TD\n  section A\n    2002 : X\n");
        assert_eq!(t.direction, TimelineDirection::TopDown);
    }

    #[test]
    fn breaks_are_folded_and_entities_stay_literal() {
        let t = timeline("timeline\n  2002 : First<br>Second : #35;tag\n");
        let events = &t.sections[0].periods[0].events;
        assert_eq!(events[0], "First\nSecond");
        assert_eq!(events[1], "#35;tag");
    }

    #[test]
    fn scene_fits_everything_inside_the_canvas() {
        let js = scene(&timeline(
            "timeline\n  title T\n  section A\n    2002 : LinkedIn\n    2004 : Facebook : Google\n  \
             section B\n    2006 : Twitter\n",
        ));
        assert!(js.width > 0.0 && js.height > 0.0);
        assert_eq!(js.sections.len(), 2);
        for sec in &js.sections {
            assert!(sec.x >= -0.01 && sec.x + sec.w <= js.width + 0.01);
            for p in &sec.periods {
                assert!(p.cx >= -0.01 && p.cx <= js.width + 0.01);
                assert!(p.line_bottom <= js.height + 0.01);
                for (_, y) in &p.events {
                    assert!(*y <= js.height + 0.01);
                }
            }
        }
    }

    #[test]
    fn to_svg_draws_sections_periods_and_events() {
        let svg = to_svg(&scene(&timeline(
            "timeline\n  title Trip\n  section Go\n    2002 : LinkedIn\n",
        )));
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains(">Trip<"), "title");
        assert!(svg.contains(">Go<"), "section name");
        assert!(svg.contains(">2002<"), "period label");
        assert!(svg.contains(">LinkedIn<"), "event");
        assert!(svg.contains("stroke-dasharray=\"4,4\""), "task line");
    }

    #[test]
    fn sectionless_timeline_has_no_header_band() {
        let svg = to_svg(&scene(&timeline("timeline\n  2002 : LinkedIn\n")));
        // Only the `svg_open` background rect — no colored section band.
        assert_eq!(
            svg.matches("<rect").count(),
            1,
            "no extra section band: {svg}"
        );
        assert!(svg.contains(">2002<"), "period label still drawn");
        assert!(svg.contains(">LinkedIn<"), "event still drawn");
    }

    /// Report Q5: section band labels hard-coded white on the band's
    /// accent (green 3.2:1, amber 2.6:1).
    #[test]
    fn section_labels_read_on_every_accent() {
        let n = crate::style::ACCENTS.len();
        let mut src = String::from("timeline\n");
        for i in 0..n {
            src.push_str(&format!("section Sec{i}\n  {} : ev{i}\n", 2000 + i));
        }
        let svg = crate::render_svg(&src).unwrap();
        for i in 0..n {
            let accent = crate::style::accent(i);
            let ink = crate::style::assert_text_reads(&svg, &format!("Sec{i}"), accent);
            assert_eq!(ink, crate::style::text_on(accent));
        }
    }

    fn fixture() -> Timeline {
        timeline(include_str!("../examples/timeline.mmd"))
    }

    /// Report timeline connector-strikes-through-text: the dashed task
    /// line ran through every event label. It is now drawn only in the
    /// gaps between texts, clear of every glyph.
    #[test]
    fn task_lines_never_cross_event_text() {
        for src in [
            include_str!("../examples/timeline.mmd").to_string(),
            "timeline TD\n  section S\n    2004 : A<br>two lines : B\n    2005 : C\n".to_string(),
        ] {
            let ts = scene(&timeline(&src));
            for sec in &ts.sections {
                for p in &sec.periods {
                    for (text, cy) in &p.events {
                        let (g0, g1) = glyph_span(*cy, line_count(text));
                        for &(y0, y1) in &p.line_segments {
                            assert!(y0 < y1);
                            assert!(y1 <= g0 || y0 >= g1, "segment crosses {text:?}");
                        }
                    }
                    if let Some(&(y0, _)) = p.line_segments.first() {
                        assert!(y0 > sec.axis_y + TICK_R, "starts under the tick");
                    }
                    if let Some((_, cy)) = p.events.last() {
                        assert!(p.line_bottom < *cy, "no tail below the last event");
                    }
                    for &(_, y1) in &p.line_segments {
                        assert!(y1 <= p.line_bottom + 1e-9);
                    }
                }
            }
        }
        // Every event with a predecessor gap gets a visible dash.
        let ts = scene(&fixture());
        let p = &ts.sections[0].periods[1]; // 2004 : Facebook : Google
        assert_eq!(p.line_segments.len(), 2);
    }

    /// Report timeline connector-over-dot: the dashed line was painted
    /// over the tick dot. Lines first, then the tick (with a ring).
    #[test]
    fn tick_is_painted_over_its_task_line() {
        let svg = to_svg(&scene(&timeline("timeline\n  2002 : LinkedIn : Two\n")));
        let line = svg.find("stroke-dasharray").unwrap();
        let tick = svg.find("<circle").unwrap();
        assert!(tick > line, "tick after the task line");
        assert!(svg.contains(&format!("r=\"{TICK_R:.1}\"")));
        assert!(svg[tick..].starts_with("<circle") && svg[tick..].contains("stroke=\"#ffffff\""));
    }

    /// Report timeline no-period-event-hierarchy: periods and events
    /// were typographically identical. Periods are bold now.
    #[test]
    fn periods_are_bold_and_events_regular() {
        let svg = to_svg(&scene(&fixture()));
        for period in ["2002", "2004", "2010"] {
            let end = svg.find(&format!(">{period}</text>")).unwrap();
            let tag = &svg[svg[..end].rfind("<text").unwrap()..end];
            assert!(tag.contains("font-weight=\"bold\""), "{period}: {tag}");
        }
        for event in ["LinkedIn", "Pinterest"] {
            let end = svg.find(&format!(">{event}</text>")).unwrap();
            let tag = &svg[svg[..end].rfind("<text").unwrap()..end];
            assert!(!tag.contains("bold"), "{event}: {tag}");
        }
        // Bold labels are measured as bold: a long period widens its
        // column by the bold factor.
        let p = "September 2023";
        let ts = scene(&timeline(&format!("timeline\n  {p} : x\n")));
        let w = ts.sections[0].w;
        assert!(w >= text_width(p) * 13.0 / TEXT_CALIBRATION * 1.06 + 2.0 * COL_PAD - 1e-9);
    }

    /// Report timeline faint-axis: #c2c8dc at 1.67:1 nearly vanished.
    #[test]
    fn axis_is_visible_and_task_lines_stay_pale() {
        let axis = crate::style::contrast_ratio(AXIS_COLOR, "#ffffff").unwrap();
        assert!(axis >= 3.0, "axis {axis:.2}:1");
        let task = crate::style::contrast_ratio(TASK_LINE_COLOR, "#ffffff").unwrap();
        assert!(task < axis, "task lines recede behind the axis");
        let svg = to_svg(&scene(&fixture()));
        assert!(svg.contains(&format!("stroke=\"{AXIS_COLOR}\" stroke-width=\"2\"")));
    }

    /// Report timeline wide section names: a name wider than its
    /// columns overflowed the band (white on white, off canvas).
    #[test]
    fn wide_section_names_fit_their_band() {
        let name = "Mobile and smartphone revolution era";
        for dir in ["", " TD"] {
            let ts = scene(&timeline(&format!(
                "timeline{dir}\n  section {name}\n    2007 : iPhone\n  section Web\n    1998 : WWW\n"
            )));
            let sec = &ts.sections[0];
            let need = text_width(name) * 13.0 / TEXT_CALIBRATION * 1.06 + 2.0 * COL_PAD;
            assert!(sec.w >= need - 1e-9, "{dir}: band {} < {need}", sec.w);
            assert!(sec.x >= 0.0 && sec.x + sec.w <= ts.width);
            // The single column spreads to the band, centred under it.
            assert!((sec.periods[0].cx - (sec.x + sec.w / 2.0)).abs() < 1e-9);
            if dir.is_empty() {
                assert!(
                    (ts.sections[1].x - (sec.x + sec.w)).abs() < 1e-9,
                    "LR touches"
                );
            }
        }
    }

    /// Report timeline long titles: measured as 14px regular text but
    /// drawn 17px bold, clipped at both edges; the content stayed
    /// left-aligned under the centred title.
    #[test]
    fn long_titles_fit_and_center_the_content() {
        let t = "A very long title for a tiny timeline that is much wider";
        for dir in ["", " TD"] {
            let ts = scene(&timeline(&format!(
                "timeline{dir}\n  title {t}\n  2007 : x\n"
            )));
            let need = text_width(t) * 17.0 / TEXT_CALIBRATION * 1.06 + 2.0 * PAD;
            assert!(ts.width >= need - 1e-9, "{dir}");
            let sec = &ts.sections[0];
            let (left, right) = (sec.x, ts.width - (sec.x + sec.w));
            assert!((left - right).abs() < 1e-9, "{dir}: centred");
        }
    }

    /// Report timeline title-crowds-band: only ~8px between the title
    /// and the first band.
    #[test]
    fn title_keeps_a_gap_above_the_first_band() {
        let ts = scene(&fixture());
        let band_top = ts.sections.iter().map(|s| s.y).fold(f64::MAX, f64::min);
        assert!(band_top >= PAD + TITLE_H + TITLE_GAP - 1e-9);
        let untitled = scene(&timeline("timeline\n  section A\n    2002 : x\n"));
        assert_eq!(untitled.sections[0].y, PAD, "no title, no gap");
    }

    /// Flowmaid draws periods declared before the first `section`
    /// (mermaid 12 drops them): keep them, with neutral ticks — they
    /// have no band to explain an accent — and let the named sections
    /// take the palette from its start.
    #[test]
    fn periods_before_the_first_section_are_kept_with_neutral_ticks() {
        let ts = scene(&fixture());
        assert_eq!(ts.sections.len(), 2);
        let lead = &ts.sections[0];
        assert!(lead.name.is_empty());
        let labels: Vec<&str> = lead.periods.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels, ["2002", "2004", "2005"]);
        assert_eq!(lead.color, UNNAMED_COLOR);
        assert_eq!(ts.sections[1].color, accent(0), "first named band");
        let svg = to_svg(&ts);
        for t in ["2002", "LinkedIn", "Facebook", "Google", "YouTube"] {
            assert!(svg.contains(&format!(">{t}</text>")), "{t} drawn");
        }
        // Only the named section gets a band (+ the canvas rect).
        assert_eq!(svg.matches("<rect").count(), 2);
    }

    /// Report timeline long events: never wrapped, so one long event
    /// made a 657px column.
    #[test]
    fn long_events_and_periods_wrap() {
        let ev = "The first iPhone was announced by Steve Jobs at Macworld in San Francisco";
        let ts = scene(&timeline(&format!("timeline\n  2007 : {ev}\n  2008 : x\n")));
        let p = &ts.sections[0].periods[0];
        let (text, _) = &p.events[0];
        assert!(text.contains('\n'));
        assert_eq!(text.replace('\n', " "), ev, "no words lost");
        for line in text.lines() {
            assert!(text_width(line) * 13.0 / TEXT_CALIBRATION <= WRAP_W);
        }
        let col = ts.sections[0].periods[1].cx - p.cx;
        assert!(col <= (WRAP_W + 2.0 * COL_PAD + COL_MIN) / 2.0 + 1e-9);
        // The wrapped block fits the canvas height.
        let (text, cy) = &p.events[0];
        assert!(glyph_span(*cy, line_count(text)).1 <= ts.height);
        // A long period label wraps too, and the axis drops under it.
        let ts2 = scene(&timeline(
            "timeline\n  The very long first period of this timeline : x\n",
        ));
        let p2 = &ts2.sections[0].periods[0];
        assert!(p2.label.contains('\n'));
        let top = p2.label_y - (line_count(&p2.label) - 1.0) * LINE_H / 2.0 - LINE_H / 2.0;
        assert!(top >= PAD - 1e-9, "label stays on the canvas");
        assert!(p2.label_y + LINE_H / 2.0 <= ts2.sections[0].axis_y);
    }

    /// accTitle / accDescr were parsed and dropped.
    #[test]
    fn acc_statements_reach_the_svg() {
        let svg = crate::render_svg(
            "timeline\n  title History\n  accTitle: Social media history\n  \
             accDescr: Six platforms\n  2002 : LinkedIn\n",
        )
        .unwrap();
        assert!(svg.contains("aria-label=\"Social media history\""));
        assert!(svg.contains(">Six platforms</desc>"));
        assert!(svg.contains(">History</text>"), "visible title unchanged");
        let titled = crate::render_svg("timeline\n  title History\n  2002 : x\n").unwrap();
        assert!(titled.contains("aria-label=\"History\""));
        let plain = crate::render_svg("timeline\n  2002 : x\n").unwrap();
        assert!(plain.contains("aria-label=\"Timeline\""));
    }
}
