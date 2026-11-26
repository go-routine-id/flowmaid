//! A few back edges must not stretch the layering of a large flowchart.
//!
//! A layered graph (`LAYERS` x `WIDTH`, two forward edges per node) plus
//! a handful of edges from late layers back to early ones has the same
//! natural depth as without them: the back edges are the feedback arc
//! set, and reversing them restores the layering. A cycle breaker that
//! reverses forward edges instead made 50 back edges turn 50 layers
//! into 350 and the layout from 0.1 s into seconds.

use flowmaid::model::Document;
use flowmaid::parser::parse_document;

const LAYERS: usize = 50;
const WIDTH: usize = 20;

/// Deterministic LCG, so the graph is the same on every run.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

fn source(back_edges: usize) -> String {
    let mut rng = Lcg(1);
    let mut s = String::from("flowchart TD\n");
    for l in 0..LAYERS - 1 {
        for w in 0..WIDTH {
            for _ in 0..2 {
                s += &format!("N{l}_{w} --> N{}_{}\n", l + 1, rng.next(WIDTH));
            }
        }
    }
    for _ in 0..back_edges {
        let from = LAYERS / 2 + rng.next(LAYERS / 2);
        let to = rng.next(LAYERS / 2);
        s += &format!(
            "N{from}_{} --> N{to}_{}\n",
            rng.next(WIDTH),
            rng.next(WIDTH)
        );
    }
    s
}

fn layers(src: &str) -> usize {
    let Document::Flowchart(g) = parse_document(src).unwrap() else {
        panic!("not a flowchart")
    };
    let lo = flowmaid::layout::layout(&g);
    lo.nodes.iter().map(|p| p.layer).max().unwrap() + 1
}

#[test]
fn back_edges_do_not_stretch_the_layering() {
    let plain = layers(&source(0));
    let cyclic = layers(&source(50));
    assert!(
        cyclic <= plain * 3 / 2,
        "50 back edges took {plain} layers to {cyclic}"
    );
}

/// A 260-stage pipeline: every 5th stage retries 1-3 stages back, every
/// 40th rolls back to the start. The chain is what the drawing follows;
/// the loops sit beside it.
fn pipeline_retry() -> String {
    let mut s = String::from(
        "flowchart TD
",
    );
    for i in 0..259 {
        s += &format!(
            "S{i} --> S{}
",
            i + 1
        );
    }
    for i in (5..260).step_by(5) {
        s += &format!(
            "S{i} --> S{}
",
            i - 1 - (i / 5) % 3
        );
    }
    for i in (40..260).step_by(40) {
        s += &format!(
            "S{i} --> S0
"
        );
    }
    s
}

/// An approval flow: step -> decision -> next step; "rejected" redoes the
/// step, and every 10th decision can send the whole phase back.
fn approval_workflow() -> String {
    let mut s = String::from(
        "flowchart TD
",
    );
    for i in 0..140 {
        s += &format!(
            "W{i} --> D{i}
D{i} --> W{}
D{i} --> W{i}
",
            i + 1
        );
        if i % 10 == 9 {
            s += &format!(
                "D{i} --> W{}
",
                i - 9
            );
        }
    }
    s
}

// Graphs that are deep by nature stay as the walk lays them out: a
// greedy cycle breaker cuts such a chain where the walk cuts the loops
// and folds it into a handful of rows (pipeline 260 -> 10 layers,
// approval flow 281 -> 4).

#[test]
fn a_chain_with_retry_loops_keeps_its_depth() {
    let n = layers(&pipeline_retry());
    assert!(n >= 250, "the 260-stage chain was folded into {n} layers");
}

#[test]
fn an_approval_flow_with_loops_keeps_its_depth() {
    let n = layers(&approval_workflow());
    assert!(n >= 250, "the 140-step flow was folded into {n} layers");
}
