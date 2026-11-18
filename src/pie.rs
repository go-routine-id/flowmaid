//! Pie chart rendering: proportional slices drawn clockwise from
//! 12 o'clock, percentage labels inside the slices, and a color
//! legend on the right (with raw values when `showData` is set).
//!
//! Unlike the `er` / `class` modules there is no synthetic graph —
//! a pie has no layout problem — and no `route()`: nothing is
//! draggable. [`scene`] computes all geometry a GUI painter needs
//! (canvas, circle, per-slice angles, percentage labels, legend rows,
//! title anchor) and [`to_svg`] serialises it; slice `i` uses
//! [`slice_color`] (and, past the sixteenth slice, [`slice_hatch`])
//! with index `i` in both, so painters match the SVG exactly.
//!
//! Every slice of at least 1% gets a percentage label, as in mermaid.
//! A label sits inside its wedge when its text box fits there (tested
//! against both radial edges and the rim); a slice too thin for that
//! gets its label just outside the rim on a short leader line instead
//! of text spilling over the separators.

use crate::layout::{text_width, TEXT_CALIBRATION};
use crate::model::PieChart;
use crate::scene::{escape, svg_open_a11y, A11y, SvgOptions};
use std::f64::consts::{FRAC_PI_2, TAU};

/// Pie radius in pixels.
pub const RADIUS: f64 = 130.0;
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
/// Percentage labels start at this fraction of the radius; a label
/// that does not fit its wedge there slides outward (where the wedge
/// is wider) up to [`LABEL_R_MAX`]. (0.32 and earlier: every label sat
/// at exactly 0.62 of the radius.)
pub const LABEL_R: f64 = 0.72;
/// Outermost fraction of the radius an inside label may slide to
/// before it moves outside the rim.
pub const LABEL_R_MAX: f64 = 0.86;
/// Slices thinner than this fraction get no percentage label — they
/// stay in the legend. Mermaid's rule: every slice of at least 1%
/// is labelled.
pub const MIN_LABEL_FRAC: f64 = 0.01;
/// Width of the white separators between wedges (thinner next to a
/// sliver, so the sliver keeps some colour).
pub const SEPARATOR_W: f64 = 1.5;
/// Distance from the rim to the elbow of an outside label's leader.
pub const LEADER_OUT: f64 = 12.0;
/// Minimum vertical pitch between two outside labels on one side.
pub const OUTSIDE_ROW_H: f64 = 15.0;
/// Base font size (legend and percentage labels).
const FONT: f64 = 13.0;
/// Title font size.
const TITLE_FONT: f64 = 16.0;
/// Width factor of bold text over regular (as [`text_width`] counts
/// `<b>` runs).
const BOLD: f64 = 1.06;
/// Half the height of a percentage label's text box (digits and `%`
/// at [`FONT`]), for the fit test.
const LABEL_HALF_H: f64 = 5.0;

/// Fill color of slice `i` (and its legend swatch): the sixteen-colour
/// [`crate::style::accent_wide`] cycle, so a ninth slice no longer
/// repeats the first one's colour. It matches [`crate::style::accent`]
/// in the first half of each turn of the cycle (slices 0–7, again 16–23,
/// and so on); slices 8–15 (24–31, …) take the deep shades where
/// `accent` would start over.
pub fn slice_color(i: usize) -> &'static str {
    crate::style::accent_wide(i)
}

/// Slices per turn of the [`slice_color`] cycle: the seventeenth slice
/// takes the first one's colour again (and on a full pie sits beside
/// it), so from there on [`slice_hatch`] adds a second cue.
const COLOR_TURN: usize = 16;

/// Stripe angles of [`slice_hatch`], one per later turn of the colour
/// cycle (the fifth turn starts over).
const HATCH_ANGLES: [f64; 4] = [45.0, 135.0, 0.0, 90.0];

/// Width (px) of a [`slice_hatch`] stripe.
pub const HATCH_W: f64 = 2.5;
/// Distance (px) from one [`slice_hatch`] stripe to the next.
pub const HATCH_PITCH: f64 = 6.0;
/// Opacity of the [`slice_hatch`] stripes, drawn in the canvas colour:
/// strong enough to read, light enough to keep the slice's hue.
pub const HATCH_OPACITY: f64 = 0.55;

/// The second cue of slice `i` (wedge and legend swatch alike) where
/// its colour repeats: `None` for the first sixteen slices, which
/// [`slice_color`] alone tells apart; past them, hatching of
/// [`HATCH_W`]-wide stripes every [`HATCH_PITCH`] px in the canvas
/// colour at [`HATCH_OPACITY`] over the fill, at this angle — degrees,
/// clockwise, of stripes that would otherwise run vertically: 45 for
/// slices 17–32, 135 for 33–48, 0 for 49–64, 90 for 65–80, then 45
/// again. So slice 17 no longer reads as part of slice 1 beside it, and
/// its legend swatch no longer matches slice 1's.
pub fn slice_hatch(i: usize) -> Option<f64> {
    let turn = i / COLOR_TURN;
    (turn > 0).then(|| HATCH_ANGLES[(turn - 1) % HATCH_ANGLES.len()])
}

/// A short, content-derived suffix for this chart's hatching ids — see
/// [`crate::scene::DefsKey`] for why it is a hash and not a counter.
fn scene_key(ps: &PieScene) -> String {
    let mut k = crate::scene::DefsKey::new();
    for sl in &ps.slices {
        k.eat(sl.label.as_bytes());
        k.eat_f64(sl.value);
    }
    k.finish()
}

/// The id of the hatching `<pattern>` at `angle` for the chart keyed
/// `key`.
fn hatch_id(key: &str, angle: f64) -> String {
    format!("fmpie-{key}-{angle:.0}")
}

/// Geometry of one slice. Angles are radians measured from
/// 12 o'clock, increasing clockwise; slices keep insertion (parse)
/// order. Color: [`slice_color`]`(i)` for slice `i`.
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

/// One percentage label (bold, at the base font size).
/// `#[non_exhaustive]`: [`scene`] builds these; read their fields.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PieLabel {
    /// Index into [`PieScene::slices`].
    pub slice: usize,
    /// The text, e.g. `42%` — rounded like mermaid's `toFixed(0)`
    /// (halves round up).
    pub text: String,
    /// Text anchor point; `y` is the vertical centre of the text.
    pub x: f64,
    /// Vertical centre of the text (see [`PieLabel::x`]).
    pub y: f64,
    /// `true`: centred inside its wedge, inked with
    /// [`crate::style::text_on`] of the slice colour. `false`: outside
    /// the rim on the canvas, inked with the theme's text colour
    /// ([`crate::style::Palette::text`]), with `x` at the end of
    /// [`PieLabel::leader`].
    pub inside: bool,
    /// SVG `text-anchor`: `middle` inside; `start` (right half) or
    /// `end` (left half) outside.
    pub anchor: &'static str,
    /// Outside labels: polyline from the wedge's rim out to the text
    /// (rim point, elbow, end beside the text). Empty inside.
    pub leader: Vec<(f64, f64)>,
}

/// Everything needed to draw a pie chart: canvas size, circle
/// centre + radius, per-slice angles, percentage labels, legend rows,
/// and the title with its centre anchor. `#[non_exhaustive]`: build
/// one with [`scene`].
#[derive(Debug, Clone)]
#[non_exhaustive]
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
    /// Percentage labels, in slice order; slices under
    /// [`MIN_LABEL_FRAC`] have none.
    pub labels: Vec<PieLabel>,
}

/// `x.toFixed(0)` for a finite `x >= 0`, as mermaid prints slice
/// percentages: the nearest integer, an exact half rounding UP (Rust's
/// `{:.0}` rounds halves to even, so 12.5% would print as 12%).
/// `x - x.floor()` is exact in binary, so the tie test is exact too.
fn to_fixed0(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 {
        f + 1.0
    } else {
        f
    }
}

/// Rendered width of a percentage label in bold at [`FONT`]: digits
/// are 0.556 em and `%` 0.889 em in Helvetica — [`text_width`]'s
/// generic 0.557 em for `%` would under-measure exactly the character
/// that makes "4%" collide with its separator.
fn label_w(text: &str) -> f64 {
    text.chars()
        .map(|c| match c {
            '0'..='9' => 0.556 * FONT,
            '%' => 0.889 * FONT,
            _ => text_width(&c.to_string()) * FONT / TEXT_CALIBRATION,
        })
        .sum::<f64>()
        * BOLD
}

/// Point on the circle around `(cx, cy)` at angle `a` — radians
/// from 12 o'clock, clockwise (the y-axis points down in SVG).
fn polar(cx: f64, cy: f64, rad: f64, a: f64) -> (f64, f64) {
    (cx + rad * a.sin(), cy - rad * a.cos())
}

/// Whether the `half_w × half_h` box centred on `(x, y)` (relative to
/// the pie centre) lies inside the wedge `start..end` of radius `r`,
/// at least `clr` from both radial edges and the rim. Corner tests
/// suffice: the box is convex, and so is every wedge up to 180°; a
/// wider wedge is the pie's dominant slice, whose label sits far from
/// the reflex side.
fn box_fits_wedge(
    (x, y): (f64, f64),
    (half_w, half_h): (f64, f64),
    (start, end): (f64, f64),
    r: f64,
    clr: f64,
) -> bool {
    let sweep = end - start;
    [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
        .iter()
        .all(|&(sx, sy)| {
            let (px, py) = (x + sx * half_w, y + sy * half_h);
            let rho = px.hypot(py);
            if rho > r - clr {
                return false;
            }
            // Angle from 12 o'clock, clockwise, relative to `start`.
            let from_start = ((px.atan2(-py) - start) % TAU + TAU) % TAU;
            if from_start >= sweep {
                return false;
            }
            // Distance to an edge ray: perpendicular while the corner
            // is within 90° of it, else to the ray's origin.
            let to_ray = |d: f64| if d < FRAC_PI_2 { rho * d.sin() } else { rho };
            to_ray(from_start) >= clr && to_ray(sweep - from_start) >= clr
        })
}

/// Where slice `sl`'s label of width `w` goes, relative to the pie
/// centre: the first radius from [`LABEL_R`] out to [`LABEL_R_MAX`]
/// (2px steps) where it fits inside the wedge, or `None`.
fn inside_spot(sl: &Slice, w: f64, r: f64) -> Option<(f64, f64)> {
    let mid = (sl.start_angle + sl.end_angle) / 2.0;
    let clr = SEPARATOR_W / 2.0 + 1.0;
    let mut d = r * LABEL_R;
    while d <= r * LABEL_R_MAX + 1e-9 {
        let p = polar(0.0, 0.0, d, mid);
        if box_fits_wedge(
            p,
            (w / 2.0, LABEL_HALF_H),
            (sl.start_angle, sl.end_angle),
            r,
            clr,
        ) {
            return Some(p);
        }
        d += 2.0;
    }
    None
}

/// Percentage labels relative to the pie centre: inside where they
/// fit, else outside the rim on a leader, with outside labels on each
/// side pushed apart vertically so neighbouring slivers never stack
/// their text.
fn place_labels(slices: &[Slice], r: f64) -> Vec<PieLabel> {
    let mut labels = Vec::new();
    for (i, sl) in slices.iter().enumerate() {
        let pct = sl.frac * 100.0;
        if pct < MIN_LABEL_FRAC * 100.0 {
            continue;
        }
        let text = format!("{:.0}%", to_fixed0(pct));
        let w = label_w(&text);
        let (inside, (x, y), anchor, leader) = match inside_spot(sl, w, r) {
            Some(p) => (true, p, "middle", Vec::new()),
            None => {
                let mid = (sl.start_angle + sl.end_angle) / 2.0;
                let rim = polar(0.0, 0.0, r, mid);
                let elbow = polar(0.0, 0.0, r + LEADER_OUT, mid);
                // Right half reads left-to-right away from the rim.
                let side = if mid.sin() >= 0.0 { 1.0 } else { -1.0 };
                let end = (elbow.0 + side * 6.0, elbow.1);
                let anchor = if side > 0.0 { "start" } else { "end" };
                (
                    false,
                    (end.0 + side * 3.0, end.1),
                    anchor,
                    vec![rim, elbow, end],
                )
            }
        };
        labels.push(PieLabel {
            slice: i,
            text,
            x,
            y,
            inside,
            anchor,
            leader,
        });
    }
    // Outside labels on one side: keep OUTSIDE_ROW_H between
    // consecutive ones, moving the leader's end with its text. Labels
    // above the centre are pushed UP and those below pushed DOWN —
    // always away from the circle (pushing a label near 12 o'clock
    // down would drive it into the pie) — each run starting from the
    // label nearest the horizontal midline, so leaders keep their
    // order and never cross.
    fn set_y(l: &mut PieLabel, y: f64) {
        l.y = y;
        if let Some(end) = l.leader.last_mut() {
            end.1 = y;
        }
    }
    for side in ["start", "end"] {
        let on_side = |l: &PieLabel| !l.inside && l.anchor == side;
        let mut up: Vec<usize> = (0..labels.len())
            .filter(|&k| on_side(&labels[k]) && labels[k].y < 0.0)
            .collect();
        let mut down: Vec<usize> = (0..labels.len())
            .filter(|&k| on_side(&labels[k]) && labels[k].y >= 0.0)
            .collect();
        up.sort_by(|&a, &b| labels[b].y.total_cmp(&labels[a].y));
        down.sort_by(|&a, &b| labels[a].y.total_cmp(&labels[b].y));
        for w in 1..up.len() {
            let ceiling = labels[up[w - 1]].y - OUTSIDE_ROW_H;
            if labels[up[w]].y > ceiling {
                set_y(&mut labels[up[w]], ceiling);
            }
        }
        // The lower run also clears the label just above the midline.
        let mut floor = up.first().map(|&k| labels[k].y + OUTSIDE_ROW_H);
        for &k in &down {
            if let Some(f) = floor {
                if labels[k].y < f {
                    set_y(&mut labels[k], f);
                }
            }
            floor = Some(labels[k].y + OUTSIDE_ROW_H);
        }
    }
    labels
}

/// Extent of a label's drawn box relative to the pie centre:
/// `(min_x, max_x, min_y, max_y)`.
fn label_extent(l: &PieLabel) -> (f64, f64, f64, f64) {
    let w = label_w(&l.text);
    let (x0, x1) = match l.anchor {
        "start" => (l.x, l.x + w),
        "end" => (l.x - w, l.x),
        _ => (l.x - w / 2.0, l.x + w / 2.0),
    };
    (x0, x1, l.y - FONT / 2.0, l.y + FONT / 2.0)
}

/// Compute all pie geometry. Never yields NaN/inf: a zero total
/// makes every `frac` 0 (drawn as an empty outline by [`to_svg`]).
pub fn scene(d: &PieChart) -> PieScene {
    let r = RADIUS;
    // Fractions are `value / sum`, exactly as mermaid computes its
    // percentages (so a 1:7 split is exactly 12.5% and its label
    // rounds the same way). When the plain sum overflows (several
    // 1e308 slices), normalise by the largest value first so each
    // term stays <= 1 and the sum <= n — an infinite total would make
    // every frac 0 and blank the chart.
    let sum: f64 = d.slices.iter().map(|s| s.value).sum();
    let max = d.slices.iter().map(|s| s.value).fold(0.0f64, f64::max);
    let total: f64 = if max > 0.0 {
        d.slices.iter().map(|s| s.value / max).sum()
    } else {
        0.0
    };
    let frac_of = |v: f64| {
        if sum.is_finite() && sum > 0.0 {
            v / sum
        } else if total > 0.0 {
            (v / max) / total
        } else {
            0.0
        }
    };
    let mut slices = Vec::with_capacity(d.slices.len());
    let mut cum = 0.0f64;
    for s in &d.slices {
        let frac = frac_of(s.value);
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
    // `text_width` is calibrated at TEXT_CALIBRATION px; the legend
    // draws at FONT.
    let text_w = texts.iter().map(|t| text_width(t)).fold(0.0, f64::max) * FONT / TEXT_CALIBRATION;
    let legend_w = if texts.is_empty() {
        0.0
    } else {
        SWATCH + 8.0 + text_w
    };

    // Labels relative to the centre; outside ones widen the pie's
    // footprint beyond the circle.
    let mut labels = place_labels(&slices, r);
    let (mut left, mut right, mut up, mut down) = (r, r, r, r);
    for l in labels.iter().filter(|l| !l.inside) {
        let (x0, x1, y0, y1) = label_extent(l);
        left = left.max(-x0);
        right = right.max(x1);
        up = up.max(-y0);
        down = down.max(y1);
    }

    let top = PAD + if d.title.is_some() { TITLE_H } else { 0.0 };
    let legend_h = texts.len() as f64 * LEGEND_ROW_H;
    // The legend is centred on the pie centre; whichever of the two
    // reaches further sets each half of the content height.
    let cy = top + up.max(legend_h / 2.0);
    let height = cy + down.max(legend_h / 2.0) + PAD;
    // pad | pie (+ outside labels) | gap | legend | pad (no legend
    // column when empty).
    let mut content_w = left + right;
    if legend_w > 0.0 {
        content_w += LEGEND_GAP + legend_w;
    }
    let mut width = 2.0 * PAD + content_w;
    if let Some(t) = &d.title {
        // The title is bold at TITLE_FONT; scale the 14px metric so a
        // long title still fits the canvas.
        width = width.max(2.0 * PAD + text_width(t) * TITLE_FONT / TEXT_CALIBRATION * BOLD);
    }
    // A title wider than the chart centres the chart under it.
    let x0 = (width - content_w) / 2.0;
    let cx = x0 + left;
    for l in &mut labels {
        l.x += cx;
        l.y += cy;
        for p in &mut l.leader {
            p.0 += cx;
            p.1 += cy;
        }
    }
    let lx = cx + right + LEGEND_GAP;
    let ly0 = cy - legend_h / 2.0;
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
        labels,
    }
}

/// Serialise a pie scene to SVG.
pub fn to_svg(ps: &PieScene) -> String {
    to_svg_with(ps, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(ps: &PieScene, opts: &SvgOptions) -> String {
    to_svg_a11y_with(ps, &A11y::default(), opts)
}

/// Width of the separator on the boundary between wedges `a` and `b`:
/// [`SEPARATOR_W`], narrowed next to a sliver to a third of its rim
/// length so the sliver keeps most of its colour instead of reading
/// as a seam between two white strokes.
fn separator_w(a: &Slice, b: &Slice, r: f64) -> f64 {
    let rim = |s: &Slice| (s.end_angle - s.start_angle) * r;
    (rim(a).min(rim(b)) / 3.0).clamp(0.5, SEPARATOR_W)
}

/// [`to_svg_with`] plus mermaid's `accTitle` / `accDescr` (see
/// [`A11y`]). Without an `accTitle` the chart's own title is the
/// accessible name, then "Pie chart".
pub(crate) fn to_svg_a11y_with(ps: &PieScene, a11y: &A11y, opts: &SvgOptions) -> String {
    let (title, desc) = a11y.resolve(ps.title.as_deref().unwrap_or("Pie chart"));
    // Slices keep their categorical colours in every theme; the title,
    // labels, legend, leaders and separators follow it.
    let pal = opts.palette();
    let mut s = String::new();
    svg_open_a11y(&mut s, ps.width, ps.height, FONT as u32, &title, desc, opts);

    if let Some(t) = &ps.title {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
            ps.title_pos.0,
            ps.title_pos.1,
            TITLE_FONT,
            pal.text,
            escape(t)
        ));
    }

    // Hatching for the slices whose colour repeats ([`slice_hatch`]),
    // one pattern per angle in use; none at all up to sixteen slices.
    let mut angles: Vec<f64> = (0..ps.slices.len()).filter_map(slice_hatch).collect();
    angles.sort_by(f64::total_cmp);
    angles.dedup();
    let key = if angles.is_empty() {
        String::new()
    } else {
        scene_key(ps)
    };
    if !angles.is_empty() {
        s.push_str("<defs>\n");
        for &a in &angles {
            s.push_str(&format!(
                "<pattern id=\"{}\" width=\"{HATCH_PITCH:.1}\" height=\"{HATCH_PITCH:.1}\" \
                 patternUnits=\"userSpaceOnUse\" patternTransform=\"rotate({a:.0})\">\
                 <rect width=\"{HATCH_W:.1}\" height=\"{HATCH_PITCH:.1}\" fill=\"{}\" \
                 fill-opacity=\"{HATCH_OPACITY:.2}\"/></pattern>\n",
                hatch_id(&key, a),
                pal.background
            ));
        }
        s.push_str("</defs>\n");
    }
    // `fill` for slice `i`'s hatching over its colour, if it has one.
    let hatch_fill = |i: usize| slice_hatch(i).map(|a| format!("url(#{})", hatch_id(&key, a)));

    // Slices. Zero-value slices are skipped (nothing to draw; their
    // legend row remains). A ~100% slice becomes a <circle>: one SVG
    // arc cannot draw a full 360° sweep (its endpoints coincide).
    // Fills carry no stroke: the white separators are drawn after
    // them, one per boundary, so each can be narrowed next to a sliver.
    let total_frac: f64 = ps.slices.iter().map(|sl| sl.frac).sum();
    if total_frac <= f64::EPSILON {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"none\" \
             stroke=\"{}\" stroke-width=\"1.6\"/>\n",
            ps.cx, ps.cy, ps.r, pal.line
        ));
    }
    for (i, sl) in ps.slices.iter().enumerate() {
        if sl.frac <= 0.0 {
            continue;
        }
        let color = slice_color(i);
        let a = polar(ps.cx, ps.cy, ps.r, sl.start_angle);
        let b = polar(ps.cx, ps.cy, ps.r, sl.end_angle);
        // A ~100% slice — or one so dominant that its arc endpoints
        // round to the same point — would render as a degenerate
        // zero-length arc (dropped by SVG renderers, so the slice
        // vanishes). Draw a full circle instead. The pixel-distance
        // test catches any value ratio, not just near-exact 1.0.
        let coincident = (a.0 - b.0).hypot(a.1 - b.1) < 0.2;
        // The wedge once in its colour, then again in its hatching.
        let fills = std::iter::once(color.to_string()).chain(hatch_fill(i));
        if sl.frac >= 1.0 - 1e-9 || (sl.frac > 0.5 && coincident) {
            for fill in fills {
                s.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\"/>\n",
                    ps.cx, ps.cy, ps.r, fill
                ));
            }
        } else {
            let large = i32::from(sl.frac > 0.5);
            for fill in fills {
                s.push_str(&format!(
                    "<path d=\"M {:.1} {:.1} L {:.1} {:.1} A {r:.1} {r:.1} 0 {} 1 {:.1} {:.1} Z\" \
                     fill=\"{}\"/>\n",
                    ps.cx,
                    ps.cy,
                    a.0,
                    a.1,
                    large,
                    b.0,
                    b.1,
                    fill,
                    r = ps.r
                ));
            }
        }
    }
    // Separators: one radial line per boundary between drawn wedges
    // (the last wedge meets the first at 12 o'clock), in the canvas
    // colour so a wedge reads cut out of it.
    let drawn: Vec<&Slice> = ps.slices.iter().filter(|sl| sl.frac > 0.0).collect();
    if drawn.len() >= 2 {
        for (k, sl) in drawn.iter().enumerate() {
            let prev = drawn[(k + drawn.len() - 1) % drawn.len()];
            let p = polar(ps.cx, ps.cy, ps.r, sl.start_angle);
            s.push_str(&format!(
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" \
                 stroke-width=\"{:.2}\"/>\n",
                ps.cx,
                ps.cy,
                p.0,
                p.1,
                pal.background,
                separator_w(prev, sl, ps.r)
            ));
        }
    }

    // Percentage labels: inside ones in white or dark ink by contrast
    // with their wedge; outside ones in the diagram ink on a leader.
    for l in &ps.labels {
        // An inside label over hatching is haloed in its slice colour,
        // so the stripes never cut through its digits.
        let mut halo = String::new();
        let fill = if l.inside {
            if slice_hatch(l.slice).is_some() {
                halo = format!(
                    " stroke=\"{}\" stroke-width=\"3\" stroke-linejoin=\"round\" \
                     paint-order=\"stroke\"",
                    slice_color(l.slice)
                );
            }
            crate::style::text_on(slice_color(l.slice))
        } else {
            let pts: Vec<String> = l
                .leader
                .iter()
                .map(|(x, y)| format!("{x:.1},{y:.1}"))
                .collect();
            s.push_str(&format!(
                "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>\n",
                pts.join(" "),
                pal.line
            ));
            &pal.text
        };
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"{}\" \
             font-weight=\"bold\" fill=\"{}\"{}>{}</text>\n",
            l.x,
            l.y,
            l.anchor,
            fill,
            halo,
            escape(&l.text)
        ));
    }

    // Legend: color swatch + label (+ raw value under showData).
    for (i, row) in ps.legend.iter().enumerate() {
        for fill in std::iter::once(slice_color(i).to_string()).chain(hatch_fill(i)) {
            s.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{sw:.0}\" height=\"{sw:.0}\" rx=\"2\" \
                 fill=\"{}\"/>\n",
                row.x,
                row.y - SWATCH / 2.0,
                fill,
                sw = SWATCH
            ));
        }
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" fill=\"{}\">{}</text>\n",
            row.x + SWATCH + 8.0,
            row.y,
            pal.text,
            escape(&row.text)
        ));
    }

    s.push_str("</svg>\n");
    s
}

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
    /// wedges. A wedge is the region its path fills, at the 0.1 px
    /// [`to_svg`] writes it to: its angular sector inside the radius,
    /// but with its arc on the circle a renderer puts through the
    /// written ends (ends that lie a few px apart can put that a px or
    /// more off the pie's centre). A slice drawn as the whole disc
    /// covers the slices before it; a slice of 0 has none. A label is
    /// its text box (as wide and tall as the layout measures it,
    /// anchored as drawn) and its leader, a stroke `tol` widens (`<= 0`:
    /// exact; the nearest label wins); a legend row is its swatch and
    /// its text box. `None` off the canvas (`0..=width` x `0..=height`).
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

    /// Whether `p` lies in slice `i` as [`to_svg`] fills it, at the
    /// 0.1 px it writes every number to ([`written`]): nothing for a
    /// slice of 0, the disc for one drawn as a circle (~100%, or a
    /// dominant slice whose arc ends meet), else the wedge its path
    /// closes ([`wedge_contains`]).
    fn slice_contains(&self, i: usize, p: (f64, f64)) -> bool {
        let sl = &self.slices[i];
        if sl.frac <= 0.0 {
            return false;
        }
        let c = (written(self.cx), written(self.cy));
        let r = written(self.r);
        let a = polar(self.cx, self.cy, self.r, sl.start_angle);
        let b = polar(self.cx, self.cy, self.r, sl.end_angle);
        let coincident = (a.0 - b.0).hypot(a.1 - b.1) < 0.2;
        if sl.frac >= 1.0 - 1e-9 || (sl.frac > 0.5 && coincident) {
            return (p.0 - c.0).hypot(p.1 - c.1) <= r;
        }
        let a = (written(a.0), written(a.1));
        let b = (written(b.0), written(b.1));
        wedge_contains(c, a, b, r, sl.frac > 0.5, p)
    }
}

/// `v` as [`to_svg`] writes it (`{:.1}`) and a renderer reads it back:
/// the nearest tenth. A value at a tie, which `{:.1}` rounds to even, is
/// formatted to be sure.
fn written(v: f64) -> f64 {
    let t = v * 10.0;
    if (t - t.floor() - 0.5).abs() < 1e-6 {
        return format!("{v:.1}").parse().unwrap_or(v);
    }
    t.round() / 10.0
}

/// Whether `p` lies in the region the wedge path
/// `M c L a A r r 0 large 1 b Z` fills (SVG's nonzero rule): the
/// triangle `c a b`, counted with its winding, plus the segment the
/// chord `a b` cuts off the arc's circle on the arc's side. The arc's
/// circle is SVG's (implementation notes, "conversion from endpoint to
/// center parameterization"): radius `r` through both ends, centred on
/// the side `large` and the clockwise sweep put it, the radius grown to
/// half the chord where the chord is longer than the diameter. An arc
/// whose ends meet is not drawn, and one of radius 0 is a straight line.
fn wedge_contains(
    c: (f64, f64),
    a: (f64, f64),
    b: (f64, f64),
    r: f64,
    large: bool,
    p: (f64, f64),
) -> bool {
    // (u - o) x (v - o): positive when v lies clockwise of u round o on
    // screen (the y-axis points down).
    let cross = |o: (f64, f64), u: (f64, f64), v: (f64, f64)| {
        (u.0 - o.0) * (v.1 - o.1) - (u.1 - o.1) * (v.0 - o.0)
    };
    let mut winding = 0;
    let turn = cross(c, a, b);
    if turn != 0.0 {
        let s = turn.signum();
        if cross(c, a, p) * s >= 0.0 && cross(a, b, p) * s >= 0.0 && cross(b, c, p) * s >= 0.0 {
            winding += if s > 0.0 { 1 } else { -1 };
        }
    }
    let (hx, hy) = ((a.0 - b.0) / 2.0, (a.1 - b.1) / 2.0);
    let d2 = hx * hx + hy * hy;
    if d2 > 0.0 && r > 0.0 {
        let rr = r.max(d2.sqrt());
        let k = ((rr * rr - d2) / d2).max(0.0).sqrt() * if large { -1.0 } else { 1.0 };
        let m = ((a.0 + b.0) / 2.0 + k * hy, (a.1 + b.1) / 2.0 - k * hx);
        // The arc runs clockwise from `a` to `b`, the chord back: the
        // segment lies to the right of `b -> a` on screen.
        if (p.0 - m.0).hypot(p.1 - m.1) <= rr && cross(b, a, p) >= 0.0 {
            winding += 1;
        }
    }
    winding != 0
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
        assert_eq!(
            ps.slices[0].start_angle, 0.0,
            "first slice starts at 12 o'clock"
        );
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
        let svg = to_svg(&scene(&pie(
            "pie\n\"big\" : 9\n\"none\" : 0\n\"small\" : 1",
        )));
        assert_eq!(
            svg.lines().filter(|l| l.starts_with("<path")).count(),
            2,
            "only the two non-zero slices are drawn"
        );
        assert!(
            svg.contains(">none</text>"),
            "zero slice keeps its legend row"
        );
        // `small` is slice 2: its wedge must use accent(2), not shift
        // down because slice 1 was skipped. (Wedges carry no stroke of
        // their own: the separators are drawn after them.)
        assert!(svg.lines().any(|l| l.starts_with("<path")
            && l.ends_with(&format!("fill=\"{}\"/>", crate::style::accent(2)))));
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
        assert!(
            svg.contains(">Dogs [6]</text>"),
            "whole values print without .0"
        );
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
            assert!(
                lo >= -1.0 && hi <= w + 1.0,
                "text {inner:?} [{lo:.0}..{hi:.0}] outside width {w}"
            );
            assert!((0.0..=h).contains(&y), "text y {y} outside height {h}");
        }
    }

    /// End-to-end showcase guarding the whole feature set as one
    /// interacting whole: header showData, standalone title, six
    /// slices including a tiny (<1%) one, comments, and blank lines.
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
        assert!(
            svg.contains(">Calcium [42.96]</text>"),
            "showData legend value"
        );
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
            assert!(
                lo >= -1.0 && hi <= w + 1.0,
                "text {inner:?} outside width {w}"
            );
            assert!((0.0..=h).contains(&y), "text y {y} outside height {h}");
        }
        // render_svg dispatches to the same output.
        assert_eq!(
            crate::render_svg(include_str!("../examples/pie.mmd")).unwrap(),
            svg
        );
    }

    // ------------------------ regressions --------------------------

    #[test]
    fn enormous_values_do_not_blank_the_chart() {
        // Bug (bug hunt): summing several 1e308 slices overflowed the
        // total to +inf, making every frac 0 and blanking the chart.
        let sc = scene(&pie("pie\n\"a\" : 1e308\n\"b\" : 1e308\n\"c\" : 1e308"));
        for sl in &sc.slices {
            assert!(
                (sl.frac - 1.0 / 3.0).abs() < 1e-6,
                "each slice ~1/3, got {}",
                sl.frac
            );
        }
        let svg = to_svg(&sc);
        assert_eq!(svg.matches("<path").count(), 3, "three real slices drawn");
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
    }

    #[test]
    fn dominant_slice_renders_as_a_visible_circle() {
        // Bug (bug hunt): a slice at ratio >= ~1e5:1 (frac ~0.99999)
        // drew a zero-length arc (start point == end point), which
        // SVG renderers drop — the dominant slice vanished. It must
        // become a full <circle> in its own color.
        let svg = to_svg(&scene(&pie("pie\n\"a\" : 100000\n\"b\" : 1")));
        let color = crate::style::accent(0);
        assert!(
            svg.contains(&format!("fill=\"{color}\"")) && svg.contains("<circle"),
            "dominant slice must be a filled circle"
        );
        // Not the zero-total blank-outline circle (fill="none").
        assert!(
            !svg.contains("fill=\"none\""),
            "must not be the empty outline"
        );
    }

    /// Report Q5 / pie white-label-low-contrast: every percentage was
    /// hard-coded white, failing WCAG AA on most wedges.
    #[test]
    fn percentage_labels_read_on_every_wedge() {
        // Eight distinct percentages so each label is findable.
        let vals = [18, 17, 16, 14, 12, 10, 8, 5];
        let src: String = vals
            .iter()
            .enumerate()
            .map(|(i, v)| format!("\"s{i}\" : {v}\n"))
            .collect();
        let ps = scene(&pie(&format!("pie\n{src}")));
        let svg = to_svg(&ps);
        assert_eq!(ps.labels.len(), vals.len());
        for l in &ps.labels {
            assert!(l.inside, "{} on its wedge", l.text);
            let fill = slice_color(l.slice);
            assert_eq!(fill, crate::style::accent(l.slice));
            let ink = crate::style::assert_text_reads(&svg, &l.text, fill);
            assert_eq!(ink, crate::style::text_on(fill), "{}", l.text);
        }
    }

    /// The drawn box of a label in absolute coordinates.
    fn label_box(l: &PieLabel) -> (f64, f64, f64, f64) {
        let (x0, x1, _, _) = label_extent(l);
        (x0, x1, l.y - LABEL_HALF_H, l.y + LABEL_HALF_H)
    }

    /// Independent check (not `box_fits_wedge`): every corner of the
    /// label box lies in its wedge's angular range, at least `clr`
    /// from both radial edges (as perpendicular distance) and the rim.
    fn inside_wedge(ps: &PieScene, l: &PieLabel, clr: f64) -> bool {
        let sl = &ps.slices[l.slice];
        let (x0, x1, y0, y1) = label_box(l);
        [(x0, y0), (x1, y0), (x0, y1), (x1, y1)]
            .iter()
            .all(|&(x, y)| {
                let (dx, dy) = (x - ps.cx, y - ps.cy);
                let a = dx.atan2(-dy).rem_euclid(TAU);
                let edge = |e: f64| {
                    // Distance from the corner to the line through the
                    // centre at angle e.
                    (dx * e.cos() + dy * e.sin()).abs()
                };
                dx.hypot(dy) <= ps.r - clr
                    && a > sl.start_angle
                    && a < sl.end_angle
                    && edge(sl.start_angle) >= clr
                    && edge(sl.end_angle) >= clr
            })
    }

    /// Report P0 #16 / pie thin-slice-labels-overflow: the fixture's
    /// '4%' sat on its wedge's white separators and its 1.3px gap to
    /// '5%' read as "5%4%". Every inside label now clears both radial
    /// edges and the rim, and no two labels touch.
    #[test]
    fn showcase_labels_fit_inside_their_wedges() {
        let ps = scene(&fixture());
        assert_eq!(ps.r, RADIUS);
        assert_eq!(ps.labels.len(), 5);
        for l in &ps.labels {
            assert!(l.inside, "{} should fit inside", l.text);
            assert!(inside_wedge(&ps, l, 1.5), "{} leaves its wedge", l.text);
        }
        let texts: Vec<&str> = ps.labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["38%", "44%", "9%", "5%", "4%"]);
        assert_labels_apart(&ps);
    }

    /// No two labels' boxes overlap (with `gap` px to spare), and no
    /// outside label's box reaches into the circle.
    fn assert_labels_apart(ps: &PieScene) {
        let gap = 2.0;
        for (i, a) in ps.labels.iter().enumerate() {
            let (ax0, ax1, ay0, ay1) = label_box(a);
            for b in &ps.labels[i + 1..] {
                let (bx0, bx1, by0, by1) = label_box(b);
                let apart =
                    ax1 + gap <= bx0 || bx1 + gap <= ax0 || ay1 + gap <= by0 || by1 + gap <= ay0;
                assert!(apart, "labels {} and {} overlap", a.text, b.text);
            }
            if !a.inside {
                // Nearest point of the box to the centre.
                let nx = ps.cx.clamp(ax0, ax1);
                let ny = ps.cy.clamp(ay0, ay1);
                let d = (nx - ps.cx).hypot(ny - ps.cy);
                assert!(d > ps.r, "outside label {} reaches into the pie", a.text);
                assert_eq!(a.leader.len(), 3, "outside label has a leader");
                let rim = a.leader[0];
                let to_rim = (rim.0 - ps.cx).hypot(rim.1 - ps.cy);
                assert!((to_rim - ps.r).abs() < 0.01, "leader starts on the rim");
            } else {
                assert!(a.leader.is_empty());
            }
        }
    }

    /// Mermaid labels every slice of at least 1% (`MIN_LABEL_FRAC`
    /// used to hide everything under 4%). Slices too thin for their
    /// text get it outside the rim on a leader; neighbouring slivers'
    /// labels are pushed apart, never into the pie or each other.
    #[test]
    fn labels_reach_down_to_one_percent_without_overlapping() {
        for src in [
            "pie\n\"Big\" : 80\n\"A\" : 3.9\n\"B\" : 2.5\n\"C\" : 1.2\n\"D\" : 0.9\n\
             \"E\" : 6\n\"F\" : 5.5",
            "pie title T\n\"A\" : 50\n\"B\" : 45\n\"C\" : 1.5\n\"D\" : 1.5\n\"E\" : 2",
            "pie\n\"A\" : 1\n\"B\" : 1\n\"C\" : 1\n\"D\" : 97",
        ] {
            let ps = scene(&pie(src));
            for (i, sl) in ps.slices.iter().enumerate() {
                let labelled = ps.labels.iter().any(|l| l.slice == i);
                assert_eq!(labelled, sl.frac >= 0.01, "{src}: slice {i}");
            }
            assert_labels_apart(&ps);
            for l in ps.labels.iter().filter(|l| l.inside) {
                assert!(inside_wedge(&ps, l, 1.5), "{src}: {}", l.text);
            }
            // Outside labels widen the canvas instead of being cut off.
            let svg = to_svg(&ps);
            for l in &ps.labels {
                let (x0, x1, y0, y1) = label_box(l);
                assert!(x0 >= 0.0 && x1 <= ps.width, "{src}: {} cut off", l.text);
                assert!(y0 >= 0.0 && y1 <= ps.height, "{src}: {} cut off", l.text);
                assert!(svg.contains(&format!(">{}</text>", l.text)));
            }
        }
        // The 3.9% slice prints "4%", as mermaid does.
        let ps = scene(&pie("pie\n\"A\" : 3.9\n\"B\" : 96.1"));
        assert_eq!(ps.labels[0].text, "4%");
        // Outside labels sit on the white canvas: diagram ink.
        let ps = scene(&pie("pie\n\"A\" : 1\n\"B\" : 99"));
        assert!(!ps.labels[0].inside);
        let svg = to_svg(&ps);
        crate::style::assert_text_reads(&svg, "1%", "#ffffff");
        assert!(svg.contains("<polyline"), "leader drawn");
    }

    /// Mermaid prints `(v / sum * 100).toFixed(0)`, which rounds an
    /// exact half UP; Rust's `{:.0}` rounded 12.5 to 12.
    #[test]
    fn percentages_round_halves_up_like_to_fixed() {
        let ps = scene(&pie("pie\n\"one\" : 1\n\"seven\" : 7"));
        let texts: Vec<&str> = ps.labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["13%", "88%"]);
        assert_eq!(to_fixed0(2.5), 3.0);
        assert_eq!(to_fixed0(0.49999999999999994), 0.0, "no x + 0.5 rounding");
        assert_eq!(to_fixed0(12.4), 12.0);
        assert_eq!(to_fixed0(99.5), 100.0);
    }

    /// Report P0 #16: `accTitle` / `accDescr` were parsed and thrown
    /// away, the accessible name was always "Pie chart".
    #[test]
    fn acc_title_and_descr_reach_the_svg() {
        let svg = crate::render_svg(
            "pie title Shown title\naccTitle: Spoken title\naccDescr: What it shows\n\"a\" : 1",
        )
        .unwrap();
        assert!(svg.contains("aria-label=\"Spoken title\""), "{svg}");
        assert!(svg.contains("<title>Spoken title</title>"));
        assert!(svg.contains(">What it shows</desc>"));
        assert!(svg.contains("aria-describedby=\"flowmaid-desc-"));
        // accTitle never draws: the visible title stays.
        assert!(svg.contains(">Shown title</text>"));
        let d = pie("pie\naccTitle: A\naccDescr {\n  multi\n  line\n}\n\"a\" : 1");
        assert_eq!(d.acc_title.as_deref(), Some("A"));
        assert_eq!(d.acc_descr.as_deref(), Some("multi\nline"));
        // Without accTitle the chart's title names it, then the default.
        let titled = crate::render_svg("pie title Budget\n\"a\" : 1").unwrap();
        assert!(titled.contains("aria-label=\"Budget\""));
        assert!(!titled.contains("<desc"));
        let plain = crate::render_svg("pie\n\"a\" : 1").unwrap();
        assert!(plain.contains("aria-label=\"Pie chart\""));
    }

    /// Report pie legend/title width: the legend was measured at the
    /// 14px calibration size but drawn at 13px (34px right margin
    /// instead of PAD), and the 16px bold title as 16/13 of the 14px
    /// metric without the bold factor.
    #[test]
    fn legend_and_title_are_measured_at_their_drawn_size() {
        let ps = scene(&fixture());
        let widest = ps
            .legend
            .iter()
            .map(|r| text_width(&r.text) * 13.0 / TEXT_CALIBRATION)
            .fold(0.0, f64::max);
        let right = ps.legend[0].x + SWATCH + 8.0 + widest;
        assert!(
            (ps.width - right - PAD).abs() < 1e-9,
            "right margin is PAD: {}",
            ps.width - right
        );
        let t = "An extremely long pie chart title that is wider than the pie";
        let ps = scene(&pie(&format!("pie title {t}\n\"a\" : 1")));
        let need = text_width(t) * 16.0 / TEXT_CALIBRATION * 1.06;
        assert!(ps.width >= need + 2.0 * PAD - 1e-9);
        // The narrower chart is centred under the title.
        let left = ps.cx - ps.r;
        let right = ps.legend[0].x + SWATCH + 8.0 + text_width("a") * 13.0 / TEXT_CALIBRATION;
        assert!((left - (ps.width - right)).abs() < 1e-9, "centred");
    }

    /// Report pie palette wrap: slice 9 repeated slice 1's colour (and
    /// with nine slices the last touched the first at 12 o'clock).
    /// Phase A follow-up: past sixteen slices the colours start over,
    /// so slice 17 took slice 1's colour — beside it at 12 o'clock on a
    /// full pie, and with the same legend swatch. Each later turn of the
    /// colours now carries hatching as a second cue; up to sixteen
    /// slices nothing changes (no pattern is even defined).
    #[test]
    fn repeated_slice_colours_are_hatched_apart() {
        for i in 0..16 {
            assert_eq!(slice_hatch(i), None, "{i}");
        }
        assert_eq!(slice_hatch(16), Some(45.0));
        assert_eq!(slice_hatch(31), Some(45.0));
        assert_eq!(slice_hatch(32), Some(135.0));
        assert_eq!(slice_hatch(48), Some(0.0));
        assert_eq!(slice_hatch(64), Some(90.0));
        assert_eq!(slice_hatch(80), Some(45.0));
        // (colour, cue) pairs never repeat in the first eighty slices.
        let cues: Vec<(&str, Option<u64>)> = (0..80)
            .map(|i| (slice_color(i), slice_hatch(i).map(f64::to_bits)))
            .collect();
        for (i, c) in cues.iter().enumerate() {
            assert!(!cues[i + 1..].contains(c), "slice {i} repeats {c:?}");
        }

        let rows = |n: usize| -> String { (0..n).map(|i| format!("\"s{i}\" : 5\n")).collect() };
        let sixteen = to_svg(&scene(&pie(&format!("pie\n{}", rows(16)))));
        assert!(!sixteen.contains("<pattern") && !sixteen.contains("fmpie"));
        let ps = scene(&pie(&format!("pie\n{}", rows(17))));
        let svg = to_svg(&ps);
        assert_eq!(slice_color(16), slice_color(0));
        assert_eq!(svg.matches("<pattern ").count(), 1, "{svg}");
        let id_at = svg.find("<pattern id=\"").unwrap() + "<pattern id=\"".len();
        let id = &svg[id_at..id_at + svg[id_at..].find('"').unwrap()];
        assert!(svg.contains("patternTransform=\"rotate(45)\""), "{svg}");
        // The 17th wedge and its swatch, each once in colour and once hatched.
        assert_eq!(
            svg.matches(&format!("fill=\"url(#{id})\"")).count(),
            2,
            "{svg}"
        );
        let last = &ps.slices[16];
        let a = polar(ps.cx, ps.cy, ps.r, last.start_angle);
        let wedge = format!("L {:.1} {:.1} A", a.0, a.1);
        let paths: Vec<&str> = svg.lines().filter(|l| l.contains(&wedge)).collect();
        assert_eq!(paths.len(), 2, "{svg}");
        assert!(paths[0].contains(&format!("fill=\"{}\"", slice_color(16))));
        assert!(paths[1].contains(&format!("fill=\"url(#{id})\"")));
        // ... and it still touches slice 1 at 12 o'clock.
        assert!((last.end_angle - TAU).abs() < 1e-9 && ps.slices[0].start_angle.abs() < 1e-9);
        // Its inside label keeps a halo in its colour over the stripes.
        let l = ps.labels.iter().find(|l| l.slice == 16).unwrap();
        if l.inside {
            assert!(svg.contains(&format!(
                "stroke=\"{}\" stroke-width=\"3\" stroke-linejoin=\"round\" \
                 paint-order=\"stroke\">{}</text>",
                slice_color(16),
                l.text
            )));
        }
        // Ids are namespaced per chart and the output is deterministic.
        assert_eq!(svg, to_svg(&scene(&pie(&format!("pie\n{}", rows(17))))));
        let other = to_svg(&scene(&pie(&format!("pie\n{}\"x\" : 1\n", rows(17)))));
        assert!(!other.contains(id), "{other}");
    }

    #[test]
    fn up_to_sixteen_slices_get_distinct_colors() {
        let colors: Vec<&str> = (0..16).map(slice_color).collect();
        for (i, a) in colors.iter().enumerate() {
            assert!(!colors[i + 1..].contains(a), "colour {a} repeats");
        }
        for i in 0..8 {
            assert_eq!(slice_color(i), crate::style::accent(i), "first 8 unchanged");
        }
        let rows: String = (0..9).map(|i| format!("\"s{i}\" : 1\n")).collect();
        let ps = scene(&pie(&format!("pie\n{rows}")));
        let svg = to_svg(&ps);
        // The ninth wedge and its swatch take the deep indigo, not
        // slice 0's indigo (which it touches at 12 o'clock).
        assert_ne!(slice_color(8), slice_color(0));
        assert!(svg.contains(&format!("fill=\"{}\"/>", slice_color(8))));
        assert_eq!(svg.matches(slice_color(8)).count(), 2, "wedge + swatch");
        // The ninth label is inked for its deep wedge.
        let ink = crate::style::text_on(slice_color(8));
        assert!(crate::style::contrast_ratio(ink, slice_color(8)).unwrap() >= 4.5);
        assert_eq!(ps.labels.len(), 9);
    }

    /// Report pie trace-sliver-seam: a 0.35% slice drew as a ~0.9px
    /// line between two 1.5px white strokes. Separators next to a
    /// sliver narrow to a third of its rim, so most of it stays coloured.
    #[test]
    fn separators_narrow_next_to_a_sliver() {
        let ps = scene(&fixture());
        let trace = &ps.slices[5];
        let rim = (trace.end_angle - trace.start_angle) * ps.r;
        assert!(rim < 4.0, "fixture has a sliver ({rim:.2}px)");
        let w = separator_w(&ps.slices[4], trace, ps.r);
        assert!(w < SEPARATOR_W && w <= rim / 3.0 + 1e-9);
        assert!(rim - w >= rim / 2.0, "most of the sliver stays visible");
        // Wide neighbours keep the full separator.
        assert_eq!(separator_w(&ps.slices[0], &ps.slices[1], ps.r), SEPARATOR_W);
        // One separator per boundary: six wedges, six lines.
        let svg = to_svg(&ps);
        assert_eq!(svg.matches("<line ").count(), 6);
        assert!(svg.contains(&format!("stroke-width=\"{w:.2}\"")));
        // A lone slice needs none.
        let one = to_svg(&scene(&pie("pie\n\"a\" : 1")));
        assert!(!one.contains("<line "));
    }
}
