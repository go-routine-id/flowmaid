// Independent probe of the #111 contract (route_partial for ER, class, mindmap).
// usage: contract <listfile>   (one .mmd path per line; types picked by parse)
use flowmaid::model::{Document, Mindmap};
use flowmaid::parser::parse_document;
use flowmaid::{class, er, mindmap};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Default)]
struct St {
    checks: usize,
    fails: Vec<String>,
    per: HashMap<String, (usize, usize)>,
    notes: HashMap<String, usize>,
}
impl St {
    fn check(&mut self, ok: bool, tag: &str, ctx: &str) {
        self.checks += 1;
        let e = self.per.entry(tag.to_string()).or_default();
        e.0 += 1;
        if !ok {
            e.1 += 1;
            if self.fails.len() < 400 {
                self.fails.push(format!("FAIL [{tag}] {ctx}"));
            }
        }
    }
    fn note(&mut self, k: &str) {
        *self.notes.entry(k.to_string()).or_default() += 1;
    }
}

fn dbg<T: std::fmt::Debug>(t: &T) -> String {
    format!("{t:?}")
}

fn quiet<R>(f: impl FnOnce() -> R) -> Result<R, ()> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|_| ())
}

fn view_box(svg: &str) -> Option<(f64, f64, f64, f64)> {
    let i = svg.find("viewBox=\"")? + 9;
    let rest = &svg[i..];
    let j = rest.find('"')?;
    let v: Vec<f64> = rest[..j]
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    (v.len() == 4).then(|| (v[0], v[1], v[2], v[3]))
}

fn sample(n: usize) -> Vec<usize> {
    let mut v = vec![0];
    if n > 1 {
        v.push(n - 1);
    }
    if n > 2 {
        v.push(n / 2);
    }
    if n > 5 {
        v.push(n / 3);
    }
    v.sort();
    v.dedup();
    v
}

const DELTAS: [(f64, f64); 4] = [(37.0, 23.0), (-3000.0, -2000.0), (1500.0, 900.0), (-20.0, 0.5)];


/// Every drawn coordinate of `svg` (path, line, polygon, rect corners,
/// circle box, text anchor) inside its viewBox (to `slack` px), or the
/// worst overshoot.
fn ink_outside(svg: &str, slack: f64) -> f64 {
    let Some((vx, vy, vw, vh)) = view_box(svg) else { return f64::INFINITY };
    let mut worst: f64 = 0.0;
    let mut pt = |x: f64, y: f64| {
        let o = (vx - x).max(x - (vx + vw)).max(vy - y).max(y - (vy + vh));
        if o > slack {
            worst = worst.max(o);
        }
    };
    let attr = |tag: &str, name: &str| -> Option<f64> {
        let k = format!(" {name}=\"");
        let i = tag.find(&k)? + k.len();
        let j = tag[i..].find('"')?;
        tag[i..i + j].parse().ok()
    };
    for raw in svg.split('<').skip(1) {
        let tag = raw.split('>').next().unwrap_or("");
        let name = tag.split_whitespace().next().unwrap_or("");
        match name {
            "path" => {
                if let Some(i) = tag.find(" d=\"") {
                    let rest = &tag[i + 4..];
                    let d = &rest[..rest.find('"').unwrap_or(rest.len())];
                    // tokens: commands and numbers
                    let mut toks: Vec<String> = Vec::new();
                    let mut cur = String::new();
                    for ch in d.chars() {
                        if ch.is_ascii_alphabetic() && ch != 'e' {
                            if !cur.is_empty() { toks.push(std::mem::take(&mut cur)); }
                            toks.push(ch.to_string());
                        } else if ch == ',' || ch.is_whitespace() {
                            if !cur.is_empty() { toks.push(std::mem::take(&mut cur)); }
                        } else { cur.push(ch); }
                    }
                    if !cur.is_empty() { toks.push(cur); }
                    let mut cmd = 'M';
                    let mut last = (0.0f64, 0.0f64);
                    let mut k = 0;
                    let num = |k: usize| -> f64 { toks.get(k).and_then(|t| t.parse().ok()).unwrap_or(f64::NAN) };
                    while k < toks.len() {
                        if let Some(c) = toks[k].chars().next().filter(|c| c.is_ascii_alphabetic()) { cmd = c; k += 1; if cmd == 'Z' || cmd == 'z' { continue; } }
                        match cmd {
                            'M' | 'L' => { last = (num(k), num(k + 1)); pt(last.0, last.1); k += 2; }
                            'H' => { last.0 = num(k); pt(last.0, last.1); k += 1; }
                            'V' => { last.1 = num(k); pt(last.0, last.1); k += 1; }
                            'C' => {
                                let (a, b, c) = ((num(k), num(k + 1)), (num(k + 2), num(k + 3)), (num(k + 4), num(k + 5)));
                                for t in 0..=32 { let t = t as f64 / 32.0; let u = 1.0 - t;
                                    let x = u*u*u*last.0 + 3.0*u*u*t*a.0 + 3.0*u*t*t*b.0 + t*t*t*c.0;
                                    let y = u*u*u*last.1 + 3.0*u*u*t*a.1 + 3.0*u*t*t*b.1 + t*t*t*c.1; pt(x, y); }
                                last = c; k += 6;
                            }
                            'Q' => {
                                let (a, c) = ((num(k), num(k + 1)), (num(k + 2), num(k + 3)));
                                for t in 0..=32 { let t = t as f64 / 32.0; let u = 1.0 - t;
                                    pt(u*u*last.0 + 2.0*u*t*a.0 + t*t*c.0, u*u*last.1 + 2.0*u*t*a.1 + t*t*c.1); }
                                last = c; k += 4;
                            }
                            _ => { k += 1; }
                        }
                    }
                }
            }
            "line" => {
                if let (Some(a), Some(b), Some(c), Some(d)) = (attr(tag, "x1"), attr(tag, "y1"), attr(tag, "x2"), attr(tag, "y2")) {
                    pt(a, b);
                    pt(c, d);
                }
            }
            "polygon" | "polyline" => {
                if let Some(i) = tag.find(" points=\"") {
                    let rest = &tag[i + 9..];
                    let d = &rest[..rest.find('"').unwrap_or(rest.len())];
                    let nums: Vec<f64> = d
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .filter_map(|t| t.parse().ok())
                        .collect();
                    for c in nums.chunks(2) {
                        if c.len() == 2 {
                            pt(c[0], c[1]);
                        }
                    }
                }
            }
            "rect" => {
                if let (Some(x), Some(y), Some(w), Some(h)) = (attr(tag, "x"), attr(tag, "y"), attr(tag, "width"), attr(tag, "height")) {
                    pt(x, y);
                    pt(x + w, y + h);
                }
            }
            "circle" => {
                if let (Some(x), Some(y), Some(r)) = (attr(tag, "cx"), attr(tag, "cy"), attr(tag, "r")) {
                    pt(x - r, y - r);
                    pt(x + r, y + r);
                }
            }
            "text" => {
                if let (Some(x), Some(y)) = (attr(tag, "x"), attr(tag, "y")) {
                    pt(x, y);
                }
            }
            _ => {}
        }
    }
    worst
}

// ---------------------------------------------------------------- ER
fn er_of(src: &str) -> Option<flowmaid::model::ErDiagram> {
    match parse_document(src) {
        Ok(Document::Er(d)) => Some(d),
        _ => None,
    }
}

fn er_frame_ok(
    st: &mut St,
    name: &str,
    tag: &str,
    d: &flowmaid::model::ErDiagram,
    base: &er::ErScene,
    f: &er::ErScene,
    mv: &[(f64, f64)],
    moved: &[bool],
) {
    let sc = &f.scene;
    let ctx = |s: &str| format!("{name} {s}");
    st.check(sc.nodes.len() == mv.len(), "er.count", &ctx(tag));
    for (k, nd) in sc.nodes.iter().enumerate() {
        st.check(
            nd.x == mv[k].0 && nd.y == mv[k].1,
            "er.verbatim",
            &ctx(&format!("{tag} node {k} at ({},{}) want {:?}", nd.x, nd.y, mv[k])),
        );
        let (ox, oy) = sc.origin;
        let inside = nd.x - nd.w / 2.0 >= ox - 1e-6
            && nd.y - nd.h / 2.0 >= oy - 1e-6
            && nd.x + nd.w / 2.0 <= ox + sc.width + 1e-6
            && nd.y + nd.h / 2.0 <= oy + sc.height + 1e-6;
        st.check(inside, "er.canvas_holds_nodes", &ctx(&format!("{tag} node {k}")));
    }
    for (j, r) in d.relations.iter().enumerate() {
        if !moved[r.from] && !moved[r.to] {
            st.check(
                dbg(&sc.edges[j]) == dbg(&base.scene.edges[j]),
                "er.kept_edge_identical",
                &ctx(&format!("{tag} edge {j}")),
            );
            st.check(f.cards[j] == base.cards[j], "er.kept_cards", &ctx(tag));
        }
    }
    // canvas never shrinks
    let (bo, fo) = (base.scene.origin, sc.origin);
    st.check(
        fo.0 <= bo.0 + 1e-9
            && fo.1 <= bo.1 + 1e-9
            && fo.0 + sc.width >= bo.0 + base.scene.width - 1e-9
            && fo.1 + sc.height >= bo.1 + base.scene.height - 1e-9,
        "er.canvas_never_shrinks",
        &ctx(tag),
    );
    let svg = er::to_svg(f);
    if let Some((vx, vy, vw, vh)) = view_box(&svg) {
        let ok = sc.nodes.iter().all(|nd| {
            nd.x - nd.w / 2.0 >= vx - 0.06
                && nd.y - nd.h / 2.0 >= vy - 0.06
                && nd.x + nd.w / 2.0 <= vx + vw + 0.06
                && nd.y + nd.h / 2.0 <= vy + vh + 0.06
        });
        st.check(ok, "er.viewbox_holds_nodes_in_scene_coords", &ctx(tag));
        let o = ink_outside(&svg, 0.5);
        st.check(o == 0.0, "er.frame_ink_inside_viewbox", &ctx(&format!("{tag} by {o:.1}")));
        let r = er::route(d, mv);
        let ro = ink_outside(&er::to_svg(&r), 0.5);
        if ro > 0.0 { st.note("info.er.route_ink_outside_viewbox"); }
        st.note("info.er.route_ink_checked");
    } else {
        st.check(false, "er.viewbox_parse", &ctx(tag));
    }
}

fn er_file(st: &mut St, name: &str, src: &str) {
    let d = er_of(src).unwrap();
    let base = er::scene(&d);
    let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let n = at.len();
    if n == 0 {
        return;
    }
    st.note("er.files");
    let f0 = er::route_partial(&d, &at, &base, &at);
    st.check(dbg(&f0) == dbg(&base), "er.no_move_is_base", name);
    let routed = er::route(&d, &at);
    for (bname, b) in [("scene", &base), ("route", &routed)] {
        for i in sample(n) {
            for &(dx, dy) in &DELTAS {
                let mut mv = at.clone();
                mv[i].0 += dx;
                mv[i].1 += dy;
                let moved: Vec<bool> = (0..n).map(|k| k == i).collect();
                let f = er::route_partial(&d, &mv, b, &at);
                let tag = format!("{bname} i={i} d=({dx},{dy})");
                er_frame_ok(st, name, &tag, &d, b, &f, &mv, &moved);
                let t = er::try_route_partial(&d, &mv, b, &at);
                st.check(t.map(|t| dbg(&t)) == Some(dbg(&f)), "er.try_eq_rp", name);
                // chain: same node again
                let mut mv2 = mv.clone();
                mv2[i].0 += 11.0;
                mv2[i].1 -= 7.0;
                let f2 = er::route_partial(&d, &mv2, &f, &mv);
                er_frame_ok(st, name, &format!("{tag} chain"), &d, &f, &f2, &mv2, &moved);
                if n > 1 && bname == "scene" {
                    let j = (i + 1) % n;
                    let mut mv3 = mv.clone();
                    mv3[j].0 += 19.0;
                    let m3: Vec<bool> = (0..n).map(|k| k == j).collect();
                    let f3 = er::route_partial(&d, &mv3, &f, &mv);
                    er_frame_ok(st, name, &format!("{tag} chain-other"), &d, &f, &f3, &mv3, &m3);
                }
            }
        }
    }
    // two nodes moved at once
    if n > 2 {
        let mut mv = at.clone();
        mv[0].0 += 30.0;
        mv[n - 1].1 += 40.0;
        let moved: Vec<bool> = (0..n).map(|k| k == 0 || k == n - 1).collect();
        let f = er::route_partial(&d, &mv, &base, &at);
        er_frame_ok(st, name, "two-moved", &d, &base, &f, &mv, &moved);
    }
    let mut mv = at.clone();
    mv[0].0 += 25.0;
    mv[0].1 += 15.0;
    let full = dbg(&er::route(&d, &mv));
    // base_centers wrong count / not where base was drawn / NaN
    if n > 1 {
        let rp = er::route_partial(&d, &mv, &base, &at[..n - 1]);
        st.check(dbg(&rp) == full, "er.fallback_base_count", name);
    }
    let mut ext = at.clone();
    ext.push((1.0, 1.0));
    st.check(dbg(&er::route_partial(&d, &mv, &base, &ext)) == full, "er.fallback_base_count", name);
    let shifted: Vec<(f64, f64)> = at.iter().map(|p| (p.0 + 5.0, p.1)).collect();
    st.check(dbg(&er::route_partial(&d, &mv, &base, &shifted)) == full, "er.fallback_base_not_at", name);
    let mut nan_at = at.clone();
    nan_at[0].0 = f64::NAN;
    st.check(dbg(&er::route_partial(&d, &mv, &base, &nan_at)) == full, "er.fallback_base_nan", name);
    // counts 0..n+5
    for k in 0..n + 6 {
        let c: Vec<(f64, f64)> = (0..k)
            .map(|t| if t < n { mv[t] } else { (10.0 * t as f64, 5.0) })
            .collect();
        let a = quiet(|| dbg(&er::route(&d, &c)));
        let b = quiet(|| dbg(&er::route_partial(&d, &c, &base, &at)));
        match (&a, &b) {
            (Ok(x), Ok(y)) => st.check(x == y || k == n, "er.count_like_route", &format!("{name} k={k}")),
            (Err(_), Err(_)) => st.check(true, "er.count_like_route", name),
            _ => st.check(false, "er.count_like_route_panic_mismatch", &format!("{name} k={k} route_ok={} rp_ok={}", a.is_ok(), b.is_ok())),
        }
        if b.is_err() {
            st.note("er.rp_panics_on_count (as route)");
        }
        let t = quiet(|| er::try_route_partial(&d, &c, &base, &at).is_some());
        st.check(t == Ok(k == n), "er.try_count", &format!("{name} k={k} {t:?}"));
    }
    // non-finite / far coordinates
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 2e6] {
        let mut c = mv.clone();
        c[n / 2].1 = bad;
        let t = quiet(|| er::try_route_partial(&d, &c, &base, &at).is_none());
        st.check(t == Ok(true), "er.try_nonfinite_none", &format!("{name} {bad}"));
        let a = quiet(|| er::route(&d, &c).scene.nodes.len());
        let b = quiet(|| er::route_partial(&d, &c, &base, &at).scene.nodes.len());
        if a.is_ok() != b.is_ok() {
            st.check(false, "er.nonfinite_panic_like_route", &format!("{name} {bad} route_ok={} rp_ok={}", a.is_ok(), b.is_ok()));
        } else {
            st.check(true, "er.nonfinite_panic_like_route", name);
        }
        if b.is_err() {
            st.note(&format!("er.rp_panics_nonfinite {bad}"));
        }
    }
    er_edits(st, name, src, &d, &base, &at);
}

fn er_edits(
    st: &mut St,
    name: &str,
    src: &str,
    d: &flowmaid::model::ErDiagram,
    base: &er::ErScene,
    at: &[(f64, f64)],
) {
    let n = at.len();
    let m = d.relations.len();
    let mut edits: Vec<(&str, flowmaid::model::ErDiagram)> = Vec::new();
    let e0 = d.entities[0].name.clone();
    let e1 = d.entities[(1).min(n - 1)].name.clone();
    if let Some(x) = er_of(&format!("{src}\n    ZZQNEW {{\n        string zzq\n    }}\n")) {
        edits.push(("add-entity", x));
    }
    if let Some(x) = er_of(&format!("{src}\n    {e0} ||--o{{ {e1} : zzqrel\n")) {
        edits.push(("add-relation", x));
    }
    if m > 0 {
        let mut x = er_of(src).unwrap();
        x.relations.pop();
        edits.push(("remove-relation", x));
        let mut x = er_of(src).unwrap();
        x.relations[0].label = Some("zz edited label".into());
        edits.push(("relabel", x));
        let mut x = er_of(src).unwrap();
        x.relations[0].card_from = if x.relations[0].card_from == flowmaid::model::Card::One {
            flowmaid::model::Card::ZeroMany
        } else {
            flowmaid::model::Card::One
        };
        edits.push(("cardinality", x));
        let mut x = er_of(src).unwrap();
        x.relations[0].identifying = !x.relations[0].identifying;
        edits.push(("identifying", x));
        if n > 2 {
            let mut x = er_of(src).unwrap();
            let r = &mut x.relations[0];
            let other = (0..n).find(|&k| k != r.from && k != r.to).unwrap();
            r.to = other;
            edits.push(("rewire", x));
        }
    }
    let mut x = er_of(src).unwrap();
    x.entities[0].alias = Some("A Much Longer Alias For This Entity".into());
    edits.push(("alias", x));
    // remove the last entity if nothing refers to it
    if !d.relations.iter().any(|r| r.from == n - 1 || r.to == n - 1) && n > 1 {
        let mut x = er_of(src).unwrap();
        x.entities.pop();
        edits.push(("remove-entity", x));
    }
    for (what, d2) in &edits {
        let n2 = d2.entities.len();
        let mut mv: Vec<(f64, f64)> = (0..n2).map(|k| if k < n { at[k] } else { (40.0, 40.0) }).collect();
        mv[0].0 += 25.0;
        let full = dbg(&er::route(d2, &mv));
        let rp = er::route_partial(d2, &mv, base, at);
        st.check(dbg(&rp) == full, "er.edit_equals_route", &format!("{name} {what}"));
        // base_centers padded to the new count
        let at2: Vec<(f64, f64)> = (0..n2).map(|k| if k < n { at[k] } else { (40.0, 40.0) }).collect();
        let rp = er::route_partial(d2, &mv, base, &at2);
        st.check(dbg(&rp) == full, "er.edit_equals_route", &format!("{name} {what} padded"));
    }
}

// ---------------------------------------------------------------- class
fn class_of(src: &str) -> Option<flowmaid::model::ClassDiagram> {
    match parse_document(src) {
        Ok(Document::Class(d)) => Some(d),
        _ => None,
    }
}

fn class_frame_ok(
    st: &mut St,
    name: &str,
    tag: &str,
    base: &class::ClassScene,
    f: &class::ClassScene,
    mv_all: &[(f64, f64)],
) {
    let sc = &f.scene;
    let ctx = |s: &str| format!("{name} {s}");
    if sc.nodes.len() != mv_all.len() {
        st.check(false, "class.count", &ctx(tag));
        return;
    }
    let idx: HashMap<&str, usize> = sc.nodes.iter().enumerate().map(|(k, n)| (n.id.as_str(), k)).collect();
    let bnodes = &base.scene.nodes;
    let moved: Vec<bool> = (0..sc.nodes.len())
        .map(|k| (bnodes[k].x, bnodes[k].y) != mv_all[k])
        .collect();
    for (k, nd) in sc.nodes.iter().enumerate() {
        st.check(
            nd.x == mv_all[k].0 && nd.y == mv_all[k].1,
            "class.verbatim",
            &ctx(&format!("{tag} node {k} ({},{}) want {:?}", nd.x, nd.y, mv_all[k])),
        );
        let (ox, oy) = sc.origin;
        let inside = nd.x - nd.w / 2.0 >= ox - 1e-6
            && nd.y - nd.h / 2.0 >= oy - 1e-6
            && nd.x + nd.w / 2.0 <= ox + sc.width + 1e-6
            && nd.y + nd.h / 2.0 <= oy + sc.height + 1e-6;
        st.check(inside, "class.canvas_holds_nodes", &ctx(&format!("{tag} node {k}")));
    }
    let bc = class::card_spots(base);
    let fc = class::card_spots(f);
    for (j, e) in sc.edges.iter().enumerate() {
        let (Some(&a), Some(&b)) = (idx.get(e.from.as_str()), idx.get(e.to.as_str())) else {
            st.check(false, "class.edge_ids", &ctx(tag));
            continue;
        };
        if !moved[a] && !moved[b] {
            st.check(
                dbg(e) == dbg(&base.scene.edges[j]),
                "class.kept_edge_identical",
                &ctx(&format!("{tag} edge {j}")),
            );
            if j < bc.len() {
                st.check(bc[j] == fc[j], "class.kept_cards_identical", &ctx(&format!("{tag} rel {j} {:?} vs {:?}", bc[j], fc[j])));
            }
        }
    }
    let (bo, fo) = (base.scene.origin, sc.origin);
    st.check(
        fo.0 <= bo.0 + 1e-9
            && fo.1 <= bo.1 + 1e-9
            && fo.0 + sc.width >= bo.0 + base.scene.width - 1e-9
            && fo.1 + sc.height >= bo.1 + base.scene.height - 1e-9,
        "class.canvas_never_shrinks",
        &ctx(tag),
    );
    let svg = class::to_svg(f);
    if let Some((vx, vy, vw, vh)) = view_box(&svg) {
        let ok = sc.nodes.iter().all(|nd| {
            nd.x - nd.w / 2.0 >= vx - 0.06
                && nd.y - nd.h / 2.0 >= vy - 0.06
                && nd.x + nd.w / 2.0 <= vx + vw + 0.06
                && nd.y + nd.h / 2.0 <= vy + vh + 0.06
        });
        st.check(ok, "class.viewbox_holds_nodes_in_scene_coords", &ctx(tag));
        let o = ink_outside(&svg, 0.5);
        st.check(o == 0.0, "class.frame_ink_inside_viewbox", &ctx(&format!("{tag} by {o:.1}")));
    }
}

fn class_file(st: &mut St, name: &str, src: &str) {
    let d = class_of(src).unwrap();
    let base = class::scene(&d);
    let all: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let n = d.classes.len();
    let k = d.notes.len();
    if n == 0 || all.len() != n + k {
        if n > 0 {
            st.note("class.nodes_not_n_plus_notes");
        }
        return;
    }
    st.note("class.files");
    if k > 0 {
        st.note("class.files_with_notes");
    }
    st.check(dbg(&class::route_partial(&d, &all, &base, &all)) == dbg(&base), "class.no_move_is_base", name);
    let routed = class::route(&d, &all);
    let nodes = n + k;
    for (bname, b) in [("scene", &base), ("route", &routed)] {
        for i in sample(nodes) {
            for &(dx, dy) in &DELTAS {
                let mut mv = all.clone();
                mv[i].0 += dx;
                mv[i].1 += dy;
                let f = class::route_partial(&d, &mv, b, &all);
                let tag = format!("{bname} i={i} d=({dx},{dy})");
                class_frame_ok(st, name, &tag, b, &f, &mv);
                let t = class::try_route_partial(&d, &mv, b, &all);
                st.check(t.map(|t| dbg(&t)) == Some(dbg(&f)), "class.try_eq_rp", name);
                let mut mv2 = mv.clone();
                mv2[i].0 += 11.0;
                mv2[i].1 -= 7.0;
                let f2 = class::route_partial(&d, &mv2, &f, &mv);
                class_frame_ok(st, name, &format!("{tag} chain"), &f, &f2, &mv2);
                if nodes > 1 && bname == "scene" {
                    let j = (i + 1) % nodes;
                    let mut mv3 = mv.clone();
                    mv3[j].0 += 19.0;
                    let f3 = class::route_partial(&d, &mv3, &f, &mv);
                    class_frame_ok(st, name, &format!("{tag} chain-other"), &f, &f3, &mv3);
                }
            }
        }
    }
    // classes-only centres (notes placed as route places them)
    {
        let cls: Vec<(f64, f64)> = all[..n].to_vec();
        let rbase = class::route(&d, &cls);
        let rall: Vec<(f64, f64)> = rbase.scene.nodes.iter().map(|q| (q.x, q.y)).collect();
        let mut mv = cls.clone();
        mv[0].0 += 33.0;
        let f = class::route_partial(&d, &mv, &rbase, &cls);
        // expected positions: route's for these classes-only centres
        let want = class::route(&d, &mv);
        let want_all: Vec<(f64, f64)> = want.scene.nodes.iter().map(|q| (q.x, q.y)).collect();
        class_frame_ok(st, name, "classes-only", &rbase, &f, &want_all);
        let _ = rall;
        // mixed counts: centres n, base_centres n+k and vice versa
        let f = quiet(|| class::route_partial(&d, &mv, &base, &all));
        st.check(f.is_ok(), "class.mixed_counts_no_panic", name);
        if let Ok(f) = f {
            let want_all: Vec<(f64, f64)> = want.scene.nodes.iter().map(|q| (q.x, q.y)).collect();
            st.check(
                f.scene.nodes.iter().zip(&want_all).all(|(a, b)| (a.x, a.y) == *b),
                "class.mixed_counts_positions",
                name,
            );
        }
        let mut mvall = all.clone();
        mvall[0].0 += 33.0;
        let f = quiet(|| class::route_partial(&d, &mvall, &rbase, &cls));
        st.check(f.is_ok(), "class.mixed_counts_no_panic", name);
        if let Ok(f) = f {
            st.check(
                f.scene.nodes.iter().zip(&mvall).all(|(a, b)| (a.x, a.y) == *b),
                "class.mixed_counts_positions",
                name,
            );
        }
    }
    let mut mv = all.clone();
    mv[0].0 += 25.0;
    mv[0].1 += 15.0;
    let full = dbg(&class::route(&d, &mv));
    let rp = class::route_partial(&d, &mv, &base, &all[..all.len() - 1]);
    let bad_count = all.len() - 1 != n; // n+k-1 might equal n when k==1
    if bad_count {
        st.check(dbg(&rp) == full, "class.fallback_base_count", name);
    }
    let shifted: Vec<(f64, f64)> = all.iter().map(|p| (p.0 + 5.0, p.1)).collect();
    st.check(dbg(&class::route_partial(&d, &mv, &base, &shifted)) == full, "class.fallback_base_not_at", name);
    let mut nan_at = all.clone();
    nan_at[0].1 = f64::NAN;
    st.check(dbg(&class::route_partial(&d, &mv, &base, &nan_at)) == full, "class.fallback_base_nan", name);
    for c in 0..nodes + 6 {
        let cc: Vec<(f64, f64)> = (0..c)
            .map(|t| if t < nodes { mv[t] } else { (10.0 * t as f64, 5.0) })
            .collect();
        let a = quiet(|| dbg(&class::route(&d, &cc)));
        let b = quiet(|| dbg(&class::route_partial(&d, &cc, &base, &all)));
        let accepted = c == n || c == n + k;
        match (&a, &b) {
            (Ok(x), Ok(y)) => st.check(x == y || accepted, "class.count_like_route", &format!("{name} c={c}")),
            (Err(_), Err(_)) => st.check(true, "class.count_like_route", name),
            _ => st.check(false, "class.count_like_route_panic_mismatch", &format!("{name} c={c} route_ok={} rp_ok={}", a.is_ok(), b.is_ok())),
        }
        let t = quiet(|| class::try_route_partial(&d, &cc, &base, &all).is_some());
        st.check(t == Ok(accepted), "class.try_count", &format!("{name} c={c} {t:?}"));
        let tr = quiet(|| class::try_route(&d, &cc).is_some());
        st.check(t == tr, "class.try_count_like_try_route", name);
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 2e6] {
        let mut c = mv.clone();
        c[nodes / 2].0 = bad;
        let t = quiet(|| class::try_route_partial(&d, &c, &base, &all).is_none());
        st.check(t == Ok(true), "class.try_nonfinite_none", &format!("{name} {bad}"));
        let a = quiet(|| class::route(&d, &c).scene.nodes.len());
        let b = quiet(|| class::route_partial(&d, &c, &base, &all).scene.nodes.len());
        st.check(a.is_ok() == b.is_ok(), "class.nonfinite_panic_like_route", &format!("{name} {bad} route_ok={} rp_ok={}", a.is_ok(), b.is_ok()));
        if b.is_err() {
            st.note(&format!("class.rp_panics_nonfinite {bad}"));
        }
    }
    class_edits(st, name, src, &d, &base, &all);
}

fn class_edits(
    st: &mut St,
    name: &str,
    src: &str,
    d: &flowmaid::model::ClassDiagram,
    base: &class::ClassScene,
    all: &[(f64, f64)],
) {
    let n = d.classes.len();
    let mut edits: Vec<(&str, flowmaid::model::ClassDiagram)> = Vec::new();
    let c0 = d.classes[0].name.clone();
    let c1 = d.classes[(1).min(n - 1)].name.clone();
    if let Some(x) = class_of(&format!("{src}\n    class ZZQNEW\n")) {
        edits.push(("add-class", x));
    }
    if let Some(x) = class_of(&format!("{src}\n    {c0} --> {c1} : zzq\n")) {
        edits.push(("add-relation", x));
    }
    if let Some(x) = class_of(&format!("{src}\n    note for {c0} \"zz new note\"\n")) {
        edits.push(("add-note", x));
    }
    if let Some(x) = class_of(&format!("{src}\n    {c0} : +int zzqMember\n")) {
        edits.push(("add-member", x));
    }
    if !d.notes.is_empty() {
        let mut x = class_of(src).unwrap();
        x.notes[0].text = "zz edited note text that is longer".into();
        edits.push(("note-text", x));
        let mut x = class_of(src).unwrap();
        x.notes.pop();
        edits.push(("remove-note", x));
    }
    if !d.relations.is_empty() {
        let mut x = class_of(src).unwrap();
        x.relations.pop();
        edits.push(("remove-relation", x));
        let mut x = class_of(src).unwrap();
        x.relations[0].label = Some("zz relabel".into());
        edits.push(("relabel", x));
        let mut x = class_of(src).unwrap();
        x.relations[0].from_card = Some("0..zz".into());
        edits.push(("cardinality", x));
        let mut x = class_of(src).unwrap();
        x.relations[0].dashed = !x.relations[0].dashed;
        edits.push(("dashed", x));
        if n > 2 {
            let mut x = class_of(src).unwrap();
            let r = &mut x.relations[0];
            let other = (0..n).find(|&k| k != r.from && k != r.to).unwrap();
            r.to = other;
            edits.push(("rewire", x));
        }
    }
    let mut x = class_of(src).unwrap();
    x.classes[0].annotations.push("zzinterface".into());
    edits.push(("annotation", x));
    for (what, d2) in &edits {
        let n2 = d2.classes.len() + d2.notes.len();
        let base_n = all.len();
        // new centres: keep the old ones, new boxes at (40,40)
        let mut mv: Vec<(f64, f64)> = Vec::new();
        let (bn, bk) = (n, d.notes.len());
        let (n2c, n2k) = (d2.classes.len(), d2.notes.len());
        for c in 0..n2c {
            mv.push(if c < bn { all[c] } else { (40.0, 40.0) });
        }
        for t in 0..n2k {
            mv.push(if t < bk { all[bn + t] } else { (60.0, 60.0) });
        }
        let _ = base_n;
        mv[0].0 += 25.0;
        debug_assert_eq!(mv.len(), n2);
        let full = dbg(&class::route(d2, &mv));
        let rp = class::route_partial(d2, &mv, base, all);
        st.check(dbg(&rp) == full, "class.edit_equals_route", &format!("{name} {what}"));
        // base_centres padded to the new counts
        let mut padded = mv.clone();
        padded[0].0 -= 25.0;
        let rp = class::route_partial(d2, &mv, base, &padded);
        st.check(dbg(&rp) == full, "class.edit_equals_route", &format!("{name} {what} padded"));
        // classes-only centres too
        let cls: Vec<(f64, f64)> = mv[..n2c].to_vec();
        let full = dbg(&class::route(d2, &cls));
        let rp = class::route_partial(d2, &cls, base, &all[..n]);
        st.check(dbg(&rp) == full, "class.edit_equals_route", &format!("{name} {what} classes-only"));
    }
}

// ---------------------------------------------------------------- mindmap
fn mind_of(src: &str) -> Option<Mindmap> {
    match parse_document(src) {
        Ok(Document::Mindmap(d)) => Some(d),
        _ => None,
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0) || (a.is_nan() && b.is_nan())
}

fn mind_eq_route(f: &mindmap::MindScene, r: &mindmap::MindScene) -> bool {
    f.nodes.len() == r.nodes.len()
        && f.edges.len() == r.edges.len()
        && close(f.width, r.width)
        && close(f.height, r.height)
        && f.nodes.iter().zip(&r.nodes).all(|(a, b)| {
            close(a.x, b.x)
                && close(a.y, b.y)
                && a.w == b.w
                && a.h == b.h
                && a.text == b.text
                && a.fill == b.fill
                && a.text_color == b.text_color
                && a.font_size == b.font_size
        })
        && f.edges.iter().zip(&r.edges).all(|(a, b)| {
            a.color == b.color
                && a.width == b.width
                && a.bezier.iter().zip(&b.bezier).all(|(p, q)| close(p.0, q.0) && close(p.1, q.1))
        })
}

fn mind_file(st: &mut St, name: &str, src: &str) {
    let d = mind_of(src).unwrap();
    let base = mindmap::scene(&d);
    let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
    let n = at.len();
    if n == 0 {
        return;
    }
    st.note("mind.files");
    let ends = mindmap::link_ends(&d);
    st.check(ends.len() == base.edges.len(), "mind.link_ends_len", name);
    for (j, &(p, c)) in ends.iter().enumerate() {
        let e = &base.edges[j];
        let ok = close(e.bezier[0].0, at[p].0) && close(e.bezier[0].1, at[p].1) && close(e.bezier[3].0, at[c].0) && close(e.bezier[3].1, at[c].1);
        st.check(ok, "mind.link_ends_match_scene", &format!("{name} {j}"));
    }
    st.check(dbg(&mindmap::route_partial(&d, &at, &base, &at)) == dbg(&base), "mind.no_move_is_base", name);
    for i in sample(n) {
        for &(dx, dy) in &DELTAS {
            let mut mv = at.clone();
            mv[i].0 += dx;
            mv[i].1 += dy;
            let f = mindmap::route_partial(&d, &mv, &base, &at);
            let r = mindmap::route(&d, &mv);
            st.check(mind_eq_route(&f, &r), "mind.equals_route", &format!("{name} i={i} ({dx},{dy})"));
            for (k, nd) in f.nodes.iter().enumerate() {
                st.check(close(nd.cx(), mv[k].0) && close(nd.cy(), mv[k].1), "mind.verbatim", name);
            }
            for (j, &(p, c)) in ends.iter().enumerate() {
                if p != i && c != i {
                    st.check(dbg(&f.edges[j]) == dbg(&base.edges[j]), "mind.kept_edge_identical", &format!("{name} {j}"));
                }
            }
            for (k, nd) in f.nodes.iter().enumerate() {
                if k != i {
                    st.check(dbg(nd) == dbg(&base.nodes[k]), "mind.kept_node_identical", name);
                }
            }
            let t = mindmap::try_route_partial(&d, &mv, &base, &at);
            st.check(t.map(|t| dbg(&t)) == Some(dbg(&f)), "mind.try_eq_rp", name);
            let mut mv2 = mv.clone();
            mv2[i].0 += 9.0;
            let f2 = mindmap::route_partial(&d, &mv2, &f, &mv);
            st.check(mind_eq_route(&f2, &mindmap::route(&d, &mv2)), "mind.chain_equals_route", name);
        }
    }
    let mut mv = at.clone();
    mv[0].0 += 25.0;
    let full = mindmap::route(&d, &mv);
    if n > 1 {
        st.check(dbg(&mindmap::route_partial(&d, &mv, &base, &at[..n - 1])) == dbg(&full), "mind.fallback_base_count", name);
    }
    let shifted: Vec<(f64, f64)> = at.iter().map(|p| (p.0 + 5.0, p.1)).collect();
    st.check(dbg(&mindmap::route_partial(&d, &mv, &base, &shifted)) == dbg(&full), "mind.fallback_base_not_at", name);
    for k in 0..n + 6 {
        let c: Vec<(f64, f64)> = (0..k).map(|t| if t < n { mv[t] } else { (5.0, 5.0) }).collect();
        let a = quiet(|| dbg(&mindmap::route(&d, &c)));
        let b = quiet(|| dbg(&mindmap::route_partial(&d, &c, &base, &at)));
        st.check(a.is_ok() && b.is_ok(), "mind.count_no_panic", &format!("{name} k={k}"));
        if let (Ok(x), Ok(y)) = (&a, &b) {
            st.check(x == y || k == n, "mind.count_like_route", &format!("{name} k={k}"));
        }
        let t = quiet(|| mindmap::try_route_partial(&d, &c, &base, &at).is_some());
        st.check(t == Ok(k == n), "mind.try_count", name);
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 2e6] {
        let mut c = mv.clone();
        c[n / 2].0 = bad;
        let t = quiet(|| mindmap::try_route_partial(&d, &c, &base, &at).is_none());
        st.check(t == Ok(true), "mind.try_nonfinite_none", name);
        let a = quiet(|| mindmap::route(&d, &c));
        let b = quiet(|| mindmap::route_partial(&d, &c, &base, &at));
        st.check(a.is_ok() && b.is_ok(), "mind.nonfinite_no_panic", name);
        if let (Ok(x), Ok(y)) = (&a, &b) {
            st.check(dbg(x) == dbg(y), "mind.nonfinite_like_route", &format!("{name} {bad}"));
        }
    }
    // edits
    let mut edits: Vec<(&str, Mindmap)> = Vec::new();
    let mut x = mind_of(src).unwrap();
    x.nodes[n - 1].text = "zz a much longer edited text".into();
    edits.push(("text", x));
    if let Some(x) = mind_of(&format!("{}\n      zzqnewleaf\n", src.trim_end())) {
        if x.nodes.len() == n + 1 {
            edits.push(("add-node", x));
        }
    }
    let leaf = (1..n).rev().find(|&k| d.nodes[k].children.is_empty());
    if let Some(l) = leaf {
        if l == n - 1 {
            let mut x = mind_of(src).unwrap();
            let p = x.nodes[l].parent.unwrap();
            x.nodes[p].children.retain(|&c| c != l);
            x.nodes.pop();
            edits.push(("remove-leaf", x));
        }
        // re-parent the leaf under another node of the same depth's parent
        let mut x = mind_of(src).unwrap();
        let p = x.nodes[l].parent.unwrap();
        if let Some(q) = (0..n).find(|&q| q != p && q != l && x.nodes[q].depth + 1 == x.nodes[l].depth) {
            x.nodes[p].children.retain(|&c| c != l);
            x.nodes[q].children.push(l);
            x.nodes[l].parent = Some(q);
            edits.push(("reparent", x));
        }
    }
    for (what, d2) in &edits {
        let n2 = d2.nodes.len();
        let mut mv: Vec<(f64, f64)> = (0..n2).map(|k| if k < n { at[k] } else { (30.0, 30.0) }).collect();
        mv[0].0 += 10.0;
        let full = mindmap::route(d2, &mv);
        let rp = mindmap::route_partial(d2, &mv, &base, &at);
        st.check(mind_eq_route(&rp, &full), "mind.edit_equals_route", &format!("{name} {what}"));
        st.check(dbg(&rp) == dbg(&full), "mind.edit_equals_route_exact", &format!("{name} {what}"));
    }
}

fn empties(st: &mut St) {
    let ed = flowmaid::model::ErDiagram::default();
    let es = er::scene(&ed);
    let r = quiet(|| er::route_partial(&ed, &[], &es, &[]));
    st.check(r.is_ok(), "empty.er_no_panic", "");
    let r = quiet(|| er::try_route_partial(&ed, &[], &es, &[]).is_some());
    st.check(r == Ok(true), "empty.er_try_some", &format!("{r:?}"));
    let r = quiet(|| er::try_route_partial(&ed, &[(1.0, 1.0)], &es, &[]).is_none());
    st.check(r == Ok(true), "empty.er_try_count", "");
    let cd = flowmaid::model::ClassDiagram::default();
    let cs = class::scene(&cd);
    let r = quiet(|| class::route_partial(&cd, &[], &cs, &[]));
    st.check(r.is_ok(), "empty.class_no_panic", "");
    let r = quiet(|| class::try_route_partial(&cd, &[], &cs, &[]).is_some());
    st.check(r == Ok(true), "empty.class_try_some", &format!("{r:?}"));
    let md = Mindmap::default();
    let ms = mindmap::route(&md, &[]);
    let r = quiet(|| mindmap::route_partial(&md, &[], &ms, &[]));
    st.check(r.is_ok(), "empty.mind_no_panic", "");
    let r = quiet(|| mindmap::try_route_partial(&md, &[], &ms, &[]).is_some());
    st.check(r == Ok(true), "empty.mind_try_some", &format!("{r:?}"));
    let r = quiet(|| mindmap::try_route_partial(&md, &[(1.0, 1.0)], &ms, &[]).is_none());
    st.check(r == Ok(true), "empty.mind_try_count", "");
    // empty parsed diagrams
    for src in ["erDiagram\n", "classDiagram\n"] {
        match parse_document(src) {
            Ok(Document::Er(d)) => {
                let s = er::scene(&d);
                st.check(quiet(|| er::route_partial(&d, &[], &s, &[])).is_ok(), "empty.er_parsed", "");
                st.check(quiet(|| er::route_partial(&d, &[(1.0, 2.0)], &s, &[])).is_ok() == quiet(|| er::route(&d, &[(1.0, 2.0)])).is_ok(), "empty.er_parsed_count", "");
            }
            Ok(Document::Class(d)) => {
                let s = class::scene(&d);
                st.check(quiet(|| class::route_partial(&d, &[], &s, &[])).is_ok(), "empty.class_parsed", "");
            }
            other => st.note(&format!("empty parse {src:?}: {}", other.is_ok())),
        }
    }
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let list = std::env::args().nth(1).expect("list");
    let only = std::env::args().nth(2);
    let mut st = St::default();
    empties(&mut st);
    for path in std::fs::read_to_string(&list).unwrap().lines() {
        let Ok(src) = std::fs::read_to_string(path) else { continue };
        let name = path.rsplit('/').take(2).collect::<Vec<_>>().join("<");
        let doc = quiet(|| parse_document(&src));
        let kind = match &doc {
            Ok(Ok(Document::Er(_))) => "er",
            Ok(Ok(Document::Class(_))) => "class",
            Ok(Ok(Document::Mindmap(_))) => "mind",
            _ => continue,
        };
        if let Some(o) = &only {
            if o != kind {
                continue;
            }
        }
        let t = std::time::Instant::now();
        let r = quiet(|| {
            let mut local = St::default();
            match kind {
                "er" => er_file(&mut local, &name, &src),
                "class" => class_file(&mut local, &name, &src),
                _ => mind_file(&mut local, &name, &src),
            }
            local
        });
        match r {
            Ok(local) => {
                st.checks += local.checks;
                for (k, v) in local.per {
                    let e = st.per.entry(k).or_default();
                    e.0 += v.0;
                    e.1 += v.1;
                }
                for (k, v) in local.notes {
                    *st.notes.entry(k).or_default() += v;
                }
                for f in local.fails {
                    if st.fails.len() < 400 {
                        st.fails.push(f);
                    }
                }
            }
            Err(()) => st.check(false, "PANIC_UNEXPECTED", &format!("{kind} {name}")),
        }
        let el = t.elapsed().as_secs_f64();
        if el > 20.0 {
            eprintln!("slow {name} {el:.1}s");
        }
    }
    let mut keys: Vec<_> = st.per.iter().collect();
    keys.sort();
    for (k, (c, f)) in keys {
        println!("{k:50} checks={c:7} fails={f}");
    }
    let mut notes: Vec<_> = st.notes.iter().collect();
    notes.sort();
    for (k, v) in notes {
        println!("note {k}: {v}");
    }
    println!("TOTAL checks={} fails={}", st.checks, st.per.values().map(|v| v.1).sum::<usize>());
    for f in &st.fails {
        println!("{f}");
    }
}
