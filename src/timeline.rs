//! Timeline rendering: a horizontal axis of period columns with events
//! stacked beneath, and sections drawn as colored header bands.
//!
//! Mirrors mermaid's timeline structure: in `LR` (the default) sections
//! sit side by side and all periods share one horizontal axis; in `TD`
//! sections stack top-to-bottom, each with its own axis. Periods run
//! left-to-right as columns in either direction; a dashed "task" line
//! drops from each period's tick on the axis down through its events.
//!
//! Like `pie`/`journey` there is nothing draggable and no `route()`:
//! [`scene`] computes every coordinate and [`to_svg`] serialises it.

use crate::layout::{text_width, LINE_H};
use crate::model::{Timeline, TimelineDirection, TimelinePeriod};
use crate::scene::{escape, svg_open, svg_text_multiline, SvgOptions, TEXT_COLOR};
use crate::style::accent;

/// Canvas margin.
pub const PAD: f64 = 24.0;
/// Title band height (when a title is present).
pub const TITLE_H: f64 = 34.0;
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
/// Base font size.
pub const FONT: u32 = 13;
/// Axis line + task-line color.
pub const AXIS_COLOR: &str = "#c2c8dc";
/// Tick radius on the axis.
pub const TICK_R: f64 = 3.0;

/// Number of rendered lines in a label (after `<br>` folding).
fn line_count(s: &str) -> f64 {
    s.split('\n').count() as f64
}

/// Column width of one period: wide enough for its label and events.
fn period_w(p: &TimelinePeriod) -> f64 {
    let mut w = text_width(&p.period) + 2.0 * COL_PAD;
    for e in &p.events {
        w = w.max(text_width(e) + 2.0 * COL_PAD);
    }
    w.max(COL_MIN)
}

/// A colored header band for one section, plus its period columns.
#[derive(Debug, Clone)]
pub struct SectionGlyph {
    pub name: String,
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
#[derive(Debug, Clone)]
pub struct PeriodGlyph {
    pub label: String,
    /// Column centre X.
    pub cx: f64,
    /// Vertical centre of the period label (above the axis).
    pub label_y: f64,
    /// Dashed line bottom (top is the section's `axis_y`).
    pub line_bottom: f64,
    /// (event text, vertical centre) per event, top to bottom.
    pub events: Vec<(String, f64)>,
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

/// Lay out one section's period columns left-to-right starting at `left`,
/// given the section header top `header_y` and its `axis_y`. Returns the
/// period glyphs and the section's content width (sum of columns).
fn layout_section_periods(
    periods: &[TimelinePeriod],
    left: f64,
    axis_y: f64,
) -> (Vec<PeriodGlyph>, f64) {
    let mut glyphs = Vec::with_capacity(periods.len());
    let mut x = left;
    for p in periods {
        let w = period_w(p);
        let cx = x + w / 2.0;
        let label_y = axis_y - EVENT_GAP - line_count(&p.period) * LINE_H / 2.0;
        let mut events = Vec::with_capacity(p.events.len());
        let mut ey = axis_y + EVENT_GAP;
        for ev in &p.events {
            let h = line_count(ev) * LINE_H;
            events.push((ev.clone(), ey + h / 2.0));
            ey += h + EVENT_GAP;
        }
        let line_bottom = if p.events.is_empty() {
            axis_y
        } else {
            ey - EVENT_GAP
        };
        glyphs.push(PeriodGlyph {
            label: p.period.clone(),
            cx,
            label_y,
            line_bottom,
            events,
        });
        x += w;
    }
    (glyphs, x - left)
}

/// Compute all geometry.
pub fn scene(d: &Timeline) -> TimelineScene {
    match d.direction {
        TimelineDirection::LeftRight => layout_lr(d),
        TimelineDirection::TopDown => layout_td(d),
    }
}

/// `LR` layout: sections side by side, one shared horizontal axis.
fn layout_lr(d: &Timeline) -> TimelineScene {
    let has_title = d.title.is_some();
    let mut top = PAD;
    if has_title {
        top += TITLE_H;
    }
    // One axis shared by every section: below the tallest period label.
    let label_h_max = d
        .sections
        .iter()
        .flat_map(|s| s.periods.iter())
        .map(|p| line_count(&p.period) * LINE_H)
        .fold(LINE_H, f64::max);
    // Reserve a band only when at least one section is named; a section-less
    // timeline sits directly on the axis (no empty colored strip up top).
    let band_h = if d.sections.iter().any(|s| !s.name.is_empty()) {
        SECTION_H
    } else {
        0.0
    };
    let axis_y = top + band_h + GAP + label_h_max + EVENT_GAP;

    let mut sections = Vec::with_capacity(d.sections.len());
    let mut x = PAD;
    for (si, sec) in d.sections.iter().enumerate() {
        let (periods, w) = layout_section_periods(&sec.periods, x, axis_y);
        let w = if w > 0.0 { w } else { COL_MIN };
        sections.push(SectionGlyph {
            name: sec.name.clone(),
            color: accent(si),
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

    let deepest = sections
        .iter()
        .flat_map(|s| s.periods.iter())
        .map(|p| p.line_bottom)
        .fold(axis_y, f64::max);
    let height = deepest + PAD;

    let title_w = d.title.as_ref().map_or(0.0, |t| text_width(t) + 2.0 * PAD);
    let width = content_w.max(title_w);
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
    let has_title = d.title.is_some();
    let mut y = PAD;
    if has_title {
        y += TITLE_H;
    }

    let mut sections = Vec::with_capacity(d.sections.len());
    let mut max_w = 0.0f64;
    let mut y_cursor = y;
    for (si, sec) in d.sections.iter().enumerate() {
        let label_h_max = sec
            .periods
            .iter()
            .map(|p| line_count(&p.period) * LINE_H)
            .fold(LINE_H, f64::max);
        let band_h = if sec.name.is_empty() { 0.0 } else { SECTION_H };
        let axis_y = y_cursor + band_h + GAP + label_h_max + EVENT_GAP;
        let (periods, w) = layout_section_periods(&sec.periods, PAD, axis_y);
        let w = if w > 0.0 { w } else { COL_MIN };
        max_w = max_w.max(w);
        let deepest = periods
            .iter()
            .map(|p| p.line_bottom)
            .fold(axis_y, f64::max);
        sections.push(SectionGlyph {
            name: sec.name.clone(),
            color: accent(si),
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

    let title_w = d.title.as_ref().map_or(0.0, |t| text_width(t) + 2.0 * PAD);
    let width = content_w.max(title_w);
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
    let mut s = String::new();
    svg_open(&mut s, ts.width, ts.height, FONT, "Timeline", opts);

    // Title.
    if let Some(title) = &ts.title {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" font-size=\"17\" fill=\"{}\">{}</text>\n",
            ts.title_pos.0,
            ts.title_pos.1,
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
                sec.x,
                sec.y,
                sec.w,
                SECTION_H,
                sec.color
            ));
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                 font-weight=\"bold\" fill=\"#ffffff\">{}</text>\n",
                sec.x + sec.w / 2.0,
                sec.y + SECTION_H / 2.0,
                escape(&sec.name)
            ));
        }

        // Horizontal axis line.
        s.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" \
             stroke-width=\"1.5\"/>\n",
            sec.x,
            sec.axis_y,
            sec.x + sec.w,
            sec.axis_y,
            AXIS_COLOR
        ));

        for p in &sec.periods {
            // Tick on the axis + dashed task line down through the events.
            s.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\"/>\n",
                p.cx, sec.axis_y, TICK_R, sec.color
            ));
            if p.line_bottom > sec.axis_y {
                s.push_str(&format!(
                    "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" \
                     stroke-width=\"1.2\" stroke-dasharray=\"4,4\"/>\n",
                    p.cx, sec.axis_y, p.cx, p.line_bottom, AXIS_COLOR
                ));
            }

            // Period label (above the axis) and its events (below).
            svg_text_multiline(&mut s, p.cx, p.label_y, TEXT_COLOR, &p.label);
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
        assert_eq!(svg.matches("<rect").count(), 1, "no extra section band: {svg}");
        assert!(svg.contains(">2002<"), "period label still drawn");
        assert!(svg.contains(">LinkedIn<"), "event still drawn");
    }
}
