//! Picking on the mindmap, sequence, pie, journey, timeline and sankey
//! scenes (issue #112), through the public API, on every repository
//! fixture of each type: a point on an element picks it, a point just
//! off a filled shape does not, `tol` widens strokes only, overlapping
//! elements resolve by paint order, and nothing is picked off the canvas.

use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::{journey, mindmap, pie, sankey, seq, timeline};
use std::path::{Path, PathBuf};

/// Every `.mmd` file under `dir`, recursively, in name order.
fn mmd_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            mmd_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "mmd") {
            out.push(p);
        }
    }
}

/// The repository's fixtures and examples, parsed (the ones that parse).
fn repo_documents() -> Vec<(String, Document)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    mmd_files(&root.join("examples"), &mut files);
    mmd_files(&root.join("tests/fixtures"), &mut files);
    files
        .into_iter()
        .filter_map(|p| {
            let src = std::fs::read_to_string(&p).unwrap();
            let name = p.strip_prefix(root).unwrap().display().to_string();
            parse_document(&src).ok().map(|d| (name, d))
        })
        .collect()
}

fn parse(src: &str) -> Document {
    parse_document(src).unwrap_or_else(|e| panic!("{src}\n{e:?}"))
}

/// The point at `t` on the cubic `b`.
fn cubic_at(b: &[(f64, f64); 4], t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    let k = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    (
        (0..4).map(|i| k[i] * b[i].0).sum(),
        (0..4).map(|i| k[i] * b[i].1).sum(),
    )
}

/// A scene's documents of one type, at least one per type.
macro_rules! docs_of {
    ($variant:ident) => {{
        let out: Vec<(String, _)> = repo_documents()
            .into_iter()
            .filter_map(|(n, d)| match d {
                Document::$variant(d) => Some((n, d)),
                _ => None,
            })
            .collect();
        assert!(!out.is_empty(), "no {} fixture", stringify!($variant));
        out
    }};
}

// ------------------------------------------------------------------
// Mindmap
// ------------------------------------------------------------------

#[test]
fn mindmap_picks_nodes_and_connectors() {
    use mindmap::MindHit;
    let mut links = 0;
    for (name, d) in docs_of!(Mindmap) {
        let ms = mindmap::scene(&d);
        for (i, n) in ms.nodes.iter().enumerate() {
            // The centre: the node, over the connectors that end there.
            assert_eq!(
                ms.hit_test(n.cx(), n.cy(), 3.0),
                Some(MindHit::Node(i)),
                "{name}"
            );
            // Just left of its box: not this node, whatever tol.
            let off = ms.hit_test(n.x - 0.5, n.cy(), 0.0);
            assert_ne!(off, Some(MindHit::Node(i)), "{name}: left of node {i}");
        }
        for (j, e) in ms.edges.iter().enumerate() {
            let (a, b) = (e.bezier[0], e.bezier[3]);
            let len = (b.0 - a.0).hypot(b.1 - a.1);
            let n = ((b.1 - a.1) / len, (a.0 - b.0) / len);
            // A spot on the connector clear of every node (by more than
            // the offsets tried below): nearest its middle.
            let clear = e.width / 2.0 + 4.0;
            let free = |p: (f64, f64)| {
                ms.nodes.iter().all(|nd| {
                    let dx = (nd.x - p.0).max(p.0 - nd.x - nd.w).max(0.0);
                    let dy = (nd.y - p.1).max(p.1 - nd.y - nd.h).max(0.0);
                    dx.hypot(dy) > clear
                })
            };
            let Some(mid) = (0..40)
                .map(|k| 0.5 + if k % 2 == 0 { 1.0 } else { -1.0 } * (k / 2) as f64 / 50.0)
                .map(|t| cubic_at(&e.bezier, t))
                .find(|&p| free(p))
            else {
                continue; // a connector hidden under its nodes
            };
            assert_eq!(
                ms.hit_test(mid.0, mid.1, 0.0),
                Some(MindHit::Link(j)),
                "{name}"
            );
            // Within tol of the stroke's edge, and beyond it.
            let half = e.width / 2.0;
            let at = |off: f64| (mid.0 + n.0 * (half + off), mid.1 + n.1 * (half + off));
            let near = at(2.5);
            assert_eq!(
                ms.hit_test(near.0, near.1, 3.0),
                Some(MindHit::Link(j)),
                "{name}"
            );
            assert_ne!(
                ms.hit_test(near.0, near.1, 2.0),
                Some(MindHit::Link(j)),
                "{name}"
            );
            assert_ne!(
                ms.hit_test(near.0, near.1, 0.0),
                Some(MindHit::Link(j)),
                "{name}"
            );
            links += 1;
        }
        assert_eq!(ms.hit_test(-1.0, 10.0, 5.0), None);
        assert_eq!(ms.hit_test(ms.width + 1.0, 10.0, 5.0), None);
    }
    assert!(links > 10, "only {links} connectors checked");
}

#[test]
fn mindmap_link_ends_name_the_two_nodes() {
    for (name, d) in docs_of!(Mindmap) {
        let ms = mindmap::scene(&d);
        let ends = mindmap::link_ends(&d);
        assert_eq!(ends.len(), ms.edges.len(), "{name}");
        for (e, &(p, c)) in ms.edges.iter().zip(&ends) {
            assert_eq!(e.bezier[0], (ms.nodes[p].cx(), ms.nodes[p].cy()), "{name}");
            assert_eq!(e.bezier[3], (ms.nodes[c].cx(), ms.nodes[c].cy()), "{name}");
        }
    }
}

#[test]
fn mindmap_later_node_wins_and_shapes_are_exact() {
    use mindmap::MindHit;
    let d = match parse("mindmap\n  root((Root))\n    a{{Hexagon}}\n    b[Square]\n    c(Pill)\n") {
        Document::Mindmap(d) => d,
        _ => unreachable!(),
    };
    let ms = mindmap::scene(&d);
    // Two nodes on one spot: the later one is on top.
    let c = (ms.nodes[1].cx(), ms.nodes[1].cy());
    let pos = vec![(c.0 + 300.0, c.1 + 200.0), c, c, (c.0 + 300.0, c.1)];
    let stacked = mindmap::route(&d, &pos);
    assert_eq!(stacked.hit_test(c.0, c.1, 0.0), Some(MindHit::Node(2)));
    // The circle root: inside its box's corner is not inside the circle.
    let r = &ms.nodes[0];
    assert_ne!(
        ms.hit_test(r.x + 1.0, r.y + 1.0, 0.0),
        Some(MindHit::Node(0))
    );
    // The hexagon's cut corner is not the hexagon.
    let h = &ms.nodes[1];
    assert_ne!(
        ms.hit_test(h.x + 1.0, h.y + 1.0, 0.0),
        Some(MindHit::Node(1))
    );
    // The square fills its box's corner.
    let s = &ms.nodes[2];
    assert_eq!(
        ms.hit_test(s.x + 0.5, s.y + 0.5, 0.0),
        Some(MindHit::Node(2))
    );
    // The pill's rounded end leaves its box's corner empty.
    let p = &ms.nodes[3];
    assert_ne!(
        ms.hit_test(p.x + 1.0, p.y + 1.0, 0.0),
        Some(MindHit::Node(3))
    );
    // An empty scene picks nothing.
    let empty = mindmap::route(&d, &[]);
    assert_eq!(empty.hit_test(10.0, 10.0, 5.0), None);
}

// ------------------------------------------------------------------
// Sequence
// ------------------------------------------------------------------

const SEQ_MIX: &str = "sequenceDiagram
  actor U as User
  participant A as App
  participant D as DB
  U->>A: open
  activate A
  loop every row
    A->>D: query
    D-->>A: rows
  end
  alt ok
    A->>U: show
  else failed
    A-xU: error
  end
  Note right of D: indexed
  deactivate A
";

fn seq_docs() -> Vec<(String, flowmaid::model::SequenceDiagram)> {
    let mut out = docs_of!(Sequence);
    if let Document::Sequence(d) = parse(SEQ_MIX) {
        out.push(("SEQ_MIX".into(), d));
    }
    out
}

#[test]
fn sequence_picks_every_element() {
    use seq::SeqHit;
    for (name, d) in seq_docs() {
        let sc = seq::scene(&d);
        for (k, b) in sc.boxes.iter().enumerate() {
            let c = if b.actor {
                seq::actor_figure(b).head
            } else {
                (b.x + b.w / 2.0, b.y + b.h / 2.0)
            };
            assert_eq!(
                sc.hit_test(c.0, c.1, 0.0),
                Some(SeqHit::Participant(k)),
                "{name}"
            );
        }
        for (k, b) in sc.footer.iter().enumerate() {
            let c = if b.actor {
                seq::actor_figure(b).head
            } else {
                (b.x + b.w / 2.0, b.y + b.h / 2.0)
            };
            assert_eq!(
                sc.hit_test(c.0, c.1, 0.0),
                Some(SeqHit::Footer(k)),
                "{name}"
            );
        }
        for (i, nb) in sc.notes.iter().enumerate() {
            let c = (nb.x + nb.w / 2.0, nb.y + nb.h / 2.0);
            assert_eq!(sc.hit_test(c.0, c.1, 0.0), Some(SeqHit::Note(i)), "{name}");
        }
        for (i, m) in sc.messages.iter().enumerate() {
            if !m.text.is_empty() && m.label_centered {
                let at = m.label_pos;
                assert_eq!(
                    sc.hit_test(at.0, at.1, 0.0),
                    Some(SeqHit::Message(i)),
                    "{name}"
                );
            }
            if m.points.len() == 2 {
                let (a, b) = (m.points[0], m.points[1]);
                let mid = ((a.0 + b.0) / 2.0, a.1);
                // On the line (a lifeline may cross it there: the
                // message is drawn over it).
                assert_eq!(
                    sc.hit_test(mid.0, mid.1, 0.0),
                    Some(SeqHit::Message(i)),
                    "{name}"
                );
                // 3 px below the line: within tol 3 of its stroke, not
                // within tol 1.
                let below = (mid.0, mid.1 + 3.0);
                assert_eq!(
                    sc.hit_test(below.0, below.1, 3.0),
                    Some(SeqHit::Message(i)),
                    "{name}"
                );
                assert_ne!(
                    sc.hit_test(below.0, below.1, 1.0),
                    Some(SeqHit::Message(i)),
                    "{name}"
                );
            }
        }
        for (i, a) in sc.activations.iter().enumerate() {
            // A spot on the bar no message runs across.
            let clear = |y: f64| {
                sc.messages
                    .iter()
                    .all(|m| m.points.iter().all(|p| (p.1 - y).abs() > 3.0))
            };
            let Some(y) = (1..20)
                .map(|k| a.y0 + (a.y1 - a.y0) * k as f64 / 20.0)
                .find(|&y| clear(y))
            else {
                continue;
            };
            assert_eq!(
                sc.hit_test(a.x, y, 0.0),
                Some(SeqHit::Activation(i)),
                "{name}"
            );
        }
        for (k, l) in sc.lifelines.iter().enumerate() {
            // Just under the header box, on the line and 2 px beside it.
            let y = sc.boxes[k].y + sc.boxes[k].h + 4.0;
            if y >= l.y1 {
                continue;
            }
            assert_eq!(
                sc.hit_test(l.x, y, 0.0),
                Some(SeqHit::Lifeline(k)),
                "{name}"
            );
            assert_eq!(
                sc.hit_test(l.x + 2.0, y, 2.0),
                Some(SeqHit::Lifeline(k)),
                "{name}"
            );
            assert_ne!(
                sc.hit_test(l.x + 2.0, y, 1.0),
                Some(SeqHit::Lifeline(k)),
                "{name}"
            );
        }
        assert_eq!(sc.hit_test(-1.0, 5.0, 3.0), None);
        assert_eq!(sc.hit_test(5.0, sc.height + 1.0, 3.0), None);
    }
}

#[test]
fn sequence_frames_tell_label_and_border_from_inside() {
    use seq::SeqHit;
    let Document::Sequence(d) = parse(SEQ_MIX) else {
        unreachable!()
    };
    let sc = seq::scene(&d);
    assert_eq!(sc.frames.len(), 2);
    for (i, f) in sc.frames.iter().enumerate() {
        // The keyword chip, and the left border.
        assert_eq!(
            sc.hit_test(f.x + 4.0, f.y + 4.0, 0.0),
            Some(SeqHit::Frame(i))
        );
        let mid = f.y + f.h * 0.4;
        assert_eq!(sc.hit_test(f.x, mid, 0.0), Some(SeqHit::Frame(i)));
        // Inside, on nothing drawn: 4 px in from the border (picked
        // only with a tol reaching the border).
        assert_eq!(
            sc.hit_test(f.x + 4.0, mid, 0.0),
            Some(SeqHit::FrameInside(i))
        );
        assert_eq!(sc.hit_test(f.x + 4.0, mid, 4.0), Some(SeqHit::Frame(i)));
        // A lifeline through the frame (DB's: never activated) is
        // picked over the frame.
        let l = &sc.lifelines[2];
        let on = (l.x, f.y + f.h - 3.0);
        assert_eq!(sc.hit_test(on.0, on.1, 0.0), Some(SeqHit::Lifeline(2)));
    }
    // The `else` divider belongs to the `alt` frame.
    let alt = &sc.frames[1];
    let (dy, _) = alt.dividers[0];
    let x = alt.x + alt.w - 6.0;
    assert_eq!(sc.hit_test(x, dy, 0.0), Some(SeqHit::Frame(1)));
}

// ------------------------------------------------------------------
// Pie
// ------------------------------------------------------------------

#[test]
fn pie_picks_slices_labels_and_legend() {
    use pie::PieHit;
    let extra =
        parse("pie title Pets\n\"Dogs\" : 386\n\"Cats\" : 85\n\"Rats\" : 15\n\"Bats\" : 1\n");
    let mut docs = docs_of!(Pie);
    if let Document::Pie(d) = extra {
        docs.push(("extra".into(), d));
    }
    for (name, d) in docs {
        let ps = pie::scene(&d);
        for (i, sl) in ps.slices.iter().enumerate() {
            if sl.frac <= 0.0 {
                continue;
            }
            let a = (sl.start_angle + sl.end_angle) / 2.0;
            let at = |r: f64| (ps.cx + r * a.sin(), ps.cy - r * a.cos());
            let inner = at(ps.r * 0.35);
            assert_eq!(
                ps.hit_test(inner.0, inner.1, 0.0),
                Some(PieHit::Slice(i)),
                "{name}"
            );
            // Just past the rim: not the slice.
            let out = at(ps.r + 0.5);
            assert_ne!(
                ps.hit_test(out.0, out.1, 0.0),
                Some(PieHit::Slice(i)),
                "{name}"
            );
        }
        for (k, l) in ps.labels.iter().enumerate() {
            let want = Some(PieHit::Label {
                label: k,
                slice: l.slice,
            });
            let x = match l.anchor {
                "start" => l.x + 4.0,
                "end" => l.x - 4.0,
                _ => l.x,
            };
            // An inside label is drawn over its wedge.
            assert_eq!(ps.hit_test(x, l.y, 0.0), want, "{name}: label {k}");
            if l.leader.len() >= 2 {
                let (a, b) = (l.leader[0], l.leader[1]);
                let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                if ps.hit_test(mid.0, mid.1, 0.0) == want {
                    let beside = (mid.0 + 2.5, mid.1);
                    assert_eq!(ps.hit_test(beside.0, beside.1, 3.0), want, "{name}");
                }
            }
        }
        for (i, row) in ps.legend.iter().enumerate() {
            let sw = (row.x + pie::SWATCH / 2.0, row.y);
            assert_eq!(
                ps.hit_test(sw.0, sw.1, 0.0),
                Some(PieHit::Legend(i)),
                "{name}"
            );
            let text = (row.x + pie::SWATCH + 10.0, row.y);
            assert_eq!(
                ps.hit_test(text.0, text.1, 0.0),
                Some(PieHit::Legend(i)),
                "{name}"
            );
        }
        assert_eq!(ps.hit_test(-1.0, ps.cy, 5.0), None);
    }
}

// ------------------------------------------------------------------
// Journey
// ------------------------------------------------------------------

#[test]
fn journey_picks_tasks_legend_and_sections() {
    use journey::JourneyHit;
    for (name, d) in docs_of!(Journey) {
        let js = journey::scene(&d);
        for (i, t) in js.tasks.iter().enumerate() {
            assert_eq!(
                js.hit_test(t.cx, t.cy, 0.0),
                Some(JourneyHit::Task(i)),
                "{name}"
            );
            assert_eq!(
                js.hit_test(t.label_pos.0, t.label_pos.1, 0.0),
                Some(JourneyHit::Task(i)),
                "{name}: label"
            );
            for &(x, y, _) in &t.actor_dots {
                assert_eq!(
                    js.hit_test(x, y, 0.0),
                    Some(JourneyHit::Task(i)),
                    "{name}: dot"
                );
            }
            // Just off the face, beside it: not the task.
            let off = (t.cx + t.r + 0.5, t.cy);
            assert_ne!(
                js.hit_test(off.0, off.1, 5.0),
                Some(JourneyHit::Task(i)),
                "{name}"
            );
        }
        for (a, it) in js.legend.iter().enumerate() {
            let mark = (it.x + journey::DOT_R, it.y);
            assert_eq!(
                js.hit_test(mark.0, mark.1, 0.0),
                Some(JourneyHit::Legend(a)),
                "{name}"
            );
            let text = (it.x + 2.0 * journey::DOT_R + 8.0, it.y);
            assert_eq!(
                js.hit_test(text.0, text.1, 0.0),
                Some(JourneyHit::Legend(a)),
                "{name}"
            );
        }
        for (i, b) in js.sections.iter().enumerate() {
            let c = (b.x + b.w / 2.0, b.y + b.h / 2.0);
            assert_eq!(
                js.hit_test(c.0, c.1, 0.0),
                Some(JourneyHit::Section(i)),
                "{name}"
            );
            // The band's rounded corner leaves its box's corner empty.
            assert_ne!(
                js.hit_test(b.x + 0.5, b.y + 0.5, 0.0),
                Some(JourneyHit::Section(i))
            );
        }
        assert_eq!(js.hit_test(js.width + 1.0, 5.0, 5.0), None);
    }
}

// ------------------------------------------------------------------
// Timeline
// ------------------------------------------------------------------

#[test]
fn timeline_picks_events_periods_and_sections() {
    use timeline::TimelineHit;
    let mut docs = docs_of!(Timeline);
    if let Document::Timeline(d) = parse("timeline TD\n  2002 : A\n  section S\n    2004 : B : C\n")
    {
        docs.push(("td".into(), d));
    }
    for (name, d) in docs {
        let ts = timeline::scene(&d);
        for (s, sec) in ts.sections.iter().enumerate() {
            for (k, p) in sec.periods.iter().enumerate() {
                let period = Some(TimelineHit::Period {
                    section: s,
                    period: k,
                });
                for (e, &(_, y)) in p.events.iter().enumerate() {
                    let want = Some(TimelineHit::Event {
                        section: s,
                        period: k,
                        event: e,
                    });
                    assert_eq!(ts.hit_test(p.cx, y, 0.0), want, "{name}");
                }
                assert_eq!(ts.hit_test(p.cx, p.label_y, 0.0), period, "{name}: label");
                assert_eq!(ts.hit_test(p.cx, sec.axis_y, 0.0), period, "{name}: tick");
                for &(y0, y1) in &p.line_segments {
                    let y = (y0 + y1) / 2.0;
                    assert_eq!(ts.hit_test(p.cx, y, 0.0), period, "{name}: task line");
                    assert_eq!(
                        ts.hit_test(p.cx + 2.5, y, 2.0),
                        period,
                        "{name}: within tol"
                    );
                    assert_ne!(
                        ts.hit_test(p.cx + 2.5, y, 1.0),
                        period,
                        "{name}: beyond tol"
                    );
                }
            }
            // The axis between two ticks, and the named band.
            if let [a, b, ..] = sec.periods.as_slice() {
                let x = (a.cx + b.cx) / 2.0;
                assert_eq!(
                    ts.hit_test(x, sec.axis_y, 0.0),
                    Some(TimelineHit::Section(s))
                );
            }
            if !sec.name.is_empty() {
                let c = (sec.x + sec.w / 2.0, sec.y + timeline::SECTION_H / 2.0);
                assert_eq!(
                    ts.hit_test(c.0, c.1, 0.0),
                    Some(TimelineHit::Section(s)),
                    "{name}"
                );
            }
        }
        assert_eq!(ts.hit_test(5.0, -1.0, 5.0), None);
    }
}

// ------------------------------------------------------------------
// Sankey
// ------------------------------------------------------------------

/// Whether `p` lies in link `l`'s ribbon, by a polygon of its outline
/// sampled densely: a check independent of the pick's own geometry.
fn in_ribbon_polygon(l: &sankey::SankeyLinkGlyph, p: (f64, f64)) -> bool {
    let half = l.thickness / 2.0;
    let cx = (l.x0 + l.x1) / 2.0;
    let top = [
        (l.x0, l.y0 - half),
        (cx, l.y0 - half),
        (cx, l.y1 - half),
        (l.x1, l.y1 - half),
    ];
    let bot = [
        (l.x1, l.y1 + half),
        (cx, l.y1 + half),
        (cx, l.y0 + half),
        (l.x0, l.y0 + half),
    ];
    let mut poly: Vec<(f64, f64)> = (0..=400)
        .map(|k| cubic_at(&top, k as f64 / 400.0))
        .collect();
    poly.extend((0..=400).map(|k| cubic_at(&bot, k as f64 / 400.0)));
    let mut inside = false;
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) * (b.0 - a.0) / (b.1 - a.1) {
            inside = !inside;
        }
    }
    inside
}

#[test]
fn sankey_picks_nodes_and_ribbons() {
    use sankey::SankeyHit;
    let mut checked = 0;
    for (name, d) in docs_of!(Sankey) {
        let ss = sankey::scene(&d);
        for (i, n) in ss.nodes.iter().enumerate() {
            let c = (n.x + n.w / 2.0, n.y + n.h / 2.0);
            assert_eq!(
                ss.hit_test(c.0, c.1, 0.0),
                Some(SankeyHit::Node(i)),
                "{name}"
            );
            assert_eq!(
                ss.hit_test(
                    n.label_pos.0 + if n.label_anchor == "end" { -3.0 } else { 3.0 },
                    n.label_pos.1,
                    0.0
                ),
                Some(SankeyHit::Node(i)),
                "{name}: label of {i}"
            );
        }
        let node_at = |p: (f64, f64)| {
            ss.nodes.iter().any(|n| {
                p.0 >= n.x - 1.0
                    && p.0 <= n.x + n.w + 1.0
                    && p.1 >= n.y - 1.0
                    && p.1 <= n.y + n.h + 1.0
            })
        };
        for (j, l) in ss.links.iter().enumerate() {
            for t in [0.25, 0.5, 0.75] {
                let x = l.x0 + (l.x1 - l.x0) * t;
                // Down the ribbon at that x: its top edge, the middle and
                // just inside the bottom edge, by the sampled outline.
                let (lo, hi) = (l.y0.min(l.y1) - l.thickness, l.y0.max(l.y1) + l.thickness);
                let ys: Vec<f64> = (0..=400)
                    .map(|k| lo + (hi - lo) * k as f64 / 400.0)
                    .filter(|&y| in_ribbon_polygon(l, (x, y)))
                    .collect();
                for &y in &ys {
                    let p = (x, y);
                    let hit = ss.hit_test(p.0, p.1, 0.0);
                    if node_at(p) || matches!(hit, Some(SankeyHit::Node(_))) {
                        continue; // a bar or a label is drawn over it
                    }
                    // The last ribbon over the point, by the outlines.
                    let top = (0..ss.links.len())
                        .rev()
                        .find(|&k| in_ribbon_polygon(&ss.links[k], p));
                    if top == Some(j) {
                        assert_eq!(hit, Some(SankeyHit::Link(j)), "{name}: ribbon {j} at {p:?}");
                        checked += 1;
                    }
                }
            }
            // Above the ribbon by a px: not this ribbon, whatever tol.
            let x = (l.x0 + l.x1) / 2.0;
            let top = (l.y0 + l.y1) / 2.0 - l.thickness / 2.0 - 1.0;
            assert_ne!(ss.hit_test(x, top, 5.0), Some(SankeyHit::Link(j)), "{name}");
        }
        assert_eq!(ss.hit_test(-1.0, 5.0, 5.0), None);
    }
    assert!(checked > 100, "only {checked} ribbon points checked");
}
