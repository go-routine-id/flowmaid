// Independent probe of the #112 contract (hit_test on six scene types).
// usage: pick <listfile>
use flowmaid::journey::JourneyHit;
use flowmaid::mindmap::MindHit;
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::pie::PieHit;
use flowmaid::sankey::SankeyHit;
use flowmaid::seq::SeqHit;
use flowmaid::timeline::TimelineHit;
use flowmaid::{journey, mindmap, pie, sankey, seq, timeline};
use std::collections::BTreeMap;

#[derive(Default)]
struct St {
    per: BTreeMap<String, (usize, usize)>,
    fails: Vec<String>,
    notes: BTreeMap<String, usize>,
}
impl St {
    fn check(&mut self, ok: bool, tag: &str, ctx: String) {
        let e = self.per.entry(tag.into()).or_default();
        e.0 += 1;
        if !ok {
            e.1 += 1;
            if self.fails.iter().filter(|f| f.contains(&format!("[{tag}]"))).count() < 12 {
                self.fails.push(format!("FAIL [{tag}] {ctx}"));
            }
        }
    }
    fn note(&mut self, k: &str) {
        *self.notes.entry(k.into()).or_default() += 1;
    }
}

fn grid(w: f64, h: f64, n: usize) -> Vec<(f64, f64)> {
    let mut v = Vec::new();
    for i in 0..n {
        for j in 0..n {
            // irrational-ish offsets so points do not sit on round coords
            v.push(((i as f64 + 0.37) * w / n as f64, (j as f64 + 0.61) * h / n as f64));
        }
    }
    v
}

fn off_canvas<T: PartialEq + std::fmt::Debug>(st: &mut St, name: &str, w: f64, h: f64, f: impl Fn(f64, f64, f64) -> Option<T>) {
    for (x, y) in [(-0.5, h / 2.0), (w + 0.5, h / 2.0), (w / 2.0, -0.5), (w / 2.0, h + 0.5), (f64::NAN, 1.0), (1.0, f64::NAN), (f64::INFINITY, 1.0)] {
        for tol in [0.0, 5.0, 100.0] {
            st.check(f(x, y, tol).is_none(), "all.off_canvas_none", format!("{name} ({x},{y}) tol {tol}"));
        }
    }
}

fn tol_invariants<T: PartialEq + std::fmt::Debug + Copy>(
    st: &mut St,
    name: &str,
    kind: &str,
    w: f64,
    h: f64,
    f: impl Fn(f64, f64, f64) -> Option<T>,
    filled: impl Fn(&T) -> bool,
    all_filled: bool,
) {
    for (x, y) in grid(w, h, 90) {
        let h0 = f(x, y, 0.0);
        st.check(f(x, y, -3.0) == h0, &format!("{kind}.neg_tol_is_exact"), format!("{name} ({x:.2},{y:.2})"));
        for tol in [2.5, 12.0] {
            let ht = f(x, y, tol);
            if let Some(t) = &ht {
                if filled(t) {
                    st.check(h0 == ht, &format!("{kind}.tol_never_grows_filled"), format!("{name} ({x:.2},{y:.2}) tol {tol}: {ht:?} vs exact {h0:?}"));
                }
            }
            if all_filled {
                st.check(h0 == ht, &format!("{kind}.tol_ignored_all_filled"), format!("{name} ({x:.2},{y:.2}) tol {tol}: {ht:?} vs {h0:?}"));
            }
            // something exact stays picked (maybe by a widened stroke above it)
            if h0.is_some() {
                st.check(ht.is_some(), &format!("{kind}.tol_keeps_a_hit"), format!("{name} ({x:.2},{y:.2})"));
            }
        }
    }
}

// --------------------------------------------------------------- mindmap
fn mind(st: &mut St, name: &str, d: &flowmaid::model::Mindmap) {
    let ms = mindmap::scene(d);
    let f = |x, y, t| ms.hit_test(x, y, t);
    off_canvas(st, name, ms.width, ms.height, f);
    tol_invariants(st, name, "mind", ms.width, ms.height, f, |h| matches!(h, MindHit::Node(_)), false);
    let inbox = |n: &mindmap::MindNodeBox, p: (f64, f64)| p.0 >= n.x - 8.0 && p.0 <= n.x + n.w + 8.0 && p.1 >= n.y - 8.0 && p.1 <= n.y + n.h + 8.0;
    for (i, n) in ms.nodes.iter().enumerate() {
        let c = (n.cx(), n.cy());
        match ms.hit_test(c.0, c.1, 0.0) {
            Some(MindHit::Node(j)) if j == i => st.check(true, "mind.node_centre", String::new()),
            Some(MindHit::Node(j)) if j > i && inbox(&ms.nodes[j], c) => st.note("mind.node_centre_occluded_by_later_node"),
            other => st.check(false, "mind.node_centre", format!("{name} node {i} {:?} got {other:?}", n.shape)),
        }
        // just outside plain shapes' left edge, at the middle
        let out = match n.shape {
            flowmaid::model::MindShape::Circle => Some((n.cx() - n.w.min(n.h) / 2.0 - 0.5, n.cy())),
            flowmaid::model::MindShape::Square | flowmaid::model::MindShape::Rounded | flowmaid::model::MindShape::Default => Some((n.x - 0.5, n.cy())),
            _ => None,
        };
        if let Some(p) = out {
            for tol in [0.0, 10.0, 40.0] {
                let hit = ms.hit_test(p.0, p.1, tol);
                st.check(hit != Some(MindHit::Node(i)), "mind.just_outside_node_not_it", format!("{name} node {i} {:?} tol {tol}", n.shape));
            }
        }
    }
    let ends = mindmap::link_ends(d);
    st.check(ends.len() == ms.edges.len(), "mind.link_ends_parallel", name.into());
    for (j, e) in ms.edges.iter().enumerate() {
        let (p, c) = ends[j];
        let (a, b) = (e.bezier[0], e.bezier[3]);
        st.check((a.0 - ms.nodes[p].cx()).abs() < 1e-9 && (b.1 - ms.nodes[c].cy()).abs() < 1e-9, "mind.link_ends_are_its_nodes", format!("{name} {j}"));
        // a point on the connector outside both end boxes
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        if len < 1.0 {
            continue;
        }
        let (ux, uy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        let (nx, ny) = (-uy, ux);
        for t in [0.5, 0.4, 0.6, 0.3, 0.7] {
            let m = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            if ms.nodes.iter().any(|n| inbox(n, m)) {
                continue;
            }
            let hit = ms.hit_test(m.0, m.1, 0.0);
            match hit {
                Some(MindHit::Link(k)) if k == j => st.check(true, "mind.link_on_ink", String::new()),
                Some(MindHit::Link(k)) => {
                    st.note("mind.link_point_on_other_link");
                    let _ = k;
                }
                other => st.check(false, "mind.link_on_ink", format!("{name} link {j} got {other:?}")),
            }
            let half = e.width / 2.0;
            for tol in [3.0, 8.0] {
                let q = (m.0 + nx * (half + 0.8 * tol), m.1 + ny * (half + 0.8 * tol));
                if ms.nodes.iter().any(|n| inbox(n, q)) || q.0 < 0.0 || q.1 < 0.0 {
                    continue;
                }
                let ht = ms.hit_test(q.0, q.1, tol);
                let h0 = ms.hit_test(q.0, q.1, 0.0);
                st.check(matches!(ht, Some(MindHit::Link(_))), "mind.tol_widens_link", format!("{name} {j} tol {tol} {ht:?}"));
                st.check(h0 != Some(MindHit::Link(j)), "mind.exact_not_beyond_stroke", format!("{name} {j}"));
                let far = (m.0 + nx * (half + tol + 0.5), m.1 + ny * (half + tol + 0.5));
                st.check(ms.hit_test(far.0, far.1, tol) != Some(MindHit::Link(j)), "mind.beyond_tol_not_link", format!("{name} {j} tol {tol}"));
            }
            break;
        }
    }
    // z-order: drag node 1 onto node 0's centre via route: the later node wins
    if ms.nodes.len() >= 2 {
        let mut at: Vec<(f64, f64)> = ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
        at[1] = at[0];
        let r = mindmap::route(d, &at);
        let c = (r.nodes[0].cx(), r.nodes[0].cy());
        st.check(r.hit_test(c.0, c.1, 0.0) == Some(MindHit::Node(1)), "mind.later_node_on_top", name.into());
    }
}

// --------------------------------------------------------------- sequence
fn seqk(st: &mut St, name: &str, d: &flowmaid::model::SequenceDiagram) {
    let sc = seq::scene(d);
    let f = |x, y, t| sc.hit_test(x, y, t);
    off_canvas(st, name, sc.width, sc.height, f);
    tol_invariants(
        st,
        name,
        "seq",
        sc.width,
        sc.height,
        f,
        |h| match h {
            SeqHit::Note(_) | SeqHit::Activation(_) | SeqHit::FrameInside(_) => true,
            SeqHit::Participant(k) => !sc.boxes[*k].actor,
            SeqHit::Footer(k) => !sc.footer[*k].actor,
            _ => false,
        },
        false,
    );
    st.check(sc.lifelines.len() == sc.boxes.len(), "seq.lifelines_parallel_boxes", name.into());
    for (k, l) in sc.lifelines.iter().enumerate() {
        if let Some(b) = sc.boxes.get(k) {
            st.check((l.x - (b.x + b.w / 2.0)).abs() < 1e-6, "seq.lifeline_k_under_box_k", format!("{name} {k}"));
        }
    }
    st.check(sc.footer.is_empty() || sc.footer.len() == sc.boxes.len(), "seq.footer_parallel_boxes", name.into());
    for (row, boxes) in [(false, &sc.boxes), (true, &sc.footer)] {
        for (k, b) in boxes.iter().enumerate() {
            let c = if b.actor { (b.x + b.w / 2.0, b.y + 7.0) } else { (b.x + b.w / 2.0, b.y + b.h / 2.0) };
            let want = if row { SeqHit::Footer(k) } else { SeqHit::Participant(k) };
            let hit = sc.hit_test(c.0, c.1, 0.0);
            st.check(hit == Some(want), "seq.box_centre", format!("{name} row {row} {k} actor {} got {hit:?}", b.actor));
            if !b.actor {
                for tol in [0.0, 10.0] {
                    let p = (b.x - 0.8 - 0.5, b.y + b.h / 2.0);
                    st.check(sc.hit_test(p.0, p.1, tol) != Some(want), "seq.just_outside_box_not_it", format!("{name} {k} tol {tol}"));
                    let p = (b.x - 0.8 + 0.3, b.y + b.h / 2.0); // on the outline stroke
                    st.check(sc.hit_test(p.0, p.1, 0.0) == Some(want), "seq.box_outline_stroke_is_it", format!("{name} {k}"));
                }
            }
        }
    }
    for (i, nb) in sc.notes.iter().enumerate() {
        let c = (nb.x + nb.w / 2.0, nb.y + nb.h / 2.0);
        match sc.hit_test(c.0, c.1, 0.0) {
            Some(SeqHit::Note(j)) if j == i => st.check(true, "seq.note_centre", String::new()),
            Some(SeqHit::Message(_)) | Some(SeqHit::Participant(_)) | Some(SeqHit::Footer(_)) => st.note("seq.note_centre_under_higher"),
            Some(SeqHit::Note(j)) if j > i => st.note("seq.note_centre_under_later_note"),
            other => st.check(false, "seq.note_centre", format!("{name} note {i} got {other:?}")),
        }
        for tol in [0.0, 10.0] {
            let p = (nb.x - 0.6 - 0.5, nb.y + nb.h / 2.0);
            st.check(sc.hit_test(p.0, p.1, tol) != Some(SeqHit::Note(i)), "seq.just_outside_note_not_it", format!("{name} {i} tol {tol}"));
        }
    }
    for (i, a) in sc.activations.iter().enumerate() {
        let c = (a.x, (a.y0 + a.y1) / 2.0);
        match sc.hit_test(c.0, c.1, 0.0) {
            Some(SeqHit::Activation(j)) if j == i => st.check(true, "seq.activation_centre", String::new()),
            Some(SeqHit::Activation(j)) if j > i => st.note("seq.activation_under_later_bar"),
            Some(SeqHit::Message(_)) | Some(SeqHit::Note(_)) | Some(SeqHit::Frame(_)) | Some(SeqHit::Participant(_)) | Some(SeqHit::Footer(_)) => st.note("seq.activation_centre_under_higher"),
            other => st.check(false, "seq.activation_centre", format!("{name} bar {i} got {other:?}")),
        }
    }
    for (i, m) in sc.messages.iter().enumerate() {
        if m.points.len() != 2 || (m.points[0].1 - m.points[1].1).abs() > 1e-9 {
            continue;
        }
        let (a, b) = (m.points[0], m.points[1]);
        let mid = ((a.0 + b.0) / 2.0 + 0.37, a.1);
        // the label sits above the line: test below it
        match sc.hit_test(mid.0, mid.1, 0.0) {
            Some(SeqHit::Message(j)) if j == i => st.check(true, "seq.message_on_line", String::new()),
            Some(SeqHit::Message(j)) => {
                st.note("seq.message_point_on_other_message");
                let _ = j;
            }
            Some(SeqHit::Participant(_)) | Some(SeqHit::Footer(_)) => st.note("seq.message_under_box"),
            other => st.check(false, "seq.message_on_line", format!("{name} msg {i} got {other:?}")),
        }
        for tol in [3.0, 8.0] {
            let q = (mid.0, mid.1 + 0.8 + 0.8 * tol);
            let ht = sc.hit_test(q.0, q.1, tol);
            st.check(
                matches!(ht, Some(SeqHit::Message(_)) | Some(SeqHit::Participant(_)) | Some(SeqHit::Footer(_))),
                "seq.tol_widens_message",
                format!("{name} {i} tol {tol} {ht:?}"),
            );
            let far = (mid.0, mid.1 + 0.8 + tol + 0.5);
            st.check(sc.hit_test(far.0, far.1, tol) != Some(SeqHit::Message(i)), "seq.beyond_tol_not_message", format!("{name} {i} tol {tol}"));
            st.check(sc.hit_test(q.0, q.1, 0.0) != Some(SeqHit::Message(i)), "seq.exact_not_beyond_message", format!("{name} {i}"));
        }
    }
    // lifelines: a point on each, away from bars and messages
    for (k, l) in sc.lifelines.iter().enumerate() {
        for s in 1..20 {
            let y = l.y0 + (l.y1 - l.y0) * s as f64 / 20.0 + 0.13;
            let hit = sc.hit_test(l.x, y, 0.0);
            if hit != Some(SeqHit::Lifeline(k)) {
                continue;
            }
            st.check(true, "seq.lifeline_on_ink", String::new());
            let q = (l.x + 0.5 + 0.8 * 4.0, y);
            let ht = sc.hit_test(q.0, q.1, 4.0);
            if sc.hit_test(q.0, q.1, 0.0).is_none() || matches!(sc.hit_test(q.0, q.1, 0.0), Some(SeqHit::FrameInside(_))) {
                st.check(ht.is_some() && !matches!(ht, Some(SeqHit::FrameInside(_))), "seq.tol_widens_lifeline", format!("{name} {k} {ht:?}"));
            }
            let far = (l.x + 0.5 + 4.0 + 0.5, y);
            st.check(sc.hit_test(far.0, far.1, 4.0) != Some(SeqHit::Lifeline(k)), "seq.beyond_tol_not_lifeline", format!("{name} {k}"));
            break;
        }
    }
    // frames: chip -> Frame(i); interior near the left border -> FrameInside (exact) / Frame (tol 3)
    for (i, fr) in sc.frames.iter().enumerate() {
        let chip = (fr.x + 4.0, fr.y + 9.0);
        let hit = sc.hit_test(chip.0, chip.1, 0.0);
        st.check(matches!(hit, Some(SeqHit::Frame(_))), "seq.frame_chip", format!("{name} frame {i} got {hit:?}"));
        let p = (fr.x + 3.0, fr.y + fr.h * 0.5 + 0.21);
        let h0 = sc.hit_test(p.0, p.1, 0.0);
        if let Some(SeqHit::FrameInside(j)) = h0 {
            // innermost: the smallest frame holding the point
            let inner = sc
                .frames
                .iter()
                .enumerate()
                .filter(|(_, g)| p.0 >= g.x && p.0 <= g.x + g.w && p.1 >= g.y && p.1 <= g.y + g.h)
                .min_by(|a, b| (a.1.w * a.1.h).total_cmp(&(b.1.w * b.1.h)))
                .map(|(k, _)| k);
            st.check(Some(j) == inner, "seq.frame_inside_is_innermost", format!("{name} frame {i} got {j} want {inner:?}"));
            let ht = sc.hit_test(p.0, p.1, 3.0);
            st.check(matches!(ht, Some(SeqHit::Frame(_))), "seq.tol_widens_frame_border", format!("{name} {i} {ht:?}"));
        } else {
            st.note("seq.frame_interior_probe_occupied");
        }
        // the border itself
        let b = (fr.x + 0.2, fr.y + fr.h * 0.5 + 0.21);
        let hb = sc.hit_test(b.0, b.1, 0.0);
        st.check(
            matches!(hb, Some(SeqHit::Frame(_)) | Some(SeqHit::Lifeline(_)) | Some(SeqHit::Message(_)) | Some(SeqHit::Note(_)) | Some(SeqHit::Activation(_))),
            "seq.frame_border_on_ink",
            format!("{name} {i} {hb:?}"),
        );
    }
    // z-order synthesized: a note over a lifeline and a message over the note
    if !sc.notes.is_empty() && !sc.lifelines.is_empty() && !sc.messages.is_empty() {
        let mut z = sc.clone();
        let l = z.lifelines[0].clone();
        let y = (l.y0 + l.y1) / 2.0;
        z.notes[0].x = l.x - 20.0;
        z.notes[0].y = y - 10.0;
        z.notes[0].w = 40.0;
        z.notes[0].h = 20.0;
        z.activations.clear();
        z.frames.clear();
        let hit = z.hit_test(l.x + 0.1, y + 3.3, 0.0);
        let msg_there = z.messages.iter().any(|m| m.points.iter().any(|p| (p.1 - (y + 3.3)).abs() < 2.0));
        if !msg_there {
            st.check(hit == Some(SeqHit::Note(0)), "seq.note_over_lifeline", format!("{name} {hit:?}"));
        }
        z.messages[0].points = vec![(l.x - 15.0, y), (l.x + 15.0, y)];
        z.messages[0].text.clear();
        z.messages[0].number = None;
        let hit = z.hit_test(l.x - 5.0, y, 0.0);
        st.check(hit == Some(SeqHit::Message(0)), "seq.message_over_note", format!("{name} {hit:?}"));
    }
}

// --------------------------------------------------------------- pie
fn piek(st: &mut St, name: &str, d: &flowmaid::model::PieChart) {
    let ps = pie::scene(d);
    let f = |x, y, t| ps.hit_test(x, y, t);
    off_canvas(st, name, ps.width, ps.height, f);
    tol_invariants(st, name, "pie", ps.width, ps.height, f, |h| matches!(h, PieHit::Slice(_) | PieHit::Legend(_)), false);
    st.check(ps.legend.len() == ps.slices.len(), "pie.legend_parallel_slices", name.into());
    for (i, row) in ps.legend.iter().enumerate() {
        let c = (row.x + pie::SWATCH / 2.0, row.y);
        st.check(ps.hit_test(c.0, c.1, 0.0) == Some(PieHit::Legend(i)), "pie.legend_swatch", format!("{name} {i}"));
        if let Some(s) = ps.slices.get(i) {
            st.check(row.text.contains(s.label.as_str()), "pie.legend_row_names_its_slice", format!("{name} {i}"));
        }
    }
    for (i, s) in ps.slices.iter().enumerate() {
        if s.frac <= 0.0 {
            continue;
        }
        let mid = (s.start_angle + s.end_angle) / 2.0;
        let at = |rr: f64| (ps.cx + rr * mid.sin(), ps.cy - rr * mid.cos());
        let mut done = false;
        for k in [0.85, 0.6, 0.35, 0.95] {
            let p = at(ps.r * k);
            match ps.hit_test(p.0, p.1, 0.0) {
                Some(PieHit::Slice(j)) if j == i => {
                    st.check(true, "pie.slice_inside", String::new());
                    done = true;
                    break;
                }
                Some(PieHit::Label { .. }) | Some(PieHit::Legend(_)) => continue,
                other => {
                    // a sliver whose sector is thinner than its 0.1 px path rounding
                    let width = (s.end_angle - s.start_angle) * ps.r * k;
                    if width < 1.0 {
                        st.note("pie.sliver_centre_not_resolved");
                        done = true;
                        break;
                    }
                    st.check(false, "pie.slice_inside", format!("{name} slice {i} frac {} r*{k} got {other:?}", s.frac));
                    done = true;
                    break;
                }
            }
        }
        if !done {
            st.note("pie.slice_all_probes_under_labels");
        }
        // just past the rim
        let p = at(ps.r + 0.6);
        for tol in [0.0, 5.0, 30.0] {
            let hit = ps.hit_test(p.0, p.1, tol);
            st.check(hit != Some(PieHit::Slice(i)), "pie.past_rim_not_slice", format!("{name} slice {i} tol {tol} {hit:?}"));
        }
    }
    for (k, l) in ps.labels.iter().enumerate() {
        st.check(l.slice < ps.slices.len(), "pie.label_slice_valid", name.into());
        let want = PieHit::Label { label: k, slice: l.slice };
        let hit = ps.hit_test(l.x + if l.anchor == "end" { -2.0 } else if l.anchor == "start" { 2.0 } else { 0.0 }, l.y, 0.0);
        st.check(hit == Some(want) || matches!(hit, Some(PieHit::Legend(_))), "pie.label_text", format!("{name} label {k} {hit:?}"));
        if l.leader.len() >= 2 && !l.inside {
            let (a, b) = (l.leader[l.leader.len() - 2], l.leader[l.leader.len() - 1]);
            let len = (b.0 - a.0).hypot(b.1 - a.1);
            if len > 4.0 {
                let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                let (nx, ny) = (-(b.1 - a.1) / len, (b.0 - a.0) / len);
                let h0 = ps.hit_test(m.0, m.1, 0.0);
                st.check(h0 == Some(want) || matches!(h0, Some(PieHit::Label { .. }) | Some(PieHit::Legend(_))), "pie.leader_on_ink", format!("{name} {k} {h0:?}"));
                let q = (m.0 + nx * (0.5 + 0.8 * 4.0), m.1 + ny * (0.5 + 0.8 * 4.0));
                let hq0 = ps.hit_test(q.0, q.1, 0.0);
                if hq0.is_none() {
                    let ht = ps.hit_test(q.0, q.1, 4.0);
                    st.check(matches!(ht, Some(PieHit::Label { .. })), "pie.tol_widens_leader", format!("{name} {k} {ht:?}"));
                }
            }
        }
    }
}

// --------------------------------------------------------------- journey
fn journeyk(st: &mut St, name: &str, d: &flowmaid::model::Journey) {
    let js = journey::scene(d);
    let f = |x, y, t| js.hit_test(x, y, t);
    off_canvas(st, name, js.width, js.height, f);
    tol_invariants(st, name, "journey", js.width, js.height, f, |_| true, true);
    for (i, t) in js.tasks.iter().enumerate() {
        let hit = js.hit_test(t.cx, t.cy, 0.0);
        st.check(hit == Some(JourneyHit::Task(i)) || matches!(hit, Some(JourneyHit::Legend(_))), "journey.task_face_centre", format!("{name} {i} {hit:?}"));
        for (k, &(dx, dy, _)) in t.actor_dots.iter().enumerate() {
            let hit = js.hit_test(dx, dy, 0.0);
            st.check(hit == Some(JourneyHit::Task(i)) || matches!(hit, Some(JourneyHit::Legend(_)) | Some(JourneyHit::Task(_))), "journey.actor_mark", format!("{name} {i}/{k} {hit:?}"));
        }
        let hit = js.hit_test(t.label_pos.0, t.label_pos.1, 0.0);
        st.check(hit == Some(JourneyHit::Task(i)) || matches!(hit, Some(JourneyHit::Legend(_))), "journey.task_label", format!("{name} {i} {hit:?}"));
        let p = (t.cx + t.r + 0.7 + 0.4, t.cy);
        for tol in [0.0, 20.0] {
            let hit = js.hit_test(p.0, p.1, tol);
            let dot_there = t.actor_dots.iter().any(|&(dx, dy, _)| (dx - p.0).hypot(dy - p.1) < 8.0);
            if !dot_there {
                st.check(hit != Some(JourneyHit::Task(i)), "journey.just_outside_face", format!("{name} {i} tol {tol} {hit:?}"));
            }
        }
    }
    for (a, it) in js.legend.iter().enumerate() {
        let hit = js.hit_test(it.x + journey::DOT_R, it.y, 0.0);
        st.check(hit == Some(JourneyHit::Legend(a)), "journey.legend_mark", format!("{name} {a} {hit:?}"));
    }
    for (s, b) in js.sections.iter().enumerate() {
        // a point in the band away from its centre text
        let p = (b.x + 7.3, b.y + b.h / 2.0);
        let hit = js.hit_test(p.0, p.1, 0.0);
        st.check(hit == Some(JourneyHit::Section(s)) || matches!(hit, Some(JourneyHit::Task(_)) | Some(JourneyHit::Legend(_))), "journey.section_band", format!("{name} {s} {hit:?}"));
        // the corners are round (rx 6)
        let hit = js.hit_test(b.x + 0.4, b.y + 0.4, 0.0);
        st.check(hit != Some(JourneyHit::Section(s)), "journey.section_round_corner_empty", format!("{name} {s}"));
    }
}

// --------------------------------------------------------------- timeline
fn timelinek(st: &mut St, name: &str, d: &flowmaid::model::Timeline) {
    let ts = timeline::scene(d);
    let f = |x, y, t| ts.hit_test(x, y, t);
    off_canvas(st, name, ts.width, ts.height, f);
    tol_invariants(st, name, "timeline", ts.width, ts.height, f, |h| matches!(h, TimelineHit::Event { .. }), false);
    for (s, sec) in ts.sections.iter().enumerate() {
        for (k, per) in sec.periods.iter().enumerate() {
            for (e, (_, ey)) in per.events.iter().enumerate() {
                let hit = ts.hit_test(per.cx + 0.3, *ey, 0.0);
                st.check(hit == Some(TimelineHit::Event { section: s, period: k, event: e }), "timeline.event_text", format!("{name} {s}/{k}/{e} {hit:?}"));
            }
            let hit = ts.hit_test(per.cx + 0.3, per.label_y, 0.0);
            st.check(hit == Some(TimelineHit::Period { section: s, period: k }), "timeline.period_label", format!("{name} {s}/{k} {hit:?}"));
            let hit = ts.hit_test(per.cx + 0.3, sec.axis_y, 0.0);
            st.check(hit == Some(TimelineHit::Period { section: s, period: k }), "timeline.period_tick", format!("{name} {s}/{k} {hit:?}"));
            for &(y0, y1) in &per.line_segments {
                if y1 - y0 < 4.0 {
                    continue;
                }
                let m = (per.cx, (y0 + y1) / 2.0);
                let hit = ts.hit_test(m.0, m.1, 0.0);
                st.check(hit == Some(TimelineHit::Period { section: s, period: k }) || matches!(hit, Some(TimelineHit::Event { .. })), "timeline.task_line", format!("{name} {s}/{k} {hit:?}"));
                let q = (m.0 + 0.6 + 0.8 * 4.0, m.1);
                if ts.hit_test(q.0, q.1, 0.0).is_none() {
                    let ht = ts.hit_test(q.0, q.1, 4.0);
                    st.check(ht == Some(TimelineHit::Period { section: s, period: k }), "timeline.tol_widens_task_line", format!("{name} {s}/{k} {ht:?}"));
                }
                let far = (m.0 + 0.6 + 4.0 + 0.5, m.1);
                st.check(ts.hit_test(far.0, far.1, 4.0) != Some(TimelineHit::Period { section: s, period: k }) || ts.hit_test(far.0, far.1, 0.0) == Some(TimelineHit::Period { section: s, period: k }), "timeline.beyond_tol_not_task_line", format!("{name} {s}/{k}"));
                break;
            }
        }
        // axis between the first two ticks
        if sec.periods.len() >= 2 {
            let x = (sec.periods[0].cx + sec.periods[1].cx) / 2.0 + 0.3;
            let hit = ts.hit_test(x, sec.axis_y, 0.0);
            st.check(hit == Some(TimelineHit::Section(s)), "timeline.axis", format!("{name} {s} {hit:?}"));
            let q = (x, sec.axis_y + 1.0 + 0.8 * 4.0);
            if ts.hit_test(q.0, q.1, 0.0).is_none() {
                st.check(ts.hit_test(q.0, q.1, 4.0) == Some(TimelineHit::Section(s)), "timeline.tol_widens_axis", format!("{name} {s}"));
            }
            let far = (x, sec.axis_y + 1.0 + 4.0 + 0.5);
            if ts.hit_test(far.0, far.1, 0.0).is_none() {
                st.check(ts.hit_test(far.0, far.1, 4.0).is_none(), "timeline.beyond_tol_not_axis", format!("{name} {s}"));
            }
        }
        if !sec.name.is_empty() {
            let hit = ts.hit_test(sec.x + 5.3, sec.y + 10.0, 0.0);
            st.check(hit == Some(TimelineHit::Section(s)), "timeline.section_band", format!("{name} {s} {hit:?}"));
        }
    }
}

// --------------------------------------------------------------- sankey
fn ribbon_band_at(l: &sankey::SankeyLinkGlyph, x: f64) -> Option<(f64, f64)> {
    // independent: sample the painter's top curve (M x0,top0 C cx,top0 cx,top1 x1,top1)
    let cx = (l.x0 + l.x1) / 2.0;
    let half = l.thickness / 2.0;
    let (t0, t1) = (l.y0 - half, l.y1 - half);
    let bez = |t: f64| {
        let u = 1.0 - t;
        let px = u * u * u * l.x0 + 3.0 * u * u * t * cx + 3.0 * u * t * t * cx + t * t * t * l.x1;
        let py = u * u * u * t0 + 3.0 * u * u * t * t0 + 3.0 * u * t * t * t1 + t * t * t * t1;
        (px, py)
    };
    if x < l.x0.min(l.x1) || x > l.x0.max(l.x1) {
        return None;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..80 {
        let m = (lo + hi) / 2.0;
        if (bez(m).0 < x) == (l.x1 > l.x0) {
            lo = m;
        } else {
            hi = m;
        }
    }
    let top = bez((lo + hi) / 2.0).1;
    Some((top, top + l.thickness))
}

fn sankeyk(st: &mut St, name: &str, d: &flowmaid::model::SankeyDiagram) {
    let ss = sankey::scene(d);
    let f = |x, y, t| ss.hit_test(x, y, t);
    off_canvas(st, name, ss.width, ss.height, f);
    tol_invariants(st, name, "sankey", ss.width, ss.height, f, |_| true, true);
    for (i, n) in ss.nodes.iter().enumerate() {
        let hit = ss.hit_test(n.x + n.w / 2.0, n.y + n.h / 2.0, 0.0);
        st.check(hit == Some(SankeyHit::Node(i)) || matches!(hit, Some(SankeyHit::Node(_))), "sankey.node_centre", format!("{name} {i} {hit:?}"));
        if hit != Some(SankeyHit::Node(i)) {
            st.note("sankey.node_centre_under_label");
        }
    }
    // ribbons against independent sampling of the painter's curve
    for (x, y) in grid(ss.width, ss.height, 120) {
        let hit = ss.hit_test(x, y, 0.0);
        if matches!(hit, Some(SankeyHit::Node(_))) {
            continue;
        }
        let want = ss
            .links
            .iter()
            .enumerate()
            .rev()
            .find(|(_, l)| l.thickness > 0.0 && ribbon_band_at(l, x).is_some_and(|(a, b)| y >= a && y <= b))
            .map(|(j, _)| SankeyHit::Link(j));
        // within 0.05 px of a band edge is undecidable
        let edge = ss.links.iter().any(|l| ribbon_band_at(l, x).is_some_and(|(a, b)| (y - a).abs() < 0.05 || (y - b).abs() < 0.05));
        if !edge {
            st.check(hit == want, "sankey.ribbon_matches_painter_curve", format!("{name} ({x:.2},{y:.2}) {hit:?} want {want:?}"));
        }
    }
    for (j, l) in ss.links.iter().enumerate() {
        if l.thickness < 2.0 {
            continue;
        }
        let x = (l.x0 + l.x1) / 2.0 + 0.3;
        if let Some((a, b)) = ribbon_band_at(l, x) {
            let hit = ss.hit_test(x, (a + b) / 2.0, 0.0);
            st.check(matches!(hit, Some(SankeyHit::Link(k)) if k >= j) || matches!(hit, Some(SankeyHit::Node(_))), "sankey.link_middle", format!("{name} {j} {hit:?}"));
            for tol in [0.0, 20.0] {
                let hit = ss.hit_test(x, a - 0.4, tol);
                st.check(hit != Some(SankeyHit::Link(j)), "sankey.just_above_ribbon_not_it", format!("{name} {j} tol {tol}"));
            }
        }
    }
}

fn empties(st: &mut St) {
    let sc = seq::SeqScene::default();
    st.check(sc.hit_test(0.0, 0.0, 5.0).is_none(), "empty.seq_default", String::new());
    let ms = mindmap::route(&flowmaid::model::Mindmap::default(), &[]);
    st.check(ms.hit_test(ms.width / 2.0, ms.height / 2.0, 5.0).is_none(), "empty.mind", String::new());
    for src in ["pie\n", "journey\n", "timeline\n", "sankey-beta\n", "sequenceDiagram\n"] {
        match parse_document(src) {
            Ok(Document::Pie(d)) => {
                let s = pie::scene(&d);
                st.check(grid(s.width, s.height, 30).iter().all(|&(x, y)| s.hit_test(x, y, 5.0).is_none()), "empty.pie", String::new());
            }
            Ok(Document::Journey(d)) => {
                let s = journey::scene(&d);
                st.check(grid(s.width, s.height, 30).iter().all(|&(x, y)| s.hit_test(x, y, 5.0).is_none()), "empty.journey", String::new());
            }
            Ok(Document::Timeline(d)) => {
                let s = timeline::scene(&d);
                st.check(grid(s.width, s.height, 30).iter().all(|&(x, y)| s.hit_test(x, y, 5.0).is_none()), "empty.timeline", String::new());
            }
            Ok(Document::Sankey(d)) => {
                let s = sankey::scene(&d);
                st.check(grid(s.width, s.height, 30).iter().all(|&(x, y)| s.hit_test(x, y, 5.0).is_none()), "empty.sankey", String::new());
            }
            Ok(Document::Sequence(d)) => {
                let s = seq::scene(&d);
                st.check(grid(s.width, s.height, 30).iter().all(|&(x, y)| s.hit_test(x, y, 5.0).is_none()), "empty.seq_parsed", String::new());
            }
            other => st.note(&format!("empty-parse-fails {src:?} {}", other.is_err())),
        }
    }
}

fn main() {
    let list = std::env::args().nth(1).unwrap();
    let mut st = St::default();
    empties(&mut st);
    for path in std::fs::read_to_string(&list).unwrap().lines() {
        let Ok(src) = std::fs::read_to_string(path) else { continue };
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut local = St::default();
            match parse_document(&src) {
                Ok(Document::Mindmap(d)) => mind(&mut local, &name, &d),
                Ok(Document::Sequence(d)) => seqk(&mut local, &name, &d),
                Ok(Document::Pie(d)) => piek(&mut local, &name, &d),
                Ok(Document::Journey(d)) => journeyk(&mut local, &name, &d),
                Ok(Document::Timeline(d)) => timelinek(&mut local, &name, &d),
                Ok(Document::Sankey(d)) => sankeyk(&mut local, &name, &d),
                _ => return None,
            }
            Some(local)
        }));
        match r {
            Ok(Some(local)) => {
                for (k, v) in local.per {
                    let e = st.per.entry(k).or_default();
                    e.0 += v.0;
                    e.1 += v.1;
                }
                for (k, v) in local.notes {
                    *st.notes.entry(k).or_default() += v;
                }
                for f in local.fails {
                    let tag = f.split(']').next().unwrap_or("").to_string();
                    if st.fails.iter().filter(|g| g.starts_with(&tag)).count() < 12 {
                        st.fails.push(f);
                    }
                }
                st.note("files");
            }
            Ok(None) => {}
            Err(_) => st.check(false, "PANIC", name.clone()),
        }
    }
    let mut total = (0, 0);
    for (k, (c, f)) in &st.per {
        println!("{k:50} checks={c:8} fails={f}");
        total.0 += c;
        total.1 += f;
    }
    for (k, v) in &st.notes {
        println!("note {k}: {v}");
    }
    println!("TOTAL checks={} fails={}", total.0, total.1);
    for f in &st.fails {
        println!("{f}");
    }
}
