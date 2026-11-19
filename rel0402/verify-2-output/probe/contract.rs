//! Contract + determinism probe for #111 route_partial (verify round 2, HEAD only).
//! usage: contract <listfile> [maxnodes]  -> stdout: one line per finding, summary on stderr
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::scene::Scene;
use flowmaid::{class, er, mindmap};
use std::io::BufRead;
use std::time::Instant;

fn dbg<T: std::fmt::Debug>(t: &T) -> String {
    format!("{t:?}")
}

/// Every drawn point of a routed scene inside its canvas (1e-6 slack)?
fn canvas_holds(sc: &Scene) -> Option<String> {
    let (ox, oy) = sc.origin;
    let (x1, y1) = (ox + sc.width, oy + sc.height);
    let inside = |x: f64, y: f64| x >= ox - 1e-6 && x <= x1 + 1e-6 && y >= oy - 1e-6 && y <= y1 + 1e-6;
    for n in &sc.nodes {
        for (x, y) in [(n.x - n.w / 2.0, n.y - n.h / 2.0), (n.x + n.w / 2.0, n.y + n.h / 2.0)] {
            if !inside(x, y) {
                return Some(format!("node {} corner ({x:.1},{y:.1}) off canvas {:?} {}x{}", n.id, sc.origin, sc.width, sc.height));
            }
        }
    }
    for e in &sc.edges {
        for &(x, y) in e.waypoints.iter() {
            if !inside(x, y) {
                return Some(format!("edge {}->{} waypoint ({x:.1},{y:.1}) off canvas {:?} {}x{}", e.from, e.to, sc.origin, sc.width, sc.height));
            }
        }
        if e.waypoints.is_empty() {
            for &(x, y) in e.bezier.iter().take(4).step_by(3) {
                if !inside(x, y) {
                    return Some(format!("edge {}->{} end ({x:.1},{y:.1}) off canvas", e.from, e.to));
                }
            }
        }
    }
    None
}

fn picks(n: usize, max: usize) -> Vec<usize> {
    if n <= max {
        (0..n).collect()
    } else {
        (0..max).map(|k| k * (n - 1) / (max - 1)).collect()
    }
}

const MOVES: [(f64, f64); 4] = [(37.0, 23.0), (900.0, 500.0), (-400.0, -300.0), (-3.0, 260.0)];

fn main() {
    let mut args = std::env::args().skip(1);
    let list = args.next().expect("list");
    let max: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(8);
    let mut stats = std::collections::BTreeMap::<String, usize>::new();
    let mut bump = |k: &str| *stats.entry(k.to_string()).or_default() += 1;
    let t0 = Instant::now();
    for line in std::io::BufReader::new(std::fs::File::open(list).unwrap()).lines() {
        let path = line.unwrap();
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let Ok(doc) = parse_document(&src) else { continue };
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut out: Vec<String> = Vec::new();
            match doc {
                Document::Er(d) => {
                    if d.entities.is_empty() { return (out, "er_empty"); }
                    let laid = er::scene(&d);
                    let at: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    let routed = er::route(&d, &at);
                    for (bn, base) in [("laid", &laid), ("routed", &routed)] {
                        for i in picks(at.len(), max) {
                            for mv in MOVES {
                                let mut to = at.clone();
                                to[i] = (at[i].0 + mv.0, at[i].1 + mv.1);
                                let f = er::route_partial(&d, &to, base, &at);
                                let f2 = er::route_partial(&d, &to, base, &at);
                                if dbg(&f) != dbg(&f2) { out.push(format!("NONDET er {bn} i={i} mv={mv:?}")); }
                                let tf = er::try_route_partial(&d, &to, base, &at);
                                if tf.map(|t| dbg(&t)) != Some(dbg(&f)) { out.push(format!("TRY_MISMATCH er {bn} i={i}")); }
                                for (k, n) in f.scene.nodes.iter().enumerate() {
                                    if (n.x, n.y) != to[k] { out.push(format!("POS er {bn} i={i} node {k}")); }
                                }
                                if dbg(&f.tables) != dbg(&base.tables) { out.push(format!("TABLES er {bn} i={i}")); }
                                if dbg(&f.cards) != dbg(&base.cards) { out.push(format!("CARDS er {bn} i={i}")); }
                                for (j, r) in d.relations.iter().enumerate() {
                                    let (a, b) = (&f.scene.edges[j], &base.scene.edges[j]);
                                    if r.from != i && r.to != i {
                                        if dbg(a) != dbg(b) { out.push(format!("KEPT_CHANGED er {bn} i={i} mv={mv:?} rel {j}")); }
                                    } else if a.from != d.entities[r.from].name || a.to != d.entities[r.to].name {
                                        out.push(format!("ENDS er {bn} i={i} rel {j}: {}->{}", a.from, a.to));
                                    }
                                }
                                if let Some(m) = canvas_holds(&f.scene) { out.push(format!("CANVAS er {bn} i={i} mv={mv:?}: {m}")); }
                                let svg = er::to_svg(&f);
                                if !svg.contains("<svg") { out.push("SVG er".into()); }
                                // chained frame: move again from f
                                let mut to2 = to.clone();
                                to2[i].0 += 13.0;
                                let g = er::route_partial(&d, &to2, &f, &to);
                                for (j, r) in d.relations.iter().enumerate() {
                                    if r.from != i && r.to != i && dbg(&g.scene.edges[j]) != dbg(&f.scene.edges[j]) {
                                        out.push(format!("CHAIN_KEPT_CHANGED er {bn} i={i} rel {j}"));
                                    }
                                }
                            }
                        }
                    }
                    // fallback: base not at base_centers -> route
                    let mut off = at.clone();
                    off[0].0 += 5.0;
                    let mut to = at.clone();
                    to[0].1 += 9.0;
                    if dbg(&er::route_partial(&d, &to, &laid, &off)) != dbg(&er::route(&d, &to)) { out.push("FALLBACK er off".into()); }
                    (out, "er")
                }
                Document::Class(d) => {
                    if d.classes.is_empty() { return (out, "cl_empty"); }
                    let laid = class::scene(&d);
                    let all: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    let nc = d.classes.len();
                    let mut variants = vec![("all", all.clone())];
                    if all.len() != nc { variants.push(("cls", all[..nc].to_vec())); }
                    for (vn, at) in variants {
                        let routed = class::route(&d, &at);
                        let at_r: Vec<(f64, f64)> = routed.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                        for (bn, base, bat) in [("laid", &laid, &all), ("routed", &routed, &at_r)] {
                            let bc = class::card_spots(base);
                            for i in picks(at.len(), max) {
                                for mv in MOVES {
                                    let mut to = at.clone();
                                    to[i] = (at[i].0 + mv.0, at[i].1 + mv.1);
                                    // base_centers = what produced base (all positions)
                                    let bcent: Vec<(f64, f64)> = if at.len() == bat.len() { bat.clone() } else { bat[..at.len()].to_vec() };
                                    let f = class::route_partial(&d, &to, base, &bcent);
                                    let f2 = class::route_partial(&d, &to, base, &bcent);
                                    if dbg(&f) != dbg(&f2) { out.push(format!("NONDET cl {vn} {bn} i={i} mv={mv:?}")); }
                                    if class::to_svg(&f) != class::to_svg(&f2) { out.push(format!("NONDET_SVG cl {vn} {bn} i={i}")); }
                                    let tf = class::try_route_partial(&d, &to, base, &bcent);
                                    if tf.map(|t| dbg(&t)) != Some(dbg(&f)) { out.push(format!("TRY_MISMATCH cl {vn} {bn} i={i}")); }
                                    for (k, &p) in to.iter().enumerate() {
                                        let n = &f.scene.nodes[k];
                                        if (n.x, n.y) != p { out.push(format!("POS cl {vn} {bn} i={i} node {k}")); }
                                    }
                                    // which graph nodes moved: compare frame nodes vs base nodes
                                    let moved: Vec<bool> = f.scene.nodes.iter().zip(&base.scene.nodes).map(|(a, b)| (a.x, a.y) != (b.x, b.y)).collect();
                                    let fc = class::card_spots(&f);
                                    let mut kept_fail: Vec<String> = Vec::new();
                                    let idx: std::collections::HashMap<&str, usize> = f.scene.nodes.iter().enumerate().map(|(k, n)| (n.id.as_str(), k)).collect();
                                    for (j, (a, b)) in f.scene.edges.iter().zip(&base.scene.edges).enumerate() {
                                        let (Some(&p), Some(&q)) = (idx.get(a.from.as_str()), idx.get(a.to.as_str())) else { out.push(format!("EDGE_IDS cl {j}")); continue };
                                        let attached = moved[p] || moved[q];
                                        if !attached {
                                            if dbg(a) != dbg(b) { kept_fail.push(format!("KEPT_CHANGED cl {vn} {bn} i={i} mv={mv:?} edge {j}")); }
                                            if j < d.relations.len() && fc[j] != bc[j] { kept_fail.push(format!("KEPT_CARDS cl {vn} {bn} i={i} mv={mv:?} rel {j} {:?} vs {:?}", fc[j], bc[j])); }
                                        }
                                    }
                                    if !kept_fail.is_empty() {
                                        if dbg(&f) == dbg(&class::route(&d, &to)) {
                                            out.push(format!("FELLBACK cl {vn} {bn} i={i} mv={mv:?}"));
                                        } else {
                                            out.extend(kept_fail);
                                        }
                                    }
                                    if let Some(m) = canvas_holds(&f.scene) { out.push(format!("CANVAS cl {vn} {bn} i={i} mv={mv:?}: {m}")); }
                                    // chained
                                    let fat: Vec<(f64, f64)> = f.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                                    let mut to2 = fat.clone();
                                    to2[i].0 += 13.0;
                                    let g = class::route_partial(&d, &to2, &f, &fat);
                                    let gc = class::card_spots(&g);
                                    for (j, (a, b)) in g.scene.edges.iter().zip(&f.scene.edges).enumerate() {
                                        let (Some(&p), Some(&q)) = (idx.get(a.from.as_str()), idx.get(a.to.as_str())) else { continue };
                                        if p != i && q != i {
                                            if dbg(a) != dbg(b) { out.push(format!("CHAIN_KEPT_CHANGED cl {vn} {bn} i={i} edge {j}")); }
                                            if j < d.relations.len() && gc[j] != fc[j] { out.push(format!("CHAIN_KEPT_CARDS cl {vn} {bn} i={i} rel {j}")); }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let mut off = all.clone();
                    off[0].0 += 5.0;
                    let mut to = all.clone();
                    to[0].1 += 9.0;
                    if dbg(&class::route_partial(&d, &to, &laid, &off)) != dbg(&class::route(&d, &to)) { out.push("FALLBACK cl off".into()); }
                    (out, "cl")
                }
                Document::Mindmap(d) => {
                    if d.nodes.is_empty() { return (out, "mm_empty"); }
                    let laid = mindmap::scene(&d);
                    let at: Vec<(f64, f64)> = laid.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                    let ends = mindmap::link_ends(&d);
                    if ends.len() != laid.edges.len() { out.push("LINK_ENDS len".into()); }
                    for i in picks(at.len(), max) {
                        for mv in MOVES {
                            let mut to = at.clone();
                            to[i] = (at[i].0 + mv.0, at[i].1 + mv.1);
                            let f = mindmap::route_partial(&d, &to, &laid, &at);
                            let full = mindmap::route(&d, &to);
                            if mindmap::to_svg(&f) != mindmap::to_svg(&full) { out.push(format!("MM_SVG_NE_ROUTE i={i} mv={mv:?}")); }
                            if dbg(&f) != dbg(&full) { out.push(format!("MM_DBG_NE_ROUTE i={i} mv={mv:?}")); }
                            for (j, &(p, c)) in ends.iter().enumerate() {
                                if p != i && c != i && dbg(&f.edges[j]) != dbg(&laid.edges[j]) { out.push(format!("KEPT_CHANGED mm i={i} link {j}")); }
                            }
                        }
                    }
                    (out, "mm")
                }
                _ => (out, "other"),
            }
        }));
        match r {
            Ok((out, kind)) => {
                bump(kind);
                for o in &out {
                    let k = o.split_whitespace().next().unwrap_or("").to_string();
                    bump(&k);
                    println!("{path}\t{o}");
                }
            }
            Err(_) => {
                bump("PANIC");
                println!("{path}\tPANIC");
            }
        }
    }
    eprintln!("elapsed {:.1}s", t0.elapsed().as_secs_f64());
    for (k, v) in stats {
        eprintln!("{k}\t{v}");
    }
}
