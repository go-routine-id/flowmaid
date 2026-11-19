//! Verifier probe: per input, FNV-1a hashes of everything the public API draws.
use flowmaid::model::Document;
use flowmaid::scene::{route, route_partial, scene, to_svg, Scene};

fn h(s: &str) -> String {
    let mut x: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        x ^= *b as u64;
        x = x.wrapping_mul(0x100000001b3);
    }
    format!("{x:x}")
}

fn dbg_scene(s: &Scene) -> String {
    // 0.39.0 added `plain_crossings`; strip only the `false` literal so a
    // `true` would still show.
    format!("{s:?}").replace(", plain_crossings: false", "").replace("plain_crossings: false, ", "")
}

fn hits(s: &Scene) -> String {
    let mut out = String::new();
    let (w, h_) = (s.width, s.height);
    let n = 23;
    for i in 0..=n {
        for j in 0..=n {
            let x = w * i as f64 / n as f64;
            let y = h_ * j as f64 / n as f64;
            out.push_str(&format!("{:?}{:?}{:?}{:?};", s.hit_test(x, y, 3.0), s.edge_at(x, y, 6.0), s.cluster_at(x, y), s.nearest_node(x, y)));
        }
    }
    out.push_str(&format!("{:?}", s.hits_in_rect(0.0, 0.0, w / 2.0, h_ / 2.0)));
    out.push_str(&format!("{:?}", s.hits_in_rect(w / 3.0, h_ / 3.0, w, h_)));
    out
}

fn geo(tag: &str, s: &Scene, out: &mut Vec<String>) {
    out.push(format!("{tag}svg={}", h(&to_svg(s))));
    out.push(format!("{tag}dbg={}", h(&dbg_scene(s))));
    out.push(format!("{tag}hit={}", h(&hits(s))));
}

fn moves(at: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let n = at.len();
    let mut v = Vec::new();
    let mut m = at.to_vec();
    m[0] = (at[0].0 + 7.0, at[0].1 + 5.0);
    v.push(m);
    let mut m = at.to_vec();
    m[n / 2] = (at[n / 2].0 - 40.0, at[n / 2].1 + 25.0);
    v.push(m);
    let mut m = at.to_vec();
    m[n - 1] = (at[n - 1].0 + 60.0, at[n - 1].1 - 35.0);
    if n > 1 {
        m[1] = (at[1].0 + 15.0, at[1].1 + 80.0);
    }
    v.push(m);
    v
}

fn row(src: &str, full: bool) -> String {
    let mut out = Vec::new();
    let svg = match flowmaid::render_svg(src) {
        Ok(s) => h(&s),
        Err(e) => match flowmaid::render_advance_text_svg(src) {
            Ok(s) => format!("adv:{}", h(&s)),
            Err(e2) => format!("ERR:{}:{}", h(&e.to_string()), h(&e2.to_string())),
        },
    };
    out.push(format!("svg={svg}"));
    if !full {
        return out.join("\t");
    }
    let mut o = flowmaid::SvgOptions::default();
    o.theme = Some(flowmaid::style::Theme::Dark);
    o.transparent = true;
    o.responsive = true;
    if let Ok(s) = flowmaid::render_svg_advanced(src, &o) {
        out.push(format!("dark={}", h(&s)));
    }
    match flowmaid::parser::parse_document(src) {
        Ok(Document::Flowchart(g) | Document::State(g)) => {
            out.push(format!("mmd={}", h(&flowmaid::to_mermaid(&g))));
            if g.nodes.is_empty() {
                return out.join("\t");
            }
            let base = scene(&g);
            geo("s.", &base, &mut out);
            let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &at);
            geo("r.", &r, &mut out);
            for (k, m) in moves(&at).iter().enumerate() {
                let p = route_partial(&g, m, &base, &at);
                geo(&format!("p{k}s."), &p, &mut out);
                let p2 = route_partial(&g, m, &r, &at);
                geo(&format!("p{k}r."), &p2, &mut out);
                let full_r = route(&g, m);
                out.push(format!("R{k}={}", h(&dbg_scene(&full_r))));
                // a second drag step from the partial frame
                let mut m2 = m.clone();
                m2[0] = (m2[0].0 - 11.0, m2[0].1 + 3.0);
                let p3 = route_partial(&g, &m2, &p2, m);
                out.push(format!("pp{k}={}:{}", h(&to_svg(&p3)), h(&dbg_scene(&p3))));
            }
        }
        Ok(Document::Class(d)) => {
            let cs = flowmaid::class::scene(&d);
            out.push(format!("c.svg={}", h(&flowmaid::class::to_svg(&cs))));
            out.push(format!("c.dbg={}", h(&format!("{cs:?}").replace(", plain_crossings: false", "").replace("plain_crossings: false, ", ""))));
            let at: Vec<(f64, f64)> = cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            if !at.is_empty() {
                for (k, m) in moves(&at).iter().enumerate() {
                    let rs = flowmaid::class::route(&d, m);
                    out.push(format!("c.r{k}={}", h(&flowmaid::class::to_svg(&rs))));
                }
            }
        }
        Ok(Document::Er(d)) => {
            let es = flowmaid::er::scene(&d);
            out.push(format!("e.svg={}", h(&flowmaid::er::to_svg(&es))));
            out.push(format!("e.dbg={}", h(&format!("{es:?}").replace(", plain_crossings: false", "").replace("plain_crossings: false, ", ""))));
            let at: Vec<(f64, f64)> = es.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            if !at.is_empty() {
                for (k, m) in moves(&at).iter().enumerate() {
                    let rs = flowmaid::er::route(&d, m);
                    out.push(format!("e.r{k}={}", h(&flowmaid::er::to_svg(&rs))));
                }
            }
        }
        _ => {}
    }
    out.join("\t")
}

/// Rows in the format of tests/fixtures/identity.tsv (without the class column).
fn id_row(src: &str) -> String {
    let svg = flowmaid::render_svg(src)
        .ok()
        .or_else(|| flowmaid::render_advance_text_svg(src).ok())
        .map(|s| h(&s))
        .unwrap_or_else(|| "NORENDER".into());
    let (mut routed, mut partial) = ("-".to_string(), "-".to_string());
    if let Ok(Document::Flowchart(g) | Document::State(g)) = flowmaid::parser::parse_document(src) {
        if !g.nodes.is_empty() {
            let base = scene(&g);
            let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
            routed = h(&to_svg(&route(&g, &at)));
            let mut moved = at.clone();
            moved[0] = (at[0].0 + 7.0, at[0].1 + 5.0);
            partial = h(&to_svg(&route_partial(&g, &moved, &base, &at)));
        }
    }
    format!("{svg}\t{routed}\t{partial}")
}


fn mid(src: &str) -> String {
    let mut out = vec![row(src, false)];
    if let Ok(Document::Flowchart(g) | Document::State(g)) = flowmaid::parser::parse_document(src) {
        out.push(format!("mmd={}", h(&flowmaid::to_mermaid(&g))));
        if !g.nodes.is_empty() {
            let base = scene(&g);
            out.push(format!("s.svg={}\ts.dbg={}", h(&to_svg(&base)), h(&dbg_scene(&base))));
            let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
            let r = route(&g, &at);
            out.push(format!("r.svg={}\tr.dbg={}", h(&to_svg(&r)), h(&dbg_scene(&r))));
            let m = &moves(&at)[0];
            let p = route_partial(&g, m, &base, &at);
            out.push(format!("p0s={}:{}", h(&to_svg(&p)), h(&dbg_scene(&p))));
            let p2 = route_partial(&g, m, &r, &at);
            out.push(format!("p0r={}:{}", h(&to_svg(&p2)), h(&dbg_scene(&p2))));
            let n = base.nodes.len();
            let mut hs = String::new();
            for k in 0..8 { let nd = &base.nodes[(k * 7919) % n]; hs.push_str(&format!("{:?}{:?}", base.hit_test(nd.x, nd.y + nd.h / 2.0 + 2.0, 3.0), base.hit_test(nd.x + 1.0, nd.y, 3.0))); }
            out.push(format!("s.hit={}", h(&hs)));
        }
    }
    out.join("\t")
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_default();
    let list = args.next().expect("list file");
    let files = std::fs::read_to_string(&list).expect("read list");
    for f in files.lines().filter(|l| !l.is_empty()) {
        let Ok(src) = std::fs::read_to_string(f) else {
            println!("{f}\tUNREADABLE");
            continue;
        };
        let r = std::panic::catch_unwind(|| match mode.as_str() {
            "id" => id_row(&src),
            "full" => row(&src, true),
            "fullon" => flowmaid::quality::with_blocks(true, || row(&src, true)),
            "midon" => flowmaid::quality::with_blocks(true, || mid(&src)),
            "mid" => mid(&src),
            _ => row(&src, false),
        });
        match r {
            Ok(r) => println!("{f}\t{r}"),
            Err(_) => println!("{f}\tPANIC"),
        }
    }
}
