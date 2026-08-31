//! Sankey diagram rendering: weighted flows drawn as ribbons whose
//! thickness is proportional to their value.
//!
//! Mirrors mermaid's `sankey-beta`, which wraps d3-sankey. Nodes are
//! placed in columns by depth, sized by the larger of their inflow and
//! outflow, ordered within a column to reduce ribbon crossings, and
//! joined by cubic-bezier ribbons.
//!
//! Like `pie`/`journey`/`timeline` there is nothing draggable and no
//! `route()`: [`scene`] computes every coordinate and [`to_svg`]
//! serialises it.

use crate::model::{SankeyAlignment, SankeyDiagram, SankeyLinkColor};
use crate::scene::{escape, svg_open, SvgOptions, TEXT_COLOR};
use crate::style::accent;

/// Canvas margin around the plot area.
pub const PAD: f64 = 24.0;
/// Gap between a node and its label.
pub const LABEL_GAP: f64 = 6.0;
/// Font size for node labels.
pub const FONT: u32 = 13;
/// Thinnest ribbon we will draw, so a tiny flow stays visible.
pub const MIN_RIBBON: f64 = 1.0;
/// Barycentre passes used to reduce ribbon crossings. Ordering nodes to
/// minimise crossings is NP-hard, so this is the usual heuristic; four
/// sweeps is where the layout stops visibly improving.
const ORDER_PASSES: usize = 4;

/// A placed node: a rectangle plus the text drawn beside it.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyNodeGlyph {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub color: &'static str,
    /// Column index, left to right.
    pub column: usize,
    /// Sum of the flows through this node.
    pub value: f64,
    /// Where the label sits, and which side of the node it hangs off.
    pub label_pos: (f64, f64),
    pub label_anchor: &'static str,
}

/// A placed ribbon. `thickness` is the flow's share of the canvas.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLinkGlyph {
    pub source: usize,
    pub target: usize,
    pub value: f64,
    /// Left end: x, and the ribbon's vertical centre.
    pub x0: f64,
    pub y0: f64,
    /// Right end.
    pub x1: f64,
    pub y1: f64,
    pub thickness: f64,
}

/// Everything needed to draw the diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyScene {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<SankeyNodeGlyph>,
    pub links: Vec<SankeyLinkGlyph>,
    pub link_color: SankeyLinkColor,
    pub show_values: bool,
    pub prefix: String,
    pub suffix: String,
}

/// Format a value the way mermaid does: trim a trailing `.0` so whole
/// numbers read as integers, then wrap in the configured affixes.
fn format_value(v: f64, prefix: &str, suffix: &str) -> String {
    let body = if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    };
    format!("{prefix}{body}{suffix}")
}

/// Assign every node a column. Depth is the longest path from a source,
/// which keeps a link always pointing rightwards. A cycle cannot extend
/// a path forever because each node is relaxed at most `n` times.
fn columns(d: &SankeyDiagram) -> Vec<usize> {
    let n = d.nodes.len();
    let mut depth = vec![0usize; n];
    for _ in 0..n {
        let mut changed = false;
        for l in &d.links {
            // A self-link cannot push a node past itself.
            if l.source != l.target && depth[l.target] < depth[l.source] + 1 {
                depth[l.target] = depth[l.source] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    depth
}

/// Push nodes around inside the column grid per `nodeAlignment`.
/// `justify` (mermaid's default) pulls every sink out to the last
/// column so the diagram's right edge is flush.
fn align(d: &SankeyDiagram, depth: &mut [usize], last: usize) {
    match d.config.node_alignment {
        SankeyAlignment::Left => {}
        SankeyAlignment::Justify | SankeyAlignment::Right => {
            let has_out: Vec<bool> = (0..d.nodes.len())
                .map(|i| d.links.iter().any(|l| l.source == i && l.source != l.target))
                .collect();
            for (i, dep) in depth.iter_mut().enumerate() {
                if !has_out[i] {
                    *dep = last;
                }
            }
        }
        SankeyAlignment::Center => {}
    }
}

/// Lay the diagram out. Coordinates are absolute and final.
pub fn scene(d: &SankeyDiagram) -> SankeyScene {
    let cfg = &d.config;
    let n = d.nodes.len();
    if n == 0 {
        return SankeyScene {
            width: PAD * 2.0,
            height: PAD * 2.0,
            nodes: Vec::new(),
            links: Vec::new(),
            link_color: cfg.link_color.clone(),
            show_values: cfg.show_values,
            prefix: cfg.prefix.clone(),
            suffix: cfg.suffix.clone(),
        };
    }

    let mut depth = columns(d);
    let last = depth.iter().copied().max().unwrap_or(0);
    align(d, &mut depth, last);
    let n_cols = depth.iter().copied().max().unwrap_or(0) + 1;

    // A node is as thick as the larger of what flows in and what flows
    // out — a node that only splits its input must not shrink.
    let value: Vec<f64> = (0..n)
        .map(|i| {
            let inflow: f64 = d.links.iter().filter(|l| l.target == i).map(|l| l.value).sum();
            let outflow: f64 = d.links.iter().filter(|l| l.source == i).map(|l| l.value).sum();
            inflow.max(outflow)
        })
        .collect();

    // Members of each column, in first-appearance order to start with.
    let mut cols: Vec<Vec<usize>> = vec![Vec::new(); n_cols];
    for (i, &c) in depth.iter().enumerate() {
        cols[c].push(i);
    }

    // Value -> pixels. Every column must fit, so take the tightest.
    let plot_h = (cfg.height - PAD * 2.0).max(40.0);
    let scale = cols
        .iter()
        .filter(|c| !c.is_empty())
        .map(|c| {
            let total: f64 = c.iter().map(|&i| value[i]).sum();
            let padding = cfg.node_padding * (c.len() as f64 - 1.0);
            if total > 0.0 {
                ((plot_h - padding).max(1.0)) / total
            } else {
                1.0
            }
        })
        .fold(f64::INFINITY, f64::min);
    let scale = if scale.is_finite() { scale } else { 1.0 };

    let height_of = |i: usize| (value[i] * scale).max(MIN_RIBBON);

    // Order within each column by the barycentre of a node's neighbours
    // in the previous column, which is the standard crossing-reduction
    // heuristic. Ties keep declaration order, so the layout is stable.
    let mut order: Vec<usize> = vec![0; n];
    for _ in 0..ORDER_PASSES {
        for c in 1..n_cols {
            let prev: Vec<usize> = cols[c - 1].clone();
            let pos_in_prev = |node: usize| prev.iter().position(|&p| p == node);
            let mut keyed: Vec<(f64, usize, usize)> = cols[c]
                .iter()
                .enumerate()
                .map(|(rank, &i)| {
                    let mut sum = 0.0;
                    let mut count = 0.0;
                    for l in &d.links {
                        if l.target == i {
                            if let Some(p) = pos_in_prev(l.source) {
                                sum += p as f64;
                                count += 1.0;
                            }
                        }
                    }
                    let bary = if count > 0.0 { sum / count } else { rank as f64 };
                    (bary, rank, i)
                })
                .collect();
            keyed.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.1.cmp(&b.1))
            });
            cols[c] = keyed.into_iter().map(|(_, _, i)| i).collect();
        }
    }
    for c in &cols {
        for (rank, &i) in c.iter().enumerate() {
            order[i] = rank;
        }
    }

    // Column x positions: the first column starts at the left margin,
    // the last ends at the right one.
    let plot_w = (cfg.width - PAD * 2.0).max(cfg.node_width * 2.0);
    let col_x = |c: usize| {
        if n_cols <= 1 {
            PAD
        } else {
            PAD + (plot_w - cfg.node_width) * (c as f64) / ((n_cols - 1) as f64)
        }
    };

    // Stack each column, centred vertically in the plot area.
    let mut nodes: Vec<SankeyNodeGlyph> = Vec::with_capacity(n);
    let mut y_of = vec![0.0f64; n];
    for members in &cols {
        let block: f64 = members.iter().map(|&i| height_of(i)).sum::<f64>()
            + cfg.node_padding * (members.len() as f64 - 1.0).max(0.0);
        let mut y = PAD + (plot_h - block).max(0.0) / 2.0;
        for &i in members {
            y_of[i] = y;
            y += height_of(i) + cfg.node_padding;
        }
    }
    for i in 0..n {
        let c = depth[i];
        let x = col_x(c);
        let h = height_of(i);
        // Labels hang outside the diagram at the edges and to the right
        // everywhere else, so they never sit on top of a ribbon.
        let (lx, anchor) = if c == n_cols - 1 && n_cols > 1 {
            (x - LABEL_GAP, "end")
        } else {
            (x + cfg.node_width + LABEL_GAP, "start")
        };
        nodes.push(SankeyNodeGlyph {
            label: d.nodes[i].clone(),
            x,
            y: y_of[i],
            w: cfg.node_width,
            h,
            color: accent(i),
            column: c,
            value: value[i],
            label_pos: (lx, y_of[i] + h / 2.0),
            label_anchor: anchor,
        });
    }

    // Ribbons leave and enter their nodes stacked in the same order the
    // nodes appear, so they fan out without crossing themselves.
    let mut out_cursor = vec![0.0f64; n];
    let mut in_cursor = vec![0.0f64; n];
    let mut idx: Vec<usize> = (0..d.links.len()).collect();
    idx.sort_by_key(|&k| {
        let l = &d.links[k];
        (order[l.source], order[l.target], k)
    });
    let mut placed: Vec<(usize, SankeyLinkGlyph)> = Vec::with_capacity(d.links.len());
    for &k in &idx {
        let l = &d.links[k];
        let t = (l.value * scale).max(MIN_RIBBON);
        let s_node = &nodes[l.source];
        let t_node = &nodes[l.target];
        let y0 = s_node.y + out_cursor[l.source] + t / 2.0;
        let y1 = t_node.y + in_cursor[l.target] + t / 2.0;
        out_cursor[l.source] += t;
        in_cursor[l.target] += t;
        placed.push((
            k,
            SankeyLinkGlyph {
                source: l.source,
                target: l.target,
                value: l.value,
                x0: s_node.x + s_node.w,
                y0,
                x1: t_node.x,
                y1,
                thickness: t,
            },
        ));
    }
    // Restore declaration order so the SVG is a stable function of the
    // input, not of the layout pass.
    placed.sort_by_key(|(k, _)| *k);
    let links: Vec<SankeyLinkGlyph> = placed.into_iter().map(|(_, g)| g).collect();

    let width = cfg.width.max(PAD * 2.0 + plot_w);
    let height = cfg.height.max(PAD * 2.0 + plot_h);
    SankeyScene {
        width,
        height,
        nodes,
        links,
        link_color: cfg.link_color.clone(),
        show_values: cfg.show_values,
        prefix: cfg.prefix.clone(),
        suffix: cfg.suffix.clone(),
    }
}

/// The ribbon outline: two cubic beziers with mirrored control points,
/// closed into a filled band.
fn ribbon_path(l: &SankeyLinkGlyph) -> String {
    let half = l.thickness / 2.0;
    let cx = (l.x0 + l.x1) / 2.0;
    let (top0, bot0) = (l.y0 - half, l.y0 + half);
    let (top1, bot1) = (l.y1 - half, l.y1 + half);
    format!(
        "M{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} \
         L{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} Z",
        l.x0, top0, cx, top0, cx, top1, l.x1, top1, l.x1, bot1, cx, bot1, cx, bot0, l.x0, bot0
    )
}

pub fn to_svg(ss: &SankeyScene) -> String {
    to_svg_with(ss, &SvgOptions::default())
}

pub fn to_svg_with(ss: &SankeyScene, opts: &SvgOptions) -> String {
    let mut s = String::new();
    svg_open(&mut s, ss.width, ss.height, FONT, "Sankey diagram", opts);

    // Gradient ids are derived from the link INDEX, never a counter, so
    // the same input always emits byte-identical SVG.
    if ss.link_color == SankeyLinkColor::Gradient && !ss.links.is_empty() {
        s.push_str("<defs>\n");
        for (i, l) in ss.links.iter().enumerate() {
            s.push_str(&format!(
                "<linearGradient id=\"fmsk{}\" gradientUnits=\"userSpaceOnUse\" \
                 x1=\"{:.1}\" x2=\"{:.1}\">\
                 <stop offset=\"0\" stop-color=\"{}\"/>\
                 <stop offset=\"1\" stop-color=\"{}\"/></linearGradient>\n",
                i,
                l.x0,
                l.x1,
                ss.nodes[l.source].color,
                ss.nodes[l.target].color
            ));
        }
        s.push_str("</defs>\n");
    }

    // Ribbons first, so nodes and labels sit on top of them.
    for (i, l) in ss.links.iter().enumerate() {
        let fill = match &ss.link_color {
            SankeyLinkColor::Gradient => format!("url(#fmsk{i})"),
            SankeyLinkColor::Source => ss.nodes[l.source].color.to_string(),
            SankeyLinkColor::Target => ss.nodes[l.target].color.to_string(),
            SankeyLinkColor::Fixed(c) => c.clone(),
        };
        s.push_str(&format!(
            "<path d=\"{}\" fill=\"{}\" fill-opacity=\"0.45\"><title>{} → {}: {}</title></path>\n",
            ribbon_path(l),
            fill,
            escape(&ss.nodes[l.source].label),
            escape(&ss.nodes[l.target].label),
            escape(&format_value(l.value, &ss.prefix, &ss.suffix))
        ));
    }

    for n in &ss.nodes {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\">\
             <title>{}</title></rect>\n",
            n.x,
            n.y,
            n.w,
            n.h,
            n.color,
            escape(&n.label)
        ));
        let text = if ss.show_values {
            format!(
                "{} ({})",
                n.label,
                format_value(n.value, &ss.prefix, &ss.suffix)
            )
        } else {
            n.label.clone()
        };
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"{}\" \
             fill=\"{}\">{}</text>\n",
            n.label_pos.0,
            n.label_pos.1,
            n.label_anchor,
            TEXT_COLOR,
            escape(&text)
        ));
    }

    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn diagram(src: &str) -> SankeyDiagram {
        match parse_document(src).unwrap() {
            Document::Sankey(d) => d,
            _ => panic!("not a sankey"),
        }
    }

    fn node<'a>(ss: &'a SankeyScene, label: &str) -> &'a SankeyNodeGlyph {
        ss.nodes.iter().find(|n| n.label == label).expect(label)
    }

    #[test]
    fn a_node_is_sized_by_the_larger_of_its_inflow_and_outflow() {
        // B takes 10 in and sends 4 out: it must stay 10 thick, or the
        // ribbons entering it would not fit.
        let ss = scene(&diagram("sankey-beta\nA,B,10\nB,C,4\n"));
        assert_eq!(node(&ss, "B").value, 10.0);
        assert_eq!(node(&ss, "A").value, 10.0);
        assert_eq!(node(&ss, "C").value, 4.0);
    }

    #[test]
    fn columns_follow_the_longest_path() {
        let ss = scene(&diagram("sankey-beta\nA,B,1\nB,C,1\nA,C,1\n"));
        assert_eq!(node(&ss, "A").column, 0);
        assert_eq!(node(&ss, "B").column, 1);
        // C is reachable in one hop and in two — the longer wins, so the
        // A->C ribbon still points rightwards.
        assert_eq!(node(&ss, "C").column, 2);
    }

    #[test]
    fn justify_pushes_sinks_to_the_last_column() {
        // D is a sink one hop in; justify (mermaid's default) flushes it
        // against the right edge alongside the deeper sink.
        let ss = scene(&diagram("sankey-beta\nA,B,1\nB,C,1\nA,D,1\n"));
        let last = ss.nodes.iter().map(|n| n.column).max().unwrap();
        assert_eq!(node(&ss, "D").column, last);
        assert_eq!(node(&ss, "C").column, last);
    }

    #[test]
    fn left_alignment_keeps_a_sink_at_its_own_depth() {
        let ss = scene(&diagram(
            "---\nconfig:\n  sankey:\n    nodeAlignment: left\n---\nsankey-beta\nA,B,1\nB,C,1\nA,D,1\n",
        ));
        assert_eq!(node(&ss, "D").column, 1);
        assert_eq!(node(&ss, "C").column, 2);
    }

    #[test]
    fn ribbon_thickness_tracks_value() {
        let ss = scene(&diagram("sankey-beta\nA,B,30\nA,C,10\n"));
        let thick = ss.links.iter().find(|l| l.value == 30.0).unwrap().thickness;
        let thin = ss.links.iter().find(|l| l.value == 10.0).unwrap().thickness;
        assert!(
            (thick / thin - 3.0).abs() < 0.01,
            "3x the flow should be 3x the ribbon: {thick} vs {thin}"
        );
    }

    #[test]
    fn ribbons_leave_and_enter_inside_their_nodes() {
        let ss = scene(&diagram("sankey-beta\nA,C,5\nB,C,5\nA,D,5\n"));
        for l in &ss.links {
            let s = &ss.nodes[l.source];
            let t = &ss.nodes[l.target];
            let half = l.thickness / 2.0;
            assert!(
                l.y0 - half >= s.y - 0.01 && l.y0 + half <= s.y + s.h + 0.01,
                "ribbon leaves outside its source node"
            );
            assert!(
                l.y1 - half >= t.y - 0.01 && l.y1 + half <= t.y + t.h + 0.01,
                "ribbon enters outside its target node"
            );
            assert!(l.x1 >= l.x0, "a ribbon must point rightwards");
        }
    }

    #[test]
    fn scene_fits_everything_inside_the_canvas() {
        let ss = scene(&diagram(
            "sankey-beta\nA,X,5\nB,X,7\nC,X,3\nX,Y,9\nX,Z,6\n",
        ));
        for n in &ss.nodes {
            assert!(n.x >= 0.0 && n.x + n.w <= ss.width + 0.01, "{} overflows x", n.label);
            assert!(n.y >= 0.0 && n.y + n.h <= ss.height + 0.01, "{} overflows y", n.label);
        }
    }

    #[test]
    fn a_cycle_terminates_and_still_lays_out() {
        // Longest-path depth would run forever on a cycle if it were not
        // capped; assert it returns and keeps every node.
        let ss = scene(&diagram("sankey-beta\nA,B,1\nB,C,1\nC,A,1\nA,A,1\n"));
        assert_eq!(ss.nodes.len(), 3);
        assert_eq!(ss.links.len(), 4);
    }

    #[test]
    fn empty_diagram_yields_an_empty_scene() {
        let ss = scene(&SankeyDiagram::default());
        assert!(ss.nodes.is_empty() && ss.links.is_empty());
        assert!(ss.width > 0.0 && ss.height > 0.0);
    }

    #[test]
    fn svg_is_deterministic_and_gradient_ids_come_from_link_index() {
        let d = diagram("sankey-beta\nA,B,3\nA,C,2\n");
        let a = to_svg(&scene(&d));
        let b = to_svg(&scene(&d));
        assert_eq!(a, b, "same input must give byte-identical SVG");
        assert!(a.contains("id=\"fmsk0\""), "{a}");
        assert!(a.contains("id=\"fmsk1\""), "{a}");
        assert!(a.contains("url(#fmsk0)"), "{a}");
    }

    #[test]
    fn flat_link_colour_emits_no_gradient() {
        let d = diagram("---\nconfig:\n  sankey:\n    linkColor: source\n---\nsankey-beta\nA,B,1\n");
        let svg = to_svg(&scene(&d));
        assert!(!svg.contains("linearGradient"), "{svg}");
        assert!(svg.contains(&format!("fill=\"{}\"", accent(0))), "{svg}");
    }

    #[test]
    fn fixed_link_colour_is_used_verbatim() {
        let d = diagram(
            "---\nconfig:\n  sankey:\n    linkColor: \"#abcdef\"\n---\nsankey-beta\nA,B,1\n",
        );
        assert!(to_svg(&scene(&d)).contains("fill=\"#abcdef\""));
    }

    #[test]
    fn show_values_toggles_the_number_beside_a_label() {
        let on = to_svg(&scene(&diagram("sankey-beta\nA,B,42\n")));
        assert!(on.contains("A (42)"), "{on}");
        let off = to_svg(&scene(&diagram(
            "---\nconfig:\n  sankey:\n    showValues: false\n---\nsankey-beta\nA,B,42\n",
        )));
        assert!(!off.contains("A (42)"), "{off}");
        assert!(off.contains(">A<"), "{off}");
    }

    #[test]
    fn affixes_wrap_the_value() {
        let svg = to_svg(&scene(&diagram(
            "---\nconfig:\n  sankey:\n    prefix: \"$\"\n    suffix: \"m\"\n---\nsankey-beta\nA,B,5\n",
        )));
        assert!(svg.contains("A ($5m)"), "{svg}");
    }

    #[test]
    fn labels_hang_outside_the_diagram_at_the_last_column() {
        let ss = scene(&diagram("sankey-beta\nA,B,1\n"));
        assert_eq!(node(&ss, "A").label_anchor, "start");
        assert_eq!(node(&ss, "B").label_anchor, "end");
    }

    #[test]
    fn labels_and_values_are_escaped() {
        let svg = to_svg(&scene(&diagram("sankey-beta\n\"A & <B>\",C,1\n")));
        assert!(svg.contains("A &amp; &lt;B&gt;"), "{svg}");
        assert!(!svg.contains("A & <B>"), "{svg}");
    }

    #[test]
    fn showcase_example_parses_and_renders() {
        let src = include_str!("../examples/sankey.mmd");
        let ss = scene(&diagram(src));
        assert!(ss.nodes.len() > 5 && ss.links.len() > 5);
        let svg = to_svg(&ss);
        assert!(svg.starts_with("<svg"), "{}", &svg[..40.min(svg.len())]);
        assert!(svg.ends_with("</svg>\n"));
    }
}
