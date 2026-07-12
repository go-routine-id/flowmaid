//! Pie chart rendering: proportional slices drawn clockwise from
//! 12 o'clock, percentage labels inside the slices, and a color
//! legend on the right (with raw values when `showData` is set).
//!
//! Unlike the `er` / `class` modules there is no synthetic graph —
//! a pie has no layout problem — and no `route()`: nothing is
//! draggable. [`scene`] computes all geometry a GUI painter needs
//! (canvas, circle, per-slice angles, legend rows, title anchor) and
//! [`to_svg`] serialises it; slice `i` uses [`crate::style::accent`]
//! with index `i` in both, so painters match the SVG exactly.

use crate::layout::text_width;
use crate::model::PieChart;
use crate::scene::{escape, svg_open, EDGE_COLOR, TEXT_COLOR};
use std::f64::consts::TAU;

/// Pie radius in pixels.
pub const RADIUS: f64 = 110.0;
/// Canvas padding in pixels.
pub const PAD: f64 = 20.0;
/// Vertical room reserved for the title, when present.
pub const TITLE_H: f64 = 34.0;
/// Legend row height in pixels.
pub const LEGEND_ROW_H: f64 = 24.0;
/// Legend color-swatch side length.
pub const SWATCH: f64 = 12.0;
/// Gap between the pie circle and the legend column.
pub const LEGEND_GAP: f64 = 28.0;
/// Percentage labels sit at this fraction of the radius.
pub const LABEL_R: f64 = 0.62;
/// Slices thinner than this fraction get no percentage label
/// (the text would overflow the wedge); they stay in the legend.
pub const MIN_LABEL_FRAC: f64 = 0.04;

/// Geometry of one slice. Angles are radians measured from
/// 12 o'clock, increasing clockwise; slices keep insertion (parse)
/// order. Color: `crate::style::accent(i)` for slice `i`.
#[derive(Debug, Clone)]
pub struct Slice {
    pub label: String,
    pub value: f64,
    /// Fraction of the total; 0 when the total itself is 0.
    pub frac: f64,
    pub start_angle: f64,
    pub end_angle: f64,
}

/// One legend row, index-aligned with [`PieScene::slices`] (same
/// order, same accent color) — zero-value slices keep their row.
#[derive(Debug, Clone)]
pub struct LegendRow {
    /// Left edge of the color swatch.
    pub x: f64,
    /// Vertical centre of the row (swatch and text).
    pub y: f64,
    /// Display text: the label, plus ` [value]` under `showData`.
    pub text: String,
}

/// Everything needed to draw a pie chart: canvas size, circle
/// centre + radius, per-slice angles, legend rows, and the title
/// with its centre anchor.
#[derive(Debug, Clone)]
pub struct PieScene {
    pub width: f64,
    pub height: f64,
    /// Circle centre.
    pub cx: f64,
    pub cy: f64,
    /// Circle radius.
    pub r: f64,
    pub title: Option<String>,
    /// Title anchor (centre of the title text); only meaningful
    /// when `title` is `Some`.
    pub title_pos: (f64, f64),
    pub show_data: bool,
    pub slices: Vec<Slice>,
    pub legend: Vec<LegendRow>,
}

/// Compute all pie geometry. Never yields NaN/inf: a zero total
/// makes every `frac` 0 (drawn as an empty outline by [`to_svg`]).
pub fn scene(d: &PieChart) -> PieScene {
    let r = RADIUS;
    let total: f64 = d.slices.iter().map(|s| s.value).sum();
    let mut slices = Vec::with_capacity(d.slices.len());
    let mut cum = 0.0f64;
    for s in &d.slices {
        let frac = if total > 0.0 { s.value / total } else { 0.0 };
        let start_angle = cum * TAU;
        cum += frac;
        slices.push(Slice {
            label: s.label.clone(),
            value: s.value,
            frac,
            start_angle,
            end_angle: cum * TAU,
        });
    }

    let texts: Vec<String> = d
        .slices
        .iter()
        .map(|s| {
            if d.show_data {
                // `{}` (Display) keeps values short: 5.0 -> "5".
                format!("{} [{}]", s.label, s.value)
            } else {
                s.label.clone()
            }
        })
        .collect();
    let text_w = texts.iter().map(|t| text_width(t)).fold(0.0, f64::max);
    let legend_w = if texts.is_empty() {
        0.0
    } else {
        SWATCH + 8.0 + text_w
    };

    let top = PAD + if d.title.is_some() { TITLE_H } else { 0.0 };
    let legend_h = texts.len() as f64 * LEGEND_ROW_H;
    let content_h = (2.0 * r).max(legend_h);
    let height = top + content_h + PAD;
    // pad | pie | gap | legend | pad (no legend column when empty).
    let mut width = PAD + 2.0 * r + PAD;
    if legend_w > 0.0 {
        width += LEGEND_GAP + legend_w;
    }
    if let Some(t) = &d.title {
        // The title renders at font-size 16 over a 13px base; scale
        // the metric so a long title still fits the canvas.
        width = width.max(2.0 * PAD + text_width(t) * 16.0 / 13.0);
    }

    let cx = PAD + r;
    let cy = top + content_h / 2.0;
    let lx = PAD + 2.0 * r + LEGEND_GAP;
    let ly0 = top + (content_h - legend_h) / 2.0;
    let legend = texts
        .into_iter()
        .enumerate()
        .map(|(i, text)| LegendRow {
            x: lx,
            y: ly0 + i as f64 * LEGEND_ROW_H + LEGEND_ROW_H / 2.0,
            text,
        })
        .collect();

    PieScene {
        width,
        height,
        cx,
        cy,
        r,
        title: d.title.clone(),
        title_pos: (width / 2.0, PAD + TITLE_H / 2.0),
        show_data: d.show_data,
        slices,
        legend,
    }
}

/// Point on the circle around `(cx, cy)` at angle `a` — radians
/// from 12 o'clock, clockwise (the y-axis points down in SVG).
fn polar(cx: f64, cy: f64, rad: f64, a: f64) -> (f64, f64) {
    (cx + rad * a.sin(), cy - rad * a.cos())
}

/// Serialise a pie scene to SVG.
pub fn to_svg(ps: &PieScene) -> String {
    let mut s = String::new();
    svg_open(&mut s, ps.width, ps.height, 13);

    if let Some(t) = &ps.title {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" font-size=\"16\" fill=\"{}\">{}</text>\n",
            ps.title_pos.0,
            ps.title_pos.1,
            TEXT_COLOR,
            escape(t)
        ));
    }

    // Slices. Zero-value slices are skipped (nothing to draw; their
    // legend row remains). A ~100% slice becomes a <circle>: one SVG
    // arc cannot draw a full 360° sweep (its endpoints coincide).
    let total_frac: f64 = ps.slices.iter().map(|sl| sl.frac).sum();
    if total_frac <= f64::EPSILON {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"none\" \
             stroke=\"{}\" stroke-width=\"1.6\"/>\n",
            ps.cx, ps.cy, ps.r, EDGE_COLOR
        ));
    }
    for (i, sl) in ps.slices.iter().enumerate() {
        if sl.frac <= 0.0 {
            continue;
        }
        let color = crate::style::accent(i);
        if sl.frac >= 1.0 - 1e-9 {
            s.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\" \
                 stroke=\"#ffffff\" stroke-width=\"1.5\"/>\n",
                ps.cx, ps.cy, ps.r, color
            ));
        } else {
            let a = polar(ps.cx, ps.cy, ps.r, sl.start_angle);
            let b = polar(ps.cx, ps.cy, ps.r, sl.end_angle);
            let large = i32::from(sl.frac > 0.5);
            s.push_str(&format!(
                "<path d=\"M {:.1} {:.1} L {:.1} {:.1} A {r:.1} {r:.1} 0 {} 1 {:.1} {:.1} Z\" \
                 fill=\"{}\" stroke=\"#ffffff\" stroke-width=\"1.5\"/>\n",
                ps.cx,
                ps.cy,
                a.0,
                a.1,
                large,
                b.0,
                b.1,
                color,
                r = ps.r
            ));
        }
    }

    // Percentage labels at the slice mid-angle, inside the wedge.
    for sl in &ps.slices {
        if sl.frac < MIN_LABEL_FRAC {
            continue;
        }
        let mid = (sl.start_angle + sl.end_angle) / 2.0;
        let p = polar(ps.cx, ps.cy, ps.r * LABEL_R, mid);
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" fill=\"#ffffff\">{:.0}%</text>\n",
            p.0,
            p.1,
            sl.frac * 100.0
        ));
    }

    // Legend: color swatch + label (+ raw value under showData).
    for (i, row) in ps.legend.iter().enumerate() {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{sw:.0}\" height=\"{sw:.0}\" rx=\"2\" \
             fill=\"{}\"/>\n",
            row.x,
            row.y - SWATCH / 2.0,
            crate::style::accent(i),
            sw = SWATCH
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" fill=\"{}\">{}</text>\n",
            row.x + SWATCH + 8.0,
            row.y,
            TEXT_COLOR,
            escape(&row.text)
        ));
    }

    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::{parse, parse_document};

    fn pie(src: &str) -> PieChart {
        match parse_document(src).unwrap() {
            Document::Pie(d) => d,
            other => panic!("expected a pie chart, got {:?}", other),
        }
    }

    fn fixture() -> PieChart {
        pie(include_str!("../examples/pie.mmd"))
    }

    fn svg_attr(s: &str, name: &str) -> f64 {
        let pat = format!("{name}=\"");
        let i = s.find(&pat).unwrap() + pat.len();
        s[i..i + s[i..].find('"').unwrap()].parse().unwrap()
    }

    // ---------------------------- parse ----------------------------

    #[test]
    fn all_header_forms_parse() {
        let d = pie("pie\n\"a\" : 1");
        assert_eq!(d.title, None);
        assert!(!d.show_data);
        let d = pie("pie showData\n\"a\" : 1");
        assert!(d.show_data && d.title.is_none());
        let d = pie("pie title Key elements\n\"a\" : 1");
        assert_eq!(d.title.as_deref(), Some("Key elements"));
        assert!(!d.show_data);
        let d = pie("pie showData title Key elements\n\"a\" : 1");
        assert!(d.show_data);
        assert_eq!(d.title.as_deref(), Some("Key elements"));
    }

    #[test]
    fn title_line_comments_and_blanks() {
        let d = pie("pie\n%% a comment\n\ntitle My Chart\n\"a\" : 1.5\n%% tail\n\n\"b\" : 2\n");
        assert_eq!(d.title.as_deref(), Some("My Chart"));
        assert_eq!(d.slices.len(), 2);
        assert_eq!(d.slices[0].label, "a");
        assert_eq!(d.slices[0].value, 1.5);
        assert_eq!(d.slices[1].value, 2.0);
    }

    #[test]
    fn duplicate_label_keeps_position_last_value_wins() {
        let d = pie("pie\n\"a\" : 1\n\"b\" : 2\n\"a\" : 5");
        assert_eq!(d.slices.len(), 2);
        assert_eq!(d.slices[0].label, "a");
        assert_eq!(d.slices[0].value, 5.0, "last value wins");
        assert_eq!(d.slices[1].label, "b");
    }

    #[test]
    fn errors_carry_line_numbers() {
        for (src, needle) in [
            ("pie\nno quotes : 1", "quoted label"),
            ("pie\n\"open : 1", "never closed"),
            ("pie\n\"a\" 1", "expected ':'"),
            ("pie\n\"a\" : -3", "non-negative"),
            ("pie\n\"a\" : inf", "non-negative"),
            ("pie\n\"a\" : NaN", "non-negative"),
            ("pie\n\"a\" : twelve", "invalid pie value"),
            ("pie\ntitle", "title needs text"),
        ] {
            let e = parse_document(src).unwrap_err();
            assert_eq!(e.line, 2, "src: {src}");
            assert!(e.message.contains(needle), "src: {src}, got: {}", e.message);
        }
        // Junk after the header errors on line 1.
        let e = parse_document("pie whatever\n\"a\" : 1").unwrap_err();
        assert_eq!(e.line, 1);
        // The flowchart-only `parse` points at the right entry point.
        let e = parse("pie\n\"a\" : 1").unwrap_err();
        assert!(e.message.contains("parse_document"), "{}", e.message);
    }

    // ---------------------------- scene ----------------------------

    #[test]
    fn fractions_sum_to_one_and_angles_are_clockwise() {
        let ps = scene(&pie("pie\n\"a\" : 30\n\"b\" : 50\n\"c\" : 20"));
        let sum: f64 = ps.slices.iter().map(|s| s.frac).sum();
        assert!((sum - 1.0).abs() < 1e-9, "fracs sum to 1, got {sum}");
        assert_eq!(ps.slices[0].start_angle, 0.0, "first slice starts at 12 o'clock");
        for w in ps.slices.windows(2) {
            assert_eq!(w[0].end_angle, w[1].start_angle, "slices are contiguous");
        }
        let last = ps.slices.last().unwrap();
        assert!((last.end_angle - std::f64::consts::TAU).abs() < 1e-9);
    }

    #[test]
    fn single_slice_renders_a_full_circle() {
        let svg = to_svg(&scene(&pie("pie\n\"only\" : 7")));
        assert!(svg.contains("<circle"), "100% slice needs a <circle>");
        assert!(!svg.contains("<path"), "no degenerate 360-degree arc");
        assert!(svg.contains(">100%</text>"));
        assert!(svg.contains(crate::style::accent(0)));
    }

    #[test]
    fn zero_total_draws_outline_without_nan() {
        let svg = to_svg(&scene(&pie("pie title Empty\n\"a\" : 0\n\"b\" : 0")));
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        assert!(svg.contains("fill=\"none\""), "empty outline circle");
        assert!(!svg.contains("%</text>"), "no percentage labels");
        // Both labels keep their legend rows.
        assert!(svg.contains(">a</text>") && svg.contains(">b</text>"));
    }

    #[test]
    fn zero_value_slice_is_legend_only_and_colors_stay_aligned() {
        let svg = to_svg(&scene(&pie("pie\n\"big\" : 9\n\"none\" : 0\n\"small\" : 1")));
        assert_eq!(
            svg.lines().filter(|l| l.starts_with("<path")).count(),
            2,
            "only the two non-zero slices are drawn"
        );
        assert!(svg.contains(">none</text>"), "zero slice keeps its legend row");
        // `small` is slice 2: its wedge must use accent(2), not shift
        // down because slice 1 was skipped.
        assert!(svg.contains(&format!("fill=\"{}\" stroke=\"#ffffff\"", crate::style::accent(2))));
    }

    #[test]
    fn many_slices_wrap_the_palette() {
        let rows: String = (0..10).map(|i| format!("\"s{i}\" : {}\n", i + 1)).collect();
        let ps = scene(&pie(&format!("pie\n{rows}")));
        assert_eq!(ps.legend.len(), 10);
        let svg = to_svg(&ps);
        assert!(!svg.contains("NaN"));
        // 10 legend swatches drawn, colors wrapping the palette.
        assert_eq!(svg.matches("rx=\"2\"").count(), 10);
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn show_data_appends_values_to_the_legend() {
        let svg = to_svg(&scene(&pie("pie showData\n\"Cats\" : 4.25\n\"Dogs\" : 6")));
        assert!(svg.contains(">Cats [4.25]</text>"));
        assert!(svg.contains(">Dogs [6]</text>"), "whole values print without .0");
        let plain = to_svg(&scene(&pie("pie\n\"Cats\" : 4.25")));
        assert!(plain.contains(">Cats</text>") && !plain.contains('['));
    }

    #[test]
    fn all_text_stays_inside_the_canvas() {
        let svg = to_svg(&scene(&pie(
            "pie showData title A Rather Long Pie Chart Title That Must Fit\n\
             \"a very long slice label indeed\" : 60.25\n\
             \"short\" : 39.75",
        )));
        let w = svg_attr(&svg, "width");
        let h = svg_attr(&svg, "height");
        for line in svg.lines().filter(|l| l.starts_with("<text")) {
            let x = svg_attr(line, "x");
            let y = svg_attr(line, "y");
            let inner = &line[line.find('>').unwrap() + 1..line.rfind("</text>").unwrap()];
            let tw = text_width(inner);
            let (lo, hi) = if line.contains("text-anchor=\"middle\"") {
                (x - tw / 2.0, x + tw / 2.0)
            } else {
                (x, x + tw)
            };
            assert!(lo >= -1.0 && hi <= w + 1.0, "text {inner:?} [{lo:.0}..{hi:.0}] outside width {w}");
            assert!((0.0..=h).contains(&y), "text y {y} outside height {h}");
        }
    }

    /// End-to-end showcase guarding the whole feature set as one
    /// interacting whole: header showData, standalone title, six
    /// slices including a tiny (<4%) one, comments, and blank lines.
    #[test]
    fn pie_showcase_parses_and_renders() {
        let d = fixture();
        assert_eq!(d.title.as_deref(), Some("Key elements in Product X"));
        assert!(d.show_data);
        assert_eq!(d.slices.len(), 6);
        assert_eq!(d.slices[0].label, "Calcium");

        let ps = scene(&d);
        let sum: f64 = ps.slices.iter().map(|s| s.frac).sum();
        assert!((sum - 1.0).abs() < 1e-9);
        assert_eq!(
            ps.slices.iter().filter(|s| s.frac < MIN_LABEL_FRAC).count(),
            1,
            "exactly one tiny slice"
        );

        let svg = to_svg(&ps);
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        assert!(svg.contains("Key elements in Product X"));
        assert!(svg.contains(">Calcium [42.96]</text>"), "showData legend value");
        assert!(svg.contains(">Iron [6]</text>"));
        // All 6 slices are non-zero wedges; the tiny one still draws
        // but skips its percentage label.
        assert_eq!(svg.lines().filter(|l| l.starts_with("<path")).count(), 6);
        assert_eq!(svg.matches("%</text>").count(), 5);
        // Full text containment on the showcase, too.
        let w = svg_attr(&svg, "width");
        let h = svg_attr(&svg, "height");
        for line in svg.lines().filter(|l| l.starts_with("<text")) {
            let x = svg_attr(line, "x");
            let y = svg_attr(line, "y");
            let inner = &line[line.find('>').unwrap() + 1..line.rfind("</text>").unwrap()];
            let tw = text_width(inner);
            let (lo, hi) = if line.contains("text-anchor=\"middle\"") {
                (x - tw / 2.0, x + tw / 2.0)
            } else {
                (x, x + tw)
            };
            assert!(lo >= -1.0 && hi <= w + 1.0, "text {inner:?} outside width {w}");
            assert!((0.0..=h).contains(&y), "text y {y} outside height {h}");
        }
        // render_svg dispatches to the same output.
        assert_eq!(crate::render_svg(include_str!("../examples/pie.mmd")).unwrap(), svg);
    }
}
