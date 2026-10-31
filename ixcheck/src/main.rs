// Interactive-API smoke test over a corpus: scene -> drag sweep -> route/route_partial -> pick -> svg.
use flowmaid::model::Document;
use flowmaid::scene::{self, Hit};
use std::time::Instant;

fn finite(svg: &str) -> bool { !svg.contains("NaN") && !svg.contains("inf") }
fn offsets() -> Vec<(f64, f64)> {
    vec![(0.0, 0.0), (37.0, -21.0), (-140.0, 90.0), (260.0, 180.0), (-900.0, -700.0), (3.0, 0.5), (0.0, -2000.0)]
}

fn flow(name: &str, g: &flowmaid::model::Graph, report: &mut Vec<String>) {
    let s = scene::scene(g);
    let base: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
    // pick == render: node centres pick their node; an edge's middle waypoint picks an edge.
    let mut node_ok = 0;
    for (i, n) in s.nodes.iter().enumerate() {
        if let Some(Hit::Node(j)) = s.hit_test(n.x, n.y, 3.0) { if j == i || s.nodes[j].x == n.x { node_ok += 1; } }
    }
    let mut edge_hits = 0; let mut edges_tested = 0;
    for e in s.edges.iter() {
        let p = if e.waypoints.len() > 2 { e.waypoints[e.waypoints.len() / 2] } else {
            let b = e.bezier; let t = 0.5; let mt = 1.0 - t;
            (mt*mt*mt*b[0].0 + 3.0*mt*mt*t*b[1].0 + 3.0*mt*t*t*b[2].0 + t*t*t*b[3].0,
             mt*mt*mt*b[0].1 + 3.0*mt*mt*t*b[1].1 + 3.0*mt*t*t*b[2].1 + t*t*t*b[3].1) };
        if s.node_at(p.0, p.1).is_some() { continue; }
        edges_tested += 1;
        if s.edge_at(p.0, p.1, 4.0).is_some() { edge_hits += 1; }
    }
    let mut worst_route = 0.0f64; let mut worst_partial = 0.0f64; let mut drags = 0; let mut bad = 0;
    for i in 0..base.len() {
        for &(dx, dy) in &offsets() {
            let mut pos = base.clone(); pos[i].0 += dx; pos[i].1 += dy;
            let t = Instant::now(); let r = scene::route(g, &pos); worst_route = worst_route.max(t.elapsed().as_secs_f64() * 1e3);
            let t = Instant::now(); let rp = scene::route_partial(g, &pos, &s, &base); worst_partial = worst_partial.max(t.elapsed().as_secs_f64() * 1e3);
            drags += 1;
            if r.edges.len() != s.edges.len() || rp.edges.len() != s.edges.len() || !finite(&scene::to_svg(&r)) { bad += 1; }
            if r.nodes.iter().zip(&pos).any(|(n, p)| (n.x - p.0).abs() > 0.01 || (n.y - p.1).abs() > 0.01) { bad += 1; }
        }
    }
    let rt = flowmaid::parser::parse(&flowmaid::emit::to_mermaid(g)).map(|g2| g2.nodes.len() == g.nodes.len() && g2.edges.len() == g.edges.len()).unwrap_or(false);
    let compact = flowmaid::fold::scene_compact(g, &flowmaid::fold::CompactOptions::for_extent(300.0));
    report.push(format!("{name:<16} flow   nodes={:<3} pick_node={}/{} pick_edge={}/{} drags={drags} bad={bad} route_max={worst_route:.2}ms partial_max={worst_partial:.2}ms to_mermaid_roundtrip={rt} compact_nodes={}",
        s.nodes.len(), node_ok, s.nodes.len(), edge_hits, edges_tested, compact.scene.nodes.len()));
}

fn main() {
    let mut report = Vec::new();
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let doc = match flowmaid::parser::parse_document(&src) { Ok(d) => d, Err(e) => { report.push(format!("{name:<16} PARSE ERROR {e:?}")); continue; } };
        match doc {
            Document::Flowchart(g) | Document::State(g) => flow(&name, &g, &mut report),
            Document::Er(d) => {
                let s = flowmaid::er::scene(&d); let base: Vec<_> = s.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let (mut bad, mut drags, mut worst) = (0, 0, 0.0f64);
                for i in 0..base.len() { for &(dx, dy) in &offsets() { let mut p = base.clone(); p[i].0 += dx; p[i].1 += dy;
                    let t = Instant::now(); let r = flowmaid::er::route(&d, &p); worst = worst.max(t.elapsed().as_secs_f64()*1e3); drags += 1;
                    if !finite(&flowmaid::er::to_svg(&r)) || r.scene.hit_test(p[i].0, p[i].1, 2.0) != Some(Hit::Node(i)) { bad += 1; } } }
                report.push(format!("{name:<16} er     tables={} drags={drags} bad={bad} route_max={worst:.2}ms", base.len()));
            }
            Document::Class(d) => {
                let s = flowmaid::class::scene(&d); let base: Vec<_> = s.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let (mut bad, mut drags, mut worst) = (0, 0, 0.0f64);
                for i in 0..base.len() { for &(dx, dy) in &offsets() { let mut p = base.clone(); p[i].0 += dx; p[i].1 += dy;
                    let t = Instant::now(); let r = flowmaid::class::route(&d, &p); worst = worst.max(t.elapsed().as_secs_f64()*1e3); drags += 1;
                    if !finite(&flowmaid::class::to_svg(&r)) || r.scene.hit_test(p[i].0, p[i].1, 2.0) != Some(Hit::Node(i)) { bad += 1; } } }
                report.push(format!("{name:<16} class  boxes={} drags={drags} bad={bad} route_max={worst:.2}ms", base.len()));
            }
            Document::Mindmap(d) => {
                let s = flowmaid::mindmap::scene(&d); let base: Vec<_> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
                let (mut bad, mut drags, mut worst) = (0, 0, 0.0f64);
                for i in 0..base.len() { for &(dx, dy) in &offsets() { let mut p = base.clone(); p[i].0 += dx; p[i].1 += dy;
                    let t = Instant::now(); let r = flowmaid::mindmap::route(&d, &p); worst = worst.max(t.elapsed().as_secs_f64()*1e3); drags += 1;
                    if !finite(&flowmaid::mindmap::to_svg(&r)) || r.nodes.len() != base.len() { bad += 1; } } }
                report.push(format!("{name:<16} mind   nodes={} drags={drags} bad={bad} route_max={worst:.2}ms", base.len()));
            }
            Document::Sequence(d) => { let ok = finite(&flowmaid::seq::to_svg(&flowmaid::seq::scene(&d))); report.push(format!("{name:<16} seq    static scene ok={ok}")); }
            Document::Pie(d) => { let ok = finite(&flowmaid::pie::to_svg(&flowmaid::pie::scene(&d))); report.push(format!("{name:<16} pie    static scene ok={ok}")); }
            Document::Journey(d) => { let ok = finite(&flowmaid::journey::to_svg(&flowmaid::journey::scene(&d))); report.push(format!("{name:<16} journey static scene ok={ok}")); }
            Document::Timeline(d) => { let ok = finite(&flowmaid::timeline::to_svg(&flowmaid::timeline::scene(&d))); report.push(format!("{name:<16} timeline static scene ok={ok}")); }
            Document::GitGraph(d) => { let ok = finite(&flowmaid::gitgraph::to_svg(&flowmaid::gitgraph::scene(&d))); report.push(format!("{name:<16} git    static scene ok={ok}")); }
            Document::Sankey(d) => { let ok = finite(&flowmaid::sankey::to_svg(&flowmaid::sankey::scene(&d))); report.push(format!("{name:<16} sankey static scene ok={ok}")); }
            Document::Architecture(d) => { let ok = finite(&flowmaid::architecture::to_svg(&flowmaid::architecture::scene(&d))); report.push(format!("{name:<16} arch   static scene ok={ok}")); }
            #[allow(unreachable_patterns)] _ => report.push(format!("{name:<16} (other)")),
        }
    }
    for l in report { println!("{l}"); }
}
