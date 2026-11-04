//! Seeded generator for the synthetic quality fixtures.
//!
//! `cargo run --example fixture_gen -- <kind> <seed> <TB|LR>` prints one
//! flowchart. `kind` is `dense` (nested frames, a labelled hub fan-out,
//! long cross-frame edges and back-edges into the hub) or `fanout` (one
//! source fanning out to frames of very different heights and plain
//! nodes). The topology, nesting, label lengths and fan-outs come from
//! the seed alone, so the fixtures under `tests/fixtures/quality/` are
//! reproducible and model no real diagram:
//!
//! - `dense-td.mmd`: `dense 1 TB`
//! - `dense-lr.mmd`: `dense 17 LR`
//! - `fanout-frames.mmd`: `fanout 7 LR`

use std::fmt::Write;

/// Filler words for labels.
const WORDS: &[&str] = &[
    "lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet",
    "consectetur",
    "adipiscing",
    "elit",
    "sed",
    "do",
    "eiusmod",
    "tempor",
    "incididunt",
    "labore",
    "magna",
    "aliqua",
    "veniam",
    "nostrud",
    "ullamco",
    "laboris",
];

/// Top-level frames of a `dense` diagram: a random count in this range.
const DENSE_TOP_FRAMES: (u64, u64) = (3, 5);
/// Deepest frame nesting of a `dense` diagram (top level is depth 1).
const DENSE_MAX_DEPTH: usize = 3;
/// Odds (in 100) that a `dense` frame gets a child frame, per try.
const DENSE_CHILD_ODDS: u64 = 45;
/// Child-frame tries per `dense` frame.
const DENSE_CHILD_TRIES: usize = 2;
/// Member nodes per `dense` frame: a random count in this range.
const DENSE_MEMBERS: (u64, u64) = (2, 7);
/// Free nodes outside any frame in a `dense` diagram.
const DENSE_FREE: (u64, u64) = (2, 5);
/// Labelled out-edges of the hub node.
const DENSE_HUB_FAN: (u64, u64) = (12, 16);
/// Back-edges into the hub from later nodes.
const DENSE_BACK: (u64, u64) = (3, 6);
/// Long edges between different top-level frames.
const DENSE_CROSS: (u64, u64) = (10, 16);
/// Edges inside one frame (chains between its members).
const DENSE_LOCAL: (u64, u64) = (12, 20);
/// Edges ending on a frame instead of a node.
const DENSE_TO_FRAME: (u64, u64) = (2, 4);
/// Odds (in 100) that a non-hub edge carries a label.
const EDGE_LABEL_ODDS: u64 = 35;
/// Odds (in 100) that an edge is drawn dotted.
const DOTTED_ODDS: u64 = 15;
/// Frames of a `fanout` diagram.
const FANOUT_FRAMES: (u64, u64) = (4, 6);
/// Members per `fanout` frame: the spread gives very different heights.
const FANOUT_MEMBERS: (u64, u64) = (1, 8);
/// Plain nodes a `fanout` source also points at.
const FANOUT_PLAIN: (u64, u64) = (1, 3);

/// xorshift64*: small, deterministic, good enough for fixtures.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in the inclusive range.
    fn range(&mut self, (lo, hi): (u64, u64)) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
    fn odds(&mut self, pct: u64) -> bool {
        self.next() % 100 < pct
    }
    fn pick(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    /// `lines` lines of 1-4 filler words.
    fn text(&mut self, lines: (u64, u64)) -> String {
        let n = self.range(lines);
        let mut out = Vec::new();
        for _ in 0..n {
            let w = self.range((1, 4));
            let line: Vec<&str> = (0..w).map(|_| WORDS[self.pick(WORDS.len())]).collect();
            out.push(line.join(" "));
        }
        out.join("<br/>")
    }
}

struct Frame {
    id: String,
    members: Vec<usize>,
    kids: Vec<Frame>,
}

fn node_decl(r: &mut Rng, id: usize) -> String {
    let t = r.text((1, 3));
    match r.pick(6) {
        0 => format!("n{id}([\"{t}\"])"),
        1 => format!("n{id}[(\"{t}\")]"),
        _ => format!("n{id}[\"{t}\"]"),
    }
}

/// Grows a frame tree, numbering nodes in declaration order.
fn grow(r: &mut Rng, depth: usize, next_node: &mut usize, next_frame: &mut usize) -> Frame {
    *next_frame += 1;
    let id = format!("g{next_frame}");
    let mut members = Vec::new();
    for _ in 0..r.range(DENSE_MEMBERS) {
        *next_node += 1;
        members.push(*next_node);
    }
    let mut kids = Vec::new();
    if depth < DENSE_MAX_DEPTH {
        for _ in 0..DENSE_CHILD_TRIES {
            if r.odds(DENSE_CHILD_ODDS) {
                kids.push(grow(r, depth + 1, next_node, next_frame));
            }
        }
    }
    Frame { id, members, kids }
}

fn write_frame(out: &mut String, r: &mut Rng, f: &Frame, indent: usize) {
    let pad = " ".repeat(indent);
    let title = r.text((1, 2));
    let _ = writeln!(out, "{pad}subgraph {}[\"{title}\"]", f.id);
    if r.odds(50) {
        let _ = writeln!(out, "{pad}    direction TB");
    }
    for &m in &f.members {
        let _ = writeln!(out, "{pad}    {}", node_decl(r, m));
    }
    for k in &f.kids {
        write_frame(out, r, k, indent + 4);
    }
    let _ = writeln!(out, "{pad}end");
}

/// The direct member list of every frame of a tree.
fn members(f: &Frame, out: &mut Vec<Vec<usize>>) {
    out.push(f.members.clone());
    for k in &f.kids {
        members(k, out);
    }
}

/// Every node of a frame tree, and every frame id.
fn collect(f: &Frame, nodes: &mut Vec<usize>, frames: &mut Vec<String>) {
    nodes.extend(&f.members);
    frames.push(f.id.clone());
    for k in &f.kids {
        collect(k, nodes, frames);
    }
}

fn arrow(r: &mut Rng, label: Option<String>) -> String {
    let dotted = r.odds(DOTTED_ODDS);
    match (dotted, label) {
        (false, None) => "-->".into(),
        (true, None) => "-.->".into(),
        (false, Some(l)) => format!("-->|{l}|"),
        (true, Some(l)) => format!("-.->|{l}|"),
    }
}

fn edge_label(r: &mut Rng, odds: u64) -> Option<String> {
    r.odds(odds).then(|| {
        let w = r.range((1, 3));
        (0..w)
            .map(|_| WORDS[r.pick(WORDS.len())])
            .collect::<Vec<_>>()
            .join(" ")
    })
}

fn dense(r: &mut Rng, dir: &str) -> String {
    let (mut nn, mut nf) = (0, 0);
    let tops: Vec<Frame> = (0..r.range(DENSE_TOP_FRAMES))
        .map(|_| grow(r, 1, &mut nn, &mut nf))
        .collect();
    let free: Vec<usize> = (0..r.range(DENSE_FREE))
        .map(|_| {
            nn += 1;
            nn
        })
        .collect();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "%% Generated by examples/fixture_gen.rs (seeded): nested frames, a labelled hub \
         fan-out, long cross-frame edges, back-edges into the hub."
    );
    let _ = writeln!(out, "flowchart {dir}");
    for &n in &free {
        let _ = writeln!(out, "    {}", node_decl(r, n));
    }
    let _ = writeln!(out);
    for f in &tops {
        write_frame(&mut out, r, f, 4);
        let _ = writeln!(out);
    }
    let per_top: Vec<(Vec<usize>, Vec<String>)> = tops
        .iter()
        .map(|f| {
            let (mut ns, mut fs) = (Vec::new(), Vec::new());
            collect(f, &mut ns, &mut fs);
            (ns, fs)
        })
        .collect();
    let all: Vec<usize> = (1..=nn).collect();
    let hub = per_top[0].0[r.pick(per_top[0].0.len())];
    let mut seen = std::collections::BTreeSet::new();
    let mut edges = Vec::new();
    let mut push = |edges: &mut Vec<String>, r: &mut Rng, a: String, b: String, odds: u64| {
        if a != b && seen.insert((a.clone(), b.clone())) {
            let label = edge_label(r, odds);
            let arr = arrow(r, label);
            edges.push(format!("    {a} {arr} {b}"));
        }
    };
    // Edges mostly run forward in declaration order (a flow), so the
    // layout has a direction to follow; back-edges are only the ones
    // into the hub.
    let forward = |r: &mut Rng, from: usize| -> usize {
        let later: Vec<usize> = all.iter().copied().filter(|&n| n > from).collect();
        if later.is_empty() {
            all[r.pick(all.len())]
        } else {
            later[r.pick(later.len())]
        }
    };
    for _ in 0..r.range(DENSE_HUB_FAN) {
        let to = forward(r, hub);
        push(&mut edges, r, format!("n{hub}"), format!("n{to}"), 100);
    }
    for _ in 0..r.range(DENSE_BACK) {
        let from = forward(r, hub);
        push(&mut edges, r, format!("n{from}"), format!("n{hub}"), 0);
    }
    for _ in 0..r.range(DENSE_CROSS) {
        let a = r.pick(per_top.len() - 1);
        let b = a + 1 + r.pick(per_top.len() - 1 - a);
        let x = per_top[a].0[r.pick(per_top[a].0.len())];
        let y = per_top[b].0[r.pick(per_top[b].0.len())];
        push(
            &mut edges,
            r,
            format!("n{x}"),
            format!("n{y}"),
            EDGE_LABEL_ODDS,
        );
    }
    let mut direct = Vec::new();
    for f in &tops {
        members(f, &mut direct);
    }
    for _ in 0..r.range(DENSE_LOCAL) {
        let t = &direct[r.pick(direct.len())];
        if t.len() < 2 {
            continue;
        }
        let i = r.pick(t.len());
        let j = r.pick(t.len());
        let (x, y) = (t[i.min(j)], t[i.max(j)]);
        push(
            &mut edges,
            r,
            format!("n{x}"),
            format!("n{y}"),
            EDGE_LABEL_ODDS,
        );
    }
    for &f in &free {
        let to = all[r.pick(all.len())];
        push(
            &mut edges,
            r,
            format!("n{f}"),
            format!("n{to}"),
            EDGE_LABEL_ODDS,
        );
    }
    for _ in 0..r.range(DENSE_TO_FRAME) {
        let t = &per_top[r.pick(per_top.len())].1;
        let g = t[r.pick(t.len())].clone();
        let from = free[r.pick(free.len())];
        push(&mut edges, r, format!("n{from}"), g, 0);
    }
    for e in edges {
        let _ = writeln!(out, "{e}");
    }
    out
}

fn fanout(r: &mut Rng, dir: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "%% Generated by examples/fixture_gen.rs (seeded): one source fanning out to frames \
         of very different heights and to plain nodes."
    );
    let _ = writeln!(out, "flowchart {dir}");
    let _ = writeln!(out, "    {}", node_decl(r, 0));
    let mut next = 0;
    let frames = r.range(FANOUT_FRAMES);
    for g in 1..=frames {
        let _ = writeln!(out);
        let title = r.text((1, 1));
        let _ = writeln!(out, "    subgraph g{g}[\"{title}\"]");
        let _ = writeln!(out, "        direction TB");
        for _ in 0..r.range(FANOUT_MEMBERS) {
            next += 1;
            let _ = writeln!(out, "        {}", node_decl(r, next));
        }
        let _ = writeln!(out, "    end");
    }
    let plain: Vec<usize> = (0..r.range(FANOUT_PLAIN))
        .map(|_| {
            next += 1;
            next
        })
        .collect();
    let _ = writeln!(out);
    for &p in &plain {
        let _ = writeln!(out, "    {}", node_decl(r, p));
    }
    let _ = writeln!(out);
    for g in 1..=frames {
        let arr = arrow(r, None);
        let _ = writeln!(out, "    n0 {arr} g{g}");
    }
    for &p in &plain {
        let arr = arrow(r, None);
        let _ = writeln!(out, "    n0 {arr} n{p}");
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [kind, seed, dir] = args.as_slice() else {
        eprintln!("usage: fixture_gen <dense|fanout> <seed> <TB|LR>");
        std::process::exit(2);
    };
    let seed: u64 = seed.parse().expect("numeric seed");
    // Mix the seed so small seeds do not start from a near-zero state.
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let out = match kind.as_str() {
        "dense" => dense(&mut r, dir),
        "fanout" => fanout(&mut r, dir),
        _ => {
            eprintln!("unknown kind {kind}");
            std::process::exit(2);
        }
    };
    print!("{out}");
}
