//! Class-diagram defects per input: curved edges, crossings (true polyline intersections), merged runs.
use flowmaid::model::Document;
use flowmaid::scene::SceneEdge;

fn edge_polyline(e: &SceneEdge) -> Vec<(f64, f64)> {
    if !e.waypoints.is_empty() {
        return e.waypoints.clone();
    }
    let b = e.bezier;
    (0..=32)
        .map(|k| {
            let t = k as f64 / 32.0;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            (
                w[0] * b[0].0 + w[1] * b[1].0 + w[2] * b[2].0 + w[3] * b[3].0,
                w[0] * b[0].1 + w[1] * b[1].1 + w[2] * b[2].1 + w[3] * b[3].1,
            )
        })
        .collect()
}

fn cross(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let s = |o: (f64, f64), p: (f64, f64), q: (f64, f64)| (p.0 - o.0) * (q.1 - o.1) - (p.1 - o.1) * (q.0 - o.0);
    let (d1, d2, d3, d4) = (s(c, d, a), s(c, d, b), s(a, b, c), s(a, b, d));
    d1 * d2 < -1e-9 && d3 * d4 < -1e-9
}

fn curved(e: &SceneEdge) -> bool {
    if e.from == e.to || !e.waypoints.is_empty() {
        return false;
    }
    let (a, b) = (e.bezier[0], e.bezier[3]);
    !((a.0 - b.0).abs() < 0.5 || (a.1 - b.1).abs() < 0.5)
}

fn main() {
    let mut tot = (0, 0);
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let d = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Class(d)) => d,
            Ok(_) => continue,
            Err(e) => {
                println!("{f}\tERR {e:?}");
                continue;
            }
        };
        let cs = flowmaid::class::scene(&d);
        let es = &cs.scene.edges;
        let lines: Vec<Vec<(f64, f64)>> = es.iter().map(edge_polyline).collect();
        let mut x = 0;
        let mut pairs = Vec::new();
        for i in 0..es.len() {
            for j in i + 1..es.len() {
                if es[i].from == es[i].to || es[j].from == es[j].to {
                    continue;
                }
                let mut n = 0;
                for p in lines[i].windows(2) {
                    for q in lines[j].windows(2) {
                        if cross(p[0], p[1], q[0], q[1]) {
                            n += 1;
                        }
                    }
                }
                if n > 0 {
                    x += n;
                    pairs.push(format!("{}>{} x {}>{} ({n})", es[i].from, es[i].to, es[j].from, es[j].to));
                }
            }
        }
        let c = es.iter().filter(|e| curved(e)).count();
        tot.0 += x;
        tot.1 += c;
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        println!("{name}\tcross={x}\tcurved={c}\t{}x{}\t{}", cs.scene.width.round(), cs.scene.height.round(), pairs.join("; "));
    }
    println!("TOTAL cross={} curved={}", tot.0, tot.1);
}
