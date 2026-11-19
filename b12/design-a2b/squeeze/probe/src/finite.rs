//! Block mode: the steps of `routing_stays_finite_for_any_node_sizes_and_coincident_centres`, timed per file and step.
use flowmaid::model::Document;
use flowmaid::quality::with_blocks;
use flowmaid::scene;
use std::io::Write;
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) { Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g, _ => continue };
        with_blocks(true, || {
            let t = |what: &str, t0: std::time::Instant| { println!("{f} {what} {:.1} ms", t0.elapsed().as_secs_f64() * 1e3); std::io::stdout().flush().unwrap(); };
            let t0 = std::time::Instant::now();
            let base = scene::scene(&g); t("scene", t0);
            let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
            let own: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.w, n.h)).collect();
            let n = at.len();
            let sets: Vec<(&str, Vec<(f64, f64)>)> = vec![
                ("all zero", vec![(0.0, 0.0); n]),
                ("some zero", (0..n).map(|i| match i % 3 { 0 => (0.0, 0.0), 1 => (own[i].0, 0.0), _ => own[i] }).collect()),
                ("negative", own.iter().map(|&(w, h)| (-w, -h)).collect()),
                ("non-finite", (0..n).map(|i| match i % 3 { 0 => (f64::NAN, f64::NAN), 1 => (f64::INFINITY, own[i].1), _ => (own[i].0, f64::NEG_INFINITY) }).collect()),
                ("huge", vec![(1e6, 1e6); n]),
            ];
            for (name, sizes) in &sets { println!("{f} start route_sized {name}"); std::io::stdout().flush().unwrap(); let t0 = std::time::Instant::now(); let _ = scene::route_sized(&g, &at, sizes); t(&format!("route_sized {name}"), t0); }
            for k in [0usize, 3] { println!("{f} start scene_sized {}", sets[k].0); std::io::stdout().flush().unwrap(); let t0 = std::time::Instant::now(); let _ = scene::scene_sized(&g, &sets[k].1); t(&format!("scene_sized {}", sets[k].0), t0); }
            let pairs: Vec<(f64, f64)> = (0..n).map(|i| at[i - i % 2]).collect();
            println!("{f} start pairs"); std::io::stdout().flush().unwrap();
            let t0 = std::time::Instant::now(); let _ = scene::route_sized(&g, &pairs, &sets[0].1); t("pairs", t0);
            let t0 = std::time::Instant::now(); let _ = scene::route(&g, &vec![(40.0, 40.0); n]); t("one point", t0);
            let t0 = std::time::Instant::now(); let _ = scene::route_partial(&g, &pairs, &base, &at); t("partial", t0);
        });
    }
}
