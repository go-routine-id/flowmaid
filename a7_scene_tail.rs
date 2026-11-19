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
    let label_w: Vec<f64> = label_text
        .iter()
        .map(|t| text_width(t) * FONT as f64 / TEXT_CALIBRATION)
        .collect();

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

    // Individual link values are checked by the parser, but their SUM
    // can still overflow to +inf, and an infinite canvas is unparseable
    // SVG. Fall back to the configured size when that happens.
    let finite = |v: f64, fallback: f64| if v.is_finite() { v } else { fallback };
    let width = finite(cfg.width.max(PAD * 2.0 + plot_w), 600.0);
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

    let mut nodes: Vec<SankeyNodeGlyph> = (0..n)
        .map(|i| SankeyNodeGlyph {
            label: d.nodes[i].clone(),
            x: col_x(depth[i]),
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
        })
        .collect();
    place_labels(&mut nodes, &label_w, &order, n_cols, cfg.label_style, width, height);

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
        (0..n_cols.saturating_sub(1)).rev().map(|c| (c, c + 1)).collect()
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

/// CIE L*a*b* (D65) of a `#rrggbb` / `#rgb` colour, for [`CLASH_DE`];
/// `None` for anything else (named colours, `rgb()`, ...).
fn lab(color: &str) -> Option<[f64; 3]> {
    let hex = color.strip_prefix('#')?;
    let d: Vec<u32> = hex.chars().map(|c| c.to_digit(16)).collect::<Option<_>>()?;
    let rgb = match d.len() {
        3 => [d[0] * 17, d[1] * 17, d[2] * 17],
        6 => [d[0] * 16 + d[1], d[2] * 16 + d[3], d[4] * 16 + d[5]],
        _ => return None,
    };
    let [r, g, b] = rgb.map(|c| {
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

/// CIE76 colour difference; `None` when either colour is not hex.
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
        let mut best: Option<((u8, u8, u8, f64, usize), &'static str)> = None;
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
    colors.into_iter().map(|c| c.unwrap_or(ACCENTS[0])).collect()
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
fn place_labels(
    nodes: &mut [SankeyNodeGlyph],
    label_w: &[f64],
    order: &[usize],
    n_cols: usize,
    style: SankeyLabelStyle,
    width: f64,
    height: f64,
) {
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
    for &i in &by_room {
        let nd = &nodes[i];
        let last = n_cols > 1 && nd.column == n_cols - 1;
        let left_first = match style {
            SankeyLabelStyle::Legacy => last,
            SankeyLabelStyle::Outlined => last || nd.column < central,
        };
        let centre = nd.y + nd.h / 2.0;
        let (lo, hi) = (nd.y + LABEL_H / 2.0, nd.y + nd.h - LABEL_H / 2.0);
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
        let mut best: Option<(f64, f64, f64, bool, Rect)> = None;
        'search: for left in [left_first, !left_first] {
            for &y in &ys {
                let (x0, x1) = if left {
                    (nd.x - LABEL_GAP - label_w[i], nd.x - LABEL_GAP)
                } else {
                    let x0 = nd.x + nd.w + LABEL_GAP;
                    (x0, x0 + label_w[i])
                };
                let r: Rect = (x0, y - LABEL_H / 2.0, x1, y + LABEL_H / 2.0);
                let grown = (r.0 - LABEL_CLEAR, r.1 - LABEL_CLEAR, r.2 + LABEL_CLEAR, r.3 + LABEL_CLEAR);
                let outside = (r.2 - r.0) * (r.3 - r.1) - overlap(r, (0.0, 0.0, width, height));
                let on_labels: f64 = placed.iter().map(|&p| overlap(grown, p)).sum();
                let on_nodes: f64 = rects
                    .iter()
                    .enumerate()
                    .filter(|&(j, _)| j != i)
                    .map(|(_, &q)| overlap(grown, q))
                    .sum();
                let cost = 4.0 * (outside + on_labels) + on_nodes;
                if best.as_ref().map_or(true, |b| cost < b.0) {
                    best = Some((cost, x0, y, left, r));
                }
                if cost == 0.0 {
                    break 'search;
                }
            }
        }
        if let Some((_, _, y, left, r)) = best {
            let nd = &mut nodes[i];
            nd.label_pos = if left {
                (nd.x - LABEL_GAP, y)
            } else {
                (nd.x + nd.w + LABEL_GAP, y)
            };
            nd.label_anchor = if left { "end" } else { "start" };
            placed.push(r);
        }
    }
}

