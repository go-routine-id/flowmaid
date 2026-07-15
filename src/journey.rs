//! User-journey rendering: a fixed horizontal band — no graph layout.
//! Tasks flow left to right in source order; each is a smiley face whose
//! color and mouth curve track its 1–5 satisfaction score (red frown →
//! green smile). Sections are colored bands spanning their tasks, a thin
//! path links the faces, and each participant gets a stable accent shown
//! in a legend and as per-task dots.
//!
//! Like `pie`/`mindmap` there is nothing draggable and no `route()`:
//! [`scene`] computes every coordinate and [`to_svg`] serialises it.

use crate::layout::text_width;
use crate::model::Journey;
use crate::scene::{escape, svg_open, TEXT_COLOR};
use crate::style::accent;

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
/// Face radius.
pub const FACE_R: f64 = 22.0;
/// Per-task actor dot radius + spacing + band height.
pub const DOT_R: f64 = 4.0;
pub const DOT_GAP: f64 = 4.0;
pub const DOTS_H: f64 = 16.0;
/// Room reserved under a face for its label.
pub const LABEL_H: f64 = 34.0;
/// Minimum column width per task.
pub const COL_MIN: f64 = 104.0;
/// Base font size.
pub const FONT: u32 = 13;
/// The line linking the faces (the journey path).
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

/// A colored band behind one section's tasks.
#[derive(Debug, Clone)]
pub struct SectionBand {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub name: String,
    pub color: &'static str,
}

/// One task: a face + label + its participant dots.
#[derive(Debug, Clone)]
pub struct TaskGlyph {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
    pub score: u8,
    pub color: &'static str,
    pub name: String,
    pub label_pos: (f64, f64),
    /// (x, y, color) per participating actor.
    pub actor_dots: Vec<(f64, f64, &'static str)>,
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
    /// Face centres in order — the connecting journey path.
    pub path: Vec<(f64, f64)>,
}

/// Compute all geometry.
pub fn scene(d: &Journey) -> JourneyScene {
    let has_title = d.title.is_some();
    let has_legend = !d.actors.is_empty();

    // Flatten tasks in source order, remembering the section boundaries.
    let flat: Vec<&crate::model::JourneyTask> =
        d.sections.iter().flat_map(|s| s.tasks.iter()).collect();

    // Per-task column: width fits the label, x accumulates left to right.
    let mut cols: Vec<(f64, f64)> = Vec::with_capacity(flat.len()); // (left, width)
    let mut centres: Vec<f64> = Vec::with_capacity(flat.len());
    let mut x = PAD;
    for t in &flat {
        let w = (text_width(&t.name) + 20.0).max(COL_MIN);
        centres.push(x + w / 2.0);
        cols.push((x, w));
        x += w;
    }
    let tasks_w = if flat.is_empty() { 2.0 * PAD + COL_MIN } else { x + PAD };

    // The canvas must also fit the title and the actor legend, or they
    // overflow to negative x when the tasks are narrow (few short tasks
    // with many/long actor names, or a long title).
    let item_w = |name: &str| 16.0 + text_width(name) + 18.0;
    let legend_total: f64 = d.actors.iter().map(|a| item_w(a)).sum();
    let title_w = d.title.as_ref().map_or(0.0, |t| text_width(t) + 2.0 * PAD);
    let legend_w = if has_legend { legend_total + 2.0 * PAD } else { 0.0 };
    let width = tasks_w.max(title_w).max(legend_w);

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
    let dots_cy = y + DOTS_H / 2.0;
    let face_cy = y + DOTS_H + FACE_R;
    let label_y = face_cy + FACE_R + 14.0;
    let height = face_cy + FACE_R + LABEL_H + PAD;

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
            color: accent(si),
        });
        idx += s.tasks.len();
    }

    // Tasks: face + centred actor dots + label; collect the path.
    let mut tasks = Vec::with_capacity(flat.len());
    let mut path = Vec::with_capacity(flat.len());
    for (i, t) in flat.iter().enumerate() {
        let cx = centres[i];
        path.push((cx, face_cy));
        let n = t.actors.len() as f64;
        let total_w = if n > 0.0 {
            n * 2.0 * DOT_R + (n - 1.0) * DOT_GAP
        } else {
            0.0
        };
        let mut dx = cx - total_w / 2.0 + DOT_R;
        let mut actor_dots = Vec::new();
        for &a in &t.actors {
            actor_dots.push((dx, dots_cy, accent(a)));
            dx += 2.0 * DOT_R + DOT_GAP;
        }
        tasks.push(TaskGlyph {
            cx,
            cy: face_cy,
            r: FACE_R,
            score: t.score,
            color: score_color(t.score),
            name: t.name.clone(),
            label_pos: (cx, label_y),
            actor_dots,
        });
    }

    // Legend: centred row of colored dot + actor name. `width` already
    // accounts for `legend_total`, so `lx` stays non-negative.
    let mut legend = Vec::new();
    if has_legend {
        let mut lx = (width - legend_total) / 2.0;
        for (a, name) in d.actors.iter().enumerate() {
            legend.push(LegendItem {
                x: lx,
                y: legend_y,
                name: name.clone(),
                color: accent(a),
            });
            lx += item_w(name);
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
    let mut s = String::new();
    svg_open(&mut s, js.width, js.height, FONT);

    // Section bands.
    for b in &js.sections {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"6\" \
             fill=\"{}\"/>\n",
            b.x, b.y, b.w, b.h, b.color
        ));
        if !b.name.is_empty() {
            s.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                 font-weight=\"bold\" fill=\"#ffffff\">{}</text>\n",
                b.x + b.w / 2.0,
                b.y + b.h / 2.0,
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
            pts, PATH_COLOR
        ));
    }

    // Faces + dots + labels.
    for t in &js.tasks {
        for (dx, dy, color) in &t.actor_dots {
            s.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\"/>\n",
                dx, dy, DOT_R, color
            ));
        }
        s.push_str(&face_svg(t));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             fill=\"{}\">{}</text>\n",
            t.label_pos.0,
            t.label_pos.1,
            TEXT_COLOR,
            escape(&t.name)
        ));
    }

    // Title.
    if let Some(title) = &js.title {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             font-weight=\"bold\" font-size=\"17\" fill=\"{}\">{}</text>\n",
            js.title_pos.0,
            js.title_pos.1,
            TEXT_COLOR,
            escape(title)
        ));
    }

    // Legend.
    for it in &js.legend {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"6\" fill=\"{}\"/>\n",
            it.x + 6.0,
            it.y,
            it.color
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" fill=\"{}\">{}</text>\n",
            it.x + 16.0,
            it.y,
            TEXT_COLOR,
            escape(&it.name)
        ));
    }

    s.push_str("</svg>\n");
    s
}

/// SVG for one face: colored disc, two eyes, and a mouth that curves
/// from a frown (score 1) through flat (3) to a smile (5).
fn face_svg(t: &TaskGlyph) -> String {
    let (cx, cy, r) = (t.cx, t.cy, t.r);
    let mut s = format!(
        "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\" stroke=\"{}\" \
         stroke-width=\"1.4\"/>\n",
        cx, cy, r, t.color, FACE_STROKE
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
    // Mouth: quadratic curve; control-point offset tracks the score.
    let my = cy + r * 0.24;
    let curv = (t.score as f64 - 3.0) * r * 0.28;
    s.push_str(&format!(
        "<path d=\"M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}\" fill=\"none\" stroke=\"{}\" \
         stroke-width=\"1.6\" stroke-linecap=\"round\"/>\n",
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
}
