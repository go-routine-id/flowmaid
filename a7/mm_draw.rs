
// ------------------------------------------------------------------
// Shapes
// ------------------------------------------------------------------

/// Perimeter points (absolute coords) for the polygon shapes — hexagon,
/// bang (spiky), cloud (lobed). `None` for box and circle shapes, which
/// a painter draws with its own rect / ellipse primitive (a circle has
/// `w == h`). Exposed so the SVG and any GUI painter share identical
/// geometry.
pub fn perimeter(n: &MindNodeBox) -> Option<Vec<(f64, f64)>> {
    match n.shape {
        MindShape::Hexagon => Some(hexagon_pts(n)),
        MindShape::Bang => Some(bang_pts(n)),
        MindShape::Cloud => Some(cloud_pts(n)),
        _ => None,
    }
}

/// The underline of a [`MindShape::Default`] node, as a rectangle
/// `(x, y, w, h)` along its flat bottom edge: paint it black at
/// [`UNDERLINE_SHADE`] opacity over the fill. `None` for other shapes.
pub fn underline(n: &MindNodeBox) -> Option<(f64, f64, f64, f64)> {
    (n.shape == MindShape::Default).then(|| (n.x, n.y + n.h - UNDERLINE_H, n.w, UNDERLINE_H))
}

/// Where the label's text block is centred: the box centre, nudged
/// right by half the icon's room when the node has an `::icon`.
pub fn label_centre(n: &MindNodeBox) -> (f64, f64) {
    (n.cx() + icon_room(n.icon.is_some()) / 2.0, n.cy())
}

fn hexagon_pts(n: &MindNodeBox) -> Vec<(f64, f64)> {
    let (x, y, w, h) = (n.x, n.y, n.w, n.h);
    let k = (h * 0.5).min(w / 3.0);
    vec![
        (x + k, y),
        (x + w - k, y),
        (x + w, y + h / 2.0),
        (x + w - k, y + h),
        (x + k, y + h),
        (x, y + h / 2.0),
    ]
}

/// Parametric angles of `count` points spaced evenly by ARC LENGTH round
/// the ellipse `(rx, ry)`, the first at 12 o'clock, clockwise. Spacing
/// by parametric angle instead bunches points at the ends of a flat
/// ellipse (the old cloud's long spikes at 3 and 9 o'clock).
fn even_angles(rx: f64, ry: f64, count: usize, phase: f64) -> Vec<f64> {
    const SAMPLES: usize = 720;
    let at = |t: f64| (rx * t.cos(), ry * t.sin());
    let mut cum = Vec::with_capacity(SAMPLES + 1);
    cum.push(0.0f64);
    let mut prev = at(-FRAC_PI_2);
    for s in 1..=SAMPLES {
        let p = at(-FRAC_PI_2 + TAU * s as f64 / SAMPLES as f64);
        cum.push(cum[s - 1] + (p.0 - prev.0).hypot(p.1 - prev.1));
        prev = p;
    }
    let total = cum[SAMPLES];
    let mut out = Vec::with_capacity(count);
    let mut s = 0;
    for j in 0..count {
        let target = total * (j as f64 + phase) / count as f64;
        while s < SAMPLES && cum[s + 1] < target {
            s += 1;
        }
        let seg = (cum[s + 1] - cum[s]).max(1e-12);
        let f = ((target - cum[s]) / seg).clamp(0.0, 1.0);
        out.push(-FRAC_PI_2 + TAU * (s as f64 + f) / SAMPLES as f64);
    }
    out
}

/// Ramanujan's approximation of an ellipse's perimeter.
fn ellipse_perimeter(rx: f64, ry: f64) -> f64 {
    PI * (3.0 * (rx + ry) - ((3.0 * rx + ry) * (rx + 3.0 * ry)).sqrt())
}

/// A bang: spikes of one depth ([`BANG_SPIKE`], along the outline's
/// normal) evenly spaced round an ellipse that holds the label. Scaling
/// a unit star by the box instead gave a flat label 4:1 longer spikes at
/// its sides than on top.
fn bang_pts(n: &MindNodeBox) -> Vec<(f64, f64)> {
    let (cx, cy) = (n.cx(), n.cy());
    let (rx, ry) = (
        (n.w / 2.0 - BANG_SPIKE).max(1.0),
        (n.h / 2.0 - BANG_SPIKE).max(1.0),
    );
    let tips = ((ellipse_perimeter(rx, ry) / BANG_STEP).round() as usize).max(8);
    even_angles(rx, ry, 2 * tips, 0.0)
        .into_iter()
        .enumerate()
        .map(|(j, t)| {
            let (px, py) = (rx * t.cos(), ry * t.sin());
            if j % 2 == 0 {
                // Tip: out along the ellipse's normal.
                let (nx, ny) = (t.cos() / rx, t.sin() / ry);
                let len = nx.hypot(ny).max(1e-12);
                (
                    cx + px + BANG_SPIKE * nx / len,
                    cy + py + BANG_SPIKE * ny / len,
                )
            } else {
                (cx + px, cy + py)
            }
        })
        .collect()
}

/// A cloud: round lobes of radius [`CLOUD_LOBE`] centred on an ellipse
/// that holds the label, spaced so neighbours overlap; the outline is
/// each lobe's outer arc between its two neighbours' crossings, sampled
/// as a polygon so painters need no arc primitive.
fn cloud_pts(n: &MindNodeBox) -> Vec<(f64, f64)> {
    let (cx, cy) = (n.cx(), n.cy());
    let r = CLOUD_LOBE;
    let (rx, ry) = ((n.w / 2.0 - r).max(1.0), (n.h / 2.0 - r).max(1.0));
    let m = ((ellipse_perimeter(rx, ry) / CLOUD_STEP).round() as usize).max(6);
    let centres: Vec<(f64, f64)> = even_angles(rx, ry, m, 0.0)
        .into_iter()
        .map(|t| (cx + rx * t.cos(), cy + ry * t.sin()))
        .collect();
    // Outer crossing of lobe j and lobe j+1.
    let cross: Vec<(f64, f64)> = (0..m)
        .map(|j| {
            let (a, b) = (centres[j], centres[(j + 1) % m]);
            let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let half = dx.hypot(dy) / 2.0;
            let t = (r * r - half * half).max(0.0).sqrt();
            // Normal to the chord, pointing away from the centre.
            let mut nrm = (dy, -dx);
            if nrm.0 * (mid.0 - cx) + nrm.1 * (mid.1 - cy) < 0.0 {
                nrm = (-nrm.0, -nrm.1);
            }
            let l = nrm.0.hypot(nrm.1).max(1e-12);
            (mid.0 + t * nrm.0 / l, mid.1 + t * nrm.1 / l)
        })
        .collect();
    const PER_LOBE: usize = 7;
    let mut pts = Vec::with_capacity(m * PER_LOBE);
    for j in 0..m {
        let c = centres[j];
        let from = cross[(j + m - 1) % m];
        let to = cross[j];
        let a0 = (from.1 - c.1).atan2(from.0 - c.0);
        let mut a1 = (to.1 - c.1).atan2(to.0 - c.0);
        // Clockwise (increasing angle on screen) round the outside.
        while a1 <= a0 {
            a1 += TAU;
        }
        for s in 0..PER_LOBE {
            let a = a0 + (a1 - a0) * s as f64 / PER_LOBE as f64;
            pts.push((c.0 + r * a.cos(), c.1 + r * a.sin()));
        }
    }
    pts
}

// ------------------------------------------------------------------
// Icons
// ------------------------------------------------------------------

/// Built-in pictograms for `::icon(…)`. flowmaid ships no icon font, so
/// a handful of common names (font-awesome / material style) map to
/// small line drawings; anything else gets a neutral sparkle. Each is a
/// set of polylines in a unit square.
fn icon_shape(icon: &str) -> Vec<Vec<(f64, f64)>> {
    let name = icon
        .split_whitespace()
        .last()
        .unwrap_or("")
        .to_ascii_lowercase();
    let has = |keys: &[&str]| keys.iter().any(|k| name.contains(k));
    let arc = |cx: f64, cy: f64, r: f64, a0: f64, a1: f64| -> Vec<(f64, f64)> {
        let steps = (((a1 - a0).abs() / (PI / 10.0)).ceil() as usize).max(2);
        (0..=steps)
            .map(|s| {
                let a = (a0 + (a1 - a0) * s as f64 / steps as f64).to_radians();
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    };
    let circle = |cx: f64, cy: f64, r: f64| arc(cx, cy, r, 0.0, 360.0);
    if has(&["file", "page", "doc"]) {
        vec![
            vec![(0.2, 0.0), (0.62, 0.0), (0.85, 0.24), (0.85, 1.0), (0.2, 1.0), (0.2, 0.0)],
            vec![(0.62, 0.0), (0.62, 0.24), (0.85, 0.24)],
        ]
    } else if has(&["book"]) {
        vec![vec![
            (0.5, 0.2),
            (0.05, 0.06),
            (0.05, 0.86),
            (0.5, 1.0),
            (0.95, 0.86),
            (0.95, 0.06),
            (0.5, 0.2),
            (0.5, 1.0),
        ]]
    } else if has(&["code", "terminal"]) {
        vec![
            vec![(0.3, 0.2), (0.02, 0.5), (0.3, 0.8)],
            vec![(0.7, 0.2), (0.98, 0.5), (0.7, 0.8)],
            vec![(0.58, 0.08), (0.42, 0.92)],
        ]
    } else if has(&["cog", "gear", "setting", "wrench", "tool"]) {
        let mut v = vec![circle(0.5, 0.5, 0.2)];
        for s in 0..8 {
            let a = (s as f64 * 45.0).to_radians();
            v.push(vec![
                (0.5 + 0.3 * a.cos(), 0.5 + 0.3 * a.sin()),
                (0.5 + 0.48 * a.cos(), 0.5 + 0.48 * a.sin()),
            ]);
        }
        v
    } else if has(&["user", "person", "account", "people", "group", "team"]) {
        vec![circle(0.5, 0.3, 0.2), arc(0.5, 1.0, 0.42, 180.0, 360.0)]
    } else if has(&["home", "house"]) {
        vec![
            vec![(0.05, 0.5), (0.5, 0.08), (0.95, 0.5)],
            vec![(0.2, 0.38), (0.2, 0.95), (0.8, 0.95), (0.8, 0.38)],
        ]
    } else if has(&["star"]) {
        let pts = (0..=10)
            .map(|s| {
                let a = (-90.0 + 36.0 * s as f64).to_radians();
                let r = if s % 2 == 0 { 0.5 } else { 0.2 };
                (0.5 + r * a.cos(), 0.54 + r * a.sin())
            })
            .collect();
        vec![pts]
    } else if has(&["heart", "like", "love"]) {
        let pts = (0..=32)
            .map(|s| {
                let t = TAU * s as f64 / 32.0;
                let x = 16.0 * t.sin().powi(3);
                let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                (0.5 + x / 34.0, 0.45 - y / 34.0)
            })
            .collect();
        vec![pts]
    } else if has(&["check", "tick", "done"]) {
        vec![vec![(0.08, 0.55), (0.38, 0.84), (0.92, 0.18)]]
    } else if has(&["bolt", "flash", "lightning", "zap", "power"]) {
        vec![vec![
            (0.62, 0.0),
            (0.18, 0.56),
            (0.48, 0.56),
            (0.38, 1.0),
            (0.82, 0.42),
            (0.52, 0.42),
            (0.62, 0.0),
        ]]
    } else if has(&["cloud"]) {
        let mut v = arc(0.26, 0.62, 0.2, 90.0, 290.0);
        v.extend(arc(0.52, 0.48, 0.26, 200.0, 340.0));
        v.extend(arc(0.77, 0.62, 0.2, 250.0, 450.0));
        v.push((0.26, 0.82));
        vec![v]
    } else if has(&["database", "server", "storage", "disk"]) {
        // A cylinder: the top rim, then the sides and the bottom rim.
        let rim = |cy: f64, a0: f64, a1: f64| -> Vec<(f64, f64)> {
            arc(0.0, 0.0, 1.0, a0, a1)
                .into_iter()
                .map(|(x, y)| (0.5 + 0.4 * x, cy + 0.14 * y))
                .collect()
        };
        let mut body = vec![(0.9, 0.16)];
        body.extend(rim(0.84, 0.0, 180.0));
        body.push((0.1, 0.16));
        vec![rim(0.16, 0.0, 360.0), body]
    } else if has(&["globe", "world", "earth", "web", "internet", "language"]) {
        vec![
            circle(0.5, 0.5, 0.48),
            arc(0.5, 0.5, 0.48, 0.0, 360.0)
                .into_iter()
                .map(|(x, y)| (0.5 + (x - 0.5) * 0.42, y))
                .collect(),
            vec![(0.02, 0.5), (0.98, 0.5)],
        ]
    } else if has(&["search", "magnif", "find"]) {
        vec![circle(0.42, 0.42, 0.3), vec![(0.64, 0.64), (0.96, 0.96)]]
    } else if has(&["envelope", "mail", "inbox"]) {
        vec![
            vec![(0.02, 0.2), (0.98, 0.2), (0.98, 0.82), (0.02, 0.82), (0.02, 0.2)],
            vec![(0.02, 0.2), (0.5, 0.56), (0.98, 0.2)],
        ]
    } else if has(&["lock", "secure", "key", "shield"]) {
        vec![
            vec![(0.15, 0.45), (0.85, 0.45), (0.85, 1.0), (0.15, 1.0), (0.15, 0.45)],
            arc(0.5, 0.45, 0.26, 180.0, 360.0),
        ]
    } else {
        // Unknown icon: a quiet four-point sparkle marks "icon here".
        vec![vec![
            (0.5, 0.02),
            (0.62, 0.38),
            (0.98, 0.5),
            (0.62, 0.62),
            (0.5, 0.98),
            (0.38, 0.62),
            (0.02, 0.5),
            (0.38, 0.38),
            (0.5, 0.02),
        ]]
    }
}

/// The node's `::icon` glyph as polylines in absolute coordinates — an
/// [`ICON`]-sized square left of the label, stroked in the label colour
/// (1.5px, round joins). `None` when the node has no icon.
pub fn icon_strokes(n: &MindNodeBox) -> Option<Vec<Vec<(f64, f64)>>> {
    let icon = n.icon.as_deref()?;
    let (tw, _) = text_block(&n.text, n.depth);
    let (lx, cy) = label_centre(n);
    let x0 = lx - tw / 2.0 - ICON_GAP - ICON;
    let y0 = cy - ICON / 2.0;
    Some(
        icon_shape(icon)
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .map(|(u, v)| (x0 + u * ICON, y0 + v * ICON))
                    .collect()
            })
            .collect(),
    )
}

// ------------------------------------------------------------------
// SVG
// ------------------------------------------------------------------

/// Serialise a [`MindScene`] to a standalone SVG document.
pub fn to_svg(ms: &MindScene) -> String {
    to_svg_with(ms, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(ms: &MindScene, opts: &SvgOptions) -> String {
    let mut s = String::new();
    svg_open(&mut s, ms.width, ms.height, FONT, "Mind map", opts);

    // Connectors behind the nodes.
    for e in &ms.edges {
        let [a, c1, c2, b] = e.bezier;
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} C {:.1} {:.1} {:.1} {:.1} {:.1} {:.1}\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linecap=\"round\"/>\n",
            a.0, a.1, c1.0, c1.1, c2.0, c2.1, b.0, b.1, e.color, e.width
        ));
    }

    for n in &ms.nodes {
        // `:::` classes travel as a class attribute, as in mermaid, so
        // page CSS can still style the node.
        let group = !n.classes.is_empty();
        if group {
            s.push_str(&format!(
                "<g class=\"{}\">\n",
                escape(&n.classes.join(" "))
            ));
        }
        s.push_str(&shape_svg(n));
        if let Some(lines) = icon_strokes(n) {
            s.push_str(&icon_svg(n, &lines));
        }
        s.push_str(&label_svg(n));
        if group {
            s.push_str("</g>\n");
        }
    }

    s.push_str("</svg>\n");
    s
}

/// `fill` (and a `classDef` stroke) attributes for a node outline.
fn paint_attrs(n: &MindNodeBox) -> String {
    let mut a = format!("fill=\"{}\"", escape(n.paint()));
    if let Some(stroke) = &n.style.stroke {
        a.push_str(&format!(
            " stroke=\"{}\" stroke-width=\"{:.1}\"",
            escape(stroke),
            n.style.stroke_width.unwrap_or(2.0)
        ));
    }
    a
}

/// SVG for a node's filled outline.
fn shape_svg(n: &MindNodeBox) -> String {
    let (x, y, w, h) = (n.x, n.y, n.w, n.h);
    let paint = paint_attrs(n);
    if let Some(pts) = perimeter(n) {
        let points = pts
            .iter()
            .map(|(px, py)| format!("{:.1},{:.1}", px, py))
            .collect::<Vec<_>>()
            .join(" ");
        return format!("<polygon points=\"{}\" {}/>\n", points, paint);
    }
    match n.shape {
        MindShape::Circle => format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" {}/>\n",
            n.cx(),
            n.cy(),
            w.min(h) / 2.0,
            paint
        ),
        MindShape::Default => {
            // Rounded top corners, flat bottom carrying the underline.
            let r = DEFAULT_RX.min(h / 2.0).min(w / 2.0);
            let mut out = format!(
                "<path d=\"M {:.1} {:.1} V {:.1} Q {:.1} {:.1} {:.1} {:.1} H {:.1} \
                 Q {:.1} {:.1} {:.1} {:.1} V {:.1} Z\" {}/>\n",
                x,
                y + h,
                y + r,
                x,
                y,
                x + r,
                y,
                x + w - r,
                x + w,
                y,
                x + w,
                y + r,
                y + h,
                paint
            );
            if let Some((ux, uy, uw, uh)) = underline(n) {
                out.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                     fill=\"#000000\" fill-opacity=\"{}\"/>\n",
                    ux, uy, uw, uh, UNDERLINE_SHADE
                ));
            }
            out
        }
        _ => {
            let rx = match n.shape {
                MindShape::Square => 0.0,
                // A pill: fully round ends.
                _ => h / 2.0,
            };
            format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                 rx=\"{:.1}\" {}/>\n",
                x, y, w, h, rx, paint
            )
        }
    }
}

/// SVG for a node's icon glyph, with the icon name as its tooltip.
fn icon_svg(n: &MindNodeBox, lines: &[Vec<(f64, f64)>]) -> String {
    let mut d = String::new();
    for line in lines {
        for (k, (x, y)) in line.iter().enumerate() {
            d.push_str(&format!(
                "{}{:.1} {:.1}",
                if k == 0 { "M" } else { " L" },
                x,
                y
            ));
            d.push(' ');
        }
    }
    format!(
        "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" \
         stroke-linecap=\"round\" stroke-linejoin=\"round\"><title>{}</title></path>\n",
        d.trim_end(),
        escape(n.ink()),
        escape(n.icon.as_deref().unwrap_or(""))
    )
}

/// SVG for a node's centred (possibly multi-line) label.
fn label_svg(n: &MindNodeBox) -> String {
    let lines: Vec<&str> = n.text.split('\n').collect();
    let mut attrs = String::new();
    if (n.font_size - FONT as f64).abs() > 1e-9 {
        attrs.push_str(&format!(" font-size=\"{}\"", n.font_size));
    }
    match n.font_weight {
        700 => attrs.push_str(" font-weight=\"bold\""),
        400 => {}
        w => attrs.push_str(&format!(" font-weight=\"{w}\"")),
    }
    let (cx, cy) = label_centre(n);
    let line_h = LINE_H * n.font_size / FONT as f64;
    let first_dy = 0.32 * n.font_size - (lines.len() as f64 - 1.0) * line_h / 2.0;
    let mut spans = String::new();
    for (i, line) in lines.iter().enumerate() {
        let dy = if i == 0 { first_dy } else { line_h };
        spans.push_str(&format!(
            "<tspan x=\"{:.1}\" dy=\"{:.1}\">{}</tspan>",
            cx,
            dy,
            escape(line)
        ));
    }
    format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"{}\"{}>{}</text>\n",
        cx,
        cy,
        escape(n.ink()),
        attrs,
        spans
    )
}
