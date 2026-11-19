// Independent verifier (v2): scene() vs route(g, scene centres) and vs a
// 1px route_partial drag, on composite + note state diagrams from a
// generator unlike the implementer's / earlier verifier's.
use flowmaid::scene::{route, route_partial, scene, SceneEdge};
use std::panic::{catch_unwind, AssertUnwindSafe};

struct R(u64);
impl R {
    fn n(&mut self, n: usize) -> usize {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as usize) % n.max(1)
    }
    fn p<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.n(v.len())]
    }
}

fn note(r: &mut R, ind: &str, at: &str, tag: &str) -> String {
    let side = *r.p(&["left", "right"]);
    match r.n(4) {
        0 => format!("{ind}note {side} of {at}\n{ind}  {tag} line one\n{ind}  second line\n{ind}end note\n"),
        _ => format!("{ind}note {side} of {at} : {tag}\n"),
    }
}

// A composite with members, maybe a nested composite; returns (members, composites)
fn comp(r: &mut R, s: &mut String, name: &str, depth: usize, ind: &str) -> (Vec<String>, Vec<String>) {
    s.push_str(&format!("{ind}state {name} {{\n"));
    let i2 = format!("{ind}  ");
    if r.n(4) == 0 {
        s.push_str(&format!("{i2}direction {}\n", r.p(&["LR", "TB", "RL", "BT"])));
    }
    let k = 1 + r.n(4);
    let mut mem: Vec<String> = (0..k).map(|i| format!("{name}_{i}")).collect();
    let mut comps = vec![name.to_string()];
    s.push_str(&format!("{i2}[*] --> {}\n", mem[0]));
    for i in 1..k {
        let f = mem[r.n(i)].clone();
        s.push_str(&format!("{i2}{f} --> {}{}\n", mem[i], if r.n(2) == 0 { format!(" : t{i}") } else { String::new() }));
    }
    if depth < 2 && r.n(3) == 0 {
        let sub = format!("{name}N");
        let (m2, c2) = comp(r, s, &sub, depth + 1, &i2);
        let f = r.p(&mem).clone();
        s.push_str(&format!("{i2}{f} --> {sub}\n"));
        if r.n(2) == 0 {
            let t = r.p(&mem).clone();
            s.push_str(&format!("{i2}{sub} --> {t} : up\n"));
        }
        if r.n(3) == 0 {
            // into a nested member directly
            let a = r.p(&mem).clone();
            let b = r.p(&m2).clone();
            s.push_str(&format!("{i2}{a} --> {b}\n"));
        }
        mem.extend(m2);
        comps.extend(c2);
    }
    if r.n(5) == 0 {
        s.push_str(&format!("{i2}state {name}_f <<fork>>\n{i2}{} --> {name}_f\n{i2}{name}_f --> {}\n{i2}{name}_f --> {}\n", mem[0], r.p(&mem), r.p(&mem)));
    }
    if r.n(5) == 0 {
        s.push_str(&format!("{i2}state {name}_c <<choice>>\n{i2}{} --> {name}_c\n{i2}{name}_c --> {} : a\n{i2}{name}_c --> {} : b\n", r.p(&mem), r.p(&mem), r.p(&mem)));
    }
    for _ in 0..r.n(4) {
        let at = r.p(&mem).clone();
        let t = format!("in{}", r.n(900));
        s.push_str(&note(r, &i2, &at, &t));
    }
    if r.n(5) == 0 {
        s.push_str(&format!("{i2}{} --> [*]\n", r.p(&mem)));
    }
    s.push_str(&format!("{ind}}}\n"));
    (mem, comps)
}

fn gen(r: &mut R) -> String {
    let mut s = String::from("stateDiagram-v2\n");
    s.push_str(r.p(&["", "direction LR\n", "direction RL\n", "direction BT\n", "direction TB\n", "direction LR\n"]));
    let mut top: Vec<String> = vec!["A".into()];
    s.push_str("[*] --> A\n");
    let mut inner: Vec<String> = Vec::new();
    let mut comps: Vec<String> = Vec::new();
    for c in 0..1 + r.n(3) {
        let name = format!("K{c}");
        let (m, cs) = comp(r, &mut s, &name, 0, "");
        inner.extend(m);
        comps.extend(cs);
        top.push(name);
    }
    for t in 0..r.n(4) {
        top.push(format!("B{t}"));
    }
    for i in 1..top.len() {
        let f = top[r.n(i)].clone();
        s.push_str(&format!("{f} --> {}{}\n", top[i], if r.n(3) == 0 { format!(" : g{i}") } else { String::new() }));
    }
    for _ in 0..r.n(3) {
        let a = r.p(&top).clone();
        let b = r.p(&inner).clone();
        if r.n(2) == 0 {
            s.push_str(&format!("{a} --> {b}\n"));
        } else {
            s.push_str(&format!("{b} --> {a} : out\n"));
        }
    }
    if r.n(3) == 0 && comps.len() > 1 {
        let a = r.p(&comps).clone();
        let b = r.p(&comps).clone();
        if a != b {
            s.push_str(&format!("{a} --> {b} : cc\n"));
        }
    }
    if r.n(2) == 0 {
        let (a, b) = (r.n(top.len()), r.n(top.len()));
        s.push_str(&format!("{} --> {} : back\n", top[a.max(b)], top[a.min(b)]));
    }
    // notes on outer states / composites; sometimes two on one state
    for _ in 0..1 + r.n(4) {
        let at = if r.n(3) == 0 { r.p(&comps).clone() } else { r.p(&top).clone() };
        let t = format!("on{}", r.n(900));
        let nn = note(r, "", &at, &t);
        s.push_str(&nn);
        if r.n(4) == 0 {
            s.push_str(&nn.replace(&t, &format!("{t}b")));
        }
    }
    if r.n(3) == 0 {
        s.push_str(&format!("{} --> [*]\n", r.p(&top)));
    }
    s
}

fn card(id: &str) -> bool {
    id.starts_with("__note_")
}

fn dev(a: &SceneEdge, b: &SceneEdge) -> f64 {
    let mut m: f64 = 0.0;
    for (p, q) in a.bezier.iter().zip(&b.bezier) {
        m = m.max((p.0 - q.0).abs()).max((p.1 - q.1).abs());
    }
    if a.waypoints.len() != b.waypoints.len() {
        return 1e9;
    }
    for (p, q) in a.waypoints.iter().zip(&b.waypoints) {
        m = m.max((p.0 - q.0).abs()).max((p.1 - q.1).abs());
    }
    m
}

fn elbow(e: &SceneEdge) -> bool {
    let [p0, c1, c2, p3] = e.bezier;
    let nr = |a: f64, b: f64| (a - b).abs() < 1e-6;
    e.waypoints.is_empty()
        && !nr(p0.0, p3.0)
        && !nr(p0.1, p3.1)
        && ((nr(c1.0, p0.0) && !nr(c1.1, p0.1) && nr(c2.1, p3.1) && !nr(c2.0, p3.0))
            || (nr(c1.1, p0.1) && !nr(c1.0, p0.0) && nr(c2.0, p3.0) && !nr(c2.1, p3.1)))
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    let n: usize = std::env::var("N").ok().and_then(|v| v.parse().ok()).unwrap_or(150);
    let mut srcs: Vec<(String, String)> = files.iter().map(|p| (p.rsplit('/').next().unwrap().to_string(), std::fs::read_to_string(p).unwrap())).collect();
    let mut r = R(0x9E37_79B9_7F4A_7C15);
    for k in 0..n {
        srcs.push((format!("g{k}"), gen(&mut r)));
    }
    if let Ok(d) = std::env::var("DUMP") {
        for (nm, s) in &srcs {
            std::fs::write(format!("{d}/{nm}.mmd"), s).ok();
        }
    }
    let mut tot = [0usize; 4]; // EE PP EP PE
    let (mut links, mut gt20, mut gt1, mut panics, mut drag20, mut other20) = (0, 0, 0, 0, 0, 0);
    for (nm, src) in &srcs {
        let res = catch_unwind(AssertUnwindSafe(|| {
            let g = match flowmaid::parser::parse_document(src) {
                Ok(flowmaid::Document::State(g)) => g,
                o => return Err(format!("{:?}", o.err())),
            };
            let s = scene(&g);
            let c: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
            let rt = route(&g, &c);
            let mut lines = Vec::new();
            let mut other = 0.0f64;
            for (ei, e) in g.edges.iter().enumerate() {
                let d = dev(&s.edges[ei], &rt.edges[ei]);
                if !(card(&g.nodes[e.from].id) || card(&g.nodes[e.to].id)) {
                    other = other.max(d);
                    continue;
                }
                let cls = (elbow(&s.edges[ei]), elbow(&rt.edges[ei]));
                lines.push((ei, cls, d, format!("{}->{} S{:?} R{:?}", g.nodes[e.from].id, g.nodes[e.to].id, s.edges[ei].bezier, rt.edges[ei].bezier)));
            }
            // first drag: 1px move of each non-card node via route_partial
            let mut dmax = 0.0f64;
            let mut dwho = String::new();
            for v in 0..g.nodes.len() {
                if card(&g.nodes[v].id) {
                    continue;
                }
                let mut c2 = c.clone();
                c2[v].0 += 1.0;
                let rp = route_partial(&g, &c2, &s, &c);
                for (ei, e) in g.edges.iter().enumerate() {
                    if !(card(&g.nodes[e.from].id) || card(&g.nodes[e.to].id)) {
                        continue;
                    }
                    let d = dev(&s.edges[ei], &rp.edges[ei]);
                    if d > dmax {
                        dmax = d;
                        dwho = format!("drag {} -> edge {} {}->{}", g.nodes[v].id, ei, g.nodes[e.from].id, g.nodes[e.to].id);
                    }
                }
            }
            Ok((lines, dmax, dwho, other))
        }));
        match res {
            Err(_) => {
                panics += 1;
                println!("PANIC {nm}");
            }
            Ok(Err(e)) => println!("SKIP {nm} {e}"),
            Ok(Ok((lines, dmax, dwho, other))) => {
                for (ei, cls, d, desc) in lines {
                    links += 1;
                    let k = match cls {
                        (true, true) => 0,
                        (false, false) => 1,
                        (true, false) => 2,
                        (false, true) => 3,
                    };
                    tot[k] += 1;
                    let tag = ["EE", "PP", "EP", "PE"][k];
                    println!("LINK {nm} {ei} {tag} {d:.2}");
                    if d > 1e-6 {
                        gt1 += 1;
                    }
                    if d > 20.0 || k >= 2 {
                        gt20 += (d > 20.0) as usize;
                        println!("BIG {nm} e{ei} {tag} dev={d:.1} {desc}");
                    }
                }
                if dmax > 20.0 {
                    drag20 += 1;
                    println!("DRAG {nm} {dmax:.1} {dwho}");
                }
                if other > 20.0 {
                    other20 += 1;
                }
                println!("DIAG {nm} drag={dmax:.2} other_edges_dev={other:.2}");
            }
        }
    }
    println!("SUMMARY diagrams={} links={links} EE={} PP={} EP={} PE={} dev>0={gt1} dev>20={gt20} drag_jump>20_diagrams={drag20} diagrams_with_nonnote_edge_dev>20={other20} panics={panics}", srcs.len(), tot[0], tot[1], tot[2], tot[3]);
}
