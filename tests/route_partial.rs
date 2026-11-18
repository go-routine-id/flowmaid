//! `route_partial` for ER, class and mindmap diagrams (issue #111),
//! through the public API: a drag frame redraws only what touches the
//! moved nodes, keeps everything else exactly as its base draws it, and
//! falls back to the full `route` when the base no longer lines up.

use flowmaid::model::{ClassDiagram, Document, ErDiagram, Mindmap};
use flowmaid::parser::parse_document;
use flowmaid::scene::{Scene, SceneEdge};
use flowmaid::{class, er, mindmap};
use std::path::{Path, PathBuf};

/// Two results are the same when every field prints the same.
fn same<T: std::fmt::Debug>(a: &T, b: &T) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

/// Everything an edge is drawn with: its ends, curve, path and label.
fn drawn(e: &SceneEdge) -> String {
    format!(
        "{} {} {:?} {:?} {:?} {:?}",
        e.from, e.to, e.bezier, e.waypoints, e.label, e.kind
    )
}

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

/// The `viewBox` (x, y, width, height) `svg` is drawn on.
fn view_box(svg: &str) -> (f64, f64, f64, f64) {
    let at = svg.find("viewBox=\"").expect("no viewBox") + 9;
    let v: Vec<f64> = svg[at..]
        .split('"')
        .next()
        .unwrap()
        .split(' ')
        .map(|t| t.parse().unwrap())
        .collect();
    (v[0], v[1], v[2], v[3])
}

/// Every box of `sc`, every point its lines are drawn through and every
/// label chip lie on its canvas (`origin`, `width` x `height`) and inside
/// the `viewBox` of `svg`, its SVG (to the 0.5 px that is written to):
/// nothing the frame draws is cut off.
fn assert_holds_the_drawing(name: &str, sc: &Scene, svg: &str) {
    let (ox, oy) = sc.origin;
    let (fx, fy) = (ox + sc.width, oy + sc.height);
    let (vx, vy, vw, vh) = view_box(svg);
    let mut parts: Vec<(String, (f64, f64, f64, f64))> = Vec::new();
    for n in &sc.nodes {
        let r = (
            n.x - n.w / 2.0,
            n.y - n.h / 2.0,
            n.x + n.w / 2.0,
            n.y + n.h / 2.0,
        );
        parts.push((format!("box {}", n.id), r));
    }
    for (j, e) in sc.edges.iter().enumerate() {
        for p in &e.waypoints {
            parts.push((
                format!("line {j} ({} -> {})", e.from, e.to),
                (p.0, p.1, p.0, p.1),
            ));
        }
        if let Some((_, m, w)) = &e.label {
            let r = (m.0 - w / 2.0, m.1 - 10.0, m.0 + w / 2.0, m.1 + 10.0);
            parts.push((format!("label of line {j}"), r));
        }
    }
    for (what, (x0, y0, x1, y1)) in parts {
        assert!(
            x0 >= ox - 1e-6 && y0 >= oy - 1e-6 && x1 <= fx + 1e-6 && y1 <= fy + 1e-6,
            "{name}: {what} at ({x0}, {y0})-({x1}, {y1}) is off the canvas ({ox}, {oy})-({fx}, {fy})"
        );
        assert!(
            x0 >= vx - 0.5 && y0 >= vy - 0.5 && x1 <= vx + vw + 0.5 && y1 <= vy + vh + 0.5,
            "{name}: {what} at ({x0}, {y0})-({x1}, {y1}) is cut off by the viewBox {vx} {vy} {vw} {vh}"
        );
    }
}

/// The far corner of `sc`'s canvas.
fn far(sc: &Scene) -> (f64, f64) {
    (sc.origin.0 + sc.width, sc.origin.1 + sc.height)
}

fn parse(src: &str) -> Document {
    parse_document(src).unwrap_or_else(|e| panic!("{src}\n{e:?}"))
}

fn er_of(src: &str) -> ErDiagram {
    match parse(src) {
        Document::Er(d) => d,
        _ => panic!("not ER"),
    }
}

fn class_of(src: &str) -> ClassDiagram {
    match parse(src) {
        Document::Class(d) => d,
        _ => panic!("not a class diagram"),
    }
}

fn mind_of(src: &str) -> Mindmap {
    match parse(src) {
        Document::Mindmap(d) => d,
        _ => panic!("not a mindmap"),
    }
}

/// An ER diagram with a cycle (a relationship laid out reversed), a
/// self-relationship, labels and every cardinality.
const ER_MIX: &str = "erDiagram
  CUSTOMER ||--o{ ORDER : places
  ORDER ||--|{ LINE : contains
  LINE }o--|| PRODUCT : of
  PRODUCT }|..|{ CATEGORY : in
  CATEGORY |o--o| CUSTOMER : likes
  CUSTOMER ||--o| CUSTOMER : refers
  ORDER }o--o{ PRODUCT : lists
";

/// The ER documents the tests drag: every repository ER fixture, then
/// [`ER_MIX`].
fn er_docs() -> Vec<(String, ErDiagram)> {
    let mut out: Vec<(String, ErDiagram)> = repo_documents()
        .into_iter()
        .filter_map(|(n, d)| match d {
            Document::Er(d) => Some((n, d)),
            _ => None,
        })
        .collect();
    assert!(!out.is_empty(), "no ER fixture in the repository");
    out.push(("ER_MIX".into(), er_of(ER_MIX)));
    out
}

fn er_centres(es: &er::ErScene) -> Vec<(f64, f64)> {
    es.scene.nodes.iter().map(|n| (n.x, n.y)).collect()
}

/// A drag frame of `base` (drawn at `at`) with entity `i` moved to `to`:
/// every entity at its new centre, every relationship without an end on
/// `i` drawn exactly as in `base`, tables and cardinalities as `d` has
/// them.
fn check_er_frame(name: &str, d: &ErDiagram, base: &er::ErScene, at: &[(f64, f64)], i: usize) {
    let mut to = at.to_vec();
    to[i].0 += 37.0;
    to[i].1 += 23.0;
    let frame = er::route_partial(d, &to, base, at);
    assert!(same(&frame.tables, &base.tables), "{name}: tables");
    assert!(same(&frame.cards, &base.cards), "{name}: cards");
    for (k, n) in frame.scene.nodes.iter().enumerate() {
        assert_eq!((n.x, n.y), to[k], "{name}: entity {k} not where it was put");
    }
    let mut redrawn = 0;
    for (j, r) in d.relations.iter().enumerate() {
        if r.from == i || r.to == i {
            redrawn += 1;
            continue;
        }
        assert_eq!(
            drawn(&frame.scene.edges[j]),
            drawn(&base.scene.edges[j]),
            "{name}: relationship {j} is not attached to entity {i} but changed"
        );
    }
    // The canvas only grows, and still holds every table.
    assert!(
        frame.scene.origin.0 <= base.scene.origin.0 && frame.scene.origin.1 <= base.scene.origin.1
    );
    let (ox, oy) = frame.scene.origin;
    for n in &frame.scene.nodes {
        assert!(
            n.x + n.w / 2.0 <= ox + frame.scene.width + 1e-6,
            "{name}: canvas too narrow"
        );
        assert!(
            n.y + n.h / 2.0 <= oy + frame.scene.height + 1e-6,
            "{name}: canvas too short"
        );
    }
    // A redrawn relationship still runs between its two tables.
    for (j, r) in d.relations.iter().enumerate() {
        if (r.from == i || r.to == i) && r.from != r.to {
            let e = &frame.scene.edges[j];
            assert_eq!(
                e.from, d.entities[r.from].name,
                "{name}: relationship {j} turned"
            );
        }
    }
    let _ = redrawn;
}

#[test]
fn er_drag_keeps_every_relationship_not_attached_to_the_moved_entity() {
    for (name, d) in er_docs() {
        let laid = er::scene(&d);
        let at = er_centres(&laid);
        let routed = er::route(&d, &at);
        for i in 0..d.entities.len() {
            check_er_frame(&name, &d, &laid, &at, i);
            check_er_frame(&name, &d, &routed, &at, i);
        }
    }
}

#[test]
fn er_drag_frames_chain() {
    let d = er_of(ER_MIX);
    let mut base = er::scene(&d);
    let mut at = er_centres(&base);
    // Drag ORDER, then PRODUCT, each frame built on the one before.
    for (step, i) in [1, 1, 3, 3, 0].into_iter().enumerate() {
        check_er_frame("ER_MIX chained", &d, &base, &at, i);
        let mut to = at.clone();
        to[i].0 += 11.0 * (step as f64 + 1.0);
        base = er::route_partial(&d, &to, &base, &at);
        at = to;
    }
}

#[test]
fn er_drag_without_a_move_is_the_base() {
    let d = er_of(ER_MIX);
    let base = er::scene(&d);
    let at = er_centres(&base);
    assert!(same(&er::route_partial(&d, &at, &base, &at), &base));
}

#[test]
fn er_drag_after_an_edit_is_a_full_route() {
    let d = er_of(ER_MIX);
    let base = er::scene(&d);
    let at = er_centres(&base);
    // An entity and a relationship added: one more position.
    let d2 = er_of(&format!("{ER_MIX}  SHIPPER ||--o{{ ORDER : ships\n"));
    let mut at2 = at.clone();
    at2.push((at[0].0 + 300.0, at[0].1));
    assert!(same(
        &er::route_partial(&d2, &at2, &base, &at),
        &er::route(&d2, &at2)
    ));
    // A relationship removed, one entity moved.
    let d3 = er_of(&ER_MIX.replace("  ORDER }o--o{ PRODUCT : lists\n", ""));
    let mut to = at.clone();
    to[2].0 += 25.0;
    assert!(same(
        &er::route_partial(&d3, &to, &base, &at),
        &er::route(&d3, &to)
    ));
    // A label changed (same counts).
    let d4 = er_of(&ER_MIX.replace(": places", ": buys"));
    assert!(same(
        &er::route_partial(&d4, &to, &base, &at),
        &er::route(&d4, &to)
    ));
    // An attribute added: the table grows.
    let d5 = er_of(&format!("{ER_MIX}  LINE {{\n    int qty\n  }}\n"));
    assert!(same(
        &er::route_partial(&d5, &to, &base, &at),
        &er::route(&d5, &to)
    ));
    // `base` not drawn at `base_centers`.
    let mut off = at.clone();
    off[4].1 += 9.0;
    assert!(same(
        &er::route_partial(&d, &to, &base, &off),
        &er::route(&d, &to)
    ));
    // Nor at a centre off the numbers.
    for bad in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let mut off = at.clone();
        off[4].0 = bad;
        assert!(same(
            &er::route_partial(&d, &to, &base, &off),
            &er::route(&d, &to)
        ));
    }
    // Base counts that do not fit.
    assert!(same(
        &er::route_partial(&d, &to, &base, &at[..3]),
        &er::route(&d, &to)
    ));
}

#[test]
fn er_drag_past_the_top_left_keeps_positions() {
    let d = er_of(ER_MIX);
    let base = er::scene(&d);
    let at = er_centres(&base);
    let mut to = at.clone();
    to[0] = (-150.0, -90.0);
    let frame = er::route_partial(&d, &to, &base, &at);
    assert_eq!((frame.scene.nodes[0].x, frame.scene.nodes[0].y), to[0]);
    assert!(frame.scene.origin.0 < 0.0 && frame.scene.origin.1 < 0.0);
    // The SVG's viewBox starts before zero on both axes: scene space.
    let svg = er::to_svg(&frame);
    assert!(svg.contains("viewBox=\"-"), "no negative viewBox origin");
    let vb = &svg[svg.find("viewBox=\"").unwrap() + 9..];
    let y: f64 = vb.split(' ').nth(1).unwrap().parse().unwrap();
    assert!(y < 0.0);
}

/// A drag that grows the canvas up and left (its origin negative), then
/// one that pushes another table past its right or bottom side: the
/// second frame's canvas, and its SVG's `viewBox`, still hold every
/// table, chained on the first frame or on a full `route` there, and
/// it never shrinks from its base's.
#[test]
fn er_drag_right_and_down_from_a_canvas_grown_up_and_left() {
    for (name, d) in er_docs() {
        let n = d.entities.len();
        if n < 2 {
            continue;
        }
        let laid = er::scene(&d);
        let at = er_centres(&laid);
        let mut to1 = at.clone();
        to1[0] = (-150.0, -90.0);
        let f1 = er::route_partial(&d, &to1, &laid, &at);
        assert!(f1.scene.origin.0 < 0.0 && f1.scene.origin.1 < 0.0, "{name}");
        assert_holds_the_drawing(&format!("{name} up-left"), &f1.scene, &er::to_svg(&f1));
        let routed = er::route(&d, &to1);
        for (dx, dy) in [(600.0, 400.0), (300.0, 200.0), (-3.0, 260.0), (260.0, -3.0)] {
            let mut to2 = to1.clone();
            to2[n - 1].0 += dx;
            to2[n - 1].1 += dy;
            for (how, base) in [("chained", &f1), ("on route", &routed)] {
                let f2 = er::route_partial(&d, &to2, base, &to1);
                let what = format!("{name} {how}, then +({dx}, {dy})");
                assert_holds_the_drawing(&what, &f2.scene, &er::to_svg(&f2));
                let (b, f) = (far(&base.scene), far(&f2.scene));
                assert!(f.0 >= b.0 && f.1 >= b.1, "{what}: canvas shrank");
                assert!(
                    f2.scene.origin.0 <= base.scene.origin.0
                        && f2.scene.origin.1 <= base.scene.origin.1,
                    "{what}: canvas shrank"
                );
            }
        }
    }
}

#[test]
fn er_drag_counts_behave_like_route() {
    let d = er_of(ER_MIX);
    let base = er::scene(&d);
    let at = er_centres(&base);
    assert!(er::try_route_partial(&d, &at[..2], &base, &at).is_none());
    let mut bad = at.clone();
    bad[1].0 = f64::NAN;
    assert!(er::try_route_partial(&d, &bad, &base, &at).is_none());
    let mut to = at.clone();
    to[1].0 += 5.0;
    let ok = er::try_route_partial(&d, &to, &base, &at).expect("fits");
    assert!(same(&ok, &er::route_partial(&d, &to, &base, &at)));
}

#[test]
#[should_panic]
fn er_drag_with_a_short_position_list_panics_like_route() {
    let d = er_of(ER_MIX);
    let base = er::scene(&d);
    let at = er_centres(&base);
    let _ = er::route_partial(&d, &at[..2], &base, &at);
}

// ------------------------------------------------------------------
// Mindmap
// ------------------------------------------------------------------

/// The mindmaps the tests drag: every repository mindmap fixture, then
/// a small styled one.
fn mind_docs() -> Vec<(String, Mindmap)> {
    let mut out: Vec<(String, Mindmap)> = repo_documents()
        .into_iter()
        .filter_map(|(n, d)| match d {
            Document::Mindmap(d) => Some((n, d)),
            _ => None,
        })
        .collect();
    assert!(!out.is_empty(), "no mindmap fixture in the repository");
    out.push((
        "small".into(),
        mind_of("mindmap\n  root((r))\n    a[A]:::hot\n      a1\n      a2\n    b)B(\n      b1\n  classDef hot fill:#f00\n"),
    ));
    out
}

fn mind_centres(ms: &mindmap::MindScene) -> Vec<(f64, f64)> {
    ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect()
}

#[test]
fn mindmap_drag_draws_what_route_draws_and_keeps_the_rest() {
    for (name, d) in mind_docs() {
        let base = mindmap::scene(&d);
        let at = mind_centres(&base);
        let ends = mindmap::link_ends(&d);
        assert_eq!(ends.len(), base.edges.len(), "{name}");
        for i in 0..d.nodes.len() {
            let mut to = at.clone();
            to[i] = (to[i].0 - 41.0, to[i].1 + 17.0);
            let frame = mindmap::route_partial(&d, &to, &base, &at);
            let full = mindmap::route(&d, &to);
            assert_eq!(
                mindmap::to_svg(&frame),
                mindmap::to_svg(&full),
                "{name}: node {i}"
            );
            assert_eq!(
                (frame.width, frame.height),
                (full.width, full.height),
                "{name}"
            );
            for (k, n) in frame.nodes.iter().enumerate() {
                if k != i {
                    assert_eq!(
                        (n.x, n.y),
                        (base.nodes[k].x, base.nodes[k].y),
                        "{name}: node {k}"
                    );
                }
            }
            for (j, &(p, c)) in ends.iter().enumerate() {
                if p != i && c != i {
                    assert_eq!(
                        frame.edges[j].bezier, base.edges[j].bezier,
                        "{name}: link {j}"
                    );
                } else {
                    assert_eq!(
                        frame.edges[j].bezier[0],
                        (frame.nodes[p].cx(), frame.nodes[p].cy())
                    );
                    assert_eq!(
                        frame.edges[j].bezier[3],
                        (frame.nodes[c].cx(), frame.nodes[c].cy())
                    );
                }
            }
        }
    }
}

#[test]
fn mindmap_drag_after_an_edit_is_a_full_route() {
    let src = "mindmap\n  root\n    a\n      a1\n    b\n";
    let d = mind_of(src);
    let base = mindmap::scene(&d);
    let at = mind_centres(&base);
    // A node added.
    let d2 = mind_of(&format!("{src}      b1\n"));
    let mut at2 = at.clone();
    at2.push((at[3].0 + 80.0, at[3].1));
    assert!(same(
        &mindmap::route_partial(&d2, &at2, &base, &at),
        &mindmap::route(&d2, &at2)
    ));
    // A node removed.
    let d3 = mind_of("mindmap\n  root\n    a\n    b\n");
    assert!(same(
        &mindmap::route_partial(&d3, &at[..3], &base, &at),
        &mindmap::route(&d3, &at[..3])
    ));
    // A node moved under another parent (same count).
    let d4 = mind_of("mindmap\n  root\n    a\n    b\n      a1\n");
    let mut to = at.clone();
    to[1].0 += 10.0;
    assert!(same(
        &mindmap::route_partial(&d4, &to, &base, &at),
        &mindmap::route(&d4, &to)
    ));
    // A label changed.
    let d5 = mind_of(&src.replace("a1", "a one"));
    assert!(same(
        &mindmap::route_partial(&d5, &to, &base, &at),
        &mindmap::route(&d5, &to)
    ));
    // `base` not drawn at `base_pos`, or at a centre off the numbers.
    for bad in [7.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let mut off = at.clone();
        off[2].0 = if bad == 7.0 { off[2].0 + bad } else { bad };
        assert!(same(
            &mindmap::route_partial(&d, &to, &base, &off),
            &mindmap::route(&d, &to)
        ));
    }
    // No move: the base.
    assert!(same(&mindmap::route_partial(&d, &at, &base, &at), &base));
}

#[test]
fn mindmap_drag_counts_behave_like_route() {
    let d = mind_of("mindmap\n  root\n    a\n    b\n");
    let base = mindmap::scene(&d);
    let at = mind_centres(&base);
    // `route` draws an empty scene for a wrong count; so does this.
    assert!(same(
        &mindmap::route_partial(&d, &at[..2], &base, &at),
        &mindmap::route(&d, &at[..2])
    ));
    assert!(mindmap::try_route_partial(&d, &at[..2], &base, &at).is_none());
    let mut to = at.clone();
    to[2].1 -= 30.0;
    let ok = mindmap::try_route_partial(&d, &to, &base, &at).expect("fits");
    assert!(same(&ok, &mindmap::route_partial(&d, &to, &base, &at)));
}

// ------------------------------------------------------------------
// Class
// ------------------------------------------------------------------

/// A class diagram with a fan, labels, cardinalities, glyphs at either
/// operand, a two-way relation, a self-relation and notes (one free).
const CLASS_MIX: &str = "classDiagram
  Animal <|-- Duck
  Animal <|-- Fish
  Animal <|-- Zebra
  Animal \"1\" *-- \"many\" Leg : has
  Duck --> Pond : swims in
  Fish ..|> Swimmer
  Zebra <--> Animal : herd
  Pond o-- Fish
  Duck --> Duck : quacks
  class Animal {
    +int age
    +isMammal() bool
  }
  note for Duck \"can fly\"
  note \"a free note\"
";

fn class_docs() -> Vec<(String, ClassDiagram)> {
    let mut out: Vec<(String, ClassDiagram)> = repo_documents()
        .into_iter()
        .filter_map(|(n, d)| match d {
            Document::Class(d) => Some((n, d)),
            _ => None,
        })
        .collect();
    assert!(!out.is_empty(), "no class fixture in the repository");
    out.push(("CLASS_MIX".into(), class_of(CLASS_MIX)));
    out
}

fn class_centres(cs: &class::ClassScene) -> Vec<(f64, f64)> {
    cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect()
}

/// A drag frame of `base` (drawn at `at`) with box `i` moved: every box
/// at its centre, every edge whose two boxes stayed drawn exactly as in
/// `base`, and the boxes, notes and styles as `d` has them.
fn check_class_frame(
    name: &str,
    d: &ClassDiagram,
    base: &class::ClassScene,
    at: &[(f64, f64)],
    i: usize,
) {
    let mut to = at.to_vec();
    to[i].0 -= 29.0;
    to[i].1 += 31.0;
    let frame = class::route_partial(d, &to, base, at);
    assert!(same(&frame.boxes, &base.boxes), "{name}: boxes");
    assert!(same(&frame.rels, &base.rels), "{name}: rels");
    assert!(same(&frame.notes, &base.notes), "{name}: notes");
    let mut moved = Vec::new();
    for (k, n) in frame.scene.nodes.iter().enumerate() {
        if k < to.len() {
            assert_eq!((n.x, n.y), to[k], "{name}: box {k} not where it was put");
        }
        moved.push((n.x, n.y) != (base.scene.nodes[k].x, base.scene.nodes[k].y));
    }
    let index = |id: &str| frame.scene.nodes.iter().position(|n| n.id == id).unwrap();
    let (cards, base_cards) = (class::card_spots(&frame), class::card_spots(base));
    for (j, e) in base.scene.edges.iter().enumerate() {
        if moved[index(&e.from)] || moved[index(&e.to)] {
            continue;
        }
        assert_eq!(
            drawn(&frame.scene.edges[j]),
            drawn(e),
            "{name}: edge {j} is not attached to a moved box but changed (box {i})"
        );
        // Its cardinalities stay where they were, and its glyphs.
        if j < base.rels.len() {
            assert_eq!(
                cards[j], base_cards[j],
                "{name}: the cardinalities of relation {j} moved (box {i})"
            );
            assert_eq!(
                format!("{:?}", class::rel_head(&frame, j)),
                format!("{:?}", class::rel_head(base, j)),
                "{name}: the glyph of relation {j} moved (box {i})"
            );
        }
    }
    assert!(
        frame.scene.origin.0 <= base.scene.origin.0 && frame.scene.origin.1 <= base.scene.origin.1
    );
    // The canvas only grows, and still holds every box.
    let (bf, ff) = (far(&base.scene), far(&frame.scene));
    assert!(
        ff.0 >= bf.0 && ff.1 >= bf.1,
        "{name}: canvas shrank (box {i})"
    );
    assert_holds_the_drawing(
        &format!("{name} (box {i})"),
        &frame.scene,
        &class::to_svg(&frame),
    );
    // The redrawn edges are drawn: an end on each of their boxes' borders.
    for (j, e) in frame.scene.edges.iter().enumerate() {
        let (a, b) = (index(&e.from), index(&e.to));
        if !(moved[a] || moved[b]) || a == b {
            continue;
        }
        let start = e.waypoints.first().copied().unwrap_or(e.bezier[0]);
        let n = &frame.scene.nodes[a];
        let off = ((start.0 - n.x).abs() - n.w / 2.0).max((start.1 - n.y).abs() - n.h / 2.0);
        assert!(
            off.abs() < 1.0,
            "{name}: edge {j} does not leave its box ({off})"
        );
    }
}

#[test]
fn class_drag_keeps_every_edge_not_attached_to_a_moved_box() {
    for (name, d) in class_docs() {
        let laid = class::scene(&d);
        let at = class_centres(&laid);
        let routed = class::route(&d, &at);
        let classes = d.classes.len();
        for i in 0..at.len() {
            check_class_frame(&name, &d, &laid, &at, i);
            check_class_frame(&name, &d, &routed, &at, i);
            // Class centres only: notes placed beside their classes.
            if i < classes {
                check_class_frame(
                    &name,
                    &d,
                    &class::route(&d, &at[..classes]),
                    &at[..classes],
                    i,
                );
            }
        }
    }
}

#[test]
fn class_drag_frames_chain() {
    let d = class_of(CLASS_MIX);
    let mut base = class::scene(&d);
    let mut at = class_centres(&base);
    for (step, i) in [1, 1, 0, 4, 4, 0].into_iter().enumerate() {
        check_class_frame("CLASS_MIX chained", &d, &base, &at, i);
        let mut to = at.clone();
        to[i].1 -= 13.0 * (step as f64 + 1.0);
        base = class::route_partial(&d, &to, &base, &at);
        at = to;
    }
}

#[test]
fn class_drag_without_a_move_is_the_base() {
    let d = class_of(CLASS_MIX);
    let base = class::scene(&d);
    let at = class_centres(&base);
    assert!(same(&class::route_partial(&d, &at, &base, &at), &base));
}

#[test]
fn class_drag_after_an_edit_is_a_full_route() {
    let d = class_of(CLASS_MIX);
    let base = class::scene(&d);
    let at = class_centres(&base);
    let classes = d.classes.len();
    let mut to = at.clone();
    to[2].0 += 21.0;
    let edits = [
        // A class added (with a relation): one more class centre.
        format!("{CLASS_MIX}  Pond <|-- Lake\n"),
        // A relation removed.
        CLASS_MIX.replace("  Pond o-- Fish\n", ""),
        // A member changed.
        CLASS_MIX.replace("+int age", "+int years"),
        // A label changed.
        CLASS_MIX.replace(": swims in", ": dives in"),
        // A cardinality changed.
        CLASS_MIX.replace("\"many\"", "\"0..n\""),
        // A note added.
        format!("{CLASS_MIX}  note for Fish \"wet\"\n"),
    ];
    for src in &edits {
        let d2 = class_of(src);
        let n2 = d2.classes.len();
        // Class centres (the notes placed beside them), as many as fit.
        let mut c2: Vec<(f64, f64)> = to[..classes.min(n2)].to_vec();
        while c2.len() < n2 {
            c2.push((to[0].0 + 250.0, to[0].1 + 40.0 * c2.len() as f64));
        }
        assert!(
            same(
                &class::route_partial(&d2, &c2, &base, &at),
                &class::route(&d2, &c2)
            ),
            "{src}"
        );
    }
    // `base` not drawn at `base_centers`.
    let mut off = at.clone();
    off[3].0 -= 7.0;
    assert!(same(
        &class::route_partial(&d, &to, &base, &off),
        &class::route(&d, &to)
    ));
    // Nor at a centre off the numbers.
    for bad in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let mut off = at.clone();
        off[3].1 = bad;
        assert!(same(
            &class::route_partial(&d, &to, &base, &off),
            &class::route(&d, &to)
        ));
    }
    // Base counts that do not fit.
    assert!(same(
        &class::route_partial(&d, &to, &base, &at[..2]),
        &class::route(&d, &to)
    ));
}

/// Nine relations into one hub, each with cardinalities at both ends:
/// a redrawn relation passes near the kept ones' cardinalities.
fn hub_src() -> String {
    let mut s = String::from("classDiagram\n");
    for k in 0..9 {
        s.push_str(&format!("  Svc{k} \"1\" ..|> \"*\" Hub\n"));
    }
    s
}

#[test]
fn class_drag_keeps_the_cardinalities_of_the_kept_relations() {
    let d = class_of(&hub_src());
    let laid = class::scene(&d);
    let at = class_centres(&laid);
    for base in [laid.clone(), class::route(&d, &at)] {
        let base_cards = class::card_spots(&base);
        for i in 0..at.len() {
            for (dx, dy) in [(40.0, 25.0), (-35.0, 60.0), (120.0, -15.0)] {
                let mut to = at.clone();
                to[i] = (at[i].0 + dx, at[i].1 + dy);
                let frame = class::route_partial(&d, &to, &base, &at);
                let cards = class::card_spots(&frame);
                let svg = class::to_svg(&frame);
                for (j, r) in d.relations.iter().enumerate() {
                    if r.from == i || r.to == i {
                        continue;
                    }
                    assert_eq!(cards[j], base_cards[j], "relation {j}, box {i} moved");
                    // ... and the SVG writes them there.
                    for c in cards[j].iter().flatten() {
                        let at = format!("<text x=\"{:.1}\" y=\"{:.1}\"", c.0, c.1);
                        assert!(svg.contains(&at), "relation {j}, box {i}: no card {at}");
                    }
                }
                // Frames chain: a frame's kept cards stay on the next.
                let mut next = to.clone();
                next[i].1 += 11.0;
                let again = class::route_partial(&d, &next, &frame, &to);
                let cards2 = class::card_spots(&again);
                for (j, r) in d.relations.iter().enumerate() {
                    if r.from != i && r.to != i {
                        assert_eq!(cards2[j], base_cards[j], "chained: relation {j}");
                    }
                }
            }
        }
    }
}

#[test]
fn class_drag_past_the_top_left_keeps_positions() {
    let d = class_of(CLASS_MIX);
    let base = class::scene(&d);
    let at = class_centres(&base);
    let mut to = at.clone();
    to[0] = (-120.0, -70.0);
    let frame = class::route_partial(&d, &to, &base, &at);
    assert_eq!((frame.scene.nodes[0].x, frame.scene.nodes[0].y), to[0]);
    assert!(frame.scene.origin.0 < 0.0 && frame.scene.origin.1 < 0.0);
    assert!(class::to_svg(&frame).contains("viewBox=\"-"));
}

/// [`er_drag_right_and_down_from_a_canvas_grown_up_and_left`] for class
/// diagrams, and boxes dragged right or down from a `route` whose canvas
/// already starts above or left of zero without a drag (a free note
/// above the first class, as `examples/class.mmd` routes at the class
/// centres): every frame's canvas and `viewBox` hold every box.
#[test]
fn class_drag_right_and_down_from_a_canvas_grown_up_and_left() {
    let moves = [(600.0, 400.0), (300.0, 200.0), (-3.0, 260.0), (260.0, -3.0)];
    for (name, d) in class_docs() {
        let classes = d.classes.len();
        if classes < 2 {
            continue;
        }
        let laid = class::scene(&d);
        let at = class_centres(&laid);
        let mut to1 = at.clone();
        to1[0] = (-150.0, -90.0);
        let f1 = class::route_partial(&d, &to1, &laid, &at);
        assert!(f1.scene.origin.0 < 0.0 && f1.scene.origin.1 < 0.0, "{name}");
        assert_holds_the_drawing(&format!("{name} up-left"), &f1.scene, &class::to_svg(&f1));
        let routed = class::route(&d, &to1);
        for (dx, dy) in moves {
            let mut to2 = to1.clone();
            to2[classes - 1].0 += dx;
            to2[classes - 1].1 += dy;
            for (how, base) in [("chained", &f1), ("on route", &routed)] {
                let f2 = class::route_partial(&d, &to2, base, &to1);
                let what = format!("{name} {how}, then +({dx}, {dy})");
                assert_holds_the_drawing(&what, &f2.scene, &class::to_svg(&f2));
                let (b, f) = (far(&base.scene), far(&f2.scene));
                assert!(f.0 >= b.0 && f.1 >= b.1, "{what}: canvas shrank");
            }
        }
        // On a route at the class centres only (notes placed beside their
        // classes, a free one above the first): no drag past the top-left.
        let cc = &at[..classes];
        let base = class::route(&d, cc);
        for i in 0..classes {
            for (dx, dy) in [(900.0, 500.0), (-3.0, 260.0), (37.0, 0.0)] {
                let mut to = cc.to_vec();
                to[i].0 += dx;
                to[i].1 += dy;
                let f = class::route_partial(&d, &to, &base, cc);
                let what = format!("{name} on route(classes), box {i} +({dx}, {dy})");
                assert_holds_the_drawing(&what, &f.scene, &class::to_svg(&f));
            }
        }
    }
}

/// Twelve labelled relations into a hub, some classes with members.
fn hub_fan_src() -> String {
    let mut s = String::from(
        "classDiagram\n  direction TB\n  class Hub {\n    +id int\n    +name String\n    +save() bool\n  }\n",
    );
    for i in (0..11).filter(|i| i % 3 != 2) {
        if i % 3 == 0 {
            s.push_str(&format!(
                "  class Svc{i} {{\n    +run()\n    -state int\n  }}\n"
            ));
        } else {
            s.push_str(&format!("  class Svc{i}\n"));
        }
    }
    for i in 0..12 {
        s.push_str(&format!("  Svc{i} ..> Hub : uses{i}\n"));
    }
    s
}

/// The hub of [`hub_fan_src`] dragged past the top-left: the router runs
/// a relation round the hub's left side, farther left than the boxes and
/// labels reach. The frame's canvas, and its SVG's `viewBox`, hold that
/// line too, also in the next frame of the drag.
#[test]
fn class_drag_frame_holds_a_line_routed_past_the_top_left() {
    let d = class_of(&hub_fan_src());
    let laid = class::scene(&d);
    let at = class_centres(&laid);
    let mut to = at.clone();
    to[0] = (-150.0, -90.0);
    let f = class::route_partial(&d, &to, &laid, &at);
    let left = |sc: &Scene| {
        let boxes = sc.nodes.iter().map(|n| n.x - n.w / 2.0);
        let labels = sc
            .edges
            .iter()
            .filter_map(|e| e.label.as_ref().map(|(_, m, w)| m.0 - w / 2.0));
        let lines = sc
            .edges
            .iter()
            .flat_map(|e| e.waypoints.iter().map(|p| p.0));
        (
            boxes.chain(labels).fold(f64::INFINITY, f64::min),
            lines.fold(f64::INFINITY, f64::min),
        )
    };
    let (drawn, line) = left(&f.scene);
    // Not vacuous: a line runs past the margin the boxes and labels take.
    assert!(line < drawn - 16.0, "{line} {drawn}");
    assert_holds_the_drawing("hub dragged up-left", &f.scene, &class::to_svg(&f));
    let mut next = to.clone();
    next[3].1 += 40.0;
    let f2 = class::route_partial(&d, &next, &f, &to);
    assert_holds_the_drawing("then a box below", &f2.scene, &class::to_svg(&f2));
    assert!(f2.scene.origin.0 <= f.scene.origin.0 && far(&f2.scene).0 >= far(&f.scene).0);
}

#[test]
fn class_drag_counts_behave_like_route() {
    let d = class_of(CLASS_MIX);
    let base = class::scene(&d);
    let at = class_centres(&base);
    let classes = d.classes.len();
    assert!(class::try_route_partial(&d, &at[..classes - 1], &base, &at).is_none());
    assert!(class::try_route_partial(&d, &at[..at.len() - 1], &base, &at).is_none());
    let mut to = at.clone();
    to[1].1 += 9.0;
    for c in [&to[..], &to[..classes]] {
        let ok = class::try_route_partial(&d, c, &base, &at).expect("fits");
        assert!(same(&ok, &class::route_partial(&d, c, &base, &at)));
    }
    let mut bad = to.clone();
    bad[0].1 = f64::INFINITY;
    assert!(class::try_route_partial(&d, &bad, &base, &at).is_none());
}

#[test]
#[should_panic]
fn class_drag_with_a_short_position_list_panics_like_route() {
    let d = class_of(CLASS_MIX);
    let base = class::scene(&d);
    let at = class_centres(&base);
    let _ = class::route_partial(&d, &at[..1], &base, &at);
}
