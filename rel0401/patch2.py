import sys
W='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0401/wt/'
def sub(path, old, new, count=1):
    p=W+path
    s=open(p).read()
    n=s.count(old)
    if n!=count:
        sys.exit(f"{path}: expected {count} got {n}: {old[:80]!r}")
    s=s.replace(old,new)
    open(p,'w').write(s)

# ---- Scene struct: origin + routed
sub('src/scene.rs','''    pub clusters: Vec<SceneCluster>,
    pub width: f64,
    pub height: f64,
    /// Set by the architecture and gitGraph scene builders,''','''    pub clusters: Vec<SceneCluster>,
    pub width: f64,
    pub height: f64,
    /// The canvas's top-left corner in scene coordinates: the canvas
    /// spans `origin.0 ..= origin.0 + width` and `origin.1 ..= origin.1 +
    /// height`. `(0, 0)` for a laid-out scene. [`route`] and
    /// [`route_partial`] (and the class and ER `route`) keep every
    /// position as given; where the drawing then reaches into the canvas
    /// margin at the left or top (a node dragged past it), the canvas
    /// grows that way instead, and `origin` is negative on that axis by
    /// as much as the drawing and its margin reach past zero. Nothing is
    /// moved: the SVG's user space is scene space and its `viewBox` starts
    /// at `origin` there (a routed flowchart's `viewBox` is fitted to the
    /// drawing as before, so on an axis that does not overflow it may
    /// start inside this canvas). Pointer coordinates map through the
    /// `viewBox` straight into scene coordinates for picking.
    pub origin: (f64, f64),
    /// Set by the edge stages of [`route`] / [`route_partial`] (and of the
    /// class and ER `route`): its positions are the host's, so the SVG
    /// writers keep them and frame the drawing with the `viewBox`
    /// ([`Scene::origin`]) rather than moving it onto the canvas.
    pub(crate) routed: bool,
    /// Set by the architecture and gitGraph scene builders,''')

sub('src/scene.rs','''            clusters: Vec::new(),
            width,
            height,
            plain_crossings: false,
        }
    }

    /// The topmost element at scene point `(x, y)`, or `None`.''','''            clusters: Vec::new(),
            width,
            height,
            origin: (0.0, 0.0),
            routed: false,
            plain_crossings: false,
        }
    }

    /// The topmost element at scene point `(x, y)`, or `None`.''')

# other literals in scene.rs (two of them with clusters: Vec::new() / clusters)
sub('src/scene.rs','''    Scene {
        nodes,
        edges,
        clusters: Vec::new(),
        width,
        height,
        plain_crossings: false,
    }
}''','''    Scene {
        nodes,
        edges,
        clusters: Vec::new(),
        width,
        height,
        origin: (0.0, 0.0),
        routed: false,
        plain_crossings: false,
    }
}''')
sub('src/scene.rs','''    Scene {
        nodes,
        edges,
        clusters,
        width: maxx + MARGIN,
        height: maxy + MARGIN,
        plain_crossings: false,
    }''','''    Scene {
        nodes,
        edges,
        clusters,
        width: maxx + MARGIN,
        height: maxy + MARGIN,
        origin: (0.0, 0.0),
        routed: false,
        plain_crossings: false,
    }''')
sub('src/architecture.rs','''            width,
            height,
            // `to_svg` here strokes each edge as a plain polyline.
            plain_crossings: true,''','''            width,
            height,
            origin: (0.0, 0.0),
            routed: false,
            // `to_svg` here strokes each edge as a plain polyline.
            plain_crossings: true,''')
sub('src/gitgraph.rs','''            width,
            height,
            // `to_svg` here strokes each link plain.
            plain_crossings: true,''','''            width,
            height,
            origin: (0.0, 0.0),
            routed: false,
            // `to_svg` here strokes each link plain.
            plain_crossings: true,''')
sub('src/compound.rs','''        clusters: clusters.into_iter().map(|(_, c)| c).collect(),
        width,
        height,
        plain_crossings: false,''','''        clusters: clusters.into_iter().map(|(_, c)| c).collect(),
        width,
        height,
        origin: (0.0, 0.0),
        routed: false,
        plain_crossings: false,''')
sub('src/compound.rs','''        width: base.width,
        height: base.height,
        plain_crossings: false,''','''        width: base.width,
        height: base.height,
        origin: base.origin,
        routed: false,
        plain_crossings: false,''')

# route_partial keeps the base canvas, origin included
sub('src/scene.rs','''    sc.width = base.width;
    sc.height = base.height;
    crate::edgepass::finish_held(''','''    sc.width = base.width;
    sc.height = base.height;
    sc.origin = base.origin;
    crate::edgepass::finish_held(''')

# ---- route doc: no translation any more
sub('src/scene.rs','''/// Re-route edges for custom node positions (e.g. after a drag).
/// `centers[i]` = centre of node i in final coordinates. Node
/// positions are NOT normalised (they stay exactly as given) so the
/// diagram doesn't "swim" while dragging; `to_svg` handles
/// translation at export time.''','''/// Re-route edges for custom node positions (e.g. after a drag).
/// `centers[i]` = centre of node i in final coordinates. Node
/// positions are NOT normalised (they stay exactly as given) so the
/// diagram doesn't "swim" while dragging, and [`to_svg`] draws them at
/// those coordinates: a drawing that reaches past the top-left margin
/// grows the canvas up or left ([`Scene::origin`]) and moves nothing.''')

# ---- fit_top_left: class/ER canvas
sub('src/scene.rs','''/// `sc` moved right/down just enough that nothing sits inside the
/// top-left `MARGIN` gutter, canvas grown to match. [`route`] keeps
/// caller positions verbatim, so a node dragged past the origin would
/// otherwise fall outside the `0 0 W H` viewBox of renderers that
/// draw `width`/`height` as-is (class, ER). Scenes already clear of
/// the gutter — every laid-out one — come back untouched.
pub(crate) fn fit_top_left(sc: &Scene) -> std::borrow::Cow<'_, Scene> {
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    if !bb.0.is_finite() {
        return std::borrow::Cow::Borrowed(sc);
    }
    let dx = (MARGIN - bb.0).max(0.0);
    let dy = (MARGIN - bb.2).max(0.0);
    if dx == 0.0 && dy == 0.0 {
        return std::borrow::Cow::Borrowed(sc);
    }
    let mut out = sc.clone();
    shift_scene(&mut out, dx, dy);
    out.width += dx;
    out.height += dy;
    std::borrow::Cow::Owned(out)
}
''','''/// A class or ER scene as its painter draws it on its canvas (the
/// painter draws `width` × `height` as-is, see [`fit_top_left`]).
pub(crate) struct Fitted<'a> {
    /// The scene to paint.
    pub(crate) scene: std::borrow::Cow<'a, Scene>,
    /// The canvas: its top-left corner (the `viewBox` origin) and size.
    pub(crate) origin: (f64, f64),
    pub(crate) width: f64,
    pub(crate) height: f64,
}

/// The canvas a class or ER painter draws `sc` on, so that nothing sits
/// inside the top-left `MARGIN` gutter. A routed scene ([`route`] keeps
/// caller positions verbatim) keeps its coordinates: its canvas grows up
/// and left over what reaches into the gutter ([`top_left_canvas`]), the
/// `viewBox` origin going negative (#76). Any other scene that reaches
/// into it is moved right/down just enough, its canvas grown to match,
/// as always. Scenes already clear of the gutter — every laid-out one —
/// come back untouched.
pub(crate) fn fit_top_left(sc: &Scene) -> Fitted<'_> {
    let borrowed = |origin, width, height| Fitted {
        scene: std::borrow::Cow::Borrowed(sc),
        origin,
        width,
        height,
    };
    if sc.routed {
        let (origin, width, height) = top_left_canvas(sc);
        return borrowed(origin, width, height);
    }
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    if !bb.0.is_finite() {
        return borrowed(sc.origin, sc.width, sc.height);
    }
    let dx = (MARGIN - bb.0).max(0.0);
    let dy = (MARGIN - bb.2).max(0.0);
    if dx == 0.0 && dy == 0.0 {
        return borrowed(sc.origin, sc.width, sc.height);
    }
    let mut out = sc.clone();
    shift_scene(&mut out, dx, dy);
    out.width += dx;
    out.height += dy;
    let (origin, width, height) = (out.origin, out.width, out.height);
    Fitted {
        scene: std::borrow::Cow::Owned(out),
        origin,
        width,
        height,
    }
}

/// `sc`'s canvas (origin, width, height) grown up and left over whatever
/// of the drawing (as [`grow_scene`] bounds it) reaches into the top-left
/// `MARGIN` gutter, nothing moved: the class and ER `route` set it on
/// their scene, and their painters draw on it. The far sides stay where
/// they are; a canvas already holding the drawing comes back as it is.
pub(crate) fn top_left_canvas(sc: &Scene) -> ((f64, f64), f64, f64) {
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &sc.nodes, &sc.edges, &sc.clusters);
    if !bb.0.is_finite() {
        return (sc.origin, sc.width, sc.height);
    }
    let ox = sc.origin.0.min(bb.0 - MARGIN);
    let oy = sc.origin.1.min(bb.2 - MARGIN);
    (
        (ox, oy),
        sc.width + (sc.origin.0 - ox),
        sc.height + (sc.origin.1 - oy),
    )
}
''')

# ---- flowchart to_svg
sub('src/scene.rs','''    let (minx, maxx, miny, maxy) = bb.finish();
    let tx = MARGIN - minx;
    let ty = MARGIN - miny;
    let width = (maxx - minx) + 2.0 * MARGIN;
    let height = (maxy - miny) + 2.0 * MARGIN;
    let t = |p: (f64, f64)| (p.0 + tx, p.1 + ty);
''','''    let (minx, maxx, miny, maxy) = bb.finish();
    // The canvas holds the drawing and its MARGIN all round. A routed
    // scene's positions are the host's and stay as they are (#76): the
    // `viewBox` starts where its canvas does. Any other drawing is moved
    // to start MARGIN in, as it always was (a laid-out one is there
    // already).
    let (tx, ox) = view_shift(sc.routed, MARGIN - minx);
    let (ty, oy) = view_shift(sc.routed, MARGIN - miny);
    let width = (maxx - minx) + 2.0 * MARGIN;
    let height = (maxy - miny) + 2.0 * MARGIN;
    let t = |p: (f64, f64)| (p.0 + tx, p.1 + ty);
''')
sub('src/scene.rs','''    svg_open_root(&mut s, width, height, 14, title, desc, dom_id, opts);''','''    svg_open_root(&mut s, (ox, oy), width, height, 14, title, desc, dom_id, opts);''')

sub('src/scene.rs','''/// Serialise any Scene (automatic or dragged) to SVG. Content is
/// translated to start inside the SVG's 16 px padding, so negative
/// coordinates are safe.''','''/// Serialise any Scene (automatic or dragged) to SVG. The canvas is the
/// drawing with [`MARGIN`] all round, so negative coordinates are safe.
/// A [`route`] / [`route_partial`] scene keeps its coordinates: SVG user
/// space is scene space, and the `viewBox` origin is where the canvas
/// starts (negative where the drawing reaches past zero), so a host maps
/// pointer coordinates through the `viewBox` straight into scene
/// coordinates for picking. A laid-out scene starts at `0 0`.''')

# helper view_shift right before to_svg doc
sub('src/scene.rs','''/// Serialise any Scene (automatic or dragged) to SVG. The canvas is the''','''/// How [`to_svg`] puts a drawing onto its canvas on one axis, `fit` being
/// how far the drawing starts short of [`MARGIN`] (negative: past it):
/// (shift of the drawing, `viewBox` origin). A routed scene keeps its
/// coordinates, the `viewBox` starting `fit` before zero; any other is
/// moved by `fit`, as is a routed one a float-noise `fit` away
/// ([`crate::edgepass::FIT_EPS`]) from its place, so its output stays
/// byte for byte.
fn view_shift(routed: bool, fit: f64) -> (f64, f64) {
    if routed && fit.abs() > crate::edgepass::FIT_EPS {
        (0.0, -fit)
    } else {
        (fit, 0.0)
    }
}

/// Serialise any Scene (automatic or dragged) to SVG. The canvas is the''')

# ---- svg_open_root with origin
sub('src/scene.rs','''    svg_open_root(s, width, height, font_size, title, desc, None, opts);
}
''','''    svg_open_root(s, (0.0, 0.0), width, height, font_size, title, desc, None, opts);
}

/// [`svg_open_a11y`] for a canvas whose top-left corner is `origin`
/// (the `viewBox` origin): a routed class or ER scene's ([`Scene::origin`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn svg_open_canvas(
    s: &mut String,
    origin: (f64, f64),
    width: f64,
    height: f64,
    font_size: u32,
    title: &str,
    desc: Option<&str>,
    opts: &SvgOptions,
) {
    svg_open_root(s, origin, width, height, font_size, title, desc, None, opts);
}

/// A `viewBox` origin coordinate as written: `0` at zero (so a canvas at
/// the origin writes `0 0`, byte for byte as before), else to 0.1 px
/// like every coordinate, without a trailing `.0`.
fn view_origin(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if r == 0.0 {
        "0".to_string()
    } else if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        format!("{r:.1}")
    }
}
''')
sub('src/scene.rs','''fn svg_open_root(
    s: &mut String,
    width: f64,
    height: f64,''','''fn svg_open_root(
    s: &mut String,
    origin: (f64, f64),
    width: f64,
    height: f64,''')
sub('src/scene.rs','''         viewBox=\\"0 0 {vw:.0} {vh:.0}\\"{par} font-family''','''         viewBox=\\"{ox} {oy} {vw:.0} {vh:.0}\\"{par} font-family''')
sub('src/scene.rs','''        vw = width,
        vh = height,
        fs = font_size,''','''        ox = view_origin(origin.0),
        oy = view_origin(origin.1),
        vw = width,
        vh = height,
        fs = font_size,''')
sub('src/scene.rs','''    if !opts.transparent {
        s.push_str(&format!(
            "<rect width=\\"{:.0}\\" height=\\"{:.0}\\" fill=\\"{}\\"/>\\n",
            width,
            height,
            opts.palette().background''','''    if !opts.transparent {
        // From the canvas's corner: `0 0`, written as before, needs no
        // `x` / `y`.
        let at = if view_origin(origin.0) == "0" && view_origin(origin.1) == "0" {
            String::new()
        } else {
            format!(
                "x=\\"{}\\" y=\\"{}\\" ",
                view_origin(origin.0),
                view_origin(origin.1)
            )
        };
        s.push_str(&format!(
            "<rect {at}width=\\"{:.0}\\" height=\\"{:.0}\\" fill=\\"{}\\"/>\\n",
            width,
            height,
            opts.palette().background''')

# ---- edgepass refit
sub('src/edgepass.rs','''pub(crate) fn refit(sc: &mut Scene, mode: Mode, static_fit: bool) {
    let mut bb = Bbox::new();''','''pub(crate) fn refit(sc: &mut Scene, mode: Mode, static_fit: bool) {
    if mode == Mode::Routed {
        sc.routed = true;
    }
    let mut bb = Bbox::new();''')
sub('src/edgepass.rs','''    let floor = if static_fit { MARGIN } else { 0.0 };
    if mode == Mode::Static && (minx < floor || miny < floor) {''','''    if mode == Mode::Routed && static_fit {
        // The host's positions stay as given (#76): a drawing reaching
        // into the top-left margin grows the canvas up or left (its
        // origin goes negative) and moves nothing; the far sides only
        // grow, as before. Float noise past the margin is no overflow.
        let far = (sc.origin.0 + sc.width, sc.origin.1 + sc.height);
        let past = |min: f64, o: f64| {
            if MARGIN - min > FIT_EPS {
                o.min(min - MARGIN)
            } else {
                o
            }
        };
        sc.origin = (past(minx, sc.origin.0), past(miny, sc.origin.1));
        sc.width = far.0.max(maxx + MARGIN) - sc.origin.0;
        sc.height = far.1.max(maxy + MARGIN) - sc.origin.1;
        return;
    }
    let floor = if static_fit { MARGIN } else { 0.0 };
    if mode == Mode::Static && (minx < floor || miny < floor) {''')
sub('src/edgepass.rs','''/// is never shifted: its node positions are the host's, exactly as given
/// (the export handles a negative origin, see `fit_top_left`); its canvas
/// only grows.''','''/// is never shifted: its node positions are the host's, exactly as given;
/// its canvas only grows, up or left too (a negative [`Scene::origin`],
/// #76) where the drawing reaches into the margin there.''')
sub('src/edgepass.rs','''const FIT_EPS: f64 = 1e-6;''','''pub(crate) const FIT_EPS: f64 = 1e-6;''')

# ---- class
sub('src/class.rs','''    let sc = route_sized(&b.g, &all, &b.sizes);
    assemble(d, b, sc, Mode::Routed)
}''','''    let sc = route_sized(&b.g, &all, &b.sizes);
    let mut cs = assemble(d, b, sc, Mode::Routed);
    // Positions stay as given (#76): the canvas grows up or left over
    // whatever reaches past its top-left margin.
    let (origin, width, height) = top_left_canvas(&cs.scene);
    (cs.scene.origin, cs.scene.width, cs.scene.height) = (origin, width, height);
    cs
}''')
sub('src/class.rs','''    let fitted = fit_top_left(&cs.scene);
    let sc = &*fitted;
    let pal = opts.palette();
    let mut s = String::new();
    svg_open_a11y(&mut s, sc.width, sc.height, 13, &title, desc, opts);''','''    let canvas = fit_top_left(&cs.scene);
    let sc = &*canvas.scene;
    let pal = opts.palette();
    let mut s = String::new();
    svg_open_canvas(
        &mut s,
        canvas.origin,
        canvas.width,
        canvas.height,
        13,
        &title,
        desc,
        opts,
    );''')
sub('src/class.rs','''                card_text(&mut s, at, text, sc.width, sc.height, &pal.text);''','''                card_text(&mut s, at, text, &canvas, &pal.text);''')
sub('src/class.rs','''fn card_text(s: &mut String, at: (f64, f64), text: &str, cw: f64, ch: f64, ink: &str) {
    // Clamp to the canvas so a long cardinality can't spill off-edge
    // (the scene bbox is sized for boxes/edges, not these labels).
    let hw = card_w(text) / 2.0;
    let px = at.0.clamp(hw, (cw - hw).max(hw));
    let py = at.1.clamp(7.0, (ch - 7.0).max(7.0));''','''fn card_text(s: &mut String, at: (f64, f64), text: &str, canvas: &Fitted, ink: &str) {
    // Clamp to the canvas so a long cardinality can't spill off-edge
    // (the scene bbox is sized for boxes/edges, not these labels).
    let hw = card_w(text) / 2.0;
    let ((x0, y0), cw, ch) = (canvas.origin, canvas.width, canvas.height);
    let px = at.0.clamp(x0 + hw, x0 + (cw - hw).max(hw));
    let py = at.1.clamp(y0 + 7.0, y0 + (ch - 7.0).max(7.0));''')
sub('src/class.rs','''    all_finite, edge_end_chord, edge_path_d, edge_polyline, edge_stroke_width, escape,
    fit_top_left, route_sized, sample_cubic, scene_sized, svg_label_box, svg_open_a11y,
    svg_text_multiline, A11y, Scene, SceneEdge, SceneNode, SvgOptions,''','''    all_finite, edge_end_chord, edge_path_d, edge_polyline, edge_stroke_width, escape,
    fit_top_left, route_sized, sample_cubic, scene_sized, svg_label_box, svg_open_canvas,
    svg_text_multiline, top_left_canvas, A11y, Fitted, Scene, SceneEdge, SceneNode, SvgOptions,''')
sub('src/er.rs','''    scene_sized, seg_aabb, svg_label_box, svg_open_a11y, A11y, Scene, SceneEdge, SvgOptions,''','''    scene_sized, seg_aabb, svg_label_box, svg_open_canvas, top_left_canvas, A11y, Scene, SceneEdge,
    SvgOptions,''')

# ---- er
sub('src/er.rs','''pub fn route(d: &ErDiagram, centers: &[(f64, f64)]) -> ErScene {
    let (g, tables, sizes) = build(d);
    assemble(d, tables, route_sized(&g, centers, &sizes), &g)
}''','''pub fn route(d: &ErDiagram, centers: &[(f64, f64)]) -> ErScene {
    let (g, tables, sizes) = build(d);
    let mut es = assemble(d, tables, route_sized(&g, centers, &sizes), &g);
    // Positions stay as given (#76): the canvas grows up or left over
    // whatever reaches past its top-left margin.
    let (origin, width, height) = top_left_canvas(&es.scene);
    (es.scene.origin, es.scene.width, es.scene.height) = (origin, width, height);
    es
}''')
sub('src/er.rs','''    let fitted = fit_top_left(&es.scene);
    let sc = &*fitted;
    let pal = opts.palette();
    let mut s = String::new();
    svg_open_a11y(&mut s, sc.width, sc.height, 13, &title, desc, opts);''','''    let canvas = fit_top_left(&es.scene);
    let sc = &*canvas.scene;
    let pal = opts.palette();
    let mut s = String::new();
    svg_open_canvas(
        &mut s,
        canvas.origin,
        canvas.width,
        canvas.height,
        13,
        &title,
        desc,
        opts,
    );''')
print("ok")
