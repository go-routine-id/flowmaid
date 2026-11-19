use flowmaid::{mindmap, model::Document, parser::parse_document};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let Ok(Document::Mindmap(d)) = parse_document(&src) else { return };
    let ms = mindmap::scene(&d);
    let (x, y): (f64, f64) = (a[2].parse().unwrap(), a[3].parse().unwrap());
    println!("hit {:?}", ms.hit_test(x, y, 0.0));
    for (i, n) in ms.nodes.iter().enumerate() {
        if x >= n.x - 10.0 && x <= n.x + n.w + 10.0 && y >= n.y - 10.0 && y <= n.y + n.h + 10.0 {
            println!("node {i} {:?} x {} y {} w {} h {} -> local ({:.3},{:.3})", n.shape, n.x, n.y, n.w, n.h, x - n.x, y - n.y);
        }
    }
}
