use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::class;
fn view_box(svg: &str) -> Option<(f64, f64, f64, f64)> {
    let i = svg.find("viewBox=\"")? + 9;
    let rest = &svg[i..];
    let j = rest.find('"')?;
    let v: Vec<f64> = rest[..j]
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    (v.len() == 4).then(|| (v[0], v[1], v[2], v[3]))
}

fn sample(n: usize) -> Vec<usize> {
    let mut v = vec![0];
    if n > 1 {
        v.push(n - 1);
    }
    if n > 2 {
        v.push(n / 2);
    }
    if n > 5 {
        v.push(n / 3);
    }
    v.sort();
    v.dedup();
    v
}

const DELTAS: [(f64, f64); 4] = [(37.0, 23.0), (-3000.0, -2000.0), (1500.0, 900.0), (-20.0, 0.5)];


/// Every drawn coordinate of `svg` (path, line, polygon, rect corners,
/// circle box, text anchor) inside its viewBox (to `slack` px), or the
/// worst overshoot.
fn ink_outside(svg: &str, slack: f64) -> f64 {
    let Some((vx, vy, vw, vh)) = view_box(svg) else { return f64::INFINITY };
    let mut worst: f64 = 0.0;
    let mut pt = |x: f64, y: f64| {
        let o = (vx - x).max(x - (vx + vw)).max(vy - y).max(y - (vy + vh));
        if o > slack { eprintln!("   outside by {o:.1}: <{}", TAGNOW.with(|t| t.borrow().clone()));
            worst = worst.max(o);
        }
    };
    let attr = |tag: &str, name: &str| -> Option<f64> {
        let k = format!(" {name}=\"");
        let i = tag.find(&k)? + k.len();
        let j = tag[i..].find('"')?;
        tag[i..i + j].parse().ok()
    };
    for raw in svg.split('<').skip(1) {
        let tag = raw.split('>').next().unwrap_or("");
        let name = tag.split_whitespace().next().unwrap_or(""); TAGNOW.with(|t| *t.borrow_mut() = tag.chars().take(160).collect());
        match name {
            "path" => {
                if let Some(i) = tag.find(" d=\"") {
                    let rest = &tag[i + 4..];
                    let d = &rest[..rest.find('"').unwrap_or(rest.len())];
                    let nums: Vec<f64> = d
                        .split(|c: char| c.is_ascii_alphabetic() || c == ',' || c.is_whitespace())
                        .filter(|t| !t.is_empty())
                        .filter_map(|t| t.parse().ok())
                        .collect();
                    for c in nums.chunks(2) {
                        if c.len() == 2 {
                            pt(c[0], c[1]);
                        }
                    }
                }
            }
            "line" => {
                if let (Some(a), Some(b), Some(c), Some(d)) = (attr(tag, "x1"), attr(tag, "y1"), attr(tag, "x2"), attr(tag, "y2")) {
                    pt(a, b);
                    pt(c, d);
                }
            }
            "polygon" | "polyline" => {
                if let Some(i) = tag.find(" points=\"") {
                    let rest = &tag[i + 9..];
                    let d = &rest[..rest.find('"').unwrap_or(rest.len())];
                    let nums: Vec<f64> = d
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .filter_map(|t| t.parse().ok())
                        .collect();
                    for c in nums.chunks(2) {
                        if c.len() == 2 {
                            pt(c[0], c[1]);
                        }
                    }
                }
            }
            "rect" => {
                if let (Some(x), Some(y), Some(w), Some(h)) = (attr(tag, "x"), attr(tag, "y"), attr(tag, "width"), attr(tag, "height")) {
                    pt(x, y);
                    pt(x + w, y + h);
                }
            }
            "circle" => {
                if let (Some(x), Some(y), Some(r)) = (attr(tag, "cx"), attr(tag, "cy"), attr(tag, "r")) {
                    pt(x - r, y - r);
                    pt(x + r, y + r);
                }
            }
            "text" => {
                if let (Some(x), Some(y)) = (attr(tag, "x"), attr(tag, "y")) {
                    pt(x, y);
                }
            }
            _ => {}
        }
    }
    worst
}


thread_local!(static TAGNOW: std::cell::RefCell<String> = std::cell::RefCell::new(String::new()));
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&path).unwrap();
    let Ok(Document::Class(d)) = parse_document(&src) else { return };
    let base = class::scene(&d);
    let all: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    eprintln!("static scene:"); let o = ink_outside(&class::to_svg(&base), 0.5); println!("static {o}");
    let r = class::route(&d, &all);
    eprintln!("route at:"); let o = ink_outside(&class::to_svg(&r), 0.5); println!("route-at {o}");
    let mut mv = all.clone(); mv[0].0 += 37.0; mv[0].1 += 23.0;
    let r = class::route(&d, &mv);
    eprintln!("route mv:"); let o = ink_outside(&class::to_svg(&r), 0.5); println!("route-mv {o}");
    let f = class::route_partial(&d, &mv, &base, &all);
    eprintln!("frame mv:"); let o = ink_outside(&class::to_svg(&f), 0.5); println!("frame-mv {o} origin {:?} w {} h {}", f.scene.origin, f.scene.width, f.scene.height);
    println!("viewBox {:?}", view_box(&class::to_svg(&f)));
}
