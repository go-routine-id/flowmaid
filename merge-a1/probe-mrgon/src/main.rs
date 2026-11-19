//! Identity probe: per input, FNV-1a hashes of render_svg, and for a
//! flowchart or state diagram of scene()/route()/route_partial() (SVG
//! and Debug geometry). One line per input; a panic prints PANIC.
use flowmaid::model::Document;
use flowmaid::scene::{route, route_partial, scene, to_svg};

/// The Debug text without the field 0.39.0 added (`plain_crossings`),
/// so geometry compares across the merge.
fn strip(s: String) -> String {
    s.replace(", plain_crossings: false", "")
}

fn h(s: &str) -> String {
    let mut x: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        x ^= *b as u64;
        x = x.wrapping_mul(0x100000001b3);
    }
    format!("{x:016x}")
}

fn row(src: &str) -> String {
    let svg = match flowmaid::render_svg(src) {
        Ok(s) => h(&s),
        Err(_) => match flowmaid::render_advance_text_svg(src) {
            Ok(s) => h(&s),
            Err(_) => "ERR".into(),
        },
    };
    let mut out = vec![svg];
    if let Ok(Document::Flowchart(g) | Document::State(g)) = flowmaid::parser::parse_document(src) {
        if !g.nodes.is_empty() {
            let base = scene(&g);
            out.push(h(&to_svg(&base)));
            out.push(h(&strip(format!("{base:?}"))));
            let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &at);
            out.push(h(&to_svg(&r)));
            out.push(h(&strip(format!("{r:?}"))));
            let mut moved = at.clone();
            moved[0] = (at[0].0 + 7.0, at[0].1 + 5.0);
            let p = route_partial(&g, &moved, &base, &at);
            out.push(h(&to_svg(&p)));
            out.push(h(&strip(format!("{p:?}"))));
        }
    }
    out.join("\t")
}

fn main() {
    for f in std::env::args().skip(1) {
        let Ok(src) = std::fs::read_to_string(&f) else {
            println!("{f}\tUNREADABLE");
            continue;
        };
        match std::panic::catch_unwind(|| row(&src)) {
            Ok(r) => println!("{f}\t{r}"),
            Err(_) => println!("{f}\tPANIC"),
        }
    }
}
