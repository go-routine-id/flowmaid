// Verifier probe: writes every output (render, scene, route variants,
// drags) of a list of inputs. With a base dir given, an output identical
// to the base build's is not kept (the base copy is removed too).
// Usage: probe <list> <outdir> <full|lite> [basedir]
use flowmaid::model::{Document, Graph};
use flowmaid::parser::parse_document;
use flowmaid::scene::{self, Scene};
use flowmaid::{class, er, mindmap};
use std::fs;
use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

struct Out {
    dir: PathBuf,
    base: Option<PathBuf>,
    tsv: fs::File,
}

impl Out {
    fn emit(&mut self, id: &str, mode: &str, svg: &str, info: &str) {
        let name = format!("{id}.{mode}.svg");
        let status = match &self.base {
            Some(b) => {
                let bp = b.join(&name);
                match fs::read_to_string(&bp) {
                    Ok(old) if old == svg => {
                        let _ = fs::remove_file(&bp);
                        "same"
                    }
                    Ok(_) => {
                        fs::write(self.dir.join(&name), svg).unwrap();
                        "diff"
                    }
                    Err(_) => {
                        fs::write(self.dir.join(&name), svg).unwrap();
                        "nobase"
                    }
                }
            }
            None => {
                fs::write(self.dir.join(&name), svg).unwrap();
                "base"
            }
        };
        writeln!(self.tsv, "{id}\t{mode}\t{:016x}\t{status}\t{info}", fnv(svg)).unwrap();
    }
}

fn guard<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|e| {
        if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            "?".into()
        }
    })
}

#[cfg(feature = "head")]
fn origin_of(sc: &Scene) -> String {
    format!("o={:.4},{:.4};", sc.origin.0, sc.origin.1)
}
#[cfg(not(feature = "head"))]
fn origin_of(_sc: &Scene) -> String {
    String::new()
}

/// Geometry hash, hits, positions check, origin, size, centres.
fn info(sc: &Scene, given: Option<&[(f64, f64)]>, centres: bool) -> String {
    let geom = fnv(&format!("{:?}{:?}{:?}", sc.nodes, sc.edges, sc.clusters));
    let mut hits = String::new();
    let mut hit_ok = 0;
    for (i, n) in sc.nodes.iter().enumerate() {
        let a = sc.node_at(n.x, n.y).map(|k| k as i64).unwrap_or(-1);
        let b = match sc.hit_test(n.x, n.y, 2.0) {
            Some(scene::Hit::Node(k)) => format!("n{k}"),
            Some(scene::Hit::Edge(k)) => format!("e{k}"),
            Some(scene::Hit::Cluster(k)) => format!("c{k}"),
            None => "-".into(),
        };
        if a == i as i64 {
            hit_ok += 1;
        }
        hits.push_str(&format!("{a}/{b},"));
    }
    // A few picks off the nodes: edge midpoints.
    for e in &sc.edges {
        let b = e.bezier;
        let mid = (
            (b[0].0 + 3.0 * b[1].0 + 3.0 * b[2].0 + b[3].0) / 8.0,
            (b[0].1 + 3.0 * b[1].1 + 3.0 * b[2].1 + b[3].1) / 8.0,
        );
        let mut pts = vec![mid];
        if let Some((_, p, _)) = &e.label {
            pts.push(*p);
        }
        if let Some(w) = e.waypoints.get(e.waypoints.len() / 2) {
            pts.push(*w);
        }
        for p in pts {
            let h = match sc.hit_test(p.0, p.1, 2.0) {
                Some(scene::Hit::Node(k)) => format!("n{k}"),
                Some(scene::Hit::Edge(k)) => format!("e{k}"),
                Some(scene::Hit::Cluster(k)) => format!("c{k}"),
                None => "-".into(),
            };
            hits.push_str(&h);
            hits.push(',');
        }
    }
    let pos = match given {
        Some(g) => {
            let ok = sc.nodes.len() == g.len()
                && sc.nodes.iter().zip(g).all(|(n, p)| n.x == p.0 && n.y == p.1);
            format!("pos={};", ok as u8)
        }
        None => String::new(),
    };
    let mut s = format!(
        "geom={geom:016x};hits={:016x};hitok={hit_ok}/{};{pos}{}w={:.4};h={:.4};",
        fnv(&hits),
        sc.nodes.len(),
        origin_of(sc),
        sc.width,
        sc.height
    );
    if centres {
        s.push_str("c=");
        for n in &sc.nodes {
            s.push_str(&format!("{:.3}:{:.3}:{:.3}:{:.3}|", n.x, n.y, n.w, n.h));
        }
        s.push(';');
    }
    s
}

type Drag = (String, Vec<(f64, f64)>);

fn drags(c: &[(f64, f64)], full: bool, movable: usize) -> Vec<Drag> {
    let n = movable;
    if n == 0 {
        return Vec::new();
    }
    let m = n / 2;
    let lft = (0..n)
        .min_by(|&a, &b| c[a].0.partial_cmp(&c[b].0).unwrap())
        .unwrap();
    let top = (0..n)
        .min_by(|&a, &b| c[a].1.partial_cmp(&c[b].1).unwrap())
        .unwrap();
    let one = |i: usize, p: (f64, f64)| {
        let mut d = c.to_vec();
        d[i] = p;
        d
    };
    let all = |dx: f64, dy: f64| c.iter().map(|p| (p.0 + dx, p.1 + dy)).collect::<Vec<_>>();
    let mut v = vec![
        ("d1".to_string(), one(m, (-60.0, -40.0))),
        ("d4".to_string(), one(lft, (c[lft].0 - 150.0, c[lft].1))),
        ("d6".to_string(), one(m, (c[m].0 + 25.0, c[m].1 + 15.0))),
        ("d9".to_string(), all(-40.0, -25.0)),
    ];
    if full {
        v.extend([
            ("d2".to_string(), one(m, (-60.0, c[m].1))),
            ("d3".to_string(), one(m, (c[m].0, -45.0))),
            ("d5".to_string(), one(top, (c[top].0, c[top].1 - 120.0))),
            ("d7".to_string(), one(lft, (c[lft].0 + 30.0, c[lft].1))),
            ("d8".to_string(), all(40.0, 25.0)),
            ("d10".to_string(), one(top, (-80.0, c[top].1 + 60.0))),
            ("d11".to_string(), one(m, (c[m].0 - 20.0, c[m].1))),
            ("d12".to_string(), one(lft, (-5.0, c[lft].1))),
        ]);
    }
    v
}

fn flow(out: &mut Out, id: &str, g: &Graph, title: &str, full: bool) {
    let svg = |sc: &Scene| scene::to_svg_titled(sc, title);
    let laid = match guard(|| scene::scene(g)) {
        Ok(s) => s,
        Err(e) => {
            out.emit(id, "scene", &format!("PANIC {e}"), "");
            return;
        }
    };
    out.emit(id, "scene", &svg(&laid), &info(&laid, None, full));
    if full {
        let labels: Vec<String> = laid.nodes.iter().map(|n| n.label.replace('\n', "\\n")).collect();
        fs::write(out.dir.join(format!("{id}.labels.txt")), labels.join("\n")).unwrap();
    }
    let c: Vec<(f64, f64)> = laid.nodes.iter().map(|n| (n.x, n.y)).collect();
    let sizes: Vec<(f64, f64)> = laid.nodes.iter().map(|n| (n.w, n.h)).collect();
    let run = |out: &mut Out, mode: &str, d: &[(f64, f64)], f: &dyn Fn() -> Scene| match guard(f) {
        Ok(sc) => out.emit(id, mode, &svg(&sc), &info(&sc, Some(d), full)),
        Err(e) => out.emit(id, mode, &format!("PANIC {e}"), ""),
    };
    run(out, "r0", &c, &|| scene::route(g, &c));
    if full {
        run(out, "rs", &c, &|| scene::route_sized(g, &c, &sizes));
        run(out, "rp0", &c, &|| scene::route_partial(g, &c, &laid, &c));
    }
    for (name, d) in drags(&c, full, c.len()) {
        run(out, &format!("{name}.r"), &d, &|| scene::route(g, &d));
        run(out, &format!("{name}.p"), &d, &|| scene::route_partial(g, &d, &laid, &c));
        if full && name == "d1" {
            run(out, "d1.s", &d, &|| scene::route_sized(g, &d, &sizes));
        }
    }
    if full && !c.is_empty() {
        // A chain of route_partial frames: out past the corner, further,
        // then back home.
        let m = c.len() / 2;
        let mut d1 = c.clone();
        d1[m] = (-60.0, -40.0);
        let mut e = c.clone();
        e[m] = (-120.0, -90.0);
        if let Ok(p1) = guard(|| scene::route_partial(g, &d1, &laid, &c)) {
            if let Ok(p2) = guard(|| scene::route_partial(g, &e, &p1, &d1)) {
                out.emit(id, "ch2", &svg(&p2), &info(&p2, Some(&e), full));
                if let Ok(p3) = guard(|| scene::route_partial(g, &c, &p2, &e)) {
                    out.emit(id, "ch3", &svg(&p3), &info(&p3, Some(&c), full));
                }
            }
        }
    }
}

fn class_doc(out: &mut Out, id: &str, d: &flowmaid::model::ClassDiagram, full: bool) {
    let laid = match guard(|| class::scene(d)) {
        Ok(s) => s,
        Err(e) => {
            out.emit(id, "scene", &format!("PANIC {e}"), "");
            return;
        }
    };
    let cinfo = |cs: &class::ClassScene, given: Option<&[(f64, f64)]>| {
        format!(
            "{}cards={:016x};",
            info(&cs.scene, given, full),
            fnv(&format!("{:?}", class::card_spots(cs)))
        )
    };
    out.emit(id, "scene", &class::to_svg(&laid), &cinfo(&laid, None));
    let c: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let n = d.classes.len();
    let run = |out: &mut Out, mode: &str, p: &[(f64, f64)]| match guard(|| class::route(d, p)) {
        Ok(cs) => out.emit(id, mode, &class::to_svg(&cs), &cinfo(&cs, Some(p))),
        Err(e) => out.emit(id, mode, &format!("PANIC {e}"), ""),
    };
    run(out, "r0", &c);
    if full && c.len() != n {
        // Classes only: notes placed by the fallback (positions differ).
        match guard(|| class::route(d, &c[..n])) {
            Ok(cs) => out.emit(id, "rc", &class::to_svg(&cs), &cinfo(&cs, None)),
            Err(e) => out.emit(id, "rc", &format!("PANIC {e}"), ""),
        }
    }
    for (name, p) in drags(&c, full, n) {
        run(out, &format!("{name}.r"), &p);
    }
}

fn er_doc(out: &mut Out, id: &str, d: &flowmaid::model::ErDiagram, full: bool) {
    let laid = match guard(|| er::scene(d)) {
        Ok(s) => s,
        Err(e) => {
            out.emit(id, "scene", &format!("PANIC {e}"), "");
            return;
        }
    };
    out.emit(id, "scene", &er::to_svg(&laid), &info(&laid.scene, None, full));
    let c: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
    let run = |out: &mut Out, mode: &str, p: &[(f64, f64)]| match guard(|| er::route(d, p)) {
        Ok(es) => out.emit(id, mode, &er::to_svg(&es), &info(&es.scene, Some(p), full)),
        Err(e) => out.emit(id, mode, &format!("PANIC {e}"), ""),
    };
    run(out, "r0", &c);
    for (name, p) in drags(&c, full, c.len()) {
        run(out, &format!("{name}.r"), &p);
    }
}

fn mind_doc(out: &mut Out, id: &str, d: &flowmaid::model::Mindmap) {
    let laid = mindmap::scene(d);
    out.emit(id, "scene", &mindmap::to_svg(&laid), "");
    let c: Vec<(f64, f64)> = laid.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
    out.emit(id, "r0", &mindmap::to_svg(&mindmap::route(d, &c)), "");
    if !c.is_empty() {
        let mut p = c.clone();
        p[c.len() / 2] = (-60.0, -40.0);
        out.emit(id, "d1.r", &mindmap::to_svg(&mindmap::route(d, &p)), "");
    }
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let args: Vec<String> = std::env::args().collect();
    let list = fs::read_to_string(&args[1]).unwrap();
    let dir = PathBuf::from(&args[2]);
    fs::create_dir_all(&dir).unwrap();
    let full = args[3] == "full";
    let base = args.get(4).map(PathBuf::from);
    let tsv = fs::File::create(dir.join("results.tsv")).unwrap();
    let mut out = Out { dir, base, tsv };
    for (i, path) in list.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let id = format!("{i:05}");
        let src = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        writeln!(out.tsv, "{id}\tfile\t-\t-\t{path}").unwrap();
        match guard(|| flowmaid::render_svg(&src)) {
            Ok(Ok(svg)) => out.emit(&id, "render", &svg, ""),
            Ok(Err(e)) => out.emit(&id, "render", &format!("ERR line {}: {}", e.line, e.message), ""),
            Err(e) => out.emit(&id, "render", &format!("PANIC {e}"), ""),
        }
        let doc = match guard(|| parse_document(&src)) {
            Ok(Ok(d)) => d,
            _ => continue,
        };
        match &doc {
            Document::Flowchart(g) => flow(&mut out, &id, g, "Flowchart diagram", full),
            Document::State(g) => flow(&mut out, &id, g, "State diagram", full),
            Document::Class(d) => class_doc(&mut out, &id, d, full),
            Document::Er(d) => er_doc(&mut out, &id, d, full),
            Document::Mindmap(d) => mind_doc(&mut out, &id, d),
            _ => {}
        }
        out.tsv.flush().unwrap();
    }
}
