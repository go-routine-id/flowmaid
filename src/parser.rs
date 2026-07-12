//! Hand-written parser for Mermaid-like diagram syntax.
//!
//! Flowchart support:
//! - Header:  `flowchart TD|TB|LR|RL|BT`  or  `graph ...`
//! - Nodes:   `A`, `A[text]`, `A(text)`, `A([text])`, `A{text}`, `A((text))`
//! - Edges:   `-->`, `---`, `-.->`, `==>`, with labels `-->|text|`
//! - Chains:  `A --> B --> C`, `;` separator, `%%` comments
//!
//! Entity-Relationship support (`erDiagram` header):
//! - Relationships: `A ||--o{ B : "label"` with the full crow's foot
//!   cardinality tokens (`||`, `|o`/`o|`, `}o`/`o{`, `}|`/`|{`) and
//!   identifying (`--`) / non-identifying (`..`) lines
//! - Entity blocks: `Name { type name [PK|FK|UK] ["comment"] }`
//!
//! Use [`parse_document`] to accept any supported diagram type;
//! [`parse`] stays flowchart-only for backwards compatibility.

use crate::model::{
    Attr, Card, Direction, Document, EdgeKind, End, ErDiagram, Graph, Key, NodeStyle, Relation,
    Shape, SubEdge, Subgraph,
};
use std::collections::HashMap;

fn parse_direction(s: &str, lineno: usize) -> Result<Direction, ParseError> {
    match s.trim().to_uppercase().as_str() {
        "TD" | "TB" => Ok(Direction::TD),
        "LR" => Ok(Direction::LR),
        "RL" => Ok(Direction::RL),
        "BT" => Ok(Direction::BT),
        other => Err(err(lineno, format!("unknown direction: '{}'", other))),
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

fn err(line: usize, message: String) -> ParseError {
    ParseError { line, message }
}

/// Simple character cursor over a single line.
struct Cur<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Cur<'a> {
    fn new(s: &'a str) -> Self {
        Cur { s, pos: 0 }
    }
    fn rest(&self) -> &'a str {
        &self.s[self.pos..]
    }
    fn at_end(&self) -> bool {
        self.pos >= self.s.len()
    }
    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }
    fn eat(&mut self, prefix: &str) -> bool {
        if self.rest().starts_with(prefix) {
            self.pos += prefix.len();
            true
        } else {
            false
        }
    }
    /// Take text up to `close` (exclusive), then skip past `close`.
    fn take_until(&mut self, close: &str) -> Option<String> {
        let idx = self.rest().find(close)?;
        let out = self.rest()[..idx].to_string();
        self.pos += idx + close.len();
        Some(out)
    }
}

/// Strip wrapping double quotes from a label, Mermaid-style
/// (`A["odd text"]`), and turn `<br>` / `<br/>` / `<br />` line
/// breaks into real newlines (mermaid renders those as line breaks).
fn clean_label(s: &str) -> String {
    let t = s.trim();
    let inner = if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        &t[1..t.len() - 1]
    } else {
        t
    };
    // Trim leading/trailing blank lines from `<br/>` at the edges
    // (`"<br/>"` → "" not two empty lines).
    normalize_breaks(inner).trim_matches('\n').to_string()
}

/// Like [`clean_label`] but collapsed to a single line — for
/// contexts without multi-line layout (edge labels, subgraph
/// titles) whose boxes/strips are single-line.
fn clean_label_1line(s: &str) -> String {
    clean_label(s).replace('\n', " ")
}

/// Replace `<br>` variants (case-insensitive, optional `/` and
/// spaces) with `\n`. Everything else passes through unchanged.
fn normalize_breaks(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while !rest.is_empty() {
        // `get(..3)` is char-boundary-safe (returns None mid-emoji).
        if rest.get(..3).is_some_and(|h| h.eq_ignore_ascii_case("<br")) {
            if let Some(gt) = rest.find('>') {
                // Only a bare `<br ... >` (no attributes) is a break.
                if rest[3..gt].trim().trim_end_matches('/').trim().is_empty() {
                    out.push('\n');
                    rest = &rest[gt + 1..];
                    continue;
                }
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// Parse any supported Mermaid diagram type, dispatching on the
/// header line: `flowchart`/`graph` (or none) -> flowchart,
/// `erDiagram` -> ER. Other known Mermaid types produce an explicit
/// "not supported yet" error.
pub fn parse_document(source: &str) -> Result<Document, ParseError> {
    for (i, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        return match diagram_type(line) {
            Some("erDiagram") => parse_er(source, i + 1).map(Document::Er),
            Some(t) => Err(err(
                i + 1,
                format!(
                    "diagram type '{}' is not supported yet (supported: flowchart, graph, erDiagram)",
                    t
                ),
            )),
            None => parse(source).map(Document::Flowchart),
        };
    }
    Ok(Document::Flowchart(Graph::default()))
}

/// Parse a flowchart. For ER diagrams use [`parse_document`].
pub fn parse(source: &str) -> Result<Graph, ParseError> {
    let mut g = Graph::default();
    let mut header_seen = false;
    // Styling is collected during the pass and resolved at the end,
    // because Mermaid allows `classDef` to appear after its usage.
    let mut class_defs: HashMap<String, NodeStyle> = HashMap::new();
    let mut assigns: Vec<(usize, String)> = Vec::new(); // (node, class)
    let mut styles: Vec<(usize, NodeStyle)> = Vec::new(); // explicit `style` lines
    // Stack of open `subgraph` blocks; new nodes join the top one.
    let mut sub_stack: Vec<usize> = Vec::new();

    // Pre-scan subgraph ids so an edge may reference a subgraph
    // declared LATER (`A --> grp` before `subgraph grp`). The scan
    // order matches the index each block gets during the main pass.
    let mut sub_ids: HashMap<String, usize> = HashMap::new();
    {
        let mut idx = 0usize;
        for raw in source.lines() {
            if let Some(rest) = strip_keyword(raw.trim(), "subgraph") {
                if let Ok((id, _)) = parse_subgraph_header(rest.trim(), 0) {
                    sub_ids.entry(id).or_insert(idx);
                    idx += 1;
                }
            }
        }
    }

    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !header_seen {
            header_seen = true;
            if let Some(t) = diagram_type(line) {
                let hint = if t == "erDiagram" {
                    "this parser is flowchart-only — use parse_document() or render_svg()"
                } else {
                    "not supported yet (supported: flowchart, graph, erDiagram)"
                };
                return Err(err(lineno, format!("diagram type '{}': {}", t, hint)));
            }
            let rest = strip_keyword(line, "flowchart").or_else(|| strip_keyword(line, "graph"));
            if let Some(rest) = rest {
                g.direction = match rest.trim().to_uppercase().as_str() {
                    "" | "TD" | "TB" => Direction::TD,
                    "LR" => Direction::LR,
                    "RL" => Direction::RL,
                    "BT" => Direction::BT,
                    other => {
                        return Err(err(lineno, format!("unknown direction: '{}'", other)))
                    }
                };
                continue;
            }
            // No header: use the default direction (TD) and
            // treat this line as a regular statement.
        }

        // Subgraph blocks: `subgraph id [Title]` ... `end`, nestable,
        // with an optional `direction XX` line inside.
        if let Some(rest) = strip_keyword(line, "subgraph") {
            let (id, title) = parse_subgraph_header(rest.trim(), lineno)?;
            let idx = g.subgraphs.len();
            g.subgraphs.push(Subgraph {
                id,
                title,
                nodes: Vec::new(),
                parent: sub_stack.last().copied(),
                direction: None,
            });
            sub_stack.push(idx);
            continue;
        }
        if line == "end" && !sub_stack.is_empty() {
            sub_stack.pop();
            continue;
        }
        if let Some(rest) = strip_keyword(line, "direction") {
            let Some(&cur) = sub_stack.last() else {
                return Err(err(
                    lineno,
                    "'direction' is only valid inside a subgraph — use the \
                     flowchart header for the top-level direction"
                        .to_string(),
                ));
            };
            g.subgraphs[cur].direction = Some(parse_direction(rest, lineno)?);
            continue;
        }

        // Styling statements (`style A fill:...`, `classDef name ...`,
        // `class A,B name`). strip_keyword requires whitespace after
        // the keyword, so `class` can't shadow `classDef`.
        if let Some(rest) = strip_keyword(line, "classDef") {
            let rest = rest.trim();
            let (names, props) = rest.split_once(char::is_whitespace).ok_or_else(|| {
                err(lineno, "classDef needs a name and properties".to_string())
            })?;
            let st = parse_props(props.trim(), lineno)?;
            for name in names.split(',').filter(|n| !n.is_empty()) {
                class_defs.insert(name.to_string(), st.clone());
            }
            continue;
        }
        if let Some(rest) = strip_keyword(line, "class") {
            let rest = rest.trim();
            let (ids, name) = rest.split_once(char::is_whitespace).ok_or_else(|| {
                err(lineno, "class needs node ids and a class name".to_string())
            })?;
            for id in ids.split(',').filter(|i| !i.is_empty()) {
                let n = g.ensure_node(id.trim(), None, None);
                assigns.push((n, name.trim().to_string()));
            }
            continue;
        }
        if let Some(rest) = strip_keyword(line, "style") {
            let rest = rest.trim();
            let (id, props) = rest.split_once(char::is_whitespace).ok_or_else(|| {
                err(lineno, "style needs a node id and properties".to_string())
            })?;
            let n = g.ensure_node(id.trim(), None, None);
            styles.push((n, parse_props(props.trim(), lineno)?));
            continue;
        }

        parse_statement(&mut g, line, lineno, &mut assigns, &sub_stack, &sub_ids)?;
    }
    if let Some(&open) = sub_stack.last() {
        return Err(err(
            source.lines().count(),
            format!(
                "subgraph '{}' is never closed with 'end'",
                g.subgraphs[open].id
            ),
        ));
    }

    // Resolve styling: classDef layers first, explicit `style`
    // lines win. Unknown class names are ignored, mermaid-style.
    for (n, name) in assigns {
        if let Some(def) = class_defs.get(&name) {
            g.nodes[n].style.apply_over(def);
        }
    }
    for (n, st) in styles {
        g.nodes[n].style.apply_over(&st);
    }
    Ok(g)
}

/// Parse `fill:#f9f,stroke:#333,stroke-width:4px,color:#fff`.
/// Unknown properties are ignored (mermaid tolerates them too).
fn parse_props(s: &str, lineno: usize) -> Result<NodeStyle, ParseError> {
    let mut st = NodeStyle::default();
    for item in s.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let Some((k, v)) = item.split_once(':') else {
            return Err(err(
                lineno,
                format!("expected 'property:value', got '{}'", item),
            ));
        };
        let v = v.trim();
        match k.trim() {
            "fill" => st.fill = Some(v.to_string()),
            "stroke" => st.stroke = Some(v.to_string()),
            "color" => st.color = Some(v.to_string()),
            "stroke-width" => {
                let n: f64 = v.trim_end_matches("px").trim().parse().map_err(|_| {
                    err(lineno, format!("invalid stroke-width: '{}'", v))
                })?;
                st.stroke_width = Some(n);
            }
            _ => {}
        }
    }
    Ok(st)
}

/// Recognise a known Mermaid diagram-type header other than
/// flowchart/graph, so we can fail with a clear message instead of
/// parsing the header as a node. Longer tokens first
/// (`stateDiagram-v2` before `stateDiagram`).
fn diagram_type(line: &str) -> Option<&'static str> {
    const TYPES: &[&str] = &[
        "erDiagram",
        "sequenceDiagram",
        "classDiagram",
        "stateDiagram-v2",
        "stateDiagram",
        "gantt",
        "pie",
        "journey",
        "mindmap",
        "timeline",
    ];
    TYPES.iter().copied().find(|t| {
        line.get(..t.len()) == Some(*t)
            && line[t.len()..].chars().next().map_or(true, char::is_whitespace)
    })
}

/// Match a header keyword only when followed by whitespace or end of
/// line, so a node named e.g. `graphics` is not mistaken for a
/// `graph` header. Uses `get` to stay safe on UTF-8 boundaries.
fn strip_keyword<'a>(line: &'a str, kw: &str) -> Option<&'a str> {
    match line.get(..kw.len()) {
        Some(head) if head.eq_ignore_ascii_case(kw) => {
            let rest = &line[kw.len()..];
            if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                Some(rest)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// `subgraph one`, `subgraph one [Pretty Title]`, or a bare
/// multi-word title (`subgraph My group` → id == title).
fn parse_subgraph_header(rest: &str, lineno: usize) -> Result<(String, String), ParseError> {
    if rest.is_empty() {
        return Err(err(lineno, "subgraph needs an id or title".to_string()));
    }
    if let Some(bi) = rest.find('[') {
        let id = rest[..bi].trim();
        let inner = rest[bi + 1..]
            .strip_suffix(']')
            .ok_or_else(|| err(lineno, "subgraph title '[' is never closed".to_string()))?;
        if id.is_empty() {
            return Err(err(lineno, "subgraph title needs an id before '['".to_string()));
        }
        return Ok((id.to_string(), clean_label_1line(inner)));
    }
    Ok((rest.to_string(), rest.to_string()))
}

/// One or more nodes joined by `&` — mermaid's fan-out lists,
/// e.g. `A & B --> C & D`.
fn parse_node_list(
    cur: &mut Cur<'_>,
    g: &mut Graph,
    lineno: usize,
    assigns: &mut Vec<(usize, String)>,
    subs: &[usize],
    sub_ids: &HashMap<String, usize>,
) -> Result<Vec<End>, ParseError> {
    let mut nodes = vec![parse_node(cur, g, lineno, assigns, subs, sub_ids)?];
    loop {
        let save = cur.pos;
        cur.skip_ws();
        if cur.eat("&") {
            cur.skip_ws();
            nodes.push(parse_node(cur, g, lineno, assigns, subs, sub_ids)?);
        } else {
            cur.pos = save;
            break;
        }
    }
    Ok(nodes)
}

fn parse_statement(
    g: &mut Graph,
    line: &str,
    lineno: usize,
    assigns: &mut Vec<(usize, String)>,
    subs: &[usize],
    sub_ids: &HashMap<String, usize>,
) -> Result<(), ParseError> {
    let mut cur = Cur::new(line);
    let mut prevs = parse_node_list(&mut cur, g, lineno, assigns, subs, sub_ids)?;
    loop {
        cur.skip_ws();
        if cur.at_end() {
            break;
        }
        // `;` separates statements on one line: `A-->B; C-->D`
        if cur.eat(";") {
            cur.skip_ws();
            if cur.at_end() {
                break;
            }
            prevs = parse_node_list(&mut cur, g, lineno, assigns, subs, sub_ids)?;
            continue;
        }
        // Inline-labelled operators (`-- text -->`) carry their own
        // label; plain operators may take a `|text|` label after.
        let (kind, mut label) = parse_edge_inline_label(&mut cur, lineno)?
            .map(|(k, l)| (Some(k), Some(l)))
            .unwrap_or((None, None));
        let kind = match kind {
            Some(k) => k,
            None => parse_edge_op(&mut cur).ok_or_else(|| {
                err(
                    lineno,
                    format!("unknown edge operator near: '{}'", cur.rest()),
                )
            })?,
        };
        cur.skip_ws();
        if label.is_none() && cur.eat("|") {
            let l = cur.take_until("|").ok_or_else(|| {
                err(lineno, "edge label opened with '|' but never closed".to_string())
            })?;
            label = Some(clean_label_1line(&l));
        }
        cur.skip_ws();
        let nexts = parse_node_list(&mut cur, g, lineno, assigns, subs, sub_ids)?;
        // Fan-out: every prev connects to every next. Node→node
        // edges go to `edges`; anything touching a subgraph to
        // `sub_edges` (drawn against the cluster box).
        for &a in &prevs {
            for &b in &nexts {
                match (a, b) {
                    (End::Node(x), End::Node(y)) => g.add_edge(x, y, label.clone(), kind),
                    _ => g.sub_edges.push(SubEdge {
                        from: a,
                        to: b,
                        label: label.clone(),
                        kind,
                    }),
                }
            }
        }
        prevs = nexts;
    }
    Ok(())
}

/// Mermaid's inline edge labels: `-- text -->`, `-- text ---`,
/// `-. text .->`, `-. text .-`, `== text ==>`, `== text ===`.
/// Returns None when the cursor isn't at an inline-label operator
/// (plain operators like `-->` fall through to `parse_edge_op`).
fn parse_edge_inline_label(
    cur: &mut Cur<'_>,
    lineno: usize,
) -> Result<Option<(EdgeKind, String)>, ParseError> {
    // (opener, [(closer, kind)] — longest closer first)
    const FORMS: &[(&str, &[(&str, EdgeKind)])] = &[
        ("-.", &[(".->", EdgeKind::Dotted), (".-", EdgeKind::DottedOpen)]),
        ("==", &[("==>", EdgeKind::Thick), ("===", EdgeKind::ThickOpen)]),
        ("--", &[("-->", EdgeKind::Arrow), ("---", EdgeKind::Open)]),
    ];
    let rest = cur.rest();
    for (open, closers) in FORMS {
        if !rest.starts_with(open) {
            continue;
        }
        // Only label mode when the opener is followed by label text,
        // not by more operator characters (`-->`, `---`, `-.-`, ...).
        let after = &rest[open.len()..];
        let Some(c0) = after.chars().next() else { continue };
        if matches!(c0, '-' | '>' | '.' | '=') {
            continue;
        }
        // Find the nearest closer; longest first so `-->` wins over `---`…
        // (they can't overlap at the same index anyway, but keep order).
        let mut best: Option<(usize, &str, EdgeKind)> = None;
        for (close, kind) in *closers {
            if let Some(i) = after.find(close) {
                if best.map_or(true, |(bi, _, _)| i < bi) {
                    best = Some((i, close, *kind));
                }
            }
        }
        let Some((i, close, kind)) = best else {
            return Err(err(
                lineno,
                format!("inline edge label after '{}' is never closed", open),
            ));
        };
        let label = clean_label_1line(&after[..i]);
        cur.pos += open.len() + i + close.len();
        return Ok(Some((kind, label)));
    }
    Ok(None)
}

fn parse_node(
    cur: &mut Cur<'_>,
    g: &mut Graph,
    lineno: usize,
    assigns: &mut Vec<(usize, String)>,
    subs: &[usize],
    sub_ids: &HashMap<String, usize>,
) -> Result<End, ParseError> {
    cur.skip_ws();
    let start = cur.pos;
    while let Some(c) = cur.peek() {
        if c.is_alphanumeric() || c == '_' {
            cur.bump();
        } else {
            break;
        }
    }
    if cur.pos == start {
        return Err(err(
            lineno,
            format!("expected a node id, found: '{}'", cur.rest()),
        ));
    }
    let id = cur.s[start..cur.pos].to_string();

    // Check order matters: longest / most-specific openers first.
    let parsed: Option<(Shape, String)> = if cur.eat("(((") {
        Some((Shape::DoubleCircle, close(cur, ")))", lineno)?))
    } else if cur.eat("((") {
        Some((Shape::Circle, close(cur, "))", lineno)?))
    } else if cur.eat("([") {
        Some((Shape::Stadium, close(cur, "])", lineno)?))
    } else if cur.eat("[(") {
        Some((Shape::Cylinder, close(cur, ")]", lineno)?))
    } else if cur.eat("[[") {
        Some((Shape::Subroutine, close(cur, "]]", lineno)?))
    } else if cur.eat("[/") {
        Some((Shape::Parallelogram, close(cur, "/]", lineno)?))
    } else if cur.eat("[\\") {
        Some((Shape::ParallelogramAlt, close(cur, "\\]", lineno)?))
    } else if cur.eat("[") {
        Some((Shape::Rect, close(cur, "]", lineno)?))
    } else if cur.eat("{{") {
        Some((Shape::Hexagon, close(cur, "}}", lineno)?))
    } else if cur.eat("(") {
        Some((Shape::Rounded, close(cur, ")", lineno)?))
    } else if cur.eat("{") {
        Some((Shape::Diamond, close(cur, "}", lineno)?))
    } else {
        None
    };

    let (shape, label) = match parsed {
        Some((s, l)) => (Some(s), Some(clean_label(&l))),
        None => (None, None),
    };
    // An id that names a declared subgraph (pre-scanned, so
    // forward references work) is an edge endpoint on the whole
    // cluster (`CF --> VPC`). A subgraph id wins even if a shape
    // was written (`VPC[x]`) — you can't have a node and a
    // subgraph sharing an id — so no duplicate node is created.
    if g.node_index(&id).is_none() {
        if let Some(&si) = sub_ids.get(&id) {
            return Ok(End::Sub(si));
        }
    }
    let n = g.ensure_node(&id, label, shape);
    // Mermaid semantics: mentioning a node inside a `subgraph`
    // block claims it for that block (the canonical docs example
    // relies on this: `c1-->a2` at top level, then `a2` inside
    // `subgraph one` puts a2 in the box). Top-level mentions never
    // un-claim.
    if let Some(&owner) = subs.last() {
        if !g.subgraphs[owner].nodes.contains(&n) {
            for s in g.subgraphs.iter_mut() {
                s.nodes.retain(|&m| m != n);
            }
            g.subgraphs[owner].nodes.push(n);
        }
    }

    // `A:::className` — inline class assignment shorthand.
    if cur.eat(":::") {
        let start = cur.pos;
        while let Some(c) = cur.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                cur.bump();
            } else {
                break;
            }
        }
        if cur.pos == start {
            return Err(err(lineno, "expected a class name after ':::'".to_string()));
        }
        assigns.push((n, cur.s[start..cur.pos].to_string()));
    }
    Ok(End::Node(n))
}

fn close(cur: &mut Cur<'_>, closer: &str, lineno: usize) -> Result<String, ParseError> {
    // Quoted label: `A["odd [text]"]` — quotes protect bracket characters.
    if cur.rest().starts_with('"') {
        cur.bump();
        let inner = cur
            .take_until("\"")
            .ok_or_else(|| err(lineno, "unclosed label quote".to_string()))?;
        cur.skip_ws();
        if !cur.eat(closer) {
            return Err(err(lineno, format!("closing '{}' not found", closer)));
        }
        return Ok(inner);
    }
    cur.take_until(closer)
        .ok_or_else(|| err(lineno, format!("closing '{}' not found", closer)))
}

/// Recognise an edge operator and advance the cursor. Tolerant of
/// extra length (`--->`, `-..->`, `===>`).
fn parse_edge_op(cur: &mut Cur<'_>) -> Option<EdgeKind> {
    let rest = cur.rest();

    // Invisible link: ~~~ (layout-only, never drawn).
    if rest.starts_with("~~~") {
        let tildes = rest.chars().take_while(|&c| c == '~').count();
        cur.pos += tildes;
        return Some(EdgeKind::Invisible);
    }

    // Dotted: -.-> / -..-> (arrow)  or  -.- / -..- (open)
    if let Some(after) = rest.strip_prefix("-.") {
        let dots = after.chars().take_while(|&c| c == '.').count();
        let tail = &after[dots..];
        if tail.starts_with("->") {
            cur.pos += 2 + dots + 2;
            return Some(EdgeKind::Dotted);
        }
        if tail.starts_with('-') {
            cur.pos += 2 + dots + 1;
            return Some(EdgeKind::DottedOpen);
        }
        return None;
    }

    // Thick: ==> / ===> (arrow)  or  === (open)
    if rest.starts_with("==") {
        let eqs = rest.chars().take_while(|&c| c == '=').count();
        let tail = &rest[eqs..];
        if tail.starts_with('>') {
            cur.pos += eqs + 1;
            return Some(EdgeKind::Thick);
        }
        if eqs >= 3 {
            cur.pos += eqs;
            return Some(EdgeKind::ThickOpen);
        }
        return None;
    }

    // Regular arrow (-->) or plain line (---)
    if rest.starts_with('-') {
        let dashes = rest.chars().take_while(|&c| c == '-').count();
        let tail = &rest[dashes..];
        if dashes >= 2 && tail.starts_with('>') {
            cur.pos += dashes + 1;
            return Some(EdgeKind::Arrow);
        }
        if dashes >= 3 {
            cur.pos += dashes;
            return Some(EdgeKind::Open);
        }
    }
    None
}

// ---------------------------------------------------------------
// Entity-Relationship (`erDiagram`)
// ---------------------------------------------------------------

/// Parse the body of an `erDiagram`. `header_line` is the
/// 1-indexed line of the `erDiagram` header; everything before and
/// including it is skipped.
fn parse_er(source: &str, header_line: usize) -> Result<ErDiagram, ParseError> {
    let mut d = ErDiagram::default();
    // Entity currently being filled, when inside a `{ ... }` block.
    let mut open: Option<(usize, usize)> = None; // (entity, opening line)
    for (i, raw) in source.lines().enumerate() {
        let lineno = i + 1;
        if lineno <= header_line {
            continue;
        }
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if let Some((ent, _)) = open {
            if line == "}" {
                open = None;
                continue;
            }
            let attr = parse_attr(line, lineno)?;
            d.entities[ent].attrs.push(attr);
            continue;
        }

        if line == "}" {
            return Err(err(lineno, "'}' without an open entity block".to_string()));
        }
        // Entity block start: `Name {`
        if let Some(head) = line.strip_suffix('{') {
            let name = head.trim();
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                open = Some((d.ensure_entity(name), lineno));
                continue;
            }
            return Err(err(lineno, format!("invalid entity name before '{{': '{}'", name)));
        }
        parse_er_statement(&mut d, line, lineno)?;
    }
    if let Some((ent, opened)) = open {
        return Err(err(
            opened,
            format!(
                "entity block '{}' is never closed with '}}'",
                d.entities[ent].name
            ),
        ));
    }
    Ok(d)
}

/// One top-level ER line: either a bare entity declaration or a
/// relationship `A <card><line><card> B : "label"`.
fn parse_er_statement(d: &mut ErDiagram, line: &str, lineno: usize) -> Result<(), ParseError> {
    let mut cur = Cur::new(line);
    let a = parse_er_name(&mut cur, lineno)?;
    cur.skip_ws();
    if cur.at_end() {
        d.ensure_entity(&a);
        return Ok(());
    }
    let (card_from, identifying, card_to) = parse_rel_op(&mut cur).ok_or_else(|| {
        err(
            lineno,
            format!("unknown relationship operator near: '{}'", cur.rest()),
        )
    })?;
    cur.skip_ws();
    let b = parse_er_name(&mut cur, lineno)?;
    cur.skip_ws();
    let label = if cur.eat(":") {
        let l = cur.rest().trim();
        if l.is_empty() {
            None
        } else {
            Some(clean_label_1line(l))
        }
    } else if cur.at_end() {
        None
    } else {
        return Err(err(
            lineno,
            format!("unexpected text after relationship: '{}'", cur.rest()),
        ));
    };
    let from = d.ensure_entity(&a);
    let to = d.ensure_entity(&b);
    d.relations.push(Relation {
        from,
        to,
        card_from,
        card_to,
        identifying,
        label,
    });
    Ok(())
}

fn parse_er_name(cur: &mut Cur<'_>, lineno: usize) -> Result<String, ParseError> {
    cur.skip_ws();
    let start = cur.pos;
    while let Some(c) = cur.peek() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            cur.bump();
        } else {
            break;
        }
    }
    if cur.pos == start {
        return Err(err(
            lineno,
            format!("expected an entity name, found: '{}'", cur.rest()),
        ));
    }
    Ok(cur.s[start..cur.pos].to_string())
}

/// Crow's foot relationship operator, e.g. `||--o{` or `}o..o|`.
/// The token adjacent to each entity is that entity's cardinality.
fn parse_rel_op(cur: &mut Cur<'_>) -> Option<(Card, bool, Card)> {
    const LEFT: &[(&str, Card)] = &[
        ("||", Card::One),
        ("|o", Card::ZeroOne),
        ("}o", Card::ZeroMany),
        ("}|", Card::OneMany),
    ];
    const RIGHT: &[(&str, Card)] = &[
        ("||", Card::One),
        ("o|", Card::ZeroOne),
        ("o{", Card::ZeroMany),
        ("|{", Card::OneMany),
    ];
    let rest = cur.rest();
    let (lt, lc) = LEFT.iter().find(|(t, _)| rest.starts_with(t))?;
    let after_left = &rest[lt.len()..];
    let identifying = if after_left.starts_with("--") {
        true
    } else if after_left.starts_with("..") {
        false
    } else {
        return None;
    };
    let after_line = &after_left[2..];
    let (rt, rc) = RIGHT.iter().find(|(t, _)| after_line.starts_with(t))?;
    cur.pos += lt.len() + 2 + rt.len();
    Some((*lc, identifying, *rc))
}

/// One attribute row: `type name [PK|FK|UK]... ["comment"]`.
/// The comment runs from the first `"` to the last `"`, so it may
/// freely contain commas, parentheses, and single quotes.
fn parse_attr(line: &str, lineno: usize) -> Result<Attr, ParseError> {
    let (head, comment) = match line.find('"') {
        Some(q0) => {
            let q1 = line.rfind('"').unwrap();
            if q1 <= q0 {
                return Err(err(lineno, "unclosed attribute comment quote".to_string()));
            }
            (line[..q0].trim_end(), Some(line[q0 + 1..q1].to_string()))
        }
        None => (line, None),
    };
    let mut toks = head.split_whitespace();
    let ty = toks
        .next()
        .ok_or_else(|| err(lineno, "expected an attribute type".to_string()))?
        .to_string();
    let name = toks
        .next()
        .ok_or_else(|| err(lineno, format!("expected an attribute name after type '{}'", ty)))?
        .to_string();
    let mut keys = Vec::new();
    for t in toks {
        match t.trim_end_matches(',') {
            "PK" => keys.push(Key::Pk),
            "FK" => keys.push(Key::Fk),
            "UK" => keys.push(Key::Uk),
            other => {
                return Err(err(
                    lineno,
                    format!("unknown attribute key: '{}' (expected PK, FK, or UK)", other),
                ))
            }
        }
    }
    Ok(Attr {
        ty,
        name,
        keys,
        comment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Shape;

    #[test]
    fn basic_parse() {
        let g = parse("flowchart TD\nA[Start] --> B{Check?}\nB -->|yes| C((Done))\n").unwrap();
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.edges.len(), 2);
        assert_eq!(g.nodes[1].shape, Shape::Diamond);
        assert_eq!(g.nodes[2].shape, Shape::Circle);
        assert_eq!(g.edges[1].label.as_deref(), Some("yes"));
    }

    #[test]
    fn chains_and_semicolons() {
        let g = parse("graph LR\nA --> B --> C; D -.-> A\n").unwrap();
        assert_eq!(g.nodes.len(), 4);
        assert_eq!(g.edges.len(), 3);
        assert_eq!(g.edges[2].kind, EdgeKind::Dotted);
    }

    #[test]
    fn classic_node_shapes() {
        let g = parse(
            "A[(DB)] --> B[[Sub]]\nB --> C{{Hex}}\nC --> D[/Par/]\nD --> E[\\Rev\\]\nE --> F(((Term)))",
        )
        .unwrap();
        assert_eq!(g.nodes[0].shape, Shape::Cylinder);
        assert_eq!(g.nodes[1].shape, Shape::Subroutine);
        assert_eq!(g.nodes[2].shape, Shape::Hexagon);
        assert_eq!(g.nodes[3].shape, Shape::Parallelogram);
        assert_eq!(g.nodes[4].shape, Shape::ParallelogramAlt);
        assert_eq!(g.nodes[5].shape, Shape::DoubleCircle);
        // Labels come out clean (openers/closers consumed).
        assert_eq!(g.nodes[0].label, "DB");
        assert_eq!(g.nodes[5].label, "Term");
        // Stadium `([ ])` still wins over cylinder `[( )]`.
        assert_eq!(parse("X([Pill])").unwrap().nodes[0].shape, Shape::Stadium);
        // Every new shape reaches the SVG without panicking + in canvas.
        let svg = crate::render_svg("flowchart LR\nA[(DB)]-->B{{H}}-->C(((T)))").unwrap();
        assert!(svg.contains("<path d=") && svg.contains("<polygon") && svg.contains("</svg>"));
    }

    #[test]
    fn quoted_labels() {
        let g = parse("A[\"odd [text]?\"] --> B").unwrap();
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.nodes[0].label, "odd [text]?");
    }

    #[test]
    fn errors_carry_line_numbers() {
        let e = parse("flowchart TD\nA --> \n").unwrap_err();
        assert_eq!(e.line, 2);
    }

    #[test]
    fn unsupported_diagram_types_get_explicit_errors() {
        for src in ["sequenceDiagram\nA->>B: hi", "gantt\ntitle x", "stateDiagram-v2\n[*] --> A"] {
            for res in [
                parse(src).map(|_| ()),
                parse_document(src).map(|_| ()),
            ] {
                let e = res.unwrap_err();
                assert_eq!(e.line, 1);
                assert!(
                    e.message.contains("not supported yet"),
                    "message should say the type is unsupported: {}",
                    e.message
                );
            }
        }
        // The flowchart-only `parse` refuses erDiagram with a pointer
        // to the right entry point.
        let e = parse("erDiagram\nA ||--o{ B : has").unwrap_err();
        assert!(e.message.contains("parse_document"), "{}", e.message);
        // A node that merely starts with a type name is still a node.
        let g = parse("pies[Pie Chart] --> B").unwrap();
        assert_eq!(g.nodes[0].id, "pies");
    }

    // ----------------------------- ER -----------------------------

    fn er(src: &str) -> ErDiagram {
        match parse_document(src).unwrap() {
            Document::Er(d) => d,
            other => panic!("expected an ER document, got {:?}", other),
        }
    }

    #[test]
    fn er_fixture_parses_with_expected_counts() {
        let d = er(include_str!("../examples/er.mmd"));
        let counts: Vec<(&str, usize)> = d
            .entities
            .iter()
            .map(|e| (e.name.as_str(), e.attrs.len()))
            .collect();
        assert_eq!(
            counts,
            [
                ("categories", 9),
                ("questions", 11),
                ("schedules", 13),
                ("settings", 9)
            ]
        );
        assert_eq!(d.relations.len(), 1);
        let r = &d.relations[0];
        assert_eq!(d.entities[r.from].name, "categories");
        assert_eq!(d.entities[r.to].name, "questions");
        assert_eq!(r.card_from, Card::One);
        assert_eq!(r.card_to, Card::ZeroMany);
        assert!(r.identifying);
        assert_eq!(r.label.as_deref(), Some("has"));
    }

    #[test]
    fn er_type_tokens_with_parens_stay_whole() {
        let d = er("erDiagram\nT {\n  varchar(255) name \"not null\"\n  varchar(20) code\n}");
        assert_eq!(d.entities[0].attrs[0].ty, "varchar(255)");
        assert_eq!(d.entities[0].attrs[1].ty, "varchar(20)");
    }

    #[test]
    fn er_comment_survives_commas_parens_and_single_quotes() {
        let d = er("erDiagram\nT {\n  varchar(20) difficulty \"not null, default 'medium' (see docs)\"\n}");
        assert_eq!(
            d.entities[0].attrs[0].comment.as_deref(),
            Some("not null, default 'medium' (see docs)")
        );
    }

    #[test]
    fn er_keys_parse_and_relation_only_entities_exist() {
        let d = er("erDiagram\nA ||..|{ B\nA {\n  uuid id PK\n  uuid b_id FK\n}");
        assert_eq!(d.entities[0].attrs[0].keys, vec![Key::Pk]);
        assert_eq!(d.entities[0].attrs[1].keys, vec![Key::Fk]);
        // B exists only through the relationship: empty table.
        assert_eq!(d.entities[1].name, "B");
        assert!(d.entities[1].attrs.is_empty());
        assert!(!d.relations[0].identifying);
        assert_eq!(d.relations[0].card_to, Card::OneMany);
        assert_eq!(d.relations[0].label, None);
    }

    #[test]
    fn er_errors_have_line_numbers() {
        // Unclosed entity block reports the opening line.
        let e = parse_document("erDiagram\nA {\n  uuid id\n").unwrap_err();
        assert_eq!(e.line, 2);
        // Stray closing brace.
        let e = parse_document("erDiagram\n}\n").unwrap_err();
        assert_eq!(e.line, 2);
        // Bad relationship operator.
        let e = parse_document("erDiagram\nA >>-- B\n").unwrap_err();
        assert_eq!(e.line, 2);
    }

    #[test]
    fn style_classdef_and_triple_colon() {
        let g = parse(
            "flowchart TD\n\
             A[Server] --> B{Ok?}\n\
             B --> C:::hot\n\
             style A fill:#f9f,stroke:#333,stroke-width:4px\n\
             classDef hot fill:#ffe3e3,stroke:#e03131,color:#c92a2a\n\
             class B hot\n\
             style B stroke:#000\n",
        )
        .unwrap();
        // Explicit style line.
        assert_eq!(g.nodes[0].style.fill.as_deref(), Some("#f9f"));
        assert_eq!(g.nodes[0].style.stroke_width, Some(4.0));
        // classDef via `class`, then `style` overrides just the stroke.
        assert_eq!(g.nodes[1].style.fill.as_deref(), Some("#ffe3e3"));
        assert_eq!(g.nodes[1].style.stroke.as_deref(), Some("#000"));
        assert_eq!(g.nodes[1].style.color.as_deref(), Some("#c92a2a"));
        // ::: shorthand, with classDef declared later in the file.
        assert_eq!(g.nodes[2].style.fill.as_deref(), Some("#ffe3e3"));
        // Unknown property is ignored, bad property syntax errors.
        assert!(parse("A --> B\nstyle A rounded").is_err());
        assert!(parse("A --> B\nstyle A glow:heavy,fill:#fff").unwrap().nodes[0]
            .style
            .fill
            .is_some());
    }

    #[test]
    fn fanout_creates_cartesian_edges() {
        // A --> B & C: two edges. Chained: B & C --> D adds two more.
        let g = parse("flowchart TD\nA --> B & C --> D\n").unwrap();
        assert_eq!(g.nodes.len(), 4);
        assert_eq!(g.edges.len(), 4);
        // Both sides can be lists.
        let g = parse("A & B --> C & D").unwrap();
        assert_eq!(g.edges.len(), 4);
        // Fan-out carries the label to every generated edge.
        let g = parse("A -->|x| B & C").unwrap();
        assert!(g.edges.iter().all(|e| e.label.as_deref() == Some("x")));
    }

    #[test]
    fn inline_edge_labels_and_new_link_types() {
        let g = parse(
            "A -- hello --> B\nB -. maybe .- C\nC == loud ==> D\nD --- E\nE -.- F\nF === G\nG ~~~ H\n",
        )
        .unwrap();
        let e = &g.edges;
        assert_eq!(e[0].label.as_deref(), Some("hello"));
        assert_eq!(e[0].kind, EdgeKind::Arrow);
        assert_eq!(e[1].label.as_deref(), Some("maybe"));
        assert_eq!(e[1].kind, EdgeKind::DottedOpen);
        assert_eq!(e[2].label.as_deref(), Some("loud"));
        assert_eq!(e[2].kind, EdgeKind::Thick);
        assert_eq!(e[3].kind, EdgeKind::Open);
        assert_eq!(e[4].kind, EdgeKind::DottedOpen);
        assert_eq!(e[5].kind, EdgeKind::ThickOpen);
        assert_eq!(e[6].kind, EdgeKind::Invisible);
        // Invisible links are layout-only: not drawn in the SVG
        // (edge paths are standalone lines; the arrow marker in
        // <defs> doesn't count).
        let svg = crate::render_svg("A ~~~ B").unwrap();
        assert_eq!(
            svg.lines().filter(|l| l.starts_with("<path d=")).count(),
            0,
            "invisible edge must not be drawn"
        );
        // ...but it still ranks the layout: B ends up below A.
        let g = parse("A ~~~ B").unwrap();
        let s = crate::scene::scene(&g);
        assert!(s.nodes[1].y > s.nodes[0].y, "invisible link still layers");
        // ...and it must NOT inflate the canvas: a wide invisible
        // back-link leaves no phantom empty space (bughunter).
        let vis = crate::scene::scene(&parse("flowchart TD\nA-->B-->C").unwrap());
        let inv = crate::scene::scene(&parse("flowchart TD\nA-->B-->C\nA ~~~ C").unwrap());
        assert!(
            (vis.width - inv.width).abs() < 1.0 && (vis.height - inv.height).abs() < 1.0,
            "invisible link must not change the canvas size ({}x{} vs {}x{})",
            vis.width, vis.height, inv.width, inv.height
        );
        // Unclosed inline label errors with a line number.
        let err = parse("A -- oops -> B").unwrap_err();
        assert!(err.message.contains("never closed"), "{}", err.message);
    }

    #[test]
    fn subgraph_membership_nesting_titles_direction() {
        let g = parse(
            "flowchart TD\n\
             In[Request] --> A1\n\
             subgraph backend [Backend Services]\n\
             direction LR\n\
             A1[API] --> W1\n\
             subgraph workers\n\
             W1[Worker 1] --> W2[Worker 2]\n\
             end\n\
             end\n\
             W2 --> Out[Response]\n",
        )
        .unwrap();
        assert_eq!(g.subgraphs.len(), 2);
        assert_eq!(g.subgraphs[0].id, "backend");
        assert_eq!(g.subgraphs[0].title, "Backend Services");
        assert_eq!(g.subgraphs[0].direction, Some(Direction::LR));
        assert_eq!(g.subgraphs[1].parent, Some(0));
        // Mermaid claim rule: A1 first appears at top level but is
        // later mentioned inside `backend`, which claims it.
        let a1 = g.node_index("A1").unwrap();
        assert!(g.subgraphs[0].nodes.contains(&a1), "backend claims A1");
        // W1 is created in `backend`, then re-mentioned inside the
        // nested `workers` block, which claims it from its parent.
        let w1 = g.node_index("W1").unwrap();
        assert!(g.subgraphs[1].nodes.contains(&w1), "workers claims W1");
        assert!(!g.subgraphs[0].nodes.contains(&w1), "and backend lets it go");
        // A later TOP-LEVEL mention (`W2 --> Out`) never un-claims.
        let w2 = g.node_index("W2").unwrap();
        assert!(g.subgraphs[1].nodes.contains(&w2), "W2 stays in workers");
        // Unclosed block errors with the subgraph's name.
        let e = parse("flowchart TD\nsubgraph x\nA\n").unwrap_err();
        assert!(e.message.contains("never closed"), "{}", e.message);
        // Edge to a declared subgraph is now a sub_edge, not an error.
        let g2 = parse("flowchart TD\nsubgraph one\nA\nend\nB --> one\n").unwrap();
        assert_eq!(g2.sub_edges.len(), 1);
        assert_eq!(g2.sub_edges[0].to, crate::model::End::Sub(0));
    }

    #[test]
    fn edges_to_and_from_a_subgraph() {
        let g = parse(
            "flowchart TD\n\
             subgraph net [Network]\n\
             A[Gateway] --> B[Router]\n\
             end\n\
             CF[CloudFront] --> net\n\
             net --> DB[Store]\n",
        )
        .unwrap();
        // Node→node edge inside the subgraph stays a normal edge.
        assert_eq!(g.edges.len(), 1);
        // The two subgraph-touching edges land in sub_edges.
        assert_eq!(g.sub_edges.len(), 2);
        use crate::model::End;
        let cf = g.node_index("CF").unwrap();
        assert_eq!(g.sub_edges[0].from, End::Node(cf));
        assert_eq!(g.sub_edges[0].to, End::Sub(0));
        assert_eq!(g.sub_edges[1].from, End::Sub(0));
        // No stray "net" node was created.
        assert!(g.node_index("net").is_none());
        // Scene renders the sub-edges (2 node edges' worth extra) and
        // stays inside the canvas.
        let s = crate::scene::scene(&g);
        // 1 real edge + 2 sub_edges = 3 drawn edges.
        assert_eq!(s.edges.len(), 3);
        // Full canvas containment for the paths.
        let svg = crate::render_svg("flowchart TD\nsubgraph net\nA-->B\nend\nCF-->net").unwrap();
        let head = svg_head(&svg);
        for line in svg.lines().filter(|l| l.starts_with("<path d=")) {
            for (x, y) in path_coords(line) {
                assert!(x >= -0.5 && x <= head.0 + 0.5, "x={x} out of {}", head.0);
                assert!(y >= -0.5 && y <= head.1 + 0.5, "y={y} out of {}", head.1);
            }
        }
    }

    fn svg_head(svg: &str) -> (f64, f64) {
        let attr = |name: &str| -> f64 {
            let pat = format!("{name}=\"");
            let i = svg.find(&pat).unwrap() + pat.len();
            svg[i..i + svg[i..].find('"').unwrap()].parse().unwrap()
        };
        (attr("width"), attr("height"))
    }

    fn path_coords(line: &str) -> Vec<(f64, f64)> {
        let i = line.find("d=\"").unwrap() + 3;
        let d = &line[i..i + line[i..].find('"').unwrap()];
        let nums: Vec<f64> = d
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        nums.chunks(2).map(|c| (c[0], c[1])).collect()
    }

    #[test]
    fn bughunt_fixes_subgraph_edges_and_br() {
        use crate::model::End;
        // Forward reference: subgraph declared AFTER the edge.
        let g = parse("flowchart TD\nA --> grp\nsubgraph grp\nX\nend\n").unwrap();
        assert!(g.node_index("grp").is_none(), "no stray 'grp' node");
        assert_eq!(g.sub_edges.len(), 1);
        assert_eq!(g.sub_edges[0].to, End::Sub(0));
        // Shape on a subgraph id doesn't create a duplicate node.
        let g = parse("flowchart TD\nsubgraph grp\nX\nend\nA --> grp[Ignore]\n").unwrap();
        assert!(g.node_index("grp").is_none(), "subgraph wins over shape");
        assert_eq!(g.sub_edges.len(), 1);
        // A label of only <br/> collapses to empty (no blank lines).
        assert_eq!(parse("A[\"<br/>\"]").unwrap().nodes[0].label, "");
        // Edge labels & subgraph titles are single-line.
        let g = parse("A -->|one<br/>two| B\nsubgraph s[\"Ti<br/>tle\"]\nB\nend").unwrap();
        assert_eq!(g.edges[0].label.as_deref(), Some("one two"));
        assert_eq!(g.subgraphs[0].title, "Ti tle");
        // Empty subgraph: scene and route agree (no phantom cluster).
        let g = parse("flowchart TD\nA-->B\nsubgraph empty\nend\nB --> empty").unwrap();
        let s = crate::scene::scene(&g);
        let pos: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        let r = crate::scene::route(&g, &pos);
        assert_eq!(s.clusters.len(), r.clusters.len(), "cluster count consistent");
        assert_eq!(s.edges.len(), r.edges.len(), "edge count consistent");
    }

    #[test]
    fn br_becomes_newline_and_grows_the_node() {
        let g = parse("flowchart TD\nA[\"CloudFront<br/>E2GQ<br />dbfu9k\"] --> B[One line]").unwrap();
        assert_eq!(g.nodes[0].label, "CloudFront\nE2GQ\ndbfu9k");
        // <BR> uppercase and bare <br> also work.
        let g2 = parse("A[x<BR>y<br>z]").unwrap();
        assert_eq!(g2.nodes[0].label, "x\ny\nz");
        // Taller node for the multi-line label; wider = widest line.
        let (w3, h3) = crate::layout::intrinsic_size(&g.nodes[0]);
        let (_w1, h1) = crate::layout::intrinsic_size(&g.nodes[1]);
        // Two extra lines add ~2×LINE_H of height.
        assert!(h3 > h1 + 30.0, "3-line node taller by ~2 lines: {h3} vs {h1}");
        // SVG carries one tspan per line.
        let svg = crate::render_svg("A[a<br/>bb]").unwrap();
        assert_eq!(svg.matches("<tspan").count(), 2);
        assert!(w3 > 0.0);
    }

    #[test]
    fn custom_fill_reaches_the_svg() {
        let svg = crate::render_svg("A[X] --> B\nstyle A fill:#123456,color:#ffffff").unwrap();
        assert!(svg.contains("fill=\"#123456\""));
        assert!(svg.contains("fill=\"#ffffff\">X</text>"));
    }

    #[test]
    fn keyword_like_id_is_not_a_header() {
        // Previously: "graphics" was mistaken for the "graph" header
        // plus direction "ics[...]" -> error.
        let g = parse("graphics[Graphics] --> B").unwrap();
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.nodes[0].id, "graphics");
        assert_eq!(g.direction, crate::model::Direction::TD);
    }
}
