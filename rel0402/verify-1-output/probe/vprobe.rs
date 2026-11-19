//! Verifier probe (identity of existing outputs): for each input file in
//! the list file argv[1], print one TSV row with FNV-1a hashes of
//! render_svg and of the existing scene/route results (Debug-formatted,
//! which prints every f64 exactly), so two builds can be compared.
use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser, scene};
use std::io::Write;

fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn moved(c: &[(f64, f64)], k: usize, dx: f64, dy: f64) -> Vec<(f64, f64)> {
    let mut m = c.to_vec();
    if let Some(p) = m.get_mut(k) {
        p.0 += dx;
        p.1 += dy;
    }
    m
}

fn main() {
    let list = std::fs::read_to_string(std::env::args().nth(1).expect("list")).unwrap();
    let full = std::env::args().nth(2).as_deref() == Some("full");
    let out = std::io::stdout();
    let mut out = out.lock();
    for path in list.lines().filter(|l| !l.is_empty()) {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        let r = std::panic::catch_unwind(|| {
            let mut cols: Vec<String> = Vec::new();
            match flowmaid::render_svg(&src) {
                Ok(s) => cols.push(fnv(&s)),
                Err(e) => cols.push(format!("ERR:{}", fnv(&format!("{e:?}")))),
            }
            let Ok(doc) = parser::parse_document(&src) else {
                return ("parse-err".to_string(), cols);
            };
            let kind;
            match doc {
                Document::Flowchart(g) | Document::State(g) => {
                    kind = "flow";
                    let sc = scene::scene(&g);
                    cols.push(fnv(&format!("{sc:?}")));
                    if full && g.nodes.len() <= 400 {
                        let c: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
                        let r0 = scene::route(&g, &c);
                        cols.push(fnv(&format!("{r0:?}")));
                        let k = c.len() / 2;
                        let m = moved(&c, k, 37.0, 11.0);
                        let r1 = scene::route(&g, &m);
                        cols.push(fnv(&format!("{r1:?}")));
                        let p = scene::route_partial(&g, &m, &sc, &c);
                        cols.push(fnv(&format!("{p:?}")));
                        cols.push(fnv(&scene::to_svg(&p)));
                        // a drag past the top-left (#76)
                        let m2 = moved(&c, 0, -400.0, -300.0);
                        let r2 = scene::route(&g, &m2);
                        cols.push(fnv(&format!("{r2:?}")));
                        cols.push(fnv(&scene::to_svg(&r2)));
                        if let Some(n) = sc.nodes.get(k) {
                            cols.push(format!("{:?}", sc.hit_test(n.x, n.y, 4.0)));
                        }
                    }
                }
                Document::Er(d) => {
                    kind = "er";
                    let es = er::scene(&d);
                    cols.push(fnv(&format!("{es:?}")));
                    if full {
                        let c: Vec<(f64, f64)> =
                            es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                        let r0 = er::route(&d, &c);
                        cols.push(fnv(&format!("{r0:?}")));
                        let m = moved(&c, c.len() / 2, 37.0, 11.0);
                        let r1 = er::route(&d, &m);
                        cols.push(fnv(&format!("{r1:?}")));
                        cols.push(fnv(&er::to_svg(&r1)));
                        let m2 = moved(&c, 0, -400.0, -300.0);
                        let r2 = er::route(&d, &m2);
                        cols.push(fnv(&format!("{r2:?}")));
                        cols.push(fnv(&er::to_svg(&r2)));
                        cols.push(format!("{}", er::try_route(&d, &m).is_some()));
                    }
                }
                Document::Class(d) => {
                    kind = "class";
                    let cs = class::scene(&d);
                    cols.push(fnv(&format!("{cs:?}")));
                    if full {
                        let c: Vec<(f64, f64)> =
                            cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                        let r0 = class::route(&d, &c);
                        cols.push(fnv(&format!("{r0:?}")));
                        let k = d.classes.len() / 2;
                        let m = moved(&c, k, 37.0, 11.0);
                        let r1 = class::route(&d, &m);
                        cols.push(fnv(&format!("{r1:?}")));
                        cols.push(fnv(&class::to_svg(&r1)));
                        cols.push(fnv(&format!("{:?}", class::card_spots(&r1))));
                        let cl = &c[..d.classes.len()];
                        let r3 = class::route(&d, &moved(cl, k, 37.0, 11.0));
                        cols.push(fnv(&format!("{r3:?}")));
                        let m2 = moved(&c, 0, -400.0, -300.0);
                        let r2 = class::route(&d, &m2);
                        cols.push(fnv(&format!("{r2:?}")));
                        cols.push(fnv(&class::to_svg(&r2)));
                    }
                }
                Document::Mindmap(d) => {
                    kind = "mind";
                    let ms = mindmap::scene(&d);
                    cols.push(fnv(&format!("{ms:?}")));
                    if full {
                        let c: Vec<(f64, f64)> = ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                        let r0 = mindmap::route(&d, &c);
                        cols.push(fnv(&format!("{r0:?}")));
                        let m = moved(&c, c.len() / 2, 37.0, 11.0);
                        let r1 = mindmap::route(&d, &m);
                        cols.push(fnv(&format!("{r1:?}")));
                        cols.push(fnv(&mindmap::to_svg(&r1)));
                    }
                }
                Document::Sequence(d) => {
                    kind = "seq";
                    cols.push(fnv(&format!("{:?}", flowmaid::seq::scene(&d))));
                }
                Document::Pie(d) => {
                    kind = "pie";
                    cols.push(fnv(&format!("{:?}", flowmaid::pie::scene(&d))));
                }
                Document::Journey(d) => {
                    kind = "journey";
                    cols.push(fnv(&format!("{:?}", flowmaid::journey::scene(&d))));
                }
                Document::Timeline(d) => {
                    kind = "timeline";
                    cols.push(fnv(&format!("{:?}", flowmaid::timeline::scene(&d))));
                }
                Document::Sankey(d) => {
                    kind = "sankey";
                    cols.push(fnv(&format!("{:?}", flowmaid::sankey::scene(&d))));
                }
                Document::GitGraph(_) => kind = "git",
                Document::Architecture(_) => kind = "arch",
            }
            (kind.to_string(), cols)
        });
        match r {
            Ok((k, cols)) => writeln!(out, "{path}\t{k}\t{}", cols.join("\t")).unwrap(),
            Err(_) => writeln!(out, "{path}\tPANIC").unwrap(),
        }
        out.flush().ok();
    }
}
