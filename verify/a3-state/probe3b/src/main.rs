use flowmaid::model::Document;
use flowmaid::scene::{route, route_partial, scene, Scene, SceneEdge};

fn state(src: &str) -> flowmaid::model::Graph {
    match flowmaid::parser::parse_document(src).unwrap() {
        Document::State(g) => g,
        _ => panic!(),
    }
}

fn poly(e: &SceneEdge) -> Vec<(f64, f64)> {
    if e.waypoints.len() >= 2 {
        // As hosts draw it: a basis spline through the waypoints (scene::basis_sample).
        let pts = &e.waypoints;
        let n = pts.len();
        if n < 3 { return vec![pts[0], pts[n - 1]]; }
        let cubic = |s: &[(f64, f64); 4], t: f64| {
            let u = 1.0 - t;
            (u*u*u*s[0].0 + 3.0*u*u*t*s[1].0 + 3.0*u*t*t*s[2].0 + t*t*t*s[3].0,
             u*u*u*s[0].1 + 3.0*u*u*t*s[1].1 + 3.0*u*t*t*s[2].1 + t*t*t*s[3].1)
        };
        let mut out = vec![pts[0]];
        let mut cur = ((5.0 * pts[0].0 + pts[1].0) / 6.0, (5.0 * pts[0].1 + pts[1].1) / 6.0);
        out.push(cur);
        let mut emit = |out: &mut Vec<(f64, f64)>, a: (f64, f64), b: (f64, f64), p: (f64, f64)| {
            let seg = [cur, ((2.0*a.0+b.0)/3.0, (2.0*a.1+b.1)/3.0), ((a.0+2.0*b.0)/3.0, (a.1+2.0*b.1)/3.0), ((a.0+4.0*b.0+p.0)/6.0, (a.1+4.0*b.1+p.1)/6.0)];
            for k in 1..=24 { out.push(cubic(&seg, k as f64 / 24.0)); }
            cur = seg[3];
        };
        for i in 2..n { emit(&mut out, pts[i-2], pts[i-1], pts[i]); }
        emit(&mut out, pts[n-2], pts[n-1], pts[n-1]);
        out.push(pts[n-1]);
        return out;
    }
    let b = e.bezier;
    (0..=200)
        .map(|i| {
            let t = i as f64 / 200.0;
            let u = 1.0 - t;
            (
                u * u * u * b[0].0 + 3.0 * u * u * t * b[1].0 + 3.0 * u * t * t * b[2].0 + t * t * t * b[3].0,
                u * u * u * b[0].1 + 3.0 * u * u * t * b[1].1 + 3.0 * u * t * t * b[2].1 + t * t * t * b[3].1,
            )
        })
        .collect()
}

fn seg_hits_rect(p: (f64, f64), q: (f64, f64), r: (f64, f64, f64, f64)) -> bool {
    // sample segment densely
    for k in 0..=20 {
        let t = k as f64 / 20.0;
        let x = p.0 + (q.0 - p.0) * t;
        let y = p.1 + (q.1 - p.1) * t;
        if x > r.0 && x < r.2 && y > r.1 && y < r.3 {
            return true;
        }
    }
    false
}

fn check(sc: &Scene, point: &str, what: &str) -> usize {
    let polys: Vec<_> = sc.edges.iter().map(poly).collect();
    let mut bad = 0;
    for (ei, e) in sc.edges.iter().enumerate() {
        let Some((t, c, w)) = &e.label else { continue };
        if e.from != point && e.to != point {
            continue;
        }
        let r = (c.0 - w / 2.0, c.1 - 10.0, c.0 + w / 2.0, c.1 + 10.0);
        // own line distance
        let d = polys[ei]
            .iter()
            .map(|p| (p.0 - c.0).hypot(p.1 - c.1))
            .fold(f64::INFINITY, f64::min);
        let mut foreign = vec![];
        for (oj, o) in polys.iter().enumerate() {
            if oj == ei {
                continue;
            }
            if o.windows(2).any(|s| seg_hits_rect(s[0], s[1], r)) {
                foreign.push(format!("{}->{}", sc.edges[oj].from, sc.edges[oj].to));
            }
        }
        let ok = d < 2.0 && foreign.is_empty();
        if !ok {
            bad += 1;
        }
        println!(
            "  {what}: '{t}' at ({:.1},{:.1}) own-dist {:.1} foreign {:?} {}",
            c.0, c.1, d, foreign, if ok { "ok" } else { "BAD" }
        );
    }
    bad
}

fn main() {
    let cases = [
        ("choice3", "stateDiagram-v2\nstate c <<choice>>\n[*] --> Order\nOrder --> c\nc --> Express : priority customer\nc --> Standard : regular customer\nc --> Hold : payment pending\nc --> Cancel : fraud suspected"),
        ("choice1", "stateDiagram-v2\nstate c <<choice>>\n[*] --> P\nP --> c\nc --> False: if n < 0\nc --> True : if n >= 0\nc --> Zero : if n == 0"),
        ("lr", "stateDiagram-v2\ndirection LR\nstate c <<choice>>\n[*] --> Order\nOrder --> c\nc --> A : one\nc --> B : two\nc --> C : three"),
    ];
    for (name, src) in cases {
        let g = state(src);
        let sc = scene(&g);
        println!("{name}");
        check(&sc, "c", "scene");
        let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let ci = g.node_index("c").unwrap();
        for (dx, dy) in [(60.0, 0.0), (-80.0, 0.0), (0.0, 50.0), (0.0, -30.0)] {
            let mut m = base.clone();
            m[ci].0 += dx;
            m[ci].1 += dy;
            let t0 = std::time::Instant::now();
            let rp = route_partial(&g, &m, &sc, &base);
            let el = t0.elapsed().as_secs_f64() * 1000.0;
            let rt = route(&g, &m);
            println!(" drag choice by ({dx},{dy}) route_partial {el:.2}ms");
            check(&rp, "c", "partial");
            check(&rt, "c", "route");
            // agreement between partial and full route labels
            for (a, b) in rp.edges.iter().zip(&rt.edges) {
                if let (Some(la), Some(lb)) = (&a.label, &b.label) {
                    let d = (la.1 .0 - lb.1 .0).hypot(la.1 .1 - lb.1 .1);
                    if d > 0.5 {
                        println!("   label '{}' partial vs route differ by {d:.1}", la.0);
                    }
                }
            }
        }
    }
    // Bar-to-bar: compact (skipped) scene vs ordinary scene
    let g = state("stateDiagram-v2\nstate f <<fork>>\nstate j <<join>>\n[*] --> f\nf --> A\nf --> B\nf --> j\nA --> j\nB --> j\nj --> End\nEnd --> [*]");
    let sc = scene(&g);
    let cs = flowmaid::fold::scene_compact(&g, &flowmaid::fold::CompactOptions::for_extent(100000.0));
    let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
    let rt = route(&g, &base);
    for id in ["f", "j"] {
        let i = g.node_index(id).unwrap();
        println!(
            "bar {id}: scene w={:.1} | compact(skipped={:?}) w={:.1} | route w={:.1}",
            sc.nodes[i].w, cs.skipped.is_some(), cs.scene.nodes[i].w, rt.nodes[i].w
        );
    }
    // route_sized with the scene's own sizes (a host passing back what it drew)
    let sizes: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.w, n.h)).collect();
    let rs = flowmaid::scene::route_sized(&g, &base, &sizes);
    for id in ["f", "j"] {
        let i = g.node_index(id).unwrap();
        println!("bar {id}: route_sized(scene sizes) w={:.1}", rs.nodes[i].w);
    }
}
