// Emit JSON per fixture: the painter's SVG and Rust's exact picks on a grid.
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::{journey, mindmap, pie, sankey, seq, timeline};
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}
fn main() {
    let list = std::env::args().nth(1).unwrap();
    let out = std::env::args().nth(2).unwrap();
    let n: usize = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(120);
    for path in std::fs::read_to_string(&list).unwrap().lines() {
        let Ok(src) = std::fs::read_to_string(path) else { continue };
        let name = path.rsplit('/').next().unwrap().to_string();
        let (ty, svg, w, h, pick): (&str, String, f64, f64, Box<dyn Fn(f64, f64) -> String>) = match parse_document(&src) {
            Ok(Document::Mindmap(d)) => { let s = mindmap::scene(&d); let svg = mindmap::to_svg(&s); let (w, h) = (s.width, s.height); ("mind", svg, w, h, Box::new(move |x, y| match s.hit_test(x, y, 0.0) { Some(v) => format!("{v:?}"), None => "null".into() })) }
            Ok(Document::Sequence(d)) => { let s = seq::scene(&d); let svg = seq::to_svg(&s); let (w, h) = (s.width, s.height); ("seq", svg, w, h, Box::new(move |x, y| match s.hit_test(x, y, 0.0) { Some(seq::SeqHit::FrameInside(_)) | None => "null".into(), Some(v) => format!("{v:?}") })) }
            Ok(Document::Pie(d)) => { let s = pie::scene(&d); let svg = pie::to_svg(&s); let (w, h) = (s.width, s.height); ("pie", svg, w, h, Box::new(move |x, y| match s.hit_test(x, y, 0.0) { Some(v) => format!("{v:?}"), None => "null".into() })) }
            Ok(Document::Journey(d)) => { let s = journey::scene(&d); let svg = journey::to_svg(&s); let (w, h) = (s.width, s.height); ("journey", svg, w, h, Box::new(move |x, y| match s.hit_test(x, y, 0.0) { Some(v) => format!("{v:?}"), None => "null".into() })) }
            Ok(Document::Timeline(d)) => { let s = timeline::scene(&d); let svg = timeline::to_svg(&s); let (w, h) = (s.width, s.height); ("timeline", svg, w, h, Box::new(move |x, y| match s.hit_test(x, y, 0.0) { Some(v) => format!("{v:?}"), None => "null".into() })) }
            Ok(Document::Sankey(d)) => { let s = sankey::scene(&d); let svg = sankey::to_svg(&s); let (w, h) = (s.width, s.height); ("sankey", svg, w, h, Box::new(move |x, y| match s.hit_test(x, y, 0.0) { Some(v) => format!("{v:?}"), None => "null".into() })) }
            _ => continue,
        };
        let mut pts = Vec::new();
        let mut rust = Vec::new();
        for i in 0..n {
            for j in 0..n {
                let x = (i as f64 + 0.413) * w / n as f64;
                let y = (j as f64 + 0.587) * h / n as f64;
                pts.push(format!("{x:.4},{y:.4}"));
                rust.push(format!("\"{}\"", esc(&pick(x, y))));
            }
        }
        let js = format!("{{\"file\":\"{}\",\"type\":\"{ty}\",\"w\":{w},\"h\":{h},\"svg\":\"{}\",\"pts\":[{}],\"rust\":[{}]}}", esc(&name), esc(&svg), pts.join(","), rust.join(","));
        std::fs::write(format!("{out}/{ty}-{name}.json"), js).unwrap();
    }
}
