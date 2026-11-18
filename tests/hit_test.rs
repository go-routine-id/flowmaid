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

/// The painter's geometry at the rims: a filled shape takes in its
/// outline's stroke (a participant box 1.6 wide, an actor's head 1.6, a
/// note 1.2, an activation bar 1.4), and a message line ends square at
/// its ends (butt caps), so `tol` reaches past an end but `tol = 0`
/// does not.
#[test]
fn sequence_picks_outline_strokes_and_square_line_ends() {
    use seq::SeqHit;
    let mut ends = 0;
    for (name, d) in seq_docs() {
        let sc = seq::scene(&d);
        for (k, b) in sc.boxes.iter().enumerate() {
            let (at, past) = if b.actor {
                let f = seq::actor_figure(b);
                let x = f.head.0 + f.head_r;
                ((x + 0.7, f.head.1), (x + 1.0, f.head.1))
            } else {
                let y = b.y + b.h / 2.0;
                ((b.x - 0.7, y), (b.x - 1.0, y))
            };
            assert_eq!(
                sc.hit_test(at.0, at.1, 0.0),
                Some(SeqHit::Participant(k)),
                "{name}: box {k}'s outline"
            );
            assert_ne!(
                sc.hit_test(past.0, past.1, 5.0),
                Some(SeqHit::Participant(k)),
                "{name}: past box {k}'s outline"
            );
        }
        for (i, nb) in sc.notes.iter().enumerate() {
            let y = nb.y + nb.h / 2.0;
            assert_eq!(
                sc.hit_test(nb.x + nb.w + 0.5, y, 0.0),
                Some(SeqHit::Note(i)),
                "{name}: note {i}'s outline"
            );
        }
        for (i, a) in sc.activations.iter().enumerate() {
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
            let x = a.x + seq::ACT_W / 2.0 + 0.6;
            if sc
                .activations
                .iter()
                .any(|o| (o.x - a.x).abs() > 0.1 && (o.x - x).abs() < 6.0)
            {
                continue; // a nested bar beside it
            }
            assert_eq!(
                sc.hit_test(x, y, 0.0),
                Some(SeqHit::Activation(i)),
                "{name}: bar {i}'s outline"
            );
        }
        // A straight message's tail end, 0.5 px back along the line from
        // where it starts: off its square end at `tol = 0` (a round cap
        // would cover it), on it at 1.
        for (i, m) in sc.messages.iter().enumerate() {
            let [a, b] = m.points[..] else { continue };
            let back = if b.0 > a.0 { -0.5 } else { 0.5 };
            let p = (a.0 + back, a.1);
            let lit = |tol: f64| sc.hit_test(p.0, p.1, tol) == Some(SeqHit::Message(i));
            if sc
                .hit_test(p.0, p.1, 0.0)
                .is_some_and(|h| h != SeqHit::Message(i))
                && !lit(1.0)
            {
                continue; // something else drawn over it there
            }
            assert!(!lit(0.0), "{name}: message {i} past its tail end");
            assert!(lit(1.0), "{name}: message {i} within tol of its tail");
            ends += 1;
        }
    }
    assert!(ends > 0);
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
        // The border's corners are 4 round: a point in the bounding
        // box's corner, past the stroke, is neither on the frame nor in
        // it, until `tol` reaches the stroke.
        let corner = (f.x + 0.5, f.y + f.h - 0.5);
        let hit = sc.hit_test(corner.0, corner.1, 0.0);
        assert!(
            !matches!(hit, Some(SeqHit::Frame(_) | SeqHit::FrameInside(_))),
            "{hit:?}"
        );
        assert_eq!(sc.hit_test(corner.0, corner.1, 1.0), Some(SeqHit::Frame(i)));
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

/// A slice as `to_svg` fills it, read off the SVG text: a disc
/// (`<circle>`, filled), or a wedge path
/// `M cx cy L ax ay A r r 0 large 1 bx by Z` as an SVG renderer fills
/// it, its arc centred where SVG's endpoint-to-centre conversion puts
/// it and flattened to a polygon (256 points along it).
enum DrawnSlice {
    Disc((f64, f64), f64),
    Wedge(Vec<(f64, f64)>),
}

/// The wedge path `M c L a A r r 0 large 1 b Z` as a polygon.
fn wedge_polygon(
    c: (f64, f64),
    a: (f64, f64),
    r: f64,
    large: bool,
    b: (f64, f64),
) -> Vec<(f64, f64)> {
    let (hx, hy) = ((a.0 - b.0) / 2.0, (a.1 - b.1) / 2.0);
    let d2 = hx * hx + hy * hy;
    let rr = r.max(d2.sqrt());
    let sign = if large { -1.0 } else { 1.0 };
    let k = sign * ((rr * rr - d2) / d2).max(0.0).sqrt();
    let m = ((a.0 + b.0) / 2.0 + k * hy, (a.1 + b.1) / 2.0 - k * hx);
    let ta = (a.1 - m.1).atan2(a.0 - m.0);
    let tb = (b.1 - m.1).atan2(b.0 - m.0);
    let sweep = (tb - ta).rem_euclid(std::f64::consts::TAU);
    let mut poly = vec![c, a];
    for i in 1..256 {
        let t = ta + sweep * i as f64 / 256.0;
        poly.push((m.0 + rr * t.cos(), m.1 + rr * t.sin()));
    }
    poly.push(b);
    poly
}

/// The slices of a pie's SVG in paint order (a hatching painted over
/// its slice's colour repeats the shape: read once).
fn drawn_slices(svg: &str) -> Vec<DrawnSlice> {
    // Each token of the path in its place (`L`, `A`, `Z` read as NaN).
    let num = |s: &str| -> Vec<f64> {
        s.split(' ')
            .map(|t| t.parse().unwrap_or(f64::NAN))
            .collect()
    };
    let attr = |tag: &str, name: &str| -> f64 {
        let at = tag.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
        tag[at..].split('"').next().unwrap().parse().unwrap()
    };
    let mut out = Vec::new();
    let mut last = String::new();
    for line in svg.lines().map(str::trim) {
        let (key, shape) = if let Some(rest) = line.strip_prefix("<path d=\"M ") {
            let d = rest.split('"').next().unwrap();
            let v = num(d);
            let poly = wedge_polygon(
                (v[0], v[1]),
                (v[3], v[4]),
                v[6],
                v[9] == 1.0,
                (v[11], v[12]),
            );
            (d.to_string(), DrawnSlice::Wedge(poly))
        } else if line.starts_with("<circle ") && !line.contains("fill=\"none\"") {
            let c = (attr(line, "cx"), attr(line, "cy"));
            let r = attr(line, "r");
            (format!("circle {c:?} {r}"), DrawnSlice::Disc(c, r))
        } else {
            continue;
        };
        if key != last {
            out.push(shape);
        }
        last = key;
    }
    out
}

/// Whether `p` lies in `s` (a wedge by its polygon's winding number).
fn drawn_contains(s: &DrawnSlice, p: (f64, f64)) -> bool {
    match s {
        DrawnSlice::Disc(c, r) => (p.0 - c.0).hypot(p.1 - c.1) <= *r,
        DrawnSlice::Wedge(poly) => {
            let mut w = 0;
            for i in 0..poly.len() {
                let (u, v) = (poly[i], poly[(i + 1) % poly.len()]);
                let side = (v.0 - u.0) * (p.1 - u.1) - (p.0 - u.0) * (v.1 - u.1);
                if u.1 <= p.1 && v.1 > p.1 && side > 0.0 {
                    w += 1;
                } else if u.1 > p.1 && v.1 <= p.1 && side < 0.0 {
                    w -= 1;
                }
            }
            w != 0
        }
    }
}

/// A slice is picked as `to_svg` fills it, at the 0.1 px its numbers
/// are written to. Where a slice's arc runs between ends a few px apart
/// (a sliver beside a dominant slice), a renderer centres the arc's
/// circle a px or more off the pie's centre: the dominant slice's rim
/// then lies past the pie's radius on one side and short of it on the
/// other, and the picks follow it. Over rings round the rim and through
/// the wedges (points within 0.05 px of a drawn edge left out), every
/// slice pick agrees with the SVG's own shapes.
#[test]
fn pie_slices_are_picked_as_their_paths_are_drawn() {
    use pie::PieHit;
    let mut docs = docs_of!(Pie);
    for src in [
        "pie\n\"a\" : 100\n\"b\" : 0.5\n",
        "pie\n\"a\" : 1000\n\"b\" : 10\n\"c\" : 5\n",
        "pie\n\"a\" : 0.004\n\"b\" : 0.006\n\"c\" : 0.99\n",
        "pie title Pets\n\"Dogs\" : 386\n\"Cats\" : 85\n\"Rats\" : 15\n\"Bats\" : 1\n",
        "pie\n\"a\" : 300\n\"b\" : 1\n",
    ] {
        if let Document::Pie(d) = parse(src) {
            docs.push((src.into(), d));
        }
    }
    for (name, d) in docs {
        let ps = pie::scene(&d);
        let drawn = drawn_slices(&pie::to_svg(&ps));
        let shown: Vec<usize> = (0..ps.slices.len())
            .filter(|&i| ps.slices[i].frac > 0.0)
            .collect();
        assert_eq!(drawn.len(), shown.len(), "{name}");
        let top = |p: (f64, f64)| {
            (0..drawn.len())
                .rev()
                .find(|&k| drawn_contains(&drawn[k], p))
                .map(|k| shown[k])
        };
        let (mut checked, mut off_exact) = (0, 0);
        for ring in (0..=12)
            .map(|k| ps.r - 3.0 + 0.5 * k as f64)
            .chain([0.3 * ps.r, 0.6 * ps.r])
        {
            for step in 0..720 {
                let t = step as f64 / 720.0 * std::f64::consts::TAU;
                let p = (ps.cx + ring * t.sin(), ps.cy - ring * t.cos());
                let want = top(p);
                let steady = [(0.05, 0.0), (-0.05, 0.0), (0.0, 0.05), (0.0, -0.05)]
                    .iter()
                    .all(|d| top((p.0 + d.0, p.1 + d.1)) == want);
                if !steady {
                    continue;
                }
                let got = match ps.hit_test(p.0, p.1, 0.0) {
                    Some(PieHit::Slice(i)) => Some(i),
                    None => None,
                    Some(_) => continue,
                };
                assert_eq!(got, want, "{name}: at {p:?} (ring {ring})");
                checked += 1;
                if (ring > ps.r) == want.is_some() {
                    off_exact += 1;
                }
            }
        }
        assert!(checked > 5_000, "{name}: {checked}");
        // Not vacuous: the near-full slices are drawn off the exact disc.
        if name.contains("\"b\" : 0.5") {
            assert!(off_exact > 100, "{name}: {off_exact}");
        }
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
            // On the face's outline, whose stroke (1.4 wide) reaches 0.7
            // past its radius: the task. Just past that stroke: not the
            // task, whatever the tolerance (it widens no filled shape).
            let rim = (t.cx + t.r + 0.5, t.cy);
            assert_eq!(
                js.hit_test(rim.0, rim.1, 0.0),
                Some(JourneyHit::Task(i)),
                "{name}: outline"
            );
            let off = (t.cx + t.r + 1.0, t.cy);
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

/// A tick takes in its ring's stroke (1.5 wide), and the axis (2 wide)
/// ends square where it is drawn (butt caps): `tol` reaches past its
/// end, `tol = 0` does not.
#[test]
fn timeline_picks_tick_rings_and_square_axis_ends() {
    use timeline::TimelineHit;
    let mut ends = 0;
    for (name, d) in docs_of!(Timeline) {
        let ts = timeline::scene(&d);
        for (s, sec) in ts.sections.iter().enumerate() {
            for (k, p) in sec.periods.iter().enumerate() {
                let y = sec.axis_y - timeline::TICK_R - 0.6;
                assert_eq!(
                    ts.hit_test(p.cx, y, 0.0),
                    Some(TimelineHit::Period {
                        section: s,
                        period: k
                    }),
                    "{name}: tick ring"
                );
            }
            let x = sec.x + sec.w + 0.6;
            // Nothing else drawn there (a section beside it).
            if ts.hit_test(x, sec.axis_y, 5.0) != Some(TimelineHit::Section(s)) {
                continue;
            }
            assert_eq!(ts.hit_test(x, sec.axis_y, 0.0), None, "{name}: axis end");
            assert_eq!(
                ts.hit_test(x, sec.axis_y, 1.0),
                Some(TimelineHit::Section(s)),
                "{name}: axis end within tol"
            );
            ends += 1;
        }
    }
    assert!(ends > 0);
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
