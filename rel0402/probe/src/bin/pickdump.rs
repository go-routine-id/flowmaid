// Every pick on a 200x200 grid (tol 0, 1.5 and 4) of each scene, one
// line per file and tol: usage pickdump <file.mmd>...
use flowmaid::model::Document;
use flowmaid::{journey, mindmap, parser, pie, sankey, seq, timeline};

fn grid(name: &str, w: f64, h: f64, f: &dyn Fn(f64, f64, f64) -> String) {
    let g: usize = std::env::var("GRID").ok().and_then(|v| v.parse().ok()).unwrap_or(200);
    for tol in [0.0, 1.5, 4.0] {
        let mut s = String::new();
        for i in 0..g {
            for j in 0..g {
                let (x, y) = (w * (i as f64 + 0.37) / g as f64, h * (j as f64 + 0.61) / g as f64);
                s.push_str(&f(x, y, tol));
                s.push(';');
            }
        }
        println!("{name} tol={tol} {s}");
    }
}

fn main() {
    for f in std::env::args().skip(1) {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let Ok(doc) = parser::parse_document(&src) else { continue };
        let name = f.rsplit('/').next().unwrap_or(&f).to_string();
        match doc {
            Document::Mindmap(d) => {
                let s = mindmap::scene(&d);
                grid(&name, s.width, s.height, &|x, y, t| format!("{:?}", s.hit_test(x, y, t)));
            }
            Document::Sequence(d) => {
                let s = seq::scene(&d);
                grid(&name, s.width, s.height, &|x, y, t| format!("{:?}", s.hit_test(x, y, t)));
            }
            Document::Sankey(d) => {
                let s = sankey::scene(&d);
                grid(&name, s.width, s.height, &|x, y, t| format!("{:?}", s.hit_test(x, y, t)));
            }
            Document::Pie(d) => {
                let s = pie::scene(&d);
                grid(&name, s.width, s.height, &|x, y, t| format!("{:?}", s.hit_test(x, y, t)));
            }
            Document::Journey(d) => {
                let s = journey::scene(&d);
                grid(&name, s.width, s.height, &|x, y, t| format!("{:?}", s.hit_test(x, y, t)));
            }
            Document::Timeline(d) => {
                let s = timeline::scene(&d);
                grid(&name, s.width, s.height, &|x, y, t| format!("{:?}", s.hit_test(x, y, t)));
            }
            _ => {}
        }
    }
}
