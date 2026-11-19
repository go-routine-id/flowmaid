// Verification probe: same source built against origin/dev (base) and HEAD
// (feature "head"). Usage:
//   vprobe all <listfile> [lite]   -> TSV: file kind hash info
//   vprobe one <file> <kind>       -> the SVG of that kind on stdout
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::{class, er, mindmap, scene};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn view_box(svg: &str) -> (f64, f64, f64, f64) {
    let i = match svg.find(" viewBox=\"") {
        Some(i) => i + 10,
        None => return (f64::NAN, f64::NAN, f64::NAN, f64::NAN),
    };
    let j = svg[i..].find('"').unwrap() + i;
    let v: Vec<f64> = svg[i..j].split(' ').map(|t| t.parse().unwrap_or(f64::NAN)).collect();
    (v[0], v[1], v[2], v[3])
}

struct Item {
    kind: String,
    svg: Option<String>,
    info: String,
}

#[allow(unused_variables)]
fn scene_info(sc: &scene::Scene, given: Option<&[(f64, f64)]>, svg: &str, is_static: bool) -> String {
    let mut info = Vec::new();
    let vb = view_box(svg);
    info.push(format!("vb={},{}", vb.0, vb.1));
    if let Some(given) = given {
        let ok = sc.nodes.len() == given.len()
            && sc.nodes.iter().zip(given).all(|(n, p)| n.x == p.0 && n.y == p.1);
        info.push(format!("nodes_ok={ok}"));
    }
    // pick at each node centre (scene coords)
    let mut bad = 0;
    for (i, n) in sc.nodes.iter().enumerate() {
        match sc.node_at(n.x, n.y) {
            Some(j) if j >= i => {}
            _ => bad += 1,
        }
    }
    info.push(format!("pickbad={bad}"));
    #[cfg(feature = "head")]
    {
        info.push(format!("origin={},{}", sc.origin.0, sc.origin.1));
        let ok = |o: f64, v: f64| if o < 0.0 { (v - o).abs() <= 0.051 } else { v >= -0.051 };
        let vok = ok(sc.origin.0, vb.0) && ok(sc.origin.1, vb.1);
        info.push(format!("vbok={vok}"));
        if is_static {
            info.push(format!("static0={}", sc.origin == (0.0, 0.0) && vb.0 == 0.0 && vb.1 == 0.0));
        }
    }
    info.join(";")
}

fn run<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|e| {
        e.downcast_ref::<String>()
            .cloned()
            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default()
            .replace(['\t', '\n'], " ")
    })
}

fn push_scene(
    out: &mut Vec<Item>,
    kind: &str,
    given: Option<&[(f64, f64)]>,
    is_static: bool,
    f: impl FnOnce() -> scene::Scene,
) -> Option<scene::Scene> {
    match run(|| {
        let sc = f();
        let svg = scene::to_svg(&sc);
        (sc, svg)
    }) {
        Ok((sc, svg)) => {
            let info = scene_info(&sc, given, &svg, is_static);
            out.push(Item { kind: kind.into(), svg: Some(svg), info });
            Some(sc)
        }
        Err(m) => {
            out.push(Item { kind: kind.into(), svg: None, info: format!("PANIC {m}") });
            None
        }
    }
}

fn graph_items(g: &flowmaid::model::Graph, lite: bool, out: &mut Vec<Item>) {
    let n = g.nodes.len();
    let base = match push_scene(out, "scene", None, true, || scene::scene(g)) {
        Some(b) => b,
        None => return,
    };
    if n == 0 {
        #[cfg(feature = "head")]
        {
            let r = run(|| scene::try_route(g, &[]).is_some() && scene::try_route(g, &[(1.0, 1.0)]).is_none());
            out.push(Item { kind: "try".into(), svg: None, info: format!("{r:?}") });
        }
        return;
    }
    let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
    let sizes: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.w, n.h)).collect();
    let k1 = (0..n).find(|&i| !base.nodes[i].label.is_empty()).unwrap_or(0);
    let last = n - 1;
    let mut drags: Vec<(&str, Vec<(f64, f64)>)> = Vec::new();
    let mv = |i: usize, p: (f64, f64)| {
        let mut m = at.clone();
        m[i] = p;
        m
    };
    drags.push(("id", mv(0, (at[0].0 + 7.0, at[0].1 + 5.0))));
    drags.push(("past1", mv(k1, (-60.0, -40.0))));
    if !lite {
        drags.push(("mid", mv(n / 2, (at[n / 2].0 + 31.0, at[n / 2].1 + 19.0))));
        drags.push(("lt", mv(0, (at[0].0 - 5.0, at[0].1 - 3.0))));
        drags.push(("past2", mv(k1, (-60.0, at[k1].1))));
        drags.push(("past3", mv(last, (at[last].0, -30.0))));
        drags.push(("past4", mv(n / 2, (-200.0, -150.0))));
    }
    push_scene(out, "r0", Some(&at), false, || scene::route(g, &at));
    if !lite {
        push_scene(out, "rs0", Some(&at), false, || scene::route_sized(g, &at, &sizes));
    }
    for (name, m) in &drags {
        push_scene(out, &format!("r_{name}"), Some(m), false, || scene::route(g, m));
        push_scene(out, &format!("p_{name}"), Some(m), false, || scene::route_partial(g, m, &base, &at));
        if !lite && name.starts_with("past1") {
            push_scene(out, &format!("rs_{name}"), Some(m), false, || scene::route_sized(g, m, &sizes));
        }
    }
    #[cfg(feature = "head")]
    {
        let r = run(|| {
            let mut bad = Vec::new();
            if !lite {
                let a = scene::try_route(g, &at).map(|s| scene::to_svg(&s));
                if a.as_deref() != Some(scene::to_svg(&scene::route(g, &at)).as_str()) {
                    bad.push("eq");
                }
            }
            for c in [0, n - 1, n + 1, n + 3] {
                if c == n {
                    continue;
                }
                let p: Vec<(f64, f64)> = (0..c).map(|i| at.get(i).copied().unwrap_or((5.0, 5.0))).collect();
                if scene::try_route(g, &p).is_some() {
                    bad.push("count");
                }
            }
            let mut p = at.clone();
            p[0].0 = f64::NAN;
            if scene::try_route(g, &p).is_some() {
                bad.push("nan");
            }
            bad.join(",")
        });
        out.push(Item { kind: "try".into(), svg: None, info: format!("{r:?}") });
    }
}

fn class_svg(cs: &class::ClassScene) -> String {
    class::to_svg(cs)
}

fn push_class(out: &mut Vec<Item>, kind: &str, given: Option<&[(f64, f64)]>, is_static: bool, f: impl FnOnce() -> class::ClassScene) {
    match run(|| {
        let cs = f();
        let svg = class_svg(&cs);
        (cs, svg)
    }) {
        Ok((cs, svg)) => {
            let mut info = scene_info(&cs.scene, given, &svg, is_static);
            #[cfg(feature = "head")]
            {
                // class/ER: the viewBox is the scene's canvas.
                let vb = view_box(&svg);
                let sc = &cs.scene;
                let exact = (vb.0 - sc.origin.0).abs() <= 0.051 && (vb.1 - sc.origin.1).abs() <= 0.051 && (vb.2 - sc.width).abs() <= 0.51 && (vb.3 - sc.height).abs() <= 0.51;
                info.push_str(&format!(";canvas={exact}"));
                // cards drawn where card_spots says (unless clamped)
                let mut cmiss = 0;
                for (rel, spots) in cs.rels.iter().zip(class::card_spots(&cs)) {
                    for (text, at) in [(&rel.from_card, spots[0]), (&rel.to_card, spots[1])] {
                        if let (Some(_), Some(at)) = (text, at) {
                            let drawn = format!("<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\"", at.0, at.1);
                            if !svg.contains(&drawn) {
                                cmiss += 1;
                            }
                        }
                    }
                }
                info.push_str(&format!(";cardmiss={cmiss}"));
            }
            out.push(Item { kind: kind.into(), svg: Some(svg), info });
        }
        Err(m) => out.push(Item { kind: kind.into(), svg: None, info: format!("PANIC {m}") }),
    }
}

fn push_er(out: &mut Vec<Item>, kind: &str, given: Option<&[(f64, f64)]>, is_static: bool, f: impl FnOnce() -> er::ErScene) {
    match run(|| {
        let es = f();
        let svg = er::to_svg(&es);
        (es, svg)
    }) {
        Ok((es, svg)) => {
            let mut info = scene_info(&es.scene, given, &svg, is_static);
            #[cfg(feature = "head")]
            {
                let vb = view_box(&svg);
                let sc = &es.scene;
                let exact = (vb.0 - sc.origin.0).abs() <= 0.051 && (vb.1 - sc.origin.1).abs() <= 0.051 && (vb.2 - sc.width).abs() <= 0.51 && (vb.3 - sc.height).abs() <= 0.51;
                info.push_str(&format!(";canvas={exact}"));
            }
            out.push(Item { kind: kind.into(), svg: Some(svg), info });
        }
        Err(m) => out.push(Item { kind: kind.into(), svg: None, info: format!("PANIC {m}") }),
    }
}

fn items(src: &str, lite: bool) -> Vec<Item> {
    let mut out = Vec::new();
    match run(|| flowmaid::render_svg(src)) {
        Ok(Ok(s)) => out.push(Item { kind: "render".into(), svg: Some(s), info: String::new() }),
        Ok(Err(e)) => out.push(Item { kind: "render".into(), svg: None, info: format!("ERR {}", e.to_string().replace(['\t', '\n'], " ")) }),
        Err(m) => out.push(Item { kind: "render".into(), svg: None, info: format!("PANIC {m}") }),
    }
    let doc = match run(|| parse_document(src)) {
        Ok(Ok(d)) => d,
        _ => return out,
    };
    match doc {
        Document::Flowchart(g) | Document::State(g) => graph_items(&g, lite, &mut out),
        Document::Class(d) => {
            let laid = match run(|| class::scene(&d)) {
                Ok(l) => l,
                Err(m) => {
                    out.push(Item { kind: "scene".into(), svg: None, info: format!("PANIC {m}") });
                    return out;
                }
            };
            push_class(&mut out, "scene", None, true, || class::scene(&d));
            let n = d.classes.len();
            let nn = d.notes.len();
            let all: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            if n > 0 {
                let mv = |i: usize, p: (f64, f64)| {
                    let mut m = all.clone();
                    m[i] = p;
                    m
                };
                let x0 = all[0];
                push_class(&mut out, "r0", Some(&all), false, || class::route(&d, &all));
                if nn > 0 {
                    push_class(&mut out, "r0c", None, false, || class::route(&d, &all[..n]));
                }
                let mut drags = vec![("id", mv(0, (x0.0 + 7.0, x0.1 + 5.0))), ("past1", mv(0, (-120.0, -80.0)))];
                if !lite {
                    drags.push(("lt", mv(0, (x0.0 - 5.0, x0.1 - 3.0))));
                    drags.push(("past2", mv(0, (-120.0, x0.1))));
                    drags.push(("past3", mv(0, (x0.0, -80.0))));
                    drags.push(("mid", mv(n / 2, (all[n / 2].0 + 31.0, all[n / 2].1 + 19.0))));
                }
                for (name, m) in &drags {
                    push_class(&mut out, &format!("r_{name}"), Some(m), false, || class::route(&d, m));
                }
            }
            #[cfg(feature = "head")]
            {
                let r = run(|| {
                    let mut bad = Vec::new();
                    for c in 0..n + nn + 4 {
                        let p: Vec<(f64, f64)> = (0..c).map(|i| all.get(i).copied().unwrap_or((5.0, 5.0))).collect();
                        let want = c == n || c == n + nn;
                        let got = class::try_route(&d, &p);
                        if got.is_some() != want {
                            bad.push(format!("c{c}"));
                        }
                        if let Some(g) = got {
                            if !lite && class::to_svg(&g) != class::to_svg(&class::route(&d, &p)) {
                                bad.push(format!("eq{c}"));
                            }
                        }
                    }
                    bad.join(",")
                });
                out.push(Item { kind: "try".into(), svg: None, info: format!("{r:?}") });
            }
        }
        Document::Er(d) => {
            let laid = match run(|| er::scene(&d)) {
                Ok(l) => l,
                Err(m) => {
                    out.push(Item { kind: "scene".into(), svg: None, info: format!("PANIC {m}") });
                    return out;
                }
            };
            push_er(&mut out, "scene", None, true, || er::scene(&d));
            let n = d.entities.len();
            let all: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
            if n > 0 {
                let mv = |i: usize, p: (f64, f64)| {
                    let mut m = all.clone();
                    m[i] = p;
                    m
                };
                let x0 = all[0];
                push_er(&mut out, "r0", Some(&all), false, || er::route(&d, &all));
                let mut drags = vec![("id", mv(0, (x0.0 + 7.0, x0.1 + 5.0))), ("past1", mv(0, (-120.0, -80.0)))];
                if !lite {
                    drags.push(("lt", mv(0, (x0.0 - 5.0, x0.1 - 3.0))));
                    drags.push(("past2", mv(0, (-120.0, x0.1))));
                    drags.push(("past3", mv(0, (x0.0, -80.0))));
                }
                for (name, m) in &drags {
                    push_er(&mut out, &format!("r_{name}"), Some(m), false, || er::route(&d, m));
                }
            }
            #[cfg(feature = "head")]
            {
                let r = run(|| {
                    let mut bad = Vec::new();
                    for c in 0..n + 4 {
                        let p: Vec<(f64, f64)> = (0..c).map(|i| all.get(i).copied().unwrap_or((5.0, 5.0))).collect();
                        let got = er::try_route(&d, &p);
                        if got.is_some() != (c == n) {
                            bad.push(format!("c{c}"));
                        }
                        if let Some(g) = got {
                            if !lite && er::to_svg(&g) != er::to_svg(&er::route(&d, &p)) {
                                bad.push(format!("eq{c}"));
                            }
                        }
                    }
                    bad.join(",")
                });
                out.push(Item { kind: "try".into(), svg: None, info: format!("{r:?}") });
            }
        }
        Document::Mindmap(d) => {
            match run(|| {
                let laid = mindmap::scene(&d);
                let pos: Vec<(f64, f64)> = laid.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                (mindmap::to_svg(&laid), mindmap::to_svg(&mindmap::route(&d, &pos)), pos)
            }) {
                Ok((a, b, _pos)) => {
                    out.push(Item { kind: "scene".into(), svg: Some(a), info: String::new() });
                    out.push(Item { kind: "r0".into(), svg: Some(b), info: String::new() });
                    #[cfg(feature = "head")]
                    {
                        let pos = _pos;
                        let n = d.nodes.len();
                        let r = run(|| {
                            let mut bad = Vec::new();
                            for c in 0..n + 4 {
                                let p: Vec<(f64, f64)> = (0..c).map(|i| pos.get(i).copied().unwrap_or((5.0, 5.0))).collect();
                                let got = mindmap::try_route(&d, &p);
                                if got.is_some() != (c == n) {
                                    bad.push(format!("c{c}"));
                                }
                                if let Some(g) = got {
                                    if mindmap::to_svg(&g) != mindmap::to_svg(&mindmap::route(&d, &p)) {
                                        bad.push(format!("eq{c}"));
                                    }
                                }
                            }
                            bad.join(",")
                        });
                        out.push(Item { kind: "try".into(), svg: None, info: format!("{r:?}") });
                    }
                }
                Err(m) => out.push(Item { kind: "scene".into(), svg: None, info: format!("PANIC {m}") }),
            }
        }
        _ => {}
    }
    out
}

fn main() {
    if std::env::var_os("NOHOOK").is_none() { std::panic::set_hook(Box::new(|_| {})); }
    let args: Vec<String> = std::env::args().collect();
    match args[1].as_str() {
        "all" => {
            let list = std::fs::read_to_string(&args[2]).unwrap();
            let lite = args.get(3).is_some_and(|a| a == "lite");
            for f in list.lines().filter(|l| !l.is_empty()) {
                let src = match std::fs::read_to_string(f) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                for it in items(&src, lite) {
                    let h = it.svg.as_deref().map(fnv).unwrap_or_else(|| "-".into());
                    println!("{f}\t{}\t{h}\t{}", it.kind, it.info);
                }
            }
        }
        "dump" => {
            // dump <listfile> <outdir> [lite]: every SVG as <outdir>/<line#>.<kind>.svg
            let list = std::fs::read_to_string(&args[2]).unwrap();
            let lite = args.get(4).is_some_and(|a| a == "lite");
            std::fs::create_dir_all(&args[3]).unwrap();
            for (i, f) in list.lines().filter(|l| !l.is_empty()).enumerate() {
                let src = match std::fs::read_to_string(f) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                for it in items(&src, lite) {
                    if let Some(svg) = it.svg {
                        std::fs::write(format!("{}/{i}.{}.svg", args[3], it.kind), svg).unwrap();
                    }
                }
            }
        }
        "fuzz" => {
            // fuzz <file> <set>: route / try_route with odd but finite (and non-finite) positions
            let src = std::fs::read_to_string(&args[2]).unwrap();
            let set = args[3].as_str();
            let gen = |n: usize, at: &[(f64, f64)]| -> Vec<(f64, f64)> {
                let mut s: u64 = 0x9e3779b97f4a7c15;
                let mut rnd = || {
                    s ^= s << 13;
                    s ^= s >> 7;
                    s ^= s << 17;
                    (s % 2_000_001) as f64 / 1_000_000.0 - 1.0
                };
                (0..n)
                    .map(|i| match set {
                        "zero" => (0.0, 0.0),
                        "same" => (100.0, 100.0),
                        "neg" => (-1.0e4 + i as f64, -2.0e4),
                        "far" => (rnd() * 1.0e6, rnd() * 1.0e6),
                        "huge1" => if i == 0 { (1.0e12, -1.0e12) } else { at[i] },
                        "huge300" => if i == 0 { (1.0e300, 1.0e300) } else { at[i] },
                        "max" => if i == 0 { (f64::MAX, f64::MIN) } else { at[i] },
                        "nan" => if i == 0 { (f64::NAN, 1.0) } else { at[i] },
                        "inf" => if i == 0 { (f64::INFINITY, 1.0) } else { at[i] },
                        "tiny" => (i as f64 * 1e-9, 0.0),
                        _ => at[i],
                    })
                    .collect()
            };
            let doc = parse_document(&src).unwrap();
            let r = run(|| match &doc {
                Document::Flowchart(g) | Document::State(g) => {
                    let at: Vec<(f64, f64)> = scene::scene(g).nodes.iter().map(|n| (n.x, n.y)).collect();
                    let p = gen(at.len(), &at);
                    #[cfg(feature = "head")]
                    {
                        let t = scene::try_route(g, &p).map(|s| scene::to_svg(&s).len());
                        format!("try={t:?}")
                    }
                    #[cfg(not(feature = "head"))]
                    {
                        format!("route={}", scene::to_svg(&scene::route(g, &p)).len())
                    }
                }
                Document::Class(d) => {
                    let at: Vec<(f64, f64)> = class::scene(d).scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    let p = gen(at.len(), &at);
                    #[cfg(feature = "head")]
                    {
                        format!("try={:?}", class::try_route(d, &p).map(|s| class::to_svg(&s).len()))
                    }
                    #[cfg(not(feature = "head"))]
                    {
                        format!("route={}", class::to_svg(&class::route(d, &p)).len())
                    }
                }
                Document::Er(d) => {
                    let at: Vec<(f64, f64)> = er::scene(d).scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    let p = gen(at.len(), &at);
                    #[cfg(feature = "head")]
                    {
                        format!("try={:?}", er::try_route(d, &p).map(|s| er::to_svg(&s).len()))
                    }
                    #[cfg(not(feature = "head"))]
                    {
                        format!("route={}", er::to_svg(&er::route(d, &p)).len())
                    }
                }
                Document::Mindmap(d) => {
                    let at: Vec<(f64, f64)> = mindmap::scene(d).nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                    let p = gen(at.len(), &at);
                    #[cfg(feature = "head")]
                    {
                        format!("try={:?}", mindmap::try_route(d, &p).map(|s| mindmap::to_svg(&s).len()))
                    }
                    #[cfg(not(feature = "head"))]
                    {
                        format!("route={}", mindmap::to_svg(&mindmap::route(d, &p)).len())
                    }
                }
                _ => "-".into(),
            });
            println!("{}\t{set}\t{}", args[2], match r {
                Ok(s) => s,
                Err(m) => format!("PANIC {m}"),
            });
        }
        #[cfg(feature = "head")]
        "chain" => {
            // chain <file>: frame 1 drags node k far past the top-left, frame 2 (route_partial
            // on frame 1's scene) brings it back to a smaller overflow; compare origin vs viewBox.
            let src = std::fs::read_to_string(&args[2]).unwrap();
            if let Ok(Document::Flowchart(g) | Document::State(g)) = parse_document(&src) {
                let laid = scene::scene(&g);
                let at: Vec<(f64, f64)> = laid.nodes.iter().map(|n| (n.x, n.y)).collect();
                let k = (0..at.len()).find(|&i| !laid.nodes[i].label.is_empty()).unwrap_or(0);
                let mut d1 = at.clone();
                d1[k] = (-300.0, -200.0);
                let f1 = scene::route_partial(&g, &d1, &laid, &at);
                let mut d2 = at.clone();
                d2[k] = (-40.0, -30.0);
                let f2 = scene::route_partial(&g, &d2, &f1, &d1);
                let f2r = scene::route(&g, &d2);
                for (name, sc) in [("f1", &f1), ("f2_partial", &f2), ("f2_route", &f2r)] {
                    let svg = scene::to_svg(sc);
                    let vb = view_box(&svg);
                    let nodes_ok = sc.nodes.iter().zip(if name == "f1" { &d1 } else { &d2 }).all(|(n, p)| (n.x, n.y) == *p);
                    println!("{name}\torigin=({:.1},{:.1})\tvb=({},{},{},{})\tsize=({:.1},{:.1})\tnodes_ok={nodes_ok}", sc.origin.0, sc.origin.1, vb.0, vb.1, vb.2, vb.3, sc.width, sc.height);
                }
            }
        }
        "one" => {
            let src = std::fs::read_to_string(&args[2]).unwrap();
            for it in items(&src, false) {
                if it.kind == args[3] {
                    print!("{}", it.svg.unwrap_or_default());
                }
            }
        }
        _ => panic!("usage"),
    }
}
