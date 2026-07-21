//! Graph → mermaid text — the inverse of [`crate::parser::parse`]
//! (issue #18).
//!
//! [`to_mermaid`] emits **canonical** mermaid flowchart source for a
//! [`Graph`]: parsing the output yields a graph semantically equal to
//! the input (same nodes by id/label/shape/style, same edges in
//! order, same subgraph tree). It does NOT preserve a human author's
//! original formatting, comments, or line order — it is the persist
//! step for **editor** consumers (`hit_test` → `route_partial` →
//! mutate → `to_mermaid`), not a source-code formatter.
//!
//! Characters mermaid's grammar reserves (`"` anywhere, `|` in edge
//! labels, a `#` that would read as an entity) are written as
//! mermaid entity codes (`#quot;`, `#124;`, `#35;`) — valid standard
//! mermaid, decoded back by the parser, so those labels round-trip
//! **losslessly**. The honest limits that remain are mermaid's own:
//!
//! - Labels round-trip modulo the parser's normalization: outer
//!   whitespace is trimmed, `<br/>` becomes `\n` (we emit `\n` back
//!   as `<br/>`). Edge labels and subgraph titles are single-line
//!   by grammar: embedded newlines flatten to spaces.
//! - An **empty or whitespace-only edge label** normalizes away to
//!   no label at all.
//! - Node ids must be word-like (`[A-Za-z0-9_]+`, the only form the
//!   parser reads back). Ids that equal a mermaid keyword (`end`,
//!   `style`, …) re-parse fine in flowmaid (we emit them in full
//!   `id["label"]` form) but standard mermaid.js rejects them — a
//!   long-standing mermaid limitation, not something text can fix.
//! - A node id must not collide with a subgraph id, and a node may
//!   be a direct member of at most one subgraph — both are
//!   unrepresentable in mermaid text (the parser guarantees graphs
//!   it produces obey them; debug builds assert it).
//! - stateDiagram-only shapes (`StateStart`, `StateEnd`, `ForkBar`)
//!   have no flowchart syntax; they degrade to a circle / rect.
//! - A non-finite `stroke_width` is dropped.

use crate::model::{Direction, EdgeKind, End, Graph, NodeStyle, Shape};

/// Emit canonical mermaid flowchart text for `g`. See the module
/// docs for the round-trip contract and its (small) honest limits.
pub fn to_mermaid(g: &Graph) -> String {
    #[cfg(debug_assertions)]
    check_preconditions(g);

    let mut out = String::new();
    out.push_str("flowchart ");
    out.push_str(dir_token(g.direction));
    out.push('\n');

    // ── 1. Node declarations (index order). Top-level mentions
    //    never claim subgraph membership, so declaring everything
    //    here is safe; the subgraph blocks below only need bare
    //    ids. Bare form when nothing but the id needs saying. ──
    for i in 0..g.nodes.len() {
        out.push_str("    ");
        out.push_str(&node_ref(g, i));
        out.push('\n');
    }

    // ── 2. Subgraph blocks, parents before children (membership
    //    and nesting both come from block structure). A parent
    //    cycle or dangling index leaves a block unreachable from
    //    the root walk; a second pass emits those as roots rather
    //    than silently dropping them. ──
    let mut visited = vec![false; g.subgraphs.len()];
    for r in 0..g.subgraphs.len() {
        if g.subgraphs[r].parent.is_none() {
            emit_subgraph(g, r, 1, &mut visited, &mut out);
        }
    }
    for r in 0..g.subgraphs.len() {
        if !visited[r] {
            emit_subgraph(g, r, 1, &mut visited, &mut out);
        }
    }

    // ── 3. Edges (insertion order — re-parse preserves it). ──
    for e in &g.edges {
        out.push_str("    ");
        out.push_str(&endpoint(g, e.from));
        push_edge_op(&mut out, e.kind, e.label.as_deref());
        out.push_str(&endpoint(g, e.to));
        out.push('\n');
    }
    // Sub-edges (whole-cluster endpoints). The parser pre-scans
    // subgraph ids, so these parse the same anywhere in the file.
    for e in &g.sub_edges {
        out.push_str("    ");
        out.push_str(&end_ref(g, e.from));
        push_edge_op(&mut out, e.kind, e.label.as_deref());
        out.push_str(&end_ref(g, e.to));
        out.push('\n');
    }

    // ── 4. Styling: one `style` line per styled node. classDef /
    //    class assignments were already resolved onto `Node.style`
    //    at parse time, so this round-trips them too. ──
    for (i, n) in g.nodes.iter().enumerate() {
        if let Some(props) = style_props(&n.style) {
            out.push_str("    style ");
            out.push_str(&g.nodes[i].id);
            out.push(' ');
            out.push_str(&props);
            out.push('\n');
        }
    }
    out
}

/// The unrepresentable-in-mermaid graph shapes (see module docs).
/// Debug builds catch them at the source instead of shipping text
/// that re-parses into something else.
#[cfg(debug_assertions)]
fn check_preconditions(g: &Graph) {
    for n in &g.nodes {
        debug_assert!(
            !g.subgraphs.iter().any(|s| s.id == n.id),
            "to_mermaid: node id {:?} collides with a subgraph id — unrepresentable in mermaid",
            n.id
        );
    }
    let mut seen = std::collections::HashSet::new();
    for s in &g.subgraphs {
        for &m in &s.nodes {
            debug_assert!(
                seen.insert(m),
                "to_mermaid: node {:?} is a direct member of two subgraphs — unrepresentable",
                g.nodes.get(m).map(|n| n.id.as_str()).unwrap_or("?")
            );
        }
    }
}

fn dir_token(d: Direction) -> &'static str {
    match d {
        Direction::TD => "TD",
        Direction::LR => "LR",
        Direction::RL => "RL",
        Direction::BT => "BT",
    }
}

/// Statement-leading keywords a bare node id must not collide with.
/// The parser matches these case-insensitively (`strip_keyword`), so
/// the check here is case-insensitive too — `Class --> b` would
/// otherwise be eaten by the `class` statement branch.
const RESERVED: &[&str] = &[
    "end", "subgraph", "direction", "style", "classDef", "class", "flowchart", "graph",
];

fn is_reserved(id: &str) -> bool {
    RESERVED.iter().any(|k| k.eq_ignore_ascii_case(id))
}

/// Escape a label for mermaid: `#` that would itself read as an
/// entity becomes `#35;`, then `"` (and, where the grammar needs
/// it, extra characters like `|`) become entities. The parser's
/// `decode_entities` is the exact inverse.
fn encode(label: &str, extra: &[char]) -> String {
    let mut out = String::with_capacity(label.len());
    let mut rest = label;
    while let Some(h) = rest.find('#') {
        out.push_str(&rest[..h]);
        let tail = &rest[h + 1..];
        let digits = tail.chars().take_while(|c| c.is_ascii_digit()).count();
        let is_entity =
            tail.starts_with("quot;") || (digits > 0 && tail[digits..].starts_with(';'));
        out.push_str(if is_entity { "#35;" } else { "#" });
        rest = tail;
    }
    out.push_str(rest);
    let mut out = out.replace('"', "#quot;");
    for &c in extra {
        out = out.replace(c, &format!("#{};", c as u32));
    }
    out
}

/// A node reference for a declaration or edge endpoint: the bare id
/// when that already says everything (default rect whose label is
/// the id), else the full `id["label"]` form. Reserved ids always
/// take the full form so the line can't read as a keyword statement
/// (`end` alone would close a subgraph block).
fn node_ref(g: &Graph, i: usize) -> String {
    let n = &g.nodes[i];
    let bare_ok = n.label == n.id && matches!(n.shape, Shape::Rect) && !is_reserved(&n.id);
    if bare_ok {
        return n.id.clone();
    }
    let (open, close) = shape_delims(n.shape);
    format!("{}{}{}{}", n.id, open, label_text(&n.label), close)
}

/// Edge endpoints just name the node; the declaration section
/// already carried label/shape. Reserved ids repeat the full form
/// (harmless — same label/shape — and keeps the line unambiguous).
fn endpoint(g: &Graph, i: usize) -> String {
    let n = &g.nodes[i];
    if is_reserved(&n.id) {
        node_ref(g, i)
    } else {
        n.id.clone()
    }
}

fn end_ref(g: &Graph, e: End) -> String {
    match e {
        End::Node(v) => endpoint(g, v),
        End::Sub(s) => g.subgraphs[s].id.clone(),
    }
}

/// The opener/closer pair mermaid uses for a shape. stateDiagram
/// pseudostates have no flowchart form — nearest look-alike.
fn shape_delims(s: Shape) -> (&'static str, &'static str) {
    match s {
        Shape::Rect | Shape::ForkBar => ("[", "]"),
        Shape::Rounded => ("(", ")"),
        Shape::Stadium => ("([", "])"),
        Shape::Diamond => ("{", "}"),
        Shape::Circle | Shape::StateStart | Shape::StateEnd => ("((", "))"),
        Shape::DoubleCircle => ("(((", ")))"),
        Shape::Cylinder => ("[(", ")]"),
        Shape::Subroutine => ("[[", "]]"),
        Shape::Hexagon => ("{{", "}}"),
        Shape::Parallelogram => ("[/", "/]"),
        Shape::ParallelogramAlt => ("[\\", "\\]"),
    }
}

/// A label as written between shape delimiters — always the quoted
/// form (quotes protect brackets, pipes, edge-operator lookalikes),
/// with `"`/entity-`#` escaped as entities so the quoted form can
/// hold ANY text. Newlines become `<br/>`; an empty label becomes a
/// single space (mermaid.js rejects `[""]`; the parser trims it
/// back to empty).
fn label_text(label: &str) -> String {
    if label.trim().is_empty() {
        return "\" \"".to_string();
    }
    format!("\"{}\"", encode(&label.replace('\n', "<br/>"), &[]))
}

/// `-->` / `-.->` / `==>` / … plus the `|label|` slot. Edge labels
/// are single-line by grammar (newlines flatten to spaces) and `|`
/// is the slot delimiter, so it rides as `#124;`. An empty or
/// whitespace-only label normalizes away to no label.
fn push_edge_op(out: &mut String, kind: EdgeKind, label: Option<&str>) {
    let op = match kind {
        EdgeKind::Arrow => "-->",
        EdgeKind::Open => "---",
        EdgeKind::Dotted => "-.->",
        EdgeKind::DottedOpen => "-.-",
        EdgeKind::Thick => "==>",
        EdgeKind::ThickOpen => "===",
        EdgeKind::Invisible => "~~~",
    };
    out.push(' ');
    out.push_str(op);
    if let Some(l) = label {
        if !l.trim().is_empty() {
            out.push('|');
            out.push_str(&encode(&l.replace('\n', " "), &['|']));
            out.push('|');
        }
    }
    out.push(' ');
}

/// One `subgraph … end` block, recursing into children. Members are
/// bare ids (their declarations already ran at top level; mentioning
/// them inside the block claims them for it). Titles are single-line
/// and live in `["…"]`, entity-escaped like any label; a title equal
/// to the id needs no bracket.
fn emit_subgraph(g: &Graph, si: usize, depth: usize, visited: &mut [bool], out: &mut String) {
    if visited[si] {
        return;
    }
    visited[si] = true;
    let s = &g.subgraphs[si];
    let pad = "    ".repeat(depth);
    out.push_str(&pad);
    out.push_str("subgraph ");
    out.push_str(&s.id);
    if s.title != s.id {
        out.push('[');
        out.push_str(&label_text(&s.title.replace('\n', " ")));
        out.push(']');
    }
    out.push('\n');
    if let Some(d) = s.direction {
        out.push_str(&pad);
        out.push_str("    direction ");
        out.push_str(dir_token(d));
        out.push('\n');
    }
    for &m in &s.nodes {
        out.push_str(&pad);
        out.push_str("    ");
        // Full form for reserved ids: a bare `end` line would close
        // the block right here.
        if is_reserved(&g.nodes[m].id) {
            out.push_str(&node_ref(g, m));
        } else {
            out.push_str(&g.nodes[m].id);
        }
        out.push('\n');
    }
    for c in 0..g.subgraphs.len() {
        if g.subgraphs[c].parent == Some(si) {
            emit_subgraph(g, c, depth + 1, visited, out);
        }
    }
    out.push_str(&pad);
    out.push_str("end\n");
}

/// `fill:…,stroke:…,stroke-width:…px,color:…` for the set fields,
/// in that fixed order; `None` when the style is all-default. A
/// non-finite stroke-width is dropped (nothing sane to write).
fn style_props(st: &NodeStyle) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(f) = &st.fill {
        parts.push(format!("fill:{}", f));
    }
    if let Some(s) = &st.stroke {
        parts.push(format!("stroke:{}", s));
    }
    if let Some(w) = st.stroke_width {
        if w.is_finite() {
            // f64 Display: `2` for 2.0, `1.5` for 1.5 — parses back
            // identically, no integer cast to saturate.
            parts.push(format!("stroke-width:{}px", w));
        }
    }
    if let Some(c) = &st.color {
        parts.push(format!("color:{}", c));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    /// Semantic equality — the round-trip contract from #18. Nodes
    /// by index (id, label, shape, style), edges by index resolved
    /// to ids, subgraphs by id (title, member ids, parent id,
    /// direction). Label comparison is modulo the parser's own
    /// normalization (trim; `<br/>` ⇒ `\n`), which parse applies to
    /// both sides equally.
    fn assert_roundtrip(g: &Graph) {
        let text = to_mermaid(g);
        let back = parse(&text).unwrap_or_else(|e| panic!("re-parse failed: {e}\n--\n{text}"));
        assert_eq!(back.direction, g.direction, "direction\n--\n{text}");
        assert_eq!(back.nodes.len(), g.nodes.len(), "node count\n--\n{text}");
        for (a, b) in g.nodes.iter().zip(&back.nodes) {
            assert_eq!(a.id, b.id, "node id\n--\n{text}");
            assert_eq!(a.label.trim(), b.label, "label of {}\n--\n{text}", a.id);
            assert_eq!(a.shape, b.shape, "shape of {}\n--\n{text}", a.id);
            assert_eq!(
                (&a.style.fill, &a.style.stroke, a.style.stroke_width, &a.style.color),
                (&b.style.fill, &b.style.stroke, b.style.stroke_width, &b.style.color),
                "style of {}\n--\n{text}",
                a.id
            );
        }
        let eid = |g: &Graph, i: usize| g.nodes[i].id.clone();
        assert_eq!(back.edges.len(), g.edges.len(), "edge count\n--\n{text}");
        for (a, b) in g.edges.iter().zip(&back.edges) {
            assert_eq!(eid(g, a.from), eid(&back, b.from), "edge from\n--\n{text}");
            assert_eq!(eid(g, a.to), eid(&back, b.to), "edge to\n--\n{text}");
            assert_eq!(a.kind, b.kind, "edge kind\n--\n{text}");
            // Single-line-slot normalization (documented): newlines
            // flatten to spaces, outer whitespace trims, empty → None.
            let norm = |l: &Option<String>| {
                l.as_deref()
                    .map(|s| s.replace('\n', " ").trim().to_string())
                    .filter(|s| !s.is_empty())
            };
            assert_eq!(norm(&a.label), norm(&b.label), "edge label\n--\n{text}");
        }
        assert_eq!(back.subgraphs.len(), g.subgraphs.len(), "subgraph count\n--\n{text}");
        for s in &g.subgraphs {
            let t = back
                .subgraphs
                .iter()
                .find(|t| t.id == s.id)
                .unwrap_or_else(|| panic!("subgraph {} lost\n--\n{text}", s.id));
            assert_eq!(s.title, t.title, "title of {}\n--\n{text}", s.id);
            let ids = |g: &Graph, m: &[usize]| {
                let mut v: Vec<String> = m.iter().map(|&i| g.nodes[i].id.clone()).collect();
                v.sort();
                v
            };
            assert_eq!(ids(g, &s.nodes), ids(&back, &t.nodes), "members of {}\n--\n{text}", s.id);
            let pid = |g: &Graph, p: Option<usize>| p.map(|i| g.subgraphs[i].id.clone());
            assert_eq!(pid(g, s.parent), pid(&back, t.parent), "parent of {}\n--\n{text}", s.id);
            assert_eq!(s.direction, t.direction, "direction of {}\n--\n{text}", s.id);
        }
    }

    /// Round-trip a source string: parse → emit → parse ≡ parse.
    fn assert_source_roundtrip(src: &str) {
        assert_roundtrip(&parse(src).unwrap());
    }

    #[test]
    fn every_shape_and_edge_kind_round_trips() {
        assert_source_roundtrip(
            "flowchart LR\n\
             A[rect] --> B(round) --> C([stadium]) --> D{diamond}\n\
             E((circle)) --> F(((double))) --> G[(db)] --> H[[sub]]\n\
             I{{hex}} --> J[/para/] --> K[\\alt\\]\n\
             A --- B\n A -.-> C\n A -.- D\n A ==> E\n A === F\n A ~~~ G\n",
        );
    }

    #[test]
    fn labels_with_brackets_pipes_and_breaks_round_trip() {
        assert_source_roundtrip(
            "flowchart TD\n\
             A[\"odd [text] here\"] -->|go / stop| B[\"a|b\"]\n\
             C[\"line<br/>break\"] --> D[\"he said 'hi'\"]\n",
        );
    }

    #[test]
    fn quotes_pipes_and_hashes_round_trip_losslessly_via_entities() {
        // The review-workflow findings: interior quotes, quote-wrapped
        // labels, quotes in edge labels, entity-lookalike text. All
        // must survive exactly — and the emitted text stays valid
        // standard mermaid (no raw '"' inside a quoted label).
        let mut g = Graph::default();
        let a = g.ensure_node("a", Some("he said \"hi\"".into()), Some(Shape::Circle));
        let b = g.ensure_node("b", Some("\"wrapped\"".into()), Some(Shape::Rect));
        let c = g.ensure_node("c", Some("#quot; literal #35; and #f00".into()), None);
        let d = g.ensure_node("d", Some("says \"hi\" :)".into()), Some(Shape::Circle));
        g.add_edge(a, b, Some("say \"go\"".into()), EdgeKind::Arrow);
        g.add_edge(b, c, Some("\"quoted label\"".into()), EdgeKind::Thick);
        g.add_edge(c, d, Some("a|b".into()), EdgeKind::Dotted);
        assert_roundtrip(&g);
        let text = to_mermaid(&g);
        // No raw double quote may appear inside a quoted label's body.
        assert!(text.contains("#quot;"), "quotes ride as entities:\n{text}");
        // Parser-sourced quotes too (source uses the unquoted form).
        assert_source_roundtrip("flowchart TD\nA[he said \"hi\"] --> B\n");
    }

    #[test]
    fn styles_and_class_assignments_round_trip_as_style_lines() {
        assert_source_roundtrip(
            "flowchart TD\n\
             A[x]:::hot --> B[y]\n\
             classDef hot fill:#f00,stroke:#900,stroke-width:3px\n\
             style B fill:#e4f5f4,color:#111,stroke-width:1.5px\n",
        );
        // CSS function values with commas (paren-aware props split).
        assert_source_roundtrip("flowchart TD\nA --> B\nstyle A fill:rgb(255,0,0),stroke:#900\n");
        // Extreme stroke widths must not saturate or panic.
        let mut g = Graph::default();
        let a = g.ensure_node("a", None, None);
        g.ensure_node("b", None, None);
        g.nodes[a].style.stroke_width = Some(1e19);
        g.add_edge(0, 1, None, EdgeKind::Arrow);
        assert_roundtrip(&g);
        // Non-finite widths are dropped, not written as "NaNpx".
        let mut st = NodeStyle::default();
        st.stroke_width = Some(f64::NAN);
        assert_eq!(style_props(&st), None);
    }

    #[test]
    fn nested_subgraphs_directions_titles_and_sub_edges_round_trip() {
        assert_source_roundtrip(
            "flowchart TD\n\
             Client --> GW\n\
             subgraph platform [The Platform]\n\
                 direction LR\n\
                 GW --> Auth\n\
                 subgraph inner\n\
                     Auth --> DB[(users)]\n\
                 end\n\
             end\n\
             Client --> platform\n\
             platform ==> Audit\n",
        );
        // A title with quotes rides as entities (parser-producible!).
        assert_source_roundtrip("flowchart TD\nsubgraph s[say \"hi\"]\nA\nend\n");
    }

    #[test]
    fn orphaned_subgraphs_are_still_emitted() {
        // A parent cycle (programmatic garbage) must not silently
        // drop blocks — they surface as root blocks instead.
        let mut g = Graph::default();
        let a = g.ensure_node("A", None, None);
        g.subgraphs.push(crate::model::Subgraph {
            id: "s0".into(),
            title: "s0".into(),
            nodes: vec![a],
            parent: Some(1),
            direction: None,
        });
        g.subgraphs.push(crate::model::Subgraph {
            id: "s1".into(),
            title: "s1".into(),
            nodes: vec![],
            parent: Some(0),
            direction: None,
        });
        let text = to_mermaid(&g);
        assert!(text.contains("subgraph s0") && text.contains("subgraph s1"), "{text}");
        assert!(parse(&text).is_ok(), "{text}");
    }

    #[test]
    fn reserved_word_ids_survive_case_insensitively() {
        // Parser keywords match case-insensitively, so `Class` and
        // `STYLE` are as dangerous as `class`/`style` — all must take
        // the full form. (`Class --> b` bare would be eaten by the
        // `class` statement branch and silently corrupt the graph.)
        for id in ["end", "Class", "STYLE", "Direction", "subGraph", "classDef"] {
            let mut g = Graph::default();
            let a = g.ensure_node(id, Some("checker".into()), None);
            let b = g.ensure_node("b", None, None);
            g.add_edge(a, b, Some("k".into()), EdgeKind::Arrow);
            g.subgraphs.push(crate::model::Subgraph {
                id: "box".into(),
                title: "box".into(),
                nodes: vec![a],
                parent: None,
                direction: None,
            });
            assert_roundtrip(&g);
        }
    }

    #[test]
    fn empty_labels_round_trip_and_avoid_invalid_mermaid() {
        let mut g = Graph::default();
        let a = g.ensure_node("a", Some(String::new()), Some(Shape::Rect));
        let b = g.ensure_node("b", None, None);
        g.add_edge(a, b, Some(String::new()), EdgeKind::Arrow);
        let text = to_mermaid(&g);
        // mermaid.js rejects [""] and -->|| — neither may appear.
        assert!(!text.contains("[\"\"]") && !text.contains("||"), "{text}");
        assert_roundtrip(&g);
    }

    #[test]
    fn programmatic_editor_graph_round_trips() {
        // The #18 story: a host mutates a Graph after drag/connect
        // and persists it.
        let mut g = Graph::default();
        g.direction = Direction::LR;
        let a = g.ensure_node("start", Some("Start".into()), Some(Shape::Stadium));
        let b = g.ensure_node("check", Some("pay_mode == cod".into()), Some(Shape::Diamond));
        let c = g.ensure_node("ship", Some("Ship it".into()), Some(Shape::Rect));
        g.add_edge(a, b, None, EdgeKind::Arrow);
        g.add_edge(b, c, Some("yes".into()), EdgeKind::Arrow);
        g.add_edge(b, a, Some("no".into()), EdgeKind::Dotted);
        g.nodes[c].style.fill = Some("#e4f5f4".into());
        g.nodes[c].style.stroke_width = Some(2.0);
        assert_roundtrip(&g);
    }

    #[test]
    fn fuzzed_graphs_round_trip() {
        // Deterministic LCG fuzz (pure std, reproducible): random
        // shapes, kinds, nasty labels, styles, nested subgraphs.
        struct Lcg(u64);
        impl Lcg {
            fn next(&mut self, n: usize) -> usize {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((self.0 >> 33) as usize) % n.max(1)
            }
        }
        let shapes = [
            Shape::Rect,
            Shape::Rounded,
            Shape::Stadium,
            Shape::Diamond,
            Shape::Circle,
            Shape::DoubleCircle,
            Shape::Cylinder,
            Shape::Subroutine,
            Shape::Hexagon,
            Shape::Parallelogram,
            Shape::ParallelogramAlt,
        ];
        let kinds = [
            EdgeKind::Arrow,
            EdgeKind::Open,
            EdgeKind::Dotted,
            EdgeKind::DottedOpen,
            EdgeKind::Thick,
            EdgeKind::ThickOpen,
            EdgeKind::Invisible,
        ];
        let labels = [
            "plain", "with space", "a[b]c", "p|q", "()", "{}", "-->", "==>", "x\ny",
            "he said \"hi\"", "\"wrapped\"", "says \"hi\" :)", "see \"n\" [1]", "#f00",
            "#quot; raw", "#65;", "subgraph", "end", "100%", "a & b", "C#",
        ];
        let mut rng = Lcg(7);
        for round in 0..60 {
            let mut g = Graph::default();
            g.direction = [Direction::TD, Direction::LR, Direction::RL, Direction::BT]
                [rng.next(4)];
            let n = 2 + rng.next(9);
            for i in 0..n {
                let id = format!("n{round}_{i}");
                let label = labels[rng.next(labels.len())].to_string();
                let ni = g.ensure_node(&id, Some(label), Some(shapes[rng.next(shapes.len())]));
                if rng.next(4) == 0 {
                    g.nodes[ni].style.fill = Some("#abc".into());
                    g.nodes[ni].style.stroke_width = Some(rng.next(5) as f64 + 0.5);
                }
            }
            for _ in 0..n {
                let (a, b) = (rng.next(n), rng.next(n));
                let label = if rng.next(3) == 0 {
                    Some(labels[rng.next(labels.len())].to_string())
                } else {
                    None
                };
                g.add_edge(a, b, label, kinds[rng.next(kinds.len())]);
            }
            // Up to two subgraphs, sometimes nested, disjoint members.
            let mut claimed: Vec<usize> = Vec::new();
            for si in 0..rng.next(3) {
                let mut mem: Vec<usize> = Vec::new();
                for i in 0..n {
                    if !claimed.contains(&i) && rng.next(3) == 0 {
                        mem.push(i);
                        claimed.push(i);
                    }
                }
                let parent = if si == 1 && rng.next(2) == 0 { Some(0) } else { None };
                g.subgraphs.push(crate::model::Subgraph {
                    id: format!("s{round}_{si}"),
                    title: format!("s{round}_{si}"),
                    nodes: mem,
                    parent,
                    direction: None,
                });
            }
            assert_roundtrip(&g);
        }
    }

    #[test]
    fn output_is_valid_standard_mermaid_shapes() {
        // Spot-check the emitted text itself (not just re-parse):
        // canonical tokens a mermaid.js renderer would accept.
        let mut g = Graph::default();
        let a = g.ensure_node("a", Some("A label".into()), Some(Shape::Stadium));
        let b = g.ensure_node("b", None, None);
        g.add_edge(a, b, Some("ok".into()), EdgeKind::Thick);
        let text = to_mermaid(&g);
        assert_eq!(
            text,
            "flowchart TD\n    a([\"A label\"])\n    b\n    a ==>|ok| b\n"
        );
    }
}
