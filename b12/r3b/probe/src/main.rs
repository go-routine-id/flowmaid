use flowmaid::model::Document;
use flowmaid::scene;
use std::time::Instant;

fn graph(path: &str) -> flowmaid::model::Graph {
    let src = std::fs::read_to_string(path).unwrap();
    match flowmaid::parser::parse_document(&src).unwrap() {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => panic!("not a flowchart"),
    }
}

fn ends(e: &scene::SceneEdge) -> ((f64, f64), (f64, f64)) {
    match (e.waypoints.first(), e.waypoints.last()) {
        (Some(&a), Some(&b)) if e.waypoints.len() >= 2 => (a, b),
        _ => (e.bezier[0], e.bezier[3]),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args[1].as_str();
    match cmd {
        // dfr <files...>: dfr_own / dfr_far per file (1 px drags from scene, route_partial)
        "dfr" => {
            for f in &args[2..] {
                let g = graph(f);
                let base = scene::scene(&g);
                let (own, far) = flowmaid::quality::frame_drags(&g, &base);
                println!("{f}\t{own:.2}\t{far:.2}");
            }
        }
        // jumps <file>: drag jumps like a_one_px_drag_leaves_unrelated_edges_put
        "jumps" => {
            let g = graph(&args[2]);
            let base = scene::scene(&g);
            let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
            for (v, node) in g.nodes.iter().enumerate() {
                let mut c = at.clone();
                c[v].0 += 1.0;
                let p = scene::route_partial(&g, &c, &base, &at);
                for (i, (a, b)) in base.edges.iter().zip(&p.edges).enumerate().take(g.edges.len()) {
                    if a.from == node.id || a.to == node.id { continue; }
                    let ((a0, a1), (b0, b1)) = (ends(a), ends(b));
                    let d = |x: (f64, f64), y: (f64, f64)| (x.0 - y.0).hypot(x.1 - y.1);
                    let m = d(a0, b0).max(d(a1, b1));
                    if m > 20.0 {
                        println!("drag {}: e{} {} -> {} {m:.1}  base {:?} {:?} now {:?} {:?} wp {} -> {}", node.id, i, a.from, a.to, a0, a1, b0, b1, a.waypoints.len(), b.waypoints.len());
                    }
                }
            }
        }
        // perf <files...>: route() at centres, drag frames (route per frame), route_partial worst frame
        "perf" => {
            for f in &args[2..] {
                let g = graph(f);
                let base = scene::scene(&g);
                let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
                let r = flowmaid::quality::fastest_ms(5, || scene::route(&g, &at));
                let frames = flowmaid::quality::drag_frames(&base);
                let (mut worst_r, mut worst_p, mut sum_r) = (0.0f64, 0.0f64, 0.0);
                let mut slow: Vec<(f64, String)> = Vec::new();
                for (v, c) in &frames {
                    let t = Instant::now(); let _ = scene::route(&g, c); let ms = t.elapsed().as_secs_f64()*1e3;
                    worst_r = worst_r.max(ms); sum_r += ms;
                    slow.push((ms, format!("{} to {:?}", g.nodes[*v].id, c[*v])));
                    let t = Instant::now(); let _ = scene::route_partial(&g, c, &base, &at); let ms = t.elapsed().as_secs_f64()*1e3;
                    worst_p = worst_p.max(ms);
                }
                slow.sort_by(|a, b| b.0.total_cmp(&a.0));
                if std::env::var_os("SLOW").is_some() { for (ms, w) in slow.iter().take(6) { println!("   slow {ms:.2} {w}"); } }
                println!("{f}\troute {r:.2}\tdrag worst {worst_r:.2} mean {:.2}\tpartial worst {worst_p:.2}\tframes {}", sum_r / frames.len() as f64, frames.len());
            }
        }
        // dfrwho <file>: per 1 px drag (+x) of each node, the frames that move more than 1.5 px
        "dfrwho" => {
            let g = graph(&args[2]);
            let base = scene::scene(&g);
            let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
            for v in 0..at.len() {
                let mut c = at.clone();
                c[v].0 += 1.0;
                let p = scene::route_partial(&g, &c, &base, &at);
                for cb in &base.clusters {
                    if let Some(cp) = p.clusters.iter().find(|x| x.id == cb.id) {
                        let d = [cp.x - cb.x, cp.y - cb.y, (cp.x + cp.w) - (cb.x + cb.w), (cp.y + cp.h) - (cb.y + cb.h)];
                        if d.iter().any(|x| x.abs() > 1.5) {
                            println!("drag {}: frame {} dL {:.2} dT {:.2} dR {:.2} dB {:.2}", g.nodes[v].id, cb.id, d[0], d[1], d[2], d[3]);
                        }
                    }
                }
            }
        }
        _ => panic!("cmd"),
    }
}
