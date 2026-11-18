//! Drag benchmark: `cargo run --release --example drag_bench [-- frames]`
//!
//! Per-frame cost of following one dragged node in a large ER, class
//! and mindmap diagram: the full `route` against the incremental
//! `route_partial` (issue #111). Each diagram is laid out once
//! (`scene`); then one node in its middle is dragged `frames` steps
//! (default 40) of 6 px right and 3 px down, and every step is routed
//! both ways from that laid-out base, then once more incrementally
//! from the frame before it, as a host following a drag does. It
//! prints one row per diagram:
//! `diagram  nodes  edges  route_ms  partial_ms  chained_ms  speedup`,
//! the times the median over the frames, the speedup `route_ms` over
//! `partial_ms`. The times depend on the machine; compare within a run.

use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser};
use std::time::Instant;

/// Deterministic LCG to stay pure-std with reproducible results.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

/// `layers` x `width` entities, each related to one in the next layer.
fn er_src(layers: usize, width: usize) -> String {
    let mut s = String::from("erDiagram\n");
    let mut rng = Lcg(9);
    let cards = ["||--o{", "||--|{", "}o--o{", "|o--||"];
    for l in 0..layers - 1 {
        for w in 0..width {
            let (a, b) = (l * width + w, (l + 1) * width + rng.next(width));
            let c = cards[rng.next(cards.len())];
            s.push_str(&format!("E{a} {c} E{b} : rel{a}\n"));
        }
    }
    for i in 0..layers * width {
        s.push_str(&format!("E{i} {{\n  int id PK\n  string name\n}}\n"));
    }
    s
}

/// `layers` x `width` classes, each related to one in the next layer,
/// a third of the relations labelled with cardinalities.
fn class_src(layers: usize, width: usize) -> String {
    let mut s = String::from("classDiagram\n");
    let mut rng = Lcg(7);
    for i in 0..layers * width {
        s.push_str(&format!("class C{i} {{\n  +int id\n  +name() String\n}}\n"));
    }
    let kinds = ["<|--", "*--", "o--", "-->", "..>", "..|>"];
    for l in 0..layers - 1 {
        for w in 0..width {
            let (a, b) = (l * width + w, (l + 1) * width + rng.next(width));
            let k = kinds[rng.next(kinds.len())];
            if rng.next(3) == 0 {
                s.push_str(&format!("C{a} \"1\" {k} \"*\" C{b} : uses\n"));
            } else {
                s.push_str(&format!("C{a} {k} C{b}\n"));
            }
        }
    }
    s
}

/// A root with `branches` branches of `leaves` leaves each.
fn mind_src(branches: usize, leaves: usize) -> String {
    let mut s = String::from("mindmap\n  root((Root))\n");
    for b in 0..branches {
        s.push_str(&format!("    Branch {b}\n"));
        for l in 0..leaves {
            s.push_str(&format!("      Leaf {b}.{l}\n"));
        }
    }
    s
}

/// Median of the per-frame times (ms) of `f` over every frame.
fn per_frame<T>(frames: &[Vec<(f64, f64)>], mut f: impl FnMut(&[(f64, f64)]) -> T) -> f64 {
    let mut ts: Vec<f64> = frames
        .iter()
        .map(|p| {
            let t = Instant::now();
            std::hint::black_box(f(p));
            t.elapsed().as_secs_f64() * 1e3
        })
        .collect();
    ts.sort_by(f64::total_cmp);
    ts[ts.len() / 2]
}

/// The drag of node `i` from `at`: `n` steps of (6, 3) px.
fn drag(at: &[(f64, f64)], i: usize, n: usize) -> Vec<Vec<(f64, f64)>> {
    (1..=n)
        .map(|k| {
            let mut p = at.to_vec();
            p[i] = (at[i].0 + 6.0 * k as f64, at[i].1 + 3.0 * k as f64);
            p
        })
        .collect()
}

fn row(name: &str, nodes: usize, edges: usize, full: f64, part: f64, chained: f64) {
    println!(
        "{name:<8} {nodes:>6} {edges:>6} {full:>9.3} {part:>11.3} {chained:>11.3} {:>8.1}x",
        full / part
    );
}

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(40);
    println!(
        "{:<8} {:>6} {:>6} {:>9} {:>11} {:>11} {:>9}",
        "diagram", "nodes", "edges", "route_ms", "partial_ms", "chained_ms", "speedup"
    );
    if let Ok(Document::Er(d)) = parser::parse_document(&er_src(12, 20)) {
        let base = er::scene(&d);
        let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let f = drag(&at, at.len() / 2, frames);
        let full = per_frame(&f, |p| er::route(&d, p));
        let part = per_frame(&f, |p| er::route_partial(&d, p, &base, &at));
        let mut prev = (base.clone(), at.clone());
        let chained = per_frame(&f, |p| {
            prev = (er::route_partial(&d, p, &prev.0, &prev.1), p.to_vec());
        });
        row("er", at.len(), base.scene.edges.len(), full, part, chained);
    }
    if let Ok(Document::Class(d)) = parser::parse_document(&class_src(12, 20)) {
        let base = class::scene(&d);
        let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let f = drag(&at, at.len() / 2, frames);
        let full = per_frame(&f, |p| class::route(&d, p));
        let part = per_frame(&f, |p| class::route_partial(&d, p, &base, &at));
        let mut prev = (base.clone(), at.clone());
        let chained = per_frame(&f, |p| {
            prev = (class::route_partial(&d, p, &prev.0, &prev.1), p.to_vec());
        });
        row(
            "class",
            at.len(),
            base.scene.edges.len(),
            full,
            part,
            chained,
        );
    }
    if let Ok(Document::Mindmap(d)) = parser::parse_document(&mind_src(40, 25)) {
        let base = mindmap::scene(&d);
        let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
        let f = drag(&at, at.len() / 2, frames);
        let full = per_frame(&f, |p| mindmap::route(&d, p));
        let part = per_frame(&f, |p| mindmap::route_partial(&d, p, &base, &at));
        let mut prev = (base.clone(), at.clone());
        let chained = per_frame(&f, |p| {
            prev = (mindmap::route_partial(&d, p, &prev.0, &prev.1), p.to_vec());
        });
        row("mindmap", at.len(), base.edges.len(), full, part, chained);
    }
}
