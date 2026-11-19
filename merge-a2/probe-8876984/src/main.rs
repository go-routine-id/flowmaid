//! Identity probe: per input, FNV-1a hashes of render_svg, and for a
//! flowchart or state diagram of scene()/route()/route_partial() (SVG
//! and Debug geometry, with the new crate-private `block_layout: false`
//! field stripped so both commits print the same struct). With DUMP=dir
//! it also writes the full texts for diffing.
use flowmaid::model::Document;
use flowmaid::scene::{route, route_partial, scene, to_svg};

fn h(s: &str) -> String {
    let mut x: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        x ^= *b as u64;
        x = x.wrapping_mul(0x100000001b3);
    }
    format!("{x:016x}")
}

fn dbg(sc: &flowmaid::scene::Scene) -> String {
    let s = format!("{sc:?}");
    let n = s.matches("block_layout: ").count();
    let t = s.replace(", block_layout: false", "");
    // Any `block_layout: true` stays and so shows as a difference.
    let _ = n;
    t
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
            out.push(h(&dbg(&base)));
            let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &at);
            out.push(h(&to_svg(&r)));
            out.push(h(&dbg(&r)));
            let mut moved = at.clone();
            moved[0] = (at[0].0 + 7.0, at[0].1 + 5.0);
            let p = route_partial(&g, &moved, &base, &at);
            out.push(h(&to_svg(&p)));
            out.push(h(&dbg(&p)));
            let k = at.len() / 2;
            let mut moved2 = at.clone();
            moved2[k] = (at[k].0 - 40.0, at[k].1 + 23.0);
            let p2 = route_partial(&g, &moved2, &base, &at);
            out.push(h(&to_svg(&p2)));
            out.push(h(&dbg(&p2)));
            let r2 = route(&g, &moved2);
            out.push(h(&to_svg(&r2)));
            out.push(h(&dbg(&r2)));
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
