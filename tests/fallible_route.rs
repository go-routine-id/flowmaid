//! The fallible `try_route` of every diagram type with a `route` (issue
//! #110), through the public API: `None` for a position count `route`
//! does not accept, or a coordinate that is not finite or lies more than
//! a million px from zero, and exactly `route`'s result otherwise.

use flowmaid::model::{ClassDiagram, Document, ErDiagram, Graph, Mindmap};
use flowmaid::parser::parse_document;
use flowmaid::{class, er, mindmap, scene};
use std::path::{Path, PathBuf};

/// Two results are the same when every field prints the same.
fn same<T: std::fmt::Debug>(a: &T, b: &T) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

/// `k` positions: the laid-out centres first, then made-up ones.
fn positions(laid: &[(f64, f64)], k: usize) -> Vec<(f64, f64)> {
    (0..k)
        .map(|i| {
            laid.get(i)
                .copied()
                .unwrap_or((40.0 + 30.0 * i as f64, 40.0 + 20.0 * i as f64))
        })
        .collect()
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

/// The node centres a document is laid out with, and the position counts
/// its `route` accepts.
type Laid = (Vec<(f64, f64)>, Vec<usize>);

/// The [`Laid`] of `d`, for a diagram type with a `route`.
fn laid_out(d: &Document) -> Option<Laid> {
    let centres = |nodes: &[scene::SceneNode]| nodes.iter().map(|n| (n.x, n.y)).collect();
    Some(match d {
        Document::Flowchart(g) | Document::State(g) => {
            (centres(&scene::scene(g).nodes), vec![g.nodes.len()])
        }
        Document::Class(c) => {
            let n = c.classes.len();
            let counts = if c.notes.is_empty() {
                vec![n]
            } else {
                vec![n, n + c.notes.len()]
            };
            (centres(&class::scene(c).scene.nodes), counts)
        }
        Document::Er(e) => (centres(&er::scene(e).scene.nodes), vec![e.entities.len()]),
        Document::Mindmap(m) => {
            let laid = mindmap::scene(m);
            let pos = laid.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
            (pos, vec![m.nodes.len()])
        }
        _ => return None,
    })
}

/// `try_route` of `d` at `pos`, and whether it equals `route` (checked
/// only where it returned a scene).
fn try_and_compare(d: &Document, pos: &[(f64, f64)]) -> Option<bool> {
    match d {
        Document::Flowchart(g) | Document::State(g) => {
            scene::try_route(g, pos).map(|s| same(&s, &scene::route(g, pos)))
        }
        Document::Class(c) => class::try_route(c, pos).map(|s| same(&s, &class::route(c, pos))),
        Document::Er(e) => er::try_route(e, pos).map(|s| same(&s, &er::route(e, pos))),
        Document::Mindmap(m) => {
            mindmap::try_route(m, pos).map(|s| same(&s, &mindmap::route(m, pos)))
        }
        _ => None,
    }
}

/// At every count `route` accepts, on every fixture and example of every
/// diagram type with a `route`, `try_route` returns exactly `route`'s
/// scene.
#[test]
fn equals_route_at_accepted_counts() {
    let mut kinds = [0usize; 4];
    for (name, d) in repo_documents() {
        let Some((laid, counts)) = laid_out(&d) else {
            continue;
        };
        kinds[match d {
            Document::Flowchart(_) | Document::State(_) => 0,
            Document::Class(_) => 1,
            Document::Er(_) => 2,
            _ => 3,
        }] += 1;
        for k in counts {
            let pos = positions(&laid, k);
            assert_eq!(
                try_and_compare(&d, &pos),
                Some(true),
                "{name}: {k} positions"
            );
        }
    }
    // Every type is covered.
    assert!(kinds.iter().all(|&k| k > 0), "{kinds:?}");
}

/// Counts 0 to n + 3 (n = the drawing's node count) on several diagrams
/// of each type, empty ones included: a scene exactly at the counts
/// `route` accepts, `None` at every other, and never a panic.
#[test]
fn sweeps_counts_without_panicking() {
    let mut docs: Vec<(String, Document)> = [
        include_str!("../examples/demo.mmd"),
        include_str!("../examples/subgraph.mmd"),
        include_str!("../examples/lr.mmd"),
        include_str!("../examples/state.mmd"),
        include_str!("fixtures/notes/spread-face.mmd"),
        include_str!("../examples/class.mmd"),
        "classDiagram\nA <|-- B\nnote for A \"on A\"\nnote \"free\"",
        "classDiagram\nnote \"only a note\"",
        include_str!("../examples/er.mmd"),
        "erDiagram\nA ||--o{ B : has\nB }|--|| C : in",
        include_str!("../examples/mindmap.mmd"),
        "mindmap\n  root\n    a\n    b\n      c",
        "flowchart TD",
        "stateDiagram-v2",
        "classDiagram",
        "erDiagram",
    ]
    .iter()
    .map(|src| (src.lines().next().unwrap_or("").to_string(), parse(src)))
    .collect();
    docs.push((
        "Graph::default".into(),
        Document::Flowchart(Graph::default()),
    ));
    docs.push((
        "ClassDiagram::default".into(),
        Document::Class(ClassDiagram::default()),
    ));
    docs.push((
        "ErDiagram::default".into(),
        Document::Er(ErDiagram::default()),
    ));
    docs.push((
        "Mindmap::default".into(),
        Document::Mindmap(Mindmap::default()),
    ));
    for (name, d) in &docs {
        let (laid, counts) = laid_out(d).unwrap();
        for k in 0..laid.len() + 4 {
            let pos = positions(&laid, k);
            let got = try_and_compare(d, &pos);
            if counts.contains(&k) {
                assert_eq!(got, Some(true), "{name}: {k} positions");
            } else {
                assert_eq!(got, None, "{name}: {k} positions");
            }
        }
    }
}

/// A coordinate that is not finite gives no scene, at a count `route`
/// accepts too (the flowchart router panics on one).
#[test]
fn non_finite_positions_give_no_scene() {
    for src in [
        include_str!("../examples/demo.mmd"),
        include_str!("../examples/state.mmd"),
        include_str!("../examples/class.mmd"),
        include_str!("../examples/er.mmd"),
        include_str!("../examples/mindmap.mmd"),
    ] {
        let d = parse(src);
        let (laid, counts) = laid_out(&d).unwrap();
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for k in &counts {
                let mut pos = positions(&laid, *k);
                pos[k / 2].0 = bad;
                assert_eq!(try_and_compare(&d, &pos), None, "{src}: x = {bad}");
                pos[k / 2].0 = laid[k / 2].0;
                pos[0].1 = bad;
                assert_eq!(try_and_compare(&d, &pos), None, "{src}: y = {bad}");
            }
        }
    }
}

/// Positions anywhere within a million px of zero are routed, exactly as
/// `route` routes them and without a panic, by every type: every node in
/// one far corner, nodes spread across the whole range, one node far off
/// the rest. A coordinate past that reach, up to `f64::MAX`, gives no
/// scene (routing one can panic or exhaust memory).
#[test]
fn far_positions_route_within_reach_and_give_no_scene_past_it() {
    const REACH: f64 = 1.0e6;
    for src in [
        include_str!("../examples/demo.mmd"),
        include_str!("../examples/subgraph.mmd"),
        include_str!("../examples/state.mmd"),
        include_str!("../examples/class.mmd"),
        "classDiagram\nA <|-- B\nnote for A \"on A\"\nnote \"free\"",
        include_str!("../examples/er.mmd"),
        include_str!("../examples/mindmap.mmd"),
    ] {
        let d = parse(src);
        let (laid, counts) = laid_out(&d).unwrap();
        for k in counts {
            let at = positions(&laid, k);
            let mut one_off = at.clone();
            one_off[0] = (-REACH, REACH);
            let within: [Vec<(f64, f64)>; 4] = [
                vec![(REACH, REACH); k],
                vec![(-REACH, -REACH); k],
                (0..k)
                    .map(|i| match i % 4 {
                        0 => (-REACH, -REACH),
                        1 => (REACH, REACH),
                        2 => (-REACH, REACH),
                        _ => (REACH, -REACH),
                    })
                    .collect(),
                one_off,
            ];
            for pos in &within {
                assert_eq!(try_and_compare(&d, pos), Some(true), "{src}: {pos:?}");
            }
            for past in [REACH + 1.0, -REACH - 1.0, 1.0e12, f64::MAX, f64::MIN] {
                let mut pos = at.clone();
                pos[k - 1].0 = past;
                assert_eq!(try_and_compare(&d, &pos), None, "{src}: x = {past}");
                let mut pos = at.clone();
                pos[0].1 = past;
                assert_eq!(try_and_compare(&d, &pos), None, "{src}: y = {past}");
            }
            let all_far: Vec<(f64, f64)> = vec![(f64::MAX, f64::MIN); k];
            assert_eq!(try_and_compare(&d, &all_far), None, "{src}");
        }
    }
}

/// An empty mindmap with no positions is a scene; a mindmap whose
/// positions do not match is none, where `route` draws the empty scene
/// for both.
#[test]
fn mindmap_mismatch_differs_from_an_empty_mindmap() {
    let empty = Mindmap::default();
    let blank = mindmap::try_route(&empty, &[]).expect("an empty mindmap is a scene");
    assert!(blank.nodes.is_empty());
    assert!(same(&blank, &mindmap::route(&empty, &[])));
    let Document::Mindmap(m) = parse("mindmap\n  root\n    a") else {
        unreachable!()
    };
    assert!(mindmap::try_route(&m, &[]).is_none());
    assert!(mindmap::try_route(&m, &[(10.0, 10.0)]).is_none());
    assert!(same(&mindmap::route(&m, &[]), &blank));
}
