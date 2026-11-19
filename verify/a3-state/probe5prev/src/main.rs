use flowmaid::model::{Document, Graph};
use flowmaid::scene::{route, route_partial, route_sized, scene, scene_sized, Scene, SceneEdge};

fn state(src: &str) -> Graph {
    match flowmaid::parser::parse_document(src).unwrap() {
        Document::State(g) => g,
        _ => panic!(),
    }
}

fn basis(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
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
    out
}

fn poly(e: &SceneEdge) -> Vec<(f64, f64)> {
    if e.waypoints.len() >= 2 { return basis(&e.waypoints); }
    let b = e.bezier;
    (0..=200).map(|i| {
        let t = i as f64 / 200.0; let u = 1.0 - t;
        (u*u*u*b[0].0 + 3.0*u*u*t*b[1].0 + 3.0*u*t*t*b[2].0 + t*t*t*b[3].0,
         u*u*u*b[0].1 + 3.0*u*u*t*b[1].1 + 3.0*u*t*t*b[2].1 + t*t*t*b[3].1)
    }).collect()
}

fn hits(p: (f64, f64), q: (f64, f64), r: (f64, f64, f64, f64)) -> bool {
    (0..=20).any(|k| { let t = k as f64 / 20.0; let x = p.0 + (q.0-p.0)*t; let y = p.1 + (q.1-p.1)*t; x > r.0 && x < r.2 && y > r.1 && y < r.3 })
}

/// Every label: own-line distance, foreign strokes under its chip, chip overlaps, node overlaps.
fn labels(sc: &Scene, what: &str) -> usize {
    let polys: Vec<_> = sc.edges.iter().map(poly).collect();
    let mut bad = 0;
    let rects: Vec<Option<(f64,f64,f64,f64)>> = sc.edges.iter().map(|e| e.label.as_ref().map(|(_, c, w)| (c.0 - w/2.0, c.1 - 10.0, c.0 + w/2.0, c.1 + 10.0))).collect();
    for (ei, e) in sc.edges.iter().enumerate() {
        let Some((t, c, _w)) = &e.label else { continue };
        let r = rects[ei].unwrap();
        let d = polys[ei].iter().map(|p| (p.0 - c.0).hypot(p.1 - c.1)).fold(f64::INFINITY, f64::min);
        let foreign: Vec<String> = polys.iter().enumerate().filter(|(oj, o)| *oj != ei && o.windows(2).any(|s| hits(s[0], s[1], r))).map(|(oj, _)| format!("{}->{}", sc.edges[oj].from, sc.edges[oj].to)).collect();
        let chip: Vec<String> = rects.iter().enumerate().filter(|(oj, o)| *oj != ei && o.map_or(false, |o| o.0 < r.2 && r.0 < o.2 && o.1 < r.3 && r.1 < o.3)).map(|(oj, _)| sc.edges[oj].label.as_ref().unwrap().0.clone()).collect();
        let node: Vec<String> = sc.nodes.iter().filter(|n| { let nr = (n.x - n.w/2.0, n.y - n.h/2.0, n.x + n.w/2.0, n.y + n.h/2.0); nr.0 < r.2 && r.0 < nr.2 && nr.1 < r.3 && r.1 < nr.3 }).map(|n| n.id.clone()).collect();
        let ok = d < 14.0 && foreign.is_empty() && chip.is_empty() && node.is_empty();
        if !ok { bad += 1; }
        println!("  {what}: '{t}' {}->{} at ({:.1},{:.1}) own {:.1} foreign {:?} chip {:?} node {:?} {}", e.from, e.to, c.0, c.1, d, foreign, chip, node, if ok {"ok"} else {"BAD"});
    }
    bad
}

fn bars(g: &Graph, sc: &Scene, tag: &str) {
    let mut s = String::new();
    for (i, n) in g.nodes.iter().enumerate() {
        if matches!(n.shape, flowmaid::model::Shape::ForkBar) {
            s += &format!(" {}={:.1}x{:.1}", n.id, sc.nodes[i].w, sc.nodes[i].h);
        }
    }
    println!("  {tag}:{s}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("all");
    if mode == "all" || mode == "bars" {
        let cases = [
            ("chain3", "stateDiagram-v2\nstate f1 <<fork>>\nstate f2 <<fork>>\nstate f3 <<fork>>\n[*] --> f1\nf1 --> A\nf1 --> f2\nf2 --> B\nf2 --> f3\nf3 --> C\nf3 --> D\nA --> [*]\nB --> [*]\nC --> [*]\nD --> [*]"),
            ("join2fork", "stateDiagram-v2\nstate f <<fork>>\nstate j <<join>>\nstate f2 <<fork>>\nstate j2 <<join>>\n[*] --> f\nf --> A\nf --> B\nA --> j\nB --> j\nj --> f2\nf2 --> C\nf2 --> D\nf2 --> E\nC --> j2\nD --> j2\nE --> j2\nj2 --> [*]"),
            ("f2j_lr", "stateDiagram-v2\ndirection LR\nstate f <<fork>>\nstate j <<join>>\n[*] --> f\nf --> Alpha\nf --> Beta\nf --> Gamma\nf --> j\nAlpha --> j\nBeta --> j\nGamma --> j\nj --> [*]"),
            ("nest3", "stateDiagram-v2\nstate f <<fork>>\nstate g <<fork>>\nstate h <<fork>>\n[*] --> f\nf --> g\nf --> X\ng --> h\ng --> Y\nh --> P\nh --> Q\nh --> R"),
            ("f2j_comp", "stateDiagram-v2\nstate f <<fork>>\nstate j <<join>>\n[*] --> f\nf --> Work\nf --> j\nstate Work {\n  [*] --> W1\n  W1 --> W2\n}\nf --> Audit\nWork --> j\nAudit --> j\nnote right of j : joined"),
        ];
        for (name, src) in cases {
            let g = state(src);
            let sc = scene(&g);
            println!("{name}");
            bars(&g, &sc, "scene");
            let cs = flowmaid::fold::scene_compact(&g, &flowmaid::fold::CompactOptions::for_extent(100000.0));
            bars(&g, &cs.scene, &format!("compact skipped={}", cs.skipped.is_some()));
            let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            bars(&g, &route(&g, &base), "route");
            let mut sizes: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.w, n.h)).collect();
            for k in 0..3 {
                let rs = route_sized(&g, &base, &sizes);
                bars(&g, &rs, &format!("route_sized#{k}"));
                sizes = rs.nodes.iter().map(|n| (n.w, n.h)).collect();
            }
            let ss = scene_sized(&g, &sizes);
            bars(&g, &ss, "scene_sized(drawn)");
            // branch straightness: for each edge from/to a bar, compare the bar-end x with the other end x (TB) 
            for e in sc.edges.iter() {
                let b = e.bezier;
                let dx = (b[0].0 - b[3].0).abs(); let dy = (b[0].1 - b[3].1).abs();
                let fb = |id: &str| g.node_index(id).map_or(false, |i| matches!(g.nodes[i].shape, flowmaid::model::Shape::ForkBar));
                if fb(&e.from) || fb(&e.to) { println!("   {}->{} dx={:.1} dy={:.1}", e.from, e.to, dx, dy); }
            }
            // drag a branch target and check bar width stays
            let ai = 2.min(g.nodes.len()-1);
            let mut m = base.clone(); m[ai].0 += 40.0;
            bars(&g, &route_partial(&g, &m, &sc, &base), "partial drag");
        }
    }
    if mode == "all" || mode == "labels" {
        let cases = [
            ("twins3", "stateDiagram-v2\nstate c <<choice>>\n[*] --> P\nP --> c\nc --> B : one\nc --> B : two\nc --> B : three"),
            ("twins_mix", "stateDiagram-v2\nstate c <<choice>>\n[*] --> P\nP --> c\nc --> B : low\nc --> B : medium\nc --> C : high\nc --> D : critical"),
            ("twins_lr", "stateDiagram-v2\ndirection LR\nstate c <<choice>>\n[*] --> P\nP --> c\nc --> B : yes\nc --> B : maybe\nc --> C : no"),
            ("wide2", "stateDiagram-v2\nstate c <<choice>>\n[*] --> P\nP --> c\nc --> A : the customer has an outstanding balance over limit\nc --> B : the customer is in good standing with the bank"),
            ("wide4", "stateDiagram-v2\nstate c <<choice>>\n[*] --> P\nP --> c\nc --> A : the request came from a verified partner\nc --> B : the request is anonymous but rate limited\nc --> C : the request failed signature validation\nc --> D : x"),
            ("wide_lr", "stateDiagram-v2\ndirection LR\nstate c <<choice>>\n[*] --> P\nP --> c\nc --> A : the request came from a verified partner\nc --> B : the request is anonymous but rate limited\nc --> C : failed"),
            ("twins_bt", "stateDiagram-v2\ndirection BT\nstate c <<choice>>\n[*] --> P\nP --> c\nc --> B : one\nc --> B : two\nc --> C : three"),
        ];
        for (name, src) in cases {
            let g = state(src);
            let sc = scene(&g);
            println!("{name}  canvas {:.0}x{:.0}", sc.width, sc.height);
            let mut bad = labels(&sc, "scene");
            let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rt = route(&g, &base);
            for (a, b) in sc.edges.iter().zip(&rt.edges) {
                if let (Some(la), Some(lb)) = (&a.label, &b.label) {
                    let d = (la.1.0 - lb.1.0).hypot(la.1.1 - lb.1.1);
                    if d > 0.5 { println!("   MISMATCH scene vs route '{}' {d:.1}", la.0); bad += 1; }
                }
            }
            let ci = g.node_index("c").unwrap();
            for (dx, dy) in [(30.0, 0.0), (-30.0, 0.0), (0.0, 25.0)] {
                let mut m = base.clone(); m[ci].0 += dx; m[ci].1 += dy;
                let rp = route_partial(&g, &m, &sc, &base);
                let rt = route(&g, &m);
                println!(" drag c ({dx},{dy})");
                labels(&rp, "partial");
                for (a, b) in rp.edges.iter().zip(&rt.edges) {
                    if let (Some(la), Some(lb)) = (&a.label, &b.label) {
                        let d = (la.1.0 - lb.1.0).hypot(la.1.1 - lb.1.1);
                        if d > 0.5 { println!("   MISMATCH partial vs route '{}' {d:.1}", la.0); }
                    }
                }
            }
            println!(" => scene bad {bad}");
        }
    }
    if mode == "files" {
        for f in &args[2..] {
            let src = std::fs::read_to_string(f).unwrap();
            let g = state(&src);
            let sc = scene(&g);
            println!("{f} canvas {:.0}x{:.0}", sc.width, sc.height);
            labels(&sc, "scene");
            let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rt = route(&g, &base);
            labels(&rt, "route");
        }
    }
    if mode == "all" || mode == "notes" {
        let cases = [
            ("lr_right", "stateDiagram-v2\ndirection LR\n[*] --> Draft\nDraft --> Review\nReview --> Draft : changes\nReview --> Done\nnote right of Draft : author edits"),
            ("lr_left", "stateDiagram-v2\ndirection LR\n[*] --> Draft\nDraft --> Review\nReview --> Done\nnote left of Review : waiting on reviewer"),
            ("tb_both", "stateDiagram-v2\n[*] --> A\nA --> B\nB --> C\nnote right of B : right card\nnote left of B : left card\nC --> [*]"),
            ("rl_notes", "stateDiagram-v2\ndirection RL\n[*] --> A\nA --> B\nB --> C\nnote right of A : r of A\nnote left of C : l of C"),
            ("bt_notes", "stateDiagram-v2\ndirection BT\n[*] --> A\nA --> B\nB --> C\nnote right of B : r of B\nnote left of A\n  multi\n  line\nend note"),
            ("busy", "stateDiagram-v2\ndirection LR\n[*] --> S\nS --> T1\nS --> T2\nS --> T3\nT1 --> S : back\nnote right of S : busy state\nnote left of T2 : t2 note"),
            ("special", "stateDiagram-v2\nstate c <<choice>>\nstate f <<fork>>\n[*] --> A\nA --> c\nc --> f : go\nc --> Z : stop\nf --> X\nf --> Y\nnote right of c : choice note\nnote left of f : fork note"),
        ];
        for (name, src) in cases {
            let g = state(src);
            let sc = scene(&g);
            println!("{name}  canvas {:.0}x{:.0}", sc.width, sc.height);
            labels(&sc, "scene");
            let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rt = route(&g, &base);
            for (i, (a, b)) in sc.edges.iter().zip(&rt.edges).enumerate() {
                let same = a.bezier == b.bezier && a.waypoints == b.waypoints;
                let note = a.to.starts_with("__note") || a.from.starts_with("__note");
                if note || !same {
                    println!("  e{i} {}->{} kind {:?} wp {} scene==route {} bez {:?}", a.from, a.to, a.kind, a.waypoints.len(), same, a.bezier.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>());
                    // pick check: sample point along drawn edge should resolve to this edge
                    let p = poly(a); let mid = p[p.len()/2];
                    let hit = sc.edge_at(mid.0, mid.1, 3.0);
                    if hit != Some(i) { println!("   PICK mismatch at mid ({:.1},{:.1}) -> {:?}", mid.0, mid.1, hit); }
                    // crossing check: does the note link cross any node box (excluding its ends)?
                    for n in &sc.nodes {
                        if n.id == a.from || n.id == a.to { continue; }
                        let r = (n.x - n.w/2.0, n.y - n.h/2.0, n.x + n.w/2.0, n.y + n.h/2.0);
                        if p.windows(2).any(|s| hits(s[0], s[1], r)) { println!("   NODE CROSS {} ", n.id); }
                    }
                    // crossing other edges
                    for (j, o) in sc.edges.iter().enumerate() {
                        if j == i { continue; }
                        let q = poly(o);
                        let mut cross = false;
                        for s in p.windows(2) { for t in q.windows(2) {
                            let d = |a: (f64,f64), b: (f64,f64), c: (f64,f64)| (b.0-a.0)*(c.1-a.1) - (b.1-a.1)*(c.0-a.0);
                            if d(s[0],s[1],t[0])*d(s[0],s[1],t[1]) < 0.0 && d(t[0],t[1],s[0])*d(t[0],t[1],s[1]) < 0.0 { cross = true; }
                        }}
                        if cross && note { println!("   CROSSES edge {}->{}", o.from, o.to); }
                    }
                }
            }
            // drag the state the note is attached to and the card
            for (ni, n) in g.nodes.iter().enumerate() {
                if !n.id.starts_with("__note") { continue; }
                for (dx, dy) in [(0.0, 40.0), (40.0, 0.0), (0.0, -60.0)] {
                    let mut m = base.clone(); m[ni].0 += dx; m[ni].1 += dy;
                    let rp = route_partial(&g, &m, &sc, &base);
                    let rt = route(&g, &m);
                    let mut diff = 0;
                    for (a, b) in rp.edges.iter().zip(&rt.edges) { if a.bezier != b.bezier || a.waypoints != b.waypoints { diff += 1; } }
                    if diff > 0 { println!("  drag {} ({dx},{dy}): partial vs route differ on {diff} edges", n.id); }
                }
            }
        }
    }
}
