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

use crate::layout::{text_width, TEXT_CALIBRATION};
use crate::model::{SankeyAlignment, SankeyDiagram, SankeyLabelStyle, SankeyLinkColor};
use crate::scene::{escape, svg_open, SvgOptions, TEXT_COLOR};
use crate::style::{accent_wide, css_rgb, ACCENTS, ACCENTS_DEEP};

/// Canvas margin around the plot area.
pub const PAD: f64 = 24.0;
/// Gap between a node and its label.
pub const LABEL_GAP: f64 = 6.0;
/// Font size for node labels — mermaid's 14px, which is also the size
/// [`text_width`] is calibrated at, so labels are measured exactly as
/// they are drawn.
pub const FONT: u32 = 14;
/// Height of a label's box for collision checks: one line of [`FONT`]
/// with its ascenders and descenders.
pub const LABEL_H: f64 = FONT as f64 * 1.25;
/// Clear space kept above and below a label box, against other labels
/// and nodes.
const LABEL_CLEAR: f64 = 2.0;
/// Clear space kept left and right of a label box. Two labels on one
/// baseline closer than twice this read as a single phrase
/// ("Electricity (124 TWh) Industry (48 TWh)"), so it is wider.
const LABEL_CLEAR_X: f64 = 8.0;
/// Thinnest ribbon we will draw, so a tiny flow stays visible.
pub const MIN_RIBBON: f64 = 1.0;
/// Barycentre sweep pairs (one forward, one backward) used to reduce
/// ribbon crossings. Ordering nodes to minimise crossings is NP-hard,
/// so this is the usual heuristic. Alternating sweeps can oscillate,
/// so the ordering with the fewest crossings seen is kept — the first
/// forward sweep's when nothing beats it, so more sweeps never cost a
/// crossing.
const ORDER_PASSES: usize = 4;
/// Two node colours closer than this (CIE76 ΔE) read as one hue at a
/// glance — indigo/steel blue (29) and green/teal (33) in the accent
/// palette — so linked or stacked nodes never get such a pair.
const CLASH_DE: f64 = 35.0;
/// Share of their configured span the columns may draw in to, to make
/// room for labels hanging off the first or last column, before the
/// canvas grows instead.
const MIN_SPAN_SHARE: f64 = 0.5;
/// Label passes (each after the columns draw in) before what still
/// hangs out widens the canvas.
const LABEL_FIT_PASSES: usize = 3;
/// Placement cost of a label hanging off the canvas where that is
/// allowed: more than a clean spot inside, less than any real overlap.
const HANG_COST: f64 = 0.5;
/// When the labels still do not fit the configured width, ones wider
/// than this share of the plot width wrap onto more lines...
const WRAP_SHARE: f64 = 0.3;
/// ...but never narrower than this (px).
const WRAP_MIN: f64 = 90.0;
/// Factor by which the columns spread apart per step while a label
/// between them still overprints another label or a node (see
/// `Placed::overprint`).
const SPREAD_STEP: f64 = 1.2;

/// A placed node: a rectangle plus the text drawn beside it.
///
/// `#[non_exhaustive]`: [`scene`] builds these; read their fields.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SankeyNodeGlyph {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Palette colour: distinct from every linked node and from the
    /// nodes directly above and below it (see [`scene`]).
    pub color: &'static str,
    /// Column index, left to right.
    pub column: usize,
    /// Sum of the flows through this node.
    pub value: f64,
    /// Where the label sits, and which side of the node it hangs off.
    /// The label pass keeps labels clear of each other, so `label_pos.1`
    /// may sit off the node's centre (always inside its vertical span).
    pub label_pos: (f64, f64),
    pub label_anchor: &'static str,
    /// The node's `config.sankey.nodeColors` entry, verbatim (any CSS
    /// colour); when set it is painted instead of `color`. See
    /// [`SankeyNodeGlyph::fill`].
    pub custom_color: Option<String>,
    /// The label text: the name, plus the value when `showValues` is on.
    pub label_text: String,
    /// The label as drawn, one entry per line, centred on `label_pos`
    /// [`LABEL_H`] apart: just `label_text`, unless the labels did not
    /// fit the configured width and this one was wrapped at spaces.
    pub label_lines: Vec<String>,
}

impl SankeyNodeGlyph {
    /// The colour to paint this node (and its gradient stops) with:
    /// the `nodeColors` override when there is one, else the palette.
    pub fn fill(&self) -> &str {
        self.custom_color.as_deref().unwrap_or(self.color)
    }
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
///
/// `#[non_exhaustive]`: [`scene`] builds it; read its fields.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SankeyScene {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<SankeyNodeGlyph>,
    pub links: Vec<SankeyLinkGlyph>,
    pub link_color: SankeyLinkColor,
    pub show_values: bool,
    pub prefix: String,
    pub suffix: String,
    /// Plain or haloed labels (`config.sankey.labelStyle`).
    pub label_style: SankeyLabelStyle,
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

/// A short, content-derived suffix for this diagram's gradient ids —
/// see [`crate::scene::DefsKey`] for why it is a hash and not a counter.
fn scene_key(ss: &SankeyScene) -> String {
    let mut k = crate::scene::DefsKey::new();
    for n in &ss.nodes {
        k.eat(n.label.as_bytes());
        k.eat_f64(n.x);
        k.eat_f64(n.y);
    }
    for l in &ss.links {
        k.eat_f64(l.value);
        k.eat_f64(l.y0);
        k.eat_f64(l.y1);
    }
    k.finish()
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
            label_style: cfg.label_style,
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
            let inflow: f64 = d
                .links
                .iter()
                .filter(|l| l.target == i)
                .map(|l| l.value)
                .sum();
            let outflow: f64 = d
                .links
                .iter()
                .filter(|l| l.source == i)
                .map(|l| l.value)
                .sum();
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

    // Order within each column by the barycentre of a node's neighbours
    // in the adjacent column — the standard crossing-reduction
    // heuristic. A forward sweep keys each column on its sources in the
    // column before; a backward sweep keys it on its targets in the
    // column after, which is the only way the FIRST column ever moves.
    // Ties keep the current order, so the layout is stable.
    let first_forward = barycentre_sweep(d, &mut cols, true);
    let mut best_x = crossings(d, &depth, &first_forward);
    let mut best = first_forward;
    for pass in 0..ORDER_PASSES {
        for forward in [false, true] {
            if pass == 0 && forward {
                // Already done above, as the baseline to beat.
                continue;
            }
            let ordered = barycentre_sweep(d, &mut cols, forward);
            let x = crossings(d, &depth, &ordered);
            // Strictly fewer only: a sweep that merely reshuffles keeps
            // the earlier ordering, so the extra sweeps can only ever
            // remove crossings, never add or move them.
            if x < best_x {
                best_x = x;
                best = ordered;
            }
        }
    }
    let cols = best;
    let mut order: Vec<usize> = vec![0; n];
    for c in &cols {
        for (rank, &i) in c.iter().enumerate() {
            order[i] = rank;
        }
    }

    // The label text of each node, measured at the size it is drawn.
    let label_text: Vec<String> = (0..n)
        .map(|i| {
            if cfg.show_values {
                format!(
                    "{} ({})",
                    d.nodes[i],
                    format_value(value[i], &cfg.prefix, &cfg.suffix)
                )
            } else {
                d.nodes[i].clone()
            }
        })
        .collect();
    let measure = |t: &str| text_width(t) * FONT as f64 / TEXT_CALIBRATION;
    let label_w: Vec<f64> = label_text.iter().map(|t| measure(t)).collect();

    // Vertical gap between two stacked nodes: the configured padding,
    // widened only where both nodes are so thin that their centred
    // labels would otherwise overprint each other.
    let gap = |a: usize, b: usize| {
        cfg.node_padding
            .max(LABEL_H + LABEL_CLEAR - (heights[a] + heights[b]) / 2.0)
    };
    let block_of = |members: &[usize]| -> f64 {
        saturate(
            members.iter().map(|&i| height_of(i)).sum::<f64>()
                + members.windows(2).map(|w| gap(w[0], w[1])).sum::<f64>(),
        )
    };

    // Clamping (and label room) can push a column past the configured
    // canvas. Grow the canvas to fit rather than draw nodes outside the
    // viewport.
    let tallest = cols.iter().map(|c| block_of(c)).fold(0.0, f64::max);
    let plot_h = plot_h.max(tallest);

    // Column x positions: the first column starts at `lo`, the last
    // ends at `hi` — the margins, unless labels need room (see below).
    let plot_w = (cfg.width - PAD * 2.0).max(cfg.node_width * 2.0);
    let col_x = |c: usize, lo: f64, hi: f64| {
        if n_cols <= 1 {
            lo
        } else {
            lo + (hi - lo - cfg.node_width) * (c as f64) / ((n_cols - 1) as f64)
        }
    };

    // Individual link values are checked by the parser, but their SUM
    // can still overflow to +inf, and an infinite canvas is unparseable
    // SVG. Fall back to the configured size when that happens.
    let finite = |v: f64, fallback: f64| if v.is_finite() { v } else { fallback };
    let height = finite(cfg.height.max(PAD * 2.0 + plot_h), 400.0);

    // Stack each column, centred vertically in the plot area.
    let mut y_of = vec![0.0f64; n];
    for members in &cols {
        let mut y = PAD + (plot_h - block_of(members)).max(0.0) / 2.0;
        for (k, &i) in members.iter().enumerate() {
            y_of[i] = y;
            y += height_of(i) + members.get(k + 1).map_or(0.0, |&j| gap(i, j));
        }
    }

    let custom: Vec<Option<String>> = (0..n)
        .map(|i| {
            cfg.node_colors
                .iter()
                .find(|(label, _)| *label == d.nodes[i])
                .map(|(_, c)| c.clone())
        })
        .collect();
    let colors = node_colors(d, &cols, &custom);

    let nodes: Vec<SankeyNodeGlyph> = (0..n)
        .map(|i| SankeyNodeGlyph {
            label: d.nodes[i].clone(),
            x: col_x(depth[i], PAD, PAD + plot_w),
            y: y_of[i],
            w: cfg.node_width,
            h: height_of(i),
            color: colors[i],
            column: depth[i],
            value: value[i],
            label_pos: (0.0, 0.0),
            label_anchor: "start",
            custom_color: custom[i].clone(),
            label_text: label_text[i].clone(),
            label_lines: vec![label_text[i].clone()],
        })
        .collect();
    // Labels of the first and last columns may hang outside the columns
    // (a long name that would otherwise run across the next column's
    // nodes). The canvas keeps its configured width while it can: the
    // columns draw in from the margins to make room, down to
    // MIN_SPAN_SHARE of their span; then long labels wrap; only what is
    // left over widens the canvas on that side. Labels between columns
    // cannot hang anywhere: while one still overprints a label or a
    // node, the columns spread further apart (see below).
    let fit = |nodes: &mut [SankeyNodeGlyph],
               label_w: &[f64],
               label_h: &[f64],
               span: f64,
               width: f64|
     -> Placed {
        let place = |nodes: &mut [SankeyNodeGlyph], lo: f64, hi: f64| {
            for nd in nodes.iter_mut() {
                nd.x = col_x(nd.column, lo, hi);
            }
            let sizes = Sizes {
                w: label_w,
                h: label_h,
            };
            place_labels(nodes, sizes, &order, n_cols, cfg.label_style, width, height)
        };
        let (mut lo, mut hi) = (PAD, PAD + span);
        let min_span = (span * MIN_SPAN_SHARE).max(cfg.node_width * 2.0).min(span);
        let mut got = place(nodes, lo, hi);
        'fit: for _ in 0..LABEL_FIT_PASSES {
            let over_l = (PAD - got.left).max(0.0);
            let over_r = (got.right - (width - PAD)).max(0.0);
            let room = (hi - lo) - min_span;
            if over_l + over_r < 0.5 || room < 0.5 {
                break;
            }
            // Drawing the columns in must not crowd the labels inside
            // them into each other: back off by halves until it does not.
            let mut take = (over_l + over_r).min(room);
            for _ in 0..4 {
                let (l, h) = (
                    lo + take * over_l / (over_l + over_r),
                    hi - take * over_r / (over_l + over_r),
                );
                let tried = place(nodes, l, h);
                if tried.crowding <= got.crowding {
                    (lo, hi, got) = (l, h, tried);
                    continue 'fit;
                }
                take /= 2.0;
            }
            got = place(nodes, lo, hi);
            break;
        }
        got
    };
    let overrun =
        |p: &Placed, width: f64| (PAD - p.left).max(0.0) + (p.right - (width - PAD)).max(0.0);
    let one_line = vec![LABEL_H; n];
    // The labels fitted with the columns spread over `span`: one line
    // each, or — when they still overrun the canvas or crowd each other —
    // the long ones wrapped (at spaces, the value kept whole), if that
    // does better on one count and no worse on the other. The wrap width
    // is the configured plot's share, however far the columns spread, so
    // wrapped labels stay narrow and need the least extra room. Returns
    // the nodes, what their labels achieved and the canvas width.
    let target = (plot_w * WRAP_SHARE).max(WRAP_MIN);
    let attempt = |span: f64| -> (Vec<SankeyNodeGlyph>, Placed, f64) {
        let width = finite(cfg.width.max(PAD * 2.0 + span), 600.0);
        let mut nodes = nodes.clone();
        let mut got = fit(&mut nodes, &label_w, &one_line, span, width);
        if overrun(&got, width) > 0.5 || got.crowding > 0.0 {
            let lines: Vec<Vec<String>> = (0..n)
                .map(|i| {
                    let value = cfg
                        .show_values
                        .then(|| format!("({})", format_value(value[i], &cfg.prefix, &cfg.suffix)));
                    wrap_label(&d.nodes[i], value.as_deref(), target, &measure)
                })
                .collect();
            let wrap_w: Vec<f64> = lines
                .iter()
                .map(|l| l.iter().map(|t| measure(t)).fold(0.0, f64::max))
                .collect();
            let wrap_h: Vec<f64> = lines.iter().map(|l| LABEL_H * l.len() as f64).collect();
            let mut trial = nodes.clone();
            let wrapped = fit(&mut trial, &wrap_w, &wrap_h, span, width);
            let (o_got, o_wrap) = (overrun(&got, width), overrun(&wrapped, width));
            let better = (o_wrap < o_got - 0.5 && wrapped.crowding <= got.crowding)
                || (wrapped.crowding < got.crowding && o_wrap <= o_got + 0.5);
            if better {
                for (nd, l) in trial.iter_mut().zip(lines) {
                    nd.label_lines = l;
                }
                nodes = trial;
                got = wrapped;
            }
        }
        (nodes, got, width)
    };
    let (mut nodes, mut got, mut width) = attempt(plot_w);
    // A label between two columns that still overprints a label or a
    // node (a long name at a narrow width) gets room by spreading the
    // columns apart, a step at a time, until none does — at most until
    // every gap holds the widest label from each side, where none can.
    // Each step keeps the configured width's preference for drawing in
    // and wrapping; a wider plot is kept only when it overprints less.
    // Labels that merely graze the clear margin above or below another
    // keep the configured width.
    if got.overprint > 0.0 && n_cols > 1 {
        let widest = label_w.iter().copied().fold(0.0, f64::max);
        let roomy = (n_cols - 1) as f64
            * (cfg.node_width + 2.0 * (widest + LABEL_GAP + LABEL_CLEAR_X))
            + cfg.node_width;
        let mut span = plot_w;
        while got.overprint > 0.0 && span < roomy {
            span = (span * SPREAD_STEP).min(roomy);
            let tried = attempt(span);
            if tried.1.overprint < got.overprint {
                (nodes, got, width) = tried;
            }
        }
    }
    let reach = (got.left, got.right);
    let shift = finite((PAD - reach.0).max(0.0), 0.0);
    if shift > 0.0 {
        for nd in &mut nodes {
            nd.x += shift;
            nd.label_pos.0 += shift;
        }
    }
    let width = finite((width + shift).max(reach.1 + shift + PAD), width);

    // Ribbons leave a node stacked in the vertical order of their
    // targets and enter it in the order of their sources, so they fan
    // out without crossing themselves — by position, not column rank,
    // so a ribbon that skips a column stacks correctly too.
    let centre = |i: usize| nodes[i].y + nodes[i].h / 2.0;
    let by_far_end = |pick_far: fn(&crate::model::SankeyLink) -> usize,
                      pick_near: fn(&crate::model::SankeyLink) -> usize| {
        let mut lists: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (k, l) in d.links.iter().enumerate() {
            lists[pick_near(l)].push(k);
        }
        for list in &mut lists {
            list.sort_by(|&a, &b| {
                centre(pick_far(&d.links[a]))
                    .partial_cmp(&centre(pick_far(&d.links[b])))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.cmp(&b))
            });
        }
        lists
    };
    let outgoing = by_far_end(|l| l.target, |l| l.source);
    let incoming = by_far_end(|l| l.source, |l| l.target);
    let mut y0 = vec![0.0f64; d.links.len()];
    let mut y1 = vec![0.0f64; d.links.len()];
    for i in 0..n {
        let mut cursor = nodes[i].y;
        for &k in &outgoing[i] {
            y0[k] = cursor + link_t[k] / 2.0;
            cursor += link_t[k];
        }
        let mut cursor = nodes[i].y;
        for &k in &incoming[i] {
            y1[k] = cursor + link_t[k] / 2.0;
            cursor += link_t[k];
        }
    }
    // Declaration order, so the SVG is a stable function of the input,
    // not of the layout pass.
    let links: Vec<SankeyLinkGlyph> = d
        .links
        .iter()
        .enumerate()
        .map(|(k, l)| {
            let (s_node, t_node) = (&nodes[l.source], &nodes[l.target]);
            SankeyLinkGlyph {
                source: l.source,
                target: l.target,
                value: l.value,
                x0: s_node.x + s_node.w,
                y0: y0[k],
                x1: t_node.x,
                y1: y1[k],
                thickness: link_t[k],
            }
        })
        .collect();

    SankeyScene {
        width,
        height,
        nodes,
        links,
        link_color: cfg.link_color.clone(),
        show_values: cfg.show_values,
        prefix: cfg.prefix.clone(),
        suffix: cfg.suffix.clone(),
        label_style: cfg.label_style,
    }
}

/// One barycentre sweep over every column. `forward` keys column `c` on
/// the mean rank of each node's sources in column `c - 1`, left to
/// right; backward keys it on its targets in column `c + 1`, right to
/// left. A node with no neighbour there keeps its current rank as its
/// key, and ties keep the current order. Returns the new ordering
/// (also left in `cols`).
fn barycentre_sweep(d: &SankeyDiagram, cols: &mut [Vec<usize>], forward: bool) -> Vec<Vec<usize>> {
    let n_cols = cols.len();
    let steps: Vec<(usize, usize)> = if forward {
        (1..n_cols).map(|c| (c, c - 1)).collect()
    } else {
        (0..n_cols.saturating_sub(1))
            .rev()
            .map(|c| (c, c + 1))
            .collect()
    };
    for (c, other) in steps {
        let rank_in_other: std::collections::HashMap<usize, usize> = cols[other]
            .iter()
            .enumerate()
            .map(|(r, &i)| (i, r))
            .collect();
        let mut keyed: Vec<(f64, usize, usize)> = cols[c]
            .iter()
            .enumerate()
            .map(|(rank, &i)| {
                let mut sum = 0.0;
                let mut count = 0.0;
                for l in &d.links {
                    let neighbour = if forward {
                        (l.target == i).then_some(l.source)
                    } else {
                        (l.source == i).then_some(l.target)
                    };
                    if let Some(p) = neighbour.and_then(|nb| rank_in_other.get(&nb)) {
                        sum += *p as f64;
                        count += 1.0;
                    }
                }
                let bary = if count > 0.0 {
                    sum / count
                } else {
                    rank as f64
                };
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
    cols.to_vec()
}

/// Ribbon crossings under an ordering: two links between the same pair
/// of columns cross when their source and target ranks are inverted.
fn crossings(d: &SankeyDiagram, depth: &[usize], cols: &[Vec<usize>]) -> usize {
    let mut rank = vec![0usize; depth.len()];
    for c in cols {
        for (r, &i) in c.iter().enumerate() {
            rank[i] = r;
        }
    }
    let links: Vec<_> = d
        .links
        .iter()
        .filter(|l| depth[l.source] < depth[l.target])
        .collect();
    let mut count = 0;
    for (a, la) in links.iter().enumerate() {
        for lb in &links[a + 1..] {
            if depth[la.source] != depth[lb.source] || depth[la.target] != depth[lb.target] {
                continue;
            }
            let ds = rank[la.source] as i64 - rank[lb.source] as i64;
            let dt = rank[la.target] as i64 - rank[lb.target] as i64;
            if ds * dt < 0 {
                count += 1;
            }
        }
    }
    count
}

/// CIE L*a*b* (D65) of a CSS colour, for [`CLASH_DE`]: hex, a named
/// colour or `rgb()` (see [`css_rgb`]); `None` for anything else.
fn lab(color: &str) -> Option<[f64; 3]> {
    let [r, g, b] = css_rgb(color)?.map(|c| {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
    let f = |t: f64| {
        if t > 216.0 / 24389.0 {
            t.cbrt()
        } else {
            (24389.0 / 27.0 * t + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    Some([116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)])
}

/// CIE76 colour difference; `None` when either colour is unreadable.
fn delta_e(a: &str, b: &str) -> Option<f64> {
    let (p, q) = (lab(a)?, lab(b)?);
    Some(((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt())
}

/// Palette colour per node. A colour is only ever judged against the
/// node's NEIGHBOURS — every node it shares a ribbon with (or a flat,
/// one-colour gradient would result) and the nodes directly above and
/// below it in its column — and must differ from each by at least
/// [`CLASH_DE`]. Among the colours that pass, one no other node uses
/// yet wins, so up to sixteen nodes ([`accent_wide`]) all read apart;
/// node `i` tries `accent_wide(i)` first, so a diagram without clashes
/// keeps the plain cycle. Greedy in declaration order, so deterministic.
/// `nodeColors` overrides count as their neighbours' colours.
fn node_colors(
    d: &SankeyDiagram,
    cols: &[Vec<usize>],
    custom: &[Option<String>],
) -> Vec<&'static str> {
    let n = d.nodes.len();
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut link = |a: usize, b: usize| {
        if a != b {
            if !neighbours[a].contains(&b) {
                neighbours[a].push(b);
            }
            if !neighbours[b].contains(&a) {
                neighbours[b].push(a);
            }
        }
    };
    for l in &d.links {
        link(l.source, l.target);
    }
    for c in cols {
        for w in c.windows(2) {
            link(w[0], w[1]);
        }
    }
    let palette = ACCENTS.len() + ACCENTS_DEEP.len();
    let mut colors: Vec<Option<&'static str>> = vec![None; n];
    for i in 0..n {
        // Colours already fixed around this node: overrides, and the
        // palette picks of neighbours placed before it.
        let around: Vec<String> = neighbours[i]
            .iter()
            .filter_map(|&j| custom[j].clone().or(colors[j].map(str::to_string)))
            .collect();
        // (neighbour's colour, near twin, already used, -ΔE, palette step)
        type Key = (u8, u8, u8, f64, usize);
        let mut best: Option<(Key, &'static str)> = None;
        for k in 0..palette {
            let c = accent_wide(i + k);
            let same = around.iter().any(|a| a.eq_ignore_ascii_case(c));
            let nearest = around
                .iter()
                .filter_map(|a| delta_e(a, c))
                .fold(f64::INFINITY, f64::min);
            let clash = nearest < CLASH_DE;
            let used = colors.iter().flatten().any(|u| *u == c);
            // Lexicographic: never a neighbour's colour, then no near
            // twin, then unused, then (only among clashes) the most
            // distinct, then palette order.
            let key = (
                u8::from(same),
                u8::from(clash),
                u8::from(used),
                if clash { -nearest } else { 0.0 },
                k,
            );
            let better = match &best {
                None => true,
                Some((b, _)) => key.partial_cmp(b) == Some(std::cmp::Ordering::Less),
            };
            if better {
                best = Some((key, c));
            }
        }
        colors[i] = best.map(|(_, c)| c);
    }
    colors
        .into_iter()
        .map(|c| c.unwrap_or(ACCENTS[0]))
        .collect()
}

/// Place every label so none overprints another or sits on another
/// node. A label starts beside its node's centre, on the right — or on
/// the left in the last column (mermaid's `outlined` style also puts
/// labels left of the busiest column on the left) — and, when that
/// spot is taken, slides along its own node's span in half-line steps,
/// then tries the other side. Thin nodes, which cannot slide, are
/// placed first; the first free spot wins, and when none is free the
/// least-overlapping one does. Nothing is measured against ribbons:
/// a label on a translucent ribbon reads fine.
///
/// The left of the first column and the right of the last are the only
/// places outside the canvas a label may go, and only when no clean spot
/// is left inside (a long name that would otherwise run across the next
/// column's nodes); the caller makes room for them.
fn place_labels(
    nodes: &mut [SankeyNodeGlyph],
    size: Sizes,
    order: &[usize],
    n_cols: usize,
    style: SankeyLabelStyle,
    width: f64,
    height: f64,
) -> Placed {
    type Rect = (f64, f64, f64, f64);
    let overlap = |a: Rect, b: Rect| -> f64 {
        let w = (a.2.min(b.2) - a.0.max(b.0)).max(0.0);
        let h = (a.3.min(b.3) - a.1.max(b.1)).max(0.0);
        w * h
    };
    // The busiest column: `outlined` labels left of it hang left.
    let central = nodes
        .iter()
        .fold((0.0f64, 0usize), |(v, c), nd| {
            if nd.value > v {
                (nd.value, nd.column)
            } else {
                (v, c)
            }
        })
        .1;
    let rects: Vec<Rect> = nodes
        .iter()
        .map(|nd| (nd.x, nd.y, nd.x + nd.w, nd.y + nd.h))
        .collect();
    let mut by_room: Vec<usize> = (0..nodes.len()).collect();
    by_room.sort_by(|&a, &b| {
        nodes[a]
            .h
            .partial_cmp(&nodes[b].h)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(nodes[a].column.cmp(&nodes[b].column))
            .then(order[a].cmp(&order[b]))
            .then(a.cmp(&b))
    });
    let mut placed: Vec<Rect> = Vec::with_capacity(nodes.len());
    let mut reach = (f64::INFINITY, f64::NEG_INFINITY);
    let mut crowding = 0.0;
    let mut overprint = 0.0;
    for &i in &by_room {
        let nd = &nodes[i];
        let first = n_cols > 1 && nd.column == 0;
        let last = n_cols > 1 && nd.column == n_cols - 1;
        let left_first = match style {
            SankeyLabelStyle::Legacy => last,
            SankeyLabelStyle::Outlined => last || nd.column < central,
        };
        let centre = nd.y + nd.h / 2.0;
        let (lw, lh) = (size.w[i], size.h[i]);
        let (lo, hi) = (nd.y + lh / 2.0, nd.y + nd.h - lh / 2.0);
        let mut ys = vec![centre];
        let step = LABEL_H / 2.0;
        let mut k = 1.0;
        while lo <= hi && (centre - k * step >= lo || centre + k * step <= hi) {
            for y in [centre - k * step, centre + k * step] {
                if (lo..=hi).contains(&y) {
                    ys.push(y);
                }
            }
            k += 1.0;
        }
        let mut best: Option<(f64, f64, bool, Rect)> = None;
        'search: for left in [left_first, !left_first] {
            for &y in &ys {
                let (x0, x1) = if left {
                    (nd.x - LABEL_GAP - lw, nd.x - LABEL_GAP)
                } else {
                    let x0 = nd.x + nd.w + LABEL_GAP;
                    (x0, x0 + lw)
                };
                let r: Rect = (x0, y - lh / 2.0, x1, y + lh / 2.0);
                let grown = (
                    r.0 - LABEL_CLEAR_X,
                    r.1 - LABEL_CLEAR,
                    r.2 + LABEL_CLEAR_X,
                    r.3 + LABEL_CLEAR,
                );
                let left_edge = if first && left {
                    f64::NEG_INFINITY
                } else {
                    0.0
                };
                let right_edge = if last && !left { f64::INFINITY } else { width };
                let outside =
                    (r.2 - r.0) * (r.3 - r.1) - overlap(r, (left_edge, 0.0, right_edge, height));
                // Hanging off the canvas is allowed there, but a clean
                // spot inside still comes first.
                let hangs = (first && left && r.0 < 0.0) || (last && !left && r.2 > width);
                let on_labels: f64 = placed.iter().map(|&p| overlap(grown, p)).sum();
                let on_nodes: f64 = rects
                    .iter()
                    .enumerate()
                    .filter(|&(j, _)| j != i)
                    .map(|(_, &q)| overlap(grown, q))
                    .sum();
                let cost =
                    4.0 * (outside + on_labels) + on_nodes + if hangs { HANG_COST } else { 0.0 };
                if best.as_ref().map_or(true, |b| cost < b.0) {
                    best = Some((cost, y, left, r));
                }
                if cost == 0.0 {
                    break 'search;
                }
            }
        }
        if let Some((cost, y, left, r)) = best {
            if cost > HANG_COST {
                crowding += cost;
                // Of that, what a reader sees: the label itself on a node
                // or on another label, or two labels on a baseline too
                // close to read apart — not the clear margins above and
                // below.
                let wide = (r.0 - LABEL_CLEAR_X, r.1, r.2 + LABEL_CLEAR_X, r.3);
                overprint += placed.iter().map(|&p| overlap(wide, p)).sum::<f64>()
                    + rects
                        .iter()
                        .enumerate()
                        .filter(|&(j, _)| j != i)
                        .map(|(_, &q)| overlap(r, q))
                        .sum::<f64>()
                    + (r.2 - r.0) * (r.3 - r.1)
                    - overlap(
                        r,
                        (
                            if first && left {
                                f64::NEG_INFINITY
                            } else {
                                0.0
                            },
                            0.0,
                            if last && !left { f64::INFINITY } else { width },
                            height,
                        ),
                    );
            }
            let nd = &mut nodes[i];
            nd.label_pos = if left {
                (nd.x - LABEL_GAP, y)
            } else {
                (nd.x + nd.w + LABEL_GAP, y)
            };
            nd.label_anchor = if left { "end" } else { "start" };
            reach = (reach.0.min(r.0), reach.1.max(r.2));
            placed.push(r);
        }
    }
    Placed {
        left: reach.0,
        right: reach.1,
        crowding,
        overprint,
    }
}

/// Each label's width and height (one [`LABEL_H`] per line), by node.
#[derive(Clone, Copy)]
struct Sizes<'a> {
    w: &'a [f64],
    h: &'a [f64],
}

/// `name` (and the `(value)` text, kept whole) broken at spaces into
/// lines no wider than `target` where it can be (a single word wider
/// than that keeps a line of its own); one line when it already fits.
fn wrap_label(
    name: &str,
    value: Option<&str>,
    target: f64,
    measure: &dyn Fn(&str) -> f64,
) -> Vec<String> {
    let whole = match value {
        Some(v) => format!("{name} {v}"),
        None => name.to_string(),
    };
    if measure(&whole) <= target {
        return vec![whole];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in name.split(' ').filter(|w| !w.is_empty()).chain(value) {
        let longer = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && measure(&longer) > target {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else {
            line = longer;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// What [`place_labels`] achieved: the leftmost and rightmost x any
/// label reaches, the summed cost of the labels that found no spot
/// clear of every other label and node (0 when all did), and how much
/// of that is visible overprinting (see [`place_labels`]).
struct Placed {
    left: f64,
    right: f64,
    crowding: f64,
    /// Area (px²) of labels drawn on another label or a node, or closer
    /// than twice [`LABEL_CLEAR_X`] to another label on their baseline,
    /// or off the canvas where they may not hang.
    overprint: f64,
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
                escape(ss.nodes[l.source].fill()),
                escape(ss.nodes[l.target].fill())
            ));
        }
        s.push_str("</defs>\n");
    }

    // Ribbons first, so nodes and labels sit on top of them.
    for (i, l) in ss.links.iter().enumerate() {
        let fill = match &ss.link_color {
            SankeyLinkColor::Gradient => format!("url(#fmsk-{key}-{i})"),
            // Escaped like every other user string here: an
            // unescaped colour (`nodeColors`, a fixed `linkColor`) can
            // close the attribute and inject markup into the SVG.
            SankeyLinkColor::Source => escape(ss.nodes[l.source].fill()),
            SankeyLinkColor::Target => escape(ss.nodes[l.target].fill()),
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
            escape(n.fill()),
            escape(&n.label)
        ));
    }
    // Labels last, above every node and ribbon. An `outlined` label is
    // drawn twice — a white stroked copy, then the text — rather than
    // with `paint-order`, which older SVG renderers ignore (they would
    // paint the white stroke OVER the glyphs).
    for n in &ss.nodes {
        let (x, y) = n.label_pos;
        let lines = n.label_lines.len();
        let (at, text) = if lines <= 1 {
            (
                format!(
                    "x=\"{x:.1}\" y=\"{y:.1}\" dy=\"0.33em\" text-anchor=\"{}\"",
                    n.label_anchor
                ),
                escape(&n.label_text),
            )
        } else {
            // A wrapped label: one <tspan> per line, the block centred
            // on `label_pos` as a single line would be.
            let first = 0.33 - 0.625 * (lines - 1) as f64;
            let spans: String = n
                .label_lines
                .iter()
                .enumerate()
                .map(|(k, line)| {
                    let dy = if k == 0 { first } else { 1.25 };
                    format!(
                        "<tspan x=\"{x:.1}\" dy=\"{dy:.3}em\">{}</tspan>",
                        escape(line)
                    )
                })
                .collect();
            (
                format!(
                    "x=\"{x:.1}\" y=\"{y:.1}\" text-anchor=\"{}\"",
                    n.label_anchor
                ),
                spans,
            )
        };
        if ss.label_style == SankeyLabelStyle::Outlined {
            s.push_str(&format!(
                "<text {at} fill=\"#ffffff\" stroke=\"#ffffff\" stroke-width=\"4\" \
                 stroke-linejoin=\"round\" aria-hidden=\"true\">{text}</text>\n"
            ));
        }
        s.push_str(&format!("<text {at} fill=\"{TEXT_COLOR}\">{text}</text>\n"));
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
        let ss = scene(&diagram("sankey-beta\nA,X,5\nB,X,7\nC,X,3\nX,Y,9\nX,Z,6\n"));
        for n in &ss.nodes {
            assert!(
                n.x >= 0.0 && n.x + n.w <= ss.width + 0.01,
                "{} overflows x",
                n.label
            );
            assert!(
                n.y >= 0.0 && n.y + n.h <= ss.height + 0.01,
                "{} overflows y",
                n.label
            );
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
        let d =
            diagram("---\nconfig:\n  sankey:\n    linkColor: source\n---\nsankey-beta\nA,B,1\n");
        let svg = to_svg(&scene(&d));
        assert!(!svg.contains("linearGradient"), "{svg}");
        assert!(
            svg.contains(&format!("fill=\"{}\"", accent_wide(0))),
            "{svg}"
        );
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
            let src =
                format!("---\nconfig:\n  sankey:\n    nodeAlignment: {alignment}\n---\n{body}");
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
            assert!(
                n.h.is_finite() && n.y.is_finite(),
                "{} is not finite",
                n.label
            );
            assert!(
                n.y + n.h <= ss.height + 0.01,
                "{} overflows the canvas",
                n.label
            );
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

    /// The showcase (examples/sankey.mmd) with a header that overrides
    /// its config: a narrow canvas, long names.
    fn showcase_with(config: &str, rename: &[(&str, &str)]) -> SankeyScene {
        let body = include_str!("../examples/sankey.mmd");
        let body = &body[body.find("sankey-beta").unwrap()..];
        let mut src = format!("---\nconfig:\n  sankey:\n    suffix: \" TWh\"\n{config}---\n{body}");
        for (from, to) in rename {
            src = src.replace(from, to);
        }
        scene(&diagram(&src))
    }

    type Rect = (f64, f64, f64, f64);

    /// A label's box as the label pass measures it (every line).
    fn label_box(n: &SankeyNodeGlyph) -> Rect {
        let w = n
            .label_lines
            .iter()
            .map(|l| text_width(l) * FONT as f64 / TEXT_CALIBRATION)
            .fold(0.0, f64::max);
        let h = LABEL_H * n.label_lines.len() as f64;
        let x0 = if n.label_anchor == "end" {
            n.label_pos.0 - w
        } else {
            n.label_pos.0
        };
        let y = n.label_pos.1;
        (x0, y - h / 2.0, x0 + w, y + h / 2.0)
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
    }

    /// Every label clear of every other label (with room to read them
    /// apart on one baseline), of every other node, and on the canvas.
    fn assert_labels_clear(ss: &SankeyScene) {
        for (i, a) in ss.nodes.iter().enumerate() {
            let ra = label_box(a);
            assert!(
                ra.0 >= 0.0 && ra.2 <= ss.width && ra.1 >= 0.0 && ra.3 <= ss.height,
                "{:?} leaves the canvas: {ra:?} in {}x{}",
                a.label_text,
                ss.width,
                ss.height
            );
            let mid = a.y + a.h / 2.0;
            assert!(
                (a.label_pos.1 - mid).abs() < 0.01
                    || (a.label_pos.1 >= a.y && a.label_pos.1 <= a.y + a.h),
                "{:?} strays off its own node",
                a.label_text
            );
            for (j, b) in ss.nodes.iter().enumerate() {
                if i == j {
                    continue;
                }
                let grown = (ra.0 - 8.0, ra.1, ra.2 + 8.0, ra.3);
                assert!(
                    !overlaps(grown, label_box(b)),
                    "{:?} runs into {:?}",
                    a.label_text,
                    b.label_text
                );
                assert!(
                    !overlaps(ra, (b.x, b.y, b.x + b.w, b.y + b.h)),
                    "{:?} is drawn over node {}",
                    a.label_text,
                    b.label
                );
            }
        }
    }

    /// Report §3 (sankey): 'Electricity (124 TWh)' printed over
    /// 'Industry (48 TWh)' at `width: 450`, and a long sink name ran
    /// across the middle column — labels were never measured.
    #[test]
    fn labels_never_overprint_even_when_narrow_or_long() {
        assert_labels_clear(&showcase_with("", &[]));
        assert_labels_clear(&showcase_with("    width: 450\n", &[]));
        let long = showcase_with(
            "",
            &[(
                "Electricity,Industry,",
                "Electricity,Heavy industry and manufacturing,",
            )],
        );
        assert_labels_clear(&long);
        // The long sink name wraps instead, and the canvas keeps its
        // configured width.
        let heavy = node(&long, "Heavy industry and manufacturing");
        assert!(heavy.label_lines.len() >= 2, "{heavy:?}");
        assert_eq!(long.width, 600.0);
        // At the default size, Electricity and Industry no longer share
        // a baseline (they read as one phrase).
        let ss = showcase_with("", &[]);
        let (e, i) = (node(&ss, "Electricity"), node(&ss, "Industry"));
        assert!(
            (e.label_pos.1 - i.label_pos.1).abs() >= LABEL_H,
            "{e:?} / {i:?}"
        );
    }

    #[test]
    fn thin_stacked_nodes_keep_room_for_their_labels() {
        // Thirty 1px sources: the configured 12px padding is less than a
        // line of text, so the gap widens instead of the labels piling up.
        let mut src = String::from("sankey-beta\n");
        for i in 0..30 {
            src.push_str(&format!("S{i},Sink,1\n"));
        }
        src.push_str("Big,Sink,1000\n");
        assert_labels_clear(&scene(&diagram(&src)));
    }

    /// Ribbon crossings in a finished scene, from the drawn ribbons:
    /// two ribbons between the same columns cross when their ends swap
    /// vertical order.
    fn drawn_crossings(ss: &SankeyScene) -> usize {
        let mut count = 0;
        for (a, la) in ss.links.iter().enumerate() {
            for lb in &ss.links[a + 1..] {
                let cols =
                    |l: &SankeyLinkGlyph| (ss.nodes[l.source].column, ss.nodes[l.target].column);
                if cols(la) != cols(lb) || la.source == lb.source || la.target == lb.target {
                    continue;
                }
                if (la.y0 - lb.y0) * (la.y1 - lb.y1) < 0.0 {
                    count += 1;
                }
            }
        }
        count
    }

    /// Report §3 / Q16: forward-only barycentre never reordered the first
    /// column, leaving Coal→Electricity across Gas→Heating.
    #[test]
    fn backward_sweep_removes_the_showcase_crossing() {
        let ss = showcase_with("", &[]);
        assert_eq!(drawn_crossings(&ss), 0);
        // Gas (which feeds Heating, at the top) moved above Coal.
        assert!(node(&ss, "Gas").y < node(&ss, "Coal").y);
    }

    #[test]
    fn extra_sweeps_never_add_a_crossing() {
        // A spread of small diagrams: the kept ordering is never worse
        // than the single forward sweep the layout used to stop at.
        for seed in 0..60u64 {
            let mut x = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let mut next = |m: u64| {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (x >> 33) % m
            };
            let mut src = String::from(
                "---\nconfig:\n  sankey:\n    nodeAlignment: left\n---\nsankey-beta\n",
            );
            for _ in 0..(6 + next(10)) {
                let layer = next(3);
                let (a, b) = (next(4), next(4));
                src.push_str(&format!(
                    "L{layer}N{a},L{}N{b},{}\n",
                    layer + 1,
                    1 + next(9)
                ));
            }
            let d = diagram(&src);
            let ss = scene(&d);
            let depth: Vec<usize> = ss.nodes.iter().map(|n| n.column).collect();
            let n_cols = depth.iter().max().unwrap() + 1;
            let mut cols: Vec<Vec<usize>> = vec![Vec::new(); n_cols];
            for (i, &c) in depth.iter().enumerate() {
                cols[c].push(i);
            }
            let forward_only = barycentre_sweep(&d, &mut cols, true);
            let mut kept: Vec<Vec<usize>> = vec![Vec::new(); n_cols];
            let mut by_y: Vec<usize> = (0..ss.nodes.len()).collect();
            by_y.sort_by(|&a, &b| ss.nodes[a].y.partial_cmp(&ss.nodes[b].y).unwrap());
            for i in by_y {
                kept[depth[i]].push(i);
            }
            assert!(
                crossings(&d, &depth, &kept) <= crossings(&d, &depth, &forward_only),
                "seed {seed}: {src}"
            );
        }
    }

    /// Report §2.11 / Q16: `accent(i % 8)` painted Losses like
    /// Electricity (a flat green ribbon) and Homes (indigo) right above
    /// Industry (steel blue).
    #[test]
    fn linked_and_stacked_nodes_never_share_a_hue() {
        let check = |ss: &SankeyScene| {
            let clash = |a: usize, b: usize| {
                let (ca, cb) = (ss.nodes[a].fill(), ss.nodes[b].fill());
                assert_ne!(ca, cb, "{} / {}", ss.nodes[a].label, ss.nodes[b].label);
                if let Some(de) = delta_e(ca, cb) {
                    assert!(
                        de >= CLASH_DE,
                        "{} {ca} / {} {cb}: ΔE {de:.1}",
                        ss.nodes[a].label,
                        ss.nodes[b].label
                    );
                }
            };
            for l in &ss.links {
                if l.source != l.target {
                    clash(l.source, l.target);
                }
            }
            for (a, na) in ss.nodes.iter().enumerate() {
                // The node directly below in the same column.
                let below = ss
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, nb)| nb.column == na.column && nb.y > na.y)
                    .min_by(|x, y| x.1.y.partial_cmp(&y.1.y).unwrap());
                if let Some((b, _)) = below {
                    clash(a, b);
                }
            }
        };
        let ss = showcase_with("", &[]);
        check(&ss);
        // Eleven nodes, eleven different colours.
        let mut fills: Vec<&str> = ss.nodes.iter().map(|n| n.fill()).collect();
        fills.sort_unstable();
        fills.dedup();
        assert_eq!(fills.len(), ss.nodes.len());
        // A dense one: twenty nodes, a hub with eighteen neighbours.
        let mut src = String::from("sankey-beta\n");
        for i in 0..9 {
            src.push_str(&format!("In{i},Hub,{}\nHub,Out{i},{}\n", 1 + i, 9 - i));
        }
        check(&scene(&diagram(&src)));
    }

    #[test]
    fn node_colors_override_the_palette_and_are_escaped() {
        let d = diagram(
            "---\nconfig:\n  sankey:\n    nodeColors:\n      A: \"#33a35c\"\n      \
             B: 'red\" onload=x'\n---\nsankey-beta\nA,B,1\nA,C,1\n",
        );
        let ss = scene(&d);
        assert_eq!(node(&ss, "A").fill(), "#33a35c");
        assert_eq!(node(&ss, "A").custom_color.as_deref(), Some("#33a35c"));
        // C keeps the palette but steers clear of A's custom green.
        let c = node(&ss, "C");
        assert!(c.custom_color.is_none());
        assert!(delta_e(c.fill(), "#33a35c").unwrap() >= CLASH_DE);
        let svg = to_svg(&ss);
        assert!(svg.contains("fill=\"#33a35c\""), "{svg}");
        assert!(
            svg.contains("stop-color=\"#33a35c\""),
            "gradients follow: {svg}"
        );
        assert!(!svg.contains("\" onload=x"), "{svg}");
    }

    /// Review: at `width: 450` with long names a first-column label was
    /// drawn across the middle column's node (there was no free spot and
    /// the least-cost one won), the middle label clipped the last
    /// column's nodes, and the canvas grew to 733px past the configured
    /// width. Labels may now hang off the first column too, the columns
    /// draw in to make room, and labels that still do not fit wrap.
    #[test]
    fn a_narrow_canvas_keeps_its_width_and_every_label_clear() {
        let src = "---\nconfig:\n  sankey:\n    width: 450\n    showValues: true\n    \
                   suffix: \" TWh\"\n---\nsankey-beta\n\
                   Coal and lignite from the north,Electricity generation,42\n\
                   Natural gas imports,Electricity generation,31\n\
                   Natural gas imports,District heating,18\n\
                   Nuclear,Electricity generation,25\n\
                   Offshore wind,Electricity generation,17\n\
                   Solar,Electricity generation,9\n\
                   Electricity generation,Heavy industry and manufacturing,48\n\
                   Electricity generation,Residential homes,40\n\
                   Electricity generation,Transmission losses,36\n\
                   District heating,Residential homes,12\n\
                   District heating,\"Industry, light\",6\n";
        let ss = scene(&diagram(src));
        assert_eq!(ss.width, 450.0);
        assert_labels_clear(&ss);
        let coal = node(&ss, "Coal and lignite from the north");
        assert!(coal.label_lines.len() >= 2, "{coal:?}");
        // Wrapped lines keep the whole value together and add up to
        // the label text.
        assert_eq!(coal.label_lines.join(" "), coal.label_text);
        assert!(coal.label_lines.last().unwrap().starts_with('('));
        let svg = to_svg(&ss);
        assert!(svg.contains("<tspan x="), "{svg}");
        // Short labels stay on one line, drawn as before.
        assert_eq!(node(&ss, "Nuclear").label_lines.len(), 1);
        assert!(
            svg.contains("dy=\"0.33em\" text-anchor=\"start\" fill=\"#232840\">Nuclear (25 TWh)<")
        );
    }

    /// Four columns of 17 nodes with long names (verifier input): at
    /// `width: 350` the canvas widened for the first and last columns'
    /// labels, yet "Heat networks (53)" printed across the Households bar,
    /// "Households (51)" across Heat networks and "Industry and
    /// manufacturing (53)" across Grid and Wind; at 250 two labels
    /// overprinted as well. Labels between columns could not hang
    /// anywhere, so nothing made room for them. The columns now spread
    /// apart until they clear, with the long names wrapped at the
    /// configured width's share — so only as far as that needs.
    #[test]
    fn middle_column_labels_never_overprint_at_narrow_widths() {
        let body = "sankey-beta\nCoal,Grid,24\nCoal,Heat networks,35\n\
            Oil,Thermal power plants,17\nOil,Refineries and upgraders,5\n\
            Gas,Refineries and upgraders,25\nGas,Thermal power plants,32\n\
            Hydro,Refineries and upgraders,36\nHydro,Heat networks,8\n\
            Wind,Refineries and upgraders,15\nWind,Thermal power plants,28\n\
            Solar,Grid,26\nSolar,Thermal power plants,12\n\
            Biomass,Thermal power plants,30\nBiomass,Heat networks,10\n\
            Thermal power plants,Households,9\n\
            Thermal power plants,Industry and manufacturing,27\n\
            Thermal power plants,Transport,9\nRefineries and upgraders,Households,13\n\
            Refineries and upgraders,Industry and manufacturing,9\n\
            Refineries and upgraders,Services,20\nGrid,Households,15\n\
            Grid,Industry and manufacturing,12\nGrid,Transport,3\n\
            Heat networks,Transport,7\nHeat networks,Households,11\n\
            Heat networks,Industry and manufacturing,5\n\
            Industry and manufacturing,Useful energy,13\n\
            Industry and manufacturing,Rejected energy and losses,12\n\
            Households,Useful energy,29\nHouseholds,Rejected energy and losses,22\n\
            Transport,Useful energy,21\nTransport,Rejected energy and losses,3\n\
            Services,Useful energy,22\nServices,Rejected energy and losses,24\n";
        for width in [250, 350, 450, 600] {
            let src = format!(
                "---\nconfig:\n  sankey:\n    width: {width}\n    height: 420\n    \
                 showValues: true\n---\n{body}"
            );
            let ss = scene(&diagram(&src));
            assert_labels_clear(&ss);
            // Room for the wrapped labels, not for every name on one line
            // (which would need about 1 000px).
            assert!(ss.width < 640.0, "width {width}: canvas {}", ss.width);
        }
    }

    /// Random diagrams of three to five columns with long names, at
    /// widths from 250 to 600: every label stays clear of every other
    /// label and node, and inside the canvas.
    #[test]
    fn random_diagrams_keep_every_label_clear() {
        let names = [
            "Coal",
            "Gas",
            "Oil",
            "Nuclear power",
            "Offshore wind farms",
            "Solar",
            "Hydro",
            "Biomass and waste",
            "Imports from abroad",
            "Electricity grid",
            "District heating networks",
            "Refineries and upgraders",
            "Thermal power plants",
            "Hydrogen",
            "Households",
            "Industry and manufacturing",
            "Transport",
            "Services and commerce",
            "Agriculture",
            "Useful energy",
            "Rejected energy and losses",
            "Exports",
        ];
        for seed in 0..24u64 {
            let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
            let mut next = |m: u64| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x % m
            };
            let mut pool: Vec<&str> = names.to_vec();
            for i in (1..pool.len()).rev() {
                pool.swap(i, next(i as u64 + 1) as usize);
            }
            let mut cols: Vec<Vec<&str>> = Vec::new();
            let mut at = 0;
            for _ in 0..3 + next(3) {
                let take = (2 + next(4) as usize).min(pool.len() - at);
                if take < 2 {
                    break;
                }
                cols.push(pool[at..at + take].to_vec());
                at += take;
            }
            let width = [250, 300, 350, 450, 600][next(5) as usize];
            let mut src = format!(
                "---\nconfig:\n  sankey:\n    width: {width}\n    showValues: {}\n---\n\
                 sankey-beta\n",
                next(2) == 0
            );
            let mut links = std::collections::BTreeSet::new();
            for c in 0..cols.len() - 1 {
                for &t in &cols[c + 1] {
                    links.insert((cols[c][next(cols[c].len() as u64) as usize], t));
                }
                for &s in &cols[c] {
                    links.insert((s, cols[c + 1][next(cols[c + 1].len() as u64) as usize]));
                }
            }
            for (s, t) in links {
                src.push_str(&format!("{s},{t},{}\n", 3 + next(58)));
            }
            assert_labels_clear(&scene(&diagram(&src)));
        }
    }

    /// A long first-column name with room to spare at the default width
    /// never sits across the next column's node.
    #[test]
    fn a_long_source_name_never_runs_across_the_next_column() {
        let src =
            "sankey-beta\nA rather long name for a source node here,Mid,10\nB,Mid,5\nMid,Out,15\n";
        let ss = scene(&diagram(src));
        assert_labels_clear(&ss);
        assert_eq!(ss.width, 600.0);
    }

    /// Review: `Electricity: steelblue` was invisible to the clash check
    /// (it could only read hex), so linked Industry took steel blue
    /// `#4d8fbf` beside it — one flat blue ribbon.
    #[test]
    fn a_named_node_colour_is_kept_clear_of_too() {
        let ss = showcase_with(
            "    nodeColors:\n      Electricity: steelblue\n      Gas: rgb(51, 163, 92)\n",
            &[],
        );
        assert_eq!(node(&ss, "Electricity").fill(), "steelblue");
        for linked in [
            "Coal", "Nuclear", "Wind", "Solar", "Industry", "Homes", "Losses",
        ] {
            let de = delta_e(node(&ss, linked).fill(), "steelblue").unwrap();
            assert!(de >= CLASH_DE, "{linked} is {de:.0} ΔE from steelblue");
        }
        let de = delta_e(node(&ss, "Heating").fill(), "#33a35c").unwrap();
        assert!(de >= CLASH_DE, "Heating is {de:.0} ΔE from Gas");
    }

    #[test]
    fn outlined_labels_get_a_halo_and_follow_mermaid_sides() {
        let ss = showcase_with("    labelStyle: outlined\n", &[]);
        let svg = to_svg(&ss);
        // One halo copy (white stroke) under every label.
        assert_eq!(
            svg.matches("stroke=\"#ffffff\" stroke-width=\"4\"").count(),
            ss.nodes.len()
        );
        // Left of the busiest column (Electricity's), a label hangs
        // left — unless that would leave the canvas, as in column 0.
        assert_eq!(node(&ss, "Coal").label_anchor, "start");
        let plain = to_svg(&showcase_with("", &[]));
        assert!(!plain.contains("stroke-width=\"4\""), "legacy has no halo");
        assert_labels_clear(&ss);
    }

    #[test]
    fn labels_are_drawn_at_mermaids_14px() {
        let svg = to_svg(&scene(&diagram("sankey-beta\nA,B,1\n")));
        assert!(svg.contains("font-size=\"14\""), "{svg}");
    }

    #[test]
    fn ribbons_stack_by_where_their_other_end_is() {
        // A→C skips a column. Stacking by column RANK compared C's rank
        // 0 (column 2) with B's rank 0 (column 1) — a tie, so A→B went on
        // top although C sits higher than B. By position, the ribbon to
        // the higher node leaves from higher up and never crosses A→B.
        let ss = scene(&diagram(
            "sankey-beta\nA,B,10\nA,C,10\nB,C,50\nB,D,5\nE,B,45\n",
        ));
        let (b, c) = (node(&ss, "B"), node(&ss, "C"));
        assert!(
            c.y + c.h / 2.0 < b.y + b.h / 2.0,
            "the case needs C above B"
        );
        for (i, _) in ss.nodes.iter().enumerate() {
            let mut out: Vec<&SankeyLinkGlyph> =
                ss.links.iter().filter(|l| l.source == i).collect();
            out.sort_by(|a, b| a.y0.partial_cmp(&b.y0).unwrap());
            for w in out.windows(2) {
                let far = |l: &SankeyLinkGlyph| ss.nodes[l.target].y + ss.nodes[l.target].h / 2.0;
                assert!(far(w[0]) <= far(w[1]), "out of {}", ss.nodes[i].label);
            }
            let mut inc: Vec<&SankeyLinkGlyph> =
                ss.links.iter().filter(|l| l.target == i).collect();
            inc.sort_by(|a, b| a.y1.partial_cmp(&b.y1).unwrap());
            for w in inc.windows(2) {
                let far = |l: &SankeyLinkGlyph| ss.nodes[l.source].y + ss.nodes[l.source].h / 2.0;
                assert!(far(w[0]) <= far(w[1]), "into {}", ss.nodes[i].label);
            }
        }
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
