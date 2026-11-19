// Independent default-build probe: hashes of every public output for one input.
use flowmaid::model::Document;
use flowmaid::scene::{self, Scene};
use std::fmt::Write as _;

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
fn strip(s: String) -> String {
    s.replace(", block_layout: false", "").replace(", bold_title: false", "")
}
fn dbg<T: std::fmt::Debug>(t: &T) -> u64 {
    fnv(&strip(format!("{:?}", t)))
}
fn hits(sc: &Scene) -> u64 {
    let mut o = String::new();
    let (ox, oy) = sc.origin;
    let (w, h) = (sc.width.max(1.0), sc.height.max(1.0));
    for i in 0..=12 {
        for j in 0..=12 {
            let x = ox + w * i as f64 / 12.0;
            let y = oy + h * j as f64 / 12.0;
            let _ = write!(o, "{:?}{:?}{:?}{:?}{:?};", sc.hit_test(x, y, 4.0), sc.node_at(x, y), sc.edge_at(x, y, 6.0), sc.cluster_at(x, y), sc.nearest_node(x, y));
        }
    }
    for n in &sc.nodes {
        let _ = write!(o, "{:?}{:?};", sc.hit_test(n.x, n.y, 2.0), sc.hit_test(n.x + n.w / 2.0 + 3.0, n.y, 5.0));
    }
    for e in &sc.edges {
        let p = e.waypoints.get(e.waypoints.len() / 2).copied().unwrap_or(e.bezier[1]);
        let _ = write!(o, "{:?}{:?};", sc.hit_test(p.0, p.1, 3.0), sc.edge_at(p.0 + 1.0, p.1 + 1.0, 4.0));
        let q = e.bezier[3];
        let _ = write!(o, "{:?};", sc.hit_test(q.0, q.1, 3.0));
    }
    let _ = write!(o, "{:?}", sc.hits_in_rect(ox, oy, ox + w / 2.0, oy + h / 2.0));
    let _ = write!(o, "{:?}", sc.hits_in_rect(ox + w / 3.0, oy + h / 3.0, ox + w, oy + h));
    fnv(&o)
}
fn sc_all(tag: &str, sc: &Scene, out: &mut String) {
    let _ = write!(out, "\t{tag}.svg={:016x}\t{tag}.dbg={:016x}\t{tag}.hit={:016x}", fnv(&scene::to_svg(sc)), dbg(sc), hits(sc));
}
fn centers(sc: &Scene) -> Vec<(f64, f64)> {
    sc.nodes.iter().map(|n| (n.x, n.y)).collect()
}
fn graph(g: &flowmaid::model::Graph, out: &mut String) {
    let sc = scene::scene(g);
    sc_all("scene", &sc, out);
    let _ = write!(out, "\tmermaid={:016x}", fnv(&flowmaid::to_mermaid(g)));
    let c0 = centers(&sc);
    let n = c0.len();
    if n == 0 { return; }
    let sizes: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.w, n.h)).collect();
    sc_all("sized", &scene::scene_sized(g, &sizes), out);
    let r0 = scene::route(g, &c0);
    sc_all("route0", &r0, out);
    let mut c1 = c0.clone(); c1[0].0 += 7.0; c1[0].1 += 5.0;
    let p1 = scene::route_partial(g, &c1, &sc, &c0);
    sc_all("part1", &p1, out);
    let mut c2 = c0.clone(); let m = n / 2; c2[m].0 -= 40.0; c2[m].1 += 23.0;
    let p2 = scene::route_partial(g, &c2, &sc, &c0);
    sc_all("part2", &p2, out);
    sc_all("route2", &scene::route(g, &c2), out);
    sc_all("chain", &scene::route_partial(g, &c2, &p1, &c1), out);
    let mut c3 = c0.clone(); let k = n / 3; c3[k].0 = -60.0; c3[k].1 = c3[k].1.min(20.0) - 30.0;
    sc_all("part3", &scene::route_partial(g, &c3, &sc, &c0), out);
    sc_all("route3", &scene::route(g, &c3), out);
    let t3 = scene::try_route(g, &c3);
    let _ = write!(out, "\ttry3={:016x}", t3.as_ref().map(|s| fnv(&scene::to_svg(s)) ^ dbg(s)).unwrap_or(1));
    sc_all("rsized", &scene::route_sized(g, &c2, &sizes), out);
}
fn main() {
    for path in std::env::args().skip(1) {
        let src = match std::fs::read(&path) { Ok(b) => String::from_utf8_lossy(&b).into_owned(), Err(_) => continue };
        let mut out = String::new();
        let _ = write!(out, "{path}");
        match flowmaid::render_svg(&src) {
            Ok(s) => { let _ = write!(out, "\trender={:016x}", fnv(&s)); }
            Err(e) => {
                let _ = write!(out, "\trender=ERR:{:016x}", fnv(&format!("{e:?}")));
                let a = flowmaid::render_advance_text_svg(&src);
                let _ = write!(out, "\tadvance={:016x}", fnv(&format!("{:?}", a)));
            }
        }
        let mut opts = flowmaid::SvgOptions::default(); opts.responsive = true; opts.transparent = true;
        if let Ok(s) = flowmaid::render_svg_advanced(&src, &opts) { let _ = write!(out, "\tresp={:016x}", fnv(&s)); }
        match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => graph(&g, &mut out),
            Ok(Document::Class(d)) => {
                let cs = flowmaid::class::scene(&d);
                let _ = write!(out, "\tclass.svg={:016x}\tclass.dbg={:016x}\tclass.hit={:016x}", fnv(&flowmaid::class::to_svg(&cs)), dbg(&cs), hits(&cs.scene));
                let mut c = centers(&cs.scene);
                if !c.is_empty() { c[0].0 -= 80.0; c[0].1 -= 40.0;
                    let r = flowmaid::class::route(&d, &c);
                    let _ = write!(out, "\tclassr.svg={:016x}\tclassr.dbg={:016x}\tclassr.hit={:016x}", fnv(&flowmaid::class::to_svg(&r)), dbg(&r), hits(&r.scene));
                    let t = flowmaid::class::try_route(&d, &c);
                    let _ = write!(out, "\tclasst={:016x}", t.as_ref().map(|s| fnv(&flowmaid::class::to_svg(s))).unwrap_or(1));
                }
            }
            Ok(Document::Er(d)) => {
                let es = flowmaid::er::scene(&d);
                let _ = write!(out, "\ter.svg={:016x}\ter.dbg={:016x}\ter.hit={:016x}", fnv(&flowmaid::er::to_svg(&es)), dbg(&es), hits(&es.scene));
                let mut c = centers(&es.scene);
                if !c.is_empty() { c[0].0 -= 80.0; c[0].1 -= 40.0;
                    let r = flowmaid::er::route(&d, &c);
                    let _ = write!(out, "\terr.svg={:016x}\terr.dbg={:016x}\terr.hit={:016x}", fnv(&flowmaid::er::to_svg(&r)), dbg(&r), hits(&r.scene));
                    let t = flowmaid::er::try_route(&d, &c);
                    let _ = write!(out, "\tert={:016x}", t.as_ref().map(|s| fnv(&flowmaid::er::to_svg(s))).unwrap_or(1));
                }
            }
            Ok(other) => { let _ = write!(out, "\tdoc={:016x}", dbg(&other)); }
            Err(_) => {}
        }
        println!("{out}");
    }
}
