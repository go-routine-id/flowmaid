// cards <class.mmd> <class index> <x> <y> <out.svg>: drag a class, print
// card_spots and the SVG's 11px card texts.
use flowmaid::class;
use flowmaid::model::Document;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let Document::Class(d) = flowmaid::parser::parse_document(&src).unwrap() else { panic!() };
    let laid = class::scene(&d);
    let mut c: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let n = d.classes.len();
    let k: usize = match a[2].as_str() {
        "left" => (0..n).min_by(|&i, &j| c[i].0.partial_cmp(&c[j].0).unwrap()).unwrap(),
        "mid" => n / 2,
        s => s.parse().unwrap(),
    };
    let rel = |s: &str, cur: f64| -> f64 { if let Some(d) = s.strip_prefix('+') { cur + d.parse::<f64>().unwrap() } else { s.parse().unwrap() } };
    if a[3] == "all" {
        let (dx, dy): (f64, f64) = (a[4].parse().unwrap(), a[5].parse().unwrap());
        for p in c.iter_mut() { p.0 += dx; p.1 += dy; }
    } else {
        c[k] = (rel(&a[3], c[k].0), rel(&a[4], c[k].1));
    }
    let cs = class::route(&d, &c);
    let svg = class::to_svg(&cs);
    std::fs::write(&a[a.len() - 1], &svg).unwrap();
    for (j, s) in class::card_spots(&cs).iter().enumerate() {
        for (w, p) in s.iter().enumerate() {
            if let Some(p) = p { println!("spot rel {j} end {w}: ({:.1}, {:.1})", p.0, p.1); }
        }
    }
    for l in svg.lines().filter(|l| l.contains("font-size=\"11\"")) {
        println!("svg: {}", &l[..l.len().min(140)]);
    }
}
