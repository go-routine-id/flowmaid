// Pick cost on a grid of points over each mindmap / other scene:
// usage: pickbench <file.mmd>...   prints name nodes mean_us p99_us max_us
use flowmaid::model::Document;
use flowmaid::{journey, mindmap, parser, pie, sankey, seq, timeline};
use std::time::Instant;

fn grid(w: f64, h: f64, f: &mut dyn FnMut(f64, f64, f64)) -> Vec<f64> {
    let mut ts = Vec::new();
    for tol in [0.0, 4.0] {
        for i in 0..100 {
            for j in 0..100 {
                let (x, y) = (w * (i as f64 + 0.5) / 100.0, h * (j as f64 + 0.5) / 100.0);
                let t = Instant::now();
                f(x, y, tol);
                ts.push(t.elapsed().as_secs_f64() * 1e6);
            }
        }
    }
    ts.sort_by(f64::total_cmp);
    ts
}

fn main() {
    for f in std::env::args().skip(1) {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let Ok(doc) = parser::parse_document(&src) else { continue };
        let (n, ts) = match doc {
            Document::Mindmap(d) => {
                let s = mindmap::scene(&d);
                (s.nodes.len(), grid(s.width, s.height, &mut |x, y, t| { std::hint::black_box(s.hit_test(x, y, t)); }))
            }
            Document::Sequence(d) => {
                let s = seq::scene(&d);
                (s.messages.len(), grid(s.width, s.height, &mut |x, y, t| { std::hint::black_box(s.hit_test(x, y, t)); }))
            }
            Document::Sankey(d) => {
                let s = sankey::scene(&d);
                (s.links.len(), grid(s.width, s.height, &mut |x, y, t| { std::hint::black_box(s.hit_test(x, y, t)); }))
            }
            Document::Pie(d) => {
                let s = pie::scene(&d);
                (s.slices.len(), grid(s.width, s.height, &mut |x, y, t| { std::hint::black_box(s.hit_test(x, y, t)); }))
            }
            Document::Journey(d) => {
                let s = journey::scene(&d);
                (0, grid(s.width, s.height, &mut |x, y, t| { std::hint::black_box(s.hit_test(x, y, t)); }))
            }
            Document::Timeline(d) => {
                let s = timeline::scene(&d);
                (s.sections.len(), grid(s.width, s.height, &mut |x, y, t| { std::hint::black_box(s.hit_test(x, y, t)); }))
            }
            _ => continue,
        };
        let mean = ts.iter().sum::<f64>() / ts.len() as f64;
        let p99 = ts[ts.len() * 99 / 100];
        let name = f.rsplit('/').next().unwrap_or(&f);
        println!("{name:<28} {n:>6} mean {mean:>9.2} us  p99 {p99:>9.2} us  max {:>9.2} us", ts[ts.len() - 1]);
    }
}
