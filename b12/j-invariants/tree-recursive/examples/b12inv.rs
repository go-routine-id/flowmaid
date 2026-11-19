//! Invariant probe for the B12 design judging (public API only).
//!
//! b12inv [--time] [--svg <dir>] <files.mmd...>
//!
//! One TSV row per flowchart / state input:
//!  contract   scene contract holds (index-parallel nodes / edges, frames outermost first)
//!  det        scene() and route() repeat byte for byte
//!  fr_max     largest frame side difference scene() vs route(scene centres), px
//!  fr_n       frames differing by more than 1 px
//!  end_n1/20  edges with an end more than 1 / 20 px apart in scene() vs route(scene centres)
//!  end_max    the largest such end distance
//!  path_n40   edges whose path strays more than 40 px (sampled Hausdorff) between the two
//!  dj         1 px drags (route_partial on the scene): unrelated edges whose end jumps > 20 px
//!  dj_path    same, unrelated edges whose path strays > 40 px
//!  dfr_far    1 px drags: frames NOT holding the dragged node that moved > 1 px
//!  dfr_own    1 px drags: largest move of a frame holding the dragged node
//!  dfull      1 px drags whose route_partial equals a plain route() (fallback)
//!  drags      number of 1 px drags tried
//!  hit_node   nodes whose centre does not pick themselves
//!  hit_frame  nodes whose centre's deepest frame is not their innermost subgraph (static, routed)
//!  hit_edge   drawn edges whose mid point does not pick any edge at tol 3 (static, routed)
//!  rt         to_mermaid round trip: 1 same SVG, 0 different (rt_dx = largest node move)
//!  fold       scene_compact == scene() for a graph with subgraphs
//!  scene_ms route_ms part_ms(worst 1 px route_partial)  (only with --time)

use flowmaid::model::{Document, Graph};
use flowmaid::scene::{self, Scene, SceneEdge};

type P = (f64, f64);

fn ends(e: &SceneEdge) -> (P, P) {
    match (e.waypoints.first(), e.waypoints.last()) {
        (Some(&a), Some(&b)) if e.waypoints.len() >= 2 => (a, b),
        _ => (e.bezier[0], e.bezier[3]),
    }
}

fn dist(a: P, b: P) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The edge's drawn path, sampled every ~8 px.
fn sample(e: &SceneEdge) -> Vec<P> {
    let mut pts: Vec<P> = if e.waypoints.len() >= 2 {
        e.waypoints.clone()
    } else {
        let b = e.bezier;
        (0..=24)
            .map(|k| {
                let t = k as f64 / 24.0;
                let u = 1.0 - t;
                (
                    u * u * u * b[0].0 + 3.0 * u * u * t * b[1].0 + 3.0 * u * t * t * b[2].0 + t * t * t * b[3].0,
                    u * u * u * b[0].1 + 3.0 * u * u * t * b[1].1 + 3.0 * u * t * t * b[2].1 + t * t * t * b[3].1,
                )
            })
            .collect()
    };
    let mut out = Vec::new();
    pts.dedup();
    for w in pts.windows(2) {
        let d = dist(w[0], w[1]);
        let k = (d / 8.0).ceil().max(1.0) as usize;
        for i in 0..k {
            let t = i as f64 / k as f64;
            out.push((w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t));
        }
    }
    if let Some(&l) = pts.last() {
        out.push(l);
    }
    out
}

fn hausdorff(a: &[P], b: &[P]) -> f64 {
    let one = |x: &[P], y: &[P]| {
        x.iter()
            .map(|&p| y.iter().map(|&q| dist(p, q)).fold(f64::INFINITY, f64::min))
            .fold(0.0, f64::max)
    };
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    one(a, b).max(one(b, a))
}

fn drawn(e: &SceneEdge) -> bool {
    let (a, b) = ends(e);
    dist(a, b) > 0.5 || e.waypoints.len() > 2
}

fn frame_delta(a: &Scene, b: &Scene, id: &str) -> Option<f64> {
    let x = a.clusters.iter().find(|c| c.id == id)?;
    let y = b.clusters.iter().find(|c| c.id == id)?;
    Some(
        (x.x - y.x)
            .abs()
            .max((x.y - y.y).abs())
            .max((x.x + x.w - y.x - y.w).abs())
            .max((x.y + x.h - y.y - y.h).abs()),
    )
}

/// Innermost subgraph of each node.
fn innermost(g: &Graph) -> Vec<Option<usize>> {
    let mut out = vec![None; g.nodes.len()];
    let depth = |mut s: usize| {
        let mut d = 0;
        while let Some(p) = g.subgraphs[s].parent {
            d += 1;
            s = p;
        }
        d
    };
    for (si, s) in g.subgraphs.iter().enumerate() {
        for &v in &s.nodes {
            if out[v].map_or(true, |o: usize| depth(si) > depth(o)) {
                out[v] = Some(si);
            }
        }
    }
    out
}

/// Whether subgraph `s` holds node `v` at any depth.
fn holds(g: &Graph, inner: &[Option<usize>], s: usize, v: usize) -> bool {
    let mut cur = inner[v];
    while let Some(c) = cur {
        if c == s {
            return true;
        }
        cur = g.subgraphs[c].parent;
    }
    false
}

fn hits(g: &Graph, sc: &Scene, inner: &[Option<usize>]) -> (usize, usize, usize) {
    let (mut hn, mut hf, mut he) = (0, 0, 0);
    for (i, n) in sc.nodes.iter().enumerate() {
        if sc.node_at(n.x, n.y) != Some(i) {
            // A later node may lie on top only when boxes overlap.
            hn += 1;
        }
        let want = inner[i].map(|s| g.subgraphs[s].id.clone());
        let got = sc.cluster_at(n.x, n.y).map(|c| sc.clusters[c].id.clone());
        if want != got {
            hf += 1;
        }
    }
    for e in sc.edges.iter() {
        if !drawn(e) {
            continue;
        }
        let s = sample(e);
        let m = s[s.len() / 2];
        if sc.edge_at(m.0, m.1, 3.0).is_none() && sc.node_at(m.0, m.1).is_none() {
            he += 1;
        }
    }
    (hn, hf, he)
}

fn contract(g: &Graph, sc: &Scene) -> bool {
    if sc.nodes.len() != g.nodes.len() {
        return false;
    }
    if sc.nodes.iter().zip(&g.nodes).any(|(a, b)| a.id != b.id) {
        return false;
    }
    if sc.edges.len() < g.edges.len() {
        return false;
    }
    for (e, se) in g.edges.iter().zip(&sc.edges) {
        if se.from != g.nodes[e.from].id || se.to != g.nodes[e.to].id {
            return false;
        }
    }
    if sc.edges.len() > g.edges.len() + g.sub_edges.len() {
        return false;
    }
    // Frames outermost first, one per non-empty subgraph at most, depth = nesting.
    if sc.clusters.windows(2).any(|w| w[0].depth > w[1].depth) {
        return false;
    }
    for c in &sc.clusters {
        let Some(si) = g.subgraphs.iter().position(|s| s.id == c.id) else {
            return false;
        };
        let mut d = 0;
        let mut s = si;
        while let Some(p) = g.subgraphs[s].parent {
            d += 1;
            s = p;
        }
        if d != c.depth {
            return false;
        }
    }
    // Every edge geometry finite.
    sc.edges.iter().all(|e| {
        e.bezier.iter().all(|p| p.0.is_finite() && p.1.is_finite())
            && e.waypoints.iter().all(|p| p.0.is_finite() && p.1.is_finite())
    })
}

fn fastest<T>(runs: usize, mut f: impl FnMut() -> T) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..runs {
        let t = std::time::Instant::now();
        std::hint::black_box(f());
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
    }
    best
}

fn main() {
    let mut timing = false;
    let mut svg: Option<String> = None;
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--time" {
            timing = true;
        } else if a == "--svg" {
            svg = args.next();
        } else {
            files.push(a);
        }
    }
    println!(
        "file\tcontract\tdet\tfr_max\tfr_n\tend_n1\tend_n20\tend_max\tpath_n40\tdj\tdj_path\tdfr_far\tdfr_own\tdfull\tdrags\thit_node\thit_frame\thit_edge\trt\trt_dx\tfold\tscene_ms\troute_ms\tpart_ms\tW\tH"
    );
    for f in &files {
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(f).expect("read");
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g) | Document::State(g)) => g,
            Ok(_) => continue,
            Err(e) => {
                eprintln!("skip {name}: {e}");
                continue;
            }
        };
        let s = scene::scene(&g);
        let at: Vec<P> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = scene::route(&g, &at);
        let det = scene::to_svg(&s) == scene::to_svg(&scene::scene(&g))
            && scene::to_svg(&r) == scene::to_svg(&scene::route(&g, &at));
        let ok = contract(&g, &s) && contract(&g, &r) && s.edges.len() == r.edges.len();
        if let Some(d) = &svg {
            std::fs::create_dir_all(d).ok();
            std::fs::write(format!("{d}/{name}.static.svg"), scene::to_svg(&s)).ok();
            std::fs::write(format!("{d}/{name}.routed.svg"), scene::to_svg(&r)).ok();
        }
        // Frames scene vs route.
        let (mut fr_max, mut fr_n) = (0.0f64, 0);
        for c in &s.clusters {
            if let Some(d) = frame_delta(&s, &r, &c.id) {
                fr_max = fr_max.max(d);
                if d > 1.0 {
                    fr_n += 1;
                }
            }
        }
        // Edges scene vs route.
        let (mut n1, mut n20, mut emax, mut p40) = (0, 0, 0.0f64, 0);
        for (a, b) in s.edges.iter().zip(&r.edges) {
            if !drawn(a) && !drawn(b) {
                continue;
            }
            let ((a0, a1), (b0, b1)) = (ends(a), ends(b));
            let m = dist(a0, b0).max(dist(a1, b1));
            emax = emax.max(m);
            if m > 1.0 {
                n1 += 1;
            }
            if m > 20.0 {
                n20 += 1;
            }
            if hausdorff(&sample(a), &sample(b)) > 40.0 {
                p40 += 1;
            }
        }
        // 1 px drags.
        let inner = innermost(&g);
        let (mut dj, mut djp, mut dfar, mut down, mut dfull, mut drags) = (0, 0, 0, 0.0f64, 0, 0);
        let mut part_ms = 0.0f64;
        let base_samples: Vec<Vec<P>> = s.edges.iter().map(sample).collect();
        for (v, node) in g.nodes.iter().enumerate() {
            let mut c = at.clone();
            c[v].0 += 1.0;
            let p = scene::route_partial(&g, &c, &s, &at);
            drags += 1;
            if timing {
                part_ms = part_ms.max(fastest(2, || scene::route_partial(&g, &c, &s, &at)));
            }
            for (i, (a, b)) in s.edges.iter().zip(&p.edges).enumerate() {
                if a.from == node.id || a.to == node.id {
                    continue;
                }
                let ((a0, a1), (b0, b1)) = (ends(a), ends(b));
                if dist(a0, b0).max(dist(a1, b1)) > 20.0 {
                    dj += 1;
                }
                if a.waypoints != b.waypoints || a.bezier != b.bezier {
                    if hausdorff(&base_samples[i], &sample(b)) > 40.0 {
                        djp += 1;
                    }
                }
            }
            for (si, sg) in g.subgraphs.iter().enumerate() {
                if let Some(d) = frame_delta(&s, &p, &sg.id) {
                    if holds(&g, &inner, si, v) {
                        down = down.max(d);
                    } else if d > 1.0 {
                        dfar += 1;
                    }
                }
            }
            let full = scene::route(&g, &c);
            if p.edges.len() == full.edges.len()
                && p.edges
                    .iter()
                    .zip(&full.edges)
                    .all(|(a, b)| a.waypoints == b.waypoints && a.bezier == b.bezier)
            {
                dfull += 1;
            }
        }
        let (hn, hf, he) = hits(&g, &s, &inner);
        let (hn2, hf2, he2) = hits(&g, &r, &inner);
        // to_mermaid round trip.
        let (mut rt, mut rt_dx) = (-1i32, 0.0f64);
        if matches!(flowmaid::parser::parse_document(&src), Ok(Document::Flowchart(_))) {
            let m = flowmaid::emit::to_mermaid(&g);
            if let Ok(Document::Flowchart(g2)) = flowmaid::parser::parse_document(&m) {
                let s2 = scene::scene(&g2);
                rt = i32::from(scene::to_svg(&s2) == scene::to_svg(&s));
                for n in &s.nodes {
                    if let Some(n2) = s2.nodes.iter().find(|x| x.id == n.id) {
                        rt_dx = rt_dx.max(dist((n.x, n.y), (n2.x, n2.y)));
                    } else {
                        rt_dx = f64::INFINITY;
                    }
                }
            } else {
                rt = -2;
            }
        }
        // Fold: a graph with subgraphs comes back as scene().
        let fold = if g.subgraphs.is_empty() {
            -1
        } else {
            let cs = flowmaid::fold::scene_compact(&g, &flowmaid::fold::CompactOptions::for_extent(400.0));
            i32::from(scene::to_svg(&cs.scene) == scene::to_svg(&s))
        };
        let (scene_ms, route_ms) = if timing {
            (fastest(7, || scene::scene(&g)), fastest(7, || scene::route(&g, &at)))
        } else {
            (0.0, 0.0)
        };
        println!(
            "{name}\t{}\t{}\t{fr_max:.1}\t{fr_n}\t{n1}\t{n20}\t{emax:.1}\t{p40}\t{dj}\t{djp}\t{dfar}\t{down:.1}\t{dfull}\t{drags}\t{}\t{}\t{}\t{rt}\t{rt_dx:.1}\t{fold}\t{scene_ms:.2}\t{route_ms:.2}\t{part_ms:.2}\t{:.0}\t{:.0}",
            u8::from(ok),
            u8::from(det),
            hn + hn2,
            format!("{hf}/{hf2}"),
            format!("{he}/{he2}"),
            s.width,
            s.height,
        );
    }
}
