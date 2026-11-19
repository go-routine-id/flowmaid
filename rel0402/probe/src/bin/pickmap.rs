use flowmaid::model::Document;
use flowmaid::{journey, mindmap, parser, pie, sankey, seq, timeline};
fn overlay(svg: &str, w: f64, h: f64, step: f64, f: &dyn Fn(f64, f64) -> Option<String>) -> String {
    let pal = ["#e6194b", "#3cb44b", "#4363d8", "#f58231", "#911eb4", "#46f0f0", "#f032e6", "#bcf60c", "#008080", "#9a6324"];
    let mut s = String::new();
    let mut y = 0.0;
    while y <= h {
        let mut x = 0.0;
        while x <= w {
            if let Some(k) = f(x, y) {
                let hsh = k.bytes().fold(7u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
                s.push_str(&format!("<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{step}\" height=\"{step}\" fill=\"{}\" fill-opacity=\"0.45\"/>", x - step / 2.0, y - step / 2.0, pal[(hsh % 10) as usize]));
            }
            x += step;
        }
        y += step;
    }
    svg.replace("</svg>", &format!("{s}</svg>"))
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = std::path::Path::new(&a[1]);
    let tol: f64 = a[2].parse().unwrap();
    for f in &a[3..] {
        let src = std::fs::read_to_string(f).unwrap();
        let stem = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let st = 2.0;
        let svg = match parser::parse_document(&src).unwrap() {
            Document::Mindmap(d) => { let s = mindmap::scene(&d); overlay(&mindmap::to_svg(&s), s.width, s.height, st, &|x, y| s.hit_test(x, y, tol).map(|h| format!("{h:?}"))) }
            Document::Sequence(d) => { let s = seq::scene(&d); overlay(&seq::to_svg(&s), s.width, s.height, st, &|x, y| s.hit_test(x, y, tol).map(|h| format!("{h:?}"))) }
            Document::Pie(d) => { let s = pie::scene(&d); overlay(&pie::to_svg(&s), s.width, s.height, st, &|x, y| s.hit_test(x, y, tol).map(|h| format!("{h:?}"))) }
            Document::Journey(d) => { let s = journey::scene(&d); overlay(&journey::to_svg(&s), s.width, s.height, st, &|x, y| s.hit_test(x, y, tol).map(|h| format!("{h:?}"))) }
            Document::Timeline(d) => { let s = timeline::scene(&d); overlay(&timeline::to_svg(&s), s.width, s.height, st, &|x, y| s.hit_test(x, y, tol).map(|h| format!("{h:?}"))) }
            Document::Sankey(d) => { let s = sankey::scene(&d); overlay(&sankey::to_svg(&s), s.width, s.height, st, &|x, y| s.hit_test(x, y, tol).map(|h| format!("{h:?}"))) }
            _ => continue,
        };
        std::fs::write(out.join(format!("{stem}.svg")), svg).unwrap();
    }
}
