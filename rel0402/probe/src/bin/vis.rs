use flowmaid::model::Document;
use flowmaid::{class, er, parser};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = std::path::Path::new(&a[1]);
    for f in &a[2..] {
        let src = std::fs::read_to_string(f).unwrap();
        let stem = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        match parser::parse_document(&src).unwrap() {
            Document::Class(d) => {
                let base = class::scene(&d);
                let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                for i in [0, at.len() / 2] {
                    let mut to = at.clone();
                    to[i].0 += 70.0; to[i].1 += 45.0;
                    std::fs::write(out.join(format!("{stem}-{i}-partial.svg")), class::to_svg(&class::route_partial(&d, &to, &base, &at))).unwrap();
                    std::fs::write(out.join(format!("{stem}-{i}-full.svg")), class::to_svg(&class::route(&d, &to))).unwrap();
                }
            }
            Document::Er(d) => {
                let base = er::scene(&d);
                let at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                for i in [0, at.len() / 2] {
                    let mut to = at.clone();
                    to[i].0 += 70.0; to[i].1 += 45.0;
                    std::fs::write(out.join(format!("{stem}-{i}-partial.svg")), er::to_svg(&er::route_partial(&d, &to, &base, &at))).unwrap();
                    std::fs::write(out.join(format!("{stem}-{i}-full.svg")), er::to_svg(&er::route(&d, &to))).unwrap();
                }
            }
            _ => {}
        }
    }
}
