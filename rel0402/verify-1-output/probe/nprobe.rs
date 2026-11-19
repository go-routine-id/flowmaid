//! Verifier probe for the NEW functions (#111 route_partial, #112 picking):
//! contract checks, determinism and cost on every corpus input.
use flowmaid::model::Document;
use flowmaid::{class, er, journey, mindmap, parser, pie, sankey, seq, timeline};
use std::fmt::Debug;
use std::io::Write;
use std::time::Instant;

fn dbg<T: Debug>(t: &T) -> String {
    format!("{t:?}")
}

fn moved(c: &[(f64, f64)], k: usize, dx: f64, dy: f64) -> Vec<(f64, f64)> {
    let mut m = c.to_vec();
    m[k].0 += dx;
    m[k].1 += dy;
    m
}

fn picks(k: usize) -> Vec<usize> {
    // up to 12 indexes spread over 0..k
    if k <= 12 {
        (0..k).collect()
    } else {
        (0..12).map(|i| i * (k - 1) / 11).collect()
    }
}

/// Grid sweep: returns (hits as Debug string, mean ns, max ns).
fn sweep<H: Debug>(w: f64, h: f64, f: impl Fn(f64, f64, f64) -> Option<H>) -> (String, f64, f64) {
    let mut out = String::new();
    let mut tot = 0.0;
    let mut mx: f64 = 0.0;
    let n = 40;
    let mut cnt = 0.0;
    for tol in [-1.0, 0.0, 4.0] {
        for i in 0..=n {
            for j in 0..=n {
                let (x, y) = (w * i as f64 / n as f64, h * j as f64 / n as f64);
                let t = Instant::now();
                let r = f(x, y, tol);
                let ns = t.elapsed().as_nanos() as f64;
                tot += ns;
                cnt += 1.0;
                mx = mx.max(ns);
                out.push_str(&format!("{r:?};"));
            }
        }
    }
    // off canvas and odd inputs must be None / not panic
    for (x, y, tol) in [
        (-1.0, -1.0, 4.0),
        (w + 1.0, h / 2.0, 4.0),
        (w / 2.0, h + 1.0, 4.0),
        (f64::NAN, 1.0, 4.0),
        (1.0, f64::NAN, 4.0),
        (f64::INFINITY, 1.0, 4.0),
    ] {
        let r = f(x, y, tol);
        if r.is_some() {
            out.push_str(&format!("OFFCANVAS-HIT({x},{y})"));
        }
    }
    let _ = f(w / 2.0, h / 2.0, f64::NAN);
    let _ = f(w / 2.0, h / 2.0, f64::INFINITY);
    let _ = f(w / 2.0, h / 2.0, 1e9);
    (out, tot / cnt, mx)
}

#[derive(Default)]
struct Tally {
    selfhit: usize,
    other: usize,
    none: usize,
    notes: Vec<String>,
}
impl Tally {
    fn add<H: Debug + PartialEq>(&mut self, what: &str, got: Option<H>, want: H) {
        match got {
            Some(g) if g == want => self.selfhit += 1,
            Some(g) => {
                self.other += 1;
                if self.notes.len() < 6 {
                    self.notes.push(format!("{what}:{want:?}->{g:?}"));
                }
            }
            None => {
                self.none += 1;
                if self.notes.len() < 12 {
                    self.notes.push(format!("{what}:{want:?}->NONE"));
                }
            }
        }
    }
    fn row(&self) -> String {
        format!("self={} other={} none={} [{}]", self.selfhit, self.other, self.none, self.notes.join(" | "))
    }
}

fn cubic_mid(b: &[(f64, f64); 4]) -> (f64, f64) {
    let t = 0.5;
    let u = 1.0 - t;
    let k = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    (
        k[0] * b[0].0 + k[1] * b[1].0 + k[2] * b[2].0 + k[3] * b[3].0,
        k[0] * b[0].1 + k[1] * b[1].1 + k[2] * b[2].1 + k[3] * b[3].1,
    )
}

fn main() {
    let list = std::fs::read_to_string(std::env::args().nth(1).expect("list")).unwrap();
    let out = std::io::stdout();
    let mut out = out.lock();
    for path in list.lines().filter(|l| !l.is_empty()) {
        let Ok(src) = std::fs::read_to_string(path) else { continue };
        let r = std::panic::catch_unwind(|| -> (String, Vec<String>) {
            let Ok(doc) = parser::parse_document(&src) else {
                return ("parse-err".into(), vec![]);
            };
            let mut cols = Vec::new();
            match doc {
                Document::Er(d) => {
                    let base = er::scene(&d);
                    let c: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    if c.is_empty() {
                        return ("er".into(), vec!["empty".into()]);
                    }
                    let (mut bad, mut frames, mut nondet, mut tp, mut tf) = (0, 0, 0, 0.0, 0.0);
                    let routed = er::route(&d, &c);
                    for i in picks(c.len()) {
                        for b in [&base, &routed] {
                            let to = moved(&c, i, 37.0, 23.0);
                            let t = Instant::now();
                            let f = er::route_partial(&d, &to, b, &c);
                            tp += t.elapsed().as_secs_f64();
                            let t = Instant::now();
                            let full = er::route(&d, &to);
                            tf += t.elapsed().as_secs_f64();
                            let f2 = er::route_partial(&d, &to, b, &c);
                            if dbg(&f) != dbg(&f2) {
                                nondet += 1;
                            }
                            frames += 1;
                            for (j, r) in d.relations.iter().enumerate() {
                                if r.from != i && r.to != i && dbg(&f.scene.edges[j]) != dbg(&b.scene.edges[j]) {
                                    bad += 1;
                                }
                            }
                            for (k, n) in f.scene.nodes.iter().enumerate() {
                                if (n.x, n.y) != to[k] || n.w != full.scene.nodes[k].w || n.h != full.scene.nodes[k].h {
                                    bad += 1;
                                }
                            }
                            if dbg(&f.tables) != dbg(&full.tables) || dbg(&f.cards) != dbg(&full.cards) {
                                bad += 1;
                            }
                            let _ = er::to_svg(&f);
                            // chained frame from f
                            let to2 = moved(&to, i, 5.0, -9.0);
                            let f3 = er::route_partial(&d, &to2, &f, &to);
                            for (j, r) in d.relations.iter().enumerate() {
                                if r.from != i && r.to != i && dbg(&f3.scene.edges[j]) != dbg(&f.scene.edges[j]) {
                                    bad += 1;
                                }
                            }
                        }
                    }
                    // edit fallback: a new entity related to the first
                    let first = &d.entities[0].name;
                    let src2 = format!("{src}\n    ZZNEWENT ||--o{{ {first} : zznew\n");
                    let mut edit = "edit=n/a".to_string();
                    if let Ok(Document::Er(d2)) = parser::parse_document(&src2) {
                        if d2.entities.len() == d.entities.len() + 1 {
                            let mut c2 = c.clone();
                            c2.push((c[0].0 + 333.0, c[0].1 + 77.0));
                            let a = er::route_partial(&d2, &c2, &base, &c);
                            let b = er::route(&d2, &c2);
                            edit = format!("edit={}", dbg(&a) == dbg(&b));
                        }
                    }
                    let same_nomove = dbg(&er::route_partial(&d, &c, &base, &c)) == dbg(&base);
                    let none_short = er::try_route_partial(&d, &c[..c.len() - 1], &base, &c).is_none();
                    cols.push(format!(
                        "n={} m={} frames={frames} bad={bad} nondet={nondet} {edit} nomove={same_nomove} tryshort={none_short} partial_ms={:.3} full_ms={:.3}",
                        c.len(),
                        d.relations.len(),
                        tp * 1e3 / frames as f64,
                        tf * 1e3 / frames as f64
                    ));
                    return ("er".into(), cols);
                }
                Document::Class(d) => {
                    let base = class::scene(&d);
                    let c: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    let nc = d.classes.len();
                    if nc == 0 {
                        return ("class".into(), vec!["empty".into()]);
                    }
                    let (mut bad, mut frames, mut nondet, mut cardmoves, mut kept_rels) = (0, 0, 0, 0, 0);
                    let (mut tp, mut tf) = (0.0, 0.0);
                    let mut badnote = Vec::new();
                    let routed = class::route(&d, &c);
                    let base_cards = class::card_spots(&base);
                    let routed_cards = class::card_spots(&routed);
                    for i in picks(nc) {
                        for (bi, b) in [&base, &routed].into_iter().enumerate() {
                            let bcards = if bi == 0 { &base_cards } else { &routed_cards };
                            for cl in [false, true] {
                                let (cc, bb, bc);
                                let rb;
                                if cl {
                                    cc = c[..nc].to_vec();
                                    rb = class::route(&d, &cc);
                                    bb = &rb;
                                    bc = class::card_spots(bb);
                                } else {
                                    cc = c.clone();
                                    bb = b;
                                    bc = bcards.clone();
                                }
                                let to = moved(&cc, i, -29.0, 31.0);
                                let t = Instant::now();
                                let f = class::route_partial(&d, &to, bb, &cc);
                                tp += t.elapsed().as_secs_f64();
                                let t = Instant::now();
                                let full = class::route(&d, &to);
                                tf += t.elapsed().as_secs_f64();
                                frames += 1;
                                if dbg(&f) != dbg(&class::route_partial(&d, &to, bb, &cc)) {
                                    nondet += 1;
                                }
                                let idx = |id: &str| f.scene.nodes.iter().position(|n| n.id == id).unwrap();
                                let fc = class::card_spots(&f);
                                for (j, e) in bb.scene.edges.iter().enumerate() {
                                    let (a, z) = (idx(&e.from), idx(&e.to));
                                    let mv = |k: usize| {
                                        (f.scene.nodes[k].x, f.scene.nodes[k].y)
                                            != (bb.scene.nodes[k].x, bb.scene.nodes[k].y)
                                    };
                                    if !mv(a) && !mv(z) {
                                        if dbg(&f.scene.edges[j]) != dbg(e) {
                                            bad += 1;
                                            if badnote.len() < 3 {
                                                badnote.push(format!("edge{j}"));
                                            }
                                        }
                                        if j < d.relations.len() {
                                            kept_rels += 1;
                                            if fc.get(j) != bc.get(j) {
                                                cardmoves += 1;
                                            }
                                        }
                                    }
                                }
                                for (k, n) in f.scene.nodes.iter().enumerate() {
                                    if k < to.len() && (n.x, n.y) != to[k] {
                                        bad += 1;
                                        badnote.push(format!("node{k}"));
                                    }
                                    if (n.x, n.y, n.w, n.h) != (full.scene.nodes[k].x, full.scene.nodes[k].y, full.scene.nodes[k].w, full.scene.nodes[k].h) {
                                        bad += 1;
                                        if badnote.len() < 4 {
                                            badnote.push(format!("nodevsfull{k}"));
                                        }
                                    }
                                }
                                if dbg(&f.boxes) != dbg(&full.boxes) || dbg(&f.rels) != dbg(&full.rels) || dbg(&f.notes) != dbg(&full.notes) {
                                    bad += 1;
                                    badnote.push("boxes".into());
                                }
                                let _ = class::to_svg(&f);
                            }
                        }
                    }
                    let first = &d.classes[0].name;
                    let src2 = format!("{src}\n    {first} <|-- ZZNewClass\n");
                    let mut edit = "edit=n/a".to_string();
                    if let Ok(Document::Class(d2)) = parser::parse_document(&src2) {
                        if d2.classes.len() == nc + 1 {
                            let mut c2 = c[..nc].to_vec();
                            c2.push((c[0].0 + 333.0, c[0].1 + 77.0));
                            let a = class::route_partial(&d2, &c2, &base, &c);
                            let b = class::route(&d2, &c2);
                            edit = format!("edit={}", dbg(&a) == dbg(&b));
                        }
                    }
                    let same_nomove = dbg(&class::route_partial(&d, &c, &base, &c)) == dbg(&base);
                    cols.push(format!(
                        "n={} notes={} frames={frames} bad={bad} nondet={nondet} {edit} nomove={same_nomove} kept_rels={kept_rels} card_moved={cardmoves} partial_ms={:.3} full_ms={:.3} {}",
                        nc,
                        d.notes.len(),
                        tp * 1e3 / frames as f64,
                        tf * 1e3 / frames as f64,
                        badnote.join(",")
                    ));
                    return ("class".into(), cols);
                }
                Document::Mindmap(d) => {
                    let base = mindmap::scene(&d);
                    let c: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                    if c.is_empty() {
                        return ("mind".into(), vec!["empty".into()]);
                    }
                    let (mut svgeq, mut dbgeq, mut frames, mut nondet, mut bad) = (0, 0, 0, 0, 0);
                    let ends = mindmap::link_ends(&d);
                    if ends.len() != base.edges.len() {
                        bad += 1000;
                    }
                    for i in picks(c.len()) {
                        let to = moved(&c, i, -41.0, 17.0);
                        let f = mindmap::route_partial(&d, &to, &base, &c);
                        let full = mindmap::route(&d, &to);
                        frames += 1;
                        if mindmap::to_svg(&f) == mindmap::to_svg(&full) {
                            svgeq += 1;
                        }
                        if dbg(&f) == dbg(&full) {
                            dbgeq += 1;
                        }
                        if dbg(&f) != dbg(&mindmap::route_partial(&d, &to, &base, &c)) {
                            nondet += 1;
                        }
                        for (j, &(p, ch)) in ends.iter().enumerate() {
                            if p != i && ch != i && dbg(&f.edges[j]) != dbg(&base.edges[j]) {
                                bad += 1;
                            }
                        }
                    }
                    // picking on the laid-out mindmap
                    let mut t = Tally::default();
                    for (i, n) in base.nodes.iter().enumerate() {
                        t.add("node", base.hit_test(n.cx(), n.cy(), 0.0), mindmap::MindHit::Node(i));
                    }
                    for (j, e) in base.edges.iter().enumerate() {
                        let m = cubic_mid(&e.bezier);
                        t.add("link", base.hit_test(m.0, m.1, 0.0), mindmap::MindHit::Link(j));
                    }
                    let (s1, mean, mx) = sweep(base.width, base.height, |x, y, tol| base.hit_test(x, y, tol));
                    let (s2, _, _) = sweep(base.width, base.height, |x, y, tol| base.hit_test(x, y, tol));
                    let empty = mindmap::route(&d, &[]);
                    let e_none = empty.hit_test(1.0, 1.0, 4.0).is_none();
                    cols.push(format!(
                        "n={} frames={frames} svgeq={svgeq} dbgeq={dbgeq} bad={bad} nondet={nondet} | pick {} det={} emptyNone={e_none} mean_ns={mean:.0} max_ns={mx:.0} off={}",
                        c.len(),
                        t.row(),
                        s1 == s2,
                        s1.contains("OFFCANVAS")
                    ));
                    return ("mind".into(), cols);
                }
                Document::Sequence(d) => {
                    let sc = seq::scene(&d);
                    let mut t = Tally::default();
                    use seq::SeqHit as H;
                    for (k, b) in sc.boxes.iter().enumerate() {
                        let p = if b.actor { (b.x + b.w / 2.0, b.y + 7.0) } else { (b.x + b.w / 2.0, b.y + b.h / 2.0) };
                        t.add("box", sc.hit_test(p.0, p.1, 0.0), H::Participant(k));
                    }
                    for (k, b) in sc.footer.iter().enumerate() {
                        let p = if b.actor { (b.x + b.w / 2.0, b.y + 7.0) } else { (b.x + b.w / 2.0, b.y + b.h / 2.0) };
                        t.add("foot", sc.hit_test(p.0, p.1, 0.0), H::Footer(k));
                    }
                    for (i, m) in sc.messages.iter().enumerate() {
                        if m.points.len() >= 2 {
                            let (a, b) = (m.points[0], m.points[1]);
                            t.add("msg", sc.hit_test((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, 0.0), H::Message(i));
                        }
                    }
                    for (i, nb) in sc.notes.iter().enumerate() {
                        t.add("note", sc.hit_test(nb.x + nb.w / 2.0, nb.y + nb.h / 2.0, 0.0), H::Note(i));
                    }
                    for (i, a) in sc.activations.iter().enumerate() {
                        t.add("act", sc.hit_test(a.x, (a.y0 + a.y1) / 2.0, 0.0), H::Activation(i));
                    }
                    for (k, l) in sc.lifelines.iter().enumerate() {
                        t.add("life", sc.hit_test(l.x, (l.y0 + l.y1) / 2.0, 2.0), H::Lifeline(k));
                    }
                    for (i, f) in sc.frames.iter().enumerate() {
                        t.add("frameB", sc.hit_test(f.x + f.w * 0.37, f.y + f.h, 0.0), H::Frame(i));
                    }
                    let (s1, mean, mx) = sweep(sc.width, sc.height, |x, y, tol| sc.hit_test(x, y, tol));
                    let (s2, _, _) = sweep(sc.width, sc.height, |x, y, tol| sc.hit_test(x, y, tol));
                    cols.push(format!(
                        "parts={} msgs={} notes={} acts={} frames={} | pick {} det={} mean_ns={mean:.0} max_ns={mx:.0} off={}",
                        sc.boxes.len(), sc.messages.len(), sc.notes.len(), sc.activations.len(), sc.frames.len(),
                        t.row(), s1 == s2, s1.contains("OFFCANVAS")
                    ));
                    return ("seq".into(), cols);
                }
                Document::Pie(d) => {
                    let ps = pie::scene(&d);
                    let mut t = Tally::default();
                    use pie::PieHit as H;
                    for (i, sl) in ps.slices.iter().enumerate() {
                        if sl.frac <= 0.0 {
                            continue;
                        }
                        let a = (sl.start_angle + sl.end_angle) / 2.0;
                        let (x, y) = (ps.cx + 0.93 * ps.r * a.sin(), ps.cy - 0.93 * ps.r * a.cos());
                        t.add("slice", ps.hit_test(x, y, 0.0), H::Slice(i));
                    }
                    for (i, r) in ps.legend.iter().enumerate() {
                        t.add("legend", ps.hit_test(r.x + 6.0, r.y, 0.0), H::Legend(i));
                    }
                    for (k, l) in ps.labels.iter().enumerate() {
                        let x = match l.anchor { "end" => l.x - 2.0, "start" => l.x + 2.0, _ => l.x };
                        t.add("label", ps.hit_test(x, l.y, 0.0), H::Label { label: k, slice: l.slice });
                    }
                    let (s1, mean, mx) = sweep(ps.width, ps.height, |x, y, tol| ps.hit_test(x, y, tol));
                    let (s2, _, _) = sweep(ps.width, ps.height, |x, y, tol| ps.hit_test(x, y, tol));
                    cols.push(format!(
                        "slices={} | pick {} det={} mean_ns={mean:.0} max_ns={mx:.0} off={}",
                        ps.slices.len(), t.row(), s1 == s2, s1.contains("OFFCANVAS")
                    ));
                    return ("pie".into(), cols);
                }
                Document::Journey(d) => {
                    let js = journey::scene(&d);
                    let mut t = Tally::default();
                    use journey::JourneyHit as H;
                    for (i, tk) in js.tasks.iter().enumerate() {
                        t.add("task", js.hit_test(tk.cx, tk.cy, 0.0), H::Task(i));
                    }
                    for (i, l) in js.legend.iter().enumerate() {
                        t.add("legend", js.hit_test(l.x + 5.0, l.y, 0.0), H::Legend(i));
                    }
                    for (i, s) in js.sections.iter().enumerate() {
                        t.add("section", js.hit_test(s.x + s.w / 2.0, s.y + s.h / 2.0, 0.0), H::Section(i));
                    }
                    let (s1, mean, mx) = sweep(js.width, js.height, |x, y, tol| js.hit_test(x, y, tol));
                    let (s2, _, _) = sweep(js.width, js.height, |x, y, tol| js.hit_test(x, y, tol));
                    cols.push(format!(
                        "tasks={} | pick {} det={} mean_ns={mean:.0} max_ns={mx:.0} off={}",
                        js.tasks.len(), t.row(), s1 == s2, s1.contains("OFFCANVAS")
                    ));
                    return ("journey".into(), cols);
                }
                Document::Timeline(d) => {
                    let ts = timeline::scene(&d);
                    let mut t = Tally::default();
                    use timeline::TimelineHit as H;
                    let mut nev = 0;
                    for (s, sec) in ts.sections.iter().enumerate() {
                        if !sec.name.is_empty() {
                            t.add("section", ts.hit_test(sec.x + 3.0, sec.y + 6.0, 0.0), H::Section(s));
                        }
                        for (k, p) in sec.periods.iter().enumerate() {
                            t.add("period", ts.hit_test(p.cx, p.label_y, 0.0), H::Period { section: s, period: k });
                            for (e, (_, y)) in p.events.iter().enumerate() {
                                nev += 1;
                                t.add("event", ts.hit_test(p.cx, *y, 0.0), H::Event { section: s, period: k, event: e });
                            }
                        }
                    }
                    let (s1, mean, mx) = sweep(ts.width, ts.height, |x, y, tol| ts.hit_test(x, y, tol));
                    let (s2, _, _) = sweep(ts.width, ts.height, |x, y, tol| ts.hit_test(x, y, tol));
                    cols.push(format!(
                        "sections={} events={nev} | pick {} det={} mean_ns={mean:.0} max_ns={mx:.0} off={}",
                        ts.sections.len(), t.row(), s1 == s2, s1.contains("OFFCANVAS")
                    ));
                    return ("timeline".into(), cols);
                }
                Document::Sankey(d) => {
                    let ss = sankey::scene(&d);
                    let mut t = Tally::default();
                    use sankey::SankeyHit as H;
                    for (i, n) in ss.nodes.iter().enumerate() {
                        t.add("node", ss.hit_test(n.x + n.w / 2.0, n.y + n.h / 2.0, 0.0), H::Node(i));
                    }
                    for (i, l) in ss.links.iter().enumerate() {
                        let (x, y) = ((l.x0 + l.x1) / 2.0, (l.y0 + l.y1) / 2.0);
                        t.add("link", ss.hit_test(x, y, 0.0), H::Link(i));
                    }
                    let (s1, mean, mx) = sweep(ss.width, ss.height, |x, y, tol| ss.hit_test(x, y, tol));
                    let (s2, _, _) = sweep(ss.width, ss.height, |x, y, tol| ss.hit_test(x, y, tol));
                    cols.push(format!(
                        "nodes={} links={} | pick {} det={} mean_ns={mean:.0} max_ns={mx:.0} off={}",
                        ss.nodes.len(), ss.links.len(), t.row(), s1 == s2, s1.contains("OFFCANVAS")
                    ));
                    return ("sankey".into(), cols);
                }
                _ => return ("other".into(), cols),
            }
        });
        match r {
            Ok((k, cols)) => {
                if k != "other" && k != "parse-err" {
                    writeln!(out, "{path}\t{k}\t{}", cols.join("\t")).unwrap()
                }
            }
            Err(_) => writeln!(out, "{path}\tPANIC").unwrap(),
        }
        out.flush().ok();
    }
}
