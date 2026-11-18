//! `route_partial` for ER, class and mindmap diagrams (issue #111),
//! through the public API: a drag frame redraws only what touches the
//! moved nodes, keeps everything else exactly as its base draws it, and
//! falls back to the full `route` when the base no longer lines up.

use flowmaid::model::{ClassDiagram, Document, ErDiagram, Mindmap};
use flowmaid::parser::parse_document;
use flowmaid::scene::SceneEdge;
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
    for (j, e) in base.scene.edges.iter().enumerate() {
        if moved[index(&e.from)] || moved[index(&e.to)] {
            continue;
        }
        assert_eq!(
            drawn(&frame.scene.edges[j]),
            drawn(e),
            "{name}: edge {j} is not attached to a moved box but changed (box {i})"
        );
    }
    assert!(
        frame.scene.origin.0 <= base.scene.origin.0 && frame.scene.origin.1 <= base.scene.origin.1
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
    // Base counts that do not fit.
    assert!(same(
        &class::route_partial(&d, &to, &base, &at[..2]),
        &class::route(&d, &to)
    ));
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
