//! Output-identity probe (verify round 2): only APIs present on origin/dev.
//! usage: idprobe <listfile>  -> stdout TSV: file \t item \t hash \t len
use flowmaid::{class, er, mindmap, model::Document, parser::parse_document, scene};
use std::io::{BufRead, Write};
use std::time::Instant;

fn h(s: &str) -> String {
    let mut x: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        x ^= b as u64;
        x = x.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}\t{}", x, s.len())
}

fn main() {
    let list = std::env::args().nth(1).expect("list");
    let f = std::fs::File::open(list).unwrap();
    let out = std::io::stdout();
    let mut o = std::io::BufWriter::new(out.lock());
    for line in std::io::BufReader::new(f).lines() {
        let path = line.unwrap();
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let t0 = Instant::now();
        let mut rows: Vec<(String, String)> = Vec::new();
        let r = std::panic::catch_unwind(|| {
            let mut rows: Vec<(String, String)> = Vec::new();
            match flowmaid::render_svg(&src) {
                Ok(s) => rows.push(("svg".into(), h(&s))),
                Err(e) => rows.push(("svg".into(), h(&format!("ERR {e:?}")))),
            }
            let Ok(doc) = parse_document(&src) else { return rows };
            match doc {
                Document::Flowchart(g) | Document::State(g) => {
                    let s = scene::scene(&g);
                    rows.push(("fc_scene".into(), h(&format!("{s:?}"))));
                    rows.push(("fc_scene_svg".into(), h(&scene::to_svg(&s))));
                    if g.nodes.len() <= 250 && !g.nodes.is_empty() {
                        let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
                        let r = scene::route(&g, &c);
                        rows.push(("fc_route".into(), h(&format!("{r:?}"))));
                        rows.push(("fc_route_svg".into(), h(&scene::to_svg(&r))));
                        let mut m = c.clone();
                        m[0].0 += 37.0;
                        m[0].1 += 23.0;
                        let rm = scene::route(&g, &m);
                        rows.push(("fc_route_mv".into(), h(&format!("{rm:?}"))));
                        let rp = scene::route_partial(&g, &m, &s, &c);
                        rows.push(("fc_rp_mv".into(), h(&format!("{rp:?}"))));
                        let mut neg = c.clone();
                        neg[0] = (-60.0, -45.0);
                        let rn = scene::route(&g, &neg);
                        rows.push(("fc_route_neg".into(), h(&format!("{rn:?}"))));
                        rows.push(("fc_route_neg_svg".into(), h(&scene::to_svg(&rn))));
                        let rpn = scene::route_partial(&g, &neg, &r, &c);
                        rows.push(("fc_rp_neg".into(), h(&format!("{rpn:?}"))));
                    }
                }
                Document::Er(d) => {
                    let s = er::scene(&d);
                    rows.push(("er_scene".into(), h(&format!("{s:?}"))));
                    rows.push(("er_scene_svg".into(), h(&er::to_svg(&s))));
                    if !d.entities.is_empty() {
                        let c: Vec<(f64, f64)> = s.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                        let r = er::route(&d, &c);
                        rows.push(("er_route".into(), h(&format!("{r:?}"))));
                        rows.push(("er_route_svg".into(), h(&er::to_svg(&r))));
                        let mut m = c.clone();
                        m[0].0 += 37.0;
                        m[0].1 += 23.0;
                        let rm = er::route(&d, &m);
                        rows.push(("er_route_mv".into(), h(&format!("{rm:?}"))));
                        rows.push(("er_route_mv_svg".into(), h(&er::to_svg(&rm))));
                        let mut neg = c.clone();
                        neg[0] = (-60.0, -45.0);
                        let rn = er::route(&d, &neg);
                        rows.push(("er_route_neg".into(), h(&format!("{rn:?}"))));
                        rows.push(("er_route_neg_svg".into(), h(&er::to_svg(&rn))));
                    }
                }
                Document::Class(d) => {
                    let s = class::scene(&d);
                    rows.push(("cl_scene".into(), h(&format!("{s:?}"))));
                    rows.push(("cl_scene_svg".into(), h(&class::to_svg(&s))));
                    rows.push(("cl_scene_cards".into(), h(&format!("{:?}", class::card_spots(&s)))));
                    if !d.classes.is_empty() {
                        let all: Vec<(f64, f64)> = s.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                        let c = all[..d.classes.len()].to_vec();
                        for (tag, cc) in [("cls", c.clone()), ("all", all.clone())] {
                            let r = class::route(&d, &cc);
                            rows.push((format!("cl_route_{tag}"), h(&format!("{r:?}"))));
                            rows.push((format!("cl_route_{tag}_svg"), h(&class::to_svg(&r))));
                            rows.push((format!("cl_route_{tag}_cards"), h(&format!("{:?}", class::card_spots(&r)))));
                            let mut m = cc.clone();
                            m[0].0 += 37.0;
                            m[0].1 += 23.0;
                            let rm = class::route(&d, &m);
                            rows.push((format!("cl_route_mv_{tag}"), h(&format!("{rm:?}"))));
                            rows.push((format!("cl_route_mv_{tag}_svg"), h(&class::to_svg(&rm))));
                            let mut neg = cc.clone();
                            neg[0] = (-60.0, -45.0);
                            let rn = class::route(&d, &neg);
                            rows.push((format!("cl_route_neg_{tag}"), h(&format!("{rn:?}"))));
                            rows.push((format!("cl_route_neg_{tag}_svg"), h(&class::to_svg(&rn))));
                        }
                    }
                }
                Document::Mindmap(d) => {
                    let s = mindmap::scene(&d);
                    rows.push(("mm_scene".into(), h(&format!("{s:?}"))));
                    rows.push(("mm_scene_svg".into(), h(&mindmap::to_svg(&s))));
                    if !d.nodes.is_empty() {
                        let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                        let r = mindmap::route(&d, &c);
                        rows.push(("mm_route".into(), h(&format!("{r:?}"))));
                        let mut m = c.clone();
                        m[0].0 += 37.0;
                        m[0].1 += 23.0;
                        let rm = mindmap::route(&d, &m);
                        rows.push(("mm_route_mv".into(), h(&format!("{rm:?}"))));
                        rows.push(("mm_route_mv_svg".into(), h(&mindmap::to_svg(&rm))));
                    }
                }
                Document::Sequence(d) => {
                    let s = flowmaid::seq::scene(&d);
                    rows.push(("seq_scene".into(), h(&format!("{s:?}"))));
                }
                Document::Pie(d) => {
                    let s = flowmaid::pie::scene(&d);
                    rows.push(("pie_scene".into(), h(&format!("{s:?}"))));
                }
                Document::Journey(d) => {
                    let s = flowmaid::journey::scene(&d);
                    rows.push(("jo_scene".into(), h(&format!("{s:?}"))));
                }
                Document::Timeline(d) => {
                    let s = flowmaid::timeline::scene(&d);
                    rows.push(("tl_scene".into(), h(&format!("{s:?}"))));
                }
                Document::Sankey(d) => {
                    let s = flowmaid::sankey::scene(&d);
                    rows.push(("sk_scene".into(), h(&format!("{s:?}"))));
                }
                Document::GitGraph(d) => {
                    let s = flowmaid::gitgraph::scene(&d);
                    rows.push(("gg_scene".into(), h(&format!("{s:?}"))));
                }
                Document::Architecture(d) => {
                    let s = flowmaid::architecture::scene(&d);
                    rows.push(("ar_scene".into(), h(&format!("{s:?}"))));
                }
            }
            rows
        });
        match r {
            Ok(v) => rows = v,
            Err(_) => rows.push(("PANIC".into(), h("panic"))),
        }
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        for (item, hv) in rows {
            writeln!(o, "{path}\t{item}\t{hv}").unwrap();
        }
        writeln!(o, "{path}\t_ms\t{ms:.1}").unwrap();
        o.flush().unwrap();
    }
}
