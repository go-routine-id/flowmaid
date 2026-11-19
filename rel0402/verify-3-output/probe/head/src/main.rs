// Identity probe: for one input file, hash the Debug form and SVG of the
// existing scene/route/route_partial/hit_test results (APIs in origin/dev).
use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser, scene};
use std::fmt::Write as _;

fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}\t{}", s.len())
}
fn out(f: &str, k: &str, s: &str) {
    println!("{f}\t{k}\t{}", fnv(s));
}
fn moves(c: &[(f64, f64)]) -> Vec<(&'static str, Vec<(f64, f64)>)> {
    let mut v = Vec::new();
    v.push(("id", c.to_vec()));
    if c.is_empty() {
        return v;
    }
    let mut m1 = c.to_vec();
    let k = c.len() / 2;
    m1[k] = (c[k].0 + 37.0, c[k].1 + 23.0);
    v.push(("m1", m1));
    let mut m2 = c.to_vec();
    m2[0] = (c[0].0 - 400.0, c[0].1 - 300.0);
    v.push(("m2", m2));
    let m3: Vec<(f64, f64)> = c.iter().map(|p| (p.0 - 60.0, p.1 - 45.0)).collect();
    v.push(("m3", m3));
    let mut m4 = c.to_vec();
    let j = c.len() - 1;
    m4[j] = (c[j].0 + 900.0, c[j].1 + 700.0);
    v.push(("m4", m4));
    v
}
fn picks(f: &str, key: &str, sc: &scene::Scene) {
    let mut s = String::new();
    for n in &sc.nodes {
        let _ = write!(s, "{:?};{:?};", sc.hit_test(n.x, n.y, 4.0), sc.nearest_node(n.x + 3.0, n.y + 5.0));
    }
    for e in &sc.edges {
        let b = e.bezier;
        let m = ((b[0].0 + 3.0 * b[1].0 + 3.0 * b[2].0 + b[3].0) / 8.0, (b[0].1 + 3.0 * b[1].1 + 3.0 * b[2].1 + b[3].1) / 8.0);
        let _ = write!(s, "{:?};{:?};", sc.hit_test(m.0, m.1, 4.0), sc.edge_at(m.0 + 1.0, m.1, 3.0));
    }
    let _ = write!(s, "{:?}", sc.hits_in_rect(0.0, 0.0, sc.width / 2.0, sc.height / 2.0));
    out(f, key, &s);
}
fn flow(f: &str, tag: &str, g: &flowmaid::model::Graph) {
    let sc = scene::scene(g);
    out(f, &format!("{tag}_scene"), &format!("{sc:?}"));
    out(f, &format!("{tag}_scene_svg"), &scene::to_svg(&sc));
    picks(f, &format!("{tag}_scene_pick"), &sc);
    let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let big = sc.edges.len() > 400;
    let mut prev: Option<(scene::Scene, Vec<(f64, f64)>)> = None;
    for (name, p) in moves(&c) {
        if big && name != "id" && name != "m1" {
            continue;
        }
        let r = scene::route(g, &p);
        out(f, &format!("{tag}_route_{name}"), &format!("{r:?}"));
        out(f, &format!("{tag}_route_{name}_svg"), &scene::to_svg(&r));
        if name == "m1" || name == "m2" {
            picks(f, &format!("{tag}_route_{name}_pick"), &r);
        }
        if name != "id" {
            let rp = scene::route_partial(g, &p, &sc, &c);
            out(f, &format!("{tag}_rp_{name}"), &format!("{rp:?}"));
            out(f, &format!("{tag}_rp_{name}_svg"), &scene::to_svg(&rp));
            if let Some((pb, pc)) = &prev {
                let ch = scene::route_partial(g, &p, pb, pc);
                out(f, &format!("{tag}_rpchain_{name}"), &format!("{ch:?}"));
                out(f, &format!("{tag}_rpchain_{name}_svg"), &scene::to_svg(&ch));
            }
        }
        prev = Some((r, p));
    }
}
fn main() {
    let f = std::env::args().nth(1).unwrap();
    let Ok(src) = std::fs::read_to_string(&f) else { return };
    out(&f, "render_svg", &format!("{:?}", flowmaid::render_svg(&src)));
    let doc = match parser::parse_document(&src) {
        Ok(d) => d,
        Err(e) => {
            out(&f, "parse_err", &format!("{e:?}"));
            return;
        }
    };
    match doc {
        Document::Flowchart(g) => flow(&f, "fc", &g),
        Document::State(g) => flow(&f, "st", &g),
        Document::Er(d) => {
            let es = er::scene(&d);
            out(&f, "er_scene", &format!("{es:?}"));
            out(&f, "er_scene_svg", &er::to_svg(&es));
            picks(&f, "er_scene_pick", &es.scene);
            let c: Vec<(f64, f64)> = es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            for (name, p) in moves(&c) {
                let r = er::route(&d, &p);
                out(&f, &format!("er_route_{name}"), &format!("{r:?}"));
                out(&f, &format!("er_route_{name}_svg"), &er::to_svg(&r));
                picks(&f, &format!("er_route_{name}_pick"), &r.scene);
            }
        }
        Document::Class(d) => {
            let cs = class::scene(&d);
            out(&f, "cl_scene", &format!("{cs:?}"));
            out(&f, "cl_scene_svg", &class::to_svg(&cs));
            out(&f, "cl_scene_cards", &format!("{:?}", class::card_spots(&cs)));
            picks(&f, "cl_scene_pick", &cs.scene);
            let all: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            let ncls = d.classes.len();
            for (tag, c) in [("a", all.clone()), ("c", all[..ncls.min(all.len())].to_vec())] {
                if tag == "c" && c.len() == all.len() {
                    continue;
                }
                for (name, p) in moves(&c) {
                    let r = class::route(&d, &p);
                    out(&f, &format!("cl_route_{tag}_{name}"), &format!("{r:?}"));
                    out(&f, &format!("cl_route_{tag}_{name}_svg"), &class::to_svg(&r));
                    out(&f, &format!("cl_route_{tag}_{name}_cards"), &format!("{:?}", class::card_spots(&r)));
                    picks(&f, &format!("cl_route_{tag}_{name}_pick"), &r.scene);
                }
            }
        }
        Document::Mindmap(d) => {
            let ms = mindmap::scene(&d);
            out(&f, "mm_scene", &format!("{ms:?}"));
            out(&f, "mm_scene_svg", &mindmap::to_svg(&ms));
            let c: Vec<(f64, f64)> = ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
            for (name, p) in moves(&c) {
                let r = mindmap::route(&d, &p);
                out(&f, &format!("mm_route_{name}"), &format!("{r:?}"));
                out(&f, &format!("mm_route_{name}_svg"), &mindmap::to_svg(&r));
            }
        }
        Document::Sequence(d) => {
            let s = flowmaid::seq::scene(&d);
            out(&f, "seq_scene", &format!("{s:?}"));
            out(&f, "seq_svg", &flowmaid::seq::to_svg(&s));
        }
        Document::Pie(d) => {
            let s = flowmaid::pie::scene(&d);
            out(&f, "pie_scene", &format!("{s:?}"));
            out(&f, "pie_svg", &flowmaid::pie::to_svg(&s));
        }
        Document::Journey(d) => {
            let s = flowmaid::journey::scene(&d);
            out(&f, "jo_scene", &format!("{s:?}"));
            out(&f, "jo_svg", &flowmaid::journey::to_svg(&s));
        }
        Document::Timeline(d) => {
            let s = flowmaid::timeline::scene(&d);
            out(&f, "tl_scene", &format!("{s:?}"));
            out(&f, "tl_svg", &flowmaid::timeline::to_svg(&s));
        }
        Document::Sankey(d) => {
            let s = flowmaid::sankey::scene(&d);
            out(&f, "sk_scene", &format!("{s:?}"));
            out(&f, "sk_svg", &flowmaid::sankey::to_svg(&s));
        }
        Document::GitGraph(d) => {
            let s = flowmaid::gitgraph::scene(&d);
            out(&f, "git_scene", &format!("{s:?}"));
            out(&f, "git_svg", &flowmaid::gitgraph::to_svg(&s));
        }
        Document::Architecture(d) => {
            let s = flowmaid::architecture::scene(&d);
            out(&f, "arch_scene", &format!("{s:?}"));
            out(&f, "arch_svg", &flowmaid::architecture::to_svg(&s));
        }
        #[allow(unreachable_patterns)]
        _ => {}
    }
}
