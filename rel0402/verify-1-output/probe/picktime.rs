//! Pick cost: for each file, a 100 x 100 grid over the canvas at tol 0
//! and 4, three passes; per pick mean, p99 and max (microseconds).
use flowmaid::model::Document;
use flowmaid::{journey, mindmap, parser, pie, sankey, seq, timeline};
use std::time::Instant;

fn time<H>(w: f64, h: f64, f: impl Fn(f64, f64, f64) -> Option<H>) -> (f64, f64, f64, usize) {
    let mut ts = Vec::new();
    let mut hits = 0;
    for _pass in 0..3 {
        for tol in [0.0, 4.0] {
            for i in 0..100 {
                for j in 0..100 {
                    let (x, y) = (w * (i as f64 + 0.5) / 100.0, h * (j as f64 + 0.5) / 100.0);
                    let t = Instant::now();
                    let r = std::hint::black_box(f(x, y, tol));
                    ts.push(t.elapsed().as_secs_f64() * 1e6);
                    if r.is_some() {
                        hits += 1;
                    }
                }
            }
        }
    }
    ts.sort_by(f64::total_cmp);
    let mean = ts.iter().sum::<f64>() / ts.len() as f64;
    (mean, ts[ts.len() * 99 / 100], ts[ts.len() - 1], hits)
}

fn main() {
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        let (kind, size, r) = match parser::parse_document(&src) {
            Ok(Document::Mindmap(d)) => {
                let s = mindmap::scene(&d);
                ("mindmap", format!("{} nodes", s.nodes.len()), time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)))
            }
            Ok(Document::Sequence(d)) => {
                let s = seq::scene(&d);
                ("sequence", format!("{} msgs {} frames", s.messages.len(), s.frames.len()), time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)))
            }
            Ok(Document::Pie(d)) => {
                let s = pie::scene(&d);
                ("pie", format!("{} slices", s.slices.len()), time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)))
            }
            Ok(Document::Journey(d)) => {
                let s = journey::scene(&d);
                ("journey", format!("{} tasks", s.tasks.len()), time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)))
            }
            Ok(Document::Timeline(d)) => {
                let s = timeline::scene(&d);
                let ev: usize = s.sections.iter().flat_map(|x| &x.periods).map(|p| p.events.len()).sum();
                ("timeline", format!("{ev} events"), time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)))
            }
            Ok(Document::Sankey(d)) => {
                let s = sankey::scene(&d);
                ("sankey", format!("{} nodes {} links", s.nodes.len(), s.links.len()), time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)))
            }
            Ok(Document::Flowchart(g)) => {
                let s = flowmaid::scene::scene(&g);
                ("flowchart(Scene)", format!("{} nodes {} edges", s.nodes.len(), s.edges.len()), time(s.width, s.height, |x, y, t| s.hit_test(x, y, t)))
            }
            _ => continue,
        };
        println!("{kind}\t{size}\tmean_us={:.2}\tp99_us={:.2}\tmax_us={:.2}\thits={}\t{path}", r.0, r.1, r.2, r.3);
    }
}
