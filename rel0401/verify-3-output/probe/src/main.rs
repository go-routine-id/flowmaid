// verify-3 probe: links origin/dev (fm_dev) and HEAD (fm_head) side by side.
//   vprobe3 all <listfile> <outdir> [lite]  -> <outdir>/results.tsv (+ diffs/)
//   vprobe3 one <file> <kind> <outprefix>    -> <outprefix>.dev.svg / .head.svg
use std::panic::{catch_unwind, AssertUnwindSafe};

pub type P = (f64, f64);

pub fn plan_nodes(c: &[P], lite: bool) -> Vec<usize> {
    let n = c.len();
    if n == 0 {
        return vec![];
    }
    let mut k: Vec<usize> = if lite {
        vec![0, n / 2, n - 1]
    } else {
        let m = n.min(6);
        (0..m).map(|i| i * (n - 1) / (m.max(2) - 1)).collect()
    };
    // leftmost and topmost node
    let lm = (0..n).min_by(|&a, &b| c[a].0.partial_cmp(&c[b].0).unwrap()).unwrap();
    let tm = (0..n).min_by(|&a, &b| c[a].1.partial_cmp(&c[b].1).unwrap()).unwrap();
    if !lite {
        k.push(lm);
        k.push(tm);
    }
    k.sort();
    k.dedup();
    k
}

macro_rules! gen {
    ($m:ident, $fm:ident) => {
        pub mod $m {
            use super::P;
            use $fm as fm;
            use fm::model::Document;
            pub enum Sc {
                Flow(fm::scene::Scene),
                Class(fm::class::ClassScene),
                Er(fm::er::ErScene),
                No,
            }
            pub struct Out {
                pub kind: String,
                pub svg: Result<String, String>,
                pub sc: Sc,
                pub given: Vec<P>,
                pub chained: bool,
            }
            fn ok(kind: String, svg: String, sc: Sc, given: Vec<P>, chained: bool) -> Out {
                Out { kind, svg: Ok(svg), sc, given, chained }
            }
            fn caught<T>(f: impl FnOnce() -> T) -> Result<T, String> {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|e| {
                    e.downcast_ref::<String>()
                        .cloned()
                        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default()
                        .replace(['\t', '\n'], " ")
                })
            }
            pub fn doc(src: &str) -> Option<Document> {
                fm::parser::parse_document(src).ok()
            }
            pub fn run(src: &str, lite: bool) -> Vec<Out> {
                let mut out = Vec::new();
                let r = caught(|| fm::render_svg(src));
                out.push(Out {
                    kind: "render".into(),
                    svg: match r {
                        Ok(Ok(s)) => Ok(s),
                        Ok(Err(e)) => Err(format!("PARSE line {}: {}", e.line, e.message)),
                        Err(p) => Err(format!("PANIC {p}")),
                    },
                    sc: Sc::No,
                    given: vec![],
                    chained: false,
                });
                let Some(d) = doc(src) else { return out };
                match d {
                    Document::Flowchart(g) | Document::State(g) => {
                        let laid = fm::scene::scene(&g);
                        out.push(ok("scene".into(), fm::scene::to_svg(&laid), Sc::No, vec![], false));
                        let c: Vec<P> = laid.nodes.iter().map(|n| (n.x, n.y)).collect();
                        if c.is_empty() {
                            return out;
                        }
                        let mut push = |kind: String, d: Vec<P>, partial: Option<(&fm::scene::Scene, &[P])>, chained: bool| {
                            let res = caught(|| match partial {
                                Some((b, bc)) => fm::scene::route_partial(&g, &d, b, bc),
                                None => fm::scene::route(&g, &d),
                            });
                            match res {
                                Ok(sc) => {
                                    let svg = fm::scene::to_svg(&sc);
                                    out.push(ok(kind, svg, Sc::Flow(sc), d, chained));
                                }
                                Err(p) => out.push(Out { kind, svg: Err(format!("PANIC {p}")), sc: Sc::No, given: d, chained }),
                            }
                        };
                        push("r0".into(), c.clone(), None, false);
                        push("p0".into(), c.clone(), Some((&laid, &c)), false);
                        let inward: Vec<P> = c.iter().map(|p| (p.0 + 30.0, p.1 + 20.0)).collect();
                        push("rin".into(), inward.clone(), None, false);
                        if !lite {
                            push("pin".into(), inward.clone(), Some((&laid, &c)), false);
                        }
                        for k in super::plan_nodes(&c, lite) {
                            let mut d = c.clone();
                            d[k] = (c[k].0 + 7.0, c[k].1 + 5.0);
                            push(format!("rn{k}"), d.clone(), None, false);
                            push(format!("pn{k}"), d.clone(), Some((&laid, &c)), false);
                            if !lite {
                                let mut d2 = c.clone();
                                d2[k] = (c[k].0 + 40.0, c[k].1 + 30.0);
                                push(format!("rm{k}"), d2.clone(), None, false);
                            }
                            let tos: Vec<(&str, P)> = if lite {
                                vec![("a", (-60.0, -40.0)), ("b", (-60.0, c[k].1))]
                            } else {
                                vec![("a", (-60.0, -40.0)), ("b", (-60.0, c[k].1)), ("c", (c[k].0, -45.0)), ("e", (-200.0, -150.0))]
                            };
                            for (tag, to) in tos {
                                let mut d = c.clone();
                                d[k] = to;
                                push(format!("r{tag}{k}"), d.clone(), None, false);
                                push(format!("p{tag}{k}"), d.clone(), Some((&laid, &c)), false);
                            }
                            if !lite {
                                // everything 40 in on x, node above the top: overflow on y only
                                let mut d: Vec<P> = c.iter().map(|p| (p.0 + 40.0, p.1)).collect();
                                d[k].1 = -45.0;
                                push(format!("rd{k}"), d.clone(), None, false);
                                push(format!("pd{k}"), d.clone(), Some((&laid, &c)), false);
                                // chain: frame past top-left, then back inside on the overflowed base
                                let mut d1 = c.clone();
                                d1[k] = (-60.0, -40.0);
                                if let Ok(f1) = caught(|| fm::scene::route_partial(&g, &d1, &laid, &c)) {
                                    let mut d2 = c.clone();
                                    d2[k] = (c[k].0 + 7.0, c[k].1 + 5.0);
                                    push(format!("pq{k}"), d2.clone(), Some((&f1, &d1)), true);
                                    let mut d3 = c.clone();
                                    d3[k] = (-90.0, -70.0);
                                    push(format!("pr{k}"), d3.clone(), Some((&f1, &d1)), true);
                                }
                            }
                        }
                    }
                    Document::Class(d) => {
                        let laid = fm::class::scene(&d);
                        out.push(ok("scene".into(), fm::class::to_svg(&laid), Sc::No, vec![], false));
                        let c: Vec<P> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                        let nc = d.classes.len();
                        if nc == 0 {
                            return out;
                        }
                        let mut push = |kind: String, p: Vec<P>| match caught(|| fm::class::route(&d, &p)) {
                            Ok(cs) => {
                                let svg = fm::class::to_svg(&cs);
                                out.push(ok(kind, svg, Sc::Class(cs), p, false));
                            }
                            Err(e) => out.push(Out { kind, svg: Err(format!("PANIC {e}")), sc: Sc::No, given: p, chained: false }),
                        };
                        push("r0".into(), c.clone());
                        if c.len() > nc {
                            push("r0c".into(), c[..nc].to_vec());
                        }
                        push("rin".into(), c.iter().map(|p| (p.0 + 30.0, p.1 + 20.0)).collect());
                        for k in super::plan_nodes(&c[..nc], lite) {
                            let mut dd = c.clone();
                            dd[k] = (c[k].0 + 7.0, c[k].1 + 5.0);
                            push(format!("rn{k}"), dd);
                            let tos: Vec<(&str, P)> = if lite {
                                vec![("a", (-60.0, -40.0)), ("b", (-60.0, c[k].1))]
                            } else {
                                vec![("a", (-60.0, -40.0)), ("b", (-60.0, c[k].1)), ("c", (c[k].0, -45.0)), ("e", (-200.0, -150.0))]
                            };
                            for (tag, to) in tos {
                                let mut dd = c.clone();
                                dd[k] = to;
                                push(format!("r{tag}{k}"), dd.clone());
                                if c.len() > nc && tag == "a" {
                                    push(format!("r{tag}{k}c"), dd[..nc].to_vec());
                                }
                            }
                        }
                    }
                    Document::Er(d) => {
                        let laid = fm::er::scene(&d);
                        out.push(ok("scene".into(), fm::er::to_svg(&laid), Sc::No, vec![], false));
                        let c: Vec<P> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                        if c.is_empty() {
                            return out;
                        }
                        let mut push = |kind: String, p: Vec<P>| match caught(|| fm::er::route(&d, &p)) {
                            Ok(es) => {
                                let svg = fm::er::to_svg(&es);
                                out.push(ok(kind, svg, Sc::Er(es), p, false));
                            }
                            Err(e) => out.push(Out { kind, svg: Err(format!("PANIC {e}")), sc: Sc::No, given: p, chained: false }),
                        };
                        push("r0".into(), c.clone());
                        push("rin".into(), c.iter().map(|p| (p.0 + 30.0, p.1 + 20.0)).collect());
                        for k in super::plan_nodes(&c, lite) {
                            let mut dd = c.clone();
                            dd[k] = (c[k].0 + 7.0, c[k].1 + 5.0);
                            push(format!("rn{k}"), dd);
                            for (tag, to) in [("a", (-60.0, -40.0)), ("b", (-60.0, c[k].1)), ("c", (c[k].0, -45.0))] {
                                let mut dd = c.clone();
                                dd[k] = to;
                                push(format!("r{tag}{k}"), dd);
                            }
                        }
                    }
                    Document::Mindmap(d) => {
                        let laid = fm::mindmap::scene(&d);
                        out.push(ok("scene".into(), fm::mindmap::to_svg(&laid), Sc::No, vec![], false));
                        let c: Vec<P> = laid.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                        if c.is_empty() {
                            return out;
                        }
                        for (kind, p) in [
                            ("r0", c.clone()),
                            ("rn0", { let mut x = c.clone(); x[0].0 += 7.0; x[0].1 += 5.0; x }),
                            ("ra0", { let mut x = c.clone(); x[0] = (-60.0, -40.0); x }),
                            ("rshort", c[..c.len() - 1].to_vec()),
                        ] {
                            match caught(|| fm::mindmap::to_svg(&fm::mindmap::route(&d, &p))) {
                                Ok(s) => out.push(ok(kind.into(), s, Sc::No, p, false)),
                                Err(e) => out.push(Out { kind: kind.into(), svg: Err(format!("PANIC {e}")), sc: Sc::No, given: p, chained: false }),
                            }
                        }
                    }
                    _ => {}
                }
                out
            }
        }
    };
}

gen!(dv, fm_dev);
gen!(hd, fm_head);

// ---------------------------------------------------------------- SVG utils
pub fn view_box(svg: &str) -> Option<(f64, f64, f64, f64)> {
    let i = svg.find(" viewBox=\"")? + 10;
    let j = svg[i..].find('"')? + i;
    let v: Vec<f64> = svg[i..j].split(' ').filter_map(|t| t.parse().ok()).collect();
    (v.len() == 4).then(|| (v[0], v[1], v[2], v[3]))
}

fn toks(line: &str) -> Vec<Result<f64, String>> {
    // numbers: -?digits(.digits)?, everything else text
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        let starts_num = c.is_ascii_digit() || (c == '-' && i + 1 < b.len() && (b[i + 1] as char).is_ascii_digit());
        // a digit glued to letters (ids like t8, s14) stays text
        let glued = i > 0 && ((b[i - 1] as char).is_ascii_alphabetic() || b[i - 1] == b'_' || b[i - 1] == b'#');
        if starts_num && !glued {
            let s = i;
            i += 1;
            while i < b.len() && (b[i] as char).is_ascii_digit() {
                i += 1;
            }
            if i + 1 < b.len() && b[i] == b'.' && (b[i + 1] as char).is_ascii_digit() {
                i += 1;
                while i < b.len() && (b[i] as char).is_ascii_digit() {
                    i += 1;
                }
            }
            if !text.is_empty() {
                out.push(Err(std::mem::take(&mut text)));
            }
            out.push(Ok(line[s..i].parse().unwrap_or(f64::NAN)));
        } else {
            text.push(c);
            i += 1;
        }
    }
    if !text.is_empty() {
        out.push(Err(text));
    }
    out
}

/// "same" / "trans" / Err(reason): is head the dev SVG drawn at its scene
/// coordinates, i.e. every coordinate + viewBox origin?
pub fn translation(dev: &str, head: &str) -> Result<&'static str, String> {
    if dev == head {
        return Ok("same");
    }
    let (Some(vd), Some(vh)) = (view_box(dev), view_box(head)) else {
        return Err("noviewbox".into());
    };
    if vh.0 == 0.0 && vh.1 == 0.0 {
        return Err(format!("vb00-but-differs"));
    }
    if vd.0 != 0.0 || vd.1 != 0.0 {
        return Err("dev-vb-nonzero".into());
    }
    if (vd.2 - vh.2).abs() > 0.5 || (vd.3 - vh.3).abs() > 0.5 {
        return Err(format!("vbsize {:?} {:?}", vd, vh));
    }
    let (ox, oy) = (vh.0, vh.1);
    let ld: Vec<&str> = dev.lines().collect();
    let lh: Vec<&str> = head.lines().collect();
    if ld.len() != lh.len() {
        return Err(format!("lines {} {}", ld.len(), lh.len()));
    }
    for (a, b) in ld.iter().zip(&lh) {
        if a == b || a.starts_with("<svg ") {
            continue;
        }
        let b2;
        let b = if b.starts_with("<rect x=\"") && a.starts_with("<rect width=\"") {
            // background rect: head carries x/y of the origin
            let want = format!("<rect x=\"{}\" y=\"{}\" ", fmt_o(ox), fmt_o(oy));
            if !b.starts_with(&want) {
                return Err(format!("bgrect {b}"));
            }
            b2 = format!("<rect {}", &b[want.len()..]);
            &b2[..]
        } else {
            b
        };
        if a == &b {
            continue;
        }
        let (ta, tb) = (toks(a), toks(b));
        if ta.len() != tb.len() {
            return Err(format!("tokcount: {} || {}", short(a), short(b)));
        }
        for (x, y) in ta.iter().zip(&tb) {
            match (x, y) {
                (Err(s), Err(t)) if s == t => {}
                (Ok(u), Ok(v)) => {
                    let d = v - u;
                    if d.abs() < 1e-9 || (d - ox).abs() <= 0.151 || (d - oy).abs() <= 0.151 {
                        continue;
                    }
                    return Err(format!("num {u}->{v} (o {ox},{oy}): {} || {}", short(a), short(b)));
                }
                _ => return Err(format!("text: {} || {}", short(a), short(b))),
            }
        }
    }
    Ok("trans")
}

fn fmt_o(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if r == 0.0 {
        "0".into()
    } else if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        format!("{r:.1}")
    }
}

fn short(s: &str) -> String {
    s.chars().take(220).collect()
}

fn attr(line: &str, k: &str) -> Option<f64> {
    let pat = format!(" {k}=\"");
    let i = line.find(&pat)? + pat.len();
    line[i..i + line[i..].find('"')?].parse().ok()
}

fn shape_centre(line: &str) -> Option<P> {
    if line.starts_with("<rect ") {
        Some((attr(line, "x")? + attr(line, "width")? / 2.0, attr(line, "y")? + attr(line, "height")? / 2.0))
    } else if line.starts_with("<circle ") || line.starts_with("<ellipse ") {
        Some((attr(line, "cx")?, attr(line, "cy")?))
    } else if line.starts_with("<polygon ") || line.starts_with("<path ") || line.starts_with("<polyline ") {
        let key = if line.starts_with("<path ") { " d=\"" } else { " points=\"" };
        let i = line.find(key)? + key.len();
        let body = &line[i..i + line[i..].find('"')?];
        let ns: Vec<f64> = body
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|t| !t.is_empty())
            .filter_map(|t| t.parse().ok())
            .collect();
        if ns.len() < 2 || body.contains('A') || body.contains('a') {
            return None;
        }
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for p in ns.chunks(2) {
            if p.len() == 2 {
                x0 = x0.min(p[0]);
                x1 = x1.max(p[0]);
                y0 = y0.min(p[1]);
                y1 = y1.max(p[1]);
            }
        }
        Some(((x0 + x1) / 2.0, (y0 + y1) / 2.0))
    } else {
        None
    }
}

/// Flowchart: drawn centre of each `<g><title>` node group, in order.
pub fn flow_drawn(svg: &str) -> Vec<Option<P>> {
    let lines: Vec<&str> = svg.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if l.starts_with("<g><title>") && l.ends_with("</title>") {
            out.push(lines.get(i + 1).and_then(|n| shape_centre(n)));
        }
    }
    out
}

/// Class / ER: centres of the white rx=4 boxes, in order.
pub fn box_drawn(svg: &str) -> Vec<P> {
    svg.lines()
        .filter(|l| l.starts_with("<rect x=") && l.contains("rx=\"4\" fill=\"#ffffff\"/>"))
        .filter_map(shape_centre)
        .collect()
}

/// Class / ER: the static SVG's boxes are the nodes in order when each is
/// drawn at its node moved by one common shift (fit_top_left); then the
/// routed boxes must sit exactly at their node centres (offset 0).
fn consistent(drawn: &[P], nodes: &[fm_head::scene::SceneNode], n: usize) -> Option<Vec<P>> {
    if drawn.len() < n || nodes.len() < n || n == 0 {
        return None;
    }
    let s0 = (drawn[0].0 - nodes[0].x, drawn[0].1 - nodes[0].y);
    let ok = (0..n).all(|k| ((drawn[k].0 - nodes[k].x - s0.0).abs() <= 0.11) && ((drawn[k].1 - nodes[k].y - s0.1).abs() <= 0.11));
    ok.then(|| vec![(0.0, 0.0); n])
}

// --------------------------------------------------------------- head checks
fn flow_check(sc: &fm_head::scene::Scene, given: &[P], svg: &str, stat_off: &Option<Vec<Option<P>>>, chained: bool, info: &mut Vec<String>) {
    let nodes_ok = sc.nodes.len() == given.len() && sc.nodes.iter().zip(given).all(|(n, p)| n.x == p.0 && n.y == p.1);
    if !nodes_ok {
        info.push("NODES_MOVED".into());
    }
    let vb = view_box(svg).unwrap();
    let o = sc.origin;
    info.push(format!("o={:.1},{:.1};vb={},{}", o.0, o.1, vb.0, vb.1));
    let vb0 = vb.0 == 0.0 && vb.1 == 0.0;
    if !vb0 {
        let over = o.0 < 0.0 || o.1 < 0.0;
        if !over {
            info.push("VB_NONZERO_NO_ORIGIN".into());
        }
        for (oa, va) in [(o.0, vb.0), (o.1, vb.1)] {
            if oa < 0.0 {
                if (va - oa).abs() > 0.051 {
                    info.push(format!("VB_NE_ORIGIN {oa} {va}"));
                }
            } else if va < -0.051 {
                info.push(format!("VB_NEG_NO_ORIGIN {oa} {va}"));
            } else if va > 0.0 {
                info.push("vbpos".into());
            }
        }
        // drawn centres == given (+ shape offset seen in the static SVG)
        let drawn = flow_drawn(svg);
        match stat_off {
            Some(off) if off.len() == drawn.len() && drawn.len() == given.len() => {
                let (mut bad, mut pickbad, mut checked) = (0, 0, 0);
                for k in 0..given.len() {
                    let (Some(dc), Some(so)) = (drawn[k], off[k]) else { continue };
                    checked += 1;
                    let e = ((dc.0 - so.0 - given[k].0).abs()).max((dc.1 - so.1 - given[k].1).abs());
                    if e > 0.11 {
                        bad += 1;
                    }
                    // pick at the drawn centre (SVG user space == scene space here)
                    let p = (dc.0 - so.0, dc.1 - so.1);
                    let at = sc.node_at(p.0, p.1);
                    let hit = sc.hit_test(p.0, p.1, 2.0);
                    let good = matches!(at, Some(j) if j >= k) && matches!(hit, Some(fm_head::scene::Hit::Node(j)) if j >= k);
                    if !good {
                        pickbad += 1;
                    }
                }
                info.push(format!("drawn={checked}"));
                if bad > 0 {
                    info.push(format!("DRAWN_OFF={bad}"));
                }
                if pickbad > 0 {
                    info.push(format!("PICK_MISS={pickbad}"));
                }
            }
            _ => info.push("nodrawn".into()),
        }
    } else if o.0 < 0.0 || o.1 < 0.0 {
        info.push(if chained { "oneg-vb00-chain".into() } else { "ONEG_VB00".into() });
    }
}

fn box_check(sc: &fm_head::scene::Scene, n: usize, given: &[P], svg: &str, stat_off: &Option<Vec<P>>, info: &mut Vec<String>) {
    let nodes_ok = sc.nodes.len() == given.len().max(sc.nodes.len()) && sc.nodes.iter().zip(given).all(|(nn, p)| nn.x == p.0 && nn.y == p.1);
    if !nodes_ok {
        info.push("NODES_MOVED".into());
    }
    let vb = view_box(svg).unwrap();
    let o = sc.origin;
    info.push(format!("o={:.1},{:.1};vb={},{}", o.0, o.1, vb.0, vb.1));
    if (vb.0 - o.0).abs() > 0.051 || (vb.1 - o.1).abs() > 0.051 {
        info.push("VB_NE_ORIGIN".into());
    }
    let drawn = box_drawn(svg);
    if let Some(off) = stat_off {
        if drawn.len() >= n && off.len() >= n {
            let (mut bad, mut pickbad) = (0, 0);
            for k in 0..n {
                let e = ((drawn[k].0 - off[k].0 - given[k].0).abs()).max((drawn[k].1 - off[k].1 - given[k].1).abs());
                if e > 0.11 {
                    bad += 1;
                }
                let p = (drawn[k].0 - off[k].0, drawn[k].1 - off[k].1);
                if !matches!(sc.node_at(p.0, p.1), Some(j) if j >= k) {
                    pickbad += 1;
                }
            }
            info.push(format!("drawn={n}"));
            if bad > 0 {
                info.push(format!("DRAWN_OFF={bad}"));
            }
            if pickbad > 0 {
                info.push(format!("PICK_MISS={pickbad}"));
            }
        } else {
            info.push("nodrawn".into());
        }
    }
}

// ---------------------------------------------------------- try_route sweep
fn same_dbg<T: std::fmt::Debug>(a: &T, b: &T) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

fn rnd(seed: &mut u64) -> f64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed % 1_000_000) as f64 / 1_000_000.0
}

fn try_sweep(src: &str, lite: bool) -> String {
    use fm_head::model::Document;
    use fm_head::{class, er, mindmap, scene};
    let Some(d) = hd::doc(src) else { return String::new() };
    let (laid, accepted): (Vec<P>, Vec<usize>) = match &d {
        Document::Flowchart(g) | Document::State(g) => (scene::scene(g).nodes.iter().map(|n| (n.x, n.y)).collect(), vec![g.nodes.len()]),
        Document::Class(c) => {
            let n = c.classes.len();
            let mut a = vec![n, n + c.notes.len()];
            a.dedup();
            (class::scene(c).scene.nodes.iter().map(|n| (n.x, n.y)).collect(), a)
        }
        Document::Er(e) => (er::scene(e).scene.nodes.iter().map(|n| (n.x, n.y)).collect(), vec![e.entities.len()]),
        Document::Mindmap(m) => (mindmap::scene(m).nodes.iter().map(|n| (n.cx(), n.cy())).collect(), vec![m.nodes.len()]),
        _ => return String::new(),
    };
    let n = laid.len();
    let small = n <= 40;
    // Some(eq) / None
    let attempt = |pos: &[P], cmp: bool| -> Result<Option<bool>, String> {
        catch_unwind(AssertUnwindSafe(|| match &d {
            Document::Flowchart(g) | Document::State(g) => scene::try_route(g, pos).map(|s| !cmp || (scene::to_svg(&s) == scene::to_svg(&scene::route(g, pos)) && (!small || same_dbg(&s, &scene::route(g, pos))))),
            Document::Class(c) => class::try_route(c, pos).map(|s| !cmp || (class::to_svg(&s) == class::to_svg(&class::route(c, pos)) && (!small || same_dbg(&s, &class::route(c, pos))))),
            Document::Er(e) => er::try_route(e, pos).map(|s| !cmp || (er::to_svg(&s) == er::to_svg(&er::route(e, pos)) && (!small || same_dbg(&s, &er::route(e, pos))))),
            Document::Mindmap(m) => mindmap::try_route(m, pos).map(|s| !cmp || (mindmap::to_svg(&s) == mindmap::to_svg(&mindmap::route(m, pos)) && same_dbg(&s, &mindmap::route(m, pos)))),
            _ => None,
        }))
        .map_err(|_| "PANIC".to_string())
    };
    let pos_k = |k: usize| -> Vec<P> { (0..k).map(|i| laid.get(i).copied().unwrap_or((40.0 + 30.0 * i as f64, 40.0 + 20.0 * i as f64))).collect() };
    let counts: Vec<usize> = if small && !lite {
        (0..n + 4).collect()
    } else {
        let mut v = vec![0, 1, n.saturating_sub(1), n, n + 1, n + 3];
        v.extend(accepted.iter().copied());
        v.extend(accepted.iter().map(|a| a + 1));
        v.extend(accepted.iter().filter(|&&a| a > 0).map(|a| a - 1));
        v.sort();
        v.dedup();
        v
    };
    let mut bad = Vec::new();
    let mut nsome = 0;
    for k in &counts {
        let want = accepted.contains(k);
        match attempt(&pos_k(*k), true) {
            Err(p) => bad.push(format!("k{k}:{p}")),
            Ok(Some(true)) if want => nsome += 1,
            Ok(Some(false)) => bad.push(format!("k{k}:NE_ROUTE")),
            Ok(None) if !want => {}
            Ok(r) => bad.push(format!("k{k}:{r:?}")),
        }
    }
    // non-finite / far: None, never a panic
    for &a in &accepted {
        if a == 0 {
            continue;
        }
        for v in [f64::NAN, f64::INFINITY, -1.0e6 - 1.0, 2.0e6, f64::MAX] {
            let mut p = pos_k(a);
            p[a / 2].0 = v;
            match attempt(&p, false) {
                Ok(None) => {}
                r => bad.push(format!("far{v}:{r:?}")),
            }
        }
        // fuzz within reach: no panic
        if a <= 60 {
            let mut seed = 0x9e3779b97f4a7c15u64 ^ (a as u64 * 7919);
            let spreads: &[f64] = if lite { &[400.0] } else { &[50.0, 600.0, 5000.0] };
            for &sp in spreads {
                let p: Vec<P> = (0..a).map(|_| ((rnd(&mut seed) - 0.5) * 2.0 * sp, (rnd(&mut seed) - 0.5) * 2.0 * sp)).collect();
                match attempt(&p, false) {
                    Ok(Some(_)) => {}
                    r => bad.push(format!("fuzz{sp}:{r:?}")),
                }
            }
            let same = vec![(-120.0, -80.0); a];
            match attempt(&same, false) {
                Ok(Some(_)) => {}
                r => bad.push(format!("coincident:{r:?}")),
            }
        }
    }
    format!("try n={n} acc={accepted:?} some={nsome} {}", if bad.is_empty() { "OK".to_string() } else { format!("BAD {}", bad.join(",")) })
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a[1].as_str() {
        "all" => {
            let list = std::fs::read_to_string(&a[2]).unwrap();
            let outdir = std::path::PathBuf::from(&a[3]);
            let lite = a.get(4).map(|s| s == "lite").unwrap_or(false);
            std::fs::create_dir_all(outdir.join("diffs")).unwrap();
            let mut tsv = String::new();
            for (fi, f) in list.lines().filter(|l| !l.is_empty()).enumerate() {
                let Ok(src) = std::fs::read_to_string(f) else { continue };
                let t0 = std::time::Instant::now();
                let dv_out = dv::run(&src, lite);
                let hd_out = hd::run(&src, lite);
                // static reference offsets (head static scene) for drawn-centre checks
                let mut flow_off: Option<Vec<Option<P>>> = None;
                let mut box_off: Option<Vec<P>> = None;
                let mut nclass = 0usize;
                match hd::doc(&src) {
                    Some(fm_head::model::Document::Flowchart(g)) | Some(fm_head::model::Document::State(g)) => {
                        let laid = fm_head::scene::scene(&g);
                        let drawn = flow_drawn(&fm_head::scene::to_svg(&laid));
                        if drawn.len() == laid.nodes.len() {
                            flow_off = Some(drawn.iter().zip(&laid.nodes).map(|(d, n)| d.map(|d| (d.0 - n.x, d.1 - n.y))).collect());
                        }
                    }
                    Some(fm_head::model::Document::Class(d)) => {
                        let laid = fm_head::class::scene(&d);
                        nclass = d.classes.len();
                        let drawn = box_drawn(&fm_head::class::to_svg(&laid));
                        box_off = consistent(&drawn, &laid.scene.nodes, nclass);
                    }
                    Some(fm_head::model::Document::Er(d)) => {
                        let laid = fm_head::er::scene(&d);
                        nclass = d.entities.len();
                        let drawn = box_drawn(&fm_head::er::to_svg(&laid));
                        box_off = consistent(&drawn, &laid.scene.nodes, nclass);
                    }
                    _ => {}
                }
                if dv_out.len() != hd_out.len() {
                    tsv.push_str(&format!("{f}\t*\tOUTCOUNT\t{} {}\n", dv_out.len(), hd_out.len()));
                }
                for (x, y) in dv_out.iter().zip(&hd_out) {
                    assert_eq!(x.kind, y.kind);
                    let status = match (&x.svg, &y.svg) {
                        (Ok(s), Ok(t)) => match translation(s, t) {
                            Ok(st) => st.to_string(),
                            Err(e) => {
                                let base = outdir.join("diffs").join(format!("{fi}.{}", y.kind));
                                std::fs::write(base.with_extension(format!("{}.dev.svg", y.kind)), s).ok();
                                std::fs::write(base.with_extension(format!("{}.head.svg", y.kind)), t).ok();
                                format!("DIFF {e}")
                            }
                        },
                        (Err(e1), Err(e2)) => if e1 == e2 { "same-err".into() } else { format!("ERRDIFF {e1} || {e2}") },
                        (Ok(_), Err(e)) => format!("NEWERR {e}"),
                        (Err(e), Ok(_)) => format!("NOWOK {e}"),
                    };
                    let mut info = Vec::new();
                    if let Ok(svg) = &y.svg {
                        match &y.sc {
                            hd::Sc::Flow(sc) => flow_check(sc, &y.given, svg, &flow_off, y.chained, &mut info),
                            hd::Sc::Class(cs) => {
                                box_check(&cs.scene, nclass, &y.given, svg, &box_off, &mut info);
                                // cardinalities drawn where card_spots puts them (unless clamped)
                                let (mut miss, mut cards) = (0, 0);
                                let vb = view_box(svg).unwrap();
                                for (rel, spots) in cs.rels.iter().zip(fm_head::class::card_spots(cs)) {
                                    for (text, at) in [(&rel.from_card, spots[0]), (&rel.to_card, spots[1])] {
                                        if let (Some(_), Some(at)) = (text, at) {
                                            cards += 1;
                                            let inside = at.0 > vb.0 + 40.0 && at.0 < vb.0 + vb.2 - 40.0 && at.1 > vb.1 + 8.0 && at.1 < vb.1 + vb.3 - 8.0;
                                            let drawn = format!("<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\"", at.0, at.1);
                                            if inside && !svg.contains(&drawn) {
                                                miss += 1;
                                                info.push(format!("cardspot=({:.1},{:.1})", at.0, at.1));
                                            }
                                        }
                                    }
                                }
                                if miss > 0 {
                                    info.push(format!("CARD_NOT_AT_SPOT={miss}/{cards}"));
                                }
                            }
                            hd::Sc::Er(es) => box_check(&es.scene, nclass, &y.given, svg, &box_off, &mut info),
                            hd::Sc::No => {}
                        }
                    }
                    if let Ok(svg) = &x.svg {
                        if let Some(vb) = view_box(svg) {
                            if (vb.0, vb.1) != (0.0, 0.0) {
                                info.push("DEVVB".into());
                            }
                        }
                    }
                    tsv.push_str(&format!("{f}\t{}\t{status}\t{}\n", y.kind, info.join(";")));
                }
                tsv.push_str(&format!("{f}\ttry\t{}\t\n", try_sweep(&src, lite)));
                tsv.push_str(&format!("{f}\ttime\t{:.2}\t\n", t0.elapsed().as_secs_f64()));
                if fi % 50 == 0 {
                    std::fs::write(outdir.join("results.tsv"), &tsv).unwrap();
                    eprintln!("{fi} {f}");
                }
            }
            std::fs::write(outdir.join("results.tsv"), &tsv).unwrap();
        }
        "one" => {
            let src = std::fs::read_to_string(&a[2]).unwrap();
            let kind = &a[3];
            for (tag, outs) in [("dev", dv::run(&src, false).into_iter().map(|o| (o.kind, o.svg, o.given)).collect::<Vec<_>>()), ("head", hd::run(&src, false).into_iter().map(|o| (o.kind, o.svg, o.given)).collect())] {
                for (k, s, g) in outs {
                    if &k == kind {
                        std::fs::write(format!("{}.{tag}.svg", a[4]), s.unwrap_or_else(|e| e)).unwrap();
                        let js: Vec<String> = g.iter().map(|p| format!("[{},{}]", p.0, p.1)).collect();
                        std::fs::write(format!("{}.given.json", a[4]), format!("[{}]", js.join(","))).unwrap();
                    }
                }
            }
        }
        "render" => {
            let src = std::fs::read_to_string(&a[2]).unwrap();
            let d = fm_dev::render_svg(&src).map_err(|e| e.message);
            let h = fm_head::render_svg(&src).map_err(|e| e.message);
            println!("render same={}", d == h);
        }
        "parse" => {
            // cases separated by a line "----"; each is prefixed with "classDiagram\n"
            let all = std::fs::read_to_string(&a[2]).unwrap();
            for (i, c) in all.split("\n----\n").enumerate() {
                let src = format!("classDiagram\n{c}\n");
                let d = match fm_dev::parser::parse_document(&src) {
                    Ok(fm_dev::model::Document::Class(d)) => format!("ok\t{}", d.classes.iter().map(|c| c.name.clone()).collect::<Vec<_>>().join(",")),
                    Ok(_) => "other".into(),
                    Err(e) => format!("ERR\t{}", e.message.chars().take(70).collect::<String>()),
                };
                let h = match fm_head::parser::parse_document(&src) {
                    Ok(fm_head::model::Document::Class(d)) => format!("ok\t{}", d.classes.iter().map(|c| c.name.clone()).collect::<Vec<_>>().join(",")),
                    Ok(_) => "other".into(),
                    Err(e) => format!("ERR\tline {} {}", e.line, e.message.chars().take(110).collect::<String>()),
                };
                println!("{i}\t{c:?}\tDEV {d}\tHEAD {h}");
            }
        }
        _ => panic!("mode"),
    }
}
