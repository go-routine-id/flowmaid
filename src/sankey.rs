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

/// Format a value the way mermaid does: whole numbers read as integers,
/// fractions keep only as many places as they need.
///
/// Node totals are SUMS, so `0.1 + 0.2` arrives as `0.30000000000000004`
/// and Rust's shortest-roundtrip `Display` would print every digit of it.
/// Rounding to four places first, then trimming the zeros it leaves,
/// keeps ordinary input readable without inventing precision.
fn format_value(v: f64, prefix: &str, suffix: &str) -> String {
    let body = if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        let rounded = format!("{v:.4}");
        let trimmed = rounded.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    };
    format!("{prefix}{body}{suffix}")
}

/// Saturate a sum that overflowed. Individual link values are finite —
/// the parser insists — but adding them can still reach +inf, and an
/// infinite length becomes `height="inf"`, which no renderer can read.
/// Every place that sums values funnels through here.
fn saturate(v: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        f64::MAX
    }
}

/// A short, content-derived suffix for the gradient ids in one diagram.
///
/// Ids must be unique per diagram — two sankey SVGs inlined on one page
/// (the docs site does exactly that) would otherwise share `fmsk0`, and
/// every ribbon in the second would pick up the first's gradient. A
/// running counter would fix that but break the crate's byte-identical
/// output promise; a hash of the content keeps both, since the same
/// diagram always hashes the same and a different one almost never
/// collides.
fn scene_key(ss: &SankeyScene) -> String {
    // FNV-1a, 64-bit — a few lines, no dependency, and stable across
    // runs and platforms (unlike `DefaultHasher`, which is not).
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for n in &ss.nodes {
        eat(n.label.as_bytes());
        eat(&n.x.to_bits().to_le_bytes());
        eat(&n.y.to_bits().to_le_bytes());
    }
    for l in &ss.links {
        eat(&l.value.to_bits().to_le_bytes());
        eat(&l.y0.to_bits().to_le_bytes());
        eat(&l.y1.to_bits().to_le_bytes());
    }
    format!("{h:x}")
}

/// A total order over the nodes with cycle edges left out. A sankey is
/// meant to be acyclic — d3-sankey rejects a cycle outright — so rather
/// than fail, flowmaid drops only the edges that close a loop and lays
/// out what remains.
fn topo_order(d: &SankeyDiagram) -> Vec<usize> {
    let n = d.nodes.len();
    let mut indeg = vec![0usize; n];
    for l in &d.links {
        if l.source != l.target {
            indeg[l.target] += 1;
        }
    }
    let mut queue: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut head = 0;
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        order.push(u);
        for l in &d.links {
            if l.source == u && l.source != l.target {
                indeg[l.target] -= 1;
                if indeg[l.target] == 0 {
                    queue.push(l.target);
                }
            }
        }
    }
    // Whatever is left sits on a cycle. Appending it in declaration
    // order keeps the result a total order, and deterministic.
    let mut seen = vec![false; n];
    for &i in &order {
        seen[i] = true;
    }
    order.extend((0..n).filter(|&i| !seen[i]));
    order
}

/// Assign every node a column: the longest path from a source, measured
/// over forward edges only. Walking in topological order settles this in
/// one pass and bounds a depth at `n - 1`, so a cycle can no longer
/// inflate the column count.
fn columns(d: &SankeyDiagram) -> Vec<usize> {
    let n = d.nodes.len();
    let order = topo_order(d);
    let mut pos = vec![0usize; n];
    for (rank, &i) in order.iter().enumerate() {
        pos[i] = rank;
    }
    let mut depth = vec![0usize; n];
    for &u in &order {
        for l in &d.links {
            if l.source == u
                && l.source != l.target
                && pos[l.target] > pos[u]
                && depth[l.target] < depth[u] + 1
            {
                depth[l.target] = depth[u] + 1;
            }
        }
    }
    depth
}

/// Reposition nodes per `nodeAlignment`, following d3-sankey: `left`
/// keeps the computed depth, `justify` (the default) flushes every sink
/// against the last column, `right` pushes each node as far right as its
/// distance to a sink allows, and `center` pulls a source-less node up
/// against its earliest target.
fn align(d: &SankeyDiagram, depth: &mut [usize], last: usize) {
    let n = depth.len();
    let forward = |l: &crate::model::SankeyLink| l.source != l.target;
    match d.config.node_alignment {
        SankeyAlignment::Left => {}
        SankeyAlignment::Justify => {
            for i in 0..n {
                if !d.links.iter().any(|l| l.source == i && forward(l)) {
                    depth[i] = last;
                }
            }
        }
        SankeyAlignment::Right => {
            // Longest path onward to a sink, on the edges that already
            // point rightwards; mirrored to give the column.
            let mut to_sink = vec![0usize; n];
            for _ in 0..n {
                let mut changed = false;
                for l in &d.links {
                    if forward(l)
                        && depth[l.target] > depth[l.source]
                        && to_sink[l.source] < to_sink[l.target] + 1
                    {
                        to_sink[l.source] = to_sink[l.target] + 1;
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
            }
            for i in 0..n {
                depth[i] = last.saturating_sub(to_sink[i]);
            }
        }
        SankeyAlignment::Center => {
            let settled = depth.to_vec();
            for i in 0..n {
                if d.links.iter().any(|l| l.target == i && forward(l)) {
                    continue;
                }
                if let Some(first_target) = d
                    .links
                    .iter()
                    .filter(|l| l.source == i && forward(l))
                    .map(|l| settled[l.target])
                    .min()
                {
                    depth[i] = first_target.saturating_sub(1);
                }
            }
        }
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
            saturate(inflow.max(outflow))
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
            let total = saturate(c.iter().map(|&i| value[i]).sum());
            let padding = cfg.node_padding * (c.len() as f64 - 1.0);
            if total > 0.0 {
                (plot_h - padding).max(1.0) / total
            } else {
                // A column of zero-valued nodes constrains nothing. A
                // finite number here would PIN the scale for the whole
                // diagram, shrinking every other column to match.
                f64::INFINITY
            }
        })
        .fold(f64::INFINITY, f64::min);
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };

    // Ribbon thickness is clamped so a tiny flow stays visible. A node
    // must then be at least as tall as the ribbons it carries, or they
    // would hang outside the rectangle they claim to leave.
    let link_t: Vec<f64> = d
        .links
        .iter()
        .map(|l| (l.value * scale).max(MIN_RIBBON))
        .collect();
    let heights: Vec<f64> = (0..n)
        .map(|i| {
            let side = |pick: fn(&crate::model::SankeyLink) -> usize| -> f64 {
                saturate(
                    d.links
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| pick(l) == i)
                        .map(|(k, _)| link_t[k])
                        .sum(),
                )
            };
            saturate(
                (value[i] * scale)
                    .max(side(|l| l.target))
                    .max(side(|l| l.source))
                    .max(MIN_RIBBON),
            )
        })
        .collect();
    let height_of = |i: usize| heights[i];

    // Clamping can push a column past the configured canvas. Grow the
    // canvas to fit rather than draw nodes outside the viewport.
    let tallest = cols
        .iter()
        .map(|c| {
            saturate(
                c.iter().map(|&i| heights[i]).sum::<f64>()
                    + cfg.node_padding * (c.len() as f64 - 1.0).max(0.0),
            )
        })
        .fold(0.0, f64::max);
    let plot_h = plot_h.max(tallest);

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
        let t = link_t[k];
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

    // Individual link values are checked by the parser, but their SUM
    // can still overflow to +inf, and an infinite canvas is unparseable
    // SVG. Fall back to the configured size when that happens.
    let finite = |v: f64, fallback: f64| if v.is_finite() { v } else { fallback };
    let width = finite(cfg.width.max(PAD * 2.0 + plot_w), 600.0);
    let height = finite(cfg.height.max(PAD * 2.0 + plot_h), 400.0);
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
    let key = scene_key(ss);
    if ss.link_color == SankeyLinkColor::Gradient && !ss.links.is_empty() {
        s.push_str("<defs>\n");
        for (i, l) in ss.links.iter().enumerate() {
            s.push_str(&format!(
                "<linearGradient id=\"fmsk-{}-{}\" gradientUnits=\"userSpaceOnUse\" \
                 x1=\"{:.1}\" x2=\"{:.1}\">\
                 <stop offset=\"0\" stop-color=\"{}\"/>\
                 <stop offset=\"1\" stop-color=\"{}\"/></linearGradient>\n",
                key,
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
            SankeyLinkColor::Gradient => format!("url(#fmsk-{key}-{i})"),
            SankeyLinkColor::Source => ss.nodes[l.source].color.to_string(),
            SankeyLinkColor::Target => ss.nodes[l.target].color.to_string(),
            // Escaped like every other user string here: an
            // unescaped colour can close the attribute and inject
            // markup into the SVG.
            SankeyLinkColor::Fixed(c) => escape(c),
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
    fn svg_is_deterministic_and_gradient_ids_track_the_link_index() {
        let d = diagram("sankey-beta\nA,B,3\nA,C,2\n");
        let a = to_svg(&scene(&d));
        let b = to_svg(&scene(&d));
        assert_eq!(a, b, "same input must give byte-identical SVG");
        // Ids carry a content-derived namespace so two diagrams on one
        // page cannot collide, then the link index within the diagram.
        let key = scene_key(&scene(&d));
        assert!(a.contains(&format!("id=\"fmsk-{key}-0\"")), "{a}");
        assert!(a.contains(&format!("id=\"fmsk-{key}-1\"")), "{a}");
        assert!(a.contains(&format!("url(#fmsk-{key}-0)")), "{a}");
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
    fn a_zero_valued_column_does_not_pin_the_scale() {
        // The all-zero column constrains nothing. Treating it as
        // 1 px/unit used to shrink the whole diagram to ~28%.
        let ss = scene(&diagram("sankey-beta\nA,B,0\nB,C,100\n"));
        let tall = node(&ss, "C").h;
        assert!(
            tall > 300.0,
            "a zero column must not cap the scale — got height {tall}"
        );
    }

    #[test]
    fn a_fixed_link_colour_cannot_inject_markup() {
        // A colour is a user string like any other: unescaped, it can
        // close the fill attribute and add one of its own.
        let d = diagram(
            "---\nconfig:\n  sankey:\n    linkColor: red\" onload=x\n---\nsankey-beta\nA,B,1\n",
        );
        let svg = to_svg(&scene(&d));
        assert!(
            !svg.contains("\" onload=x"),
            "the colour escaped its attribute: {svg}"
        );
        assert!(svg.contains("fill=\"red&quot; onload=x\""), "{svg}");
    }

    #[test]
    fn a_cycle_keeps_the_column_count_bounded() {
        // Blind relaxation used to add a column per pass, so a 3-cycle
        // produced ten columns and shoved every node to the right edge.
        let ss = scene(&diagram("sankey-beta\nA,B,1\nB,C,1\nC,A,1\n"));
        let cols = ss.nodes.iter().map(|n| n.column).max().unwrap() + 1;
        assert!(cols <= ss.nodes.len(), "{cols} columns for 3 nodes");
        // Only the edge that closes the loop may point backwards.
        let backwards = ss.links.iter().filter(|l| l.x1 < l.x0).count();
        assert!(backwards <= 1, "{backwards} ribbons run backwards");
    }

    #[test]
    fn a_tall_column_grows_the_canvas_instead_of_clipping() {
        // Thirty sources into one sink: once ribbons are clamped up to
        // MIN_RIBBON the column outgrows the configured height.
        let mut src = String::from("sankey-beta\n");
        for i in 0..30 {
            src.push_str(&format!("S{i},Sink,1\n"));
        }
        let ss = scene(&diagram(&src));
        for n in &ss.nodes {
            assert!(
                n.y + n.h <= ss.height + 0.01,
                "{} ends at {} but the canvas is {}",
                n.label,
                n.y + n.h,
                ss.height
            );
        }
    }

    #[test]
    fn a_node_is_tall_enough_for_its_clamped_ribbons() {
        // One huge flow sets the scale; ten tiny ones then clamp up to
        // MIN_RIBBON and used to stack outside the node they leave.
        let mut src = String::from("sankey-beta\nX,Y,1000\n");
        for i in 0..10 {
            src.push_str(&format!("A,B{i},0.01\n"));
        }
        let ss = scene(&diagram(&src));
        for l in &ss.links {
            let s = &ss.nodes[l.source];
            let half = l.thickness / 2.0;
            assert!(
                l.y0 - half >= s.y - 0.01 && l.y0 + half <= s.y + s.h + 0.01,
                "a ribbon leaves outside {} (node {}..{}, ribbon {}..{})",
                s.label,
                s.y,
                s.y + s.h,
                l.y0 - half,
                l.y0 + half
            );
        }
    }

    #[test]
    fn every_alignment_places_nodes_differently() {
        // `right` and `center` were accepted, documented, then ignored.
        // X is a source whose only target sits three columns in, which
        // is where the four alignments disagree.
        let body = "sankey-beta\nA,B,1\nB,C,1\nC,D,1\nX,D,1\n";
        let col_of_x = |alignment: &str| -> usize {
            let src = format!(
                "---\nconfig:\n  sankey:\n    nodeAlignment: {alignment}\n---\n{body}"
            );
            node(&scene(&diagram(&src)), "X").column
        };
        // `left` leaves X at its own depth; `center` pulls it up against
        // its earliest target; `right` pushes it as far right as its
        // distance to a sink allows; `justify` only moves sinks, and X
        // has an outgoing link.
        assert_eq!(col_of_x("left"), 0);
        assert_eq!(col_of_x("justify"), 0);
        assert_eq!(col_of_x("center"), 2, "center must not fall back to left");
        assert_eq!(col_of_x("right"), 2, "right must not fall back to justify");
    }

    #[test]
    fn gradient_ids_are_namespaced_per_diagram() {
        // Two sankey SVGs inlined on one page (the docs site does this)
        // used to share `fmsk0`, so every ribbon in the second picked up
        // the first's gradient — wrong colours AND wrong geometry.
        let a = to_svg(&scene(&diagram("sankey-beta\nA,B,1\n")));
        let b = to_svg(&scene(&diagram("sankey-beta\nX,Y,5\n")));
        let id_of = |svg: &str| {
            let i = svg.find("id=\"fmsk-").expect("gradient id");
            svg[i..].split('"').nth(1).unwrap().to_string()
        };
        assert_ne!(id_of(&a), id_of(&b), "two diagrams share a gradient id");
        // Still a pure function of the content, so output stays
        // byte-identical across runs.
        assert_eq!(a, to_svg(&scene(&diagram("sankey-beta\nA,B,1\n"))));
    }

    #[test]
    fn an_overflowing_sum_still_renders_finite_geometry() {
        // Each link value passes the parser's finite check, but their
        // sum reaches +inf — which used to become `height="inf"`.
        let ss = scene(&diagram("sankey-beta\nA,B,1e308\nA,C,1e308\n"));
        for n in &ss.nodes {
            assert!(n.h.is_finite() && n.y.is_finite(), "{} is not finite", n.label);
            assert!(n.y + n.h <= ss.height + 0.01, "{} overflows the canvas", n.label);
        }
        assert!(ss.width.is_finite() && ss.height.is_finite());
        let svg = to_svg(&ss);
        assert!(!svg.contains("inf") && !svg.contains("NaN"), "{svg}");
    }

    #[test]
    fn computed_totals_do_not_leak_float_noise() {
        // Node totals are sums, so 0.1 + 0.2 arrives as
        // 0.30000000000000004 and used to be printed in full.
        let svg = to_svg(&scene(&diagram("sankey-beta\nA,B,0.1\nA,C,0.2\n")));
        assert!(svg.contains("A (0.3)"), "{svg}");
        assert!(!svg.contains("0.30000000000000004"), "{svg}");
        // A whole number still reads as an integer.
        assert!(to_svg(&scene(&diagram("sankey-beta\nA,B,42\n"))).contains("A (42)"));
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
