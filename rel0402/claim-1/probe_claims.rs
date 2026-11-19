//! Claim probe for 0.40.2 (head only). Usage: probe_claims <list-file>
use flowmaid::model::Document;
use flowmaid::{class, er, journey, mindmap, parser, pie, sankey, scene, seq, timeline};
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Default)]
struct Tally {
    n: BTreeMap<String, (usize, usize, Vec<String>)>,
}
impl Tally {
    fn check(&mut self, key: &str, ok: bool, ctx: impl FnOnce() -> String) {
        let e = self.n.entry(key.to_string()).or_default();
        e.0 += 1;
        if !ok {
            e.1 += 1;
            if e.2.len() < 4 {
                e.2.push(ctx());
            }
        }
    }
    fn print(&self) {
        for (k, (n, bad, ex)) in &self.n {
            println!("{k:<48} checks={n:<7} fails={bad}");
            for e in ex {
                println!("      e.g. {e}");
            }
        }
    }
}

fn edge_eq(a: &scene::SceneEdge, b: &scene::SceneEdge) -> bool {
    a.bezier == b.bezier && a.waypoints == b.waypoints && format!("{:?}", a.label) == format!("{:?}", b.label)
}

/// Bounds of what a scene draws: node boxes, waypoints, bezier ends, labels.
fn drawn(sc: &scene::Scene) -> (f64, f64, f64, f64) {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut add = |x: f64, y: f64| {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    };
    for n in &sc.nodes {
        add(n.x - n.w / 2.0, n.y - n.h / 2.0);
        add(n.x + n.w / 2.0, n.y + n.h / 2.0);
    }
    for e in &sc.edges {
        for p in &e.waypoints {
            add(p.0, p.1);
        }
        add(e.bezier[0].0, e.bezier[0].1);
        add(e.bezier[3].0, e.bezier[3].1);
        if let Some((_, c, w)) = &e.label {
            add(c.0 - w / 2.0, c.1 - 8.0);
            add(c.0 + w / 2.0, c.1 + 8.0);
        }
    }
    (x0, y0, x1, y1)
}

fn holds(sc: &scene::Scene) -> Option<String> {
    let (x0, y0, x1, y1) = drawn(sc);
    let (ox, oy) = sc.origin;
    let eps = 0.5;
    if x0 < ox - eps || y0 < oy - eps || x1 > ox + sc.width + eps || y1 > oy + sc.height + eps {
        Some(format!(
            "drawn ({x0:.1},{y0:.1})-({x1:.1},{y1:.1}) canvas ({ox:.1},{oy:.1})-({:.1},{:.1})",
            ox + sc.width,
            oy + sc.height
        ))
    } else {
        None
    }
}

fn contains(outer: &scene::Scene, inner: &scene::Scene) -> bool {
    let e = 1e-9;
    outer.origin.0 <= inner.origin.0 + e
        && outer.origin.1 <= inner.origin.1 + e
        && outer.origin.0 + outer.width >= inner.origin.0 + inner.width - e
        && outer.origin.1 + outer.height >= inner.origin.1 + inner.height - e
}

fn er_checks(t: &mut Tally, f: &str, src: &str, d: &flowmaid::model::ErDiagram) {
    let n = d.entities.len();
    if n < 2 || n > 60 {
        return;
    }
    let base = er::scene(d);
    let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let picks = [n / 2, 0, n - 1];
    let offs = [(37.0, 23.0), (-2000.0, -1500.0), (900.0, 700.0)];
    for &i in &picks {
        for &(dx, dy) in &offs {
            let mut c = at.clone();
            c[i] = (at[i].0 + dx, at[i].1 + dy);
            let fr = er::route_partial(d, &c, &base, &at);
            for (j, r) in d.relations.iter().enumerate() {
                if r.from != i && r.to != i {
                    t.check("er: kept line == base (bezier/waypoints/label)", edge_eq(&fr.scene.edges[j], &base.scene.edges[j]), || format!("{f} node {i} edge {j}"));
                    let g1 = format!("{:?}", er::end_glyphs(&fr.scene.edges[j], fr.cards[j]));
                    let g0 = format!("{:?}", er::end_glyphs(&base.scene.edges[j], base.cards[j]));
                    t.check("er: kept line end glyphs == base", g1 == g0, || format!("{f} {j}"));
                }
            }
            let pos_ok = fr.scene.nodes.iter().zip(&c).all(|(n, p)| n.x == p.0 && n.y == p.1);
            t.check("er: positions kept as given", pos_ok, || format!("{f} {i}"));
            let h = holds(&fr.scene);
            t.check("er: frame canvas holds drawing", h.is_none(), || format!("{f} {i} {dx} {:?}", h));
            t.check("er: frame canvas contains base canvas", contains(&fr.scene, &base.scene), || format!("{f} {i} {dx}"));
            let tr = er::try_route_partial(d, &c, &base, &at).map(|s| er::to_svg(&s));
            t.check("er: try_route_partial Some == route_partial", tr.as_deref() == Some(er::to_svg(&fr).as_str()), || f.to_string());
        }
    }
    // Chain: one box far up-left, then another far right/down, chained.
    {
        let mut c1 = at.clone();
        c1[0] = (at[0].0 - 800.0, at[0].1 - 600.0);
        let f1 = er::route_partial(d, &c1, &base, &at);
        let mut c2 = c1.clone();
        c2[n - 1] = (at[n - 1].0 + 1500.0, at[n - 1].1 + 1100.0);
        let f2 = er::route_partial(d, &c2, &f1, &c1);
        let h1 = holds(&f1.scene);
        let h2 = holds(&f2.scene);
        t.check("er: chain frame1 holds", h1.is_none(), || format!("{f} {:?}", h1));
        t.check("er: chain frame2 holds", h2.is_none(), || format!("{f} {:?}", h2));
        t.check("er: chain frame2 contains frame1 canvas", contains(&f2.scene, &f1.scene), || f.to_string());
        let f2b = er::route_partial(d, &c2, &base, &at);
        let h3 = holds(&f2b.scene);
        t.check("er: two-moved-from-base holds", h3.is_none(), || format!("{f} {:?}", h3));
        let r2 = er::route(d, &c2);
        let hr = holds(&r2.scene);
        t.check("er: (info) route canvas holds", hr.is_none(), || format!("{f} {:?}", hr));
    }
    // Mismatch: other counts, edit since, base not at base_centers.
    let mut c = at.clone();
    c[0].0 += 50.0;
    let want = er::to_svg(&er::route(d, &c));
    let fr = er::route_partial(d, &c, &base, &at[..n - 1]);
    t.check("er: base_centers other count -> route", er::to_svg(&fr) == want, || f.to_string());
    let mut off = at.clone();
    off[1].1 += 30.0;
    let fr = er::route_partial(d, &c, &base, &off);
    t.check("er: base not drawn at base_centers -> route", er::to_svg(&fr) == want, || f.to_string());
    // Edits: add a relationship, change a label.
    let edits = [
        format!("{src}\nZZnew ||--o{{ {} : addedrel\n", d.entities[0].name),
        src.replacen(":", ": x", 1),
    ];
    for (k, e) in edits.iter().enumerate() {
        if let Ok(Document::Er(d2)) = parser::parse_document(e) {
            let base2 = er::scene(&d2);
            let mut c2: Vec<(f64, f64)> = base2.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            // keep the old positions where the entity existed
            for (ii, ent) in d2.entities.iter().enumerate() {
                if let Some(o) = d.entities.iter().position(|x| x.name == ent.name) {
                    c2[ii] = at[o];
                }
            }
            c2[0].0 += 20.0;
            let want = er::to_svg(&er::route(&d2, &c2));
            let got = catch_unwind(AssertUnwindSafe(|| er::to_svg(&er::route_partial(&d2, &c2, &base, &at))));
            let changed = d2.entities.len() != d.entities.len() || d2.relations.len() != d.relations.len() || format!("{:?}", d2.relations) != format!("{:?}", d.relations);
            if changed {
                t.check(&format!("er: edit {k} -> route"), got.as_deref().ok() == Some(want.as_str()), || f.to_string());
            }
        }
    }
    // try_route_partial None cases
    let mut bad = at.clone();
    bad[0].0 = f64::NAN;
    t.check("er: try_rp None for NaN", er::try_route_partial(d, &bad, &base, &at).is_none(), || f.into());
    bad[0].0 = f64::INFINITY;
    t.check("er: try_rp None for inf", er::try_route_partial(d, &bad, &base, &at).is_none(), || f.into());
    bad[0].0 = 2e6;
    t.check("er: try_rp None for 2e6", er::try_route_partial(d, &bad, &base, &at).is_none(), || f.into());
    t.check("er: try_rp None for short", er::try_route_partial(d, &at[..n - 1], &base, &at).is_none(), || f.into());
    let panics = catch_unwind(AssertUnwindSafe(|| er::route_partial(d, &at[..n - 1], &base, &at))).is_err();
    t.check("er: route_partial panics on short centers", panics, || f.into());
    let mut nb = at.clone();
    nb[0].0 = f64::NAN;
    let ok = catch_unwind(AssertUnwindSafe(|| er::try_route_partial(d, &c, &base, &nb))).is_ok();
    t.check("er: try_rp no panic on NaN base_centers", ok, || f.into());
}

fn class_checks(t: &mut Tally, f: &str, src: &str, d: &flowmaid::model::ClassDiagram) {
    let n = d.classes.len();
    if n < 2 || n > 40 {
        return;
    }
    let base = class::scene(d);
    let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let at_cls = &at[..n];
    let spots0 = class::card_spots(&base);
    let picks = [n / 2, 0];
    let offs = [(37.0, 23.0), (-2000.0, -1500.0), (900.0, 700.0)];
    for (mode, base_c) in [("all", &at[..]), ("classes", at_cls)] {
        for &i in &picks {
            for &(dx, dy) in &offs {
                let mut c = base_c.to_vec();
                c[i] = (base_c[i].0 + dx, base_c[i].1 + dy);
                let fr = class::route_partial(d, &c, &base, base_c);
                // edges: relations + note links; ends via scene ids
                let moved_id = &base.scene.nodes[i].id;
                let spots1 = class::card_spots(&fr);
                for j in 0..base.scene.edges.len() {
                    let e = &base.scene.edges[j];
                    if &e.from != moved_id && &e.to != moved_id {
                        t.check(&format!("class[{mode}]: kept line == base"), edge_eq(&fr.scene.edges[j], e), || format!("{f} node {i} edge {j}"));
                        if j < d.relations.len() {
                            t.check(&format!("class[{mode}]: kept cardinalities == base"), spots1.get(j) == spots0.get(j), || format!("{f} {j}"));
                            let hh = format!("{:?}{:?}", class::rel_head(&fr, j), class::rel_tail(&fr, j)) == format!("{:?}{:?}", class::rel_head(&base, j), class::rel_tail(&base, j));
                            t.check(&format!("class[{mode}]: kept end glyphs == base"), hh, || format!("{f} {j}"));
                        }
                    }
                }
                let pos_ok = fr.scene.nodes.iter().zip(&c).all(|(n, p)| n.x == p.0 && n.y == p.1);
                t.check(&format!("class[{mode}]: positions kept as given"), pos_ok, || format!("{f} {i}"));
                let h = holds(&fr.scene);
                t.check("class: frame canvas holds drawing", h.is_none(), || format!("{f} {i} {dx} {:?}", h));
                t.check("class: frame canvas contains base canvas", contains(&fr.scene, &base.scene), || format!("{f} {i} {dx}"));
                let tr = class::try_route_partial(d, &c, &base, base_c).map(|s| class::to_svg(&s));
                t.check("class: try_rp Some == route_partial", tr.as_deref() == Some(class::to_svg(&fr).as_str()), || f.to_string());
            }
        }
    }
    {
        let mut c1 = at.clone();
        c1[0] = (at[0].0 - 800.0, at[0].1 - 600.0);
        let f1 = class::route_partial(d, &c1, &base, &at);
        let mut c2 = c1.clone();
        c2[n - 1] = (at[n - 1].0 + 1500.0, at[n - 1].1 + 1100.0);
        let f2 = class::route_partial(d, &c2, &f1, &c1);
        let h1 = holds(&f1.scene);
        let h2 = holds(&f2.scene);
        t.check("class: chain frame1 holds", h1.is_none(), || format!("{f} {:?}", h1));
        t.check("class: chain frame2 holds", h2.is_none(), || format!("{f} {:?}", h2));
        t.check("class: chain frame2 contains frame1 canvas", contains(&f2.scene, &f1.scene), || f.to_string());
        let r2 = class::route(d, &c2);
        let hr = holds(&r2.scene);
        t.check("class: (info) route canvas holds", hr.is_none(), || format!("{f} {:?}", hr));
    }
    let mut c = at.clone();
    c[0].0 += 50.0;
    let want = class::to_svg(&class::route(d, &c));
    let fr = class::route_partial(d, &c, &base, &at[..n.saturating_sub(1)]);
    t.check("class: base_centers other count -> route", class::to_svg(&fr) == want, || f.to_string());
    let mut off = at.clone();
    off[1].1 += 30.0;
    let fr = class::route_partial(d, &c, &base, &off);
    t.check("class: base not drawn at base_centers -> route", class::to_svg(&fr) == want, || f.to_string());
    let edits = [
        format!("{src}\n{} <|-- ZZnew\n", d.classes[0].name),
        format!("{src}\nclass {} {{\n  +int addedMember\n}}\n", d.classes[n - 1].name),
    ];
    for (k, e) in edits.iter().enumerate() {
        if let Ok(Document::Class(d2)) = parser::parse_document(e) {
            let base2 = class::scene(&d2);
            let mut c2: Vec<(f64, f64)> = base2.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            for (ii, cl) in d2.classes.iter().enumerate() {
                if let Some(o) = d.classes.iter().position(|x| x.name == cl.name) {
                    c2[ii] = at[o];
                }
            }
            c2[0].0 += 20.0;
            let c2 = &c2[..d2.classes.len()];
            let want = class::to_svg(&class::route(&d2, c2));
            let got = catch_unwind(AssertUnwindSafe(|| class::to_svg(&class::route_partial(&d2, c2, &base, at_cls))));
            t.check(&format!("class: edit {k} -> route"), got.as_deref().ok() == Some(want.as_str()), || f.to_string());
        }
    }
    let mut bad = at.clone();
    bad[0].0 = f64::NAN;
    t.check("class: try_rp None for NaN", class::try_route_partial(d, &bad, &base, &at).is_none(), || f.into());
    bad[0].0 = -2e6;
    t.check("class: try_rp None for -2e6", class::try_route_partial(d, &bad, &base, &at).is_none(), || f.into());
    t.check("class: try_rp None for short", class::try_route_partial(d, &at[..n - 1], &base, &at).is_none(), || f.into());
    let panics = catch_unwind(AssertUnwindSafe(|| class::route_partial(d, &at[..n - 1], &base, &at))).is_err();
    t.check("class: route_partial panics on short centers", panics, || f.into());
}

fn mind_checks(t: &mut Tally, f: &str, src: &str, d: &flowmaid::model::Mindmap) {
    let n = d.nodes.len();
    if n < 2 {
        return;
    }
    let base = mindmap::scene(d);
    let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
    let ends = mindmap::link_ends(d);
    t.check("mind: link_ends len == edges", ends.len() == base.edges.len(), || f.into());
    for (j, &(p, c)) in ends.iter().enumerate() {
        let e = &base.edges[j];
        let ok = (e.bezier[0].0 - at[p].0).abs() < 1e-9 && (e.bezier[0].1 - at[p].1).abs() < 1e-9 && (e.bezier[3].0 - at[c].0).abs() < 1e-9 && (e.bezier[3].1 - at[c].1).abs() < 1e-9;
        t.check("mind: link_ends names connector's nodes", ok, || format!("{f} {j}"));
    }
    for &i in &[n / 2, 0, n - 1] {
        for &(dx, dy) in &[(37.0, 23.0), (-3000.0, -2000.0), (900.0, 700.0)] {
            let mut c = at.clone();
            c[i] = (at[i].0 + dx, at[i].1 + dy);
            let fr = mindmap::route_partial(d, &c, &base, &at);
            let r = mindmap::route(d, &c);
            for (j, &(p, ch)) in ends.iter().enumerate() {
                if p != i && ch != i {
                    t.check("mind: kept connector == base", fr.edges[j].bezier == base.edges[j].bezier, || format!("{f} {j}"));
                }
            }
            t.check("mind: frame svg == route svg", mindmap::to_svg(&fr) == mindmap::to_svg(&r), || format!("{f} {i} {dx}"));
            t.check("mind: frame canvas == route canvas", fr.width == r.width && fr.height == r.height, || format!("{f} {i} {dx} {} {} vs {} {}", fr.width, fr.height, r.width, r.height));
            let pos = fr.nodes.iter().zip(&c).all(|(nb, p)| (nb.cx() - p.0).abs() < 1e-9 && (nb.cy() - p.1).abs() < 1e-9);
            t.check("mind: positions kept as given", pos, || format!("{f} {i}"));
            let tr = mindmap::try_route_partial(d, &c, &base, &at).map(|s| mindmap::to_svg(&s));
            t.check("mind: try_rp Some == route_partial", tr.as_deref() == Some(mindmap::to_svg(&fr).as_str()), || f.into());
        }
    }
    // canvas does not grow up/left: a node dragged to negative coords
    let mut c = at.clone();
    c[n - 1] = (-500.0, -400.0);
    let fr = mindmap::route_partial(d, &c, &base, &at);
    let maxx = fr.nodes.iter().map(|n| n.x + n.w).fold(0.0, f64::max);
    t.check("mind: canvas not grown up/left (width = max x + margin)", fr.width >= maxx && fr.width < maxx + 100.0, || f.into());
    // edits
    let edits = [format!("{src}\n      zzadded\n"), src.replacen("root", "rootx", 1)];
    for (k, e) in edits.iter().enumerate() {
        if let Ok(Document::Mindmap(d2)) = parser::parse_document(e) {
            if d2.nodes.len() == n && e == src {
                continue;
            }
            let base2 = mindmap::scene(&d2);
            let mut c2: Vec<(f64, f64)> = base2.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
            for ii in 0..c2.len().min(n) {
                c2[ii] = at[ii];
            }
            c2[0].0 += 20.0;
            let want = mindmap::to_svg(&mindmap::route(&d2, &c2));
            let got = mindmap::to_svg(&mindmap::route_partial(&d2, &c2, &base, &at));
            t.check(&format!("mind: edit {k} -> route"), got == want, || f.into());
        }
    }
    let mut bad = at.clone();
    bad[0].1 = f64::NAN;
    t.check("mind: try_rp None NaN", mindmap::try_route_partial(d, &bad, &base, &at).is_none(), || f.into());
    bad[0].1 = 1.5e6;
    t.check("mind: try_rp None 1.5e6", mindmap::try_route_partial(d, &bad, &base, &at).is_none(), || f.into());
    t.check("mind: try_rp None short", mindmap::try_route_partial(d, &at[..n - 1], &base, &at).is_none(), || f.into());
    t.check("mind: route_partial short == route (empty)", mindmap::route_partial(d, &at[..n - 1], &base, &at).nodes.is_empty(), || f.into());
}

// ---------------- picking ----------------

fn grid(w: f64, h: f64, k: usize) -> Vec<(f64, f64)> {
    let mut v = Vec::new();
    for i in 0..=k {
        for j in 0..=k {
            v.push((w * i as f64 / k as f64, h * j as f64 / k as f64));
        }
    }
    v
}

fn off_canvas(w: f64, h: f64) -> Vec<(f64, f64)> {
    vec![(-0.5, h / 2.0), (w + 0.5, h / 2.0), (w / 2.0, -0.5), (w / 2.0, h + 0.5), (f64::NAN, 1.0), (1.0, f64::NAN), (-100.0, -100.0)]
}

fn pick_common<H: PartialEq + std::fmt::Debug>(t: &mut Tally, kind: &str, f: &str, w: f64, h: f64, hit: &dyn Fn(f64, f64, f64) -> Option<H>, lines: bool) {
    for p in grid(w, h, 40) {
        let a = hit(p.0, p.1, 0.0);
        let b = hit(p.0, p.1, -3.0);
        let c = hit(p.0, p.1, f64::NEG_INFINITY);
        t.check(&format!("{kind}: tol<=0 all equal (0,-3,-inf)"), a == b && b == c, || format!("{f} {p:?} {a:?} {b:?} {c:?}"));
        if !lines {
            let d = hit(p.0, p.1, 50.0);
            t.check(&format!("{kind}: no lines, tol ignored"), a == d, || format!("{f} {p:?} {a:?} {d:?}"));
        }
        // a tolerance never removes a hit that is on the ink
        if a.is_some() {
            let d = hit(p.0, p.1, 5.0);
            t.check(&format!("{kind}: (info) hit at tol 0 still a hit at tol 5"), d.is_some(), || format!("{f} {p:?}"));
        }
    }
    for p in off_canvas(w, h) {
        t.check(&format!("{kind}: off canvas -> None"), hit(p.0, p.1, 10.0).is_none(), || format!("{f} {p:?}"));
    }
}

fn mind_pick(t: &mut Tally, f: &str, ms: &mindmap::MindScene) {
    use mindmap::MindHit;
    pick_common(t, "mindpick", f, ms.width, ms.height, &|x, y, tol| ms.hit_test(x, y, tol), true);
    for (i, n) in ms.nodes.iter().enumerate() {
        let h = ms.hit_test(n.cx(), n.cy(), 0.0);
        let ok = matches!(h, Some(MindHit::Node(k)) if k >= i);
        t.check("mindpick: node centre -> that node (or a later node)", ok, || format!("{f} {i} {h:?}"));
        // just outside the box on the left, at its centre row, tol 10: never this node
        let h2 = ms.hit_test(n.x - 30.0, n.cy(), 10.0);
        t.check("mindpick: 30px left of box, tol 10 -> not that node", h2 != Some(MindHit::Node(i)), || format!("{f} {i} {h2:?}"));
    }
    for (j, e) in ms.edges.iter().enumerate() {
        let b = e.bezier;
        let m = ((b[0].0 + b[3].0) / 2.0, (b[0].1 + b[3].1) / 2.0);
        let len = ((b[3].0 - b[0].0).powi(2) + (b[3].1 - b[0].1).powi(2)).sqrt();
        if len < 1.0 {
            continue;
        }
        let nrm = (-(b[3].1 - b[0].1) / len, (b[3].0 - b[0].0) / len);
        let h = ms.hit_test(m.0, m.1, 0.0);
        let on_node = matches!(h, Some(MindHit::Node(_)));
        if !on_node {
            t.check("mindpick: connector midpoint -> that connector", matches!(h, Some(MindHit::Link(_))), || format!("{f} {j} {h:?}"));
            if h == Some(MindHit::Link(j)) {
                let d = e.width / 2.0 + 2.0;
                let q = (m.0 + nrm.0 * d, m.1 + nrm.1 * d);
                let h0 = ms.hit_test(q.0, q.1, 0.0);
                let h3 = ms.hit_test(q.0, q.1, 3.0);
                if !matches!(h3, Some(MindHit::Node(_))) && !matches!(h0, Some(MindHit::Node(_))) {
                    t.check("mindpick: 2px off stroke, tol 0 -> not it", h0 != Some(MindHit::Link(j)), || format!("{f} {j} {h0:?}"));
                    t.check("mindpick: 2px off stroke, tol 3 -> a connector", matches!(h3, Some(MindHit::Link(_))), || format!("{f} {j} {h3:?}"));
                }
            }
        }
    }
}

fn seq_pick(t: &mut Tally, f: &str, sc: &seq::SeqScene) {
    use seq::SeqHit;
    pick_common(t, "seqpick", f, sc.width, sc.height, &|x, y, tol| sc.hit_test(x, y, tol), true);
    for (i, b) in sc.boxes.iter().enumerate() {
        let h = sc.hit_test(b.x + b.w / 2.0, b.y + b.h / 2.0, 0.0);
        t.check("seqpick: participant box centre", matches!(h, Some(SeqHit::Participant(k)) if k == i) || matches!(h, Some(SeqHit::Footer(_))), || format!("{f} {i} {h:?}"));
    }
    for (i, b) in sc.footer.iter().enumerate() {
        let h = sc.hit_test(b.x + b.w / 2.0, b.y + b.h / 2.0, 0.0);
        t.check("seqpick: footer box centre", h == Some(SeqHit::Footer(i)), || format!("{f} {i} {h:?}"));
    }
    for (i, m) in sc.messages.iter().enumerate() {
        if m.points.len() == 2 && (m.points[0].1 - m.points[1].1).abs() < 1e-9 && (m.points[0].0 - m.points[1].0).abs() > 40.0 {
            let x = (m.points[0].0 + m.points[1].0) / 2.0;
            let y = m.points[0].1;
            let h = sc.hit_test(x, y, 0.0);
            t.check("seqpick: message midpoint -> a message/participant", matches!(h, Some(SeqHit::Message(_)) | Some(SeqHit::Participant(_)) | Some(SeqHit::Footer(_))), || format!("{f} {i} {h:?}"));
            // below the line by 0.8 + 2 px (labels sit above)
            let q = (x, y + 2.8);
            let h0 = sc.hit_test(q.0, q.1, 0.0);
            let h3 = sc.hit_test(q.0, q.1, 3.0);
            if h == Some(SeqHit::Message(i)) {
                t.check("seqpick: 2px below msg, tol 0 -> not that msg", h0 != Some(SeqHit::Message(i)), || format!("{f} {i} {h0:?}"));
                if !matches!(h0, Some(SeqHit::Participant(_)) | Some(SeqHit::Footer(_)) | Some(SeqHit::Note(_)) | Some(SeqHit::Message(_))) {
                    t.check("seqpick: 2px below msg, tol 3 -> a message", matches!(h3, Some(SeqHit::Message(_))), || format!("{f} {i} {h3:?} {h0:?}"));
                }
            }
        }
    }
    for (i, n) in sc.notes.iter().enumerate() {
        let h = sc.hit_test(n.x + n.w / 2.0, n.y + n.h / 2.0, 0.0);
        t.check("seqpick: note centre -> note/message/box", matches!(h, Some(SeqHit::Note(_)) | Some(SeqHit::Message(_)) | Some(SeqHit::Participant(_)) | Some(SeqHit::Footer(_))), || format!("{f} {i} {h:?}"));
    }
    for (i, l) in sc.lifelines.iter().enumerate() {
        let y = (l.y0 + l.y1) / 2.0;
        let h = sc.hit_test(l.x, y, 0.0);
        let h_off0 = sc.hit_test(l.x + 2.5, y, 0.0);
        let h_off3 = sc.hit_test(l.x + 2.5, y, 3.0);
        if h == Some(SeqHit::Lifeline(i)) {
            t.check("seqpick: 2px off lifeline tol 0 -> not lifeline", h_off0 != Some(SeqHit::Lifeline(i)), || format!("{f} {i} {h_off0:?}"));
            if h_off0.is_none() || matches!(h_off0, Some(SeqHit::FrameInside(_))) {
                t.check("seqpick: 2px off lifeline tol 3 -> lifeline", h_off3 == Some(SeqHit::Lifeline(i)), || format!("{f} {i} {h_off3:?}"));
            }
        }
        t.check("seqpick: (info) lifeline midpoint -> lifeline", h == Some(SeqHit::Lifeline(i)), || format!("{f} {i} {h:?}"));
    }
    for (i, a) in sc.activations.iter().enumerate() {
        let h = sc.hit_test(a.x, (a.y0 + a.y1) / 2.0, 0.0);
        t.check("seqpick: (info) activation centre -> activation", matches!(h, Some(SeqHit::Activation(_))), || format!("{f} {i} {h:?}"));
    }
    for (i, fr) in sc.frames.iter().enumerate() {
        // left border mid-height
        let h = sc.hit_test(fr.x, fr.y + fr.h / 2.0, 0.0);
        t.check("seqpick: (info) frame left border -> frame", h == Some(SeqHit::Frame(i)) || matches!(h, Some(SeqHit::Frame(_))), || format!("{f} {i} {h:?}"));
        // inside near left border, 6px in, where nothing drawn
        let q = (fr.x + 6.0, fr.y + fr.h / 2.0);
        let h2 = sc.hit_test(q.0, q.1, 0.0);
        t.check("seqpick: (info) inside frame near border -> FrameInside", matches!(h2, Some(SeqHit::FrameInside(_))), || format!("{f} {i} {h2:?}"));
        t.check("seqpick: inside frame never Frame(i) border hit at 6px in", h2 != Some(SeqHit::Frame(i)) || fr.dividers.iter().any(|(dy, _)| (dy - q.1).abs() < 3.0), || format!("{f} {i} {h2:?}"));
    }
}

fn pie_pick(t: &mut Tally, f: &str, ps: &pie::PieScene) {
    use pie::PieHit;
    pick_common(t, "piepick", f, ps.width, ps.height, &|x, y, tol| ps.hit_test(x, y, tol), true);
    for (i, s) in ps.slices.iter().enumerate() {
        if s.frac <= 0.0 || s.frac > 0.999 {
            continue;
        }
        let a = (s.start_angle + s.end_angle) / 2.0;
        for rr in [0.35, 0.6, 0.9] {
            // angle convention: try both sin/cos conventions and see which one hits
            let p1 = (ps.cx + ps.r * rr * a.sin(), ps.cy - ps.r * rr * a.cos());
            let h = ps.hit_test(p1.0, p1.1, 0.0);
            t.check("piepick: point mid-wedge -> slice or a label", matches!(h, Some(PieHit::Slice(k)) if k == i) || matches!(h, Some(PieHit::Label { .. }) | Some(PieHit::Legend(_))), || format!("{f} {i} r{rr} {h:?}"));
        }
    }
    for (i, l) in ps.legend.iter().enumerate() {
        let h = ps.hit_test(l.x + 4.0, l.y, 0.0);
        t.check("piepick: legend swatch -> legend", h == Some(PieHit::Legend(i)), || format!("{f} {i} {h:?}"));
    }
    for (k, l) in ps.labels.iter().enumerate() {
        let h = ps.hit_test(l.x, l.y, 0.0);
        t.check("piepick: (info) label anchor -> label", matches!(h, Some(PieHit::Label { label, slice }) if label == k && slice == l.slice), || format!("{f} {k} {h:?}"));
    }
}

fn journey_pick(t: &mut Tally, f: &str, js: &journey::JourneyScene) {
    use journey::JourneyHit;
    pick_common(t, "journeypick", f, js.width, js.height, &|x, y, tol| js.hit_test(x, y, tol), false);
    for (i, tk) in js.tasks.iter().enumerate() {
        let h = js.hit_test(tk.cx, tk.cy, 0.0);
        t.check("journeypick: face centre -> task", h == Some(JourneyHit::Task(i)), || format!("{f} {i} {h:?}"));
    }
    for (i, s) in js.sections.iter().enumerate() {
        let h = js.hit_test(s.x + 8.0, s.y + s.h / 2.0, 0.0);
        t.check("journeypick: (info) section band left -> section", matches!(h, Some(JourneyHit::Section(k)) if k == i) || matches!(h, Some(JourneyHit::Task(_))), || format!("{f} {i} {h:?}"));
    }
}

fn timeline_pick(t: &mut Tally, f: &str, ts: &timeline::TimelineScene) {
    use timeline::TimelineHit;
    pick_common(t, "timelinepick", f, ts.width, ts.height, &|x, y, tol| ts.hit_test(x, y, tol), true);
    for (s, sec) in ts.sections.iter().enumerate() {
        for (k, p) in sec.periods.iter().enumerate() {
            for (e, (_, y)) in p.events.iter().enumerate() {
                let h = ts.hit_test(p.cx, *y, 0.0);
                t.check("timelinepick: event centre -> event", h == Some(TimelineHit::Event { section: s, period: k, event: e }) || matches!(h, Some(TimelineHit::Event { .. }) | Some(TimelineHit::Period { .. })), || format!("{f} {s} {k} {e} {h:?}"));
            }
            let h = ts.hit_test(p.cx, sec.axis_y, 0.0);
            t.check("timelinepick: tick -> period", h == Some(TimelineHit::Period { section: s, period: k }) || matches!(h, Some(TimelineHit::Period { .. }) | Some(TimelineHit::Event { .. })), || format!("{f} {s} {k} {h:?}"));
        }
        // axis between first two periods, or at left end
        if !sec.periods.is_empty() {
            let x = sec.x + 2.0;
            let h = ts.hit_test(x, sec.axis_y, 0.0);
            let h0 = ts.hit_test(x, sec.axis_y + 3.0, 0.0);
            let h3 = ts.hit_test(x, sec.axis_y + 3.0, 3.0);
            if h == Some(TimelineHit::Section(s)) && h0.is_none() {
                t.check("timelinepick: 2px off axis tol 3 -> section", h3 == Some(TimelineHit::Section(s)), || format!("{f} {s} {h3:?}"));
            }
            t.check("timelinepick: (info) axis left end -> section", h == Some(TimelineHit::Section(s)), || format!("{f} {s} {h:?}"));
        }
    }
}

fn sankey_pick(t: &mut Tally, f: &str, ss: &sankey::SankeyScene) {
    use sankey::SankeyHit;
    pick_common(t, "sankeypick", f, ss.width, ss.height, &|x, y, tol| ss.hit_test(x, y, tol), false);
    for (i, n) in ss.nodes.iter().enumerate() {
        let h = ss.hit_test(n.x + n.w / 2.0, n.y + n.h / 2.0, 0.0);
        t.check("sankeypick: bar centre -> a node", matches!(h, Some(SankeyHit::Node(k)) if k == i) || matches!(h, Some(SankeyHit::Node(_))), || format!("{f} {i} {h:?}"));
    }
    for (j, l) in ss.links.iter().enumerate() {
        let m = ((l.x0 + l.x1) / 2.0, (l.y0 + l.y1) / 2.0);
        let h = ss.hit_test(m.0, m.1, 0.0);
        t.check("sankeypick: (info) ribbon midpoint -> link or node", matches!(h, Some(SankeyHit::Link(_)) | Some(SankeyHit::Node(_))), || format!("{f} {j} {h:?}"));
        if l.thickness > 4.0 {
            let h2 = ss.hit_test(m.0, m.1 + l.thickness / 2.0 + 1.5, 0.0);
            t.check("sankeypick: (info) 1.5px below thick ribbon mid -> not that link", h2 != Some(SankeyHit::Link(j)), || format!("{f} {j} {h2:?}"));
        }
    }
}

fn main() {
    let list = std::env::args().nth(1).expect("list");
    let mut t = Tally::default();
    for f in std::fs::read_to_string(&list).unwrap().lines() {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let r = catch_unwind(AssertUnwindSafe(|| {
            let mut tt = Tally::default();
            match parser::parse_document(&src) {
                Ok(Document::Er(d)) => er_checks(&mut tt, f, &src, &d),
                Ok(Document::Class(d)) => class_checks(&mut tt, f, &src, &d),
                Ok(Document::Mindmap(d)) => {
                    mind_checks(&mut tt, f, &src, &d);
                    mind_pick(&mut tt, f, &mindmap::scene(&d));
                }
                Ok(Document::Sequence(d)) => seq_pick(&mut tt, f, &seq::scene(&d)),
                Ok(Document::Pie(d)) => pie_pick(&mut tt, f, &pie::scene(&d)),
                Ok(Document::Journey(d)) => journey_pick(&mut tt, f, &journey::scene(&d)),
                Ok(Document::Timeline(d)) => timeline_pick(&mut tt, f, &timeline::scene(&d)),
                Ok(Document::Sankey(d)) => sankey_pick(&mut tt, f, &sankey::scene(&d)),
                _ => {}
            }
            tt
        }));
        match r {
            Ok(tt) => {
                for (k, (n, bad, ex)) in tt.n {
                    let e = t.n.entry(k).or_default();
                    e.0 += n;
                    e.1 += bad;
                    for x in ex {
                        if e.2.len() < 4 {
                            e.2.push(x);
                        }
                    }
                }
            }
            Err(_) => t.check("PANIC in probe", false, || f.to_string()),
        }
    }
    t.print();
}
