//! B12 probe (scratch only): classify inputs by subgraph use and report
//! how scene() / route() draw clustered diagrams.
//!
//! b12probe summary <files...>   one TSV row per file
//! b12probe detail <file>        per-frame / per-edge report

use flowmaid::model::{End, Graph};
use flowmaid::scene::{route, route_partial, scene, Scene, SceneEdge};
use flowmaid::Document;

fn load(path: &str) -> Option<(Graph, &'static str)> {
    let src = std::fs::read_to_string(path).ok()?;
    match flowmaid::parser::parse_document(&src).ok()? {
        Document::Flowchart(g) => Some((g, "flow")),
        Document::State(g) => Some((g, "state")),
        _ => None,
    }
}

fn chains(g: &Graph) -> Vec<Vec<usize>> {
    let mut owner = vec![None; g.nodes.len()];
    for (si, s) in g.subgraphs.iter().enumerate() {
        for &v in &s.nodes {
            owner[v] = Some(si);
        }
    }
    (0..g.nodes.len())
        .map(|v| {
            let mut p = Vec::new();
            let mut cur = owner[v];
            while let Some(s) = cur {
                p.push(s);
                cur = g.subgraphs[s].parent;
            }
            p.reverse();
            p
        })
        .collect()
}

fn poly(e: &SceneEdge) -> Vec<(f64, f64)> {
    if e.waypoints.len() >= 2 {
        e.waypoints.clone()
    } else {
        let b = e.bezier;
        (0..=24)
            .map(|i| {
                let t = i as f64 / 24.0;
                let u = 1.0 - t;
                let f = |a: f64, b1: f64, c: f64, d: f64| {
                    u * u * u * a + 3.0 * u * u * t * b1 + 3.0 * u * t * t * c + t * t * t * d
                };
                (
                    f(b[0].0, b[1].0, b[2].0, b[3].0),
                    f(b[0].1, b[1].1, b[2].1, b[3].1),
                )
            })
            .collect()
    }
}

fn seg_in_rect(a: (f64, f64), b: (f64, f64), r: (f64, f64, f64, f64)) -> bool {
    // sample
    (0..=16).any(|i| {
        let t = i as f64 / 16.0;
        let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        p.0 > r.0 + 1.0 && p.0 < r.2 - 1.0 && p.1 > r.1 + 1.0 && p.1 < r.3 - 1.0
    })
}

struct Info {
    cross_edges: usize,
    sub_edges: usize,
    dir_subs: usize,
    nested: usize,
    in_sub: usize,
    cyclic_blocks: bool,
}

fn info(g: &Graph) -> Info {
    let ch = chains(g);
    let cross = g
        .edges
        .iter()
        .filter(|e| e.from != e.to && ch[e.from] != ch[e.to])
        .count();
    // Quotient graph on top-level units (top-level subgraph or free node).
    let unit = |v: usize| ch[v].first().map_or(g.subgraphs.len() + v, |&s| s);
    let n = g.subgraphs.len() + g.nodes.len();
    let mut adj = vec![Vec::new(); n];
    for e in &g.edges {
        let (a, b) = (unit(e.from), unit(e.to));
        if a != b {
            adj[a].push(b);
        }
    }
    let top = |s: usize| {
        let mut s = s;
        while let Some(p) = g.subgraphs[s].parent {
            s = p;
        }
        s
    };
    for e in &g.sub_edges {
        let u = |end: End| match end {
            End::Node(v) => unit(v),
            End::Sub(s) => top(s),
        };
        let (a, b) = (u(e.from), u(e.to));
        if a != b {
            adj[a].push(b);
        }
    }
    // A cycle through at least one top-level subgraph unit: some
    // subgraph unit reaches itself.
    let ns = g.subgraphs.len();
    let mut cyc = false;
    for s in (0..ns).filter(|&s| g.subgraphs[s].parent.is_none()) {
        let mut seen = vec![false; n];
        let mut stack: Vec<usize> = adj[s].clone();
        while let Some(u) = stack.pop() {
            if u == s {
                cyc = true;
                break;
            }
            if std::mem::replace(&mut seen[u], true) {
                continue;
            }
            stack.extend(adj[u].iter().copied());
        }
    }
    Info {
        cross_edges: cross,
        sub_edges: g.sub_edges.len(),
        dir_subs: g.subgraphs.iter().filter(|s| s.direction.is_some()).count(),
        nested: g.subgraphs.iter().filter(|s| s.parent.is_some()).count(),
        in_sub: ch.iter().filter(|p| !p.is_empty()).count(),
        cyclic_blocks: cyc,
    }
}

fn frames_match(a: &Scene, b: &Scene) -> f64 {
    let mut worst: f64 = 0.0;
    for c in &a.clusters {
        match b.clusters.iter().find(|d| d.id == c.id) {
            Some(d) => {
                for (x, y) in [(c.x, d.x), (c.y, d.y), (c.w, d.w), (c.h, d.h)] {
                    worst = worst.max((x - y).abs());
                }
            }
            None => worst = f64::INFINITY,
        }
    }
    worst
}

/// Frames holding a non-member node centre, and overlapping sibling frames.
fn frame_faults(g: &Graph, sc: &Scene) -> (usize, usize) {
    let ch = chains(g);
    let mut foreign = 0;
    for c in &sc.clusters {
        let Some(si) = g.subgraphs.iter().position(|s| s.id == c.id) else {
            continue;
        };
        for (v, n) in sc.nodes.iter().enumerate() {
            if !ch[v].contains(&si) && n.x > c.x && n.x < c.x + c.w && n.y > c.y && n.y < c.y + c.h
            {
                foreign += 1;
            }
        }
    }
    let mut overlap = 0;
    for (i, a) in sc.clusters.iter().enumerate() {
        for b in sc.clusters.iter().skip(i + 1) {
            let (Some(sa), Some(sb)) = (
                g.subgraphs.iter().position(|s| s.id == a.id),
                g.subgraphs.iter().position(|s| s.id == b.id),
            ) else {
                continue;
            };
            if g.subgraphs[sa].parent != g.subgraphs[sb].parent {
                continue;
            }
            if a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h {
                overlap += 1;
            }
        }
    }
    (foreign, overlap)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("summary") => {
            println!("file\tkind\tnodes\tedges\tsubgraphs\tnested\tin_sub\tcross_edges\tsub_edges\tdir_subs\tblock_cycle\tframe_dmax(scene-vs-route)\tforeign_in_frame\tsibling_overlap\tw\th");
            for f in &args[1..] {
                let Some((g, kind)) = load(f) else { continue };
                let i = info(&g);
                let name = std::path::Path::new(f)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                if g.subgraphs.is_empty() {
                    println!(
                        "{name}\t{kind}\t{}\t{}\t0\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-",
                        g.nodes.len(),
                        g.edges.len()
                    );
                    continue;
                }
                let sc = scene(&g);
                let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
                let r = route(&g, &at);
                let (foreign, overlap) = frame_faults(&g, &sc);
                println!(
                    "{name}\t{kind}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.1}\t{}\t{}\t{:.0}\t{:.0}",
                    g.nodes.len(),
                    g.edges.len(),
                    g.subgraphs.len(),
                    i.nested,
                    i.in_sub,
                    i.cross_edges,
                    i.sub_edges,
                    i.dir_subs,
                    i.cyclic_blocks,
                    frames_match(&sc, &r),
                    foreign,
                    overlap,
                    sc.width,
                    sc.height
                );
            }
        }
        Some("detail") => {
            let (g, kind) = load(&args[1]).expect("flowchart or state diagram");
            let ch = chains(&g);
            let sc = scene(&g);
            let at: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &at);
            println!("kind={kind} canvas={:.0}x{:.0}", sc.width, sc.height);
            println!("== frames: id depth x y w h | route() frame dmax");
            for c in &sc.clusters {
                let d = r.clusters.iter().find(|d| d.id == c.id).unwrap();
                println!(
                    "{:<6} d{} x={:.0} y={:.0} w={:.0} h={:.0} | route: x={:.0} y={:.0} w={:.0} h={:.0}",
                    c.id, c.depth, c.x, c.y, c.w, c.h, d.x, d.y, d.w, d.h
                );
            }
            let (foreign, overlap) = frame_faults(&g, &sc);
            println!("foreign node centres inside frames: {foreign}; sibling frame overlaps: {overlap}");
            println!("== edges: from->to | len | lateral(|dx|) | along(|dy|) | foreign frames crossed | label dist to nearer end / path len");
            let mut tot_lat = 0.0;
            let mut tot_foreign = 0;
            for (ei, e) in sc.edges.iter().enumerate() {
                let p = poly(e);
                let (mut lat, mut along, mut len) = (0.0, 0.0, 0.0);
                for w in p.windows(2) {
                    lat += (w[1].0 - w[0].0).abs();
                    along += (w[1].1 - w[0].1).abs();
                    len += (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
                }
                let ends: Vec<usize> = if ei < g.edges.len() {
                    vec![g.edges[ei].from, g.edges[ei].to]
                } else {
                    Vec::new()
                };
                let mut crossed = Vec::new();
                for c in &sc.clusters {
                    let Some(si) = g.subgraphs.iter().position(|s| s.id == c.id) else {
                        continue;
                    };
                    if ends.iter().any(|&v| ch[v].contains(&si)) || c.id == e.from || c.id == e.to
                    {
                        continue;
                    }
                    let r = (c.x, c.y, c.x + c.w, c.y + c.h);
                    if p.windows(2).any(|w| seg_in_rect(w[0], w[1], r)) {
                        crossed.push(c.id.clone());
                    }
                }
                let lbl = e.label.as_ref().map(|(_, m, _)| {
                    let d0 = (m.0 - p[0].0).hypot(m.1 - p[0].1);
                    let d1 = (m.0 - p[p.len() - 1].0).hypot(m.1 - p[p.len() - 1].1);
                    d0.min(d1)
                });
                tot_lat += lat;
                tot_foreign += crossed.len();
                println!(
                    "{:<7}->{:<7} len={:>6.0} lat={:>6.0} along={:>5.0} foreign={:?} label={}",
                    e.from,
                    e.to,
                    len,
                    lat,
                    along,
                    crossed,
                    lbl.map_or("-".to_string(), |d| format!("{d:.0}"))
                );
            }
            println!("total lateral run {tot_lat:.0}px; foreign frame crossings {tot_foreign}");
            // A 1 px drag of each node: is the frame of route_partial that of route()?
            let mut fallback_like = 0;
            for v in 0..g.nodes.len() {
                let mut c = at.clone();
                c[v].0 += 1.0;
                let p = route_partial(&g, &c, &sc, &at);
                let full = route(&g, &c);
                // identical edge geometry to a full route() on every edge = it fell back
                let same = p
                    .edges
                    .iter()
                    .zip(&full.edges)
                    .all(|(a, b)| a.waypoints == b.waypoints && a.bezier == b.bezier);
                if same {
                    fallback_like += 1;
                }
            }
            println!(
                "1px drags whose route_partial equals a full route() on every edge: {fallback_like}/{}",
                g.nodes.len()
            );
            // scene vs route at the scene's own centres: edge end distance
            let mut moved = 0;
            let mut worst: f64 = 0.0;
            for (a, b) in sc.edges.iter().zip(&r.edges) {
                let (pa, pb) = (poly(a), poly(b));
                let d = (pa[0].0 - pb[0].0)
                    .hypot(pa[0].1 - pb[0].1)
                    .max((pa[pa.len() - 1].0 - pb[pb.len() - 1].0).hypot(pa[pa.len() - 1].1 - pb[pb.len() - 1].1));
                worst = worst.max(d);
                if d > 1.0 {
                    moved += 1;
                }
            }
            println!(
                "scene() vs route(scene centres): {moved}/{} edges with an end > 1px apart (worst {worst:.0}px); frames dmax {:.1}",
                sc.edges.len(),
                frames_match(&sc, &r)
            );
        }
        Some("pos") => {
            let (g, _) = load(&args[1]).expect("flowchart or state diagram");
            let sc = scene(&g);
            for n in &sc.nodes {
                println!("{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}", n.id, n.x, n.y, n.w, n.h);
            }
        }
        Some("routeat") => {
            // routeat <file> <tsv id x y> <out.svg> [partial]
            let (g, _) = load(&args[1]).expect("flowchart or state diagram");
            let tsv = std::fs::read_to_string(&args[2]).unwrap();
            let mut at = vec![(0.0, 0.0); g.nodes.len()];
            for l in tsv.lines() {
                let f: Vec<&str> = l.split('\t').collect();
                let v = g.node_index(f[0]).expect("node id");
                at[v] = (f[1].parse().unwrap(), f[2].parse().unwrap());
            }
            let r = route(&g, &at);
            std::fs::write(&args[3], flowmaid::scene::to_svg(&r)).unwrap();
            let m = flowmaid::quality::measure(&g, &r);
            println!(
                "routed at given centres: canvas {:.0}x{:.0} defects={} edge_frame_pairs={} edge_node_pairs={} x_drawn={} frame_side={} wiggles={} lat_wiggles={} band_pairs={} band_len={:.0} lbl_edge={} hug_len={:.0} border_len={:.0}",
                r.width, r.height, m.defects(), m.edge_frame_pairs, m.edge_node_pairs, m.x_drawn, m.frame_side, m.wiggles, m.lat_wiggles, m.band_pairs, m.band_len, m.lbl_edge, m.hug_len, m.border_len
            );
            let ms = flowmaid::quality::fastest_ms(20, || route(&g, &at));
            println!("route_ms={ms:.2}");
            for c in &r.clusters {
                println!("frame {:<6} x={:.0} y={:.0} w={:.0} h={:.0}", c.id, c.x, c.y, c.w, c.h);
            }
            let ch = chains(&g);
            let mut tot_lat = 0.0;
            for (ei, e) in r.edges.iter().enumerate() {
                let p = poly(e);
                let lat: f64 = p.windows(2).map(|w| (w[1].0 - w[0].0).abs()).sum();
                tot_lat += lat;
                let _ = (ei, &ch);
            }
            println!("total lateral run {tot_lat:.0}px");
        }
        _ => eprintln!("usage: b12probe summary <files...> | detail <file> | pos <file> | routeat <file> <tsv> <out.svg>"),
    }
}
