use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser};
use std::time::Instant;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

fn class_src(layers: usize, width: usize) -> String {
    let mut s = String::from("classDiagram\n");
    let mut r = Lcg(7);
    for l in 0..layers {
        for w in 0..width {
            let i = l * width + w;
            s.push_str(&format!("class C{i} {{\n  +int id\n  +name() String\n}}\n"));
        }
    }
    let kinds = ["<|--", "*--", "o--", "-->", "..>", "..|>"];
    for l in 0..layers - 1 {
        for w in 0..width {
            let a = l * width + w;
            let b = (l + 1) * width + r.next(width);
            let k = kinds[r.next(kinds.len())];
            if r.next(3) == 0 {
                s.push_str(&format!("C{a} \"1\" {k} \"*\" C{b} : uses\n"));
            } else {
                s.push_str(&format!("C{a} {k} C{b}\n"));
            }
        }
    }
    s
}

fn er_src(layers: usize, width: usize) -> String {
    let mut s = String::from("erDiagram\n");
    let mut r = Lcg(9);
    let cards = ["||--o{", "||--|{", "}o--o{", "|o--||"];
    for l in 0..layers - 1 {
        for w in 0..width {
            let a = l * width + w;
            let b = (l + 1) * width + r.next(width);
            s.push_str(&format!("E{a} {} E{b} : rel{a}\n", cards[r.next(cards.len())]));
        }
    }
    for l in 0..layers {
        for w in 0..width {
            let i = l * width + w;
            s.push_str(&format!("E{i} {{\n  int id PK\n  string name\n}}\n"));
        }
    }
    s
}

fn time<T>(n: usize, mut f: impl FnMut() -> T) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..n {
        let t = Instant::now();
        std::hint::black_box(f());
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
    }
    best
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (l, w) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    if let Ok(Document::Class(d)) = parser::parse_document(&class_src(l, w)) {
        let cs = class::scene(&d);
        let c: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut m = c.clone();
        m[c.len() / 2].0 += 20.0;
        println!("class n={} e={} route {:.2} ms", c.len(), cs.scene.edges.len(), time(3, || class::route(&d, &m)));
    }
    if let Ok(Document::Er(d)) = parser::parse_document(&er_src(l, w)) {
        let es = er::scene(&d);
        let c: Vec<(f64, f64)> = es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut m = c.clone();
        m[c.len() / 2].0 += 20.0;
        println!("er n={} e={} route {:.2} ms", c.len(), es.scene.edges.len(), time(3, || er::route(&d, &m)));
    }
    let mut src = String::from("mindmap\n  root\n");
    for i in 0..w {
        src.push_str(&format!("    b{i}\n"));
        for j in 0..l {
            src.push_str(&format!("      l{i}x{j}\n"));
        }
    }
    if let Ok(Document::Mindmap(d)) = parser::parse_document(&src) {
        let ms = mindmap::scene(&d);
        let c: Vec<(f64, f64)> = ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
        println!("mind n={} route {:.3} ms", c.len(), time(20, || mindmap::route(&d, &c)));
    }
}
