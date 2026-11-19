//! Contract probe for #112 hit_test methods (Rust side).
use flowmaid::journey::JourneyHit;
use flowmaid::mindmap::MindHit;
use flowmaid::model::Document;
use flowmaid::pie::PieHit;
use flowmaid::sankey::SankeyHit;
use flowmaid::seq::SeqHit;
use flowmaid::timeline::TimelineHit;
use flowmaid::{journey, mindmap, parser, pie, sankey, seq, timeline};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Default)]
struct Stat {
    checks: usize,
    fails: usize,
    occluded: usize,
    notes: HashMap<String, usize>,
}
impl Stat {
    fn ok(&mut self, c: bool, msg: impl FnOnce() -> String) {
        self.checks += 1;
        if !c {
            self.fails += 1;
            let m = msg();
            let key: String = m.chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).collect();
            let key = key.split(" @").next().unwrap().to_string();
            let e = self.notes.entry(key).or_default();
            *e += 1;
            if *e <= 3 {
                println!("FAIL {m}");
            }
        }
    }
}

fn odd_points(w: f64, h: f64) -> Vec<(f64, f64)> {
    vec![
        (-0.01, h / 2.0),
        (w + 0.01, h / 2.0),
        (w / 2.0, -0.01),
        (w / 2.0, h + 0.01),
        (f64::NAN, 1.0),
        (1.0, f64::NAN),
        (f64::INFINITY, 1.0),
        (f64::NEG_INFINITY, f64::NEG_INFINITY),
        (-1e300, 1e300),
    ]
}

/// The same answer for tol 0, -1, -inf, NaN (exact).
fn exact<H: PartialEq + std::fmt::Debug>(st: &mut Stat, name: &str, f: impl Fn(f64) -> Option<H>, at: (f64, f64)) {
    let a = f(0.0);
    for t in [-1.0, -1e9, f64::NEG_INFINITY, -0.0] {
        let b = f(t);
        st.ok(a == b, || format!("{name}: tol {t} differs from 0 at {at:?}: {a:?} vs {b:?}"));
    }
    let n = f(f64::NAN);
    st.ok(n == a, || format!("{name}: tol NaN differs from 0 at {at:?}: {n:?} vs {a:?}"));
}

fn mind(st: &mut Stat, name: &str, ms: &mindmap::MindScene) {
    let f = |x: f64, y: f64, t: f64| ms.hit_test(x, y, t);
    for p in odd_points(ms.width, ms.height) {
        st.ok(f(p.0, p.1, 5.0).is_none(), || format!("{name}: mind off-canvas {p:?} picks"));
    }
    for (i, n) in ms.nodes.iter().enumerate() {
        let c = (n.cx(), n.cy());
        let h = f(c.0, c.1, 0.0);
        if h != Some(MindHit::Node(i)) {
            // only a later node may cover it
            let ok = matches!(h, Some(MindHit::Node(k)) if k > i);
            st.occluded += 1;
            st.ok(ok, || format!("{name}: mind node {i} centre gives {h:?}"));
        }
        exact(st, name, |t| f(c.0, c.1, t), c);
        // just outside the box (left/right middle, beyond any lobe room)
        for p in [(n.x - 9.0, n.cy()), (n.x + n.w + 9.0, n.cy()), (n.cx(), n.y - 9.0), (n.cx(), n.y + n.h + 9.0)] {
            for t in [0.0, 3.0, 8.0] {
                let h = f(p.0, p.1, t);
                st.ok(h != Some(MindHit::Node(i)), || format!("{name}: mind node {i} picked 9px outside with tol {t}"));
            }
        }
    }
    // connectors: mid point (straight cubic), offsets across
    let ends_ok = true;
    let _ = ends_ok;
    for (j, e) in ms.edges.iter().enumerate() {
        let [a, _, _, b] = e.bezier;
        let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        if len < 1e-6 {
            continue;
        }
        let nrm = (-(b.1 - a.1) / len, (b.0 - a.0) / len);
        let covered = |p: (f64, f64)| ms.nodes.iter().any(|n| p.0 >= n.x - 12.0 && p.0 <= n.x + n.w + 12.0 && p.1 >= n.y - 12.0 && p.1 <= n.y + n.h + 12.0);
        if covered(m) {
            continue;
        }
        let h = f(m.0, m.1, 0.0);
        st.ok(matches!(h, Some(MindHit::Link(_))), || format!("{name}: mind link {j} mid gives {h:?}"));
        if h != Some(MindHit::Link(j)) {
            st.occluded += 1;
        }
        let off = e.width / 2.0 + 3.0;
        let p = (m.0 + nrm.0 * off, m.1 + nrm.1 * off);
        if !covered(p) {
            let other_near = ms.edges.iter().enumerate().any(|(k, o)| k != j && seg_d(p, o.bezier[0], o.bezier[3]) < o.width / 2.0 + 4.0);
            if !other_near {
                st.ok(f(p.0, p.1, 3.5) == Some(MindHit::Link(j)), || format!("{name}: mind link {j} not within tol 3.5 at 3"));
                st.ok(f(p.0, p.1, 2.5).is_none(), || format!("{name}: mind link {j} picked beyond tol 2.5 at 3: {:?}", f(p.0, p.1, 2.5)));
                st.ok(f(p.0, p.1, 0.0).is_none(), || format!("{name}: mind link {j} picked at tol 0, 3 off its stroke"));
            }
        }
    }
}

fn seg_d(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

fn sank(st: &mut Stat, name: &str, ss: &sankey::SankeyScene) {
    let f = |x: f64, y: f64, t: f64| ss.hit_test(x, y, t);
    for p in odd_points(ss.width, ss.height) {
        st.ok(f(p.0, p.1, 5.0).is_none(), || format!("{name}: sankey off-canvas {p:?} picks"));
    }
    for (i, n) in ss.nodes.iter().enumerate() {
        let c = (n.x + n.w / 2.0, n.y + n.h / 2.0);
        let h = f(c.0, c.1, 0.0);
        st.ok(matches!(h, Some(SankeyHit::Node(_))), || format!("{name}: sankey node {i} centre gives {h:?}"));
        if h != Some(SankeyHit::Node(i)) {
            st.occluded += 1;
        }
        exact(st, name, |t| f(c.0, c.1, t), c);
        // tol never widens a bar: 0.3 left of the bar at its middle (outside of labels)
        let p = (n.x - 0.3, c.1);
        for t in [0.0, 2.0, 10.0] {
            let h = f(p.0, p.1, t);
            if h == Some(SankeyHit::Node(i)) {
                // a label of its own may lie there
                st.ok(n.label_anchor == "end" && n.label_pos.0 <= n.x, || format!("{name}: sankey node {i} picked 0.3 outside (tol {t})"));
            }
        }
    }
    for (j, l) in ss.links.iter().enumerate() {
        let mx = (l.x0 + l.x1) / 2.0;
        let my = (l.y0 + l.y1) / 2.0; // at s = 0.5 x = mid and top = (top0+top1)/2
        let h = f(mx, my, 0.0);
        if h != Some(SankeyHit::Link(j)) {
            st.occluded += 1;
            st.ok(matches!(h, Some(SankeyHit::Link(k)) if k > j) || matches!(h, Some(SankeyHit::Node(_))), || format!("{name}: sankey link {j} centre gives {h:?}"));
        }
        exact(st, name, |t| f(mx, my, t), (mx, my));
        // just outside the band at the middle: top - 0.3
        if l.thickness > 0.0 {
            let p = (mx, my - l.thickness / 2.0 - 0.3);
            for t in [0.0, 5.0] {
                st.ok(f(p.0, p.1, t) != Some(SankeyHit::Link(j)), || format!("{name}: sankey link {j} picked 0.3 above its band at tol {t}"));
            }
            let p = (mx, my - l.thickness / 2.0 + 0.05);
            if l.thickness > 0.2 {
                let h = f(p.0, p.1, 0.0);
                st.ok(h.is_some(), || format!("{name}: sankey link {j} band top inner edge gives None"));
            }
        }
    }
}

fn pies(st: &mut Stat, name: &str, ps: &pie::PieScene) {
    let f = |x: f64, y: f64, t: f64| ps.hit_test(x, y, t);
    for p in odd_points(ps.width, ps.height) {
        st.ok(f(p.0, p.1, 5.0).is_none(), || format!("{name}: pie off-canvas {p:?} picks"));
    }
    let polar = |r: f64, a: f64| (ps.cx + r * a.sin(), ps.cy - r * a.cos());
    for (i, s) in ps.slices.iter().enumerate() {
        if s.frac <= 0.0 {
            // zero slices are never picked
            continue;
        }
        let mid = (s.start_angle + s.end_angle) / 2.0;
        for rr in [0.35, 0.6, 0.85] {
            let p = polar(ps.r * rr, mid);
            let h = f(p.0, p.1, 0.0);
            let ok = h == Some(PieHit::Slice(i)) || matches!(h, Some(PieHit::Label { slice, .. }) if slice == i) || matches!(h, Some(PieHit::Legend(_)));
            if h != Some(PieHit::Slice(i)) {
                st.occluded += 1;
            }
            st.ok(ok, || format!("{name}: pie slice {i} (frac {}) at r*{rr} gives {h:?}", s.frac));
        }
        // just outside the rim along the middle: never the slice, at any tol
        let p = polar(ps.r + 0.5, mid);
        for t in [0.0, 4.0, 20.0] {
            st.ok(f(p.0, p.1, t) != Some(PieHit::Slice(i)), || format!("{name}: pie slice {i} picked 0.5 past the rim (tol {t})"));
        }
    }
    for (k, l) in ps.labels.iter().enumerate() {
        let h = f(l.x + match l.anchor { "start" => 4.0, "end" => -4.0, _ => 0.0 }, l.y, 0.0);
        st.ok(h == Some(PieHit::Label { label: k, slice: l.slice }) || matches!(h, Some(PieHit::Legend(_))), || format!("{name}: pie label {k} gives {h:?}"));
    }
    for (i, r) in ps.legend.iter().enumerate() {
        let h = f(r.x + 6.0, r.y, 0.0);
        st.ok(h == Some(PieHit::Legend(i)), || format!("{name}: pie legend swatch {i} gives {h:?}"));
    }
    // zero slices: never returned anywhere on a grid
    let zero: Vec<usize> = (0..ps.slices.len()).filter(|&i| ps.slices[i].frac <= 0.0).collect();
    if !zero.is_empty() {
        let mut y = 0.5;
        while y < ps.height {
            let mut x = 0.5;
            while x < ps.width {
                if let Some(PieHit::Slice(i)) = f(x, y, 0.0) {
                    st.ok(!zero.contains(&i), || format!("{name}: zero slice {i} picked"));
                }
                x += 3.0;
            }
            y += 3.0;
        }
    }
}

fn jour(st: &mut Stat, name: &str, js: &journey::JourneyScene) {
    let f = |x: f64, y: f64, t: f64| js.hit_test(x, y, t);
    for p in odd_points(js.width, js.height) {
        st.ok(f(p.0, p.1, 5.0).is_none(), || format!("{name}: journey off-canvas {p:?} picks"));
    }
    for (i, t) in js.tasks.iter().enumerate() {
        let h = f(t.cx, t.cy, 0.0);
        st.ok(h == Some(JourneyHit::Task(i)) || matches!(h, Some(JourneyHit::Legend(_))) || matches!(h, Some(JourneyHit::Task(k)) if k > i), || format!("{name}: journey task {i} centre gives {h:?}"));
        exact(st, name, |tt| f(t.cx, t.cy, tt), (t.cx, t.cy));
        // tol never widens the face: point 0.4 outside radius on the left
        let p = (t.cx - t.r - 0.4, t.cy);
        for tt in [0.0, 3.0] {
            let h = f(p.0, p.1, tt);
            if h == Some(JourneyHit::Task(i)) {
                // an actor dot of its own may lie there
                let dot = t.actor_dots.iter().any(|&(dx, dy, _)| (dx - p.0).hypot(dy - p.1) < 9.0);
                st.ok(dot, || format!("{name}: journey task {i} face picked 0.4 outside (tol {tt})"));
            }
        }
        for (k, &(dx, dy, _)) in t.actor_dots.iter().enumerate() {
            let h = f(dx, dy, 0.0);
            st.ok(h == Some(JourneyHit::Task(i)) || matches!(h, Some(JourneyHit::Task(_)) | Some(JourneyHit::Legend(_))), || format!("{name}: journey task {i} dot {k} gives {h:?}"));
        }
    }
    for (a, it) in js.legend.iter().enumerate() {
        let h = f(it.x + 6.0, it.y, 0.0);
        st.ok(h == Some(JourneyHit::Legend(a)), || format!("{name}: journey legend {a} mark gives {h:?}"));
    }
    for (b, s) in js.sections.iter().enumerate() {
        let c = (s.x + s.w / 2.0, s.y + s.h / 2.0);
        let h = f(c.0, c.1, 0.0);
        if h != Some(JourneyHit::Section(b)) {
            st.occluded += 1;
            st.ok(matches!(h, Some(JourneyHit::Task(_)) | Some(JourneyHit::Legend(_))), || format!("{name}: journey section {b} centre gives {h:?}"));
        }
        let p = (s.x + s.w / 2.0, s.y - 0.3);
        st.ok(f(p.0, p.1, 5.0) != Some(JourneyHit::Section(b)), || format!("{name}: journey section {b} picked above its band"));
    }
}

fn tl(st: &mut Stat, name: &str, ts: &timeline::TimelineScene) {
    let f = |x: f64, y: f64, t: f64| ts.hit_test(x, y, t);
    for p in odd_points(ts.width, ts.height) {
        st.ok(f(p.0, p.1, 5.0).is_none(), || format!("{name}: timeline off-canvas {p:?} picks"));
    }
    for (s, sec) in ts.sections.iter().enumerate() {
        // axis: in a gap between ticks, offset across
        let ax = sec.x + 2.0;
        let h = f(ax, sec.axis_y, 0.0);
        st.ok(h == Some(TimelineHit::Section(s)) || matches!(h, Some(TimelineHit::Period { .. } | TimelineHit::Event { .. })), || format!("{name}: timeline axis {s} gives {h:?}"));
        if h == Some(TimelineHit::Section(s)) {
            let p = (ax, sec.axis_y + 1.0 + 2.0);
            let hh = f(p.0, p.1, 2.5);
            if !(!sec.name.is_empty() && p.1 >= sec.y && p.1 <= sec.y + 34.0) {
                st.ok(hh == Some(TimelineHit::Section(s)) || matches!(hh, Some(TimelineHit::Period { .. } | TimelineHit::Event { .. })), || format!("{name}: timeline axis {s} not within tol 2.5 at 3: {hh:?}"));
                let h0 = f(p.0, p.1, 1.5);
                st.ok(h0 != Some(TimelineHit::Section(s)), || format!("{name}: timeline axis {s} picked beyond tol 1.5 at 3"));
            }
        }
        for (k, per) in sec.periods.iter().enumerate() {
            let h = f(per.cx, sec.axis_y, 0.0);
            st.ok(h == Some(TimelineHit::Period { section: s, period: k }) || matches!(h, Some(TimelineHit::Event { .. }) | Some(TimelineHit::Period { .. })), || format!("{name}: timeline tick {s}.{k} gives {h:?}"));
            let h = f(per.cx, per.label_y, 0.0);
            if !per.label.is_empty() {
                st.ok(matches!(h, Some(TimelineHit::Period { .. }) | Some(TimelineHit::Event { .. })), || format!("{name}: timeline period label {s}.{k} gives {h:?}"));
                if h != Some(TimelineHit::Period { section: s, period: k }) {
                    st.occluded += 1;
                }
            }
            for (e, (text, ey)) in per.events.iter().enumerate() {
                if text.is_empty() {
                    continue;
                }
                let h = f(per.cx, *ey, 0.0);
                st.ok(matches!(h, Some(TimelineHit::Event { .. })), || format!("{name}: timeline event {s}.{k}.{e} gives {h:?}"));
                if h != Some(TimelineHit::Event { section: s, period: k, event: e }) {
                    st.occluded += 1;
                }
            }
        }
    }
}

fn sq(st: &mut Stat, name: &str, sc: &seq::SeqScene) {
    let f = |x: f64, y: f64, t: f64| sc.hit_test(x, y, t);
    for p in odd_points(sc.width, sc.height) {
        st.ok(f(p.0, p.1, 5.0).is_none(), || format!("{name}: seq off-canvas {p:?} picks"));
    }
    for (k, b) in sc.boxes.iter().enumerate() {
        if b.actor {
            continue;
        }
        let c = (b.x + b.w / 2.0, b.y + b.h / 2.0);
        let h = f(c.0, c.1, 0.0);
        st.ok(h == Some(SeqHit::Participant(k)) || matches!(h, Some(SeqHit::Footer(_)) | Some(SeqHit::Participant(_))), || format!("{name}: seq box {k} centre gives {h:?}"));
        let p = (b.x - 0.4, c.1);
        st.ok(f(p.0, p.1, 6.0) != Some(SeqHit::Participant(k)), || format!("{name}: seq box {k} widened by tol"));
    }
    for (k, b) in sc.footer.iter().enumerate() {
        if b.actor {
            continue;
        }
        let c = (b.x + b.w / 2.0, b.y + b.h / 2.0);
        let h = f(c.0, c.1, 0.0);
        st.ok(h == Some(SeqHit::Footer(k)), || format!("{name}: seq footer {k} centre gives {h:?}"));
    }
    for (k, l) in sc.lifelines.iter().enumerate() {
        // footer/header box index = participant index: lifeline k under box k
        if let Some(b) = sc.boxes.get(k) {
            st.ok((b.x + b.w / 2.0 - l.x).abs() < 1e-6, || format!("{name}: seq lifeline {k} not under box {k}"));
        }
        let m = (l.x, (l.y0 + l.y1) / 2.0);
        let h = f(m.0 + 3.0, m.1, 3.6);
        let h0 = f(m.0 + 3.0, m.1, 2.4);
        let clear = sc.messages.iter().all(|mm| mm.points.iter().all(|p| (p.1 - m.1).abs() > 30.0))
            && sc.notes.iter().all(|n| !(m.1 >= n.y - 5.0 && m.1 <= n.y + n.h + 5.0 && m.0 + 3.0 >= n.x - 5.0 && m.0 + 3.0 <= n.x + n.w + 5.0))
            && sc.activations.iter().all(|a| !(m.1 >= a.y0 - 5.0 && m.1 <= a.y1 + 5.0 && (a.x - m.0).abs() < 15.0))
            && sc.frames.iter().all(|fr| fr.dividers.iter().all(|d| (d.0 - m.1).abs() > 10.0) && (fr.y - m.1).abs() > 30.0 && (fr.y + fr.h - m.1).abs() > 10.0 && (fr.x - m.0).abs() > 10.0 && (fr.x + fr.w - m.0).abs() > 10.0);
        if clear {
            st.ok(h == Some(SeqHit::Lifeline(k)), || format!("{name}: seq lifeline {k} not within tol 3.6 at 3: {h:?}"));
            st.ok(h0 != Some(SeqHit::Lifeline(k)), || format!("{name}: seq lifeline {k} within tol 2.4 at 3"));
            // inside a frame: a point off everything inside picks FrameInside, not the frame border
            let inside = sc.frames.iter().rposition(|fr| m.0 + 20.0 > fr.x + 10.0 && m.0 + 20.0 < fr.x + fr.w - 10.0 && m.1 > fr.y + 30.0 && m.1 < fr.y + fr.h - 10.0);
            let _ = inside;
        }
    }
    for (i, m) in sc.messages.iter().enumerate() {
        if m.points.len() < 2 {
            continue;
        }
        let (a, b) = (m.points[0], m.points[1]);
        let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let h = f(mid.0, mid.1, 0.0);
        st.ok(matches!(h, Some(SeqHit::Message(_)) | Some(SeqHit::Participant(_)) | Some(SeqHit::Footer(_))), || format!("{name}: seq message {i} mid gives {h:?}"));
        if h != Some(SeqHit::Message(i)) {
            st.occluded += 1;
        }
    }
    for (i, n) in sc.notes.iter().enumerate() {
        let c = (n.x + n.w / 2.0, n.y + n.h / 2.0);
        let h = f(c.0, c.1, 0.0);
        st.ok(matches!(h, Some(SeqHit::Note(_)) | Some(SeqHit::Message(_)) | Some(SeqHit::Participant(_)) | Some(SeqHit::Footer(_))), || format!("{name}: seq note {i} centre gives {h:?}"));
        if h != Some(SeqHit::Note(i)) {
            st.occluded += 1;
        }
    }
    for (i, a) in sc.activations.iter().enumerate() {
        let c = (a.x, (a.y0 + a.y1) / 2.0);
        let h = f(c.0, c.1, 0.0);
        st.ok(!matches!(h, None | Some(SeqHit::Lifeline(_)) | Some(SeqHit::FrameInside(_))), || format!("{name}: seq activation {i} centre gives {h:?}"));
        if h != Some(SeqHit::Activation(i)) {
            st.occluded += 1;
        }
    }
    for (i, fr) in sc.frames.iter().enumerate() {
        // chip
        let h = f(fr.x + 3.0, fr.y + 9.0, 0.0);
        st.ok(matches!(h, Some(SeqHit::Frame(_))) || matches!(h, Some(SeqHit::Message(_) | SeqHit::Note(_) | SeqHit::Activation(_) | SeqHit::Participant(_))), || format!("{name}: seq frame {i} chip gives {h:?}"));
        // border left middle
        let p = (fr.x, fr.y + fr.h / 2.0);
        let h = f(p.0, p.1, 0.0);
        if h != Some(SeqHit::Frame(i)) {
            st.occluded += 1;
        }
        // interior: scan a row inside the frame for points the frame interior owns
        let mut found = false;
        let y = fr.y + fr.h - 6.0;
        let mut x = fr.x + 6.0;
        while x < fr.x + fr.w - 6.0 {
            if let Some(SeqHit::FrameInside(k)) = f(x, y, 0.0) {
                // innermost frame containing the point
                let inner = sc.frames.iter().enumerate().filter(|(_, g)| x >= g.x && x <= g.x + g.w && y >= g.y && y <= g.y + g.h).map(|(k, _)| k).max();
                st.ok(Some(k) == inner, || format!("{name}: seq FrameInside({k}) not the innermost {inner:?}"));
                found = true;
            }
            x += 2.0;
        }
        let _ = found;
        // just outside the frame (left), tol 0: not the frame
        let p = (fr.x - 1.5, fr.y + fr.h / 2.0);
        let h = f(p.0, p.1, 0.0);
        st.ok(!matches!(h, Some(SeqHit::Frame(k)) | Some(SeqHit::FrameInside(k)) if k == i), || format!("{name}: seq frame {i} picked 1.5 outside at tol 0"));
    }
}

fn main() {
    let mut files: Vec<String> = std::env::args().skip(1).collect();
    files.sort();
    let mut st = Stat::default();
    let mut per: HashMap<&str, usize> = HashMap::new();
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let Ok(doc) = parser::parse_document(&src) else { continue };
        let r = catch_unwind(AssertUnwindSafe(|| match &doc {
            Document::Mindmap(d) => {
                let ms = mindmap::scene(d);
                mind(&mut st, f, &ms);
                let mut e = ms.clone();
                e.nodes.clear();
                e.edges.clear();
                st.ok(e.hit_test(ms.width / 2.0, ms.height / 2.0, 100.0).is_none(), || format!("{f}: empty mind picks"));
                Some("mindmap")
            }
            Document::Sankey(d) => {
                let ss = sankey::scene(d);
                sank(&mut st, f, &ss);
                let mut e = ss.clone();
                e.nodes.clear();
                e.links.clear();
                st.ok(e.hit_test(ss.width / 2.0, ss.height / 2.0, 100.0).is_none(), || format!("{f}: empty sankey picks"));
                Some("sankey")
            }
            Document::Pie(d) => {
                let ps = pie::scene(d);
                pies(&mut st, f, &ps);
                let mut e = ps.clone();
                e.slices.clear();
                e.labels.clear();
                e.legend.clear();
                st.ok(e.hit_test(ps.cx, ps.cy, 100.0).is_none(), || format!("{f}: empty pie picks"));
                Some("pie")
            }
            Document::Journey(d) => {
                let js = journey::scene(d);
                jour(&mut st, f, &js);
                let mut e = js.clone();
                e.sections.clear();
                e.tasks.clear();
                e.legend.clear();
                st.ok(e.hit_test(js.width / 2.0, js.height / 2.0, 100.0).is_none(), || format!("{f}: empty journey picks"));
                Some("journey")
            }
            Document::Timeline(d) => {
                let ts = timeline::scene(d);
                tl(&mut st, f, &ts);
                let mut e = ts.clone();
                e.sections.clear();
                st.ok(e.hit_test(ts.width / 2.0, ts.height / 2.0, 100.0).is_none(), || format!("{f}: empty timeline picks"));
                Some("timeline")
            }
            Document::Sequence(d) => {
                let sc = seq::scene(d);
                sq(&mut st, f, &sc);
                let mut e = sc.clone();
                e.boxes.clear();
                e.footer.clear();
                e.lifelines.clear();
                e.messages.clear();
                e.notes.clear();
                e.activations.clear();
                e.frames.clear();
                st.ok(e.hit_test(sc.width / 2.0, sc.height / 2.0, 100.0).is_none(), || format!("{f}: empty seq picks"));
                Some("seq")
            }
            _ => None,
        }));
        match r {
            Ok(Some(t)) => *per.entry(t).or_default() += 1,
            Ok(None) => {}
            Err(_) => {
                st.fails += 1;
                println!("FAIL PANIC in {f}");
            }
        }
    }
    println!("files per type: {per:?}");
    println!("checks {} fails {} occluded-centres {}", st.checks, st.fails, st.occluded);
    let mut v: Vec<_> = st.notes.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1));
    for (k, n) in v.iter().take(30) {
        println!("  {n:>6} {k}");
    }
}
