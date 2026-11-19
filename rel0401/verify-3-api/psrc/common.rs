// Shared probe code (compiles against origin/dev and HEAD).
use flowmaid::model::Document;
use flowmaid::{class, er, mindmap, parser, scene};
use std::fs;
use std::path::Path;

pub fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn view_box(svg: &str) -> String {
    match svg.find(" viewBox=\"") {
        Some(i) => {
            let r = &svg[i + 10..];
            r[..r.find('"').unwrap_or(0)].to_string()
        }
        None => "-".into(),
    }
}

pub fn name_of(p: &str) -> String {
    let mut parts: Vec<&str> = p.rsplit('/').take(2).collect();
    parts.reverse();
    parts.join("__").trim_end_matches(".mmd").to_string()
}

/// Drags over centres `c` with half sizes `hw`: (label, positions).
pub fn drags(c: &[(f64, f64)], half: &[(f64, f64)]) -> Vec<(String, Vec<(f64, f64)>)> {
    let n = c.len();
    if n == 0 {
        return vec![];
    }
    let left = (0..n).min_by(|&a, &b| (c[a].0 - half[a].0).total_cmp(&(c[b].0 - half[b].0))).unwrap();
    let top = (0..n).min_by(|&a, &b| (c[a].1 - half[a].1).total_cmp(&(c[b].1 - half[b].1))).unwrap();
    let mut out: Vec<(String, Vec<(f64, f64)>)> = Vec::new();
    let mut d = c.to_vec();
    d[left].0 += 30.0;
    out.push(("in".into(), d));
    out.push(("allin".into(), c.iter().map(|p| (p.0 + 40.0, p.1 + 25.0)).collect()));
    let mut d = c.to_vec();
    d[n / 2].0 += 15.0;
    d[n / 2].1 += 10.0;
    out.push(("mid".into(), d));
    let mut d = c.to_vec();
    d[top].1 = -40.0;
    d[left].0 = -60.0;
    out.push(("over".into(), d));
    let mut d = c.to_vec();
    d[left].0 -= 25.0;
    out.push(("left".into(), d));
    let mut d = c.to_vec();
    d[top].1 -= 12.0;
    out.push(("up".into(), d));
    let mut d = c.to_vec();
    d[left].0 -= 0.03;
    out.push(("tiny".into(), d));
    let mut d = c.to_vec();
    d[0] = (-400.0, -300.0);
    out.push(("far".into(), d));
    // y-only overflow with x starting further in
    let mut d: Vec<(f64, f64)> = c.iter().map(|p| (p.0 + 40.0, p.1)).collect();
    d[top].1 = -45.0;
    out.push(("yonly".into(), d));
    out
}

pub struct Out<'a> {
    pub dir: Option<&'a Path>,
    pub rows: Vec<String>,
}
impl<'a> Out<'a> {
    pub fn put(&mut self, n: &str, v: &str, svg: &str) {
        self.rows.push(format!("{n}\t{v}\t{:016x}\t{}", fnv(svg), view_box(svg)));
        if let Some(d) = self.dir {
            fs::write(d.join(format!("{n}.{v}.svg")), svg).unwrap();
        }
    }
}

pub fn dump_one(o: &mut Out, f: &str) {
    let src = match fs::read_to_string(f) {
        Ok(s) => s,
        Err(_) => return,
    };
    let n = name_of(f);
    match flowmaid::render_svg(&src) {
        Ok(s) => o.put(&n, "render", &s),
        Err(e) => {
            o.rows.push(format!("{n}\trender\tERR\t{}:{}", e.line, e.message.replace('\t', " ").replace('\n', " ")));
            return;
        }
    }
    let doc = match parser::parse_document(&src) {
        Ok(d) => d,
        Err(_) => return,
    };
    match doc {
        Document::Flowchart(g) | Document::State(g) => {
            if g.nodes.is_empty() {
                return;
            }
            let laid = scene::scene(&g);
            o.put(&n, "scene", &scene::to_svg(&laid));
            let c: Vec<(f64, f64)> = laid.nodes.iter().map(|k| (k.x, k.y)).collect();
            let half: Vec<(f64, f64)> = laid.nodes.iter().map(|k| (k.w / 2.0, k.h / 2.0)).collect();
            let sizes: Vec<(f64, f64)> = laid.nodes.iter().map(|k| (k.w, k.h)).collect();
            o.put(&n, "route0", &scene::to_svg(&scene::route(&g, &c)));
            o.put(&n, "sized0", &scene::to_svg(&scene::route_sized(&g, &c, &sizes)));
            let mut m = c.clone();
            m[0] = (c[0].0 + 7.0, c[0].1 + 5.0);
            o.put(&n, "part0", &scene::to_svg(&scene::route_partial(&g, &m, &laid, &c)));
            for (lbl, d) in drags(&c, &half) {
                o.put(&n, &format!("route-{lbl}"), &scene::to_svg(&scene::route(&g, &d)));
                o.put(&n, &format!("sized-{lbl}"), &scene::to_svg(&scene::route_sized(&g, &d, &sizes)));
                let p = scene::route_partial(&g, &d, &laid, &c);
                o.put(&n, &format!("part-{lbl}"), &scene::to_svg(&p));
                if lbl == "over" || lbl == "far" {
                    // chain back inside from an overflowing frame
                    let p2 = scene::route_partial(&g, &c, &p, &d);
                    o.put(&n, &format!("chain-{lbl}"), &scene::to_svg(&p2));
                }
            }
        }
        Document::Class(d) => {
            let laid = class::scene(&d);
            o.put(&n, "scene", &class::to_svg(&laid));
            let c: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|k| (k.x, k.y)).collect();
            if c.is_empty() {
                return;
            }
            let half: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|k| (k.w / 2.0, k.h / 2.0)).collect();
            o.put(&n, "route0", &class::to_svg(&class::route(&d, &c)));
            let nc = d.classes.len();
            if nc > 0 && nc < c.len() {
                o.put(&n, "routecls", &class::to_svg(&class::route(&d, &c[..nc])));
            }
            for (lbl, dd) in drags(&c, &half) {
                o.put(&n, &format!("route-{lbl}"), &class::to_svg(&class::route(&d, &dd)));
            }
        }
        Document::Er(d) => {
            let laid = er::scene(&d);
            o.put(&n, "scene", &er::to_svg(&laid));
            let c: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|k| (k.x, k.y)).collect();
            if c.is_empty() {
                return;
            }
            let half: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|k| (k.w / 2.0, k.h / 2.0)).collect();
            o.put(&n, "route0", &er::to_svg(&er::route(&d, &c)));
            for (lbl, dd) in drags(&c, &half) {
                o.put(&n, &format!("route-{lbl}"), &er::to_svg(&er::route(&d, &dd)));
            }
        }
        Document::Mindmap(d) => {
            let laid = mindmap::scene(&d);
            o.put(&n, "scene", &mindmap::to_svg(&laid));
            let c: Vec<(f64, f64)> = laid.nodes.iter().map(|k| (k.cx(), k.cy())).collect();
            if c.is_empty() {
                return;
            }
            let half: Vec<(f64, f64)> = laid.nodes.iter().map(|k| (k.w / 2.0, k.h / 2.0)).collect();
            o.put(&n, "route0", &mindmap::to_svg(&mindmap::route(&d, &c)));
            for (lbl, dd) in drags(&c, &half) {
                o.put(&n, &format!("route-{lbl}"), &mindmap::to_svg(&mindmap::route(&d, &dd)));
            }
        }
        _ => {}
    }
}

pub fn parse_cases(file: &str) {
    for line in fs::read_to_string(file).unwrap().lines() {
        if line.is_empty() {
            continue;
        }
        let src = line.replace("\\n", "\n").replace("\\t", "\t");
        let r = match parser::parse_document(&src) {
            Ok(Document::Class(d)) => {
                let names: Vec<&str> = d.classes.iter().map(|c| c.name.as_str()).collect();
                format!("OK {:?}", names)
            }
            Ok(_) => "OK other".into(),
            Err(e) => format!("ERR {}: {}", e.line, e.message),
        };
        println!("{line}\t{r}");
    }
}

pub fn run_common(args: &[String]) -> bool {
    match args[1].as_str() {
        "dump" => {
            let dir = if args[2] == "-" { None } else { Some(Path::new(&args[2])) };
            if let Some(d) = dir {
                fs::create_dir_all(d).unwrap();
            }
            let list = fs::read_to_string(&args[3]).unwrap();
            let mut o = Out { dir, rows: vec![] };
            for f in list.lines().filter(|l| !l.is_empty()) {
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut oo = Out { dir, rows: vec![] };
                    dump_one(&mut oo, f);
                    oo.rows
                }));
                match r {
                    Ok(rows) => o.rows.extend(rows),
                    Err(_) => o.rows.push(format!("{}\tPANIC\t-\t-", name_of(f))),
                }
            }
            for r in &o.rows {
                println!("{r}");
            }
            true
        }
        "edges" => {
            let src = fs::read_to_string(&args[2]).unwrap();
            if let Ok(Document::Flowchart(g) | Document::State(g)) = parser::parse_document(&src) {
                let laid = scene::scene(&g);
                let c: Vec<(f64, f64)> = laid.nodes.iter().map(|k| (k.x, k.y)).collect();
                let half: Vec<(f64, f64)> = laid.nodes.iter().map(|k| (k.w / 2.0, k.h / 2.0)).collect();
                for (lbl, d) in drags(&c, &half) {
                    let sc = scene::route(&g, &d);
                    println!("route-{lbl}\t{:?}\t{:?}", sc.edges, sc.nodes.iter().map(|n| (n.x, n.y, n.w, n.h)).collect::<Vec<_>>());
                }
            }
            true
        }
        "parse" => {
            parse_cases(&args[2]);
            true
        }
        _ => false,
    }
}
