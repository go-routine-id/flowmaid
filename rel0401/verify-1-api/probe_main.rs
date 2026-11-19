// Verifier probe: dumps SVGs for static render and routed variants.
// usage: probe dump <outdir> <listfile>
//        probe check <listfile>        (head only: try_route checks)
use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser, scene};
use std::fs;
use std::path::Path;

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
    d[l].0 += 40.0; // leftmost moved right: nothing overflows top-left
    v.push(("right", d));
    let mut d = c.to_vec();
    d[t].1 += 40.0; // topmost moved down
    v.push(("down", d));
    let mut d = c.to_vec();
    d[n / 2].0 += 15.0;
    d[n / 2].1 += 10.0;
    v.push(("mid", d));
    let mut d = c.to_vec();
    d[l].0 -= 25.0;
    v.push(("left", d));
    let mut d = c.to_vec();
    d[t].1 -= 30.0;
    v.push(("up", d));
    let mut d = c.to_vec();
    d[t].1 = -40.0;
    d[l].0 = -60.0;
    v.push(("over", d));
    let mut d = c.to_vec();
    let far = (0..n)
        .max_by(|&a, &b| boxes[a].2.total_cmp(&boxes[b].2))
        .unwrap();
    d[far].0 += 50.0;
    d[far].1 += 20.0;
    v.push(("farout", d));
    v
}

fn boxes_of(ns: &[scene::SceneNode]) -> Vec<B> {
    ns.iter()
        .map(|n| (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0))
        .collect()
}

fn meta(out: &Path, n: &str, v: &str, sc: &scene::Scene) {
    #[cfg(feature = "head")]
    let o = sc.origin;
    #[cfg(not(feature = "head"))]
    let o = (0.0f64, 0.0f64);
    let mut s = format!("{} {} {} {}\n", o.0, o.1, sc.width, sc.height);
    for nd in &sc.nodes {
        s.push_str(&format!("{} {} {} {}\n", nd.x, nd.y, nd.w, nd.h));
    }
    fs::write(out.join(format!("{n}.{v}.meta")), s).unwrap();
}

fn dump(out: &Path, list: &str) {
    fs::create_dir_all(out).unwrap();
    for (i, f) in fs::read_to_string(list).unwrap().lines().enumerate() {
        let Ok(src) = fs::read_to_string(f) else { continue };
        let n = name_of(i, f);
        match flowmaid::render_svg(&src) {
            Ok(s) => put(out, &n, "static", &s),
            Err(e) => put(out, &n, "static", &format!("ERR {e:?}")),
        }
        let Ok(doc) = parser::parse_document(&src) else { continue };
        match doc {
            Document::Flowchart(g) | Document::State(g) => {
                let sc = scene::scene(&g);
                let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
                let sizes: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.w, n.h)).collect();
                let b = boxes_of(&sc.nodes);
                put(out, &n, "scene", &scene::to_svg(&sc));
                let r0 = scene::route(&g, &c);
                put(out, &n, "route0", &scene::to_svg(&r0));
                meta(out, &n, "route0", &r0);
                put(out, &n, "sized0", &scene::to_svg(&scene::route_sized(&g, &c, &sizes)));
                for (v, d) in drags(&c, &b) {
                    let r = scene::route(&g, &d);
                    put(out, &n, &format!("route-{v}"), &scene::to_svg(&r));
                    meta(out, &n, &format!("route-{v}"), &r);
                    let p = scene::route_partial(&g, &d, &sc, &c);
                    put(out, &n, &format!("partial-{v}"), &scene::to_svg(&p));
                    meta(out, &n, &format!("partial-{v}"), &p);
                    // chained partial: base = previous routed frame
                    let p2 = scene::route_partial(&g, &c, &p, &d);
                    put(out, &n, &format!("partialback-{v}"), &scene::to_svg(&p2));
                    put(out, &n, &format!("sized-{v}"), &scene::to_svg(&scene::route_sized(&g, &d, &sizes)));
                }
            }
            Document::Class(d) => {
                let cs = class::scene(&d);
                let c: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let b = boxes_of(&cs.scene.nodes);
                put(out, &n, "scene", &class::to_svg(&cs));
                put(out, &n, "route0", &class::to_svg(&class::route(&d, &c)));
                let k = d.classes.len();
                put(out, &n, "route0c", &class::to_svg(&class::route(&d, &c[..k])));
                for (v, p) in drags(&c[..k], &b[..k]) {
                    let r = class::route(&d, &p);
                    put(out, &n, &format!("route-{v}"), &class::to_svg(&r));
                    meta(out, &n, &format!("route-{v}"), &r.scene);
                }
                for (v, p) in drags(&c, &b) {
                    let r = class::route(&d, &p);
                    put(out, &n, &format!("routeall-{v}"), &class::to_svg(&r));
                }
            }
            Document::Er(d) => {
                let es = er::scene(&d);
                let c: Vec<(f64, f64)> = es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let b = boxes_of(&es.scene.nodes);
                put(out, &n, "scene", &er::to_svg(&es));
                put(out, &n, "route0", &er::to_svg(&er::route(&d, &c)));
                for (v, p) in drags(&c, &b) {
                    let r = er::route(&d, &p);
                    put(out, &n, &format!("route-{v}"), &er::to_svg(&r));
                    meta(out, &n, &format!("route-{v}"), &r.scene);
                }
            }
            Document::Mindmap(d) => {
                let ms = mindmap::scene(&d);
                let c: Vec<(f64, f64)> = ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
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
        "dump" => dump(Path::new(&a[2]), &a[3]),
        #[cfg(feature = "head")]
        "check" => check::run(&a[2], a.get(3).map(|s| s.as_str())),
        _ => panic!("mode"),
    }
}
