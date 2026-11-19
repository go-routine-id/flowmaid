// Corpus dump: static render plus route()/route_partial() SVGs at the
// layout positions and after a few drags, for every diagram type with a
// route. usage: probe <outdir> <file.mmd>...
use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser, scene};
use std::fs;
use std::path::Path;

fn name_of(p: &str) -> String {
    let p = p.trim_start_matches("./");
    let mut parts: Vec<&str> = p.rsplit('/').take(2).collect();
    parts.reverse();
    parts.join("__").trim_end_matches(".mmd").to_string()
}

fn put(out: &Path, n: &str, v: &str, s: &str) {
    fs::write(out.join(format!("{n}.{v}.svg")), s).unwrap();
}

fn drags(c: &[(f64, f64)], boxes: &[(f64, f64, f64, f64)]) -> Vec<(&'static str, Vec<(f64, f64)>)> {
    // boxes: (x0, y0, x1, y1)
    let n = c.len();
    if n == 0 {
        return Vec::new();
    }
    let arg = |f: &dyn Fn(&(f64, f64, f64, f64)) -> f64| {
        (0..n).min_by(|&a, &b| f(&boxes[a]).total_cmp(&f(&boxes[b]))).unwrap()
    };
    let l = arg(&|b| b.0);
    let t = arg(&|b| b.1);
    let mut inw = c.to_vec();
    inw[l].0 += 30.0;
    let mut mid = c.to_vec();
    mid[n / 2].0 += 15.0;
    mid[n / 2].1 += 10.0;
    let mut over = c.to_vec();
    over[t].1 = -40.0;
    over[l].0 = -60.0;
    let mut left = c.to_vec();
    left[l].0 -= 25.0;
    vec![("in", inw), ("mid", mid), ("over", over), ("left", left)]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = Path::new(&args[1]);
    fs::create_dir_all(out).unwrap();
    for f in &args[2..] {
        let src = match fs::read_to_string(f) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let n = name_of(f);
        match flowmaid::render_svg(&src) {
            Ok(s) => put(out, &n, "static", &s),
            Err(e) => put(out, &n, "static", &format!("ERR {e:?}")),
        }
        let Ok(doc) = parser::parse_document(&src) else { continue };
        match doc {
            Document::Flowchart(g) | Document::State(g) => {
                let sc = scene::scene(&g);
                let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
                let b: Vec<_> = sc
                    .nodes
                    .iter()
                    .map(|n| (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0))
                    .collect();
                put(out, &n, "scene", &scene::to_svg(&sc));
                put(out, &n, "route0", &scene::to_svg(&scene::route(&g, &c)));
                let sizes: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.w, n.h)).collect();
                put(out, &n, "sized0", &scene::to_svg(&scene::route_sized(&g, &c, &sizes)));
                for (v, d) in drags(&c, &b) {
                    put(out, &n, &format!("route-{v}"), &scene::to_svg(&scene::route(&g, &d)));
                    put(
                        out,
                        &n,
                        &format!("partial-{v}"),
                        &scene::to_svg(&scene::route_partial(&g, &d, &sc, &c)),
                    );
                }
            }
            Document::Class(d) => {
                let cs = class::scene(&d);
                let c: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let b: Vec<_> = cs
                    .scene
                    .nodes
                    .iter()
                    .map(|n| (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0))
                    .collect();
                put(out, &n, "scene", &class::to_svg(&cs));
                put(out, &n, "route0", &class::to_svg(&class::route(&d, &c)));
                let k = d.classes.len();
                if k < c.len() {
                    put(out, &n, "route0c", &class::to_svg(&class::route(&d, &c[..k])));
                }
                for (v, dd) in drags(&c, &b) {
                    put(out, &n, &format!("route-{v}"), &class::to_svg(&class::route(&d, &dd)));
                }
            }
            Document::Er(d) => {
                let es = er::scene(&d);
                let c: Vec<(f64, f64)> = es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let b: Vec<_> = es
                    .scene
                    .nodes
                    .iter()
                    .map(|n| (n.x - n.w / 2.0, n.y - n.h / 2.0, n.x + n.w / 2.0, n.y + n.h / 2.0))
                    .collect();
                put(out, &n, "scene", &er::to_svg(&es));
                put(out, &n, "route0", &er::to_svg(&er::route(&d, &c)));
                for (v, dd) in drags(&c, &b) {
                    put(out, &n, &format!("route-{v}"), &er::to_svg(&er::route(&d, &dd)));
                }
            }
            Document::Mindmap(d) => {
                let ms = mindmap::scene(&d);
                let c: Vec<(f64, f64)> = ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                let b: Vec<_> = ms.nodes.iter().map(|n| (n.x, n.y, n.x + n.w, n.y + n.h)).collect();
                put(out, &n, "scene", &mindmap::to_svg(&ms));
                put(out, &n, "route0", &mindmap::to_svg(&mindmap::route(&d, &c)));
                for (v, dd) in drags(&c, &b) {
                    put(out, &n, &format!("route-{v}"), &mindmap::to_svg(&mindmap::route(&d, &dd)));
                }
            }
            _ => {}
        }
    }
}
