//! Rendering identity: a recorded hash of everything the examples and
//! the test fixtures draw.
//!
//! `tests/fixtures/identity.tsv` holds, per `.mmd` file under
//! `examples/` and `tests/fixtures/`, the FNV-1a hash
//! ([`crate::scene::DefsKey`]: stable across toolchains, unlike
//! `DefaultHasher`) of its SVG, and for a flowchart or state diagram
//! also of the drawing [`crate::scene::route`] makes at the static
//! scene's centres and of the one [`crate::scene::route_partial`] makes
//! with the first node dragged by [`DRAG`]. The table was recorded
//! before the block layout of subgraphs was begun, and the test below
//! holds every output to it while that layout sits behind its switch:
//! with the switch off, nothing a user can draw changes by a byte.
//!
//! The last column says how the block layout sees the file: `blocks`
//! (a link crosses a subgraph's border: it will be laid out as blocks),
//! `unlinked` (subgraphs, none of them linked to anything outside it),
//! `flat` (no subgraph) or `-` (another diagram type). Every row but
//! the `blocks` ones is permanent: those drawings never change.
//!
//! To record the table afresh after an intended change, run
//! `FLOWMAID_RECORD_IDENTITY=1 cargo test --lib identity` and say in
//! the commit which rows moved and why.

use crate::model::Document;
use crate::scene::{route, route_partial, scene, to_svg, DefsKey};
use std::path::{Path, PathBuf};

/// How far the first node is dragged for the `route_partial` hash (px).
const DRAG: (f64, f64) = (7.0, 5.0);

/// The recorded table.
const RECORDED: &str = include_str!("../tests/fixtures/identity.tsv");

fn hash(svg: &str) -> String {
    let mut k = DefsKey::new();
    k.eat(svg.as_bytes());
    k.finish()
}

/// Every `.mmd` file under `dir`, at any depth.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "mmd") {
            out.push(p);
        }
    }
}

/// One row: the file (relative to the crate root), the three hashes
/// (`-` where the diagram type has no such drawing), and how the block
/// layout sees the file (module docs).
fn row(root: &Path, path: &Path) -> String {
    let src = std::fs::read_to_string(path).expect("read fixture");
    let rel = path.strip_prefix(root).expect("under the crate root");
    let rel = rel.to_string_lossy().replace('\\', "/");
    // The advance text examples have their own entry point.
    let svg = crate::render_svg(&src)
        .ok()
        .or_else(|| crate::render_advance_text_svg(&src).ok())
        .unwrap_or_else(|| panic!("{rel}: does not render"));
    let (mut routed, mut partial) = ("-".to_string(), "-".to_string());
    let mut class = "-";
    if let Ok(Document::Flowchart(g) | Document::State(g)) = crate::parser::parse_document(&src) {
        class = if g.subgraphs.is_empty() {
            "flat"
        } else if crate::scene::block_mode(&g) {
            "blocks"
        } else {
            "unlinked"
        };
        if !g.nodes.is_empty() {
            let base = scene(&g);
            let at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.x, n.y)).collect();
            routed = hash(&to_svg(&route(&g, &at)));
            let mut moved = at.clone();
            moved[0] = (at[0].0 + DRAG.0, at[0].1 + DRAG.1);
            partial = hash(&to_svg(&route_partial(&g, &moved, &base, &at)));
        }
    }
    format!("{rel}\t{}\t{routed}\t{partial}\t{class}", hash(&svg))
}

/// Every example and fixture, measured now.
fn table() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect(&root.join("examples"), &mut files);
    collect(&root.join("tests/fixtures"), &mut files);
    files.iter().map(|p| row(root, p)).collect()
}

/// With the block layout switched off (as every ordinary build has
/// it), every example and every fixture draws exactly what it drew
/// before that layout was begun: its SVG, its `route()` at the scene's
/// centres and a `route_partial()` drag frame.
#[test]
fn everything_renders_as_before_with_blocks_off() {
    let now = crate::levels::with_blocks(false, table);
    if std::env::var_os("FLOWMAID_RECORD_IDENTITY").is_some() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/identity.tsv");
        let head = RECORDED
            .lines()
            .take_while(|l| l.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, format!("{head}\n{}\n", now.join("\n"))).expect("write the table");
        return;
    }
    let recorded: Vec<&str> = RECORDED
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .collect();
    let name = |row: &str| row.split('\t').next().unwrap_or("").to_string();
    let mut bad = Vec::new();
    for r in &now {
        match recorded.iter().find(|x| name(x) == name(r)) {
            Some(x) if x == r => {}
            Some(x) => bad.push(format!("changed: {x}\n     now: {r}")),
            None => bad.push(format!("no recorded row: {r}")),
        }
    }
    for x in &recorded {
        if !now.iter().any(|r| name(r) == name(x)) {
            bad.push(format!("recorded, but the file is gone: {x}"));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} drawings differ from tests/fixtures/identity.tsv:\n{}",
        bad.len(),
        now.len(),
        bad.join("\n")
    );
}
