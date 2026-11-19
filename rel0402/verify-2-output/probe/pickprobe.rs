//! #112 picking probe (verify round 2, HEAD only): determinism, tol rules, off-canvas, centres, cost.
//! usage: pickprobe <listfile> [grid]  -> stdout TSV per file; findings lines start with '!'
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::{journey, mindmap, pie, sankey, seq, timeline};
use std::io::BufRead;
use std::time::Instant;

/// (hit as string, whether the hit is a purely filled element)
type Pick<'a> = Box<dyn Fn(f64, f64, f64) -> Option<(String, bool)> + 'a>;

fn grid_run(name: &str, kind: &str, size: usize, w: f64, h: f64, pick: &Pick, grid: usize, out: &mut Vec<String>) {
    let mut pts = Vec::with_capacity(grid * grid);
    for a in 0..grid {
        for b in 0..grid {
            pts.push((w * (a as f64 + 0.37) / grid as f64, h * (b as f64 + 0.61) / grid as f64));
        }
    }
    // determinism + tol rules
    let mut viol = 0usize;
    let mut negdiff = 0usize;
    let mut nondet = 0usize;
    let mut hits = 0usize;
    let mut example = String::new();
    for &(x, y) in &pts {
        let r0 = pick(x, y, 0.0);
        let rneg = pick(x, y, -5.0);
        let r0b = pick(x, y, 0.0);
        let r10 = pick(x, y, 10.0);
        if r0 != r0b { nondet += 1; }
        if r0 != rneg { negdiff += 1; }
        if r0.is_some() { hits += 1; }
        if let Some((ref s, true)) = r10 {
            if r0.as_ref().map(|r| &r.0) != Some(s) {
                viol += 1;
                if example.is_empty() { example = format!("({x:.2},{y:.2}) tol10={s} tol0={r0:?}"); }
            }
        }
    }
    if nondet > 0 { out.push(format!("!NONDET {kind} {name} {nondet}")); }
    if negdiff > 0 { out.push(format!("!NEGTOL {kind} {name} {negdiff}")); }
    if viol > 0 { out.push(format!("!FILLED_WIDENED {kind} {name} {viol} e.g. {example}")); }
    for (x, y) in [(-1.0, -1.0), (w + 1.0, 1.0), (1.0, h + 1.0), (f64::NAN, 1.0), (1.0, f64::INFINITY)] {
        if let Some(r) = pick(x, y, 50.0) { out.push(format!("!OFFCANVAS {kind} {name} ({x},{y}) -> {r:?}")); }
    }
    // cost: tol 4, every grid point, take mean and max
    let reps = 3;
    let mut max_ns = 0u128;
    let t0 = Instant::now();
    for _ in 0..reps {
        for &(x, y) in &pts {
            let t = Instant::now();
            std::hint::black_box(pick(x, y, 4.0));
            max_ns = max_ns.max(t.elapsed().as_nanos());
        }
    }
    let mean_us = t0.elapsed().as_secs_f64() * 1e6 / (reps * pts.len()) as f64;
    out.push(format!("COST\t{kind}\t{name}\t{size}\t{mean_us:.3}\t{:.3}\t{hits}/{}", max_ns as f64 / 1e3, pts.len()));
}

fn main() {
    let mut args = std::env::args().skip(1);
    let list = args.next().expect("list");
    let grid: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(60);
    for line in std::io::BufReader::new(std::fs::File::open(list).unwrap()).lines() {
        let path = line.unwrap();
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        let Ok(doc) = parse_document(&src) else { continue };
        let mut out: Vec<String> = Vec::new();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match doc {
                Document::Mindmap(d) => {
                    let ms = mindmap::scene(&d);
                    let pick: Pick = Box::new(|x, y, t| ms.hit_test(x, y, t).map(|h| (format!("{h:?}"), matches!(h, mindmap::MindHit::Node(_)))));
                    // centres
                    let mut bad = 0;
                    for (i, n) in ms.nodes.iter().enumerate() {
                        if ms.hit_test(n.cx(), n.cy(), 0.0) != Some(mindmap::MindHit::Node(i)) { bad += 1; }
                    }
                    let ends = mindmap::link_ends(&d);
                    let mut lbad = 0;
                    let mut lcount = 0;
                    for (j, e) in ms.edges.iter().enumerate() {
                        let m = ((e.bezier[0].0 + e.bezier[3].0) / 2.0, (e.bezier[0].1 + e.bezier[3].1) / 2.0);
                        match ms.hit_test(m.0, m.1, 0.0) {
                            Some(mindmap::MindHit::Link(k)) if k == j => {}
                            Some(mindmap::MindHit::Node(_)) => {} // covered by a node box
                            other => { lbad += 1; if lbad < 3 { out.push(format!("!MM_LINK_MID {path} link {j} -> {other:?}")); } }
                        }
                        lcount += 1;
                        let (p, c) = ends[j];
                        if e.bezier[0] != (ms.nodes[p].cx(), ms.nodes[p].cy()) || e.bezier[3] != (ms.nodes[c].cx(), ms.nodes[c].cy()) { out.push(format!("!LINK_ENDS {path} {j}")); }
                    }
                    out.push(format!("CENTRES\tmindmap\t{path}\tnodes_bad={bad}/{}\tlinks_bad={lbad}/{lcount}", ms.nodes.len()));
                    grid_run(&path, "mindmap", ms.nodes.len() + ms.edges.len(), ms.width, ms.height, &pick, grid, &mut out);
                }
                Document::Sequence(d) => {
                    let sc = seq::scene(&d);
                    use seq::SeqHit as H;
                    let pick: Pick = Box::new(|x, y, t| sc.hit_test(x, y, t).map(|h| (format!("{h:?}"), matches!(h, H::Note(_) | H::Activation(_) | H::FrameInside(_)))));
                    let mut bad = Vec::new();
                    for (k, b) in sc.boxes.iter().enumerate() {
                        let r = sc.hit_test(b.x + b.w / 2.0, b.y + b.h / 2.0, 0.0);
                        if r != Some(H::Participant(k)) && !b.actor { bad.push(format!("box{k}->{r:?}")); }
                    }
                    for (k, b) in sc.footer.iter().enumerate() {
                        let r = sc.hit_test(b.x + b.w / 2.0, b.y + b.h / 2.0, 0.0);
                        if r != Some(H::Footer(k)) && !b.actor { bad.push(format!("foot{k}->{r:?}")); }
                    }
                    for (k, n) in sc.notes.iter().enumerate() {
                        let r = sc.hit_test(n.x + n.w / 2.0, n.y + n.h / 2.0, 0.0);
                        if r != Some(H::Note(k)) { bad.push(format!("note{k}->{r:?}")); }
                    }
                    for (k, a) in sc.activations.iter().enumerate() {
                        let r = sc.hit_test(a.x, (a.y0 + a.y1) / 2.0, 0.0);
                        if r != Some(H::Activation(k)) { bad.push(format!("act{k}->{r:?}")); }
                    }
                    for (k, m) in sc.messages.iter().enumerate() {
                        if m.points.len() == 2 && m.points[0].1 == m.points[1].1 {
                            let p = ((m.points[0].0 * 0.3 + m.points[1].0 * 0.7), m.points[0].1);
                            let r = sc.hit_test(p.0, p.1, 0.0);
                            if r != Some(H::Message(k)) { bad.push(format!("msg{k}->{r:?}")); }
                        }
                    }
                    for (k, l) in sc.lifelines.iter().enumerate() {
                        // lifeline: off-centre by 0.4 inside the 1px stroke at tol 0; by 3.0 within tol 3
                        let y = l.y0 + (l.y1 - l.y0) * 0.13;
                        for (dx, tol, want) in [(0.4, 0.0, true), (0.5 + 2.9, 3.0, true), (0.5 + 3.2, 3.0, false)] {
                            let r = sc.hit_test(l.x + dx, y, tol);
                            let is = r == Some(H::Lifeline(k));
                            if is != want && matches!(r, None | Some(H::Lifeline(_)) | Some(H::FrameInside(_))) { bad.push(format!("life{k} dx={dx} tol={tol}->{r:?}")); }
                        }
                    }
                    out.push(format!("CENTRES\tseq\t{path}\tbad={}\t{}", bad.len(), bad.iter().take(4).cloned().collect::<Vec<_>>().join(" ")));
                    let size = sc.boxes.len() + sc.messages.len() + sc.notes.len() + sc.frames.len() + sc.activations.len();
                    grid_run(&path, "seq", size, sc.width, sc.height, &pick, grid, &mut out);
                }
                Document::Pie(d) => {
                    let ps = pie::scene(&d);
                    use pie::PieHit as H;
                    let pick: Pick = Box::new(|x, y, t| ps.hit_test(x, y, t).map(|h| (format!("{h:?}"), matches!(h, H::Slice(_) | H::Legend(_)))));
                    let mut bad = Vec::new();
                    for (i, sl) in ps.slices.iter().enumerate() {
                        if sl.frac <= 0.0 { continue; }
                        let a = (sl.start_angle + sl.end_angle) / 2.0;
                        let p = (ps.cx + 0.85 * ps.r * a.sin(), ps.cy - 0.85 * ps.r * a.cos());
                        match ps.hit_test(p.0, p.1, 0.0) {
                            Some(H::Slice(k)) if k == i => {}
                            Some(H::Label { slice, .. }) if slice == i => {}
                            r => bad.push(format!("slice{i}->{r:?}")),
                        }
                    }
                    for (i, row) in ps.legend.iter().enumerate() {
                        let r = ps.hit_test(row.x + 6.0, row.y, 0.0);
                        if r != Some(H::Legend(i)) { bad.push(format!("legend{i}->{r:?}")); }
                    }
                    out.push(format!("CENTRES\tpie\t{path}\tbad={}\t{}", bad.len(), bad.iter().take(4).cloned().collect::<Vec<_>>().join(" ")));
                    grid_run(&path, "pie", ps.slices.len(), ps.width, ps.height, &pick, grid, &mut out);
                }
                Document::Journey(d) => {
                    let js = journey::scene(&d);
                    use journey::JourneyHit as H;
                    let pick: Pick = Box::new(|x, y, t| js.hit_test(x, y, t).map(|h| (format!("{h:?}"), true)));
                    let mut bad = Vec::new();
                    for (i, t) in js.tasks.iter().enumerate() {
                        let r = js.hit_test(t.cx, t.cy, 0.0);
                        if r != Some(H::Task(i)) { bad.push(format!("task{i}->{r:?}")); }
                    }
                    for (i, b) in js.sections.iter().enumerate() {
                        let r = js.hit_test(b.x + 3.0, b.y + b.h / 2.0, 0.0);
                        if r != Some(H::Section(i)) { bad.push(format!("sec{i}->{r:?}")); }
                    }
                    let mut tdiff = 0;
                    for a in 0..80 { for b in 0..80 {
                        let (x, y) = (js.width * (a as f64 + 0.5) / 80.0, js.height * (b as f64 + 0.5) / 80.0);
                        if js.hit_test(x, y, 0.0) != js.hit_test(x, y, 25.0) { tdiff += 1; }
                    }}
                    if tdiff > 0 { out.push(format!("!JOURNEY_TOL {path} {tdiff}")); }
                    out.push(format!("CENTRES\tjourney\t{path}\tbad={}\t{}", bad.len(), bad.iter().take(4).cloned().collect::<Vec<_>>().join(" ")));
                    grid_run(&path, "journey", js.tasks.len() + js.sections.len(), js.width, js.height, &pick, grid, &mut out);
                }
                Document::Timeline(d) => {
                    let ts = timeline::scene(&d);
                    use timeline::TimelineHit as H;
                    let pick: Pick = Box::new(|x, y, t| ts.hit_test(x, y, t).map(|h| (format!("{h:?}"), matches!(h, H::Event { .. }))));
                    let mut bad = Vec::new();
                    let mut n = 0;
                    for (s, sec) in ts.sections.iter().enumerate() {
                        for (k, p) in sec.periods.iter().enumerate() {
                            n += 1;
                            for (e, (_, ey)) in p.events.iter().enumerate() {
                                let r = ts.hit_test(p.cx, *ey, 0.0);
                                if r != Some(H::Event { section: s, period: k, event: e }) { bad.push(format!("ev{s}.{k}.{e}->{r:?}")); }
                            }
                            let r = ts.hit_test(p.cx, p.label_y, 0.0);
                            if r != Some(H::Period { section: s, period: k }) { bad.push(format!("per{s}.{k}->{r:?}")); }
                        }
                    }
                    out.push(format!("CENTRES\ttimeline\t{path}\tbad={}\t{}", bad.len(), bad.iter().take(4).cloned().collect::<Vec<_>>().join(" ")));
                    grid_run(&path, "timeline", n, ts.width, ts.height, &pick, grid, &mut out);
                }
                Document::Sankey(d) => {
                    let ss = sankey::scene(&d);
                    use sankey::SankeyHit as H;
                    let pick: Pick = Box::new(|x, y, t| ss.hit_test(x, y, t).map(|h| (format!("{h:?}"), true)));
                    let mut bad = Vec::new();
                    for (i, nd) in ss.nodes.iter().enumerate() {
                        let r = ss.hit_test(nd.x + nd.w / 2.0, nd.y + nd.h / 2.0, 0.0);
                        if r != Some(H::Node(i)) { bad.push(format!("node{i}->{r:?}")); }
                    }
                    let mut lnk_other = 0;
                    for (i, l) in ss.links.iter().enumerate() {
                        let r = ss.hit_test((l.x0 + l.x1) / 2.0, (l.y0 + l.y1) / 2.0, 0.0);
                        match r {
                            Some(H::Link(k)) if k == i => {}
                            Some(H::Link(_)) => lnk_other += 1, // a later ribbon over it
                            r => bad.push(format!("link{i}->{r:?}")),
                        }
                    }
                    let mut tdiff = 0;
                    for a in 0..80 { for b in 0..80 {
                        let (x, y) = (ss.width * (a as f64 + 0.5) / 80.0, ss.height * (b as f64 + 0.5) / 80.0);
                        if ss.hit_test(x, y, 0.0) != ss.hit_test(x, y, 25.0) { tdiff += 1; }
                    }}
                    if tdiff > 0 { out.push(format!("!SANKEY_TOL {path} {tdiff}")); }
                    out.push(format!("CENTRES\tsankey\t{path}\tbad={}\tlinks_under_later={lnk_other}\t{}", bad.len(), bad.iter().take(4).cloned().collect::<Vec<_>>().join(" ")));
                    grid_run(&path, "sankey", ss.nodes.len() + ss.links.len(), ss.width, ss.height, &pick, grid, &mut out);
                }
                _ => {}
            }
        }));
        if r.is_err() { out.push(format!("!PANIC {path}")); }
        for o in out { println!("{o}"); }
    }
}
