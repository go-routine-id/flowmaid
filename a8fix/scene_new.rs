/// Positioned geometry for an architecture diagram.
///
/// `#[non_exhaustive]`: future versions may add fields without a
/// breaking release; field access stays ordinary.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ArchScene {
    /// Services and junctions (index-parallel with
    /// `Architecture::services`; a junction is a small `Circle` with an
    /// empty label), group frames, and one edge per
    /// `Architecture::edges` entry in order. An edge's `waypoints` are
    /// its routed, corner-rounded path, densely sampled at the corners.
    pub scene: Scene,
    /// Optional icon glyph for each service node, index-parallel with `scene.nodes`.
    pub icons: Vec<Option<String>>,
    /// Whether each edge (index-parallel with `scene.edges`) has an
    /// arrowhead pointing at its `from` / `to` endpoint.
    pub arrow_starts: Vec<bool>,
    pub arrow_ends: Vec<bool>,
    /// Where each group title is drawn, index-parallel with
    /// `scene.clusters`: the left end of its text and the centre line
    /// it sits on. Usually [`GROUP_PAD`] in from the frame's left side;
    /// further right when an edge rises through the title band into a
    /// port below it, so that the edge runs beside the title instead
    /// of striking it. Edges are routed clear of the title drawn here.
    pub title_at: Vec<(f64, f64)>,
}

type Pt = (f64, f64);

/// Compute the scene for an architecture diagram.
pub fn scene(d: &Architecture) -> ArchScene {
    let cards: Vec<Card> = d
        .services
        .iter()
        .map(|s| {
            if s.junction {
                Card {
                    lines: Vec::new(),
                    w: JUNCTION_D,
                    h: JUNCTION_D,
                }
            } else {
                card(&s.title, s.icon.as_deref().and_then(icon_glyph).is_some())
            }
        })
        .collect();
    let sizes: Vec<Pt> = cards.iter().map(|c| (c.w, c.h)).collect();
    let t = Tree::new(d, &sizes);
    let cell = place_cells(d, &t);
    let titles: Vec<String> = d
        .groups
        .iter()
        .map(|g| match g.icon.as_deref().and_then(icon_glyph) {
            Some(glyph) if g.title.is_empty() => glyph.to_string(),
            Some(glyph) => format!("{} {}", glyph, g.title),
            None => g.title.clone(),
        })
        .collect();
    let title_w: Vec<f64> = titles.iter().map(|s| title_px(s)).collect();

    // Lay out, then move every title off the edges that rise through
    // its band into a port below it. A title that moves may need a
    // wider frame: lay out again with that width (the frame's own
    // contents keep their places, only what lies right of it moves),
    // and a title that still does not fit stays put for the router to
    // go around.
    let na = t.na();
    let ns = d.services.len();
    let ng = d.groups.len();
    let mut min_w: Vec<f64> = title_w.iter().map(|w| w + 2.0 * GROUP_PAD).collect();
    let mut pass = 0;
    let (boxes, frames, ends, title_x) = loop {
        let (x, y) = solve(d, &t, &cell, &min_w);
        let boxes: Vec<Rect> = (0..ns)
            .map(|i| Rect {
                x0: x.at[i] - sizes[i].0 / 2.0,
                y0: y.at[i] - sizes[i].1 / 2.0,
                x1: x.at[i] + sizes[i].0 / 2.0,
                y1: y.at[i] + sizes[i].1 / 2.0,
            })
            .collect();
        let frames: Vec<Rect> = (0..ng)
            .map(|g| Rect {
                x0: x.lo(na + g),
                y0: y.lo(na + g),
                x1: x.hi(na + g),
                y1: y.hi(na + g),
            })
            .collect();
        let ends = edge_ends(d, &boxes, &frames);
        let mut title_x = title_starts(d, &t, &boxes, &frames, &ends, &title_w);
        let mut grown = false;
        for g in 0..ng {
            let need = title_x[g] + title_w[g] + GROUP_PAD - frames[g].x0;
            if need > frames[g].x1 - frames[g].x0 + EPS {
                if pass < 2 {
                    min_w[g] = min_w[g].max(need);
                    grown = true;
                } else {
                    title_x[g] = frames[g].x0 + GROUP_PAD;
                }
            }
        }
        pass += 1;
        if !grown {
            break (boxes, frames, ends, title_x);
        }
    };
    let (mut right, mut bottom) = (PAD, PAD);
    for r in boxes.iter().chain(&frames) {
        right = right.max(r.x1);
        bottom = bottom.max(r.y1);
    }

    let clusters: Vec<SceneCluster> = d
        .groups
        .iter()
        .enumerate()
        .map(|(g, grp)| {
            let f = frames[g];
            SceneCluster {
                id: grp.id.clone(),
                x: f.x0,
                y: f.y0,
                w: f.x1 - f.x0,
                h: f.y1 - f.y0,
                title: titles[g].clone(),
                depth: t.depth[g],
            }
        })
        .collect();

    // Services take the accent of their top-level group, so colour
    // tells which box belongs where; a service outside every group gets
    // the next accent after the groups'.
    let top_groups: Vec<usize> = t.kids[t.root]
        .iter()
        .filter(|&&it| t.is_group(it))
        .map(|&it| it - na)
        .collect();
    let mut loose = top_groups.len();
    let mut nodes = Vec::with_capacity(ns);
    let mut icons: Vec<Option<String>> = Vec::with_capacity(ns);
    for (i, s) in d.services.iter().enumerate() {
        let top = *t.chain[i]
            .last()
            .expect("an atom's chain starts with itself");
        let tint = if t.is_group(top) {
            top_groups
                .iter()
                .position(|&g| g == top - na)
                .unwrap_or_default()
        } else {
            loose += 1;
            loose - 1
        };
        let (cx, cy) = (
            (boxes[i].x0 + boxes[i].x1) / 2.0,
            (boxes[i].y0 + boxes[i].y1) / 2.0,
        );
        let c = &cards[i];
        if s.junction {
            icons.push(None);
            nodes.push(SceneNode {
                id: s.id.clone(),
                x: cx,
                y: cy,
                w: c.w,
                h: c.h,
                shape: Shape::Circle,
                label: String::new(),
                style: NodeStyle {
                    fill: Some(EDGE_COLOR.to_string()),
                    stroke: Some(EDGE_COLOR.to_string()),
                    stroke_width: Some(0.0),
                    color: None,
                },
                link: None,
            });
            continue;
        }
        icons.push(
            s.icon
                .as_deref()
                .and_then(icon_glyph)
                .map(|g| g.to_string()),
        );
        nodes.push(SceneNode {
            id: s.id.clone(),
            x: cx,
            y: cy,
            w: c.w,
            h: c.h,
            shape: Shape::Rounded,
            label: c.lines.join("\n"),
            style: NodeStyle {
                fill: Some("#ffffff".to_string()),
                stroke: Some(accent(tint).to_string()),
                stroke_width: Some(2.0),
                color: None,
            },
            link: None,
        });
    }

    // Obstacles: every service box, every junction (a route through a
    // junction's dot would read as joining it) and every group title,
    // which edges must never strike.
    let mut obstacles: Vec<(Rect, bool)> = boxes.iter().map(|b| (b.grow(CLEAR), true)).collect();
    for g in 0..ng {
        if title_w[g] > 0.0 {
            let tr = title_rect(frames[g], title_x[g], title_w[g]);
            obstacles.push((tr.grow_xy(TITLE_CLEAR, TITLE_CLEAR_Y), true));
        }
    }
    let mut plain: Vec<Rect> = boxes
        .iter()
        .zip(&d.services)
        .filter(|(_, s)| !s.junction)
        .map(|(b, _)| *b)
        .collect();
    plain.extend(frames.iter().copied());
    let outer = Rect {
        x0: PAD - OUTER,
        y0: PAD - OUTER,
        x1: right + OUTER,
        y1: bottom + OUTER,
    };
    let mut grid = Grid::new(&obstacles, &plain, outer, d, &ends);

    let mut edges: Vec<SceneEdge> = Vec::with_capacity(d.edges.len());
    let mut arrow_starts: Vec<bool> = Vec::with_capacity(d.edges.len());
    let mut arrow_ends: Vec<bool> = Vec::with_capacity(d.edges.len());
    for (k, (e, &(p0, p1))) in d.edges.iter().zip(&ends).enumerate() {
        let corners = grid.route(k, p0, e.from_side, p1, e.to_side);
        let label = e.title.as_ref().map(|text| {
            let w = text_px(text) + LABEL_PAD_W;
            (text.clone(), label_spot(&corners, w), w)
        });
        edges.push(SceneEdge {
            from: d.services[e.from].id.clone(),
            to: d.services[e.to].id.clone(),
            bezier: [p0, p0, p1, p1],
            waypoints: rounded(&corners),
            kind: if e.arrow_start || e.arrow_end {
                crate::model::EdgeKind::Arrow
            } else {
                crate::model::EdgeKind::Open
            },
            // The arrow ends ride `arrow_starts` / `arrow_ends`.
            ends: Default::default(),
            meta: Default::default(),
            label,
        });
        arrow_starts.push(e.arrow_start);
        arrow_ends.push(e.arrow_end);
    }

    // The canvas keeps PAD around the boxes and frames, and OUTER
    // around every route and label chip — on all four sides: a route
    // over the top row (on a top port's stub line, above the outermost
    // channel) or its label chip may reach above or left of the
    // margin, and then the whole drawing moves down or right instead
    // of the chip being cut off.
    let (mut lo_x, mut lo_y) = (0.0f64, 0.0f64);
    let (mut width, mut height) = (right + PAD, bottom + PAD);
    for e in &edges {
        for p in &e.waypoints {
            lo_x = lo_x.min(p.0 - OUTER);
            lo_y = lo_y.min(p.1 - OUTER);
            width = width.max(p.0 + OUTER);
            height = height.max(p.1 + OUTER);
        }
        if let Some((_, c, w)) = &e.label {
            lo_x = lo_x.min(c.0 - w / 2.0 - OUTER);
            lo_y = lo_y.min(c.1 - LABEL_BOX_H / 2.0 - OUTER);
            width = width.max(c.0 + w / 2.0 + OUTER);
            height = height.max(c.1 + LABEL_BOX_H / 2.0 + OUTER);
        }
    }
    let (dx, dy) = (-lo_x, -lo_y);
    let mv = |p: &mut Pt| {
        p.0 += dx;
        p.1 += dy;
    };
    let mut title_at: Vec<Pt> = (0..ng)
        .map(|g| (title_x[g], frames[g].y0 + TITLE_CY))
        .collect();
    let mut clusters = clusters;
    if dx > 0.0 || dy > 0.0 {
        for n in &mut nodes {
            n.x += dx;
            n.y += dy;
        }
        for c in &mut clusters {
            c.x += dx;
            c.y += dy;
        }
        for e in &mut edges {
            e.waypoints.iter_mut().for_each(mv);
            e.bezier.iter_mut().for_each(mv);
            if let Some((_, c, _)) = &mut e.label {
                mv(c);
            }
        }
        title_at.iter_mut().for_each(mv);
        width += dx;
        height += dy;
    }

    ArchScene {
        scene: Scene {
            nodes,
            edges,
            clusters,
            width,
            height,
        },
        icons,
        arrow_starts,
        arrow_ends,
        title_at,
    }
}

/// The attach point of `side` on a box: the middle of that side, or a
/// junction's centre (every side of a junction is its centre).
fn port(r: &Rect, side: ArchSide, junction: bool) -> Pt {
    let (cx, cy) = ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0);
    if junction {
        return (cx, cy);
    }
    match side {
        ArchSide::T => (cx, r.y0),
        ArchSide::B => (cx, r.y1),
        ArchSide::L => (r.x0, cy),
        ArchSide::R => (r.x1, cy),
    }
}

/// Both ends of every edge: a port on a box side, a junction's centre,
/// or — for a `{group}` end — the point where the port's normal leaves
/// the service's group frame.
fn edge_ends(d: &Architecture, boxes: &[Rect], frames: &[Rect]) -> Vec<(Pt, Pt)> {
    let end = |s: usize, side: ArchSide, group_end: bool| -> Pt {
        let p = port(&boxes[s], side, d.services[s].junction);
        match d.services[s].group.filter(|_| group_end) {
            Some(g) if g < frames.len() => {
                let f = frames[g];
                match side {
                    ArchSide::T => (p.0, f.y0),
                    ArchSide::B => (p.0, f.y1),
                    ArchSide::L => (f.x0, p.1),
                    ArchSide::R => (f.x1, p.1),
                }
            }
            _ => p,
        }
    };
    d.edges
        .iter()
        .map(|e| {
            (
                end(e.from, e.from_side, e.from_group),
                end(e.to, e.to_side, e.to_group),
            )
        })
        .collect()
}

/// The ink box of a group title of width `w` that starts at `x`.
fn title_rect(f: Rect, x: f64, w: f64) -> Rect {
    Rect {
        x0: x,
        y0: f.y0 + TITLE_CY - TITLE_HALF_H,
        x1: x + w,
        y1: f.y0 + TITLE_CY + TITLE_HALF_H,
    }
}

/// Where each group title starts: [`GROUP_PAD`] in from its frame,
/// or right of every edge that rises through its band into a port
/// below. Such an edge leaves (or enters) a top port straight up, and
/// when nothing stands between the port and the band it either runs
/// straight through the band or — if the title sat on its line —
/// would have to wiggle round the title just above the port, with no
/// room to do it cleanly. The edge's line is kept for it when the port
/// is that close below the band, or when the edge's far end lies above
/// the band (so the edge comes straight down to it).
fn title_starts(
    d: &Architecture,
    t: &Tree,
    boxes: &[Rect],
    frames: &[Rect],
    ends: &[(Pt, Pt)],
    title_w: &[f64],
) -> Vec<f64> {
    let na = t.na();
    let mut cols: Vec<Vec<f64>> = vec![Vec::new(); frames.len()];
    for (e, &(p0, p1)) in d.edges.iter().zip(ends) {
        for (s, side, group_end, p, q) in [
            (e.from, e.from_side, e.from_group, p0, p1),
            (e.to, e.to_side, e.to_group, p1, p0),
        ] {
            if side != ArchSide::T {
                continue;
            }
            // The frames around the port: around the service, or —
            // for a `{group}` end, which sits on its group's frame —
            // around that frame.
            let skip = 1 + usize::from(group_end && t.chain[s].len() > 1);
            for &it in t.chain[s].iter().skip(skip) {
                let g = it - na;
                if title_w[g] <= 0.0 {
                    continue;
                }
                let f = frames[g];
                let (top, bottom) = (f.y0 + TITLE_CY - TITLE_HALF_H, f.y0 + TITLE_CY + TITLE_HALF_H);
                let between = boxes.iter().enumerate().any(|(b, r)| {
                    b != s
                        && !d.services[b].junction
                        && r.x0 - CLEAR < p.0
                        && p.0 < r.x1 + CLEAR
                        && r.y1 > bottom
                        && r.y0 < p.1
                });
                if between {
                    // The frames further out are behind that box too.
                    break;
                }
                let tight = p.1 - STUB < bottom + TITLE_CLEAR_Y + 2.0 * CORNER_R;
                if tight || q.1 < top {
                    cols[g].push(p.0);
                }
            }
        }
    }
    cols.iter_mut()
        .enumerate()
        .map(|(g, c)| {
            c.sort_by(f64::total_cmp);
            let mut x = frames[g].x0 + GROUP_PAD;
            for &r in c.iter() {
                if r > x - TITLE_SHIFT && r < x + title_w[g] + TITLE_SHIFT {
                    x = r + TITLE_SHIFT;
                }
            }
            x
        })
        .collect()
}
