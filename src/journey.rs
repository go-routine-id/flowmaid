//! User-journey rendering: a fixed horizontal band — no graph layout.
//! Tasks flow left to right in source order; each is a smiley face whose
//! color and mouth curve track its 1–5 satisfaction score (red frown →
//! green smile), and whose height does too: a happier task sits higher,
//! so the faces and the thin path linking them trace the journey's
//! satisfaction curve. Sections are pale bands spanning their tasks,
//! with a gutter between neighbours, and each participant gets a stable
//! color — chosen away from the score colors — shown in a legend and as
//! per-task dots (past ten participants the colors repeat, and the dots
//! change shape: [`actor_mark`]).
//!
//! Like `pie`/`mindmap` there is nothing draggable and no `route()`:
//! [`scene`] computes every coordinate and [`to_svg`] serialises it.

use crate::layout::{line_count, text_width, LINE_H, TEXT_CALIBRATION};
use crate::model::Journey;
use crate::scene::{escape, svg_open_a11y, svg_text_lines, A11y, SvgOptions, TEXT_COLOR};

/// Canvas margin.
pub const PAD: f64 = 24.0;
/// Title band height (when a title is present).
pub const TITLE_H: f64 = 34.0;
/// Actor-legend band height (when there are actors).
pub const LEGEND_H: f64 = 30.0;
/// Section band height.
pub const SECTION_H: f64 = 30.0;
/// Gap under the section band.
pub const GAP: f64 = 14.0;
/// Horizontal gutter between two neighbouring section bands (and the
/// columns under them), so bands never touch.
pub const SECTION_GAP: f64 = 8.0;
/// Face radius.
pub const FACE_R: f64 = 22.0;
/// How far a face (with its dots and label) drops per score point
/// below 5: score 5 sits highest, score 1 `4 * SCORE_STEP` lower.
pub const SCORE_STEP: f64 = 14.0;
/// Actor dot radius (per-task dots and legend dots alike) + spacing +
/// the height of the dot row above a face.
pub const DOT_R: f64 = 5.0;
/// Space between two neighbouring actor dots; see [`DOT_R`].
pub const DOT_GAP: f64 = 4.0;
/// Height of the actor-dot row above a face; see [`DOT_R`].
pub const DOTS_H: f64 = 20.0;
/// Room reserved under a face for a one-line label (each extra
/// wrapped line adds [`LINE_H`]).
pub const LABEL_H: f64 = 34.0;
/// Minimum column width per task.
pub const COL_MIN: f64 = 104.0;
/// Task labels wider than this wrap onto more lines instead of
/// widening their column without bound.
pub const LABEL_WRAP_W: f64 = 150.0;
/// Base font size.
pub const FONT: u32 = 13;
/// Title font size.
const TITLE_FONT: f64 = 17.0;
/// Width factor of bold text over regular (as `text_width` counts
/// `<b>` runs).
const BOLD: f64 = 1.06;
/// The line linking the faces (the journey path), in the default theme
/// ([`crate::style::Palette::guide`] in any other).
pub const PATH_COLOR: &str = "#c2c8dc";
/// Face outline stroke.
pub const FACE_STROKE: &str = "#2f3550";
/// Face ink (eyes + mouth) — the shared engine text color.
pub const FACE_INK: &str = TEXT_COLOR;

/// Satisfaction color for a 1–5 score (red → green).
pub fn score_color(score: u8) -> &'static str {
    match score {
        1 => "#e8574f",
        2 => "#f0894e",
        3 => "#f2c94c",
        4 => "#9fcf6a",
        _ => "#57b563",
    }
}

/// Section band fills: each [`crate::style::ACCENTS`] entry mixed 74%
/// toward white (index-parallel). Pale, so a band recedes behind the
/// faces and never reads as an actor's or a score's color, and its
/// name reads in the diagram ink at ≥ 10:1.
pub const SECTION_FILLS: &[&str] = &[
    "#d4d9ef", // indigo
    "#cae7d5", // green
    "#f5e2c2", // amber
    "#efd4e1", // rose
    "#c3e5e2", // teal
    "#e1d5f4", // violet
    "#f5d7d4", // coral
    "#d1e2ee", // steel blue
];

/// Fill of section band `i` (wraps around [`SECTION_FILLS`]).
pub fn section_color(i: usize) -> &'static str {
    SECTION_FILLS[i % SECTION_FILLS.len()]
}

/// Indices into [`crate::style::accent_wide`] that actors cycle
/// through: the accents without green, amber and coral (which sit next
/// to the score colors), then their deep variants — ten colors before
/// the cycle repeats.
const ACTOR_ACCENTS: &[usize] = &[0, 5, 4, 7, 3, 8, 13, 12, 15, 11];

/// Color of actor `i` (legend dot and per-task dots). Never a score
/// color look-alike, so a dot can't be read as a rating. The ten colors
/// repeat from the eleventh actor on; [`actor_mark`] tells the repeats
/// apart.
pub fn actor_color(i: usize) -> &'static str {
    crate::style::accent_wide(ACTOR_ACCENTS[i % ACTOR_ACCENTS.len()])
}

/// The shape an actor's dots (legend and per-task alike) are drawn as:
/// the second cue beside [`actor_color`], whose colors repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActorMark {
    /// A disc of radius [`DOT_R`]: the first ten actors.
    Circle,
    /// A square of the disc's area (side `1.8 *` [`DOT_R`], corners
    /// rounded by 1 px): actors 11–20.
    Square,
    /// A square on its point, `1.25 *` [`DOT_R`] from centre to tip
    /// (the disc's area): actors 21–30.
    Diamond,
    /// An upward triangle whose corners lie `1.25 *` [`DOT_R`] from its
    /// centre: actors 31–40 (then the circle again).
    Triangle,
}

/// The mark actor `i`'s dots take: one per turn of the [`actor_color`]
/// cycle, so the eleventh actor — the first one's color again — reads
/// apart from it in the legend and on every task.
pub fn actor_mark(i: usize) -> ActorMark {
    match (i / ACTOR_ACCENTS.len()) % 4 {
        0 => ActorMark::Circle,
        1 => ActorMark::Square,
        2 => ActorMark::Diamond,
        _ => ActorMark::Triangle,
    }
}

/// SVG for one actor dot centred on `(cx, cy)` in `mark`'s shape,
/// carrying `tip` (a `<title>`) when there is one. A circle without a
/// tip is written as it always was, self-closed.
fn dot_svg(cx: f64, cy: f64, color: &str, mark: ActorMark, tip: Option<&str>) -> String {
    let close = |open: String| match tip {
        Some(t) => format!(
            "{open}>{t}</{}>\n",
            open[1..].split(' ').next().unwrap_or("")
        ),
        None => format!("{open}/>\n"),
    };
    let reach = 1.25 * DOT_R;
    match mark {
        ActorMark::Circle => close(format!(
            "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{DOT_R:.1}\" fill=\"{color}\""
        )),
        ActorMark::Square => {
            let h = 0.9 * DOT_R;
            close(format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"1\" \
                 fill=\"{color}\"",
                cx - h,
                cy - h,
                2.0 * h,
                2.0 * h
            ))
        }
        ActorMark::Diamond => close(format!(
            "<polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" fill=\"{color}\"",
            cx,
            cy - reach,
            cx + reach,
            cy,
            cx,
            cy + reach,
            cx - reach,
            cy
        )),
        ActorMark::Triangle => {
            let dx = reach * 3f64.sqrt() / 2.0;
            close(format!(
                "<polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" fill=\"{color}\"",
                cx,
                cy - reach,
                cx + dx,
                cy + reach / 2.0,
                cx - dx,
                cy + reach / 2.0
            ))
        }
    }
}

/// A colored band behind one section's tasks.
#[derive(Debug, Clone)]
pub struct SectionBand {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub name: String,
    /// Band fill: [`section_color`] of the section's index.
    pub color: &'static str,
}

/// One task: a face + label + its participant dots.
/// `#[non_exhaustive]`: [`scene`] builds these; read their fields.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TaskGlyph {
    pub cx: f64,
    /// Face centre height: lower for a lower score (see [`SCORE_STEP`]).
    pub cy: f64,
    pub r: f64,
    pub score: u8,
    pub color: &'static str,
    pub name: String,
    /// Centre of the label's FIRST line (further lines go below).
    pub label_pos: (f64, f64),
    /// (x, y, color) per participating actor.
    pub actor_dots: Vec<(f64, f64, &'static str)>,
    /// The label as drawn: `name` wrapped at [`LABEL_WRAP_W`] into as
    /// few lines as it needs, evened out so no word is left stranded
    /// on the last (`\n` between lines).
    pub label: String,
    /// Actor index (into [`JourneyScene::legend`]) per entry of
    /// `actor_dots` — the dot's tooltip names that actor.
    pub actor_ids: Vec<usize>,
    /// The score as written (clamped to 1..=5, not rounded), which
    /// sets `cy`.
    pub exact_score: f64,
}

/// One legend entry (actor color key). `x` is the item's left edge.
#[derive(Debug, Clone)]
pub struct LegendItem {
    pub x: f64,
    pub y: f64,
    pub name: String,
    pub color: &'static str,
}

/// Everything needed to draw a journey.
#[derive(Debug, Clone)]
pub struct JourneyScene {
    pub width: f64,
    pub height: f64,
    pub title: Option<String>,
    pub title_pos: (f64, f64),
    pub sections: Vec<SectionBand>,
    pub tasks: Vec<TaskGlyph>,
    pub legend: Vec<LegendItem>,
    /// Face centres in order — the connecting journey path, which
    /// follows the faces up and down (the satisfaction curve).
    pub path: Vec<(f64, f64)>,
}

/// Width of `s` drawn at `size` px (bold when `bold`).
fn width_at(s: &str, size: f64, bold: bool) -> f64 {
    literal_width(s) * size / TEXT_CALIBRATION * if bold { BOLD } else { 1.0 }
}

/// Width of `s` as a journey draws it, at [`TEXT_CALIBRATION`]: every
/// text here is written verbatim (mermaid's journey prints tags as
/// they are), so a `<b>` or a `$$` is as wide as its characters, where
/// [`text_width`] would read them as styling and math fences that
/// measure nothing. `<` and `$` are the characters that start those;
/// each measures as a plain letter, as `a` does, and `text_width` of a
/// line without either is the plain per-character sum.
fn literal_width(s: &str) -> f64 {
    if s.contains(['<', '$']) {
        text_width(&s.replace(['<', '$'], "a"))
    } else {
        text_width(s)
    }
}

/// Task name `name` as drawn: wrapped at [`LABEL_WRAP_W`] (at [`FONT`])
/// into as few lines as it needs, evened out so no word is left
/// stranded on the last ([`crate::wrap::wrap_block`]); its tags stay
/// text.
fn wrap_label(name: &str) -> String {
    let scale = f64::from(FONT) / TEXT_CALIBRATION;
    crate::wrap::wrap_block(name, LABEL_WRAP_W, |t| literal_width(t) * scale, false)
}

/// Compute all geometry.
pub fn scene(d: &Journey) -> JourneyScene {
    let has_title = d.title.is_some();
    let has_legend = !d.actors.is_empty();
    let font = f64::from(FONT);

    // Flatten tasks in source order, with their section index and the
    // label as drawn (wrapped at LABEL_WRAP_W).
    let flat: Vec<(usize, &crate::model::JourneyTask)> = d
        .sections
        .iter()
        .enumerate()
        .flat_map(|(si, s)| s.tasks.iter().map(move |t| (si, t)))
        .collect();
    let labels: Vec<String> = flat.iter().map(|(_, t)| wrap_label(&t.name)).collect();

    // Per-task column width: fits the label and the actor-dot row.
    let mut widths: Vec<f64> = flat
        .iter()
        .zip(&labels)
        .map(|((_, t), label)| {
            let n = t.actors.len() as f64;
            let dots_w = n * 2.0 * DOT_R + (n - 1.0).max(0.0) * DOT_GAP;
            (width_at(label, font, false) + 20.0)
                .max(dots_w + 12.0)
                .max(COL_MIN)
        })
        .collect();
    // A section's columns together must fit its bold name; spread any
    // shortfall evenly over them so the name never spills out of its
    // band (where it would be lost, white-on-white or under the next
    // band).
    let mut k = 0usize;
    for s in &d.sections {
        let n = s.tasks.len();
        if n == 0 {
            continue;
        }
        let need = width_at(&s.name, font, true) + 24.0;
        let have: f64 = widths[k..k + n].iter().sum();
        if have < need {
            for w in &mut widths[k..k + n] {
                *w += (need - have) / n as f64;
            }
        }
        k += n;
    }
    // Columns left to right, with a gutter where a new section starts.
    let mut cols: Vec<(f64, f64)> = Vec::with_capacity(flat.len()); // (left, width)
    let mut x = PAD;
    for (i, &w) in widths.iter().enumerate() {
        if i > 0 && flat[i].0 != flat[i - 1].0 {
            x += SECTION_GAP;
        }
        cols.push((x, w));
        x += w;
    }
    let tasks_w = if flat.is_empty() {
        2.0 * PAD + COL_MIN
    } else {
        x + PAD
    };

    // The canvas must also fit the title and the actor legend, or they
    // overflow to negative x when the tasks are narrow (few short tasks
    // with many/long actor names, or a long title). A legend item is
    // dot + gap + name, then LEGEND_ITEM_GAP before the next; the last
    // item's trailing gap is not part of the row's visible width.
    const LEGEND_ITEM_GAP: f64 = 18.0;
    let item_w = |name: &str| 2.0 * DOT_R + 6.0 + width_at(name, font, false);
    let legend_vis: f64 = d.actors.iter().map(|a| item_w(a)).sum::<f64>()
        + d.actors.len().saturating_sub(1) as f64 * LEGEND_ITEM_GAP;
    let title_w = d
        .title
        .as_ref()
        .map_or(0.0, |t| width_at(t, TITLE_FONT, true) + 2.0 * PAD);
    let legend_w = if has_legend {
        legend_vis + 2.0 * PAD
    } else {
        0.0
    };
    let width = tasks_w.max(title_w).max(legend_w);
    // Centre the task row when the title or legend makes the canvas
    // wider than the tasks need.
    let shift = (width - tasks_w) / 2.0;
    for c in &mut cols {
        c.0 += shift;
    }

    // Vertical bands, top to bottom.
    let mut y = PAD;
    let title_pos = (width / 2.0, y + TITLE_H / 2.0);
    if has_title {
        y += TITLE_H;
    }
    let legend_y = y + LEGEND_H / 2.0;
    if has_legend {
        y += LEGEND_H;
    }
    let sec_y = y;
    y += SECTION_H + GAP;
    // Face centre for a score of 5; lower scores drop by SCORE_STEP.
    let face_top = y + DOTS_H + FACE_R;
    let face_cy = |score: f64| face_top + (5.0 - score) * SCORE_STEP;

    // Section bands span the columns of their tasks.
    let mut sections = Vec::new();
    let mut idx = 0usize;
    for (si, s) in d.sections.iter().enumerate() {
        if s.tasks.is_empty() {
            continue;
        }
        let first = idx;
        let last = idx + s.tasks.len() - 1;
        let x0 = cols[first].0;
        let x1 = cols[last].0 + cols[last].1;
        sections.push(SectionBand {
            x: x0,
            y: sec_y,
            w: x1 - x0,
            h: SECTION_H,
            name: s.name.clone(),
            color: section_color(si),
        });
        idx += s.tasks.len();
    }

    // Tasks: face + centred actor dots + label, all riding the score
    // height; collect the path.
    let mut tasks = Vec::with_capacity(flat.len());
    let mut path = Vec::with_capacity(flat.len());
    let mut bottom = face_cy(1.0) + FACE_R + LABEL_H;
    for (i, ((_, t), label)) in flat.iter().zip(labels).enumerate() {
        let cx = cols[i].0 + cols[i].1 / 2.0;
        let cy = face_cy(t.exact_score);
        path.push((cx, cy));
        let dots_cy = cy - FACE_R - DOTS_H / 2.0;
        let n = t.actors.len() as f64;
        let total_w = if n > 0.0 {
            n * 2.0 * DOT_R + (n - 1.0) * DOT_GAP
        } else {
            0.0
        };
        let mut dx = cx - total_w / 2.0 + DOT_R;
        let mut actor_dots = Vec::new();
        for &a in &t.actors {
            actor_dots.push((dx, dots_cy, actor_color(a)));
            dx += 2.0 * DOT_R + DOT_GAP;
        }
        let extra = (line_count(&label) - 1) as f64 * LINE_H;
        bottom = bottom.max(cy + FACE_R + LABEL_H + extra);
        tasks.push(TaskGlyph {
            cx,
            cy,
            r: FACE_R,
            score: t.score,
            color: score_color(t.score),
            name: t.name.clone(),
            label_pos: (cx, cy + FACE_R + 14.0),
            actor_dots,
            label,
            actor_ids: t.actors.clone(),
            exact_score: t.exact_score,
        });
    }
    let height = bottom + PAD;

    // Legend: centred row of colored dot + actor name. `width` already
    // accounts for `legend_vis`, so `lx` stays non-negative.
    let mut legend = Vec::new();
    if has_legend {
        let mut lx = (width - legend_vis) / 2.0;
        for (a, name) in d.actors.iter().enumerate() {
            legend.push(LegendItem {
                x: lx,
                y: legend_y,
                name: name.clone(),
                color: actor_color(a),
            });
            lx += item_w(name) + LEGEND_ITEM_GAP;
        }
    }

    JourneyScene {
        width,
        height,
        title: d.title.clone(),
        title_pos,
        sections,
        tasks,
        legend,
        path,
    }
}

/// Serialise a [`JourneyScene`] to a standalone SVG document.
pub fn to_svg(js: &JourneyScene) -> String {
    to_svg_with(js, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(js: &JourneyScene, opts: &SvgOptions) -> String {
    to_svg_a11y_with(js, &A11y::default(), opts)
}

/// [`to_svg_with`] plus mermaid's `accTitle` / `accDescr` (see
/// [`A11y`]). Without an `accTitle` the journey's own title is the
/// accessible name, then "User journey".
pub(crate) fn to_svg_a11y_with(js: &JourneyScene, a11y: &A11y, opts: &SvgOptions) -> String {
    let (title, desc) = a11y.resolve(js.title.as_deref().unwrap_or("User journey"));
    // Bands, faces and actor dots keep their colours in every theme;
    // the path, labels, title and legend follow it.
    let pal = opts.palette();
    let mut s = String::new();
    svg_open_a11y(&mut s, js.width, js.height, FONT, &title, desc, opts);

    // Section bands: pale fills with the name in the diagram ink.
    for b in &js.sections {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"6\" \
             fill=\"{}\"/>\n",
            b.x, b.y, b.w, b.h, b.color
        ));
        if !b.name.is_empty() {
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                 font-weight=\"bold\" fill=\"{}\">{}</text>\n",
                b.x + b.w / 2.0,
                b.y + b.h / 2.0,
                crate::style::text_on(b.color),
                escape(&b.name)
            ));
        }
    }

    // The journey path behind the faces.
    if js.path.len() >= 2 {
        let pts = js
            .path
            .iter()
            .map(|(x, y)| format!("{:.1},{:.1}", x, y))
            .collect::<Vec<_>>()
            .join(" ");
        s.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>\n",
            pts,
            pal.tuned(PATH_COLOR, &pal.guide)
        ));
    }

    // Faces + dots + labels. Each dot names its actor and each face its
    // task and score in a native tooltip, as mermaid's circles do.
    for t in &js.tasks {
        for (k, (dx, dy, color)) in t.actor_dots.iter().enumerate() {
            let tip = t
                .actor_ids
                .get(k)
                .and_then(|&a| js.legend.get(a))
                .map(|it| format!("<title>{}</title>", escape(&it.name)))
                .unwrap_or_default();
            let mark = t
                .actor_ids
                .get(k)
                .map_or(ActorMark::Circle, |&a| actor_mark(a));
            s.push_str(&dot_svg(*dx, *dy, color, mark, Some(&tip)));
        }
        s.push_str(&face_svg(t));
        // `label_pos` is the first line; the block centre sits lower.
        let lines = line_count(&t.label) as f64;
        svg_text_lines(
            &mut s,
            t.label_pos.0,
            t.label_pos.1 + (lines - 1.0) * LINE_H / 2.0,
            &pal.text,
            &t.label,
            "",
            true,
        );
    }

    // Title.
    if let Some(title) = &js.title {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
            js.title_pos.0,
            js.title_pos.1,
            TITLE_FONT,
            pal.text,
            escape(title)
        ));
    }

    // Legend.
    for (a, it) in js.legend.iter().enumerate() {
        s.push_str(&dot_svg(it.x + DOT_R, it.y, it.color, actor_mark(a), None));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" fill=\"{}\">{}</text>\n",
            it.x + 2.0 * DOT_R + 6.0,
            it.y,
            pal.text,
            escape(&it.name)
        ));
    }

    s.push_str("</svg>\n");
    s
}

/// SVG for one face: colored disc, two eyes, and a mouth that curves
/// from a frown (score 1) through flat (3) to a smile (5), grouped
/// under a `<title>` tooltip naming the task and its score.
fn face_svg(t: &TaskGlyph) -> String {
    let (cx, cy, r) = (t.cx, t.cy, t.r);
    let mut s = format!(
        "<g><title>{}: {}</title>\n<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\" \
         stroke=\"{}\" stroke-width=\"1.4\"/>\n",
        escape(&t.name),
        t.exact_score,
        cx,
        cy,
        r,
        t.color,
        FACE_STROKE
    );
    for ex in [cx - r * 0.32, cx + r * 0.32] {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\"/>\n",
            ex,
            cy - r * 0.18,
            r * 0.1,
            FACE_INK
        ));
    }
    // Mouth: quadratic curve; control-point offset tracks the score. A
    // frown's chord drops with its depth, so its apex stays below the
    // eyes instead of rising into them (a smile's chord stays put).
    let curv = (t.score as f64 - 3.0) * r * 0.28;
    let my = cy + r * 0.24 + (-curv).max(0.0) * 0.6;
    s.push_str(&format!(
        "<path d=\"M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}\" fill=\"none\" stroke=\"{}\" \
         stroke-width=\"1.6\" stroke-linecap=\"round\"/>\n</g>\n",
        cx - r * 0.42,
        my,
        cx,
        my + curv,
        cx + r * 0.42,
        my,
        FACE_INK
    ));
    s
}

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

/// The stroke width [`face_svg`] outlines a face with.
const FACE_STROKE_W: f64 = 1.4;

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
    /// task is its face (with its outline's stroke), its actor marks and its label's text box (each
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
                || in_box(
                    p,
                    text_box(it.x + 2.0 * DOT_R + 6.0, it.y, w, LINE_H, "start"),
                )
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
            in_disc(p, (t.cx, t.cy), t.r + FACE_STROKE_W / 2.0)
                || t.actor_dots.iter().enumerate().any(|(k, &(dx, dy, _))| {
                    let mark = t
                        .actor_ids
                        .get(k)
                        .map_or(ActorMark::Circle, |&a| actor_mark(a));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn journey(src: &str) -> Journey {
        match parse_document(src).unwrap() {
            Document::Journey(j) => j,
            _ => panic!("expected a journey"),
        }
    }

    #[test]
    fn parses_title_sections_tasks_actors() {
        let j = journey(
            "journey\n  title My day\n  section Work\n    Make tea: 5: Me\n    \
             Do work: 1: Me, Cat\n  section Home\n    Sit down: 3: Me\n",
        );
        assert_eq!(j.title.as_deref(), Some("My day"));
        assert_eq!(j.sections.len(), 2);
        assert_eq!(j.sections[0].tasks.len(), 2);
        assert_eq!(j.sections[0].tasks[0].score, 5);
        // Actors de-duplicated in first-appearance order.
        assert_eq!(j.actors, vec!["Me", "Cat"]);
        assert_eq!(j.sections[0].tasks[1].actors, vec![0, 1]);
    }

    #[test]
    fn score_is_clamped_and_tasks_before_section_get_a_leading_one() {
        let j = journey("journey\n  Lonely task: 9: A\n");
        assert_eq!(j.sections.len(), 1);
        assert_eq!(j.sections[0].name, "");
        assert_eq!(j.sections[0].tasks[0].score, 5, "9 clamps to 5");
    }

    #[test]
    fn missing_score_is_an_error() {
        let e = parse_document("journey\n  section S\n    Just a name\n").unwrap_err();
        assert!(e.message.contains("score"), "{}", e.message);
    }

    #[test]
    fn scene_places_everything_inside_the_canvas() {
        let js = scene(&journey(
            "journey\n  title T\n  section A\n    One: 1: Me\n    Two: 5: Me, You\n  \
             section B\n    Three: 3: You\n",
        ));
        assert!(js.width > 0.0 && js.height > 0.0);
        assert_eq!(js.tasks.len(), 3);
        assert_eq!(js.sections.len(), 2);
        assert_eq!(js.legend.len(), 2, "Me + You");
        assert_eq!(js.path.len(), 3);
        for t in &js.tasks {
            assert!(t.cx - t.r >= -0.01 && t.cx + t.r <= js.width + 0.01);
            assert!(t.cy + t.r <= js.height + 0.01);
        }
        for b in &js.sections {
            assert!(b.x >= -0.01 && b.x + b.w <= js.width + 0.01);
        }
    }

    #[test]
    fn wide_legend_or_title_widens_the_canvas() {
        // One tiny task, several long actor names: the canvas widens so
        // the legend fits — no negative-x overflow.
        let js = scene(&journey(
            "journey\n  A: 3: Alexander, Bartholomew, Christopher, Wolfeschlegelstein\n",
        ));
        for it in &js.legend {
            assert!(it.x >= -0.01, "legend item x {} went negative", it.x);
            assert!(it.x <= js.width + 0.01, "legend item x {} past width", it.x);
        }
        // A long title also widens the canvas.
        let js2 = scene(&journey(
            "journey\n  title A very very very long journey title goes here\n  \
             section S\n    T: 3: Me\n",
        ));
        assert!(js2.width >= text_width("A very very very long journey title goes here"));
    }

    #[test]
    fn to_svg_draws_faces_sections_and_legend() {
        let svg = to_svg(&scene(&journey(
            "journey\n  title Trip\n  section Go\n    Pack: 4: Me\n",
        )));
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains(">Trip<"), "title");
        assert!(svg.contains(">Go<"), "section name");
        assert!(svg.contains(">Pack<"), "task label");
        assert!(svg.contains("<path"), "mouth curve");
        assert!(svg.contains(score_color(4)), "score-4 face color");
    }

    /// Report Q5: section band labels hard-coded white on the band's
    /// accent (green 3.2:1, amber 2.6:1). Bands are now pale section
    /// fills; every label still reads on its own band.
    #[test]
    fn section_labels_read_on_every_band() {
        let n = SECTION_FILLS.len();
        let mut src = String::from("journey\ntitle T\n");
        for i in 0..n {
            src.push_str(&format!("section Sec{i}\n  Task{i}: 3: Me\n"));
        }
        let svg = crate::render_svg(&src).unwrap();
        for i in 0..n {
            let fill = section_color(i);
            let ink = crate::style::assert_text_reads(&svg, &format!("Sec{i}"), fill);
            assert_eq!(ink, crate::style::text_on(fill));
            // Far above AA: the bands are pale by design.
            assert!(crate::style::contrast_ratio(ink, fill).unwrap() >= 10.0);
        }
    }

    fn fixture() -> Journey {
        journey(include_str!("../examples/journey.mmd"))
    }

    /// Report journey flat-journey-path: every face sat on one
    /// baseline, so the satisfaction curve was a flat line. Faces (with
    /// their dots and labels) now drop SCORE_STEP per point below 5,
    /// and the path follows them.
    #[test]
    fn faces_trace_the_satisfaction_curve() {
        let js = scene(&fixture());
        let ys: Vec<f64> = js.tasks.iter().map(|t| t.cy).collect();
        // Scores 5, 3, 1, 5, 5.
        assert_eq!(ys[0], ys[3]);
        assert_eq!(ys[3], ys[4]);
        assert!((ys[1] - ys[0] - 2.0 * SCORE_STEP).abs() < 1e-9);
        assert!((ys[2] - ys[0] - 4.0 * SCORE_STEP).abs() < 1e-9);
        for (t, &(px, py)) in js.tasks.iter().zip(&js.path) {
            assert_eq!((px, py), (t.cx, t.cy), "path runs through the faces");
            // Dots ride above their face, the label below it.
            for &(_, dy, _) in &t.actor_dots {
                assert!(dy + DOT_R < t.cy - t.r, "dot clears the face");
                assert!(t.cy - t.r - (dy + DOT_R) >= 4.0, "dot breathing room");
            }
            assert!(t.label_pos.1 > t.cy + t.r);
            assert!(t.label_pos.1 + 7.0 <= js.height, "label inside the canvas");
        }
        let svg = to_svg(&js);
        assert!(!svg.contains("points=\"76.0,170.0"), "not a flat line");
        // A fractional score sits between its neighbours.
        let js = scene(&journey("journey\n  a: 3: Me\n  b: 3.5: Me\n  c: 4: Me\n"));
        let mid = (js.tasks[0].cy + js.tasks[2].cy) / 2.0;
        assert!((js.tasks[1].cy - mid).abs() < 1e-9);
    }

    /// Report journey score parsing: mermaid reads the score with
    /// `Number()`, so `3.5` and `-1` rendered there and failed here.
    #[test]
    fn fractional_and_out_of_range_scores_parse() {
        let j = journey("journey\n  a: 3.5: Me\n  b: -1: Me\n  c: 9: Me\n  d: 2.4: Me\n");
        let t = &j.sections[0].tasks;
        assert_eq!((t[0].score, t[0].exact_score), (4, 3.5));
        assert_eq!((t[1].score, t[1].exact_score), (1, 1.0), "-1 clamps to 1");
        assert_eq!((t[2].score, t[2].exact_score), (5, 5.0), "9 clamps to 5");
        assert_eq!((t[3].score, t[3].exact_score), (2, 2.4));
        for bad in ["x: NaN: Me", "x: three: Me", "x: : Me"] {
            let e = parse_document(&format!("journey\n  {bad}\n")).unwrap_err();
            assert!(
                e.message.contains("invalid journey score"),
                "{bad}: {}",
                e.message
            );
            assert_eq!(e.line, 2);
        }
        // The face tooltip shows the score as written.
        let svg = crate::render_svg("journey\n  Tea: 3.5: Me\n").unwrap();
        assert!(svg.contains("<title>Tea: 3.5</title>"), "{svg}");
    }

    /// Report journey palette-collision: section i and actor i shared
    /// accent(i), and the green actor/section matched the score-5
    /// face. Sections are pale fills, actors skip the score look-alikes.
    #[test]
    fn section_actor_and_score_colors_never_coincide() {
        let scores: Vec<&str> = (1..=5).map(score_color).collect();
        let actors: Vec<&str> = (0..ACTOR_ACCENTS.len()).map(actor_color).collect();
        for (i, a) in actors.iter().enumerate() {
            assert!(!actors[i + 1..].contains(a), "actor colour {a} repeats");
            assert!(!scores.contains(a));
            // Never green, amber or coral (score look-alikes).
            for k in [1, 2, 6] {
                assert_ne!(*a, crate::style::accent(k));
            }
        }
        for i in 0..SECTION_FILLS.len() {
            let f = section_color(i);
            assert!(!actors.contains(&f) && !scores.contains(&f));
            // Pale: recedes behind faces and dots.
            assert!(crate::style::relative_luminance(f).unwrap() > 0.6, "{f}");
        }
        // Nine actors: the ninth no longer repeats the first.
        assert_ne!(actor_color(8), actor_color(0));
    }

    /// Report journey abutting-section-bands: neighbouring bands shared
    /// an edge and their rounded corners cut notches at the join.
    #[test]
    fn section_bands_keep_a_gutter() {
        let js = scene(&fixture());
        assert_eq!(js.sections.len(), 2);
        for w in js.sections.windows(2) {
            let gap = w[1].x - (w[0].x + w[0].w);
            assert!((gap - SECTION_GAP).abs() < 1e-9, "gap {gap}");
        }
        // Faces stay inside their own band's span.
        let (a, b) = (&js.sections[0], &js.sections[1]);
        assert!(js.tasks[..3].iter().all(|t| t.cx > a.x && t.cx < a.x + a.w));
        assert!(js.tasks[3..].iter().all(|t| t.cx > b.x && t.cx < b.x + b.w));
    }

    /// Report journey long section name: the band was sized from its
    /// tasks only, so a long name spilled out (white on white, or under
    /// the next band). Columns now widen to fit the bold name.
    #[test]
    fn long_section_names_fit_their_band() {
        let name = "Commute to the office early in the morning";
        let js = scene(&journey(&format!(
            "journey\n  section {name}\n    Tea: 5: Me\n  section B\n    x: 3: Me\n"
        )));
        let band = &js.sections[0];
        let need = text_width(name) * 13.0 / TEXT_CALIBRATION * 1.06;
        assert!(band.w >= need + 20.0, "band {} < name {need}", band.w);
        assert!(js.sections[1].x > band.x + band.w);
        // The task sits centred under the widened band.
        assert!((js.tasks[0].cx - (band.x + band.w / 2.0)).abs() < 1e-9);
    }

    /// Report journey many actors: twelve dots overflowed the column
    /// and the canvas margin.
    #[test]
    fn many_actor_dots_fit_their_column() {
        let actors: Vec<String> = (1..=12).map(|i| format!("A{i}")).collect();
        let js = scene(&journey(&format!(
            "journey\n  section S\n    T: 2: {}\n    U: 4: Me\n",
            actors.join(", ")
        )));
        let t = &js.tasks[0];
        assert_eq!(t.actor_dots.len(), 12);
        let (lo, hi) = (js.sections[0].x, js.tasks[1].cx - COL_MIN / 2.0);
        for &(x, _, _) in &t.actor_dots {
            assert!(
                x - DOT_R >= lo && x + DOT_R <= hi,
                "dot {x} leaves its column"
            );
        }
        assert!(t.actor_dots[0].0 - DOT_R >= PAD);
    }

    /// Report journey long title: measured as 14px regular text but
    /// drawn 17px bold, so it was clipped at both edges.
    #[test]
    fn long_titles_fit_and_center_the_tasks() {
        let t = "A very long journey title that is wider than the tasks";
        let js = scene(&journey(&format!("journey\n  title {t}\n  a: 3: Me\n")));
        let need = text_width(t) * 17.0 / TEXT_CALIBRATION * 1.06;
        assert!(js.width >= need + 2.0 * PAD - 1e-9);
        // The single task is centred under the title.
        assert!((js.tasks[0].cx - js.width / 2.0).abs() < 1e-9);
    }

    /// Report journey legend-off-center: the trailing inter-item gap
    /// was counted in the centred width (~9px left of centre).
    #[test]
    fn legend_row_is_centred() {
        let js = scene(&fixture());
        let first = js.legend.first().unwrap();
        let last = js.legend.last().unwrap();
        let right = last.x + 2.0 * DOT_R + 6.0 + text_width(&last.name) * 13.0 / TEXT_CALIBRATION;
        assert!(((first.x + right) / 2.0 - js.width / 2.0).abs() < 1e-9);
        // Legend dots and task dots are one size.
        let svg = to_svg(&js);
        assert!(svg.contains(&format!("r=\"{DOT_R:.1}\" fill=\"{}\"/>", first.color)));
        assert!(svg.contains(&format!(
            "r=\"{DOT_R:.1}\" fill=\"{}\"><title>Me",
            first.color
        )));
    }

    /// Report journey long task names: a long name widened its column
    /// without bound. It now wraps at LABEL_WRAP_W.
    #[test]
    fn long_task_names_wrap() {
        let name = "Write a very long report about the quarterly results and send it";
        let js = scene(&journey(&format!("journey\n  {name}: 3: Me\n  b: 4: Me\n")));
        let t = &js.tasks[0];
        assert_eq!(t.name, name, "name stays as written");
        assert!(t.label.contains('\n'));
        assert_eq!(t.label.replace('\n', " "), name, "no words lost");
        for line in t.label.lines() {
            assert!(text_width(line) * 13.0 / TEXT_CALIBRATION <= LABEL_WRAP_W);
        }
        assert!(js.tasks[1].cx - t.cx <= (LABEL_WRAP_W + 20.0 + COL_MIN) / 2.0 + 1e-9);
        let lines = t.label.lines().count() as f64;
        let svg = to_svg(&js);
        assert!(svg.contains("<tspan"), "multi-line label");
        // The block's last line stays inside the canvas.
        assert!(t.label_pos.1 + (lines - 1.0) * LINE_H + 7.0 <= js.height);
        // Task text stays literal (mermaid prints tags verbatim).
        let svg = crate::render_svg("journey\n  a <b>b</b>: 3: Me\n").unwrap();
        assert!(svg.contains("a &lt;b&gt;b&lt;/b&gt;</text>"), "{svg}");
    }

    /// Phase A follow-up: task names wrapped greedily, stranding a last
    /// word ("… report for the | team"), and their tags — drawn verbatim
    /// — were measured as styling, worth nothing, so a name with tags
    /// overran its wrap width and its column.
    #[test]
    fn task_names_wrap_balanced_and_measure_their_tags() {
        let name = "Sit down and write the weekly report for the team";
        let js = scene(&journey(&format!("journey\n  {name}: 3: Me\n")));
        let label = &js.tasks[0].label;
        assert_eq!(label.replace('\n', " "), name);
        assert_ne!(label.lines().last(), Some("team"), "{label:?}");
        let tagged = "Make a <b>very strong cup of coffee</b> before the meeting";
        let js = scene(&journey(&format!("journey\n  {tagged}: 3: Me\n")));
        let t = &js.tasks[0];
        assert_eq!(t.label.replace('\n', " "), tagged);
        assert_eq!(t.label.matches("<b>").count(), 1, "tags stay text");
        let scale = 13.0 / TEXT_CALIBRATION;
        for line in t.label.lines() {
            assert!(literal_width(line) * scale <= LABEL_WRAP_W, "{line:?}");
        }
        // The tags count as the characters they are drawn as.
        assert!(literal_width("<b>x</b>") > text_width("<b>x</b>"));
        assert_eq!(literal_width("<b>x</b>"), text_width("ab>xa/b>"));
        assert_eq!(literal_width("plain"), text_width("plain"));
        // The column holds the widest line as drawn.
        let widest = t.label.lines().map(literal_width).fold(0.0, f64::max) * scale;
        let x0 = js.sections[0].x;
        assert!(t.cx - widest / 2.0 >= x0 - 1e-9, "label inside its band");
    }

    /// Phase A follow-up: the ten actor colours start over at the
    /// eleventh actor, whose dots then matched the first one's in the
    /// legend and on every task. Each later turn of the colours now
    /// takes another dot shape; up to ten actors the dots are the same
    /// discs as ever.
    #[test]
    fn repeated_actor_colours_take_another_mark() {
        for i in 0..10 {
            assert_eq!(actor_mark(i), ActorMark::Circle, "{i}");
        }
        assert_eq!(actor_mark(10), ActorMark::Square);
        assert_eq!(actor_mark(20), ActorMark::Diamond);
        assert_eq!(actor_mark(30), ActorMark::Triangle);
        assert_eq!(actor_mark(40), ActorMark::Circle);
        // (colour, mark) pairs never repeat in the first forty actors.
        let cues: Vec<_> = (0..40).map(|i| (actor_color(i), actor_mark(i))).collect();
        for (i, c) in cues.iter().enumerate() {
            assert!(!cues[i + 1..].contains(c), "actor {i} repeats {c:?}");
        }
        let names: Vec<String> = (1..=12).map(|i| format!("A{i}")).collect();
        let svg = crate::render_svg(&format!(
            "journey\n  section S\n    Plan: 5: {}\n    Ship: 2: A11, A1\n",
            names.join(", ")
        ))
        .unwrap();
        let (a1, a11) = (actor_color(0), actor_color(10));
        assert_eq!(a1, a11);
        // A1 stays a disc, A11 is a square: in the legend (untitled)...
        assert!(svg.contains(&format!("r=\"{DOT_R:.1}\" fill=\"{a1}\"/>")));
        assert!(svg.contains(&format!("rx=\"1\" fill=\"{a11}\"/>")), "{svg}");
        // ... and on the tasks, each named in its tooltip.
        assert!(
            svg.contains(&format!("fill=\"{a11}\"><title>A11</title></rect>")),
            "{svg}"
        );
        assert!(svg.contains(&format!("fill=\"{a1}\"><title>A1</title></circle>")));
        assert_eq!(svg.matches("<title>A11</title></rect>").count(), 2, "{svg}");
        // The square keeps within the dot's slot and about its area.
        let side = 1.8 * DOT_R;
        assert!(side <= 2.0 * DOT_R);
        assert!((side * side / (std::f64::consts::PI * DOT_R * DOT_R) - 1.0).abs() < 0.05);
        // Ten actors or fewer: no mark but the disc.
        let ten = crate::render_svg(&format!(
            "journey\n  section S\n    Plan: 5: {}\n",
            names[..10].join(", ")
        ))
        .unwrap();
        assert!(
            !ten.contains("rx=\"1\"") && !ten.contains("<polygon"),
            "{ten}"
        );
    }

    /// Report journey frown-cramped: the sad mouth rose from a high
    /// chord to eye level.
    #[test]
    fn frown_stays_below_the_eyes() {
        let js = scene(&journey("journey\n  sad: 1: Me\n"));
        let t = &js.tasks[0];
        let svg = face_svg(t);
        let d = &svg[svg.find("d=\"M ").unwrap() + 5..];
        let nums: Vec<f64> = d
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter_map(|v| v.parse().ok())
            .take(6)
            .collect();
        let (chord_y, ctrl_y) = (nums[1], nums[3]);
        // Quadratic apex = midpoint of chord and control point.
        let apex = (chord_y + ctrl_y) / 2.0;
        let eye_bottom = t.cy - t.r * 0.18 + t.r * 0.1;
        assert!(apex > eye_bottom + 2.0, "apex {apex} vs eyes {eye_bottom}");
        assert!(chord_y < t.cy + t.r * 0.8, "chord inside the face");
    }

    /// Report journey accessible name: accTitle/accDescr were dropped
    /// and the name was always "User journey"; dots had no tooltips.
    #[test]
    fn acc_statements_and_tooltips_reach_the_svg() {
        let svg = crate::render_svg(
            "journey\n  title Day\n  accTitle: My day, spoken\n  accDescr: Mostly good\n  \
             section S\n    Tea: 5: Me, Cat\n",
        )
        .unwrap();
        assert!(svg.contains("aria-label=\"My day, spoken\""));
        assert!(svg.contains(">Mostly good</desc>"));
        assert!(svg.contains("><title>Me</title></circle>"));
        assert!(svg.contains("><title>Cat</title></circle>"));
        assert!(svg.contains("<g><title>Tea: 5</title>"));
        let titled = crate::render_svg("journey\n  title Day\n  a: 3: Me\n").unwrap();
        assert!(titled.contains("aria-label=\"Day\""));
        let plain = crate::render_svg("journey\n  a: 3: Me\n").unwrap();
        assert!(plain.contains("aria-label=\"User journey\""));
    }
}
