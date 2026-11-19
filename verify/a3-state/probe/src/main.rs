use flowmaid::model::Document;
use flowmaid::scene::{route, route_partial, scene, Hit};
use std::time::Instant;
fn st(src: &str) -> flowmaid::model::Graph {
    match flowmaid::parser::parse_document(src).unwrap() { Document::State(g) => g, _ => panic!() }
}
fn main() {
    let dir = std::env::args().nth(1).unwrap();
    for f in std::fs::read_dir(&dir).unwrap() {
        let p = f.unwrap().path();
        if p.extension().map_or(true, |e| e != "mmd") { continue; }
        let src = std::fs::read_to_string(&p).unwrap();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::State(g)) => g, Ok(Document::Flowchart(g)) => g, _ => continue };
        let sc = scene(&g);
        // pick matches render: every node centre hits itself
        for (i, n) in sc.nodes.iter().enumerate() {
            let h = sc.hit_test(n.x, n.y, 2.0);
            if h != Some(Hit::Node(i)) { println!("{}: node {} '{}' centre hit {:?}", p.display(), n.id, n.label, h); }
        }
        let centres: Vec<(f64,f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = route(&g, &centres);
        for (a, b) in sc.nodes.iter().zip(&rt.nodes) {
            if (a.w, a.h) != (b.w, b.h) { println!("{}: size differs {} {:?} {:?}", p.display(), a.id, (a.w,a.h), (b.w,b.h)); }
        }
        // drag each node 30px and time route_partial
        let mut worst = 0.0f64;
        for i in 0..g.nodes.len() {
            let mut m = centres.clone(); m[i].0 += 30.0; m[i].1 += 10.0;
            let t = Instant::now();
            let s = route_partial(&g, &m, &sc, &centres);
            worst = worst.max(t.elapsed().as_secs_f64()*1000.0);
            for e in &s.edges { for q in &e.bezier { if !q.0.is_finite() || !q.1.is_finite() { println!("NaN {}", p.display()); } } }
        }
        // round trip
        let text = flowmaid::to_mermaid(&g);
        let back = flowmaid::parser::parse(&text);
        let rt_ok = match &back { Ok(b) => b.nodes.len() == g.nodes.len() && b.edges.len() == g.edges.len(), Err(_) => false };
        println!("{:<22} nodes {:>3} edges {:>3} sub {:>2} route_partial worst {:.2}ms roundtrip {}", p.file_name().unwrap().to_string_lossy(), g.nodes.len(), g.edges.len(), g.sub_edges.len(), worst, if rt_ok {"ok"} else {"MISMATCH"});
        if !rt_ok { if let Ok(b)=&back { println!("   back nodes {} edges {}", b.nodes.len(), b.edges.len()); } else { println!("   {:?}", back.err()); } }
    }
    let _ = st("stateDiagram-v2\nA-->B");
}
