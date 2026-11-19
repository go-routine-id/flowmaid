// Verifier-2 (API lens) probe.
//   parse <casefile>          one case per line, "\n" / "\t" escaped
//   dump <outdir> <listfile>  static + routed SVGs (+ .meta on head)
//   check <listfile>          head only: try_route acceptance checks
//   fuzz <listfile> <seed> <iters>   head only: random finite positions
use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser, scene};
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

fn unesc(s: &str) -> String {
    let mut o = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => o.push('\n'),
                Some('t') => o.push('\t'),
                Some('\\') => o.push('\\'),
                Some(x) => {
                    o.push('\\');
                    o.push(x)
                }
                None => o.push('\\'),
            }
        } else {
            o.push(c)
        }
    }
    o
}

fn parse_cases(f: &str) {
    for (i, l) in fs::read_to_string(f).unwrap().lines().enumerate() {
        let src = unesc(l);
        let r = catch_unwind(|| parser::parse_document(&src));
        let out = match r {
            Err(_) => "PANIC".to_string(),
            Ok(Err(e)) => format!("ERR line={} {}", e.line, e.message),
            Ok(Ok(Document::Class(d))) => {
                let names: Vec<String> = d
                    .classes
                    .iter()
                    .map(|c| match &c.generic {
                        Some(g) => format!("{}~{}~", c.name, g),
                        None => c.name.clone(),
                    })
                    .collect();
                format!("OK {}", names.join(" | "))
            }
            Ok(Ok(_)) => "OK (other)".to_string(),
        };
        println!("{i}\t{out}");
    }
}

fn name_of(i: usize, p: &str) -> String {
    let b = Path::new(p).file_stem().unwrap().to_string_lossy().to_string();
    format!("{i:04}_{b}")
}

fn put(out: &Path, n: &str, v: &str, s: &str) {
    fs::write(out.join(format!("{n}.{v}.svg")), s).unwrap();
}

type B = (f64, f64, f64, f64);
fn drags(c: &[(f64, f64)], boxes: &[B]) -> Vec<(&'static str, Vec<(f64, f64)>)> {
    let n = c.len();
    if n == 0 {
        return Vec::new();
    }
    let arg = |f: &dyn Fn(&B) -> f64| {
        (0..n)
            .min_by(|&a, &b| f(&boxes[a]).total_cmp(&f(&boxes[b])))
            .unwrap()
    };
    let l = arg(&|b| b.0);
    let t = arg(&|b| b.1);
    let mut v = Vec::new();
    let mut d = c.to_vec();
    d[l].0 += 30.0;
    v.push(("in-x", d));
    let mut d = c.to_vec();
    d[t].1 += 30.0;
    v.push(("in-y", d));
    let mut d = c.to_vec();
    d[n / 2].0 += 7.0;
    d[n / 2].1 += 11.0;
    v.push(("mid", d));
    let mut d = c.to_vec();
    d[l].0 -= 33.3;
    v.push(("out-x", d));
    let mut d = c.to_vec();
    d[t].1 -= 27.7;
    v.push(("out-y", d));
    let mut d = c.to_vec();
    d[n / 2] = (-120.0, -80.0);
    v.push(("out-xy", d));
    let mut d = c.to_vec();
    d[l].0 -= 0.04; // sub-0.1 px overflow attempt
    v.push(("tiny-x", d));
    let mut d = c.to_vec();
    d[l].0 -= 400.0;
    d[t].1 += 500.0;
    v.push(("mixed", d));
    v
}

fn boxes_of(ns: &[scene::SceneNode]) -> Vec<B> {
    ns.iter()
        .map(|n| (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0))
        .collect()
}

#[allow(unused_variables)]
fn meta(out: &Path, n: &str, v: &str, sc: &scene::Scene, given: &[(f64, f64)]) {
    #[cfg(feature = "head")]
    let o = sc.origin;
    #[cfg(not(feature = "head"))]
    let o = (0.0f64, 0.0f64);
    let mut s = format!("{} {} {} {}\n", o.0, o.1, sc.width, sc.height);
    for (i, nd) in sc.nodes.iter().enumerate() {
        let g = given.get(i).copied().unwrap_or((f64::NAN, f64::NAN));
        let hit = sc.hit_test(nd.x, nd.y, 0.0);
        s.push_str(&format!(
            "{} {} {} {} {} {} {:?}\n",
            nd.x, nd.y, nd.w, nd.h, g.0, g.1, hit
        ));
    }
    fs::write(out.join(format!("{n}.{v}.meta")), s).unwrap();
}

fn dump(out: &Path, list: &str) {
    fs::create_dir_all(out).unwrap();
    for (i, f) in fs::read_to_string(list).unwrap().lines().enumerate() {
        let Ok(src) = fs::read_to_string(f) else {
            continue;
        };
        let n = name_of(i, f);
        match flowmaid::render_svg(&src) {
            Ok(s) => put(out, &n, "static", &s),
            Err(e) => put(out, &n, "static", &format!("ERR {e:?}")),
        }
        let Ok(doc) = parser::parse_document(&src) else {
            continue;
        };
        match doc {
            Document::Flowchart(g) | Document::State(g) => {
                let sc = scene::scene(&g);
                let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
                let sizes: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.w, n.h)).collect();
                let b = boxes_of(&sc.nodes);
                put(out, &n, "scene", &scene::to_svg(&sc));
                if c.is_empty() {
                    continue;
                }
                let r0 = scene::route(&g, &c);
                put(out, &n, "route0", &scene::to_svg(&r0));
                meta(out, &n, "route0", &r0, &c);
                let s0 = scene::route_sized(&g, &c, &sizes);
                put(out, &n, "sized0", &scene::to_svg(&s0));
                for (v, d) in drags(&c, &b) {
                    let r = scene::route(&g, &d);
                    put(out, &n, &format!("route-{v}"), &scene::to_svg(&r));
                    meta(out, &n, &format!("route-{v}"), &r, &d);
                    let p = scene::route_partial(&g, &d, &sc, &c);
                    put(out, &n, &format!("partial-{v}"), &scene::to_svg(&p));
                    meta(out, &n, &format!("partial-{v}"), &p, &d);
                    let p2 = scene::route_partial(&g, &c, &p, &d);
                    put(out, &n, &format!("partialback-{v}"), &scene::to_svg(&p2));
                    meta(out, &n, &format!("partialback-{v}"), &p2, &c);
                    let s = scene::route_sized(&g, &d, &sizes);
                    put(out, &n, &format!("sized-{v}"), &scene::to_svg(&s));
                    meta(out, &n, &format!("sized-{v}"), &s, &d);
                }
            }
            Document::Class(d) => {
                let cs = class::scene(&d);
                let c: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let b = boxes_of(&cs.scene.nodes);
                put(out, &n, "scene", &class::to_svg(&cs));
                let k = d.classes.len();
                if k == 0 {
                    continue;
                }
                let r = class::route(&d, &c);
                put(out, &n, "route0", &class::to_svg(&r));
                meta(out, &n, "route0", &r.scene, &c);
                let r = class::route(&d, &c[..k]);
                put(out, &n, "route0c", &class::to_svg(&r));
                for (v, p) in drags(&c[..k], &b[..k]) {
                    let r = class::route(&d, &p);
                    put(out, &n, &format!("route-{v}"), &class::to_svg(&r));
                    meta(out, &n, &format!("route-{v}"), &r.scene, &p);
                }
                if c.len() > k {
                    for (v, p) in drags(&c, &b) {
                        let r = class::route(&d, &p);
                        put(out, &n, &format!("routeall-{v}"), &class::to_svg(&r));
                        meta(out, &n, &format!("routeall-{v}"), &r.scene, &p);
                    }
                }
            }
            Document::Er(d) => {
                let es = er::scene(&d);
                let c: Vec<(f64, f64)> = es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let b = boxes_of(&es.scene.nodes);
                put(out, &n, "scene", &er::to_svg(&es));
                if c.is_empty() {
                    continue;
                }
                let r = er::route(&d, &c);
                put(out, &n, "route0", &er::to_svg(&r));
                meta(out, &n, "route0", &r.scene, &c);
                for (v, p) in drags(&c, &b) {
                    let r = er::route(&d, &p);
                    put(out, &n, &format!("route-{v}"), &er::to_svg(&r));
                    meta(out, &n, &format!("route-{v}"), &r.scene, &p);
                }
            }
            Document::Mindmap(d) => {
                let ms = mindmap::scene(&d);
                let c: Vec<(f64, f64)> = ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                put(out, &n, "mscene", &mindmap::to_svg(&ms));
                put(out, &n, "route0", &mindmap::to_svg(&mindmap::route(&d, &c)));
                if !c.is_empty() {
                    let mut p = c.clone();
                    p[0].0 -= 500.0;
                    put(out, &n, "route-left", &mindmap::to_svg(&mindmap::route(&d, &p)));
                }
            }
            _ => {}
        }
    }
}

#[cfg(feature = "head")]
mod check;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a[1].as_str() {
        "parse" => parse_cases(&a[2]),
        "dump" => dump(Path::new(&a[2]), &a[3]),
        "glyph" => glyph(&a[2]),
        "cards" => cards(&a[2]),
        #[cfg(feature = "head")]
        "check" => check::run(&a[2]),
        #[cfg(feature = "head")]
        "fuzz" => check::fuzz(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap()),
        _ => panic!("mode"),
    }
    let _ = AssertUnwindSafe(());
}

fn glyph(f: &str) {
    let src = fs::read_to_string(f).unwrap();
    let Ok(Document::Class(d)) = parser::parse_document(&src) else { return };
    let cs = class::scene(&d);
    let k = d.classes.len();
    let c: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let b = boxes_of(&cs.scene.nodes);
    let (_, p) = drags(&c[..k], &b[..k]).into_iter().find(|(v, _)| *v == "mixed").unwrap();
    for dx in [0.0, 236.7, 300.0, 1000.0, 0.125, 1024.0] {
        let q: Vec<(f64, f64)> = p.iter().map(|&(x, y)| (x + dx, y)).collect();
        let svg = class::to_svg(&class::route(&d, &q));
        let vb = svg.split("viewBox=\"").nth(1).unwrap().split('"').next().unwrap().to_string();
        let ox: f64 = vb.split(' ').next().unwrap().parse().unwrap();
        for l in svg.lines().filter(|l| l.starts_with("<polygon points=\"") && l.contains("572.0")) {
            println!("dx {dx} vb [{vb}] {}", &l[..60.min(l.len())]);
        }
        let _ = ox;
    }
}

fn cards(f: &str) {
    let src = fs::read_to_string(f).unwrap();
    let Ok(Document::Class(d)) = parser::parse_document(&src) else { return };
    let cs = class::scene(&d);
    let c: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let mut first: Option<Vec<(f64, f64)>> = None;
    for dx in [500.0, 500.125, 537.3, 1000.0, 2048.0, 4096.7] {
        let q: Vec<(f64, f64)> = c.iter().map(|&(x, y)| (x + dx, y + dx)).collect();
        let r = class::route(&d, &q);
        let spots: Vec<(f64, f64)> = class::card_spots(&r)
            .iter()
            .flat_map(|s| s.iter().flatten().map(|&(x, y)| (x - dx, y - dx)))
            .collect();
        match &first {
            None => first = Some(spots),
            Some(f0) => {
                let moved = f0.iter().zip(&spots).filter(|(a, b)| (a.0 - b.0).abs() > 0.01 || (a.1 - b.1).abs() > 0.01).count();
                println!("dx {dx}: {moved} of {} card spots not translation-equal to dx 500", spots.len());
            }
        }
    }
}
