//! For each input: write <out>/<name>.svg (the scene's to_svg) and
//! <out>/<name>.hits.tsv (x, y, hit_test(x, y, 0) Debug) over a grid.
use flowmaid::model::Document;
use flowmaid::{journey, mindmap, parser, pie, sankey, seq, timeline};
use std::fmt::Write as _;

fn grid<H: std::fmt::Debug>(w: f64, h: f64, step: f64, f: impl Fn(f64, f64) -> Option<H>) -> String {
    let mut s = String::new();
    let mut y = 0.5;
    while y < h {
        let mut x = 0.5;
        while x < w {
            let r = f(x, y);
            let r = match r {
                Some(r) => format!("{r:?}").trim_matches('"').to_string(),
                None => "None".into(),
            };
            writeln!(s, "{x}\t{y}\t{r}").unwrap();
            x += step;
        }
        y += step;
    }
    s
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap();
    let step: f64 = args.next().unwrap().parse().unwrap();
    for path in args {
        let src = std::fs::read_to_string(&path).unwrap();
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().to_string();
        let (svg, hits) = match parser::parse_document(&src) {
            Ok(Document::Mindmap(d)) => {
                let s = mindmap::scene(&d);
                (mindmap::to_svg(&s), grid(s.width, s.height, step, |x, y| s.hit_test(x, y, 0.0)))
            }
            Ok(Document::Pie(d)) => {
                let s = pie::scene(&d);
                (pie::to_svg(&s), grid(s.width, s.height, step, |x, y| s.hit_test(x, y, 0.0).map(|h| match h {
                    pie::PieHit::Slice(i) => format!("Slice({i})|{}|{}", pie::slice_color(i), pie::slice_hatch(i).is_some()),
                    pie::PieHit::Legend(i) => format!("Legend({i})|{}|{}", pie::slice_color(i), pie::slice_hatch(i).is_some()),
                    o => format!("{o:?}"),
                })))
            }
            Ok(Document::Sankey(d)) => {
                let s = sankey::scene(&d);
                (sankey::to_svg(&s), grid(s.width, s.height, step, |x, y| s.hit_test(x, y, 0.0)))
            }
            Ok(Document::Journey(d)) => {
                let s = journey::scene(&d);
                (journey::to_svg(&s), grid(s.width, s.height, step, |x, y| s.hit_test(x, y, 0.0)))
            }
            Ok(Document::Timeline(d)) => {
                let s = timeline::scene(&d);
                (timeline::to_svg(&s), grid(s.width, s.height, step, |x, y| s.hit_test(x, y, 0.0)))
            }
            Ok(Document::Sequence(d)) => {
                let s = seq::scene(&d);
                (seq::to_svg(&s), grid(s.width, s.height, step, |x, y| s.hit_test(x, y, 0.0)))
            }
            _ => continue,
        };
        std::fs::write(format!("{out}/{name}.svg"), svg).unwrap();
        std::fs::write(format!("{out}/{name}.hits.tsv"), hits).unwrap();
    }
}
