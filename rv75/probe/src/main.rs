use flowmaid::{layout, parser, scene};
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(path).unwrap();
    let g = parser::parse(&src).unwrap();
    // cluster paths
    let mut nc: Vec<Vec<usize>> = vec![Vec::new(); g.nodes.len()];
    for (si, s) in g.subgraphs.iter().enumerate() {
        for &v in &s.nodes {
            let mut chain = vec![si];
            let mut p = s.parent;
            while let Some(pp) = p { chain.insert(0, pp); p = g.subgraphs[pp].parent; }
            nc[v] = chain;
        }
    }
    let sizes: Vec<(f64, f64)> = g.nodes.iter().map(layout::intrinsic_size).collect();
    let lo = layout::layout_clustered(&g, &sizes, &nc);
    println!("total_b {:.0} total_l {:.0}", lo.total_b, lo.total_l);
    let mut idx: Vec<usize> = (0..g.nodes.len()).collect();
    idx.sort_by(|&a, &b| (lo.nodes[a].layer, lo.nodes[a].b as i64).cmp(&(lo.nodes[b].layer, lo.nodes[b].b as i64)));
    for v in idx { println!("L{:2} b={:7.1} {}", lo.nodes[v].layer, lo.nodes[v].b, g.nodes[v].id); }
    let mut back = 0;
    for e in &g.edges { if lo.nodes[e.to].layer < lo.nodes[e.from].layer { back += 1; println!("BACK {} -> {} ({}->{})", g.nodes[e.from].id, g.nodes[e.to].id, lo.nodes[e.from].layer, lo.nodes[e.to].layer);} }
    println!("back edges {}", back);
    let span: usize = g.edges.iter().map(|e| (lo.nodes[e.to].layer as i64 - lo.nodes[e.from].layer as i64).unsigned_abs() as usize).sum();
    println!("total layer span {}", span);
    let sc = scene::scene(&g);
    for c in &sc.clusters { println!("cluster {} x={:.0} y={:.0} w={:.0} h={:.0}", c.id, c.x, c.y, c.w, c.h); }
    let mut nw = 0; for e in &sc.edges { if e.waypoints.is_empty() { nw += 1; } }
    println!("edges without waypoints (single cubic): {}", nw);
}
